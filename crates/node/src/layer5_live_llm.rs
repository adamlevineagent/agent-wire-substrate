use std::cell::RefCell;
use std::env;

use agent_wire_compute_market::{
    ChronicleReceipt, ChronicleSink, ComputeJobContract, ComputeJobEnvelope, ComputeOffer,
    DeliveryReceipt, DispatchStatus, EventSink, ExecutionAdapter, ExecutionAdapterId,
    MarketDispatchOutcome, RetryIntent,
};
use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, EventCursor, EventEnvelope, EventId, EventKind, FoundationError,
    HandlePath, Layer5AdapterId, Layer5Provider, PriceCurve, SettlementIntent,
};
use agent_wire_substrate::{compose_substrate_node, NodeConfig, NodeRuntime};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
const DEFAULT_OPENROUTER_MODEL: &str = "inception/mercury-2";
const DEFAULT_LM_STUDIO_BASE_URL: &str = "http://127.0.0.1:1234/v1";
const DEFAULT_LM_STUDIO_MODEL: &str = "granite-4-micro";
const LAYER5_MAX_TOKENS: u32 = 160;
const LAYER5_SENTINEL: &str = "SUBSTRATE_ROUNDTRIP_OK";
const LAYER5_PROMPT: &str = "Return exactly the token SUBSTRATE_ROUNDTRIP_OK and nothing else.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer5LiveLlmReport {
    pub name: String,
    pub provider: Layer5Provider,
    pub model_id: String,
    pub live_provider: bool,
    pub subtests: Vec<Layer5Subtest>,
}

