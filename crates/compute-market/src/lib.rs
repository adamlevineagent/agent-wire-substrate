use agent_wire_contracts::ContractWrap;
use agent_wire_foundation::{
    CallbackUrl, CreditAmount, CrossGraphRef, EventEnvelope, HandlePath, PriceCurve,
    SettlementIntent,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComputeOfferId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComputeQuoteId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComputeReservationId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ComputeDispatchId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueueMirrorSnapshotId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProviderNodeId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ExecutionAdapterId(pub String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    Local,
    Bridge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatencyPreference {
    BestPrice,
    Balanced,
    LowestLatency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputePrivacyTier {
    Direct,
    BootstrapRelay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryIntent {
    Never,
    Transient,
    Backoff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchRejectReason {
    QueueDepthExceeded,
    ForeignDispatcherConflict,
    ProviderUnavailable,
    ReservationExpired,
    BudgetExceeded,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueDiscount {
    pub queue_depth: u32,
    pub discount_bps: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeOffer {
    pub offer_id: ComputeOfferId,
    pub provider: HandlePath,
    pub provider_node_id: ProviderNodeId,
    pub provider_type: ProviderType,
    pub model_id: String,
    pub adapter: ExecutionAdapterId,
    pub price: PriceCurve,
    pub reservation_fee: CreditAmount,
    pub queue_discount_curve: Vec<QueueDiscount>,
    pub max_queue_depth: u32,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeQuoteRequest {
    pub requester: HandlePath,
    pub requester_node_id: Option<ProviderNodeId>,
    pub model_id: String,
    pub input_tokens_est: u32,
    pub max_tokens: Option<u32>,
    pub latency_preference: LatencyPreference,
    pub max_budget: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceBreakdown {
    pub input: CreditAmount,
    pub output: CreditAmount,
    pub reservation_fee: CreditAmount,
    pub queue_discount_bps: u16,
    pub total: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeQuote {
    pub quote_id: ComputeQuoteId,
    pub offer_id: ComputeOfferId,
    pub provider: HandlePath,
    pub provider_node_id: ProviderNodeId,
    pub model_id: String,
    pub price_breakdown: PriceBreakdown,
    pub expires_at_ms: u64,
    pub quote_ref: Option<CrossGraphRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputePurchaseTrigger {
    Immediate,
    DeferredAt { not_before_ms: u64 },
    DeferredSignal { signal_ref: CrossGraphRef },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputePurchaseRequest {
    pub quote_id: ComputeQuoteId,
    pub requester: HandlePath,
    pub trigger: ComputePurchaseTrigger,
    pub delivery: DeliveryPolicy,
    pub settlement: SettlementIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeReservation {
    pub reservation_id: ComputeReservationId,
    pub job_ref: CrossGraphRef,
    pub quote: ComputeQuote,
    pub trigger: ComputePurchaseTrigger,
    pub charged: Option<CreditAmount>,
}

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
    pub max_tokens: Option<u32>,
    pub temperature_milli: Option<u32>,
}

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
    pub latency_preference: LatencyPreference,
    pub require_reputation: bool,
    pub max_price: CreditAmount,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueMirrorSnapshot {
    pub snapshot_id: QueueMirrorSnapshotId,
    pub provider: HandlePath,
    pub provider_node_id: ProviderNodeId,
    pub snapshot_seq: u64,
    pub is_serving: bool,
    pub offers: Vec<QueueMirrorOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueMirrorOffer {
    pub model_id: String,
    pub offer_id: ComputeOfferId,
    pub current_queue_depth: u32,
    pub max_queue_depth: u32,
    pub allow_market_visibility: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content_ref: Option<CrossGraphRef>,
    pub content_inline: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeFillRequest {
    pub reservation_id: ComputeReservationId,
    pub job_ref: CrossGraphRef,
    pub offer_id: ComputeOfferId,
    pub requester: HandlePath,
    pub input_token_count: u32,
    pub max_tokens: Option<u32>,
    pub temperature_milli: Option<u32>,
    pub relay_count: u16,
    pub privacy_tier: ComputePrivacyTier,
    pub requester_callback_url: Option<CallbackUrl>,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchCredentials {
    pub credential_ref: CrossGraphRef,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDispatchRequest {
    pub dispatch_id: ComputeDispatchId,
    pub provider_node_id: ProviderNodeId,
    pub fill: ComputeFillRequest,
    pub credentials: DispatchCredentials,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchStatus {
    Accepted,
    Rejected { reason: DispatchRejectReason },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketDispatchOutcome {
    pub dispatch_id: ComputeDispatchId,
    pub job_ref: CrossGraphRef,
    pub status: DispatchStatus,
    pub provider_receipt_ref: Option<CrossGraphRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub job_ref: CrossGraphRef,
    pub delivered_to: Option<CallbackUrl>,
    pub result_ref: CrossGraphRef,
    pub charged: CreditAmount,
    pub retry_intent: RetryIntent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceSnapshot {
    pub snapshot_ref: CrossGraphRef,
    pub generated_at_ms: u64,
    pub models: Vec<MarketSurfaceModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceModel {
    pub model_id: String,
    pub visible_offers: u32,
    pub offers: Vec<MarketSurfaceOffer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceOffer {
    pub offer_id: ComputeOfferId,
    pub provider: HandlePath,
    pub provider_type: ProviderType,
    pub queue_depth: u32,
    pub max_queue_depth: u32,
    pub price_breakdown: PriceBreakdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeFailure {
    pub code: ComputeFailureCode,
    pub retry_intent: RetryIntent,
    pub detail: Option<String>,
    pub retry_after_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeFailureCode {
    QuoteExpired,
    QuoteBudgetExceeded,
    OfferUnavailable,
    ReservationConflict,
    QueueDepthExceeded,
    DispatchTimeout,
    ProviderUnavailable,
    ForeignDispatcherConflict,
    DeliveryFailed,
}

pub trait ComputeMarket {
    type Error;

    fn publish_offer(&self, offer: ComputeOffer) -> Result<ComputeOfferId, Self::Error>;
    fn withdraw_offer(&self, offer_id: ComputeOfferId) -> Result<(), Self::Error>;
    fn plan_quote(&self, request: ComputeQuoteRequest) -> Result<ComputeQuote, Self::Error>;
    fn purchase(&self, request: ComputePurchaseRequest) -> Result<ComputeReservation, Self::Error>;
    fn fill(&self, request: ComputeFillRequest) -> Result<MarketDispatchOutcome, Self::Error>;
    fn record_queue_mirror(
        &self,
        snapshot: QueueMirrorSnapshot,
    ) -> Result<QueueMirrorSnapshotId, Self::Error>;
    fn market_surface(&self) -> Result<MarketSurfaceSnapshot, Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sats(value: u128) -> CreditAmount {
        CreditAmount::from_sats(value)
    }

    fn playful_ref(sequence: u32) -> CrossGraphRef {
        format!("playful/122/compute/{sequence}").parse().unwrap()
    }

    fn provider_handle() -> HandlePath {
        HandlePath::new(["agent", "playful", "compute-provider"]).unwrap()
    }

    #[test]
    fn compute_offer_carries_pricing_queue_and_settlement() {
        let offer = ComputeOffer {
            offer_id: ComputeOfferId("offer-1".to_owned()),
            provider: provider_handle(),
            provider_node_id: ProviderNodeId("node-1".to_owned()),
            provider_type: ProviderType::Local,
            model_id: "local-test-model".to_owned(),
            adapter: ExecutionAdapterId("mock-adapter".to_owned()),
            price: PriceCurve {
                base: sats(20),
                per_unit: sats(4),
            },
            reservation_fee: sats(3),
            queue_discount_curve: vec![QueueDiscount {
                queue_depth: 8,
                discount_bps: 250,
            }],
            max_queue_depth: 64,
            settlement: SettlementIntent {
                max_price: sats(1_000),
                escrow_required: true,
            },
        };

        assert_eq!(offer.provider_type, ProviderType::Local);
        assert_eq!(offer.queue_discount_curve[0].discount_bps, 250);
        assert_eq!(offer.price.per_unit.as_sats(), 4);
        assert!(offer.settlement.escrow_required);
    }

    #[test]
    fn quote_purchase_and_fill_share_stable_ids() {
        let quote = ComputeQuote {
            quote_id: ComputeQuoteId("quote-1".to_owned()),
            offer_id: ComputeOfferId("offer-1".to_owned()),
            provider: provider_handle(),
            provider_node_id: ProviderNodeId("node-1".to_owned()),
            model_id: "local-test-model".to_owned(),
            price_breakdown: PriceBreakdown {
                input: sats(10),
                output: sats(30),
                reservation_fee: sats(5),
                queue_discount_bps: 100,
                total: sats(45),
            },
            expires_at_ms: 1_800_000,
            quote_ref: Some(playful_ref(1)),
        };

        let purchase = ComputePurchaseRequest {
            quote_id: quote.quote_id.clone(),
            requester: HandlePath::new(["agent", "playful", "kramer"]).unwrap(),
            trigger: ComputePurchaseTrigger::Immediate,
            delivery: DeliveryPolicy {
                max_attempts: 3,
                timeout_ms: 30_000,
                require_chronicle_receipt: true,
            },
            settlement: SettlementIntent {
                max_price: sats(50),
                escrow_required: true,
            },
        };

        let fill = ComputeFillRequest {
            reservation_id: ComputeReservationId("reservation-1".to_owned()),
            job_ref: playful_ref(2),
            offer_id: quote.offer_id.clone(),
            requester: purchase.requester.clone(),
            input_token_count: 250,
            max_tokens: Some(128),
            temperature_milli: Some(700),
            relay_count: 0,
            privacy_tier: ComputePrivacyTier::BootstrapRelay,
            requester_callback_url: Some(
                CallbackUrl::parse("https://example.com/callback").unwrap(),
            ),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content_ref: None,
                content_inline: Some("hello".to_owned()),
            }],
        };

        let dispatch = MarketDispatchRequest {
            dispatch_id: ComputeDispatchId("dispatch-1".to_owned()),
            provider_node_id: quote.provider_node_id,
            fill,
            credentials: DispatchCredentials {
                credential_ref: playful_ref(3),
                expires_at_ms: 1_900_000,
            },
            deadline_ms: 1_860_000,
        };

        assert_eq!(purchase.quote_id, ComputeQuoteId("quote-1".to_owned()));
        assert_eq!(dispatch.fill.offer_id, ComputeOfferId("offer-1".to_owned()));
        assert_eq!(
            dispatch.fill.privacy_tier,
            ComputePrivacyTier::BootstrapRelay
        );
    }

    #[test]
    fn queue_mirror_shape_exposes_only_market_safe_depths() {
        let snapshot = QueueMirrorSnapshot {
            snapshot_id: QueueMirrorSnapshotId("mirror-1".to_owned()),
            provider: provider_handle(),
            provider_node_id: ProviderNodeId("node-1".to_owned()),
            snapshot_seq: 42,
            is_serving: true,
            offers: vec![QueueMirrorOffer {
                model_id: "local-test-model".to_owned(),
                offer_id: ComputeOfferId("offer-1".to_owned()),
                current_queue_depth: 5,
                max_queue_depth: 64,
                allow_market_visibility: true,
            }],
        };

        assert!(snapshot.is_serving);
        assert_eq!(snapshot.offers[0].current_queue_depth, 5);
        assert!(snapshot.offers[0].allow_market_visibility);
    }

    #[test]
    fn compute_contract_wrap_stays_at_contract_boundary() {
        let invocation = ModelInvocation {
            model_id: "local-test-model".to_owned(),
            adapter: ExecutionAdapterId("mock-adapter".to_owned()),
            prompt_ref: playful_ref(1),
            input_ref: None,
            max_tokens: Some(128),
            temperature_milli: Some(700),
        };

        let job = ComputeJobEnvelope {
            job_ref: playful_ref(2),
            requester: "codex-elaine".to_owned(),
            requester_handle: HandlePath::new(["codex-elaine"]).unwrap(),
            invocation,
            budget: sats(1_000),
            settlement: SettlementIntent {
                max_price: sats(800),
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
                latency_preference: LatencyPreference::LowestLatency,
                require_reputation: true,
                max_price: sats(750),
            },
        };

        let wrapped = ComputeJobContract::wrap(job.clone());

        assert_eq!(wrapped.payload, job);
    }

    #[test]
    fn dispatch_rejection_and_failure_are_retry_typed() {
        let outcome = MarketDispatchOutcome {
            dispatch_id: ComputeDispatchId("dispatch-1".to_owned()),
            job_ref: playful_ref(2),
            status: DispatchStatus::Rejected {
                reason: DispatchRejectReason::ProviderUnavailable,
            },
            provider_receipt_ref: None,
        };
        let failure = ComputeFailure {
            code: ComputeFailureCode::ProviderUnavailable,
            retry_intent: RetryIntent::Backoff,
            detail: Some("provider returned 503".to_owned()),
            retry_after_ms: Some(1_000),
        };

        assert_eq!(
            outcome.status,
            DispatchStatus::Rejected {
                reason: DispatchRejectReason::ProviderUnavailable
            }
        );
        assert_eq!(failure.retry_intent, RetryIntent::Backoff);
    }
}
