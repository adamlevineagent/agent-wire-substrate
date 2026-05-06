use std::collections::hash_map::DefaultHasher;
use std::env;
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use agent_wire_transport_cloudflare::ensure_cloudflared_binary;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::layer5_live_llm::ChatCompletionsProviderConfig;
use crate::mainnet_auth::load_persisted_mainnet_credential;
use crate::v1_runtime::default_state_dir;

const DEFAULT_D3_MODEL: &str = "granite-4-micro";
const DEFAULT_SUPABASE_URL: &str = "https://supabase.newsbleach.com";
const D3_PROMPT: &str = "Return exactly this sentence: D3 live compute settlement green.";
const DEFAULT_FILL_RETRY_TIMEOUT_SECS: u64 = 180;
const DEFAULT_FILL_RETRY_MAX_ATTEMPTS: u32 = 24;
const DEFAULT_FILL_RETRY_BACKOFF_MILLIS: u64 = 5_000;
const DEFAULT_FILL_RETRY_MAX_JITTER_MILLIS: u64 = 1_000;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct D3LiveComputeSettlementReport {
    pub endpoint: String,
    pub supabase_url: String,
    pub model_id: String,
    pub provider_node_id: Option<String>,
    pub requester_node_id: Option<String>,
    pub tunnel_url: Option<String>,
    pub offer_id: Option<String>,
    pub job_id: Option<String>,
    pub uuid_job_id: Option<String>,
    pub settlement_id: Option<String>,
    pub settlement_status: Option<String>,
    pub actual_cost: Option<i64>,
    pub provider_payout: Option<i64>,
    pub requester_adjustment: Option<i64>,
    pub subtests: Vec<D3Subtest>,
}

