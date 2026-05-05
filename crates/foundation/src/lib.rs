#![forbid(unsafe_code)]

pub mod contracts_boundary;
pub mod economics;
pub mod error;
pub mod events;
pub mod identity;
pub mod mirror;
pub mod namespace;
pub mod refs;
pub mod sandbox;
pub mod transport;
pub mod vocabulary;
pub mod wake;

#[cfg(test)]
mod dependency_guard;

pub use economics::{
    CreditAmount, FillKey, IdempotencyKey, PriceCurve, QuoteReceipt, SettlementCommit,
    SettlementIntent, SettlementSettled,
};
pub use error::FoundationError;
pub use events::{EventCursor, EventEnvelope, EventId, EventKind, EventTrigger, TriggerFilter};
pub use identity::{
    HandleClaim, MasterKeyId, MasterKeyRotation, MasterPublicKey, MasterSignature, MasterSigner,
    MasterVerifier, OperatorEmail, PrivateAliasMapping, PrivateGraphRegistration,
    ReputationSnapshot, SignatureAlgorithm, SignedStatement,
};
pub use mirror::{
    compute_mirror_diff, CachedDocument, ContentHash, CorpusSlug, LocalDocumentSnapshot,
    MirrorConflict, MirrorDiff, MirrorDirection, MirrorFileStatus, MirrorLink, MirrorPath,
    MirrorState, RemoteDocumentInfo,
};
pub use namespace::{GraphKind, GraphSlug, NamespaceId, ReputationRegistryId};
pub use refs::{ContributionRef, CrossGraphRef, HandlePath};
pub use sandbox::{
    BoundSandboxPolicy, BudgetAccountant, Capability, CapabilityGrant, ExtensionCapability,
    ResourceBudget, SandboxPolicy, DEFAULT_MAX_HEAP_BYTES, DEFAULT_MAX_RECURSION_DEPTH,
    DEFAULT_MAX_STACK_DEPTH,
};
pub use transport::{
    CallbackUrl, EndpointUrl, PublicEndpoint, TransportDriver, TunnelRequest, TunnelSession,
    TunnelUrl,
};
pub use vocabulary::{
    canonical_ops, VocabularyEntry, VocabularyKey, VocabularyNamespace, VocabularyResolver,
    VocabularyTermRef,
};
pub use wake::{
    WakeBatch, WakeEvent, WakeFilter, WakeRequest, WakeRuntime, WakeTrigger, DEFAULT_WAKE_LIMIT,
    MAX_WAKE_FILTER_KEY_BYTES, MAX_WAKE_FILTER_VALUE_BYTES, MAX_WAKE_TIMEOUT_SECONDS,
    MAX_WAKE_TRIGGER_BYTES,
};
