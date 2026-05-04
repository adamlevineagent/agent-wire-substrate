//! Wire substrate foundation primitives.
//!
//! Foundation owns local runtime semantics. Bilateral wire DTOs live in
//! `agent-wire-contracts` and cross this boundary through explicit `From`
//! conversions in each module. Do not re-export contract DTOs from here.

pub mod event;
pub mod identity;
pub mod money;
pub mod namespace;
pub mod refs;
pub mod transport;

pub use event::{EventCursor, EventEnvelope, EventVisibility, SourceCrate};
pub use identity::{
    AliasVisibility, EmailAttestation, Handle, HandleClaim, MasterKeyRotation, MasterPublicKey,
    PrivateAliasMapping, PrivateGraphRegistration, ReputationSnapshot, Signature,
};
pub use money::CreditAmount;
pub use namespace::NamespaceId;
pub use refs::{GraphSlug, HandlePath};
pub use transport::{CallbackUrl, EndpointUrl, TransportDriver, TunnelUrl};
