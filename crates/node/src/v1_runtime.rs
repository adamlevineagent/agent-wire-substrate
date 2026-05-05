use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::thread;

use agent_wire_foundation::canonical_ops::{
    CanonicalOp, HttpOperation, HttpRoute, MaintenanceOperation, MaintenanceTask, McpTool,
};
use agent_wire_substrate::NodeConfig;
use serde::{Deserialize, Serialize};

use crate::v1_surface::{
    dispatch_http_route, dispatch_maintenance_task, dispatch_mcp_tool, maintenance_implementation,
    MaintenanceImplementation, V1ProtocolDispatch, V1ProtocolStatus,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1HttpRequest {
    pub method: String,
    pub path: String,
}

impl V1HttpRequest {
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1ListenerDispatchReport {
    pub protocol: V1ListenerProtocol,
    pub edge_name: String,
    pub dispatch: V1ProtocolDispatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V1ListenerProtocol {
    Http,
    Mcp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1HttpLoopbackSmoke {
    pub bound_addr: String,
    pub request: V1HttpRequest,
    pub dispatch: V1ListenerDispatchReport,
    pub response_status: u16,
    pub response_contains_canonical_route: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1PersistedIdentity {
    pub node_id: String,
    pub operator: String,
    pub namespace: String,
    pub master_key_id: String,
    pub mainnet_endpoint: String,
    pub local_api_endpoint: String,
    pub persisted_at_ms: u64,
}

impl V1PersistedIdentity {
    pub fn from_config(config: &NodeConfig, persisted_at_ms: u64) -> Self {
        Self {
            node_id: config.node_id.clone(),
            operator: config.operator.to_string(),
            namespace: config.namespace.as_str().to_owned(),
            master_key_id: config.keys.master_public_key.key_id.as_str().to_owned(),
            mainnet_endpoint: config.mainnet_endpoint.as_url().to_string(),
            local_api_endpoint: config.local_api_endpoint.as_url().to_string(),
            persisted_at_ms,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1IdentityStoreReport {
    pub state_file: String,
    pub persisted: V1PersistedIdentity,
    pub loaded: V1PersistedIdentity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V1IdentityStore {
    state_file: PathBuf,
}

impl V1IdentityStore {
    pub fn new(state_dir: impl AsRef<Path>) -> Self {
        Self {
            state_file: state_dir.as_ref().join("v1-identity.json"),
        }
    }

    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    pub fn persist(
        &self,
        config: &NodeConfig,
        persisted_at_ms: u64,
    ) -> Result<V1IdentityStoreReport, String> {
        let persisted = V1PersistedIdentity::from_config(config, persisted_at_ms);
        if let Some(parent) = self.state_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create state dir: {error}"))?;
        }
        let temp_file = self.state_file.with_extension("json.tmp");
        let encoded = serde_json::to_vec_pretty(&persisted)
            .map_err(|error| format!("failed to encode identity state: {error}"))?;
        fs::write(&temp_file, encoded)
            .map_err(|error| format!("failed to write identity temp file: {error}"))?;
        fs::rename(&temp_file, &self.state_file)
            .map_err(|error| format!("failed to atomically persist identity state: {error}"))?;
        let loaded = self.load()?;
        Ok(V1IdentityStoreReport {
            state_file: self.state_file.display().to_string(),
            persisted,
            loaded,
        })
    }

    pub fn load(&self) -> Result<V1PersistedIdentity, String> {
        let body = fs::read_to_string(&self.state_file)
            .map_err(|error| format!("failed to read identity state: {error}"))?;
        serde_json::from_str(&body)
            .map_err(|error| format!("failed to decode identity state: {error}"))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1ScheduledMaintenance {
    pub task: MaintenanceTask,
    pub canonical_name: &'static str,
    pub cadence: &'static str,
    pub next_due_ms: u64,
    pub implementation: MaintenanceImplementation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1SchedulerTickReport {
    pub now_ms: u64,
    pub fired: Vec<V1ProtocolDispatch>,
    pub skipped_future: Vec<V1ProtocolDispatch>,
    pub next_schedule: Vec<V1ScheduledMaintenance>,
}

impl V1SchedulerTickReport {
    pub fn local_count(&self) -> usize {
        self.fired
            .iter()
            .filter(|dispatch| dispatch.status == V1ProtocolStatus::Ready)
            .count()
    }

    pub fn stubbed_count(&self) -> usize {
        self.skipped_future.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V1MaintenanceScheduler {
    schedule: Vec<V1ScheduledMaintenance>,
}

impl V1MaintenanceScheduler {
    pub fn due_now(now_ms: u64) -> Self {
        Self {
            schedule: MaintenanceTask::ALL
                .iter()
                .map(|task| V1ScheduledMaintenance {
                    task: *task,
                    canonical_name: task.name(),
                    cadence: task.cron_hint(),
                    next_due_ms: now_ms,
                    implementation: maintenance_implementation(*task),
                })
                .collect(),
        }
    }

    pub fn tick(mut self, now_ms: u64) -> V1SchedulerTickReport {
        let mut fired = Vec::new();
        let mut skipped_future = Vec::new();
        for scheduled in &mut self.schedule {
            let dispatch = dispatch_maintenance_task(scheduled.task);
            match scheduled.implementation {
                MaintenanceImplementation::Local if scheduled.next_due_ms <= now_ms => {
                    fired.push(dispatch);
                    scheduled.next_due_ms = now_ms + cadence_ms(scheduled.cadence);
                }
                MaintenanceImplementation::StubbedFuture => skipped_future.push(dispatch),
                MaintenanceImplementation::Local => {}
            }
        }
        V1SchedulerTickReport {
            now_ms,
            fired,
            skipped_future,
            next_schedule: self.schedule,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1RuntimeSmokeReport {
    pub http: V1HttpLoopbackSmoke,
    pub mcp: V1ListenerDispatchReport,
    pub identity: V1IdentityStoreReport,
    pub maintenance: V1SchedulerTickReport,
}

pub fn dispatch_http_request(request: &V1HttpRequest) -> Result<V1ListenerDispatchReport, String> {
    let route = HttpRoute::ALL
        .iter()
        .copied()
        .find(|route| {
            route.method().as_str() == request.method.as_str()
                && route_template_matches(route.path(), &request.path)
        })
        .ok_or_else(|| {
            format!(
                "no registered HTTP route for {} {}",
                request.method, request.path
            )
        })?;
    Ok(V1ListenerDispatchReport {
        protocol: V1ListenerProtocol::Http,
        edge_name: format!("{} {}", request.method, request.path),
        dispatch: dispatch_http_route(route),
    })
}

pub fn dispatch_mcp_request(tool_name: &str) -> Result<V1ListenerDispatchReport, String> {
    let tool = McpTool::ALL
        .iter()
        .copied()
        .find(|tool| tool.name() == tool_name)
        .ok_or_else(|| format!("no registered MCP tool for {tool_name}"))?;
    Ok(V1ListenerDispatchReport {
        protocol: V1ListenerProtocol::Mcp,
        edge_name: tool_name.to_owned(),
        dispatch: dispatch_mcp_tool(tool),
    })
}

pub fn run_http_loopback_smoke() -> Result<V1HttpLoopbackSmoke, String> {
    let request = V1HttpRequest::new("GET", "/wire/pulse");
    let expected_dispatch = dispatch_http_request(&request)?;
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("failed to bind HTTP loopback listener: {error}"))?;
    let bound_addr = listener
        .local_addr()
        .map_err(|error| format!("failed to read listener address: {error}"))?;
    let server = thread::spawn(move || serve_one_http_request(listener));
    let response = send_loopback_http_request(bound_addr, &request)?;
    server
        .join()
        .map_err(|_| "HTTP listener thread panicked".to_owned())??;
    Ok(V1HttpLoopbackSmoke {
        bound_addr: bound_addr.to_string(),
        request,
        dispatch: expected_dispatch,
        response_status: response_status(&response),
        response_contains_canonical_route: response.contains("\"canonical_name\":\"/wire/pulse\"")
            || response.contains("\"canonical_name\": \"/wire/pulse\""),
    })
}

pub fn run_v1_runtime_smoke(state_dir: &Path) -> Result<V1RuntimeSmokeReport, String> {
    let config = NodeConfig::demo().map_err(|error| error.to_string())?;
    let identity = V1IdentityStore::new(state_dir).persist(&config, 1_000)?;
    let maintenance = V1MaintenanceScheduler::due_now(1_000).tick(1_000);
    Ok(V1RuntimeSmokeReport {
        http: run_http_loopback_smoke()?,
        mcp: dispatch_mcp_request("wire_pulse")?,
        identity,
        maintenance,
    })
}

pub fn default_state_dir() -> PathBuf {
    if let Ok(value) = std::env::var("AGENT_WIRE_NODE_STATE_DIR") {
        return PathBuf::from(value);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".wire-node").join("state");
    }
    std::env::temp_dir().join("agent-wire-substrate-node-state")
}

fn serve_one_http_request(listener: TcpListener) -> Result<(), String> {
    let (mut stream, _) = listener
        .accept()
        .map_err(|error| format!("failed to accept HTTP loopback request: {error}"))?;
    let mut buffer = [0_u8; 1024];
    let read = stream
        .read(&mut buffer)
        .map_err(|error| format!("failed to read HTTP loopback request: {error}"))?;
    let request_line = String::from_utf8_lossy(&buffer[..read])
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    let response_body = match parse_http_request_line(&request_line).and_then(|request| {
        dispatch_http_request(&request).and_then(|dispatch| {
            serde_json::to_string(&dispatch)
                .map_err(|error| format!("failed to encode HTTP dispatch: {error}"))
        })
    }) {
        Ok(body) => body,
        Err(error) => serde_json::json!({ "error": error }).to_string(),
    };
    let response = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|error| format!("failed to write HTTP loopback response: {error}"))
}

fn send_loopback_http_request(addr: SocketAddr, request: &V1HttpRequest) -> Result<String, String> {
    let mut stream = TcpStream::connect(addr)
        .map_err(|error| format!("failed to connect to HTTP loopback listener: {error}"))?;
    let request = format!(
        "{} {} HTTP/1.1\r\nhost: {}\r\nconnection: close\r\n\r\n",
        request.method, request.path, addr
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write HTTP loopback request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read HTTP loopback response: {error}"))?;
    Ok(response)
}

fn parse_http_request_line(line: &str) -> Result<V1HttpRequest, String> {
    let parts = line.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [method, path, _version] => Ok(V1HttpRequest::new(*method, *path)),
        _ => Err("invalid HTTP request line".to_owned()),
    }
}

fn response_status(response: &str) -> u16 {
    response
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<u16>().ok())
        .unwrap_or(0)
}

fn route_template_matches(template: &str, path: &str) -> bool {
    let template_parts = template.trim_matches('/').split('/').collect::<Vec<_>>();
    let path_parts = path.trim_matches('/').split('/').collect::<Vec<_>>();
    if template_parts.len() != path_parts.len() {
        return false;
    }
    template_parts
        .iter()
        .zip(path_parts.iter())
        .all(|(template_part, path_part)| {
            (template_part.starts_with('{') && template_part.ends_with('}'))
                || template_part == path_part
        })
}

fn cadence_ms(cadence: &str) -> u64 {
    match cadence {
        "every_5_minutes" => 5 * 60 * 1_000,
        "hourly" => 60 * 60 * 1_000,
        "daily" => 24 * 60 * 60 * 1_000,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_state_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("agent-wire-node-{name}-{}", std::process::id()))
    }

    #[test]
    fn http_listener_routes_requests_through_foundation_registry() {
        let report = dispatch_http_request(&V1HttpRequest::new("GET", "/wire/messages/abc"))
            .expect("registered templated route");

        assert_eq!(report.protocol, V1ListenerProtocol::Http);
        assert!(matches!(
            report.dispatch.binding,
            crate::v1_surface::V1ProtocolBinding::Http {
                route: HttpRoute::WireMessageById,
                ..
            }
        ));
        assert!(dispatch_http_request(&V1HttpRequest::new("GET", "/not-real")).is_err());
    }

    #[test]
    fn mcp_listener_routes_tool_names_through_foundation_registry() {
        let report = dispatch_mcp_request("wire_wait").expect("registered MCP tool");

        assert_eq!(report.protocol, V1ListenerProtocol::Mcp);
        assert!(matches!(
            report.dispatch.binding,
            crate::v1_surface::V1ProtocolBinding::Mcp {
                tool: McpTool::WireWait
            }
        ));
        assert!(dispatch_mcp_request("wire_not_real").is_err());
    }

    #[test]
    fn identity_store_persists_node_identity_and_loads_it_back() {
        let state_dir = temp_state_dir("identity");
        let _ = fs::remove_dir_all(&state_dir);
        let config = NodeConfig::demo().unwrap();
        let report = V1IdentityStore::new(&state_dir)
            .persist(&config, 42)
            .expect("identity persisted");

        assert_eq!(report.persisted, report.loaded);
        assert_eq!(report.loaded.node_id, "node2-demo");
        assert_eq!(report.loaded.operator, "agent/playful/kramer");
        assert!(Path::new(&report.state_file).exists());
        let _ = fs::remove_dir_all(state_dir);
    }

    #[test]
    fn scheduler_tick_fires_local_tasks_and_skips_future_stubs() {
        let report = V1MaintenanceScheduler::due_now(1_000).tick(1_000);

        assert_eq!(report.local_count(), 8);
        assert_eq!(report.stubbed_count(), 4);
        assert!(report
            .fired
            .iter()
            .any(|dispatch| dispatch.canonical_name == "worker_liveness_check"));
        assert!(report
            .skipped_future
            .iter()
            .any(|dispatch| dispatch.canonical_name == "chronicle_retention"));
    }

    #[test]
    fn runtime_smoke_exercises_http_mcp_identity_and_scheduler() {
        let state_dir = temp_state_dir("runtime-smoke");
        let _ = fs::remove_dir_all(&state_dir);
        let report = run_v1_runtime_smoke(&state_dir).expect("runtime smoke");

        assert_eq!(report.http.response_status, 200);
        assert!(report.http.response_contains_canonical_route);
        assert!(matches!(
            report.mcp.dispatch.binding,
            crate::v1_surface::V1ProtocolBinding::Mcp {
                tool: McpTool::WirePulse
            }
        ));
        assert_eq!(report.identity.loaded.node_id, "node2-demo");
        assert_eq!(report.maintenance.local_count(), 8);
        let _ = fs::remove_dir_all(state_dir);
    }
}
