use std::fmt;
use std::fs;
use std::path::Path;

use agent_wire_compiler::{
    CanonicalWireActionDefinition, CompileError, CompiledOperation, CompilerContext,
    WireActionDefinition, WireCompiledPlan, WireCompiler,
};
use agent_wire_foundation::canonical_ops::{
    CanonicalOp, HttpOperation, HttpRoute, InvocationMode, MaintenanceOperation, MaintenanceTask,
    McpTool,
};
use agent_wire_foundation::{CrossGraphRef, FoundationError, IdempotencyKey};
use serde::Serialize;

use crate::v1_runtime::{
    default_state_dir, dispatch_http_request, dispatch_mcp_request, run_http_loopback_smoke,
    run_v1_runtime_smoke, V1HttpRequest, V1IdentityStore, V1MaintenanceScheduler,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum V1SurfaceDisposition {
    Essential,
    NiceToHave,
    Deferred,
    NotInV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum V1CliCommand {
    SurfaceManifest,
    IdentitySignup,
    IdentityLogin,
    IdentityStatus,
    ChainCompile,
    ChainExecute,
    ChainQuote,
    ComputeOffer,
    ComputeQuote,
    ComputePurchase,
    ComputeFill,
    ComputeJobs,
    McpManifest,
    McpDispatch,
    HttpManifest,
    HttpDispatch,
    HttpSmoke,
    IdentityPersist,
    MaintenanceRunOnce,
    MaintenanceScheduleTick,
    RuntimeSmoke,
}

impl V1CliCommand {
    pub const ALL: [Self; 21] = [
        Self::SurfaceManifest,
        Self::IdentitySignup,
        Self::IdentityLogin,
        Self::IdentityStatus,
        Self::IdentityPersist,
        Self::ChainCompile,
        Self::ChainExecute,
        Self::ChainQuote,
        Self::ComputeOffer,
        Self::ComputeQuote,
        Self::ComputePurchase,
        Self::ComputeFill,
        Self::ComputeJobs,
        Self::McpManifest,
        Self::McpDispatch,
        Self::HttpManifest,
        Self::HttpDispatch,
        Self::HttpSmoke,
        Self::MaintenanceRunOnce,
        Self::MaintenanceScheduleTick,
        Self::RuntimeSmoke,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            Self::SurfaceManifest => "surface manifest",
            Self::IdentitySignup => "identity signup",
            Self::IdentityLogin => "identity login",
            Self::IdentityStatus => "identity status",
            Self::ChainCompile => "chain compile",
            Self::ChainExecute => "chain execute",
            Self::ChainQuote => "chain quote",
            Self::ComputeOffer => "compute offer",
            Self::ComputeQuote => "compute quote",
            Self::ComputePurchase => "compute purchase",
            Self::ComputeFill => "compute fill",
            Self::ComputeJobs => "compute jobs",
            Self::McpManifest => "mcp manifest",
            Self::McpDispatch => "mcp dispatch",
            Self::HttpManifest => "http manifest",
            Self::HttpDispatch => "http dispatch",
            Self::HttpSmoke => "http smoke",
            Self::IdentityPersist => "identity persist",
            Self::MaintenanceRunOnce => "maintenance run-once",
            Self::MaintenanceScheduleTick => "maintenance schedule-tick",
            Self::RuntimeSmoke => "runtime smoke",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CliSurface {
    pub command: V1CliCommand,
    pub name: &'static str,
    pub disposition: V1SurfaceDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct McpSurface {
    pub tool: McpTool,
    pub name: &'static str,
    pub disposition: V1SurfaceDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HttpSurface {
    pub route: HttpRoute,
    pub method: &'static str,
    pub path: &'static str,
    pub disposition: V1SurfaceDisposition,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaintenanceImplementation {
    Local,
    StubbedFuture,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MaintenanceSurface {
    pub task: MaintenanceTask,
    pub name: &'static str,
    pub cron_hint: &'static str,
    pub implementation: MaintenanceImplementation,
    pub disposition: V1SurfaceDisposition,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1NodeSurfaceManifest {
    pub cli: Vec<CliSurface>,
    pub mcp_tools: Vec<McpSurface>,
    pub http_routes: Vec<HttpSurface>,
    pub maintenance_tasks: Vec<MaintenanceSurface>,
}

impl V1NodeSurfaceManifest {
    pub fn v1() -> Self {
        Self {
            cli: V1CliCommand::ALL
                .iter()
                .map(|command| CliSurface {
                    command: *command,
                    name: command.name(),
                    disposition: V1SurfaceDisposition::Essential,
                })
                .collect(),
            mcp_tools: McpTool::ALL
                .iter()
                .map(|tool| McpSurface {
                    tool: *tool,
                    name: tool.name(),
                    disposition: mcp_disposition(*tool),
                })
                .collect(),
            http_routes: HttpRoute::ALL
                .iter()
                .map(|route| HttpSurface {
                    route: *route,
                    method: route.method().as_str(),
                    path: route.path(),
                    disposition: http_disposition(*route),
                })
                .collect(),
            maintenance_tasks: MaintenanceTask::ALL
                .iter()
                .map(|task| MaintenanceSurface {
                    task: *task,
                    name: task.name(),
                    cron_hint: task.cron_hint(),
                    implementation: maintenance_implementation(*task),
                    disposition: maintenance_disposition(*task),
                })
                .collect(),
        }
    }

    pub fn implemented_maintenance_count(&self) -> usize {
        self.maintenance_tasks
            .iter()
            .filter(|surface| surface.implementation == MaintenanceImplementation::Local)
            .count()
    }

    pub fn stubbed_maintenance_count(&self) -> usize {
        self.maintenance_tasks
            .iter()
            .filter(|surface| surface.implementation == MaintenanceImplementation::StubbedFuture)
            .count()
    }
}

pub fn mcp_disposition(tool: McpTool) -> V1SurfaceDisposition {
    match tool {
        McpTool::WireIdentify
        | McpTool::WireStatus
        | McpTool::WireMe
        | McpTool::WireBalance
        | McpTool::WirePulse
        | McpTool::WireQuery
        | McpTool::WireContribute
        | McpTool::WireRead
        | McpTool::WireAccessContribution
        | McpTool::WireInspect
        | McpTool::WireResolveHandle
        | McpTool::WireHandles
        | McpTool::WireActionInvoke
        | McpTool::WireActionChain
        | McpTool::WirePrepare
        | McpTool::WireMarket
        | McpTool::WireMessages
        | McpTool::WireTasks
        | McpTool::WireWait
        | McpTool::WireRoster
        | McpTool::WireMyContributions
        | McpTool::WireEarnings
        | McpTool::WireSupersede
        | McpTool::WireCorrect => V1SurfaceDisposition::Essential,
        McpTool::WireBrowse
        | McpTool::WireGraph
        | McpTool::WirePearlDive
        | McpTool::WireCorpora
        | McpTool::WireDocuments
        | McpTool::WireReadDocument
        | McpTool::WireDiscover
        | McpTool::WireSubscriptions
        | McpTool::WireNotifications
        | McpTool::WireOpportunities
        | McpTool::WireRate
        | McpTool::WireFlag
        | McpTool::WireTemplates
        | McpTool::WireListManage
        | McpTool::WireQueryRipePredictions
        | McpTool::WireRetract => V1SurfaceDisposition::NiceToHave,
        McpTool::WireCircles
        | McpTool::WireCircleAdmin
        | McpTool::WireAgentManage
        | McpTool::WireEventSubscriptions
        | McpTool::WireFeedback
        | McpTool::WireGames
        | McpTool::WirePatrol
        | McpTool::WirePins
        | McpTool::WireRequests
        | McpTool::WireSync
        | McpTool::WireLegal
        | McpTool::WireMesh
        | McpTool::WireMeshBoard
        | McpTool::WireMeshIntent
        | McpTool::WireMeshStatus => V1SurfaceDisposition::Deferred,
    }
}

pub fn http_disposition(route: HttpRoute) -> V1SurfaceDisposition {
    match route {
        HttpRoute::NodeHeartbeat | HttpRoute::WireMaintenance | HttpRoute::WireMaintenanceTick => {
            V1SurfaceDisposition::NiceToHave
        }
        _ => V1SurfaceDisposition::Essential,
    }
}

pub fn maintenance_implementation(task: MaintenanceTask) -> MaintenanceImplementation {
    match task {
        MaintenanceTask::CallbackSecretRetention
        | MaintenanceTask::CoordinationEventRetention
        | MaintenanceTask::FillIdempotencyRetention
        | MaintenanceTask::ProviderSettlementExpiry
        | MaintenanceTask::PurchaseExpiry
        | MaintenanceTask::MarketSnapshot
        | MaintenanceTask::MarketSnapshotRetention
        | MaintenanceTask::WorkerLivenessCheck => MaintenanceImplementation::Local,
        MaintenanceTask::Sweep
        | MaintenanceTask::StaleOffers
        | MaintenanceTask::ChronicleRetention
        | MaintenanceTask::ObservationRetention => MaintenanceImplementation::StubbedFuture,
    }
}

pub fn maintenance_disposition(task: MaintenanceTask) -> V1SurfaceDisposition {
    match task {
        MaintenanceTask::CallbackSecretRetention
        | MaintenanceTask::CoordinationEventRetention
        | MaintenanceTask::FillIdempotencyRetention
        | MaintenanceTask::ProviderSettlementExpiry
        | MaintenanceTask::PurchaseExpiry
        | MaintenanceTask::WorkerLivenessCheck => V1SurfaceDisposition::Essential,
        MaintenanceTask::MarketSnapshot | MaintenanceTask::MarketSnapshotRetention => {
            V1SurfaceDisposition::NiceToHave
        }
        MaintenanceTask::Sweep
        | MaintenanceTask::StaleOffers
        | MaintenanceTask::ChronicleRetention
        | MaintenanceTask::ObservationRetention => V1SurfaceDisposition::Deferred,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum V1ProtocolStatus {
    Ready,
    Deferred,
    StubbedOutOfV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "surface", rename_all = "snake_case")]
pub enum V1ProtocolBinding {
    Cli {
        command: V1CliCommand,
    },
    Mcp {
        tool: McpTool,
    },
    Http {
        route: HttpRoute,
        method: &'static str,
        path: &'static str,
    },
    Maintenance {
        task: MaintenanceTask,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1ProtocolDispatch {
    pub binding: V1ProtocolBinding,
    pub canonical_name: &'static str,
    pub disposition: V1SurfaceDisposition,
    pub status: V1ProtocolStatus,
    pub detail: String,
}

pub fn dispatch_mcp_tool(tool: McpTool) -> V1ProtocolDispatch {
    let disposition = mcp_disposition(tool);
    V1ProtocolDispatch {
        binding: V1ProtocolBinding::Mcp { tool },
        canonical_name: tool.name(),
        disposition,
        status: status_for_disposition(disposition),
        detail: detail_for_disposition(disposition, "MCP tool binding"),
    }
}

pub fn dispatch_http_route(route: HttpRoute) -> V1ProtocolDispatch {
    let disposition = http_disposition(route);
    V1ProtocolDispatch {
        binding: V1ProtocolBinding::Http {
            route,
            method: route.method().as_str(),
            path: route.path(),
        },
        canonical_name: route.name(),
        disposition,
        status: status_for_disposition(disposition),
        detail: detail_for_disposition(disposition, "HTTP route binding"),
    }
}

pub fn dispatch_maintenance_task(task: MaintenanceTask) -> V1ProtocolDispatch {
    let disposition = maintenance_disposition(task);
    let implementation = maintenance_implementation(task);
    V1ProtocolDispatch {
        binding: V1ProtocolBinding::Maintenance { task },
        canonical_name: task.name(),
        disposition,
        status: match implementation {
            MaintenanceImplementation::Local => status_for_disposition(disposition),
            MaintenanceImplementation::StubbedFuture => V1ProtocolStatus::Deferred,
        },
        detail: match implementation {
            MaintenanceImplementation::Local => {
                format!(
                    "local maintenance task fired with {} cadence",
                    task.cron_hint()
                )
            }
            MaintenanceImplementation::StubbedFuture => {
                "stubbed future maintenance task; logged and skipped in V1".to_owned()
            }
        },
    }
}

fn status_for_disposition(disposition: V1SurfaceDisposition) -> V1ProtocolStatus {
    match disposition {
        V1SurfaceDisposition::Essential | V1SurfaceDisposition::NiceToHave => {
            V1ProtocolStatus::Ready
        }
        V1SurfaceDisposition::Deferred => V1ProtocolStatus::Deferred,
        V1SurfaceDisposition::NotInV1 => V1ProtocolStatus::StubbedOutOfV1,
    }
}

fn detail_for_disposition(disposition: V1SurfaceDisposition, surface: &str) -> String {
    match disposition {
        V1SurfaceDisposition::Essential => {
            format!("{surface} is in the V1 node surface and ready for protocol-edge wiring")
        }
        V1SurfaceDisposition::NiceToHave => {
            format!("{surface} is V1 nice-to-have and exposed as a typed local binding")
        }
        V1SurfaceDisposition::Deferred => {
            format!("{surface} is registered but deferred beyond V1 runtime execution")
        }
        V1SurfaceDisposition::NotInV1 => {
            format!("{surface} is explicitly out of V1 scope")
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1MaintenanceRunReport {
    pub fired: Vec<V1ProtocolDispatch>,
    pub local_count: usize,
    pub stubbed_count: usize,
}

pub fn run_maintenance_once() -> V1MaintenanceRunReport {
    let fired = MaintenanceTask::ALL
        .iter()
        .map(|task| dispatch_maintenance_task(*task))
        .collect::<Vec<_>>();
    let local_count = fired
        .iter()
        .filter(|dispatch| dispatch.status == V1ProtocolStatus::Ready)
        .count();
    let stubbed_count = fired
        .iter()
        .filter(|dispatch| dispatch.status == V1ProtocolStatus::Deferred)
        .count();
    V1MaintenanceRunReport {
        fired,
        local_count,
        stubbed_count,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum V1StepExecutionStatus {
    RoutedToComputeMarket,
    RoutedToWireProtocol,
    RoutedToTaskBoard,
    StubbedOutOfV1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1ExecutedStep {
    pub name: String,
    pub operation_name: String,
    pub status: V1StepExecutionStatus,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct V1ExecutionReport {
    pub total_steps: usize,
    pub all_steps_routable: bool,
    pub steps: Vec<V1ExecutedStep>,
}

pub fn execute_compiled_plan(plan: &WireCompiledPlan) -> V1ExecutionReport {
    let steps = plan
        .steps
        .iter()
        .map(|step| {
            let (status, detail) = match step.operation {
                CompiledOperation::Llm(_) => (
                    V1StepExecutionStatus::RoutedToComputeMarket,
                    "LLM primitive routed through compute-market execution adapter".to_owned(),
                ),
                CompiledOperation::Wire(_) => (
                    V1StepExecutionStatus::RoutedToWireProtocol,
                    "Wire primitive routed through typed HTTP/MCP protocol binding".to_owned(),
                ),
                CompiledOperation::Task(_) => (
                    V1StepExecutionStatus::RoutedToTaskBoard,
                    "Task primitive routed through typed task-board protocol binding".to_owned(),
                ),
                CompiledOperation::Game => (
                    V1StepExecutionStatus::StubbedOutOfV1,
                    "game primitive is explicitly out of V1 scope".to_owned(),
                ),
            };
            V1ExecutedStep {
                name: step.name.clone(),
                operation_name: step.operation_name.clone(),
                status,
                detail,
            }
        })
        .collect::<Vec<_>>();
    let all_steps_routable = steps
        .iter()
        .all(|step| step.status != V1StepExecutionStatus::StubbedOutOfV1);
    V1ExecutionReport {
        total_steps: plan.total_steps,
        all_steps_routable,
        steps,
    }
}

pub fn load_action_definition(path: &Path) -> Result<WireActionDefinition, V1NodeSurfaceError> {
    let body = fs::read_to_string(path).map_err(|error| V1NodeSurfaceError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("yaml") | Some("yml") => load_yaml_action_definition(&body),
        _ => load_json_action_definition(&body),
    }
}

fn load_json_action_definition(body: &str) -> Result<WireActionDefinition, V1NodeSurfaceError> {
    if let Ok(canonical) = serde_json::from_str::<CanonicalWireActionDefinition>(body) {
        return canonical
            .into_internal()
            .map_err(|error| V1NodeSurfaceError::Compile(error.to_string()));
    }
    serde_json::from_str(body).map_err(|error| V1NodeSurfaceError::Json(error.to_string()))
}

fn load_yaml_action_definition(body: &str) -> Result<WireActionDefinition, V1NodeSurfaceError> {
    if let Ok(canonical) = serde_yaml::from_str::<CanonicalWireActionDefinition>(body) {
        return canonical
            .into_internal()
            .map_err(|error| V1NodeSurfaceError::Compile(error.to_string()));
    }
    serde_yaml::from_str(body).map_err(|error| V1NodeSurfaceError::Yaml(error.to_string()))
}

pub fn compile_chain_definition(
    definition: &WireActionDefinition,
    mode: InvocationMode,
) -> Result<WireCompiledPlan, V1NodeSurfaceError> {
    let compiler = WireCompiler::default();
    let context = compiler_context(&definition.name)?;
    compiler
        .compile(definition, mode, &context)
        .map_err(V1NodeSurfaceError::from)
}

pub fn compile_chain_file(
    path: &Path,
    mode: InvocationMode,
) -> Result<WireCompiledPlan, V1NodeSurfaceError> {
    let definition = load_action_definition(path)?;
    compile_chain_definition(&definition, mode)
}

fn compiler_context(action_name: &str) -> Result<CompilerContext, V1NodeSurfaceError> {
    let slug = slugify(action_name);
    let quote_key = IdempotencyKey::new(format!("node-quote-{slug}"))?;
    let quote_ref = "playful/124/ws5/1".parse::<CrossGraphRef>()?;
    Ok(CompilerContext {
        compiled_at_ms: 0,
        quote_ref,
        quote_key,
        quote_expires_at_ms: 60_000,
    })
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
        if slug.len() >= 48 {
            break;
        }
    }
    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "action".to_owned()
    } else {
        slug
    }
}

pub fn run_v1_node_cli(args: &[String]) -> Result<Option<String>, V1NodeSurfaceError> {
    let Some(command) = args.first().map(String::as_str) else {
        return Ok(None);
    };

    match command {
        "surface" | "v1-surface" => Ok(Some(to_pretty_json(&V1NodeSurfaceManifest::v1())?)),
        "mcp" => run_mcp_cli(&args[1..]).map(Some),
        "http" => run_http_cli(&args[1..]).map(Some),
        "maintenance" => run_maintenance_cli(&args[1..]).map(Some),
        "chain" => run_chain_cli(&args[1..]).map(Some),
        "identity" => run_identity_cli(&args[1..]).map(Some),
        "compute" => run_compute_cli(&args[1..]).map(Some),
        "runtime" => run_runtime_cli(&args[1..]).map(Some),
        _ => Ok(None),
    }
}

fn run_mcp_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("manifest") | None => to_pretty_json(&V1NodeSurfaceManifest::v1().mcp_tools),
        Some("dispatch") => {
            let tool = required_value(args.get(1), "mcp dispatch <tool_name>")?;
            to_pretty_json(&dispatch_mcp_request(tool).map_err(V1NodeSurfaceError::Runtime)?)
        }
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!("mcp {other}"))),
    }
}

fn run_http_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("manifest") | None => to_pretty_json(&V1NodeSurfaceManifest::v1().http_routes),
        Some("dispatch") => {
            let method = required_value(args.get(1), "http dispatch <method> <path>")?;
            let path = required_value(args.get(2), "http dispatch <method> <path>")?;
            to_pretty_json(
                &dispatch_http_request(&V1HttpRequest::new(method, path))
                    .map_err(V1NodeSurfaceError::Runtime)?,
            )
        }
        Some("smoke") => {
            to_pretty_json(&run_http_loopback_smoke().map_err(V1NodeSurfaceError::Runtime)?)
        }
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!("http {other}"))),
    }
}

fn run_maintenance_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("run-once") | Some("tick") | None => to_pretty_json(&run_maintenance_once()),
        Some("schedule-tick") => {
            to_pretty_json(&V1MaintenanceScheduler::due_now(1_000).tick(1_000))
        }
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!(
            "maintenance {other}"
        ))),
    }
}

fn run_chain_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("compile") => {
            let path = required_path(args.get(1), "chain compile <chain.yaml>")?;
            let mode = parse_mode(args.get(2).map(String::as_str).unwrap_or("quote"))?;
            to_pretty_json(&compile_chain_file(path, mode)?)
        }
        Some("execute") => {
            let path = required_path(args.get(1), "chain execute <chain.yaml>")?;
            let mode = parse_mode(args.get(2).map(String::as_str).unwrap_or("trusted"))?;
            let plan = compile_chain_file(path, mode)?;
            to_pretty_json(&execute_compiled_plan(&plan))
        }
        Some("quote") => {
            let path = required_path(args.get(1), "chain quote <chain.yaml>")?;
            to_pretty_json(&compile_chain_file(path, InvocationMode::Quote)?)
        }
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!("chain {other}"))),
        None => Err(V1NodeSurfaceError::MissingArgument("chain subcommand")),
    }
}

fn run_identity_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("signup") => to_pretty_json(&dispatch_http_route(HttpRoute::Register)),
        Some("login") => to_pretty_json(&dispatch_mcp_tool(McpTool::WireIdentify)),
        Some("persist") => {
            let state_dir = args
                .get(1)
                .map(Path::new)
                .map(Path::to_path_buf)
                .unwrap_or_else(default_state_dir);
            let config = agent_wire_substrate::NodeConfig::demo()?;
            to_pretty_json(
                &V1IdentityStore::new(state_dir)
                    .persist(&config, 1_000)
                    .map_err(V1NodeSurfaceError::Runtime)?,
            )
        }
        Some("load") => {
            let state_dir = args
                .get(1)
                .map(Path::new)
                .map(Path::to_path_buf)
                .unwrap_or_else(default_state_dir);
            to_pretty_json(
                &V1IdentityStore::new(state_dir)
                    .load()
                    .map_err(V1NodeSurfaceError::Runtime)?,
            )
        }
        Some("status") | None => to_pretty_json(&dispatch_mcp_tool(McpTool::WireStatus)),
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!(
            "identity {other}"
        ))),
    }
}

