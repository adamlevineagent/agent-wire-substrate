use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::namespace::{GraphKind, GraphSlug, NamespaceId, ReputationRegistryId};
use crate::refs::{CrossGraphRef, HandlePath};
use crate::FoundationError;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MasterKeyId(String);

impl MasterKeyId {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(FoundationError::EmptyField {
                field: "master_key_id",
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MasterPublicKey {
    pub key_id: MasterKeyId,
    pub algorithm: SignatureAlgorithm,
    pub bytes: Vec<u8>,
}

impl MasterPublicKey {
    pub fn new(
        key_id: MasterKeyId,
        algorithm: SignatureAlgorithm,
        bytes: Vec<u8>,
    ) -> Result<Self, FoundationError> {
        if bytes.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "master_public_key",
            });
        }
        Ok(Self {
            key_id,
            algorithm,
            bytes,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MasterSignature {
    pub key_id: MasterKeyId,
    pub algorithm: SignatureAlgorithm,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    Ed25519,
    Secp256k1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedStatement<T> {
    pub statement: T,
    pub signed_at: OffsetDateTime,
    pub signature: MasterSignature,
}

pub trait MasterSigner {
    type Error;

    fn sign<T: Serialize>(&self, statement: &T) -> Result<MasterSignature, Self::Error>;
}

pub trait MasterVerifier {
    type Error;

    fn verify<T: Serialize>(
        &self,
        public_key: &MasterPublicKey,
        statement: &T,
        signature: &MasterSignature,
    ) -> Result<(), Self::Error>;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorEmail(String);

impl OperatorEmail {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if !value.contains('@') {
            return Err(FoundationError::InvalidFormat {
                field: "operator_email",
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleClaim {
    pub handle: HandlePath,
    pub namespace: NamespaceId,
    pub master_key: MasterPublicKey,
    pub operator_email: Option<OperatorEmail>,
    pub issued_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateAliasMapping {
    pub private_alias: HandlePath,
    pub public_handle: Option<HandlePath>,
    pub namespace: NamespaceId,
    pub signed_ref: CrossGraphRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateGraphRegistration {
    pub namespace: NamespaceId,
    pub graph_slug: GraphSlug,
    pub graph_kind: GraphKind,
    pub master_key: MasterPublicKey,
    pub reputation_registry: Option<ReputationRegistryId>,
    pub registered_at: OffsetDateTime,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterKeyRotation {
    pub namespace: NamespaceId,
    pub previous_key: MasterKeyId,
    pub next_key: MasterPublicKey,
    pub effective_at: OffsetDateTime,
    pub proof_ref: CrossGraphRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationSnapshot {
    pub namespace: NamespaceId,
    pub registry: ReputationRegistryId,
    pub source_ref: CrossGraphRef,
    pub exported_at: OffsetDateTime,
    pub signature: MasterSignature,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_public_key_requires_bytes() {
        let key_id = MasterKeyId::new("primary").unwrap();

        assert_eq!(
            MasterPublicKey::new(key_id, SignatureAlgorithm::Ed25519, vec![]),
            Err(FoundationError::EmptyField {
                field: "master_public_key"
            })
        );
    }
}
