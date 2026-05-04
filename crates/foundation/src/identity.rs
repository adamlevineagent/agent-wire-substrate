use agent_wire_contracts::{
    AliasVisibilityDto, HandleClaimDto, MasterKeyRotationDto, PrivateAliasMappingDto,
    PrivateGraphRegistrationDto, ReputationSnapshotDto,
};
use serde::{Deserialize, Serialize};

use crate::money::CreditAmount;
use crate::refs::GraphSlug;
use crate::transport::EndpointUrl;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Handle(String);

impl Handle {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for Handle {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<Handle> for String {
    fn from(value: Handle) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterPublicKey(String);

impl MasterPublicKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(String);

impl Signature {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailAttestation(String);

impl EmailAttestation {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandleClaim {
    pub handle: Handle,
    pub master_public_key: MasterPublicKey,
    pub claimed_at_wt: String,
    pub signature: Signature,
}

impl From<HandleClaimDto> for HandleClaim {
    fn from(value: HandleClaimDto) -> Self {
        Self {
            handle: Handle::new(value.handle),
            master_public_key: MasterPublicKey::new(value.master_public_key),
            claimed_at_wt: value.claimed_at_wt,
            signature: Signature::new(value.signature),
        }
    }
}

impl From<HandleClaim> for HandleClaimDto {
    fn from(value: HandleClaim) -> Self {
        Self {
            handle: value.handle.0,
            master_public_key: value.master_public_key.0,
            claimed_at_wt: value.claimed_at_wt,
            signature: value.signature.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AliasVisibility {
    Public,
    Scoped,
}

impl From<AliasVisibilityDto> for AliasVisibility {
    fn from(value: AliasVisibilityDto) -> Self {
        match value {
            AliasVisibilityDto::Public => Self::Public,
            AliasVisibilityDto::Scoped => Self::Scoped,
        }
    }
}

impl From<AliasVisibility> for AliasVisibilityDto {
    fn from(value: AliasVisibility) -> Self {
        match value {
            AliasVisibility::Public => Self::Public,
            AliasVisibility::Scoped => Self::Scoped,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateAliasMapping {
    pub graph_slug: GraphSlug,
    pub alias_handle: Handle,
    pub mainnet_handle: Handle,
    pub master_public_key: MasterPublicKey,
    pub visibility: AliasVisibility,
    pub signature: Signature,
}

impl From<PrivateAliasMappingDto> for PrivateAliasMapping {
    fn from(value: PrivateAliasMappingDto) -> Self {
        Self {
            graph_slug: GraphSlug::new(value.graph_slug),
            alias_handle: Handle::new(value.alias_handle),
            mainnet_handle: Handle::new(value.mainnet_handle),
            master_public_key: MasterPublicKey::new(value.master_public_key),
            visibility: value.visibility.into(),
            signature: Signature::new(value.signature),
        }
    }
}

impl From<PrivateAliasMapping> for PrivateAliasMappingDto {
    fn from(value: PrivateAliasMapping) -> Self {
        Self {
            graph_slug: value.graph_slug.into_inner(),
            alias_handle: value.alias_handle.0,
            mainnet_handle: value.mainnet_handle.0,
            master_public_key: value.master_public_key.0,
            visibility: value.visibility.into(),
            signature: value.signature.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateGraphRegistration {
    pub slug: GraphSlug,
    pub operator_handle: Handle,
    pub endpoint: EndpointUrl,
    pub annual_renewal: CreditAmount,
    pub grace_days: u16,
    pub competitive_bidding: bool,
    pub signature: Signature,
}

impl PrivateGraphRegistration {
    pub fn no_competitive_bidding(
        slug: GraphSlug,
        operator_handle: Handle,
        endpoint: EndpointUrl,
    ) -> Self {
        Self {
            slug,
            operator_handle,
            endpoint,
            annual_renewal: CreditAmount::ZERO,
            grace_days: 45,
            competitive_bidding: false,
            signature: Signature::new(""),
        }
    }
}

impl From<PrivateGraphRegistrationDto> for PrivateGraphRegistration {
    fn from(value: PrivateGraphRegistrationDto) -> Self {
        Self {
            slug: GraphSlug::new(value.slug),
            operator_handle: Handle::new(value.operator_handle),
            endpoint: EndpointUrl::new(value.endpoint),
            annual_renewal: CreditAmount::new(value.annual_renewal_credits),
            grace_days: value.grace_days,
            competitive_bidding: value.competitive_bidding,
            signature: Signature::new(value.signature),
        }
    }
}

impl From<PrivateGraphRegistration> for PrivateGraphRegistrationDto {
    fn from(value: PrivateGraphRegistration) -> Self {
        Self {
            slug: value.slug.into_inner(),
            operator_handle: value.operator_handle.0,
            endpoint: value.endpoint.into_inner(),
            annual_renewal_credits: value.annual_renewal.as_credits(),
            grace_days: value.grace_days,
            competitive_bidding: value.competitive_bidding,
            signature: value.signature.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasterKeyRotation {
    pub operator_email: String,
    pub old_master_public_key: MasterPublicKey,
    pub new_master_public_key: MasterPublicKey,
    pub email_attestation: EmailAttestation,
    pub rotated_at_wt: String,
}

impl From<MasterKeyRotationDto> for MasterKeyRotation {
    fn from(value: MasterKeyRotationDto) -> Self {
        Self {
            operator_email: value.operator_email,
            old_master_public_key: MasterPublicKey::new(value.old_master_public_key),
            new_master_public_key: MasterPublicKey::new(value.new_master_public_key),
            email_attestation: EmailAttestation::new(value.email_attestation),
            rotated_at_wt: value.rotated_at_wt,
        }
    }
}

impl From<MasterKeyRotation> for MasterKeyRotationDto {
    fn from(value: MasterKeyRotation) -> Self {
        Self {
            operator_email: value.operator_email,
            old_master_public_key: value.old_master_public_key.0,
            new_master_public_key: value.new_master_public_key.0,
            email_attestation: value.email_attestation.0,
            rotated_at_wt: value.rotated_at_wt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReputationSnapshot {
    pub graph_slug: GraphSlug,
    pub master_public_key: MasterPublicKey,
    pub score: i64,
    pub snapshot_at_wt: String,
    pub signature: Signature,
}

impl From<ReputationSnapshotDto> for ReputationSnapshot {
    fn from(value: ReputationSnapshotDto) -> Self {
        Self {
            graph_slug: GraphSlug::new(value.graph_slug),
            master_public_key: MasterPublicKey::new(value.master_public_key),
            score: value.score,
            snapshot_at_wt: value.snapshot_at_wt,
            signature: Signature::new(value.signature),
        }
    }
}

impl From<ReputationSnapshot> for ReputationSnapshotDto {
    fn from(value: ReputationSnapshot) -> Self {
        Self {
            graph_slug: value.graph_slug.into_inner(),
            master_public_key: value.master_public_key.0,
            score: value.score,
            snapshot_at_wt: value.snapshot_at_wt,
            signature: value.signature.0,
        }
    }
}
