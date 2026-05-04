//! Operator bundle composition crate.
//!
//! The boot/API split is intentionally outside foundation. This crate hosts
//! substrate boot and substrate API composition while leaving vertical APIs to
//! downstream application crates.

pub mod boot;
pub mod config;
pub mod contribution_sync;
pub mod layer3_synthetic;
pub mod layer4_synthetic;
pub mod layer5_live_llm;
pub mod lifecycle;
pub mod mainnet_auth;
pub mod parity_demo;
pub mod server;

pub use boot::{compose_substrate_node, MarketComposition, NodeRuntime};
pub use config::{LocalPersistence, NodeConfig, NodeKeys, OptInPolicy};
pub use contribution_sync::{
    run_live_contribution_sync, ContributionSyncItem, ContributionSyncReport,
    ContributionSyncStatus, ContributionSyncSubtest,
};
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
pub use lifecycle::{BackgroundWorker, BackgroundWorkerLifecycle};
pub use mainnet_auth::{
    run_mainnet_auth, MainnetAuthReport, MainnetAuthStatus, MainnetAuthSubtest, MainnetIdentity,
};
pub use parity_demo::{
    build_parity_demo, parity_demo_assertions, ParityDemoReport, ParityDemoStep,
};
pub use server::{OperatorApiProtocol, OperatorApiSurface, OperatorRoute};

pub fn substrate_stack_name() -> &'static str {
    "agent-wire-substrate"
}

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
        assert!(runtime.config.opt_in.compute_provider);
        assert!(runtime.config.opt_in.compute_requester);
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
    fn layer5_live_llm_validation_fails_closed_without_provider_config() {
        if std::env::var("OPENROUTER_API_KEY").is_ok() {
            return;
        }

        let report = run_layer5_live_llm_compute_roundtrip();

        assert!(!report.all_green());
        assert_eq!(report.subtests[0].name, "provider-config-resolves-live-llm");
    }
}
