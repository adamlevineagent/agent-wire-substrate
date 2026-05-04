use serde::{Deserialize, Serialize};

use crate::namespace::{validate_slug, NamespaceId};
use crate::refs::CrossGraphRef;
use crate::FoundationError;

pub const MAX_VOCABULARY_LABEL_BYTES: usize = 120;
pub const MAX_VOCABULARY_DESCRIPTION_BYTES: usize = 2_000;

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
        if is_reserved_primitive_name(&value) {
            return Err(FoundationError::ReservedName {
                field: "vocabulary_key",
            });
        }
        Ok(Self(value))
    }

    pub fn system(value: impl Into<String>) -> Result<Self, FoundationError> {
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
    term: VocabularyTermRef,
    label: String,
    description: Option<String>,
}

impl VocabularyEntry {
    pub fn new(
        term: VocabularyTermRef,
        label: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Result<Self, FoundationError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(FoundationError::EmptyField {
                field: "vocabulary_label",
            });
        }
        if label.len() > MAX_VOCABULARY_LABEL_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "vocabulary_label",
            });
        }
        let description = description.map(Into::into);
        if description
            .as_ref()
            .is_some_and(|value| value.len() > MAX_VOCABULARY_DESCRIPTION_BYTES)
        {
            return Err(FoundationError::OutOfRange {
                field: "vocabulary_description",
            });
        }
        Ok(Self {
            term,
            label,
            description,
        })
    }

    pub fn term(&self) -> &VocabularyTermRef {
        &self.term
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub trait VocabularyResolver {
    type Error;

    fn resolve(&self, term: &VocabularyTermRef) -> Result<VocabularyEntry, Self::Error>;
}

pub fn is_reserved_primitive_name(value: &str) -> bool {
    matches!(
        value,
        "compute-market"
            | "storage-market"
            | "relay-market"
            | "transport-cloudflare"
            | "identity"
            | "reputation"
            | "sandbox"
            | "contracts"
            | "foundation"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_vocabulary_cannot_hijack_reserved_primitives() {
        assert_eq!(
            VocabularyKey::new("compute-market"),
            Err(FoundationError::ReservedName {
                field: "vocabulary_key"
            })
        );
        assert!(VocabularyKey::system("compute-market").is_ok());
    }

    #[test]
    fn vocabulary_entry_caps_user_payloads() {
        let vocabulary =
            VocabularyNamespace::new(NamespaceId::new("playful").unwrap(), "wire-v2").unwrap();
        let term = VocabularyTermRef {
            vocabulary,
            key: VocabularyKey::new("safe-term").unwrap(),
            definition_ref: "playful/123/vocabulary/1".parse().unwrap(),
        };

        assert_eq!(
            VocabularyEntry::new(term.clone(), "", None::<String>),
            Err(FoundationError::EmptyField {
                field: "vocabulary_label"
            })
        );
        assert_eq!(
            VocabularyEntry::new(
                term.clone(),
                "x".repeat(MAX_VOCABULARY_LABEL_BYTES + 1),
                None::<String>
            ),
            Err(FoundationError::OutOfRange {
                field: "vocabulary_label"
            })
        );
        assert!(VocabularyEntry::new(
            term,
            "Safe Term",
            Some("bounded definition anchored by typed CrossGraphRef")
        )
        .is_ok());
    }
}