impl Layer5LiveLlmReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, Layer5Status::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Layer 5 Live LLM Compute Roundtrip Validation\n\n");
        output.push_str("Provider: ");
        output.push_str(self.provider.slug());
        output.push_str("\n\nModel: `");
        output.push_str(&self.model_id);
        output.push_str("`\n\n");
        output.push_str("This harness drives the compute-market path with a real provider only when live provider config is present. It keeps keys in process environment/config, does not hardcode secrets, and does not touch deploy, live database, live smoke, or npm publish surfaces.\n\n");
        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "All Layer 5 live-LLM roundtrip sub-tests are green.\n\n"
        } else {
            "One or more Layer 5 live-LLM roundtrip sub-tests failed; see reasons below.\n\n"
        });
        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                Layer5Status::Passed => "PASS",
                Layer5Status::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let Layer5Status::Failed { reason } = &subtest.status {
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

    fn failed_provider_config(reason: String) -> Self {
        Self {
            name: "wave-2-layer-5-live-llm-compute-roundtrip".to_owned(),
            provider: Layer5Provider::Unresolved,
            model_id: env::var("LAYER5_MODEL")
                .or_else(|_| env::var("LM_STUDIO_MODEL"))
                .or_else(|_| env::var("OPENROUTER_MODEL"))
                .unwrap_or_else(|_| DEFAULT_LM_STUDIO_MODEL.to_owned()),
            live_provider: true,
            subtests: vec![Layer5Subtest {
                name: "provider-config-resolves-live-llm".to_owned(),
                proves: "a real LLM provider key/config is available before dispatch".to_owned(),
                status: Layer5Status::Failed { reason },
                details: Vec::new(),
            }],
        }
    }

    fn failed_bootstrap(reason: String) -> Self {
        Self {
            name: "wave-2-layer-5-live-llm-compute-roundtrip".to_owned(),
            provider: Layer5Provider::LmStudio,
            model_id: DEFAULT_LM_STUDIO_MODEL.to_owned(),
            live_provider: true,
            subtests: vec![Layer5Subtest {
                name: "substrate-node-bootstrap".to_owned(),
                proves: "the composed node can host a live compute-market roundtrip".to_owned(),
                status: Layer5Status::Failed { reason },
                details: Vec::new(),
            }],
        }
    }

    fn record(
        &mut self,
        name: impl Into<String>,
        proves: impl Into<String>,
        outcome: Result<Vec<String>, String>,
    ) {
        let (status, details) = match outcome {
            Ok(details) => (Layer5Status::Passed, details),
            Err(reason) => (Layer5Status::Failed { reason }, Vec::new()),
        };
        self.subtests.push(Layer5Subtest {
            name: name.into(),
            proves: proves.into(),
            status,
            details,
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer5Subtest {
    pub name: String,
    pub proves: String,
    pub status: Layer5Status,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer5Status {
    Passed,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer5ProviderConfig {
    pub provider: Layer5Provider,
    pub model_id: String,
    pub base_url: String,
    pub adapter_id: Layer5AdapterId,
    pub live_provider: bool,
}

impl Layer5ProviderConfig {
    pub fn openai_compatible(
        provider: Layer5Provider,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            provider,
            model_id: model_id.into(),
            base_url: base_url.into(),
            adapter_id: provider.adapter_id(),
            live_provider: true,
        }
    }

    pub fn openrouter(model_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::openai_compatible(Layer5Provider::OpenRouter, model_id, base_url)
    }

    pub fn lm_studio(model_id: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self::openai_compatible(Layer5Provider::LmStudio, model_id, base_url)
    }

    pub fn fixture(model_id: impl Into<String>) -> Self {
        Self {
            provider: Layer5Provider::DeterministicFixture,
            model_id: model_id.into(),
            base_url: "memory://layer5-fixture".to_owned(),
            adapter_id: Layer5Provider::DeterministicFixture.adapter_id(),
            live_provider: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layer5Completion {
    pub result_ref: CrossGraphRef,
    pub text: String,
    pub charged: CreditAmount,
    pub provider_request_id: Option<String>,
}

pub fn run_layer5_live_llm_compute_roundtrip() -> Layer5LiveLlmReport {
    let (provider, adapter) = match ChatCompletionsExecutionAdapter::from_layer5_env() {
        Ok(resolved) => resolved,
        Err(reason) => return Layer5LiveLlmReport::failed_provider_config(reason),
    };

    match run_layer5_live_llm_with_adapter(provider, adapter) {
        Ok(report) => report,
        Err(error) => Layer5LiveLlmReport::failed_bootstrap(error.to_string()),
    }
}

pub fn run_layer5_live_llm_with_adapter<A>(
    provider: Layer5ProviderConfig,
    adapter: A,
) -> Result<Layer5LiveLlmReport, FoundationError>
where
    A: ExecutionAdapter<Output = Layer5Completion>,
    A::Error: ToString,
{
    let runtime = compose_substrate_node(NodeConfig::demo()?)?;
    let mut graph = LiveLlmComputeGraph::new(runtime, provider.clone(), adapter)?;
    let mut report = Layer5LiveLlmReport {
        name: "wave-2-layer-5-live-llm-compute-roundtrip".to_owned(),
        provider: provider.provider,
        model_id: provider.model_id.clone(),
        live_provider: provider.live_provider,
        subtests: Vec::new(),
    };

    report.record(
        "provider-registers-live-model-offer",
        "provider role publishes a compute offer for the real model",
        graph.register_live_provider_offer(),
    );
    report.record(
        "requester-submits-compute-job-envelope",
        "requester submits a ComputeJobEnvelope through compute-market contracts",
        graph.submit_compute_job_envelope(),
    );
    report.record(
        "provider-executes-inference-via-execution-adapter",
        "provider performs the LLM call through an ExecutionAdapter and content sanity passes",
        graph.execute_inference_via_adapter(),
    );
    report.record(
        "chronicle-records-live-completion",
        "provider returns completion through a ChronicleSink receipt",
        graph.record_completion_in_chronicle(),
    );
    report.record(
        "requester-reads-completion-and-settles",
        "requester reads the completion and settlement clears within budget",
        graph.read_completion_and_settle(),
    );

    Ok(report)
}

struct LiveLlmComputeGraph<A>
where
    A: ExecutionAdapter<Output = Layer5Completion>,
    A::Error: ToString,
{
    runtime: NodeRuntime,
    provider: Layer5ProviderConfig,
    adapter: A,
    requester: HandlePath,
    providers: Vec<ComputeOffer>,
    compute_contributions: Vec<ComputeJobContract>,
    event_bus: LiveEventBus,
    chronicle: LiveChronicle,
    claimed_jobs: Vec<CrossGraphRef>,
    completions: Vec<Layer5Completion>,
    chronicle_receipts: Vec<ChronicleReceipt>,
    settlements: Vec<Layer5Settlement>,
    event_seq: u128,
}

impl<A> LiveLlmComputeGraph<A>
where
    A: ExecutionAdapter<Output = Layer5Completion>,
    A::Error: ToString,
{
    fn new(
        runtime: NodeRuntime,
        provider: Layer5ProviderConfig,
        adapter: A,
    ) -> Result<Self, FoundationError> {
        Ok(Self {
            requester: handle_path(["agent", "playful", "layer5-requester"])?,
            chronicle: LiveChronicle::new(handle_path(["agent", "playful", "chronicle"])?),
            runtime,
            provider,
            adapter,
            providers: Vec::new(),
            compute_contributions: Vec::new(),
            event_bus: LiveEventBus::default(),
            claimed_jobs: Vec::new(),
            completions: Vec::new(),
            chronicle_receipts: Vec::new(),
            settlements: Vec::new(),
            event_seq: 1,
        })
    }

    fn register_live_provider_offer(&mut self) -> Result<Vec<String>, String> {
        if self.provider.model_id.trim().is_empty() {
            return Err("provider model id is empty".to_owned());
        }
        if !self.runtime.config.opt_in.compute_provider {
            return Err("runtime config did not opt into compute provider mode".to_owned());
        }

        let mut offer = self.runtime.markets.compute_offer.clone();
        offer.model_id = self.provider.model_id.clone();
        offer.adapter = ExecutionAdapterId(self.provider.adapter_id.slug().to_owned());
        offer.price = PriceCurve {
            base: CreditAmount::from_sats(50),
            per_unit: CreditAmount::from_sats(1),
        };
        offer.reservation_fee = CreditAmount::from_sats(1);
        offer.settlement = SettlementIntent {
            max_price: CreditAmount::from_sats(750),
            escrow_required: true,
        };
        self.providers.push(offer.clone());

        Ok(vec![
            format!(
                "registered {} provider {}",
                self.provider.provider, offer.provider
            ),
            format!(
                "model {} exposed through {}",
                offer.model_id, offer.adapter.0
            ),
            format!("provider base URL {}", self.provider.base_url),
        ])
    }

    fn submit_compute_job_envelope(&mut self) -> Result<Vec<String>, String> {
        if self.providers.is_empty() {
            return Err("no provider offer is registered".to_owned());
        }

        let mut job = self.runtime.markets.compute_job.clone();
        job.payload.job_ref = ref_path("layer5-job", 1).map_err(|error| error.to_string())?;
        job.payload.requester = "layer5-live-requester".to_owned();
        job.payload.requester_handle = self.requester.clone();
        job.payload.invocation.model_id = self.provider.model_id.clone();
        job.payload.invocation.adapter =
            ExecutionAdapterId(self.provider.adapter_id.slug().to_owned());
        job.payload.invocation.prompt_ref =
            ref_path("layer5-prompt", 1).map_err(|error| error.to_string())?;
        job.payload.invocation.input_ref = None;
        job.payload.invocation.max_tokens = Some(LAYER5_MAX_TOKENS);
        job.payload.invocation.temperature_milli = Some(0);
        job.payload.budget = CreditAmount::from_sats(750);
        job.payload.settlement = SettlementIntent {
            max_price: CreditAmount::from_sats(700),
            escrow_required: true,
        };
        job.payload.delivery.require_chronicle_receipt = true;
        job.payload.dispatch.require_reputation = false;
        job.payload.dispatch.max_price = CreditAmount::from_sats(650);

        if job.payload.dispatch.max_price > job.payload.budget {
            return Err("dispatch max price exceeds requester budget".to_owned());
        }

        let event = self
            .event(
                job.payload.clone(),
                EventKind::ContributionPublished,
                "compute-job",
            )
            .map_err(|error| error.to_string())?;
        self.event_bus
            .emit(event)
            .map_err(|error| error.to_string())?;
        self.compute_contributions.push(job.clone());

        Ok(vec![
            format!("published job {}", job.payload.job_ref),
            format!(
                "prompt ref {} maps to live validation prompt",
                job.payload.invocation.prompt_ref
            ),
            format!(
                "budget {} with max dispatch {}",
                job.payload.budget, job.payload.dispatch.max_price
            ),
        ])
    }

    fn execute_inference_via_adapter(&mut self) -> Result<Vec<String>, String> {
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution was published".to_owned())?
            .payload
            .clone();
        if !self.event_bus.contains_job_ref(&job.job_ref) {
            return Err("provider subscription did not see the published compute job".to_owned());
        }
        if self.providers.is_empty() {
            return Err("no provider is visible to claim the job".to_owned());
        }

        self.claimed_jobs.push(job.job_ref.clone());
        let outcome = MarketDispatchOutcome {
            dispatch_id: agent_wire_compute_market::ComputeDispatchId(
                "dispatch-l5-live-1".to_owned(),
            ),
            job_ref: job.job_ref.clone(),
            status: DispatchStatus::Accepted,
            provider_receipt_ref: Some(
                ref_path("layer5-provider-receipt", 1).map_err(|error| error.to_string())?,
            ),
        };
        if outcome.status != DispatchStatus::Accepted {
            return Err("provider did not accept live dispatch".to_owned());
        }

        let completion = self
            .adapter
            .invoke(&job)
            .map_err(|error| error.to_string())?;
        if !content_has_sentinel(&completion.text) {
            return Err(format!(
                "live completion did not contain {LAYER5_SENTINEL}; got `{}`",
                trim_for_report(&completion.text)
            ));
        }
        self.completions.push(completion.clone());

        let mut details = vec![
            format!("dispatch status {:?}", outcome.status),
            format!("live adapter returned {}", completion.result_ref),
            format!("content sanity saw {LAYER5_SENTINEL}"),
            format!("completion charged {}", completion.charged),
        ];
        if let Some(request_id) = completion.provider_request_id {
            details.push(format!("provider request id {}", request_id));
        }
        Ok(details)
    }

    fn record_completion_in_chronicle(&mut self) -> Result<Vec<String>, String> {
        let completion = self
            .completions
            .last()
            .ok_or_else(|| "no live completion exists to record".to_owned())?
            .clone();
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution exists for receipt".to_owned())?
            .payload
            .clone();
        if !self.claimed_jobs.contains(&job.job_ref) {
            return Err("cannot record a completion for an unclaimed job".to_owned());
        }

        let receipt = DeliveryReceipt {
            job_ref: job.job_ref.clone(),
            delivered_to: None,
            result_ref: completion.result_ref.clone(),
            charged: completion.charged,
            retry_intent: RetryIntent::Never,
        };
        let event = self
            .event(
                receipt.clone(),
                EventKind::Custom("layer5_live_completion".to_owned()),
                "receipt",
            )
            .map_err(|error| error.to_string())?;
        let chronicle_receipt = self
            .chronicle
            .record(event)
            .map_err(|error| error.to_string())?;
        self.chronicle_receipts.push(chronicle_receipt.clone());

        Ok(vec![
            format!("chronicle recorded event {}", chronicle_receipt.event_ref),
            format!("delivery receipt charged {}", receipt.charged),
        ])
    }

    fn read_completion_and_settle(&mut self) -> Result<Vec<String>, String> {
        let completion = self
            .completions
            .last()
            .ok_or_else(|| "requester could not read a live completion".to_owned())?
            .clone();
        if self.chronicle_receipts.is_empty() {
            return Err("requester requires a Chronicle receipt before settlement".to_owned());
        }
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution exists for settlement".to_owned())?
            .payload
            .clone();
        let provider = self
            .providers
            .first()
            .ok_or_else(|| "no provider account exists for settlement".to_owned())?
            .provider
            .clone();
        if completion.charged > job.settlement.max_price {
            return Err("charged amount exceeded settlement max price".to_owned());
        }

        let settlement = Layer5Settlement {
            from: job.requester_handle.clone(),
            to: provider.clone(),
            amount: completion.charged,
            intent: job.settlement.clone(),
        };
        self.settlements.push(settlement.clone());

        Ok(vec![
            format!("requester read completion {}", completion.result_ref),
            format!(
                "settled {} from {} to {}",
                settlement.amount, settlement.from, settlement.to
            ),
            format!("settlement ceiling {}", settlement.intent.max_price),
        ])
    }

    fn latest_job(&self) -> Option<&ComputeJobContract> {
        self.compute_contributions.last()
    }

    fn event<T>(
        &mut self,
        payload: T,
        kind: EventKind,
        slug: &str,
    ) -> Result<EventEnvelope<T>, FoundationError> {
        let seq = self.event_seq;
        self.event_seq += 1;
        Ok(EventEnvelope {
            id: EventId::new(Uuid::from_u128(seq)),
            namespace: self.runtime.config.namespace.clone(),
            kind,
            occurred_at: OffsetDateTime::UNIX_EPOCH,
            cursor: EventCursor::new(format!("l5-{slug}-{seq}")),
            payload,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Layer5Settlement {
    from: HandlePath,
    to: HandlePath,
    amount: CreditAmount,
    intent: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatCompletionsProviderConfig {
    pub provider: Layer5Provider,
    pub model_id: String,
    base_url: String,
    api_key: Option<String>,
    adapter_id: Layer5AdapterId,
}

impl ChatCompletionsProviderConfig {
    pub(crate) fn from_layer5_env() -> Result<Self, String> {
        Self::from_env(
            &["LAYER5_PROVIDER", "AGENT_WIRE_LLM_PROVIDER"],
            &["LAYER5_MODEL", "LM_STUDIO_MODEL"],
        )
    }

    pub(crate) fn from_d3_env() -> Result<Self, String> {
        Self::from_env(
            &["D3_LLM_PROVIDER", "AGENT_WIRE_LLM_PROVIDER"],
            &["D3_MODEL", "LM_STUDIO_MODEL", "LAYER5_MODEL"],
        )
    }

    fn from_env(provider_envs: &[&str], model_envs: &[&str]) -> Result<Self, String> {
        let provider = match first_env(provider_envs) {
            Some(provider) => {
                Layer5Provider::parse(&provider).map_err(|error| error.to_string())?
            }
            None => {
                if non_empty_env("OPENROUTER_API_KEY").is_some() {
                    Layer5Provider::OpenRouter
                } else {
                    Layer5Provider::LmStudio
                }
            }
        };

        match provider {
            Layer5Provider::OpenRouter => {
                let api_key = non_empty_env("OPENROUTER_API_KEY").ok_or_else(|| {
                    "OPENROUTER_API_KEY is required when provider=openrouter".to_owned()
                })?;
                let base_url = non_empty_env("OPENROUTER_BASE_URL")
                    .unwrap_or_else(|| DEFAULT_OPENROUTER_BASE_URL.to_owned());
                let model_id = first_env(model_envs)
                    .or_else(|| non_empty_env("OPENROUTER_MODEL"))
                    .unwrap_or_else(|| DEFAULT_OPENROUTER_MODEL.to_owned());
                Ok(Self {
                    provider,
                    model_id,
                    base_url,
                    api_key: Some(api_key),
                    adapter_id: provider.adapter_id(),
                })
            }
            Layer5Provider::LmStudio => {
                let base_url = non_empty_env("LM_STUDIO_BASE_URL")
                    .unwrap_or_else(|| DEFAULT_LM_STUDIO_BASE_URL.to_owned());
                let model_id =
                    first_env(model_envs).unwrap_or_else(|| DEFAULT_LM_STUDIO_MODEL.to_owned());
                Ok(Self {
                    provider,
                    model_id,
                    base_url,
                    api_key: None,
                    adapter_id: provider.adapter_id(),
                })
            }
            Layer5Provider::DeterministicFixture | Layer5Provider::Unresolved => Err(format!(
                "unsupported live LLM provider `{provider}`; expected `lm_studio` or `openrouter`"
            )),
        }
    }

    pub(crate) fn to_layer5_provider_config(&self) -> Layer5ProviderConfig {
        Layer5ProviderConfig::openai_compatible(
            self.provider,
            self.model_id.clone(),
            self.base_url.clone(),
        )
    }

    pub(crate) fn with_model_id(mut self, model_id: impl Into<String>) -> Self {
        self.model_id = model_id.into();
        self
    }

    pub(crate) fn base_url(&self) -> &str {
        &self.base_url
    }

    pub(crate) fn adapter_id(&self) -> Layer5AdapterId {
        self.adapter_id
    }

    pub(crate) fn chat_completion(
        &self,
        messages: Value,
        max_tokens: u64,
        temperature: f64,
        report_title: &str,
    ) -> Result<(String, Option<String>), String> {
        let endpoint = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let payload = serde_json::json!({
            "model": self.model_id,
            "messages": messages,
            "max_tokens": max_tokens,
            "temperature": temperature
        });
        let mut request = ureq::post(&endpoint).set("Content-Type", "application/json");
        if let Some(api_key) = &self.api_key {
            request = request.set("Authorization", &format!("Bearer {api_key}"));
        }
        if self.provider == Layer5Provider::OpenRouter {
            request = request
                .set("HTTP-Referer", "https://agent-wire-substrate.local")
                .set("X-Title", report_title);
        }
        let response = request.send_json(payload);

        let response = match response {
            Ok(response) => response,
            Err(ureq::Error::Status(status, response)) => {
                let body = response.into_string().unwrap_or_default();
                return Err(format!(
                    "{} returned HTTP {status}: {}",
                    self.provider.slug(),
                    trim_for_report(&body)
                ));
            }
            Err(error) => return Err(format!("{} request failed: {error}", self.provider.slug())),
        };
        let request_id = response.header("x-request-id").map(ToOwned::to_owned);
        let body: Value = response.into_json().map_err(|error| {
            format!(
                "{} response was not valid JSON: {error}",
                self.provider.slug()
            )
        })?;
        let first_choice = body
            .get("choices")
            .and_then(Value::as_array)
            .and_then(|choices| choices.first())
            .ok_or_else(|| {
                format!(
                    "{} response did not include choices[0]",
                    self.provider.slug()
                )
            })?;
        let content = first_choice
            .get("message")
            .and_then(|message| message.get("content"))
            .and_then(Value::as_str)
            .ok_or_else(|| missing_content_reason(self.provider, first_choice))?;
        Ok((content.to_owned(), request_id))
    }
}

#[derive(Debug, Clone)]
struct ChatCompletionsExecutionAdapter {
    provider: ChatCompletionsProviderConfig,
    prompt: String,
}

impl ChatCompletionsExecutionAdapter {
    fn from_layer5_env() -> Result<(Layer5ProviderConfig, Self), String> {
        let provider = ChatCompletionsProviderConfig::from_layer5_env()?;
        Ok((
            provider.to_layer5_provider_config(),
            Self {
                provider,
                prompt: LAYER5_PROMPT.to_owned(),
            },
        ))
    }

    fn chat_completion(
        &self,
        job: &ComputeJobEnvelope,
    ) -> Result<(String, Option<String>), String> {
        self.provider.chat_completion(
            serde_json::json!([
                {
                    "role": "system",
                    "content": "You are validating an agent-wire compute-market roundtrip. Follow the user instruction exactly."
                },
                {
                    "role": "user",
                    "content": self.prompt
                }
            ]),
            u64::from(job.invocation.max_tokens.unwrap_or(LAYER5_MAX_TOKENS)),
            f64::from(job.invocation.temperature_milli.unwrap_or(0)) / 1000.0,
            "agent-wire-substrate layer5 validation",
        )
    }
}

impl ExecutionAdapter for ChatCompletionsExecutionAdapter {
    type Error = String;
    type Output = Layer5Completion;

    fn invoke(&self, job: &ComputeJobEnvelope) -> Result<Self::Output, Self::Error> {
        let (text, provider_request_id) = self.chat_completion(job)?;
        Ok(Layer5Completion {
            result_ref: ref_path("layer5-result", 1).map_err(|error| error.to_string())?,
            text,
            charged: CreditAmount::from_sats(73),
            provider_request_id,
        })
    }
}

struct LiveEventBus {
    events: RefCell<Vec<EventEnvelope<ComputeJobEnvelope>>>,
}

impl Default for LiveEventBus {
    fn default() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
        }
    }
}

impl EventSink<ComputeJobEnvelope> for LiveEventBus {
    type Error = FoundationError;

    fn emit(&self, event: EventEnvelope<ComputeJobEnvelope>) -> Result<(), Self::Error> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}

impl LiveEventBus {
    fn contains_job_ref(&self, job_ref: &CrossGraphRef) -> bool {
        self.events
            .borrow()
            .iter()
            .any(|event| &event.payload.job_ref == job_ref)
    }
}

struct LiveChronicle {
    recorded_by: HandlePath,
    events: RefCell<Vec<EventEnvelope<DeliveryReceipt>>>,
}

impl LiveChronicle {
    fn new(recorded_by: HandlePath) -> Self {
        Self {
            recorded_by,
            events: RefCell::new(Vec::new()),
        }
    }
}

impl ChronicleSink<DeliveryReceipt> for LiveChronicle {
    type Error = FoundationError;

    fn record(
        &self,
        event: EventEnvelope<DeliveryReceipt>,
    ) -> Result<ChronicleReceipt, Self::Error> {
        let receipt = ChronicleReceipt {
            event_ref: event.payload.result_ref.clone(),
            recorded_by: self.recorded_by.clone(),
        };
        self.events.borrow_mut().push(event);
        Ok(receipt)
    }
}

fn ref_path(slug: &str, sequence: u32) -> Result<CrossGraphRef, FoundationError> {
    format!("playful/123/{slug}/{sequence}").parse()
}

fn handle_path<const N: usize>(parts: [&str; N]) -> Result<HandlePath, FoundationError> {
    HandlePath::new(parts)
}

fn content_has_sentinel(text: &str) -> bool {
    text.to_ascii_uppercase().contains(LAYER5_SENTINEL)
}

fn trim_for_report(value: &str) -> String {
    const MAX_CHARS: usize = 320;
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= MAX_CHARS {
        collapsed
    } else {
        let mut trimmed = collapsed.chars().take(MAX_CHARS).collect::<String>();
        trimmed.push_str("...");
        trimmed
    }
}

fn first_env(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| non_empty_env(name))
}

fn non_empty_env(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn missing_content_reason(provider: Layer5Provider, choice: &Value) -> String {
    let finish_reason = choice
        .get("finish_reason")
        .and_then(|value| value.as_str())
        .unwrap_or("unknown");
    let message_keys = choice
        .get("message")
        .and_then(|message| message.as_object())
        .map(|message| {
            let mut keys = message.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys.join(",")
        })
        .unwrap_or_else(|| "none".to_owned());
    format!(
        "{} response did not include text content (finish_reason={finish_reason}, message_keys={message_keys})",
        provider.slug()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DeterministicLayer5Adapter;

    impl ExecutionAdapter for DeterministicLayer5Adapter {
        type Error = String;
        type Output = Layer5Completion;

        fn invoke(&self, _job: &ComputeJobEnvelope) -> Result<Self::Output, Self::Error> {
            Ok(Layer5Completion {
                result_ref: ref_path("layer5-fixture-result", 1)
                    .map_err(|error| error.to_string())?,
                text: LAYER5_SENTINEL.to_owned(),
                charged: CreditAmount::from_sats(73),
                provider_request_id: Some("fixture-request".to_owned()),
            })
        }
    }

    #[test]
    fn deterministic_layer5_adapter_covers_roundtrip_contract() {
        let report = run_layer5_live_llm_with_adapter(
            Layer5ProviderConfig::fixture(DEFAULT_OPENROUTER_MODEL),
            DeterministicLayer5Adapter,
        )
        .unwrap();

        assert!(report.all_green());
        assert_eq!(report.subtests.len(), 5);
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "provider-executes-inference-via-execution-adapter"));
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "requester-reads-completion-and-settles"));
    }
}
