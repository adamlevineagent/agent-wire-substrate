use agent_wire_foundation::EndpointUrl;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorApiSurface {
    pub endpoint: EndpointUrl,
    pub protocols: Vec<OperatorApiProtocol>,
    pub routes: Vec<OperatorRoute>,
}

impl OperatorApiSurface {
    pub fn all_enabled(endpoint: EndpointUrl) -> Self {
        Self {
            endpoint,
            protocols: vec![
                OperatorApiProtocol::OperatorHttp,
                OperatorApiProtocol::Mcp,
                OperatorApiProtocol::Ipc,
                OperatorApiProtocol::Rest,
            ],
            routes: vec![
                OperatorRoute::AuthenticateMainnet,
                OperatorRoute::SyncContributions,
                OperatorRoute::PublishComputeOffer,
                OperatorRoute::RequestComputeQuote,
                OperatorRoute::DispatchComputeJob,
                OperatorRoute::PinStorage,
                OperatorRoute::LeaseRelayPath,
                OperatorRoute::ResolveVocabulary,
                OperatorRoute::WaitForWakeTrigger,
            ],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorApiProtocol {
    OperatorHttp,
    Mcp,
    Ipc,
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatorRoute {
    AuthenticateMainnet,
    SyncContributions,
    PublishComputeOffer,
    RequestComputeQuote,
    DispatchComputeJob,
    PinStorage,
    LeaseRelayPath,
    ResolveVocabulary,
    WaitForWakeTrigger,
}