fn run_runtime_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    match args.first().map(String::as_str) {
        Some("smoke") | None => {
            let state_dir = args
                .get(1)
                .map(Path::new)
                .map(Path::to_path_buf)
                .unwrap_or_else(default_state_dir);
            to_pretty_json(&run_v1_runtime_smoke(&state_dir).map_err(V1NodeSurfaceError::Runtime)?)
        }
        Some(other) => Err(V1NodeSurfaceError::UnknownCommand(format!(
            "runtime {other}"
        ))),
    }
}

fn run_compute_cli(args: &[String]) -> Result<String, V1NodeSurfaceError> {
    let route = match args.first().map(String::as_str) {
        Some("offer") | Some("offers") => HttpRoute::ComputeOffers,
        Some("quote") => HttpRoute::ComputeQuote,
        Some("purchase") => HttpRoute::ComputePurchase,
        Some("fill") => HttpRoute::ComputeFill,
        Some("jobs") | None => HttpRoute::ComputeJobs,
        Some("market-surface") => HttpRoute::ComputeMarketSurface,
        Some(other) => {
            return Err(V1NodeSurfaceError::UnknownCommand(format!(
                "compute {other}"
            )))
        }
    };
    to_pretty_json(&dispatch_http_route(route))
}

fn parse_mode(value: &str) -> Result<InvocationMode, V1NodeSurfaceError> {
    match value {
        "quote" => Ok(InvocationMode::Quote),
        "review" => Ok(InvocationMode::Review),
        "trusted" => Ok(InvocationMode::Trusted),
        other => Err(V1NodeSurfaceError::UnknownCommand(format!(
            "invocation mode {other}"
        ))),
    }
}

