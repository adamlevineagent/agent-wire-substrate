#![forbid(unsafe_code)]

pub mod contracts_boundary;
pub mod economics;
pub mod error;
pub mod events;
pub mod identity;
pub mod namespace;
pub mod refs;
pub mod sandbox;
pub mod transport;
pub mod vocabulary;

#[cfg(test)]
mod dependency_guard;

pub use economics::{CreditAmount, PriceCurve, SettlementIntent};
pub use error::FoundationError;
pub use events::{EventCursor, EventEnvelope, EventId, EventKind, EventTrigger, TriggerFilter};
pub use identity::{
    HandleClaim, MasterKeyId, MasterKeyRotation, MasterPublicKey, MasterSignature, MasterSigner,
    MasterVerifier, OperatorEmail, PrivateAliasMapping, PrivateGraphRegistration,
    ReputationSnapshot, SignatureAlgorithm, SignedStatement,
};
pub use namespace::{GraphKind, GraphSlug, NamespaceId, ReputationRegistryId};
pub use refs::{ContributionRef, CrossGraphRef, HandlePath};
pub use sandbox::{Capability, CapabilityGrant, ResourceBudget, SandboxPolicy};
pub use transport::{
    CallbackUrl, EndpointUrl, PublicEndpoint, TransportDriver, TunnelRequest, TunnelSession,
    TunnelUrl,
};
pub use vocabulary::{
    VocabularyEntry, VocabularyKey, VocabularyNamespace, VocabularyResolver, VocabularyTermRef,
};
