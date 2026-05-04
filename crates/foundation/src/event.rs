use agent_wire_contracts::{EventEnvelopeDto, EventVisibilityDto};
use serde::{Deserialize, Serialize};

use crate::namespace::NamespaceId;
use crate::refs::HandlePath;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceCrate(String);

impl SourceCrate {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventCursor(String);

impl EventCursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventVisibility {
    Public,
    Circle,
    Private,
}

impl From<EventVisibilityDto> for EventVisibility {
    fn from(value: EventVisibilityDto) -> Self {
        match value {
            EventVisibilityDto::Public => Self::Public,
            EventVisibilityDto::Circle => Self::Circle,
            EventVisibilityDto::Private => Self::Private,
        }
    }
}

impl From<EventVisibility> for EventVisibilityDto {
    fn from(value: EventVisibility) -> Self {
        match value {
            EventVisibility::Public => Self::Public,
            EventVisibility::Circle => Self::Circle,
            EventVisibility::Private => Self::Private,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub namespace: NamespaceId,
    pub source_crate: SourceCrate,
    pub event_type: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
    pub causal_ref: Option<HandlePath>,
    pub visibility: EventVisibility,
}

impl From<EventEnvelopeDto> for EventEnvelope {
    fn from(value: EventEnvelopeDto) -> Self {
        Self {
            namespace: NamespaceId::new(value.namespace),
            source_crate: SourceCrate::new(value.source_crate),
            event_type: value.event_type,
            payload: value.payload,
            correlation_id: value.correlation_id,
            causal_ref: value.causal_ref.map(HandlePath::from),
            visibility: value.visibility.into(),
        }
    }
}

impl From<EventEnvelope> for EventEnvelopeDto {
    fn from(value: EventEnvelope) -> Self {
        Self {
            namespace: value.namespace.as_str().to_owned(),
            source_crate: value.source_crate.0,
            event_type: value.event_type,
            payload: value.payload,
            correlation_id: value.correlation_id,
            causal_ref: value.causal_ref.map(Into::into),
            visibility: value.visibility.into(),
        }
    }
}
