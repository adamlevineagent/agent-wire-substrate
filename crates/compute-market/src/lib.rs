use agent_wire_foundation::{CreditAmount, EventEnvelope, HandlePath};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJobEnvelope {
    pub job_id: HandlePath,
    pub requester: String,
    pub budget: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInvocation {
    pub model_id: String,
    pub prompt_ref: HandlePath,
}

pub trait ExecutionAdapter {
    type Output;

    fn invoke(&self, invocation: ModelInvocation) -> Self::Output;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryPolicy {
    pub max_attempts: u16,
}

pub trait EventSink {
    fn emit(&self, event: EventEnvelope);
}

pub trait ChronicleSink {
    fn record(&self, event: EventEnvelope);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueAdmission {
    pub max_depth: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchPolicy {
    pub prefer_low_latency: bool,
}