fn required_path<'a>(
    value: Option<&'a String>,
    usage: &'static str,
) -> Result<&'a Path, V1NodeSurfaceError> {
    value
        .map(Path::new)
        .ok_or(V1NodeSurfaceError::MissingArgument(usage))
}

fn required_value<'a>(
    value: Option<&'a String>,
    usage: &'static str,
) -> Result<&'a str, V1NodeSurfaceError> {
    value
        .map(String::as_str)
        .ok_or(V1NodeSurfaceError::MissingArgument(usage))
}

fn to_pretty_json<T: Serialize>(value: &T) -> Result<String, V1NodeSurfaceError> {
    serde_json::to_string_pretty(value).map_err(|error| V1NodeSurfaceError::Json(error.to_string()))
}

#[derive(Debug)]
pub enum V1NodeSurfaceError {
    Io { path: String, message: String },
    Json(String),
    Yaml(String),
    Compile(String),
    Runtime(String),
    Foundation(FoundationError),
    MissingArgument(&'static str),
    UnknownCommand(String),
}

impl fmt::Display for V1NodeSurfaceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, message } => write!(formatter, "failed to read {path}: {message}"),
            Self::Json(message) => write!(formatter, "JSON error: {message}"),
            Self::Yaml(message) => write!(formatter, "YAML error: {message}"),
            Self::Compile(message) => write!(formatter, "compile error: {message}"),
            Self::Runtime(message) => write!(formatter, "runtime error: {message}"),
            Self::Foundation(error) => write!(formatter, "{error}"),
            Self::MissingArgument(usage) => write!(formatter, "missing argument: {usage}"),
            Self::UnknownCommand(command) => {
                write!(formatter, "unknown V1 node command: {command}")
            }
        }
    }
}