impl D3LiveComputeSettlementReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, D3Status::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# D3 Live Compute Settlement Validation\n\n");
        output.push_str("Endpoint: `");
        output.push_str(&self.endpoint);
        output.push_str("`\n\n");
        output.push_str("Supabase read surface: `");
        output.push_str(&self.supabase_url);
        output.push_str("`\n\n");
        output.push_str("Model: `");
        output.push_str(&self.model_id);
        output.push_str("`\n\n");
        if let Some(provider_node_id) = &self.provider_node_id {
            output.push_str("- provider_node_id: `");
            output.push_str(provider_node_id);
            output.push_str("`\n");
        }
        if let Some(requester_node_id) = &self.requester_node_id {
            output.push_str("- requester_node_id: `");
            output.push_str(requester_node_id);
            output.push_str("`\n");
        }
        if let Some(tunnel_url) = &self.tunnel_url {
            output.push_str("- tunnel_url: `");
            output.push_str(tunnel_url);
            output.push_str("`\n");
        }
        if let Some(offer_id) = &self.offer_id {
            output.push_str("- offer_id: `");
            output.push_str(offer_id);
            output.push_str("`\n");
        }
        if let Some(job_id) = &self.job_id {
            output.push_str("- job_id: `");
            output.push_str(job_id);
            output.push_str("`\n");
        }
        if let Some(uuid_job_id) = &self.uuid_job_id {
            output.push_str("- uuid_job_id: `");
            output.push_str(uuid_job_id);
            output.push_str("`\n");
        }
        if let Some(settlement_id) = &self.settlement_id {
            output.push_str("- settlement_id: `");
            output.push_str(settlement_id);
            output.push_str("`\n");
        }
        if let Some(settlement_status) = &self.settlement_status {
            output.push_str("- settlement_status: `");
            output.push_str(settlement_status);
            output.push_str("`\n");
        }
        if self.actual_cost.is_some()
            || self.provider_payout.is_some()
            || self.requester_adjustment.is_some()
        {
            output.push_str("- actual_cost/provider_payout/requester_adjustment: `");
            output.push_str(&format!(
                "{}/{}/{}",
                self.actual_cost.unwrap_or_default(),
                self.provider_payout.unwrap_or_default(),
                self.requester_adjustment.unwrap_or_default()
            ));
            output.push_str("`\n");
        }
        output.push('\n');

        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "D3 is green: the provider accepted a real mainnet dispatch, executed a live LLM call, posted settlement, and `wire_settlements` exposes the cleared row.\n\n"
        } else {
            "D3 failed closed; see the sub-test reasons below.\n\n"
        });

        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                D3Status::Passed => "PASS",
                D3Status::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let D3Status::Failed { reason } = &subtest.status {
                output.push_str(" Reason: ");
                output.push_str(reason);
            }
            output.push('\n');
            for detail in &subtest.details {
                output.push_str("  - ");
                output.push_str(detail);
                output.push('\n');
            }
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct D3Subtest {
    pub name: String,
    pub proves: String,
    pub status: D3Status,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum D3Status {
    Passed,
    Failed { reason: String },
}

#[derive(Debug, Clone)]
struct D3Config {
    endpoint: String,
    supabase_url: String,
    service_role_key: String,
    llm_provider: ChatCompletionsProviderConfig,
    model_id: String,
    cloudflared_path: PathBuf,
    max_tokens: u32,
    max_budget: i64,
    tunnel_health_timeout_secs: u64,
    fill_retry_policy: D3FillRetryPolicy,
}

#[derive(Debug, Clone, Copy)]
struct D3FillRetryPolicy {
    timeout_secs: u64,
    max_attempts: u32,
    backoff_millis: u64,
    max_jitter_millis: u64,
}

#[derive(Debug, Clone)]
struct NodeInfo {
    id: String,
    registration_detail: String,
}

#[derive(Debug, Clone)]
struct TunnelInfo {
    token: String,
    url: String,
}

#[derive(Debug, Default)]
struct ProviderState {
    dispatch_seen: bool,
    requester_result_seen: bool,
    settlement_posted: bool,
    provider_request_id: Option<String>,
    settlement_response: Option<String>,
    error: Option<String>,
}

struct ProviderServer {
    port: u16,
    state: Arc<Mutex<ProviderState>>,
    stop: Arc<AtomicBool>,
    join: Option<thread::JoinHandle<()>>,
}

impl Drop for ProviderServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

struct CloudflaredChild {
    child: Child,
}

impl Drop for CloudflaredChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn run_d3_live_compute_settlement() -> D3LiveComputeSettlementReport {
    match run_d3_live_compute_settlement_inner() {
        Ok(report) => report,
        Err((mut report, stage, reason)) => {
            report.subtests.push(failed_step(
                stage,
                "the live D3 validation fails closed and reports the blocking stage",
                reason,
            ));
            report
        }
    }
}

fn run_d3_live_compute_settlement_inner(
) -> Result<D3LiveComputeSettlementReport, (D3LiveComputeSettlementReport, &'static str, String)> {
    let config = D3Config::from_env().map_err(|reason| {
        (
            D3LiveComputeSettlementReport::failed_config(),
            "d3-config-resolves",
            reason,
        )
    })?;
    let mut report = D3LiveComputeSettlementReport {
        endpoint: config.endpoint.clone(),
        supabase_url: config.supabase_url.clone(),
        model_id: config.model_id.clone(),
        provider_node_id: None,
        requester_node_id: None,
        tunnel_url: None,
        offer_id: None,
        job_id: None,
        uuid_job_id: None,
        settlement_id: None,
        settlement_status: None,
        actual_cost: None,
        provider_payout: None,
        requester_adjustment: None,
        subtests: vec![passed_step(
            "d3-config-resolves",
            "all live endpoints, settlement read credentials, LLM provider config, and cloudflared binary are present",
            vec![
                format!("endpoint {}", config.endpoint),
                format!("supabase {}", config.supabase_url),
                format!(
                    "llm provider {} via {} at {}",
                    config.llm_provider.provider,
                    config.llm_provider.adapter_id(),
                    config.llm_provider.base_url()
                ),
                format!("model {}", config.model_id),
                format!("cloudflared {}", config.cloudflared_path.display()),
            ],
        )],
    };

    let credential = load_persisted_mainnet_credential()
        .map_err(|reason| (report.clone(), "mainnet-auth-loads", reason))?;
    report.subtests.push(passed_step(
        "mainnet-auth-loads",
        "the substrate node can reuse the persisted confirmed Wire credential",
        vec![
            format!("agent {}", credential.identity.name),
            format!("agent_id {}", credential.identity.agent_id),
            format!("handle {}", credential.identity.handle_path),
        ],
    ));

    let provider_server = start_provider_server(config.clone())
        .map_err(|reason| (report.clone(), "local-provider-server-starts", reason))?;
    report.subtests.push(passed_step(
        "local-provider-server-starts",
        "the substrate provider exposes local job-dispatch and requester-result endpoints",
        vec![format!("local port {}", provider_server.port)],
    ));

    let suffix = Uuid::new_v4().to_string()[..8].to_owned();
    let provider = ensure_node(
        &config,
        &credential.api_token,
        &credential.identity.agent_id,
        &format!("d3-kramer-provider-{suffix}"),
    )
    .map_err(|reason| (report.clone(), "provider-node-registers", reason))?;
    report.provider_node_id = Some(provider.id.clone());
    report.subtests.push(passed_step(
        "provider-node-registers",
        "the D3 provider exists as a real mainnet wire_nodes row",
        vec![
            format!("provider_node_id {}", provider.id),
            provider.registration_detail.clone(),
        ],
    ));

    let requester = ensure_node(
        &config,
        &credential.api_token,
        &credential.identity.agent_id,
        &format!("d3-kramer-requester-{suffix}"),
    )
    .map_err(|reason| (report.clone(), "requester-node-registers", reason))?;
    report.requester_node_id = Some(requester.id.clone());
    report.subtests.push(passed_step(
        "requester-node-registers",
        "the D3 requester exists as a separate real mainnet wire_nodes row",
        vec![
            format!("requester_node_id {}", requester.id),
            requester.registration_detail.clone(),
        ],
    ));

    let tunnel = provision_tunnel(
        &config,
        &credential.api_token,
        &provider.id,
        provider_server.port,
    )
    .map_err(|reason| (report.clone(), "cloudflare-tunnel-provisions", reason))?;
    report.tunnel_url = Some(tunnel.url.clone());
    let cloudflared = start_cloudflared(&config, &tunnel)
        .map_err(|reason| (report.clone(), "cloudflared-process-starts", reason))?;
    wait_for_tunnel_health(&tunnel.url, config.tunnel_health_timeout_secs)
        .map_err(|reason| (report.clone(), "cloudflare-tunnel-reachable", reason))?;
    heartbeat_node(
        &config,
        &credential.api_token,
        &provider.id,
        Some(&tunnel.url),
    )
    .map_err(|reason| (report.clone(), "provider-heartbeat-updates-tunnel", reason))?;
    heartbeat_node(&config, &credential.api_token, &requester.id, None)
        .map_err(|reason| (report.clone(), "requester-heartbeat-updates", reason))?;
    report.subtests.push(passed_step(
        "cloudflare-tunnel-reachable",
        "a live Cloudflare tunnel reaches the local substrate provider server",
        vec![tunnel.url.clone()],
    ));

    let offer_id = publish_offer(&config, &credential.api_token, &provider.id)
        .map_err(|reason| (report.clone(), "provider-offer-publishes", reason))?;
    report.offer_id = Some(offer_id.clone());
    push_queue_mirror(&config, &credential.api_token, &provider.id, &offer_id)
        .map_err(|reason| (report.clone(), "provider-queue-mirror-pushes", reason))?;
    report.subtests.push(passed_step(
        "provider-offer-publishes-and-mirrors",
        "the provider publishes a live compute offer and fresh queue mirror for the model",
        vec![format!("offer_id {}", offer_id)],
    ));

    let quote = request_quote(&config, &credential.api_token, &requester.id)
        .map_err(|reason| (report.clone(), "requester-quote-reserves-winner", reason))?;
    let purchase = purchase_quote(&config, &credential.api_token, &quote)
        .map_err(|reason| (report.clone(), "requester-purchase-commits-job", reason))?;
    report.job_id = Some(purchase.job_id.clone());
    report.uuid_job_id = Some(purchase.uuid_job_id.clone());
    report.subtests.push(passed_step(
        "requester-purchases-real-compute-job",
        "a real mainnet quote is purchased into a reserved compute job",
        vec![
            format!("job_id {}", purchase.job_id),
            format!("uuid_job_id {}", purchase.uuid_job_id),
        ],
    ));

    let fill = fill_job(
        &config,
        &credential.api_token,
        &purchase.job_id,
        &format!("{}/v1/compute/job-result", tunnel.url.trim_end_matches('/')),
    )
    .map_err(|reason| {
        (
            report.clone(),
            "requester-fill-dispatches-to-provider",
            reason,
        )
    })?;
    report.subtests.push(passed_step(
        "requester-fill-dispatches-to-provider",
        "Wire charges the fill deposit and dispatches the job to the provider tunnel",
        vec![format!(
            "fill response {}",
            trim_for_report(&fill.to_string())
        )],
    ));

    wait_for_provider_completion(&provider_server.state).map_err(|reason| {
        (
            report.clone(),
            "provider-executes-and-posts-settlement",
            reason,
        )
    })?;
    report.subtests.push(passed_step(
        "provider-executes-and-posts-settlement",
        "the provider performs a live LLM call, delivers requester content, and posts Wire settlement",
        provider_completion_details(&provider_server.state),
    ));

    let job = wait_for_job_settled(&config, &credential.api_token, &purchase.job_id)
        .map_err(|reason| (report.clone(), "job-status-settled", reason))?;
    report.subtests.push(passed_step(
        "job-status-settled",
        "the mainnet compute job reaches completed/settled state",
        vec![format!("job status {}", trim_for_report(&job.to_string()))],
    ));

    let settlement = read_settlement(&config, &purchase.uuid_job_id)
        .map_err(|reason| (report.clone(), "wire-settlements-row-visible", reason))?;
    report.settlement_id = settlement
        .get("settlement_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    report.settlement_status = settlement
        .get("settlement_status")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    report.actual_cost = settlement.get("actual_cost").and_then(Value::as_i64);
    report.provider_payout = settlement.get("provider_payout").and_then(Value::as_i64);
    report.requester_adjustment = settlement
        .get("requester_adjustment")
        .and_then(Value::as_i64);
    report.subtests.push(passed_step(
        "wire-settlements-row-visible",
        "service_role can read the real settlement clearance row from wire_settlements",
        vec![format!(
            "settlement {}",
            trim_for_report(&settlement.to_string())
        )],
    ));

    drop(cloudflared);
    drop(provider_server);

    Ok(report)
}

impl D3LiveComputeSettlementReport {
    fn failed_config() -> Self {
        Self {
            endpoint: env::var("WIRE_MAINNET_ENDPOINT")
                .unwrap_or_else(|_| "https://newsbleach.com/api/v1".to_owned()),
            supabase_url: env::var("NEXT_PUBLIC_SUPABASE_URL")
                .or_else(|_| env::var("SUPABASE_URL"))
                .unwrap_or_else(|_| DEFAULT_SUPABASE_URL.to_owned()),
            model_id: env::var("D3_MODEL")
                .or_else(|_| env::var("LM_STUDIO_MODEL"))
                .or_else(|_| env::var("LAYER5_MODEL"))
                .or_else(|_| env::var("OPENROUTER_MODEL"))
                .unwrap_or_else(|_| DEFAULT_D3_MODEL.to_owned()),
            provider_node_id: None,
            requester_node_id: None,
            tunnel_url: None,
            offer_id: None,
            job_id: None,
            uuid_job_id: None,
            settlement_id: None,
            settlement_status: None,
            actual_cost: None,
            provider_payout: None,
            requester_adjustment: None,
            subtests: Vec::new(),
        }
    }
}

impl D3Config {
    fn from_env() -> Result<Self, String> {
        let endpoint = env::var("WIRE_MAINNET_ENDPOINT")
            .unwrap_or_else(|_| "https://newsbleach.com/api/v1".to_owned())
            .trim_end_matches('/')
            .to_owned();
        let supabase_url = env::var("SUPABASE_URL")
            .or_else(|_| env::var("NEXT_PUBLIC_SUPABASE_URL"))
            .unwrap_or_else(|_| DEFAULT_SUPABASE_URL.to_owned())
            .trim_end_matches('/')
            .to_owned();
        let service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| "SUPABASE_SERVICE_ROLE_KEY is not set".to_owned())?;
        if service_role_key.trim().is_empty() {
            return Err("SUPABASE_SERVICE_ROLE_KEY is present but empty".to_owned());
        }
        let llm_provider = ChatCompletionsProviderConfig::from_d3_env()?;
        let model_id = llm_provider.model_id.clone();
        let state_dir = default_state_dir();
        let cloudflared_path = ensure_cloudflared_binary(&state_dir)
            .map_err(|error| format!("cloudflared resolver failed: {error}"))?;
        let max_tokens = env::var("D3_MAX_TOKENS")
            .ok()
            .and_then(|raw| raw.parse::<u32>().ok())
            .unwrap_or(160);
        let max_budget = env::var("D3_MAX_BUDGET")
            .ok()
            .and_then(|raw| raw.parse::<i64>().ok())
            .unwrap_or(100);
        let tunnel_health_timeout_secs = env::var("D3_TUNNEL_HEALTH_TIMEOUT_SECS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .unwrap_or(120);
        let fill_retry_policy = D3FillRetryPolicy::from_env();

        Ok(Self {
            endpoint,
            supabase_url,
            service_role_key,
            llm_provider,
            model_id,
            cloudflared_path,
            max_tokens,
            max_budget,
            tunnel_health_timeout_secs,
            fill_retry_policy,
        })
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/{}", self.endpoint, path.trim_start_matches('/'))
    }

    fn rest_url(&self, path: &str) -> String {
        format!(
            "{}/rest/v1/{}",
            self.supabase_url,
            path.trim_start_matches('/')
        )
    }
}

impl D3FillRetryPolicy {
    fn from_env() -> Self {
        Self {
            timeout_secs: env_u64(
                "D3_FILL_RETRY_TIMEOUT_SECS",
                DEFAULT_FILL_RETRY_TIMEOUT_SECS,
            )
            .max(1),
            max_attempts: env_u64(
                "D3_FILL_RETRY_MAX_ATTEMPTS",
                u64::from(DEFAULT_FILL_RETRY_MAX_ATTEMPTS),
            )
            .clamp(1, u64::from(u32::MAX)) as u32,
            backoff_millis: env_u64(
                "D3_FILL_RETRY_BACKOFF_MILLIS",
                DEFAULT_FILL_RETRY_BACKOFF_MILLIS,
            )
            .max(250),
            max_jitter_millis: env_u64(
                "D3_FILL_RETRY_MAX_JITTER_MILLIS",
                DEFAULT_FILL_RETRY_MAX_JITTER_MILLIS,
            ),
        }
    }

    fn next_sleep(
        &self,
        reason: &str,
        attempts: u32,
        started_at: Instant,
        idempotency_key: &str,
    ) -> Option<Duration> {
        if !fill_error_is_retryable(reason) || attempts >= self.max_attempts {
            return None;
        }
        let elapsed = started_at.elapsed();
        let timeout = Duration::from_secs(self.timeout_secs);
        if elapsed >= timeout {
            return None;
        }
        let remaining = timeout.saturating_sub(elapsed);
        let sleep = self.retry_sleep_duration(idempotency_key, attempts);
        Some(if sleep > remaining { remaining } else { sleep })
    }

    fn retry_sleep_duration(&self, idempotency_key: &str, attempts: u32) -> Duration {
        Duration::from_millis(
            self.backoff_millis
                + retry_jitter_millis(idempotency_key, attempts, self.max_jitter_millis),
        )
    }
}

fn env_u64(name: &str, default: u64) -> u64 {
    env::var(name)
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(default)
}

fn ensure_node(
    config: &D3Config,
    api_token: &str,
    agent_id: &str,
    name: &str,
) -> Result<NodeInfo, String> {
    match register_node(config, api_token, name) {
        Ok(node) => Ok(node),
        Err(register_error) => {
            let mut node =
                bootstrap_node_via_service_role(config, agent_id, name).map_err(|bootstrap_error| {
                format!(
                    "node/register failed ({register_error}); service-role bootstrap also failed ({bootstrap_error})"
                )
            })?;
            node.registration_detail = format!(
                "node/register failed ({}); {}",
                trim_for_report(&register_error),
                node.registration_detail
            );
            Ok(node)
        }
    }
}

fn register_node(config: &D3Config, api_token: &str, name: &str) -> Result<NodeInfo, String> {
    let response = api_post(
        &config.api_url("/node/register"),
        api_token,
        json!({ "name": name, "capabilities": ["compute"] }),
    )?;
    let id = response
        .get("node_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node/register response missing node_id: {response}"))?;
    Ok(NodeInfo {
        id: id.to_owned(),
        registration_detail: "node/register succeeded".to_owned(),
    })
}

fn bootstrap_node_via_service_role(
    config: &D3Config,
    agent_id: &str,
    name: &str,
) -> Result<NodeInfo, String> {
    let agent_rows = rest_get(
        config,
        &format!(
            "wire_agents?id=eq.{}&select=id,operator_id",
            percent_encode_component(agent_id)
        ),
    )?;
    let agent = agent_rows
        .as_array()
        .and_then(|rows| rows.first())
        .ok_or_else(|| format!("wire_agents row not found for agent_id {agent_id}"))?;
    let operator_id = agent
        .get("operator_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "wire_agents.operator_id is missing".to_owned())?;
    let node_handle = sanitize_node_handle(name);
    let now = now_rfc3339();
    let payload = json!({
        "name": name,
        "agent_id": agent_id,
        "operator_id": operator_id,
        "node_handle": node_handle,
        "capabilities": ["compute"],
        "status": "online",
        "last_heartbeat": now,
        "last_seen_at": now
    });
    let inserted = rest_post(
        config,
        "wire_nodes?select=id,name,node_handle,status",
        payload,
    )?;
    let id = inserted
        .as_array()
        .and_then(|rows| rows.first())
        .and_then(|row| row.get("id"))
        .and_then(Value::as_str)
        .ok_or_else(|| format!("wire_nodes insert response missing id: {inserted}"))?;
    Ok(NodeInfo {
        id: id.to_owned(),
        registration_detail: format!(
            "node/register fallback: service_role bootstrap succeeded with node_handle `{node_handle}`"
        ),
    })
}

fn provision_tunnel(
    config: &D3Config,
    api_token: &str,
    node_id: &str,
    local_port: u16,
) -> Result<TunnelInfo, String> {
    let response = api_post(
        &config.api_url("/node/tunnel"),
        api_token,
        json!({ "node_id": node_id, "local_port": local_port }),
    )?;
    let token = response
        .get("tunnel_token")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node/tunnel response missing tunnel_token: {response}"))?;
    let url = response
        .get("tunnel_url")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("node/tunnel response missing tunnel_url: {response}"))?;
    Ok(TunnelInfo {
        token: token.to_owned(),
        url: url.trim_end_matches('/').to_owned(),
    })
}

fn start_cloudflared(config: &D3Config, tunnel: &TunnelInfo) -> Result<CloudflaredChild, String> {
    let child = Command::new(&config.cloudflared_path)
        .arg("--no-autoupdate")
        .arg("tunnel")
        .arg("run")
        .arg("--token")
        .arg(&tunnel.token)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to spawn cloudflared: {error}"))?;
    Ok(CloudflaredChild { child })
}

fn wait_for_tunnel_health(tunnel_url: &str, timeout_secs: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(timeout_secs);
    let health_url = format!("{}/health", tunnel_url.trim_end_matches('/'));
    let mut last = String::new();
    while Instant::now() < deadline {
        match ureq::get(&health_url)
            .timeout(Duration::from_secs(5))
            .call()
        {
            Ok(response) if response.status() == 200 => return Ok(()),
            Ok(response) => last = format!("HTTP {}", response.status()),
            Err(error) => last = error.to_string(),
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err(format!("tunnel health did not become reachable: {last}"))
}

fn heartbeat_node(
    config: &D3Config,
    api_token: &str,
    node_id: &str,
    tunnel_url: Option<&str>,
) -> Result<(), String> {
    let mut payload = json!({ "node_id": node_id });
    if let Some(tunnel_url) = tunnel_url {
        payload["tunnel_url"] = Value::String(tunnel_url.to_owned());
    }
    api_post(&config.api_url("/node/heartbeat"), api_token, payload).map(|_| ())
}

fn publish_offer(config: &D3Config, api_token: &str, node_id: &str) -> Result<String, String> {
    let response = api_post(
        &config.api_url("/compute/offers"),
        api_token,
        json!({
            "node_id": node_id,
            "model_id": config.model_id,
            "provider_type": "local",
            "rate_per_m_input": 1,
            "rate_per_m_output": 1,
            "reservation_fee": 1,
            "queue_discount_curve": [],
            "max_queue_depth": 1
        }),
    )?;
    response
        .get("offer_id")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("offers response missing offer_id: {response}"))
}

fn push_queue_mirror(
    config: &D3Config,
    api_token: &str,
    node_id: &str,
    offer_id: &str,
) -> Result<(), String> {
    let snapshot_seq = OffsetDateTime::now_utc().unix_timestamp() as u64;
    api_post(
        &config.api_url("/compute/queue-mirror"),
        api_token,
        json!({
            "node_id": node_id,
            "is_serving": true,
            "snapshot_seq": snapshot_seq,
            "offers": [{
                "model_id": config.model_id,
                "wire_offer_id": offer_id,
                "current_queue_depth": 0,
                "max_queue_depth": 1,
                "allow_market_visibility": true
            }]
        }),
    )
    .map(|_| ())
}

fn request_quote(
    config: &D3Config,
    api_token: &str,
    requester_node_id: &str,
) -> Result<String, String> {
    let response = api_post(
        &config.api_url("/compute/quote"),
        api_token,
        json!({
            "model_id": config.model_id,
            "input_tokens_est": 32,
            "max_tokens": config.max_tokens,
            "latency_preference": "best_price",
            "max_budget": config.max_budget,
            "requester_node_id": requester_node_id
        }),
    )?;
    response
        .get("quote_jwt")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("quote response missing quote_jwt: {response}"))
}

#[derive(Debug, Clone)]
struct PurchaseResult {
    job_id: String,
    uuid_job_id: String,
}

fn purchase_quote(
    config: &D3Config,
    api_token: &str,
    quote_jwt: &str,
) -> Result<PurchaseResult, String> {
    let response = api_post(
        &config.api_url("/compute/purchase"),
        api_token,
        json!({
            "quote_jwt": quote_jwt,
            "trigger": "immediate",
            "idempotency_key": Uuid::new_v4().to_string()
        }),
    )?;
    let job_id = response
        .get("job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("purchase response missing job_id: {response}"))?;
    let uuid_job_id = response
        .get("uuid_job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("purchase response missing uuid_job_id: {response}"))?;
    Ok(PurchaseResult {
        job_id: job_id.to_owned(),
        uuid_job_id: uuid_job_id.to_owned(),
    })
}

fn fill_job(
    config: &D3Config,
    api_token: &str,
    job_id: &str,
    requester_callback_url: &str,
) -> Result<Value, String> {
    let url = config.api_url("/compute/fill");
    let auth_header = format!("Bearer {api_token}");
    let idempotency_key = Uuid::new_v4().to_string();
    let payload = json!({
        "job_id": job_id,
        "input_token_count": 32,
        "max_tokens": config.max_tokens,
        "temperature": 0,
        "relay_count": 0,
        "privacy_tier": "direct",
        "requester_callback_url": requester_callback_url,
        "messages": [
            {
                "role": "system",
                "content": "You are validating an agent-wire D3 live compute market roundtrip."
            },
            {
                "role": "user",
                "content": D3_PROMPT
            }
        ]
    });
    let started_at = Instant::now();
    let mut attempts: u32 = 0;
    loop {
        attempts += 1;
        let response = ureq::post(&url)
            .set("Authorization", &auth_header)
            .set("Content-Type", "application/json")
            .set("Idempotency-Key", &idempotency_key)
            .send_json(payload.clone());
        match parse_ureq_response(response) {
            Ok(value) => return Ok(value),
            Err(reason) => {
                let Some(sleep) = config.fill_retry_policy.next_sleep(
                    &reason,
                    attempts,
                    started_at,
                    &idempotency_key,
                ) else {
                    if attempts > 1 {
                        return Err(format!(
                            "fill failed after {attempts} attempts with stable idempotency key: {reason}"
                        ));
                    }
                    return Err(reason);
                };
                thread::sleep(sleep);
            }
        }
    }
}

fn fill_error_is_retryable(reason: &str) -> bool {
    reason.contains("provider_unreachable")
        || reason.contains("http_530")
        || reason.contains("HTTP 502")
        || reason.contains("HTTP 503")
        || reason.contains("HTTP 504")
}

fn retry_jitter_millis(idempotency_key: &str, attempts: u32, max_jitter_millis: u64) -> u64 {
    if max_jitter_millis == 0 {
        return 0;
    }
    let mut hasher = DefaultHasher::new();
    idempotency_key.hash(&mut hasher);
    attempts.hash(&mut hasher);
    hasher.finish() % (max_jitter_millis + 1)
}

fn wait_for_provider_completion(state: &Arc<Mutex<ProviderState>>) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    while Instant::now() < deadline {
        let guard = state
            .lock()
            .map_err(|_| "provider state mutex poisoned".to_owned())?;
        if let Some(error) = &guard.error {
            return Err(error.clone());
        }
        if guard.dispatch_seen && guard.requester_result_seen && guard.settlement_posted {
            return Ok(());
        }
        drop(guard);
        thread::sleep(Duration::from_millis(500));
    }
    Err("provider did not complete dispatch/result/settlement before timeout".to_owned())
}

fn provider_completion_details(state: &Arc<Mutex<ProviderState>>) -> Vec<String> {
    let guard = state.lock().ok();
    let Some(guard) = guard else {
        return vec!["provider state unavailable".to_owned()];
    };
    let mut details = vec![
        format!("dispatch_seen {}", guard.dispatch_seen),
        format!("requester_result_seen {}", guard.requester_result_seen),
        format!("settlement_posted {}", guard.settlement_posted),
    ];
    if let Some(request_id) = &guard.provider_request_id {
        details.push(format!("provider request id {}", request_id));
    }
    if let Some(response) = &guard.settlement_response {
        details.push(format!("settlement response {}", trim_for_report(response)));
    }
    details
}

fn wait_for_job_settled(config: &D3Config, api_token: &str, job_id: &str) -> Result<Value, String> {
    let deadline = Instant::now() + Duration::from_secs(90);
    let encoded = percent_encode_component(job_id);
    let url = config.api_url(&format!("/compute/jobs/{encoded}"));
    while Instant::now() < deadline {
        let job = api_get(&url, api_token)?;
        let status = job.get("status").and_then(Value::as_str).unwrap_or("");
        let delivery_status = job
            .get("delivery")
            .and_then(|delivery| delivery.get("status"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if status == "completed" && delivery_status == "settled" {
            return Ok(job);
        }
        if status == "failed" || delivery_status == "failed_settlement" {
            return Err(format!("job reached failure state: {job}"));
        }
        thread::sleep(Duration::from_secs(2));
    }
    Err("job did not reach completed/settled before timeout".to_owned())
}

fn read_settlement(config: &D3Config, uuid_job_id: &str) -> Result<Value, String> {
    let rows = rest_get(
        config,
        &format!(
            "wire_settlements?job_id=eq.{}&select=*&limit=1",
            percent_encode_component(uuid_job_id)
        ),
    )?;
    rows.as_array()
        .and_then(|rows| rows.first())
        .cloned()
        .ok_or_else(|| format!("wire_settlements row not found for job_id {uuid_job_id}"))
}

fn start_provider_server(config: D3Config) -> Result<ProviderServer, String> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("failed to bind local provider server: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("failed to read local provider port: {error}"))?
        .port();
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to set provider listener nonblocking: {error}"))?;
    let state = Arc::new(Mutex::new(ProviderState::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let thread_state = Arc::clone(&state);
    let thread_stop = Arc::clone(&stop);
    let join = thread::spawn(move || {
        while !thread_stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let state = Arc::clone(&thread_state);
                    let config = config.clone();
                    thread::spawn(move || {
                        let _ = handle_provider_connection(stream, state, config);
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(_) => break,
            }
        }
    });
    Ok(ProviderServer {
        port,
        state,
        stop,
        join: Some(join),
    })
}

fn handle_provider_connection(
    mut stream: TcpStream,
    state: Arc<Mutex<ProviderState>>,
    config: D3Config,
) -> Result<(), String> {
    let request = read_http_request(&mut stream)?;
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => write_http_json(&mut stream, 200, json!({ "ok": true })),
        ("POST", "/v1/compute/job-result") => {
            let mut guard = state
                .lock()
                .map_err(|_| "provider state mutex poisoned".to_owned())?;
            guard.requester_result_seen = true;
            drop(guard);
            write_http_json(&mut stream, 200, json!({ "status": "received" }))
        }
        ("POST", "/v1/compute/job-dispatch") => {
            let body: Value = serde_json::from_slice(&request.body)
                .map_err(|error| format!("dispatch body was not JSON: {error}"))?;
            {
                let mut guard = state
                    .lock()
                    .map_err(|_| "provider state mutex poisoned".to_owned())?;
                guard.dispatch_seen = true;
            }
            let background_state = Arc::clone(&state);
            thread::spawn(move || {
                if let Err(error) = execute_dispatch(body, background_state.clone(), config) {
                    if let Ok(mut guard) = background_state.lock() {
                        guard.error = Some(error);
                    }
                }
            });
            write_http_json(
                &mut stream,
                202,
                json!({ "status": "accepted", "peer_queue_depth": 0 }),
            )
        }
        _ => write_http_json(&mut stream, 404, json!({ "error": "not_found" })),
    }
}

fn execute_dispatch(
    dispatch: Value,
    state: Arc<Mutex<ProviderState>>,
    config: D3Config,
) -> Result<(), String> {
    let start = Instant::now();
    let job_id = dispatch
        .get("job_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing job_id".to_owned())?
        .to_owned();
    let model_id = dispatch
        .get("model_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing model_id".to_owned())?
        .to_owned();
    let callback_url = dispatch
        .get("callback_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing callback_url".to_owned())?
        .to_owned();
    let callback_token = dispatch
        .get("callback_auth")
        .and_then(|auth| auth.get("token"))
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing callback_auth.token".to_owned())?
        .to_owned();
    let requester_callback_url = dispatch
        .get("requester_callback_url")
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing requester_callback_url".to_owned())?
        .to_owned();
    let requester_delivery_jwt = dispatch
        .get("requester_delivery_jwt")
        .and_then(Value::as_str)
        .ok_or_else(|| "dispatch missing requester_delivery_jwt".to_owned())?
        .to_owned();
    let messages = dispatch
        .get("messages")
        .cloned()
        .ok_or_else(|| "dispatch missing messages".to_owned())?;
    let max_tokens = dispatch
        .get("max_tokens")
        .and_then(Value::as_u64)
        .unwrap_or(u64::from(config.max_tokens));
    let temperature = dispatch
        .get("temperature")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);

    let completion = chat_completion(&config, &model_id, messages, max_tokens, temperature)?;
    if let Ok(mut guard) = state.lock() {
        guard.provider_request_id = completion.provider_request_id.clone();
    }
    let input_tokens = estimate_tokens(D3_PROMPT).max(1);
    let output_tokens = estimate_tokens(&completion.text).max(1);

    post_requester_result(
        &requester_callback_url,
        &requester_delivery_jwt,
        json!({
            "type": "success",
            "job_id": job_id,
            "result": {
                "content": completion.text,
                "model_used": model_id,
                "finish_reason": "stop"
            }
        }),
    )?;
    if let Ok(mut guard) = state.lock() {
        guard.requester_result_seen = true;
    }

    let latency_ms = start.elapsed().as_millis().min(i64::MAX as u128) as i64;
    let settlement_response = post_settlement(
        &callback_url,
        &callback_token,
        json!({
            "type": "success",
            "job_id": job_id,
            "result": {
                "input_tokens": input_tokens,
                "output_tokens": output_tokens,
                "model_used": model_id,
                "latency_ms": latency_ms,
                "finish_reason": "stop"
            }
        }),
    )?;
    if let Ok(mut guard) = state.lock() {
        guard.settlement_posted = true;
        guard.settlement_response = Some(settlement_response);
    }
    Ok(())
}

#[derive(Debug)]
struct ChatCompletion {
    text: String,
    provider_request_id: Option<String>,
}

fn chat_completion(
    config: &D3Config,
    model_id: &str,
    messages: Value,
    max_tokens: u64,
    temperature: f64,
) -> Result<ChatCompletion, String> {
    let provider = config.llm_provider.clone().with_model_id(model_id);
    let (text, provider_request_id) = provider.chat_completion(
        messages,
        max_tokens,
        temperature,
        "agent-wire-substrate D3 validation",
    )?;
    Ok(ChatCompletion {
        text,
        provider_request_id,
    })
}

fn post_requester_result(url: &str, jwt: &str, payload: Value) -> Result<(), String> {
    let auth_header = format!("Bearer {jwt}");
    let response = ureq::post(url)
        .set("Authorization", &auth_header)
        .set("Content-Type", "application/json")
        .send_json(payload);
    parse_ureq_response(response).map(|_| ())
}

fn post_settlement(url: &str, token: &str, payload: Value) -> Result<String, String> {
    let auth_header = format!("Bearer {token}");
    let response = ureq::post(url)
        .set("Authorization", &auth_header)
        .set("Content-Type", "application/json")
        .send_json(payload);
    parse_ureq_response(response).map(|value| value.to_string())
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(15)))
        .map_err(|error| format!("set read timeout failed: {error}"))?;
    let mut bytes = Vec::new();
    let mut buf = [0u8; 1024];
    let header_end;
    loop {
        let read = stream
            .read(&mut buf)
            .map_err(|error| format!("read request failed: {error}"))?;
        if read == 0 {
            return Err("connection closed before headers".to_owned());
        }
        bytes.extend_from_slice(&buf[..read]);
        if let Some(pos) = find_header_end(&bytes) {
            header_end = pos;
            break;
        }
        if bytes.len() > 64 * 1024 {
            return Err("request headers too large".to_owned());
        }
    }
    let header_text = String::from_utf8_lossy(&bytes[..header_end]).to_string();
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "missing request line".to_owned())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| "missing method".to_owned())?
        .to_owned();
    let path = request_parts
        .next()
        .ok_or_else(|| "missing path".to_owned())?
        .split('?')
        .next()
        .unwrap_or("/")
        .to_owned();
    let mut content_length = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = value
                .trim()
                .parse::<usize>()
                .map_err(|error| format!("invalid content-length: {error}"))?;
        }
    }
    let body_start = header_end + 4;
    while bytes.len() < body_start + content_length {
        let read = stream
            .read(&mut buf)
            .map_err(|error| format!("read body failed: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..read]);
    }
    let body_end = (body_start + content_length).min(bytes.len());
    Ok(HttpRequest {
        method,
        path,
        body: bytes[body_start..body_end].to_vec(),
    })
}

fn write_http_json(stream: &mut TcpStream, status: u16, body: Value) -> Result<(), String> {
    let status_text = match status {
        200 => "OK",
        202 => "Accepted",
        404 => "Not Found",
        _ => "OK",
    };
    let body = body.to_string();
    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("write response failed: {error}"))
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn api_get(url: &str, api_token: &str) -> Result<Value, String> {
    let auth_header = format!("Bearer {api_token}");
    let response = ureq::get(url).set("Authorization", &auth_header).call();
    parse_ureq_response(response)
}

fn api_post(url: &str, api_token: &str, payload: Value) -> Result<Value, String> {
    let auth_header = format!("Bearer {api_token}");
    let response = ureq::post(url)
        .set("Authorization", &auth_header)
        .set("Content-Type", "application/json")
        .send_json(payload);
    parse_ureq_response(response)
}

fn rest_get(config: &D3Config, path: &str) -> Result<Value, String> {
    let url = config.rest_url(path);
    let auth_header = format!("Bearer {}", config.service_role_key);
    let response = ureq::get(&url)
        .set("apikey", &config.service_role_key)
        .set("Authorization", &auth_header)
        .call();
    parse_ureq_response(response)
}

fn rest_post(config: &D3Config, path: &str, payload: Value) -> Result<Value, String> {
    let url = config.rest_url(path);
    let auth_header = format!("Bearer {}", config.service_role_key);
    let response = ureq::post(&url)
        .set("apikey", &config.service_role_key)
        .set("Authorization", &auth_header)
        .set("Content-Type", "application/json")
        .set("Prefer", "return=representation")
        .send_json(payload);
    parse_ureq_response(response)
}

fn parse_ureq_response(response: Result<ureq::Response, ureq::Error>) -> Result<Value, String> {
    match response {
        Ok(response) => response
            .into_json()
            .map_err(|error| format!("response was not valid JSON: {error}")),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(format!("HTTP {status}: {}", trim_for_report(&body)))
        }
        Err(error) => Err(format!("request failed: {error}")),
    }
}

