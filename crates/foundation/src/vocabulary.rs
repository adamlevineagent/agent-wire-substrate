use serde::{Deserialize, Serialize};

use crate::namespace::{validate_slug, NamespaceId};
use crate::refs::CrossGraphRef;
use crate::FoundationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VocabularyNamespace {
    pub namespace: NamespaceId,
    pub name: String,
}

impl VocabularyNamespace {
    pub fn new(namespace: NamespaceId, name: impl Into<String>) -> Result<Self, FoundationError> {
        let name = name.into();
        validate_slug("vocabulary_namespace", &name)?;
        Ok(Self { namespace, name })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VocabularyKey(String);

impl VocabularyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_slug("vocabulary_key", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyTermRef {
    pub vocabulary: VocabularyNamespace,
    pub key: VocabularyKey,
    pub definition_ref: CrossGraphRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyEntry {
    pub term: VocabularyTermRef,
    pub label: String,
    pub description: Option<String>,
}

pub trait VocabularyResolver {
    type Error;

    fn resolve(&self, term: &VocabularyTermRef) -> Result<VocabularyEntry, Self::Error>;
}
