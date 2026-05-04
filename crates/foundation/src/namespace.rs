use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NamespaceId(String);

impl NamespaceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn mainnet() -> Self {
        Self::new("wire.mainnet")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
