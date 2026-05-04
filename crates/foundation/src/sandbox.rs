use serde::{Deserialize, Serialize};

use crate::economics::CreditAmount;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    ReadContribution,
    WriteContribution,
    OpenTunnel,
    ExecuteModel,
    EmitEvent,
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    pub capability: Capability,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudget {
    pub max_credits: CreditAmount,
    pub max_events: u64,
    pub max_wall_time_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    pub grants: Vec<CapabilityGrant>,
    pub budget: ResourceBudget,
}
