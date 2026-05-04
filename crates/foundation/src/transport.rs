use agent_wire_contracts::TunnelEndpointDto;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointUrl(String);

impl EndpointUrl {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallbackUrl(String);

impl CallbackUrl {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TunnelUrl(EndpointUrl);

impl TunnelUrl {
    pub fn new(value: impl Into<String>) -> Self {
        Self(EndpointUrl::new(value))
    }

    pub fn endpoint(&self) -> &EndpointUrl {
        &self.0
    }
}

impl From<TunnelEndpointDto> for TunnelUrl {
    fn from(value: TunnelEndpointDto) -> Self {
        Self::new(value.url)
    }
}

impl From<TunnelUrl> for TunnelEndpointDto {
    fn from(value: TunnelUrl) -> Self {
        Self {
            url: value.0.into_inner(),
        }
    }
}

pub trait TransportDriver {
    fn driver_name(&self) -> &'static str;
    fn tunnel_url(&self) -> Option<TunnelUrl>;
}
