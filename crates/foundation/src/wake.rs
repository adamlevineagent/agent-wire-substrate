use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{EventCursor, FoundationError};

pub const MAX_WAKE_TRIGGER_BYTES: usize = 96;
pub const MAX_WAKE_FILTER_KEY_BYTES: usize = 64;
pub const MAX_WAKE_FILTER_VALUE_BYTES: usize = 256;
pub const MAX_WAKE_TIMEOUT_SECONDS: u64 = 1_800;
pub const DEFAULT_WAKE_LIMIT: u16 = 50;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct WakeTrigger(String);

impl WakeTrigger {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "wake_trigger",
            });
        }
        if value.len() > MAX_WAKE_TRIGGER_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "wake_trigger",
            });
        }
        if !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(byte, b'.' | b'_' | b'-' | b'*')
        }) {
            return Err(FoundationError::InvalidCharacter {
                field: "wake_trigger",
            });
        }
        Ok(Self(value))
    }

    pub fn message_unread() -> Self {
        Self("message.unread".to_owned())
    }

    pub fn task_moved() -> Self {
        Self("task.moved".to_owned())
    }

    pub fn task_assigned() -> Self {
        Self("task.assigned".to_owned())
    }

    pub fn board_changed() -> Self {
        Self("board.changed".to_owned())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for WakeTrigger {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<WakeTrigger> for String {
    fn from(value: WakeTrigger) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeFilter {
    pub key: String,
    pub value: String,
}

impl WakeFilter {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Result<Self, FoundationError> {
        let key = key.into();
        let value = value.into();
        if key.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "wake_filter_key",
            });
        }
        if key.len() > MAX_WAKE_FILTER_KEY_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "wake_filter_key",
            });
        }
        if !key.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        }) {
            return Err(FoundationError::InvalidCharacter {
                field: "wake_filter_key",
            });
        }
        if value.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "wake_filter_value",
            });
        }
        if value.len() > MAX_WAKE_FILTER_VALUE_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "wake_filter_value",
            });
        }
        Ok(Self { key, value })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeRequest {
    pub triggers: Vec<WakeTrigger>,
    pub cursor: Option<EventCursor>,
    pub filters: Vec<WakeFilter>,
    pub timeout_seconds: u64,
    pub limit: u16,
}

impl WakeRequest {
    pub fn new(triggers: Vec<WakeTrigger>) -> Result<Self, FoundationError> {
        if triggers.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "wake_triggers",
            });
        }
        Ok(Self {
            triggers,
            cursor: None,
            filters: Vec::new(),
            timeout_seconds: 30,
            limit: DEFAULT_WAKE_LIMIT,
        })
    }

    pub fn with_cursor(mut self, cursor: EventCursor) -> Self {
        self.cursor = Some(cursor);
        self
    }

    pub fn with_filter(mut self, filter: WakeFilter) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, FoundationError> {
        let seconds = timeout.as_secs();
        if seconds > MAX_WAKE_TIMEOUT_SECONDS {
            return Err(FoundationError::OutOfRange {
                field: "wake_timeout_seconds",
            });
        }
        self.timeout_seconds = seconds;
        Ok(self)
    }

    pub fn with_limit(mut self, limit: u16) -> Result<Self, FoundationError> {
        if limit == 0 {
            return Err(FoundationError::OutOfRange {
                field: "wake_limit",
            });
        }
        self.limit = limit;
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WakeEvent {
    pub trigger: WakeTrigger,
    pub cursor: EventCursor,
    pub payload: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WakeBatch {
    pub events: Vec<WakeEvent>,
    pub next_cursor: Option<EventCursor>,
}

impl WakeBatch {
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

pub trait WakeRuntime {
    type Error;

    fn poll(&self, request: WakeRequest) -> Result<WakeBatch, Self::Error>;
    fn wait(&self, request: WakeRequest) -> Result<WakeBatch, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_trigger_rejects_unbounded_free_strings() {
        assert!(WakeTrigger::new("message.unread").is_ok());
        assert_eq!(
            WakeTrigger::new("MessageUnread"),
            Err(FoundationError::InvalidCharacter {
                field: "wake_trigger"
            })
        );
        assert_eq!(
            WakeTrigger::new("x".repeat(MAX_WAKE_TRIGGER_BYTES + 1)),
            Err(FoundationError::OutOfRange {
                field: "wake_trigger"
            })
        );
    }

    #[test]
    fn wake_request_bounds_timeout_and_limit() {
        let request = WakeRequest::new(vec![
            WakeTrigger::message_unread(),
            WakeTrigger::task_assigned(),
        ])
        .unwrap()
        .with_cursor(EventCursor::new("wv1.cursor"))
        .with_filter(WakeFilter::new("assignee", "me").unwrap())
        .with_timeout(Duration::from_secs(60))
        .unwrap()
        .with_limit(10)
        .unwrap();

        assert_eq!(request.triggers.len(), 2);
        assert_eq!(request.cursor.unwrap().as_str(), "wv1.cursor");
        assert_eq!(request.filters[0].key, "assignee");
        assert_eq!(request.timeout_seconds, 60);
        assert_eq!(request.limit, 10);

        assert_eq!(
            WakeRequest::new(vec![WakeTrigger::board_changed()])
                .unwrap()
                .with_timeout(Duration::from_secs(MAX_WAKE_TIMEOUT_SECONDS + 1)),
            Err(FoundationError::OutOfRange {
                field: "wake_timeout_seconds"
            })
        );
    }

    #[test]
    fn wake_filter_bounds_protocol_edge_shape() {
        assert!(WakeFilter::new("task_id", "af9a096b-9afc-4a17-b6f5-4e8ff2ccffd7").is_ok());
        assert_eq!(
            WakeFilter::new("TaskId", "value"),
            Err(FoundationError::InvalidCharacter {
                field: "wake_filter_key"
            })
        );
        assert_eq!(
            WakeFilter::new("x".repeat(MAX_WAKE_FILTER_KEY_BYTES + 1), "value"),
            Err(FoundationError::OutOfRange {
                field: "wake_filter_key"
            })
        );
        assert_eq!(
            WakeFilter::new("task_id", "x".repeat(MAX_WAKE_FILTER_VALUE_BYTES + 1)),
            Err(FoundationError::OutOfRange {
                field: "wake_filter_value"
            })
        );
    }

    #[test]
    fn wake_batch_reports_empty_state() {
        let batch = WakeBatch {
            events: Vec::new(),
            next_cursor: Some(EventCursor::new("wv1.next")),
        };

        assert!(batch.is_empty());
    }
}
