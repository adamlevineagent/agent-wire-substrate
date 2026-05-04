use agent_wire_foundation::{EndpointUrl, TunnelUrl};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelayRoute {
    pub from: EndpointUrl,
    pub to: TunnelUrl,
}
