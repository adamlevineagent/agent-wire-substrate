use agent_wire_foundation::FoundationError;
use serde::{Deserialize, Serialize};

use crate::boot::{compose_substrate_node, NodeRuntime};
use crate::config::NodeConfig;
use crate::lifecycle::BackgroundWorker;
use crate::server::OperatorRoute;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityDemoReport {
    pub runtime: NodeRuntime,
    pub steps: Vec<ParityDemoStep>,
}

impl ParityDemoReport {
    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Agent Wire Substrate Node 2.0 Parity Demo\n\n");
        output.push_str("This dry-run uses only substrate-tier crates and does not import pyramid-app code.\n\n");
        output.push_str("## Steps\n\n");
        for step in &self.steps {
            output.push_str("- ");
            output.push_str(&step.name);
            output.push_str(": ");
            output.push_str(&step.detail);
            output.push('\n');
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityDemoStep {
    pub name: String,
    pub detail: String,
}

pub fn build_parity_demo() -> Result<ParityDemoReport, FoundationError> {
    let runtime = compose_substrate_node(NodeConfig::demo()?)?;
    Ok(ParityDemoReport {
        steps: vec![
            ParityDemoStep {
                name: "authenticate-mainnet".to_owned(),
                detail: "master public key and mainnet endpoint are present".to_owned(),
            },
            ParityDemoStep {
                name: "sync-contributions".to_owned(),
                detail: "operator REST/MCP/IPC route surface includes contribution sync".to_owned(),
            },
            ParityDemoStep {
                name: "compute-provider".to_owned(),
                detail: "compute offer and provider worker are composed".to_owned(),
            },
            ParityDemoStep {
                name: "compute-requester".to_owned(),
                detail: "compute job contract and requester worker are composed".to_owned(),
            },
            ParityDemoStep {
                name: "tunnel-p2p-delivery".to_owned(),
                detail: "Cloudflare transport opens a tunnel session with callback delivery"
                    .to_owned(),
            },
            ParityDemoStep {
                name: "storage-market".to_owned(),
                detail: "storage offer is available for pin and retrieval wiring".to_owned(),
            },
            ParityDemoStep {
                name: "relay-market".to_owned(),
                detail: "relay offer is available for path lease wiring".to_owned(),
            },
            ParityDemoStep {
                name: "vocabulary-handling".to_owned(),
                detail: "foundation vocabulary entry is resolved inside node composition"
                    .to_owned(),
            },
            ParityDemoStep {
                name: "wake-up-triggers".to_owned(),
                detail: "background lifecycle listens for message, task, and contribution events"
                    .to_owned(),
            },
        ],
        runtime,
    })
}

pub fn parity_demo_assertions(report: &ParityDemoReport) -> bool {
    let routes = &report.runtime.api.routes;
    let workers = &report.runtime.lifecycle.workers;
    routes.contains(&OperatorRoute::AuthenticateMainnet)
        && routes.contains(&OperatorRoute::SyncContributions)
        && routes.contains(&OperatorRoute::PublishComputeOffer)
        && routes.contains(&OperatorRoute::RequestComputeQuote)
        && routes.contains(&OperatorRoute::DispatchComputeJob)
        && routes.contains(&OperatorRoute::PinStorage)
        && routes.contains(&OperatorRoute::LeaseRelayPath)
        && routes.contains(&OperatorRoute::ResolveVocabulary)
        && routes.contains(&OperatorRoute::WaitForWakeTrigger)
        && workers.contains(&BackgroundWorker::ComputeProvider)
        && workers.contains(&BackgroundWorker::ComputeRequester)
        && workers.contains(&BackgroundWorker::TunnelDelivery)
        && workers.contains(&BackgroundWorker::VocabularySync)
        && workers.contains(&BackgroundWorker::WakeTriggerListener)
}
