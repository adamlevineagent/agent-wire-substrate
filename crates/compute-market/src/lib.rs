use agent_wire_contracts::ContractWrap;
use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, EventEnvelope, HandlePath, SettlementIntent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJobEnvelope {
    pub job_ref: CrossGraphRef,
    pub requester: String,
    pub requester_handle: HandlePath,
    pub invocation: ModelInvocation,
    pub budget: CreditAmount,
    pub settlement: SettlementIntent,
    pub delivery: DeliveryPolicy,
    pub admission: QueueAdmission,
    pub dispatch: DispatchPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelInvocation {
    pub model_id: String,
    pub adapter: ExecutionAdapterId,
    pub prompt_ref: CrossGraphRef,
    pub input_ref: Option<CrossGraphRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionAdapterId(pub String);

pub type ComputeJobContract = ContractWrap<ComputeJobEnvelope>;

pub trait ExecutionAdapter {
    type Error;
    type Output;

    fn invoke(&self, job: &ComputeJobEnvelope) -> Result<Self::Output, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryPolicy {
    pub max_attempts: u16,
    pub timeout_ms: u64,
    pub require_chronicle_receipt: bool,
}

pub trait EventSink<T> {
    type Error;

    fn emit(&self, event: EventEnvelope<T>) -> Result<(), Self::Error>;
}

pub trait ChronicleSink<T> {
    type Error;

    fn record(&self, event: EventEnvelope<T>) -> Result<ChronicleReceipt, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChronicleReceipt {
    pub event_ref: CrossGraphRef,
    pub recorded_by: HandlePath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueAdmission {
    pub max_depth: u32,
    pub max_concurrent_jobs: u16,
    pub reject_when_over_budget: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchPolicy {
    pub prefer_low_latency: bool,
    pub require_reputation: bool,
    pub max_price: CreditAmount,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_contract_wrap_stays_at_contract_boundary() {
        let invocation = ModelInvocation {
            model_id: "local-test-model".to_owned(),
            adapter: ExecutionAdapterId("mock-adapter".to_owned()),
            prompt_ref: "playful/122/compute/1".parse().unwrap(),
            input_ref: None,
        };

        let job = ComputeJobEnvelope {
            job_ref: "playful/122/compute/2".parse().unwrap(),
            requester: "codex-elaine".to_owned(),
            requester_handle: HandlePath::new(["codex-elaine"]).unwrap(),
            invocation,
            budget: CreditAmount::from_sats(1_000),
            settlement: SettlementIntent {
                max_price: CreditAmount::from_sats(800),
                escrow_required: true,
            },
            delivery: DeliveryPolicy {
                max_attempts: 3,
                timeout_ms: 30_000,
                require_chronicle_receipt: true,
            },
            admission: QueueAdmission {
                max_depth: 64,
                max_concurrent_jobs: 4,
                reject_when_over_budget: true,
            },
            dispatch: DispatchPolicy {
                prefer_low_latency: true,
                require_reputation: true,
                max_price: CreditAmount::from_sats(750),
            },
        };

        let wrapped = ComputeJobContract::wrap(job.clone());

        assert_eq!(wrapped.payload, job);
    }
}
