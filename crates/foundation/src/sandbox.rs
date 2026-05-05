use serde::{Deserialize, Serialize};

use crate::economics::CreditAmount;
use crate::namespace::validate_slug;
use crate::FoundationError;

pub const MAX_EXTENSION_CAPABILITY_BYTES: usize = 64;
pub const MAX_CAPABILITY_REASON_BYTES: usize = 240;
pub const DEFAULT_MAX_STACK_DEPTH: u32 = 64;
pub const DEFAULT_MAX_HEAP_BYTES: u64 = 16 * 1024 * 1024;
pub const DEFAULT_MAX_RECURSION_DEPTH: u32 = 32;

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
#[serde(try_from = "ResourceBudgetParts", into = "ResourceBudgetParts")]
pub struct ResourceBudget {
    max_credits: CreditAmount,
    max_events: u64,
    max_wall_time_ms: u64,
    max_stack_depth: u32,
    max_heap_bytes: u64,
    max_recursion_depth: u32,
}

impl ResourceBudget {
    pub fn new(
        max_credits: CreditAmount,
        max_events: u64,
        max_wall_time_ms: u64,
    ) -> Result<Self, FoundationError> {
        Self::bounded(
            max_credits,
            max_events,
            max_wall_time_ms,
            DEFAULT_MAX_STACK_DEPTH,
            DEFAULT_MAX_HEAP_BYTES,
            DEFAULT_MAX_RECURSION_DEPTH,
        )
    }

    pub fn bounded(
        max_credits: CreditAmount,
        max_events: u64,
        max_wall_time_ms: u64,
        max_stack_depth: u32,
        max_heap_bytes: u64,
        max_recursion_depth: u32,
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
        if max_stack_depth == 0 {
            return Err(FoundationError::OutOfRange {
                field: "max_stack_depth",
            });
        }
        if max_heap_bytes == 0 {
            return Err(FoundationError::OutOfRange {
                field: "max_heap_bytes",
            });
        }
        if max_recursion_depth == 0 {
            return Err(FoundationError::OutOfRange {
                field: "max_recursion_depth",
            });
        }
        Ok(Self {
            max_credits,
            max_events,
            max_wall_time_ms,
            max_stack_depth,
            max_heap_bytes,
            max_recursion_depth,
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

    pub fn max_stack_depth(&self) -> u32 {
        self.max_stack_depth
    }

    pub fn max_heap_bytes(&self) -> u64 {
        self.max_heap_bytes
    }

    pub fn max_recursion_depth(&self) -> u32 {
        self.max_recursion_depth
    }
}

#[derive(Serialize, Deserialize)]
struct ResourceBudgetParts {
    max_credits: CreditAmount,
    max_events: u64,
    max_wall_time_ms: u64,
    #[serde(default = "default_max_stack_depth")]
    max_stack_depth: u32,
    #[serde(default = "default_max_heap_bytes")]
    max_heap_bytes: u64,
    #[serde(default = "default_max_recursion_depth")]
    max_recursion_depth: u32,
}

impl TryFrom<ResourceBudgetParts> for ResourceBudget {
    type Error = FoundationError;

    fn try_from(value: ResourceBudgetParts) -> Result<Self, Self::Error> {
        Self::bounded(
            value.max_credits,
            value.max_events,
            value.max_wall_time_ms,
            value.max_stack_depth,
            value.max_heap_bytes,
            value.max_recursion_depth,
        )
    }
}

impl From<ResourceBudget> for ResourceBudgetParts {
    fn from(value: ResourceBudget) -> Self {
        Self {
            max_credits: value.max_credits,
            max_events: value.max_events,
            max_wall_time_ms: value.max_wall_time_ms,
            max_stack_depth: value.max_stack_depth,
            max_heap_bytes: value.max_heap_bytes,
            max_recursion_depth: value.max_recursion_depth,
        }
    }
}

fn default_max_stack_depth() -> u32 {
    DEFAULT_MAX_STACK_DEPTH
}

fn default_max_heap_bytes() -> u64 {
    DEFAULT_MAX_HEAP_BYTES
}

fn default_max_recursion_depth() -> u32 {
    DEFAULT_MAX_RECURSION_DEPTH
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

    fn reserve_heap_bytes(
        &mut self,
        bytes: u64,
        policy: &BoundSandboxPolicy,
    ) -> Result<(), Self::Error>;

    fn enter_stack_frame(
        &mut self,
        depth: u32,
        policy: &BoundSandboxPolicy,
    ) -> Result<(), Self::Error>;

    fn enter_recursion(
        &mut self,
        depth: u32,
        policy: &BoundSandboxPolicy,
    ) -> Result<(), Self::Error>;

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
        assert_eq!(bound.budget().max_stack_depth(), DEFAULT_MAX_STACK_DEPTH);
        assert_eq!(bound.budget().max_heap_bytes(), DEFAULT_MAX_HEAP_BYTES);
        assert_eq!(
            bound.budget().max_recursion_depth(),
            DEFAULT_MAX_RECURSION_DEPTH
        );
    }

    #[test]
    fn resource_budget_requires_runtime_execution_caps() {
        assert_eq!(
            ResourceBudget::bounded(
                CreditAmount::from_sats(10),
                1,
                1_000,
                0,
                DEFAULT_MAX_HEAP_BYTES,
                DEFAULT_MAX_RECURSION_DEPTH
            ),
            Err(FoundationError::OutOfRange {
                field: "max_stack_depth"
            })
        );
        let budget =
            ResourceBudget::bounded(CreditAmount::from_sats(10), 1, 1_000, 8, 4096, 4).unwrap();

        assert_eq!(budget.max_stack_depth(), 8);
        assert_eq!(budget.max_heap_bytes(), 4096);
        assert_eq!(budget.max_recursion_depth(), 4);
    }

    #[test]
    fn resource_budget_serde_uses_constructor_validation() {
        let invalid = serde_json::json!({
            "max_credits": 10,
            "max_events": 1,
            "max_wall_time_ms": 1_000,
            "max_stack_depth": 0,
            "max_heap_bytes": DEFAULT_MAX_HEAP_BYTES,
            "max_recursion_depth": DEFAULT_MAX_RECURSION_DEPTH
        });
        assert!(serde_json::from_value::<ResourceBudget>(invalid).is_err());

        let legacy_shape = serde_json::json!({
            "max_credits": 10,
            "max_events": 1,
            "max_wall_time_ms": 1_000
        });
        let budget = serde_json::from_value::<ResourceBudget>(legacy_shape).unwrap();

        assert_eq!(budget.max_stack_depth(), DEFAULT_MAX_STACK_DEPTH);
        assert_eq!(budget.max_heap_bytes(), DEFAULT_MAX_HEAP_BYTES);
        assert_eq!(budget.max_recursion_depth(), DEFAULT_MAX_RECURSION_DEPTH);
    }
}
