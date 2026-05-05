//! Umbrella composition library for the Agent Wire substrate stack.
//!
//! The public surface here is the reference-client composition layer: node
//! config, operator API surface, background lifecycle, market composition, and
//! dry-run parity demo. The binary crate stays a CLI shell around this library
//! plus validation commands.

pub mod boot;
pub mod config;
pub mod lifecycle;
pub mod parity_demo;
pub mod server;

pub use agent_wire_compiler::{
    CompilerOpManifest, WireActionDefinition, WireActionStep, WireCompiledPlan, WireCompiler,
};
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
    fn runtime_composes_all_substrate_crates() {
        let runtime = compose_substrate_node(NodeConfig::demo().unwrap()).unwrap();

        assert_eq!(runtime.transport.driver_name, "cloudflare");
        assert_eq!(runtime.markets.contract_verb, ContractVerb::Wrap);
        assert_eq!(runtime.markets.compute_offer.model_id, "wire-demo-model");
        assert_eq!(runtime.markets.storage_offer.capacity_bytes, 1_000_000);
        assert_eq!(runtime.markets.relay_offer.capabilities.len(), 2);
        assert_eq!(runtime.compiler.logical_leaf_count(), 77);
        assert_eq!(runtime.vocabulary.term().key.as_str(), "compute-market");
        assert!(runtime.config.opt_in.compute_provider);
        assert!(runtime.config.opt_in.compute_requester);
    }

    #[test]
    fn parity_demo_covers_reference_client_behaviors() {
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
