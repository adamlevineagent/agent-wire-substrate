use serde::{Deserialize, Serialize};

use crate::economics::CreditAmount;
use crate::namespace::validate_slug;
use crate::FoundationError;

pub const MAX_EXTENSION_CAPABILITY_BYTES: usize = 64;
pub const MAX_CAPABILITY_REASON_BYTES: usize = 240;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    ReadContribution,
    WriteContribution,
    OpenTunnel,
    ExecuteModel,
    EmitEvent,
    Extension(ExtensionCapability),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExtensionCapability(String);

impl ExtensionCapability {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_slug("extension_capability", &value)?;
        if value.len() > MAX_EXTENSION_CAPABILITY_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "extension_capability",
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    capability: Capability,
    reason: String,
}

impl CapabilityGrant {
    pub fn new(capability: Capability, reason: impl Into<String>) -> Result<Self, FoundationError> {
        let reason = reason.into();
        if reason.trim().is_empty() {
            return Err(FoundationError::EmptyField {
                field: "capability_reason",
            });
        }
        if reason.len() > MAX_CAPABILITY_REASON_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "capability_reason",
            });
        }
        Ok(Self { capability, reason })
    }

    pub fn capability(&self) -> &Capability {
        &self.capability
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    max_credits: CreditAmount,
    max_events: u64,
    max_wall_time_ms: u64,
}

impl ResourceBudget {
    pub fn new(
        max_credits: CreditAmount,
        max_events: u64,
        max_wall_time_ms: u64,
    ) -> Result<Self, FoundationError> {
        if max_credits == CreditAmount::zero() {
            return Err(FoundationError::OutOfRange {
                field: "max_credits",
            });
        }
        if max_events == 0 {
            return Err(FoundationError::OutOfRange {
                field: "max_events",
            });
        }
        if max_wall_time_ms == 0 {
            return Err(FoundationError::OutOfRange {
                field: "max_wall_time_ms",
            });
        }
        Ok(Self {
            max_credits,
            max_events,
            max_wall_time_ms,
        })
    }

    pub fn max_credits(&self) -> CreditAmount {
        self.max_credits
    }

    pub fn max_events(&self) -> u64 {
        self.max_events
    }

    pub fn max_wall_time_ms(&self) -> u64 {
        self.max_wall_time_ms
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    grants: Vec<CapabilityGrant>,
    budget: ResourceBudget,
}

impl SandboxPolicy {
    pub fn new(
        grants: Vec<CapabilityGrant>,
        budget: ResourceBudget,
    ) -> Result<Self, FoundationError> {
        if grants.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "capability_grants",
            });
        }
        Ok(Self { grants, budget })
    }

    pub fn grants(&self) -> &[CapabilityGrant] {
        &self.grants
    }

    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }

    pub fn bind(&self) -> BoundSandboxPolicy {
        BoundSandboxPolicy {
            grants: self.grants.clone(),
            budget: self.budget.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundSandboxPolicy {
    grants: Vec<CapabilityGrant>,
    budget: ResourceBudget,
}

impl BoundSandboxPolicy {
    pub fn allows(&self, capability: &Capability) -> bool {
        self.grants
            .iter()
            .any(|grant| grant.capability() == capability)
    }

    pub fn budget(&self) -> &ResourceBudget {
        &self.budget
    }
}

pub trait BudgetAccountant {
    type Error;

    fn reserve_credits(
        &mut self,
        amount: CreditAmount,
        policy: &BoundSandboxPolicy,
    ) -> Result<(), Self::Error>;

    fn record_event(&mut self, policy: &BoundSandboxPolicy) -> Result<(), Self::Error>;

    fn elapsed_wall_time_ms(&self) -> u64;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_capability_names_are_foundation_bounded() {
        assert!(ExtensionCapability::new("provider-foo").is_ok());
        assert_eq!(
            ExtensionCapability::new("ProviderFoo"),
            Err(FoundationError::InvalidCharacter {
                field: "extension_capability"
            })
        );
        assert_eq!(
            ExtensionCapability::new("x".repeat(MAX_EXTENSION_CAPABILITY_BYTES + 1)),
            Err(FoundationError::OutOfRange {
                field: "extension_capability"
            })
        );
    }

    #[test]
    fn sandbox_policy_binds_immutable_grants_and_budget() {
        let grant =
            CapabilityGrant::new(Capability::ReadContribution, "sync mainnet contributions")
                .unwrap();
        let budget = ResourceBudget::new(CreditAmount::from_sats(10), 1, 1_000).unwrap();
        let policy = SandboxPolicy::new(vec![grant], budget).unwrap();
        let bound = policy.bind();

        assert!(bound.allows(&Capability::ReadContribution));
        assert_eq!(bound.budget().max_credits().as_sats(), 10);
    }
}