impl std::error::Error for V1NodeSurfaceError {}

impl From<CompileError> for V1NodeSurfaceError {
    fn from(error: CompileError) -> Self {
        Self::Compile(error.to_string())
    }
}

impl From<FoundationError> for V1NodeSurfaceError {
    fn from(error: FoundationError) -> Self {
        Self::Foundation(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_wire_compiler::{WireActionPermissions, WireActionStep};
    use agent_wire_foundation::canonical_ops::{LlmPrimitive, TaskPrimitive, WirePrimitive};
    use agent_wire_foundation::CreditAmount;

    fn permissions() -> WireActionPermissions {
        WireActionPermissions::trusted_local(CreditAmount::from_sats(1_000))
    }

    #[test]
    fn manifest_exposes_v1_protocol_surface_without_runtime_strings() {
        let manifest = V1NodeSurfaceManifest::v1();

        assert_eq!(manifest.cli.len(), 21);
        assert_eq!(manifest.mcp_tools.len(), 55);
        assert_eq!(manifest.http_routes.len(), 56);
        assert_eq!(manifest.maintenance_tasks.len(), 12);
        assert_eq!(manifest.implemented_maintenance_count(), 8);
        assert_eq!(manifest.stubbed_maintenance_count(), 4);
        assert!(manifest
            .mcp_tools
            .iter()
            .any(|surface| surface.tool == McpTool::WireActionInvoke));
        assert!(manifest
            .http_routes
            .iter()
            .any(|surface| surface.route == HttpRoute::ComputeQuote));
        assert!(manifest
            .cli
            .iter()
            .any(|surface| surface.command == V1CliCommand::RuntimeSmoke));
    }

    #[test]
    fn maintenance_run_fires_local_tasks_and_logs_future_stubs() {
        let report = run_maintenance_once();

        assert_eq!(report.local_count, 8);
        assert_eq!(report.stubbed_count, 4);
        assert!(report
            .fired
            .iter()
            .any(|task| task.canonical_name == "worker_liveness_check"));
        assert!(report
            .fired
            .iter()
            .any(|task| task.canonical_name == "chronicle_retention"));
    }

    #[test]
    fn chain_compile_and_execute_route_typed_operations() {
        let definition = WireActionDefinition::chain(
            "ws5-flow",
            vec![
                WireActionStep::llm("extract", LlmPrimitive::Extract),
                WireActionStep::wire("publish", WirePrimitive::Contribute),
                WireActionStep::task("claim", TaskPrimitive::Claim),
            ],
            permissions(),
        );

        let plan = compile_chain_definition(&definition, InvocationMode::Trusted).unwrap();
        let execution = execute_compiled_plan(&plan);

        assert_eq!(plan.total_steps, 3);
        assert!(execution.all_steps_routable);
        assert_eq!(
            execution.steps[0].status,
            V1StepExecutionStatus::RoutedToComputeMarket
        );
        assert_eq!(
            execution.steps[1].status,
            V1StepExecutionStatus::RoutedToWireProtocol
        );
        assert_eq!(
            execution.steps[2].status,
            V1StepExecutionStatus::RoutedToTaskBoard
        );
    }

    #[test]
    fn game_step_executes_as_explicit_v1_stub() {
        let definition =
            WireActionDefinition::single("game-stub", WireActionStep::game("play"), permissions());

        let plan = compile_chain_definition(&definition, InvocationMode::Review).unwrap();
        let execution = execute_compiled_plan(&plan);

        assert!(!execution.all_steps_routable);
        assert_eq!(
            execution.steps[0].status,
            V1StepExecutionStatus::StubbedOutOfV1
        );
    }

    #[test]
    fn cli_loader_accepts_canonical_wire_json_shape() {
        let body = r#"{
          "schemaVersion": 1,
          "actionType": "chain",
          "permissions": {
            "contribute": true,
            "maxCost": 1000
          },
          "steps": [
            {
              "name": "publish",
              "operation": "wire",
              "tool": "wire.contribute",
              "outputSchema": {"type": "object"},
              "modelTier": "low"
            }
          ]
        }"#;

        let definition = load_json_action_definition(body).unwrap();

        assert_eq!(definition.steps[0].wire, Some(WirePrimitive::Contribute));
        assert_eq!(
            definition.steps[0].model_tier,
            Some(agent_wire_compiler::ModelTier::Low)
        );
    }
}
