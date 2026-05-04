//! Operator bundle composition crate.
//!
//! The boot/API split is intentionally outside foundation. This crate hosts
//! substrate boot and substrate API composition while leaving vertical APIs to
//! downstream application crates.

pub mod boot;
pub mod config;
pub mod lifecycle;
pub mod parity_demo;
pub mod server;

pub use boot::{compose_substrate_node, MarketComposition, NodeRuntime};
pub use config::{LocalPersistence, NodeConfig, NodeKeys, OptInPolicy};
pub use lifecycle::{BackgroundWorker, BackgroundWorkerLifecycle};
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
}