fn sanitize_node_handle(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "d3-kramer-node".to_owned()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn percent_encode_component(input: &str) -> String {
    let mut out = String::new();
    for byte in input.as_bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
        if is_unreserved {
            out.push(*byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn estimate_tokens(text: &str) -> i64 {
    ((text.chars().count() as i64) / 4).max(1)
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn trim_for_report(input: &str) -> String {
    const MAX: usize = 500;
    let compact = input.replace('\n', " ");
    if compact.chars().count() <= MAX {
        return compact;
    }
    let mut output = compact.chars().take(MAX).collect::<String>();
    output.push_str("...");
    output
}

fn passed_step(name: &str, proves: &str, details: Vec<String>) -> D3Subtest {
    D3Subtest {
        name: name.to_owned(),
        proves: proves.to_owned(),
        status: D3Status::Passed,
        details,
    }
}

fn failed_step(name: &str, proves: &str, reason: String) -> D3Subtest {
    D3Subtest {
        name: name.to_owned(),
        proves: proves.to_owned(),
        status: D3Status::Failed { reason },
        details: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encoding_handles_handle_paths() {
        assert_eq!(
            percent_encode_component("playful/123/45"),
            "playful%2F123%2F45"
        );
    }

    #[test]
    fn node_handle_sanitizer_keeps_stable_ascii_shape() {
        assert_eq!(
            sanitize_node_handle("D3 Kramer Provider 123"),
            "d3-kramer-provider-123"
        );
    }

    #[test]
    fn fill_retry_classifier_catches_tunnel_propagation_errors() {
        assert!(fill_error_is_retryable(
            "HTTP 503: {\"error\":\"provider_unreachable\",\"detail\":{\"message\":\"http_530\"}}"
        ));
        assert!(fill_error_is_retryable("HTTP 502: edge unavailable"));
        assert!(fill_error_is_retryable("HTTP 504: timeout"));
        assert!(!fill_error_is_retryable("HTTP 400: malformed job"));
    }

    #[test]
    fn fill_retry_policy_enforces_attempt_and_timeout_bounds() {
        let policy = D3FillRetryPolicy {
            timeout_secs: 30,
            max_attempts: 2,
            backoff_millis: 250,
            max_jitter_millis: 0,
        };
        let started_at = Instant::now();

        assert_eq!(
            policy
                .next_sleep(
                    "HTTP 503: provider_unreachable",
                    1,
                    started_at,
                    "stable-key"
                )
                .unwrap(),
            Duration::from_millis(250)
        );
        assert!(policy
            .next_sleep(
                "HTTP 503: provider_unreachable",
                2,
                started_at,
                "stable-key"
            )
            .is_none());
    }

    #[test]
    fn fill_retry_jitter_is_deterministic_and_bounded() {
        let policy = D3FillRetryPolicy {
            timeout_secs: 30,
            max_attempts: 3,
            backoff_millis: 500,
            max_jitter_millis: 100,
        };

        let first = policy.retry_sleep_duration("stable-key", 1);
        let second = policy.retry_sleep_duration("stable-key", 1);
        assert_eq!(first, second);
        assert!(first >= Duration::from_millis(500));
        assert!(first <= Duration::from_millis(600));
    }
}
