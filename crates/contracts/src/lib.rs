//! Bilateral Wire protocol DTOs.
//!
//! These are serialized contract shapes. Runtime semantics live in
//! `agent-wire-foundation` and cross this boundary through explicit
//! conversions rather than re-exports.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractVerb {
    Wrap,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractWrap<T> {
    pub verb: ContractVerb,
    pub payload: T,
}

impl<T> ContractWrap<T> {
    pub fn wrap(payload: T) -> Self {
        Self {
            verb: ContractVerb::Wrap,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandlePathDto {
    pub handle: String,
    pub wire_day: u32,
    pub graph_slug: Option<String>,
    pub sequence: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelEndpointDto {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoneyAmountDto {
    pub credits: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleClaimDto {
    pub handle: String,
    pub master_public_key: String,
    pub claimed_at_wt: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateAliasMappingDto {
    pub graph_slug: String,
    pub alias_handle: String,
    pub mainnet_handle: String,
    pub master_public_key: String,
    pub visibility: AliasVisibilityDto,
    pub signature: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AliasVisibilityDto {
    Public,
    Scoped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateGraphRegistrationDto {
    pub slug: String,
    pub operator_handle: String,
    pub endpoint: String,
    pub annual_renewal_credits: u64,
    pub grace_days: u16,
    pub competitive_bidding: bool,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterKeyRotationDto {
    pub operator_email: String,
    pub old_master_public_key: String,
    pub new_master_public_key: String,
    pub email_attestation: String,
    pub rotated_at_wt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationSnapshotDto {
    pub graph_slug: String,
    pub master_public_key: String,
    pub score: i64,
    pub snapshot_at_wt: String,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelopeDto {
    pub namespace: String,
    pub source_crate: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
    pub correlation_id: Option<String>,
    pub causal_ref: Option<HandlePathDto>,
    pub visibility: EventVisibilityDto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventVisibilityDto {
    Public,
    Circle,
    Private,
}
