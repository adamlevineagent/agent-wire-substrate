use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::namespace::NamespaceId;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(Uuid);

impl EventId {
    pub fn new(value: Uuid) -> Self {
        Self(value)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventCursor(String);

impl EventCursor {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventKind {
    MessageUnread,
    TaskMoved,
    TaskAssigned,
    ContributionPublished,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventEnvelope<T> {
    pub id: EventId,
    pub namespace: NamespaceId,
    pub kind: EventKind,
    pub occurred_at: OffsetDateTime,
    pub cursor: EventCursor,
    pub payload: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventTrigger {
    Kind(EventKind),
    Namespace(NamespaceId),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerFilter {
    pub triggers: Vec<EventTrigger>,
    pub after: Option<EventCursor>,
}
