//! CLI validation crate for the Agent Wire substrate node.
//!
//! Runtime composition lives in the umbrella `agent-wire-substrate` library.
//! This crate keeps operator-facing validation commands and the binary shell.

pub mod contribution_sync;
pub mod d3_live_compute_settlement;
pub mod l6_failure_injection;
pub mod l6_observability;
pub mod l6_recovery;
pub mod l6_stability_driver;
pub mod layer3_synthetic;
pub mod layer4_synthetic;
pub mod layer5_live_llm;
pub mod mainnet_auth;
pub mod v1_runtime;
pub mod v1_surface;

pub use agent_wire_substrate::{
    build_parity_demo, compose_substrate_node, parity_demo_assertions, substrate_stack_name,
    BackgroundWorker, BackgroundWorkerLifecycle, LocalPersistence, MarketComposition, NodeConfig,
    NodeKeys, NodeRuntime, OperatorApiProtocol, OperatorApiSurface, OperatorRoute, OptInPolicy,
    ParityDemoReport, ParityDemoStep,
};
pub use contribution_sync::{
    run_live_contribution_sync, ContributionSyncItem, ContributionSyncReport,
    ContributionSyncStatus, ContributionSyncSubtest,
};
pub use d3_live_compute_settlement::{
    run_d3_live_compute_settlement, D3LiveComputeSettlementReport, D3Status, D3Subtest,
};
pub use l6_failure_injection::{
    run_failure_injection_scenarios, run_failure_injection_scenarios_with_policy, InjectionReport,
    InjectionScenarioResult, InjectionState, KillPoint, RecoveryPolicy, WriteOncePolicy,
};
pub use l6_observability::{
    observe_l6_stability, L6CycleObservability, L6ObservabilityReport, ObservabilityFinding,
    ObservabilityKind, ObservabilityScan,
};
pub use l6_recovery::{run_l6_recovery_injection_scenarios, SubstrateRecoveryPolicy};
pub use l6_stability_driver::{run_l6_stability_driver, L6CycleResult, L6StabilityReport};
pub use layer3_synthetic::{
    run_layer3_single_graph_synthetic, Layer3Status, Layer3Subtest, Layer3SyntheticReport,
};
pub use layer4_synthetic::{
    run_layer4_two_graph_bridged_synthetic, Layer4Status, Layer4Subtest, Layer4SyntheticReport,
};
pub use layer5_live_llm::{
    run_layer5_live_llm_compute_roundtrip, run_layer5_live_llm_with_adapter, Layer5LiveLlmReport,
    Layer5ProviderConfig, Layer5Status, Layer5Subtest,
};
pub use mainnet_auth::{
    run_mainnet_auth, MainnetAuthReport, MainnetAuthStatus, MainnetAuthSubtest, MainnetIdentity,
};
pub use v1_runtime::{
    default_state_dir, dispatch_http_request, dispatch_mcp_request, run_http_loopback_smoke,
    run_v1_runtime_smoke, V1HttpLoopbackSmoke, V1HttpRequest, V1IdentityStore,
    V1IdentityStoreReport, V1ListenerDispatchReport, V1ListenerProtocol, V1MaintenanceScheduler,
    V1PersistedIdentity, V1RuntimeSmokeReport, V1ScheduledMaintenance, V1SchedulerTickReport,
};
pub use v1_surface::{
    compile_chain_definition, compile_chain_file, dispatch_http_route, dispatch_maintenance_task,
    dispatch_mcp_tool, execute_compiled_plan, load_action_definition, run_maintenance_once,
    run_v1_node_cli, CliSurface, HttpSurface, MaintenanceImplementation, MaintenanceSurface,
    McpSurface, V1CliCommand, V1ExecutedStep, V1ExecutionReport, V1MaintenanceRunReport,
    V1NodeSurfaceError, V1NodeSurfaceManifest, V1ProtocolBinding, V1ProtocolDispatch,
    V1ProtocolStatus, V1StepExecutionStatus, V1SurfaceDisposition,
};

#[cfg(test)]
mod tests {
    use super::*;
    use agent_wire_contracts::ContractVerb;

    #[test]
    fn node_runtime_composes_all_substrate_crates() {
        let runtime = compose_substrate_node(NodeConfig::demo().unwrap()).unwrap();

        assert_eq!(runtime.transport.driver_name, "cloudflare");
        assert_eq!(runtime.markets.contract_verb, ContractVerb::Wrap);
        assert_eq!(runtime.markets.compute_offer.model_id, "wire-demo-model");
        assert_eq!(runtime.markets.storage_offer.capacity_bytes, 1_000_000);
        assert_eq!(runtime.markets.relay_offer.capabilities.len(), 2);
        assert_eq!(runtime.compiler.logical_leaf_count(), 77);
        assert!(runtime.config.opt_in.compute_provider);
        assert!(runtime.config.opt_in.compute_requester);
    }

    #[test]
    fn v1_node_surface_exposes_cli_mcp_http_and_maintenance_bindings() {
        let manifest = V1NodeSurfaceManifest::v1();

        assert_eq!(manifest.cli.len(), 21);
        assert_eq!(manifest.mcp_tools.len(), 55);
        assert_eq!(manifest.http_routes.len(), 56);
        assert_eq!(manifest.implemented_maintenance_count(), 8);
        assert_eq!(manifest.stubbed_maintenance_count(), 4);
        assert!(manifest
            .cli
            .iter()
            .any(|surface| surface.command == V1CliCommand::ChainCompile));
    }

    #[test]
    fn parity_demo_covers_required_stage_10_behaviors() {
        let report = build_parity_demo().unwrap();

        assert!(parity_demo_assertions(&report));
        assert!(report
            .steps
            .iter()
            .any(|step| step.name == "tunnel-p2p-delivery"));
        assert!(report
            .steps
            .iter()
            .any(|step| step.name == "wake-up-triggers"));
    }

    #[test]
    fn layer3_synthetic_validation_covers_wave2_single_graph_checks() {
        let report = run_layer3_single_graph_synthetic().unwrap();

        assert!(report.all_green());
        assert_eq!(report.subtests.len(), 10);
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "provider-registers-requester-sees-provider"));
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "cloudflare-rotation-mid-flight"));
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "duplicate-job-through-rotated-tunnel-cannot-double-claim"));
    }

    #[test]
    fn layer4_synthetic_validation_covers_wave2_two_graph_checks() {
        let report = run_layer4_two_graph_bridged_synthetic().unwrap();

        assert!(report.all_green());
        assert_eq!(report.subtests.len(), 9);
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "identity-claim-master-signature-both-graphs"));
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "bridge-severed-graphs-independent"));
        assert!(report
            .subtests
            .iter()
            .any(|step| step.name == "reputation-snapshot-import-is-one-shot"));
    }

    #[test]
    fn layer5_live_llm_provider_config_supports_lm_studio() {
        let provider =
            Layer5ProviderConfig::lm_studio("granite-4-micro", "http://127.0.0.1:1234/v1");

        assert_eq!(provider.provider, "lm_studio");
        assert_eq!(provider.model_id, "granite-4-micro");
        assert_eq!(provider.base_url, "http://127.0.0.1:1234/v1");
        assert_eq!(provider.adapter_id, "lm-studio-chat-completions");
    }

    #[test]
    fn d3_live_validation_fails_closed_without_settlement_read_config() {
        if std::env::var("SUPABASE_SERVICE_ROLE_KEY").is_ok() {
            return;
        }

        let report = run_d3_live_compute_settlement();

        assert!(!report.all_green());
        assert_eq!(report.subtests[0].name, "d3-config-resolves");
    }
}
