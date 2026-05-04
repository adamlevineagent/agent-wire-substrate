use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

const DEFAULT_MAINNET_ENDPOINT: &str = "https://newsbleach.com/api/v1";
const DEFAULT_AGENT_NAME: &str = "agent-wire-substrate-node";
const DEFAULT_STATE_RELATIVE_PATH: &str = ".wire-node/state/agent-wire-substrate-node-auth.json";
const AUTH_STATE_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainnetAuthReport {
    pub endpoint: String,
    pub requested_agent_name: String,
    pub state_path: String,
    pub token_source: String,
    pub identity: Option<MainnetIdentity>,
    pub subtests: Vec<MainnetAuthSubtest>,
}

impl MainnetAuthReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, MainnetAuthStatus::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Mainnet Auth Validation\n\n");
        output.push_str("Endpoint: `");
        output.push_str(&self.endpoint);
        output.push_str("`\n\n");
        output.push_str("Requested agent: `");
        output.push_str(&self.requested_agent_name);
        output.push_str("`\n\n");
        output.push_str("State file: `");
        output.push_str(&self.state_path);
        output.push_str("`\n\n");
        output.push_str("Token source: `");
        output.push_str(&self.token_source);
        output.push_str("`\n\n");

        if let Some(identity) = &self.identity {
            output.push_str("## Identity\n\n");
            output.push_str("- name: `");
            output.push_str(&identity.name);
            output.push_str("`\n");
            output.push_str("- handle_path: `");
            output.push_str(&identity.handle_path);
            output.push_str("`\n");
            output.push_str("- pseudonym: `");
            output.push_str(&identity.pseudonym);
            output.push_str("`\n");
            output.push_str("- agent_id: `");
            output.push_str(&identity.agent_id);
            output.push_str("`\n\n");
        }

        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "Mainnet auth is green; the reference client has a validated persisted credential.\n\n"
        } else {
            "Mainnet auth failed closed; see the sub-test reasons below.\n\n"
        });
        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                MainnetAuthStatus::Passed => "PASS",
                MainnetAuthStatus::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let MainnetAuthStatus::Failed { reason } = &subtest.status {
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
pub struct MainnetIdentity {
    pub name: String,
    pub slot: String,
    pub handle_path: String,
    pub pseudonym: String,
    pub agent_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PersistedMainnetCredential {
    pub(crate) endpoint: String,
    pub(crate) api_token: String,
    pub(crate) identity: MainnetIdentity,
    pub(crate) state_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainnetAuthSubtest {
    pub name: String,
    pub proves: String,
    pub status: MainnetAuthStatus,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainnetAuthStatus {
    Passed,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MainnetAuthConfig {
    endpoint: String,
    agent_name: String,
    operator_email: Option<String>,
    device_secret: Option<String>,
    seed_token: Option<String>,
    seed_token_source: Option<String>,
    state_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MainnetAuthState {
    version: u32,
    endpoint: String,
    agent_name: String,
    api_token: String,
    agent_id: String,
    pseudonym: String,
    handle_path: String,
    updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IssuedCredential {
    api_token: String,
    agent_id: String,
    pseudonym: String,
    operator_email: Option<String>,
    source: String,
}

trait MainnetAuthTransport {
    fn validate_token(&self, endpoint: &str, token: &str) -> Result<MainnetIdentity, String>;

    fn resume_agent(
        &self,
        endpoint: &str,
        device_secret: &str,
        agent_name: &str,
        nonce: &str,
    ) -> Result<IssuedCredential, String>;

    fn register_agent(
        &self,
        endpoint: &str,
        agent_name: &str,
        operator_email: &str,
    ) -> Result<IssuedCredential, String>;
}

#[derive(Debug, Default)]
struct UreqMainnetAuthTransport;

pub fn run_mainnet_auth() -> MainnetAuthReport {
    match MainnetAuthConfig::from_env() {
        Ok(config) => run_mainnet_auth_with_transport(config, &UreqMainnetAuthTransport),
        Err(reason) => MainnetAuthReport {
            endpoint: env::var("WIRE_MAINNET_ENDPOINT")
                .unwrap_or_else(|_| DEFAULT_MAINNET_ENDPOINT.to_owned()),
            requested_agent_name: env::var("WIRE_AGENT_NAME")
                .unwrap_or_else(|_| DEFAULT_AGENT_NAME.to_owned()),
            state_path: env::var("WIRE_AUTH_STATE_PATH")
                .unwrap_or_else(|_| format!("~/{DEFAULT_STATE_RELATIVE_PATH}")),
            token_source: "unresolved".to_owned(),
            identity: None,
            subtests: vec![failed_step(
                "auth-config-resolves",
                "the reference client can find endpoint, state path, and at least one credential issuer path",
                reason,
            )],
        },
    }
}

pub(crate) fn load_persisted_mainnet_credential() -> Result<PersistedMainnetCredential, String> {
    let endpoint = env::var("WIRE_MAINNET_ENDPOINT")
        .unwrap_or_else(|_| DEFAULT_MAINNET_ENDPOINT.to_owned())
        .trim_end_matches('/')
        .to_owned();
    let state_path = match env::var("WIRE_AUTH_STATE_PATH") {
        Ok(path) if !path.trim().is_empty() => expand_home(path.trim())?,
        _ => default_state_path()?,
    };
    let text = fs::read_to_string(&state_path).map_err(|error| {
        format!(
            "failed to read persisted mainnet auth state `{}`: {error}. Run `agent-wire-substrate-node auth` first.",
            path_for_report(&state_path)
        )
    })?;
    let state = serde_json::from_str::<MainnetAuthState>(&text)
        .map_err(|error| format!("persisted mainnet auth state is invalid JSON: {error}"))?;
    if state.endpoint != endpoint {
        return Err(format!(
            "persisted auth endpoint `{}` does not match selected endpoint `{endpoint}`",
            state.endpoint
        ));
    }
    let slot = state
        .agent_name
        .trim()
        .to_lowercase()
        .strip_prefix("codex-")
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| state.agent_name.trim().to_lowercase());
    Ok(PersistedMainnetCredential {
        endpoint,
        api_token: state.api_token,
        identity: MainnetIdentity {
            name: state.agent_name,
            slot,
            handle_path: state.handle_path,
            pseudonym: state.pseudonym,
            agent_id: state.agent_id,
        },
        state_path,
    })
}

fn run_mainnet_auth_with_transport(
    config: MainnetAuthConfig,
    transport: &impl MainnetAuthTransport,
) -> MainnetAuthReport {
    let mut report = MainnetAuthReport {
        endpoint: config.endpoint.clone(),
        requested_agent_name: config.agent_name.clone(),
        state_path: path_for_report(&config.state_path),
        token_source: "unresolved".to_owned(),
        identity: None,
        subtests: Vec::new(),
    };

    if let Some(state) = load_state(&config.state_path, &mut report) {
        if state.endpoint == config.endpoint {
            match transport.validate_token(&config.endpoint, &state.api_token) {
                Ok(identity) => {
                    report.token_source = "persisted_state".to_owned();
                    report.identity = Some(identity);
                    report.subtests.push(passed_step(
                        "persisted-token-validates-mainnet",
                        "restart re-auth uses the on-disk credential without issuing a new token",
                        vec![format!(
                            "loaded persisted identity for `{}`",
                            state.handle_path
                        )],
                    ));
                    return report;
                }
                Err(reason) => report.subtests.push(failed_step(
                    "persisted-token-validates-mainnet",
                    "restart re-auth uses the on-disk credential without issuing a new token",
                    format!("persisted token was not accepted: {reason}"),
                )),
            }
        } else {
            report.subtests.push(failed_step(
                "persisted-token-endpoint-match",
                "the cached credential is bound to the selected mainnet endpoint",
                format!(
                    "state endpoint `{}` did not match selected endpoint `{}`",
                    state.endpoint, config.endpoint
                ),
            ));
        }
    }

    let issued = if let Some(seed_token) = config.seed_token.as_deref() {
        match transport.validate_token(&config.endpoint, seed_token) {
            Ok(identity) => {
                report.subtests.push(passed_step(
                    "seed-token-validates-mainnet",
                    "an externally issued mainnet credential can be validated before persistence",
                    vec![format!("validated `{}`", identity.handle_path)],
                ));
                Ok(IssuedCredential {
                    api_token: seed_token.to_owned(),
                    agent_id: identity.agent_id.clone(),
                    pseudonym: identity.pseudonym.clone(),
                    operator_email: None,
                    source: config
                        .seed_token_source
                        .clone()
                        .unwrap_or_else(|| "seed_token".to_owned()),
                })
            }
            Err(reason) => Err(format!("seed token was not accepted: {reason}")),
        }
    } else if let Some(device_secret) = config.device_secret.as_deref() {
        let nonce = format!("agent-wire-substrate-node-{}", Uuid::new_v4());
        transport.resume_agent(&config.endpoint, device_secret, &config.agent_name, &nonce)
    } else if let Some(operator_email) = config.operator_email.as_deref() {
        transport.register_agent(&config.endpoint, &config.agent_name, operator_email)
    } else {
        Err(
            "set WIRE_API_TOKEN, WIRE_API_TOKEN_FILE, WIRE_DEVICE_SECRET, or WIRE_OPERATOR_EMAIL"
                .to_owned(),
        )
    };

    let issued = match issued {
        Ok(issued) => {
            report.token_source = issued.source.clone();
            report.subtests.push(passed_step(
                "mainnet-credential-issued-or-imported",
                "the reference client obtains a live mainnet bearer credential before writing state",
                vec![format!("source `{}` returned agent `{}`", issued.source, issued.agent_id)],
            ));
            issued
        }
        Err(reason) => {
            report.subtests.push(failed_step(
                "mainnet-credential-issued-or-imported",
                "the reference client obtains a live mainnet bearer credential before writing state",
                reason,
            ));
            return report;
        }
    };

    match transport.validate_token(&config.endpoint, &issued.api_token) {
        Ok(identity) => {
            report.identity = Some(identity.clone());
            report.subtests.push(passed_step(
                "issued-token-validates-mainnet",
                "the credential identifies as a real mainnet Wire handle",
                vec![format!("resolved `{}`", identity.handle_path)],
            ));

            let state = MainnetAuthState {
                version: AUTH_STATE_VERSION,
                endpoint: config.endpoint.clone(),
                agent_name: identity.name.clone(),
                api_token: issued.api_token.clone(),
                agent_id: identity.agent_id.clone(),
                pseudonym: identity.pseudonym.clone(),
                handle_path: identity.handle_path.clone(),
                updated_at: utc_now(),
            };
            match persist_state(&config.state_path, &state) {
                Ok(()) => report.subtests.push(passed_step(
                    "credential-persists-to-disk",
                    "the restart path can reload the mainnet credential from the reference-client state file",
                    vec![
                        format!("wrote `{}`", path_for_report(&config.state_path)),
                        "token material is omitted from command output".to_owned(),
                    ],
                )),
                Err(reason) => report.subtests.push(failed_step(
                    "credential-persists-to-disk",
                    "the restart path can reload the mainnet credential from the reference-client state file",
                    reason,
                )),
            }
        }
        Err(reason) => report.subtests.push(failed_step(
            "issued-token-validates-mainnet",
            "the credential identifies as a real mainnet Wire handle",
            reason,
        )),
    }

    report
}

impl MainnetAuthConfig {
    fn from_env() -> Result<Self, String> {
        let endpoint = env::var("WIRE_MAINNET_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_MAINNET_ENDPOINT.to_owned())
            .trim_end_matches('/')
            .to_owned();
        if endpoint.is_empty() {
            return Err("WIRE_MAINNET_ENDPOINT resolved empty".to_owned());
        }

        let agent_name = env::var("WIRE_AGENT_NAME")
            .unwrap_or_else(|_| DEFAULT_AGENT_NAME.to_owned())
            .trim()
            .to_owned();
        if agent_name.is_empty() {
            return Err("WIRE_AGENT_NAME resolved empty".to_owned());
        }

        let state_path = match env::var("WIRE_AUTH_STATE_PATH") {
            Ok(path) if !path.trim().is_empty() => expand_home(path.trim())?,
            _ => default_state_path()?,
        };

        let token_from_env = env::var("WIRE_API_TOKEN")
            .ok()
            .map(|token| token.trim().to_owned())
            .filter(|token| !token.is_empty());
        let token_from_file = match token_from_env {
            Some(token) => Some((token, "WIRE_API_TOKEN".to_owned())),
            None => read_optional_token_file()?,
        };
        let (seed_token, seed_token_source) = match token_from_file {
            Some((token, source)) => (Some(token), Some(source)),
            None => (None, None),
        };

        Ok(Self {
            endpoint,
            agent_name,
            operator_email: env_optional("WIRE_OPERATOR_EMAIL"),
            device_secret: env_optional("WIRE_DEVICE_SECRET"),
            seed_token,
            seed_token_source,
            state_path,
        })
    }
}

impl MainnetAuthTransport for UreqMainnetAuthTransport {
    fn validate_token(&self, endpoint: &str, token: &str) -> Result<MainnetIdentity, String> {
        let auth_header = format!("Bearer {token}");
        let response = ureq::get(&join_endpoint(endpoint, "/me"))
            .set("Authorization", &auth_header)
            .call();
        let body = response_json(response, "GET /me")?;
        parse_identity(&body)
    }

    fn resume_agent(
        &self,
        endpoint: &str,
        device_secret: &str,
        agent_name: &str,
        nonce: &str,
    ) -> Result<IssuedCredential, String> {
        let payload = serde_json::json!({
            "agent_name": agent_name,
            "nonce": nonce,
        });
        let response = ureq::post(&join_endpoint(endpoint, "/wire/agent/resume"))
            .set("Content-Type", "application/json")
            .set("X-Wire-Device-Secret", device_secret)
            .send_json(payload);
        let body = response_json(response, "POST /wire/agent/resume")?;
        let data = envelope_data(&body);
        parse_issued_credential(data, "agent_resume")
    }

    fn register_agent(
        &self,
        endpoint: &str,
        agent_name: &str,
        operator_email: &str,
    ) -> Result<IssuedCredential, String> {
        let payload = serde_json::json!({
            "name": agent_name,
            "operator_email": operator_email,
            "domains": ["agent-wire-substrate"],
            "agent_message": "agent-wire-substrate-node reference client auth bootstrap",
        });
        let response = ureq::post(&join_endpoint(endpoint, "/register"))
            .set("Content-Type", "application/json")
            .send_json(payload);
        let body = response_json(response, "POST /register")?;
        let data = envelope_data(&body);
        parse_issued_credential(data, "register")
    }
}

fn load_state(path: &Path, report: &mut MainnetAuthReport) -> Option<MainnetAuthState> {
    if !path.exists() {
        report.subtests.push(passed_step(
            "auth-state-read",
            "the reference client can check whether a persisted credential exists",
            vec!["no persisted auth state yet".to_owned()],
        ));
        return None;
    }

    match fs::read_to_string(path) {
        Ok(text) => match serde_json::from_str::<MainnetAuthState>(&text) {
            Ok(state) => {
                report.subtests.push(passed_step(
                    "auth-state-read",
                    "the reference client can check whether a persisted credential exists",
                    vec![format!("loaded `{}`", path_for_report(path))],
                ));
                Some(state)
            }
            Err(error) => {
                report.subtests.push(failed_step(
                    "auth-state-read",
                    "the reference client can check whether a persisted credential exists",
                    format!("state JSON was invalid: {error}"),
                ));
                None
            }
        },
        Err(error) => {
            report.subtests.push(failed_step(
                "auth-state-read",
                "the reference client can check whether a persisted credential exists",
                format!("state file could not be read: {error}"),
            ));
            None
        }
    }
}

fn persist_state(path: &Path, state: &MainnetAuthState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create auth state directory: {error}"))?;
    }

    let json = serde_json::to_string_pretty(state)
        .map_err(|error| format!("failed to serialize auth state: {error}"))?;
    let tmp_path = path.with_extension("json.tmp");

    write_private_file(&tmp_path, json.as_bytes())
        .map_err(|error| format!("failed to write auth state: {error}"))?;
    fs::rename(&tmp_path, path).map_err(|error| format!("failed to install auth state: {error}"))
}

#[cfg(unix)]
fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()
}

#[cfg(not(unix))]
fn write_private_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::write(path, bytes)
}

fn response_json(
    response: Result<ureq::Response, ureq::Error>,
    operation: &str,
) -> Result<Value, String> {
    match response {
        Ok(response) => response
            .into_json()
            .map_err(|error| format!("{operation} returned invalid JSON: {error}")),
        Err(ureq::Error::Status(status, response)) => {
            let body = response.into_string().unwrap_or_default();
            Err(format!(
                "{operation} returned HTTP {status}: {}",
                trim_report(&body)
            ))
        }
        Err(error) => Err(format!("{operation} request failed: {error}")),
    }
}

fn parse_identity(body: &Value) -> Result<MainnetIdentity, String> {
    let identity = body.get("identity").ok_or_else(|| {
        format!(
            "missing identity object in {}",
            trim_report(&body.to_string())
        )
    })?;

    Ok(MainnetIdentity {
        name: string_field(identity, "name")?,
        slot: string_field(identity, "slot")?,
        handle_path: string_field(identity, "handle_path")?,
        pseudonym: string_field(identity, "pseudonym")?,
        agent_id: string_field(identity, "agent_id")?,
    })
}

fn parse_issued_credential(data: &Value, source: &str) -> Result<IssuedCredential, String> {
    Ok(IssuedCredential {
        api_token: string_field(data, "api_token")?,
        agent_id: string_field(data, "agent_id")?,
        pseudonym: string_field(data, "pseudo_id")?,
        operator_email: optional_string_field(data, "operator_email"),
        source: source.to_owned(),
    })
}

fn string_field(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing string field `{field}`"))
}

fn optional_string_field(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn envelope_data(body: &Value) -> &Value {
    body.get("data").unwrap_or(body)
}

fn join_endpoint(endpoint: &str, path: &str) -> String {
    format!("{}{}", endpoint.trim_end_matches('/'), path)
}

fn read_optional_token_file() -> Result<Option<(String, String)>, String> {
    let raw_path = match env_optional("WIRE_API_TOKEN_FILE") {
        Some(path) => path,
        None => return Ok(None),
    };
    let path = expand_home(&raw_path)?;
    let token = fs::read_to_string(&path)
        .map_err(|error| {
            format!(
                "failed to read WIRE_API_TOKEN_FILE `{}`: {error}",
                path.display()
            )
        })?
        .trim()
        .to_owned();
    if token.is_empty() {
        return Err(format!(
            "WIRE_API_TOKEN_FILE `{}` did not contain a token",
            path.display()
        ));
    }
    Ok(Some((token, "WIRE_API_TOKEN_FILE".to_owned())))
}

fn env_optional(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn default_state_path() -> Result<PathBuf, String> {
    let home =
        env::var("HOME").map_err(|_| "HOME is not set; set WIRE_AUTH_STATE_PATH".to_owned())?;
    Ok(Path::new(&home).join(DEFAULT_STATE_RELATIVE_PATH))
}

fn expand_home(raw: &str) -> Result<PathBuf, String> {
    if raw == "~" {
        return env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| "HOME is not set; cannot expand `~`".to_owned());
    }
    if let Some(rest) = raw.strip_prefix("~/") {
        let home =
            env::var("HOME").map_err(|_| format!("HOME is not set; cannot expand `{raw}`"))?;
        return Ok(Path::new(&home).join(rest));
    }
    Ok(PathBuf::from(raw))
}

fn path_for_report(path: &Path) -> String {
    path.display().to_string()
}

fn utc_now() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
}

fn passed_step(
    name: impl Into<String>,
    proves: impl Into<String>,
    details: Vec<String>,
) -> MainnetAuthSubtest {
    MainnetAuthSubtest {
        name: name.into(),
        proves: proves.into(),
        status: MainnetAuthStatus::Passed,
        details,
    }
}

fn failed_step(
    name: impl Into<String>,
    proves: impl Into<String>,
    reason: impl Into<String>,
) -> MainnetAuthSubtest {
    MainnetAuthSubtest {
        name: name.into(),
        proves: proves.into(),
        status: MainnetAuthStatus::Failed {
            reason: reason.into(),
        },
        details: Vec::new(),
    }
}

fn trim_report(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() > 300 {
        format!("{}...", &trimmed[..300])
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeAuthTransport {
        valid_tokens: HashMap<String, MainnetIdentity>,
        resumed_token: Option<String>,
        registered_token: Option<String>,
        calls: Mutex<Vec<String>>,
    }

    impl MainnetAuthTransport for FakeAuthTransport {
        fn validate_token(&self, _endpoint: &str, token: &str) -> Result<MainnetIdentity, String> {
            self.calls.lock().unwrap().push(format!("validate:{token}"));
            self.valid_tokens
                .get(token)
                .cloned()
                .ok_or_else(|| "invalid token".to_owned())
        }

        fn resume_agent(
            &self,
            _endpoint: &str,
            _device_secret: &str,
            agent_name: &str,
            nonce: &str,
        ) -> Result<IssuedCredential, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("resume:{agent_name}:{nonce}"));
            let token = self
                .resumed_token
                .clone()
                .ok_or_else(|| "resume unavailable".to_owned())?;
            Ok(IssuedCredential {
                api_token: token,
                agent_id: "agent-resumed".to_owned(),
                pseudonym: "wire_agent_resumed".to_owned(),
                operator_email: Some("operator@example.test".to_owned()),
                source: "agent_resume".to_owned(),
            })
        }

        fn register_agent(
            &self,
            _endpoint: &str,
            agent_name: &str,
            operator_email: &str,
        ) -> Result<IssuedCredential, String> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("register:{agent_name}:{operator_email}"));
            let token = self
                .registered_token
                .clone()
                .ok_or_else(|| "register unavailable".to_owned())?;
            Ok(IssuedCredential {
                api_token: token,
                agent_id: "agent-registered".to_owned(),
                pseudonym: "wire_agent_registered".to_owned(),
                operator_email: Some(operator_email.to_owned()),
                source: "register".to_owned(),
            })
        }
    }

    #[test]
    fn validates_persisted_state_without_issuing_new_token() {
        let path = test_state_path("persisted");
        let identity = test_identity("agent-wire-substrate-node");
        let state = MainnetAuthState {
            version: AUTH_STATE_VERSION,
            endpoint: DEFAULT_MAINNET_ENDPOINT.to_owned(),
            agent_name: identity.name.clone(),
            api_token: "persisted-token".to_owned(),
            agent_id: identity.agent_id.clone(),
            pseudonym: identity.pseudonym.clone(),
            handle_path: identity.handle_path.clone(),
            updated_at: utc_now(),
        };
        persist_state(&path, &state).unwrap();
        let mut valid_tokens = HashMap::new();
        valid_tokens.insert("persisted-token".to_owned(), identity);
        let transport = FakeAuthTransport {
            valid_tokens,
            ..Default::default()
        };

        let report = run_mainnet_auth_with_transport(test_config(path), &transport);

        assert!(report.all_green());
        assert_eq!(report.token_source, "persisted_state");
        assert!(transport
            .calls
            .lock()
            .unwrap()
            .iter()
            .all(|call| !call.starts_with("resume") && !call.starts_with("register")));
    }

    #[test]
    fn imports_seed_token_and_persists_it_for_restart() {
        let path = test_state_path("seed");
        let identity = test_identity("codex-kramer");
        let mut valid_tokens = HashMap::new();
        valid_tokens.insert("seed-token".to_owned(), identity.clone());
        let transport = FakeAuthTransport {
            valid_tokens,
            ..Default::default()
        };
        let mut config = test_config(path.clone());
        config.seed_token = Some("seed-token".to_owned());
        config.seed_token_source = Some("WIRE_API_TOKEN_FILE".to_owned());

        let report = run_mainnet_auth_with_transport(config, &transport);

        assert!(report.all_green());
        assert_eq!(report.token_source, "WIRE_API_TOKEN_FILE");
        let persisted =
            serde_json::from_str::<MainnetAuthState>(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(persisted.api_token, "seed-token");
        assert_eq!(persisted.handle_path, identity.handle_path);
    }

    #[test]
    fn resumes_with_device_secret_when_no_persisted_token_exists() {
        let path = test_state_path("resume");
        let identity = test_identity("agent-wire-substrate-node");
        let mut valid_tokens = HashMap::new();
        valid_tokens.insert("resumed-token".to_owned(), identity);
        let transport = FakeAuthTransport {
            valid_tokens,
            resumed_token: Some("resumed-token".to_owned()),
            ..Default::default()
        };
        let mut config = test_config(path.clone());
        config.device_secret = Some("device-secret".to_owned());

        let report = run_mainnet_auth_with_transport(config, &transport);

        assert!(report.all_green());
        assert_eq!(report.token_source, "agent_resume");
        let calls = transport.calls.lock().unwrap();
        assert!(calls.iter().any(|call| call.starts_with("resume:")));
        assert!(path.exists());
    }

    #[test]
    fn fails_closed_without_any_credential_path() {
        let path = test_state_path("missing");
        let report =
            run_mainnet_auth_with_transport(test_config(path), &FakeAuthTransport::default());

        assert!(!report.all_green());
        assert!(matches!(
            report.subtests.last().unwrap().status,
            MainnetAuthStatus::Failed { .. }
        ));
    }

    fn test_config(path: PathBuf) -> MainnetAuthConfig {
        MainnetAuthConfig {
            endpoint: DEFAULT_MAINNET_ENDPOINT.to_owned(),
            agent_name: DEFAULT_AGENT_NAME.to_owned(),
            operator_email: None,
            device_secret: None,
            seed_token: None,
            seed_token_source: None,
            state_path: path,
        }
    }

    fn test_identity(name: &str) -> MainnetIdentity {
        let slot = name.trim_start_matches("codex-").to_owned();
        MainnetIdentity {
            name: name.to_owned(),
            slot: slot.clone(),
            handle_path: format!("agent/playful/{slot}"),
            pseudonym: "wire_agent_test".to_owned(),
            agent_id: "123a61b2-c33d-4d4d-bab8-feea57d9c625".to_owned(),
        }
    }

    fn test_state_path(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!(
            "agent-wire-substrate-node-auth-test-{name}-{}",
            Uuid::new_v4()
        ));
        dir.join("auth.json")
    }
}
