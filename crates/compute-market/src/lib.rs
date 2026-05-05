use agent_wire_contracts::ContractWrap;
use agent_wire_foundation::{
    CallbackUrl, CreditAmount, CrossGraphRef, EventEnvelope, FillKey, HandlePath, IdempotencyKey,
    NamespaceId, PriceCurve, QuoteReceipt, SettlementIntent,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    pub quote_receipt: QuoteReceipt,
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
    pub idempotency_key: IdempotencyKey,
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
    pub fill_key: FillKey,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeJobStatus {
    Dispatched,
    Executing,
    Completed,
    Failed,
    Settled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeJobState {
    pub reservation_id: ComputeReservationId,
    pub job_ref: CrossGraphRef,
    pub offer_id: ComputeOfferId,
    pub status: ComputeJobStatus,
    pub dispatched_at_ms: u64,
    pub updated_at_ms: u64,
    pub failure: Option<ComputeFailure>,
    pub output: Option<NeutralComputeOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeutralComputeEnvelope {
    pub job: ComputeJobEnvelope,
    pub messages: Vec<ChatMessage>,
    pub request_id: IdempotencyKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeutralComputeOutput {
    pub result_ref: CrossGraphRef,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub latency_ms: u64,
    pub finish_reason: String,
}

pub trait ComputeExecutionAdapter {
    type Error;

    fn adapter_id(&self) -> &ExecutionAdapterId;
    fn execute(
        &self,
        envelope: &NeutralComputeEnvelope,
    ) -> Result<NeutralComputeOutput, Self::Error>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeMarketConfig {
    pub graph_handle: NamespaceId,
    pub graph_day: u32,
    pub graph_slug: Option<String>,
    pub quote_ttl_ms: u64,
    pub purchase_dispatch_window_ms: u64,
}

impl ComputeMarketConfig {
    pub fn new(graph_handle: NamespaceId, graph_day: u32, graph_slug: Option<String>) -> Self {
        Self {
            graph_handle,
            graph_day,
            graph_slug,
            quote_ttl_ms: 30_000,
            purchase_dispatch_window_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteLedgerEntry {
    pub quote: ComputeQuote,
    pub request: ComputeQuoteRequest,
    pub purchased_by: Option<IdempotencyKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeMarketStateMachine {
    pub config: ComputeMarketConfig,
    pub now_ms: u64,
    pub offers: HashMap<ComputeOfferId, ComputeOffer>,
    pub quotes: HashMap<ComputeQuoteId, QuoteLedgerEntry>,
    pub reservations: HashMap<ComputeReservationId, ComputeReservation>,
    pub jobs: HashMap<CrossGraphRef, ComputeJobState>,
    pub purchase_by_key: HashMap<IdempotencyKey, ComputeReservationId>,
    pub fill_by_key: HashMap<FillKey, MarketDispatchOutcome>,
    pub queue_depth_by_offer: HashMap<ComputeOfferId, u32>,
    pub queue_mirrors: HashMap<QueueMirrorSnapshotId, QueueMirrorSnapshot>,
    next_ref_sequence: u32,
    next_quote_sequence: u64,
    next_reservation_sequence: u64,
    next_dispatch_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComputeMarketError {
    DuplicateOffer(ComputeOfferId),
    MissingOffer(ComputeOfferId),
    NoOfferForModel(String),
    QuoteNotFound(ComputeQuoteId),
    QuoteExpired(ComputeQuoteId),
    QuoteAlreadyPurchased(ComputeQuoteId),
    BudgetExceeded {
        quoted: CreditAmount,
        max_budget: CreditAmount,
    },
    ReservationNotFound(ComputeReservationId),
    ReservationOfferMismatch {
        expected: ComputeOfferId,
        actual: ComputeOfferId,
    },
    QueueDepthExceeded {
        offer_id: ComputeOfferId,
        current: u32,
        max: u32,
    },
    JobNotFound(CrossGraphRef),
    InvalidJobTransition {
        from: ComputeJobStatus,
        to: ComputeJobStatus,
    },
    InvalidReference,
}

impl ComputeMarketStateMachine {
    pub fn new(config: ComputeMarketConfig) -> Self {
        Self {
            config,
            now_ms: 0,
            offers: HashMap::new(),
            quotes: HashMap::new(),
            reservations: HashMap::new(),
            jobs: HashMap::new(),
            purchase_by_key: HashMap::new(),
            fill_by_key: HashMap::new(),
            queue_depth_by_offer: HashMap::new(),
            queue_mirrors: HashMap::new(),
            next_ref_sequence: 1,
            next_quote_sequence: 1,
            next_reservation_sequence: 1,
            next_dispatch_sequence: 1,
        }
    }

    pub fn set_now_ms(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
    }

    pub fn advance_ms(&mut self, delta_ms: u64) {
        self.now_ms = self.now_ms.saturating_add(delta_ms);
    }

    fn next_ref(&mut self) -> CrossGraphRef {
        let sequence = self.next_ref_sequence;
        self.next_ref_sequence = self.next_ref_sequence.saturating_add(1);
        CrossGraphRef {
            namespace: self.config.graph_handle.clone(),
            day: self.config.graph_day,
            slug: self.config.graph_slug.clone(),
            sequence,
        }
    }

    fn next_quote_id(&mut self) -> ComputeQuoteId {
        let sequence = self.next_quote_sequence;
        self.next_quote_sequence = self.next_quote_sequence.saturating_add(1);
        ComputeQuoteId(format!("quote-{sequence}"))
    }

    fn next_reservation_id(&mut self) -> ComputeReservationId {
        let sequence = self.next_reservation_sequence;
        self.next_reservation_sequence = self.next_reservation_sequence.saturating_add(1);
        ComputeReservationId(format!("reservation-{sequence}"))
    }

    fn next_dispatch_id(&mut self) -> ComputeDispatchId {
        let sequence = self.next_dispatch_sequence;
        self.next_dispatch_sequence = self.next_dispatch_sequence.saturating_add(1);
        ComputeDispatchId(format!("dispatch-{sequence}"))
    }

    fn quote_price(
        offer: &ComputeOffer,
        request: &ComputeQuoteRequest,
        queue_depth: u32,
    ) -> PriceBreakdown {
        let max_tokens = request.max_tokens.unwrap_or(request.input_tokens_est);
        let input = scale_amount(offer.price.per_unit, request.input_tokens_est);
        let output = scale_amount(offer.price.base, max_tokens);
        let discount_bps = offer
            .queue_discount_curve
            .iter()
            .filter(|point| queue_depth >= point.queue_depth)
            .map(|point| point.discount_bps)
            .max()
            .unwrap_or(0);
        let subtotal = checked_sum([input, output, offer.reservation_fee]);
        let discount = scale_bps(subtotal, discount_bps);
        let total = subtotal
            .checked_sub(discount)
            .unwrap_or(CreditAmount::zero());
        PriceBreakdown {
            input,
            output,
            reservation_fee: offer.reservation_fee,
            queue_discount_bps: discount_bps,
            total,
        }
    }

    fn sorted_matching_offers(&self, request: &ComputeQuoteRequest) -> Vec<&ComputeOffer> {
        let mut offers = self
            .offers
            .values()
            .filter(|offer| offer.model_id == request.model_id)
            .filter(|offer| {
                self.queue_depth_by_offer
                    .get(&offer.offer_id)
                    .copied()
                    .unwrap_or(0)
                    < offer.max_queue_depth
            })
            .collect::<Vec<_>>();
        offers.sort_by(|a, b| {
            let a_depth = self
                .queue_depth_by_offer
                .get(&a.offer_id)
                .copied()
                .unwrap_or(0);
            let b_depth = self
                .queue_depth_by_offer
                .get(&b.offer_id)
                .copied()
                .unwrap_or(0);
            match request.latency_preference {
                LatencyPreference::LowestLatency => a_depth
                    .cmp(&b_depth)
                    .then_with(|| a.offer_id.0.cmp(&b.offer_id.0)),
                LatencyPreference::BestPrice | LatencyPreference::Balanced => {
                    let a_price = Self::quote_price(a, request, a_depth).total.as_sats();
                    let b_price = Self::quote_price(b, request, b_depth).total.as_sats();
                    a_price
                        .cmp(&b_price)
                        .then_with(|| a_depth.cmp(&b_depth))
                        .then_with(|| a.offer_id.0.cmp(&b.offer_id.0))
                }
            }
        });
        offers
    }

    pub fn transition_job_status(
        &mut self,
        job_ref: &CrossGraphRef,
        next: ComputeJobStatus,
    ) -> Result<ComputeJobStatus, ComputeMarketError> {
        let (prior, offer_id, release_slot) = {
            let job = self
                .jobs
                .get_mut(job_ref)
                .ok_or_else(|| ComputeMarketError::JobNotFound(job_ref.clone()))?;
            let prior = job.status;
            if !valid_job_transition(prior, next) {
                return Err(ComputeMarketError::InvalidJobTransition {
                    from: prior,
                    to: next,
                });
            }
            job.status = next;
            job.updated_at_ms = self.now_ms;
            (
                prior,
                job.offer_id.clone(),
                releases_queue_slot(prior, next),
            )
        };
        if release_slot {
            let depth = self.queue_depth_by_offer.entry(offer_id).or_insert(0);
            *depth = depth.saturating_sub(1);
        }
        Ok(prior)
    }

    pub fn complete_job(
        &mut self,
        job_ref: &CrossGraphRef,
        output: NeutralComputeOutput,
    ) -> Result<ComputeJobStatus, ComputeMarketError> {
        let prior = self.transition_job_status(job_ref, ComputeJobStatus::Completed)?;
        if let Some(job) = self.jobs.get_mut(job_ref) {
            job.output = Some(output);
        }
        Ok(prior)
    }

    pub fn fail_job(
        &mut self,
        job_ref: &CrossGraphRef,
        failure: ComputeFailure,
    ) -> Result<ComputeJobStatus, ComputeMarketError> {
        let prior = self.transition_job_status(job_ref, ComputeJobStatus::Failed)?;
        if let Some(job) = self.jobs.get_mut(job_ref) {
            job.failure = Some(failure);
        }
        Ok(prior)
    }
}

fn valid_job_transition(from: ComputeJobStatus, to: ComputeJobStatus) -> bool {
    matches!(
        (from, to),
        (ComputeJobStatus::Dispatched, ComputeJobStatus::Executing)
            | (ComputeJobStatus::Dispatched, ComputeJobStatus::Failed)
            | (ComputeJobStatus::Executing, ComputeJobStatus::Completed)
            | (ComputeJobStatus::Executing, ComputeJobStatus::Failed)
            | (ComputeJobStatus::Completed, ComputeJobStatus::Settled)
    )
}

fn releases_queue_slot(from: ComputeJobStatus, to: ComputeJobStatus) -> bool {
    matches!(
        (from, to),
        (ComputeJobStatus::Dispatched, ComputeJobStatus::Failed)
            | (ComputeJobStatus::Executing, ComputeJobStatus::Completed)
            | (ComputeJobStatus::Executing, ComputeJobStatus::Failed)
    )
}

fn checked_sum(values: impl IntoIterator<Item = CreditAmount>) -> CreditAmount {
    values.into_iter().fold(CreditAmount::zero(), |acc, value| {
        acc.checked_add(value)
            .unwrap_or(CreditAmount::from_sats(u128::MAX))
    })
}

fn scale_amount(amount: CreditAmount, units: u32) -> CreditAmount {
    CreditAmount::from_sats(amount.as_sats().saturating_mul(units as u128))
}

fn scale_bps(amount: CreditAmount, bps: u16) -> CreditAmount {
    CreditAmount::from_sats(amount.as_sats().saturating_mul(bps as u128) / 10_000)
}

impl ComputeMarket for ComputeMarketStateMachine {
    type Error = ComputeMarketError;

    fn publish_offer(&mut self, offer: ComputeOffer) -> Result<ComputeOfferId, Self::Error> {
        if self.offers.contains_key(&offer.offer_id) {
            return Err(ComputeMarketError::DuplicateOffer(offer.offer_id));
        }
        let offer_id = offer.offer_id.clone();
        self.queue_depth_by_offer
            .entry(offer_id.clone())
            .or_insert(0);
        self.offers.insert(offer_id.clone(), offer);
        Ok(offer_id)
    }

    fn withdraw_offer(&mut self, offer_id: ComputeOfferId) -> Result<(), Self::Error> {
        self.offers
            .remove(&offer_id)
            .ok_or_else(|| ComputeMarketError::MissingOffer(offer_id.clone()))?;
        self.queue_depth_by_offer.remove(&offer_id);
        Ok(())
    }

    fn plan_quote(&mut self, request: ComputeQuoteRequest) -> Result<ComputeQuote, Self::Error> {
        let offer = self
            .sorted_matching_offers(&request)
            .into_iter()
            .next()
            .cloned()
            .ok_or_else(|| ComputeMarketError::NoOfferForModel(request.model_id.clone()))?;
        let queue_depth = self
            .queue_depth_by_offer
            .get(&offer.offer_id)
            .copied()
            .unwrap_or(0);
        let price_breakdown = Self::quote_price(&offer, &request, queue_depth);
        if price_breakdown.total > request.max_budget {
            return Err(ComputeMarketError::BudgetExceeded {
                quoted: price_breakdown.total,
                max_budget: request.max_budget,
            });
        }

        let quote_id = self.next_quote_id();
        let quote_ref = self.next_ref();
        let quote_receipt = QuoteReceipt::new(
            quote_ref.clone(),
            IdempotencyKey::new(quote_id.0.clone())
                .map_err(|_| ComputeMarketError::InvalidReference)?,
            price_breakdown.total,
            self.now_ms.saturating_add(self.config.quote_ttl_ms),
        )
        .map_err(|_| ComputeMarketError::InvalidReference)?;
        let quote = ComputeQuote {
            quote_id: quote_id.clone(),
            offer_id: offer.offer_id.clone(),
            provider: offer.provider.clone(),
            provider_node_id: offer.provider_node_id.clone(),
            model_id: offer.model_id.clone(),
            price_breakdown,
            quote_receipt,
            expires_at_ms: self.now_ms.saturating_add(self.config.quote_ttl_ms),
            quote_ref: Some(quote_ref),
        };
        self.quotes.insert(
            quote_id,
            QuoteLedgerEntry {
                quote: quote.clone(),
                request,
                purchased_by: None,
            },
        );
        Ok(quote)
    }

    fn purchase(
        &mut self,
        request: ComputePurchaseRequest,
    ) -> Result<ComputeReservation, Self::Error> {
        if let Some(reservation_id) = self.purchase_by_key.get(&request.idempotency_key) {
            return self
                .reservations
                .get(reservation_id)
                .cloned()
                .ok_or_else(|| ComputeMarketError::ReservationNotFound(reservation_id.clone()));
        }

        let (quote, was_purchased) = {
            let entry = self
                .quotes
                .get(&request.quote_id)
                .ok_or_else(|| ComputeMarketError::QuoteNotFound(request.quote_id.clone()))?;
            (entry.quote.clone(), entry.purchased_by.clone())
        };
        if quote.expires_at_ms <= self.now_ms {
            return Err(ComputeMarketError::QuoteExpired(request.quote_id));
        }
        if was_purchased.is_some() {
            return Err(ComputeMarketError::QuoteAlreadyPurchased(request.quote_id));
        }
        if quote.price_breakdown.total > request.settlement.max_price {
            return Err(ComputeMarketError::BudgetExceeded {
                quoted: quote.price_breakdown.total,
                max_budget: request.settlement.max_price,
            });
        }

        let charged = quote.price_breakdown.total;
        let reservation_id = self.next_reservation_id();
        let reservation = ComputeReservation {
            reservation_id: reservation_id.clone(),
            job_ref: self.next_ref(),
            quote,
            trigger: request.trigger,
            charged: Some(charged),
        };
        if let Some(entry) = self.quotes.get_mut(&request.quote_id) {
            entry.purchased_by = Some(request.idempotency_key.clone());
        }
        self.purchase_by_key
            .insert(request.idempotency_key, reservation_id.clone());
        self.reservations
            .insert(reservation_id, reservation.clone());
        Ok(reservation)
    }

    fn fill(&mut self, request: ComputeFillRequest) -> Result<MarketDispatchOutcome, Self::Error> {
        if let Some(outcome) = self.fill_by_key.get(&request.fill_key) {
            return Ok(outcome.clone());
        }
        let reservation = self
            .reservations
            .get(&request.reservation_id)
            .ok_or_else(|| ComputeMarketError::ReservationNotFound(request.reservation_id.clone()))?
            .clone();
        if reservation.quote.offer_id != request.offer_id {
            return Err(ComputeMarketError::ReservationOfferMismatch {
                expected: reservation.quote.offer_id,
                actual: request.offer_id,
            });
        }
        let offer = self
            .offers
            .get(&request.offer_id)
            .ok_or_else(|| ComputeMarketError::MissingOffer(request.offer_id.clone()))?
            .clone();
        let current = self
            .queue_depth_by_offer
            .get(&request.offer_id)
            .copied()
            .unwrap_or(0);
        if current >= offer.max_queue_depth {
            return Err(ComputeMarketError::QueueDepthExceeded {
                offer_id: request.offer_id,
                current,
                max: offer.max_queue_depth,
            });
        }

        self.queue_depth_by_offer
            .insert(request.offer_id.clone(), current.saturating_add(1));
        let outcome = MarketDispatchOutcome {
            dispatch_id: self.next_dispatch_id(),
            job_ref: request.job_ref.clone(),
            status: DispatchStatus::Accepted,
            provider_receipt_ref: Some(self.next_ref()),
        };
        let job_state = ComputeJobState {
            reservation_id: request.reservation_id,
            job_ref: request.job_ref,
            offer_id: request.offer_id,
            status: ComputeJobStatus::Dispatched,
            dispatched_at_ms: self.now_ms,
            updated_at_ms: self.now_ms,
            failure: None,
            output: None,
        };
        self.jobs.insert(job_state.job_ref.clone(), job_state);
        self.fill_by_key.insert(request.fill_key, outcome.clone());
        Ok(outcome)
    }

    fn record_queue_mirror(
        &mut self,
        snapshot: QueueMirrorSnapshot,
    ) -> Result<QueueMirrorSnapshotId, Self::Error> {
        for mirror_offer in &snapshot.offers {
            self.queue_depth_by_offer.insert(
                mirror_offer.offer_id.clone(),
                mirror_offer.current_queue_depth,
            );
        }
        let snapshot_id = snapshot.snapshot_id.clone();
        self.queue_mirrors.insert(snapshot_id.clone(), snapshot);
        Ok(snapshot_id)
    }

    fn market_surface(&mut self) -> Result<MarketSurfaceSnapshot, Self::Error> {
        let mut models = HashMap::<String, Vec<MarketSurfaceOffer>>::new();
        for offer in self.offers.values() {
            let queue_depth = self
                .queue_depth_by_offer
                .get(&offer.offer_id)
                .copied()
                .unwrap_or(0);
            models
                .entry(offer.model_id.clone())
                .or_default()
                .push(MarketSurfaceOffer {
                    offer_id: offer.offer_id.clone(),
                    provider: offer.provider.clone(),
                    provider_type: offer.provider_type,
                    queue_depth,
                    max_queue_depth: offer.max_queue_depth,
                    price_breakdown: PriceBreakdown {
                        input: CreditAmount::zero(),
                        output: CreditAmount::zero(),
                        reservation_fee: offer.reservation_fee,
                        queue_discount_bps: 0,
                        total: offer.reservation_fee,
                    },
                });
        }
        let mut models = models
            .into_iter()
            .map(|(model_id, mut offers)| {
                offers.sort_by(|a, b| a.offer_id.0.cmp(&b.offer_id.0));
                MarketSurfaceModel {
                    model_id,
                    visible_offers: offers.len() as u32,
                    offers,
                }
            })
            .collect::<Vec<_>>();
        models.sort_by(|a, b| a.model_id.cmp(&b.model_id));
        Ok(MarketSurfaceSnapshot {
            snapshot_ref: self.next_ref(),
            generated_at_ms: self.now_ms,
            models,
        })
    }
}

pub trait ComputeMarket {
    type Error;

    fn publish_offer(&mut self, offer: ComputeOffer) -> Result<ComputeOfferId, Self::Error>;
    fn withdraw_offer(&mut self, offer_id: ComputeOfferId) -> Result<(), Self::Error>;
    fn plan_quote(&mut self, request: ComputeQuoteRequest) -> Result<ComputeQuote, Self::Error>;
    fn purchase(
        &mut self,
        request: ComputePurchaseRequest,
    ) -> Result<ComputeReservation, Self::Error>;
    fn fill(&mut self, request: ComputeFillRequest) -> Result<MarketDispatchOutcome, Self::Error>;
    fn record_queue_mirror(
        &mut self,
        snapshot: QueueMirrorSnapshot,
    ) -> Result<QueueMirrorSnapshotId, Self::Error>;
    fn market_surface(&mut self) -> Result<MarketSurfaceSnapshot, Self::Error>;
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

    fn market() -> ComputeMarketStateMachine {
        let mut market = ComputeMarketStateMachine::new(ComputeMarketConfig::new(
            NamespaceId::new("playful").unwrap(),
            124,
            Some("compute".to_owned()),
        ));
        market.set_now_ms(1_000);
        market
    }

    fn offer(
        id: &str,
        model_id: &str,
        input_rate: u128,
        output_rate: u128,
        max_depth: u32,
    ) -> ComputeOffer {
        ComputeOffer {
            offer_id: ComputeOfferId(id.to_owned()),
            provider: provider_handle(),
            provider_node_id: ProviderNodeId(format!("{id}-node")),
            provider_type: ProviderType::Local,
            model_id: model_id.to_owned(),
            adapter: ExecutionAdapterId("mock-adapter".to_owned()),
            price: PriceCurve {
                base: sats(output_rate),
                per_unit: sats(input_rate),
            },
            reservation_fee: sats(3),
            queue_discount_curve: vec![QueueDiscount {
                queue_depth: 1,
                discount_bps: 100,
            }],
            max_queue_depth: max_depth,
            settlement: SettlementIntent {
                max_price: sats(10_000),
                escrow_required: true,
            },
        }
    }

    fn quote_request(preference: LatencyPreference) -> ComputeQuoteRequest {
        ComputeQuoteRequest {
            requester: HandlePath::new(["agent", "playful", "requester"]).unwrap(),
            requester_node_id: Some(ProviderNodeId("requester-node".to_owned())),
            model_id: "local-test-model".to_owned(),
            input_tokens_est: 10,
            max_tokens: Some(5),
            latency_preference: preference,
            max_budget: sats(10_000),
        }
    }

    fn purchase_request(quote: &ComputeQuote, key: &str) -> ComputePurchaseRequest {
        ComputePurchaseRequest {
            quote_id: quote.quote_id.clone(),
            idempotency_key: IdempotencyKey::new(key).unwrap(),
            requester: HandlePath::new(["agent", "playful", "requester"]).unwrap(),
            trigger: ComputePurchaseTrigger::Immediate,
            delivery: DeliveryPolicy {
                max_attempts: 3,
                timeout_ms: 30_000,
                require_chronicle_receipt: true,
            },
            settlement: SettlementIntent {
                max_price: sats(10_000),
                escrow_required: true,
            },
        }
    }

    fn fill_request(reservation: &ComputeReservation, key: &str) -> ComputeFillRequest {
        ComputeFillRequest {
            reservation_id: reservation.reservation_id.clone(),
            fill_key: FillKey::new(key).unwrap(),
            job_ref: reservation.job_ref.clone(),
            offer_id: reservation.quote.offer_id.clone(),
            requester: HandlePath::new(["agent", "playful", "requester"]).unwrap(),
            input_token_count: 10,
            max_tokens: Some(5),
            temperature_milli: Some(200),
            relay_count: 0,
            privacy_tier: ComputePrivacyTier::Direct,
            requester_callback_url: None,
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content_ref: None,
                content_inline: Some("hello".to_owned()),
            }],
        }
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
            quote_receipt: QuoteReceipt::new(
                playful_ref(1),
                IdempotencyKey::new("quote-1").unwrap(),
                sats(45),
                1_800_000,
            )
            .unwrap(),
            expires_at_ms: 1_800_000,
            quote_ref: Some(playful_ref(1)),
        };

        let purchase = ComputePurchaseRequest {
            quote_id: quote.quote_id.clone(),
            idempotency_key: IdempotencyKey::new("purchase-quote-1").unwrap(),
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
            fill_key: FillKey::new("reservation-1-fill").unwrap(),
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
            provider_node_id: quote.provider_node_id.clone(),
            fill,
            credentials: DispatchCredentials {
                credential_ref: playful_ref(3),
                expires_at_ms: 1_900_000,
            },
            deadline_ms: 1_860_000,
        };

        assert_eq!(purchase.quote_id, ComputeQuoteId("quote-1".to_owned()));
        assert_eq!(purchase.idempotency_key.as_str(), "purchase-quote-1");
        assert_eq!(quote.quote_receipt.idempotency_key().as_str(), "quote-1");
        assert_eq!(dispatch.fill.offer_id, ComputeOfferId("offer-1".to_owned()));
        assert_eq!(dispatch.fill.fill_key.as_str(), "reservation-1-fill");
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

    #[test]
    fn state_machine_plans_deterministic_best_price_quote() {
        let mut market = market();
        market
            .publish_offer(offer("expensive", "local-test-model", 5, 5, 4))
            .unwrap();
        market
            .publish_offer(offer("cheap", "local-test-model", 1, 2, 4))
            .unwrap();

        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();

        assert_eq!(quote.offer_id, ComputeOfferId("cheap".to_owned()));
        assert_eq!(quote.expires_at_ms, 31_000);
        assert_eq!(quote.price_breakdown.input.as_sats(), 10);
        assert_eq!(quote.price_breakdown.output.as_sats(), 10);
        assert_eq!(quote.price_breakdown.reservation_fee.as_sats(), 3);
        assert_eq!(quote.price_breakdown.total.as_sats(), 23);
        assert_eq!(quote.quote_receipt.idempotency_key().as_str(), "quote-1");
    }

    #[test]
    fn state_machine_purchase_is_idempotent_and_quote_is_single_redeem() {
        let mut market = market();
        market
            .publish_offer(offer("cheap", "local-test-model", 1, 2, 4))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();

        let first = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();
        let replay = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();
        let second_key = market.purchase(purchase_request(&quote, "purchase-2"));

        assert_eq!(first, replay);
        assert_eq!(first.charged.unwrap().as_sats(), 23);
        assert_eq!(
            second_key,
            Err(ComputeMarketError::QuoteAlreadyPurchased(quote.quote_id))
        );
    }

    #[test]
    fn state_machine_rejects_expired_quote_before_purchase() {
        let mut market = market();
        market
            .publish_offer(offer("cheap", "local-test-model", 1, 2, 4))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();
        market.advance_ms(30_000);

        assert_eq!(
            market.purchase(purchase_request(&quote, "purchase-1")),
            Err(ComputeMarketError::QuoteExpired(quote.quote_id))
        );
    }

    #[test]
    fn state_machine_fill_uses_fill_key_idempotency_and_queue_cap() {
        let mut market = market();
        market
            .publish_offer(offer("single-slot", "local-test-model", 1, 2, 1))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();
        let reservation = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();

        let first = market.fill(fill_request(&reservation, "fill-1")).unwrap();
        let replay = market.fill(fill_request(&reservation, "fill-1")).unwrap();
        let second_fill = market.fill(fill_request(&reservation, "fill-2"));

        assert_eq!(first, replay);
        assert_eq!(first.status, DispatchStatus::Accepted);
        assert_eq!(
            second_fill,
            Err(ComputeMarketError::QueueDepthExceeded {
                offer_id: ComputeOfferId("single-slot".to_owned()),
                current: 1,
                max: 1,
            })
        );
    }

    #[test]
    fn state_machine_queue_mirror_updates_market_surface_depths() {
        let mut market = market();
        market
            .publish_offer(offer("cheap", "local-test-model", 1, 2, 4))
            .unwrap();
        market
            .record_queue_mirror(QueueMirrorSnapshot {
                snapshot_id: QueueMirrorSnapshotId("mirror-1".to_owned()),
                provider: provider_handle(),
                provider_node_id: ProviderNodeId("cheap-node".to_owned()),
                snapshot_seq: 7,
                is_serving: true,
                offers: vec![QueueMirrorOffer {
                    model_id: "local-test-model".to_owned(),
                    offer_id: ComputeOfferId("cheap".to_owned()),
                    current_queue_depth: 2,
                    max_queue_depth: 4,
                    allow_market_visibility: true,
                }],
            })
            .unwrap();

        let surface = market.market_surface().unwrap();

        assert_eq!(surface.models.len(), 1);
        assert_eq!(surface.models[0].model_id, "local-test-model");
        assert_eq!(surface.models[0].offers[0].queue_depth, 2);
        assert_eq!(surface.models[0].offers[0].max_queue_depth, 4);
    }

    #[test]
    fn state_machine_job_lifecycle_releases_queue_slot_on_completion() {
        let mut market = market();
        market
            .publish_offer(offer("single-slot", "local-test-model", 1, 2, 1))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();
        let reservation = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();
        let fill = market.fill(fill_request(&reservation, "fill-1")).unwrap();

        assert_eq!(
            market
                .queue_depth_by_offer
                .get(&ComputeOfferId("single-slot".to_owned())),
            Some(&1)
        );
        assert_eq!(
            market.jobs.get(&fill.job_ref).map(|job| job.status),
            Some(ComputeJobStatus::Dispatched)
        );

        let dispatched = market
            .transition_job_status(&fill.job_ref, ComputeJobStatus::Executing)
            .unwrap();
        let executing = market
            .complete_job(
                &fill.job_ref,
                NeutralComputeOutput {
                    result_ref: playful_ref(99),
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    latency_ms: 42,
                    finish_reason: "stop".to_owned(),
                },
            )
            .unwrap();

        assert_eq!(dispatched, ComputeJobStatus::Dispatched);
        assert_eq!(executing, ComputeJobStatus::Executing);
        assert_eq!(
            market
                .queue_depth_by_offer
                .get(&ComputeOfferId("single-slot".to_owned())),
            Some(&0)
        );
        let job = market.jobs.get(&fill.job_ref).unwrap();
        assert_eq!(job.status, ComputeJobStatus::Completed);
        assert_eq!(job.output.as_ref().unwrap().result_ref, playful_ref(99));

        assert_eq!(
            market
                .transition_job_status(&fill.job_ref, ComputeJobStatus::Settled)
                .unwrap(),
            ComputeJobStatus::Completed
        );
        assert_eq!(
            market.jobs.get(&fill.job_ref).map(|job| job.status),
            Some(ComputeJobStatus::Settled)
        );
    }

    #[test]
    fn state_machine_rejects_invalid_job_transition() {
        let mut market = market();
        market
            .publish_offer(offer("single-slot", "local-test-model", 1, 2, 1))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();
        let reservation = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();
        let fill = market.fill(fill_request(&reservation, "fill-1")).unwrap();

        assert_eq!(
            market.transition_job_status(&fill.job_ref, ComputeJobStatus::Completed),
            Err(ComputeMarketError::InvalidJobTransition {
                from: ComputeJobStatus::Dispatched,
                to: ComputeJobStatus::Completed,
            })
        );
        assert_eq!(
            market
                .queue_depth_by_offer
                .get(&ComputeOfferId("single-slot".to_owned())),
            Some(&1)
        );
    }

    #[test]
    fn state_machine_fail_job_records_failure_and_releases_queue_slot() {
        let mut market = market();
        market
            .publish_offer(offer("single-slot", "local-test-model", 1, 2, 1))
            .unwrap();
        let quote = market
            .plan_quote(quote_request(LatencyPreference::BestPrice))
            .unwrap();
        let reservation = market
            .purchase(purchase_request(&quote, "purchase-1"))
            .unwrap();
        let fill = market.fill(fill_request(&reservation, "fill-1")).unwrap();

        assert_eq!(
            market
                .fail_job(
                    &fill.job_ref,
                    ComputeFailure {
                        code: ComputeFailureCode::ProviderUnavailable,
                        retry_intent: RetryIntent::Backoff,
                        detail: Some("provider exited".to_owned()),
                        retry_after_ms: Some(1_000),
                    },
                )
                .unwrap(),
            ComputeJobStatus::Dispatched
        );

        let job = market.jobs.get(&fill.job_ref).unwrap();
        assert_eq!(job.status, ComputeJobStatus::Failed);
        assert_eq!(
            job.failure.as_ref().unwrap().code,
            ComputeFailureCode::ProviderUnavailable
        );
        assert_eq!(
            market
                .queue_depth_by_offer
                .get(&ComputeOfferId("single-slot".to_owned())),
            Some(&0)
        );
    }

    struct EchoAdapter {
        id: ExecutionAdapterId,
    }

    impl ComputeExecutionAdapter for EchoAdapter {
        type Error = ComputeFailure;

        fn adapter_id(&self) -> &ExecutionAdapterId {
            &self.id
        }

        fn execute(
            &self,
            envelope: &NeutralComputeEnvelope,
        ) -> Result<NeutralComputeOutput, Self::Error> {
            Ok(NeutralComputeOutput {
                result_ref: envelope.job.job_ref.clone(),
                prompt_tokens: envelope.messages.len() as u32,
                completion_tokens: 1,
                latency_ms: 5,
                finish_reason: "stop".to_owned(),
            })
        }
    }

    #[test]
    fn neutral_compute_envelope_executes_without_pyramid_types() {
        let job = ComputeJobEnvelope {
            job_ref: playful_ref(20),
            requester: "codex-kramer".to_owned(),
            requester_handle: HandlePath::new(["codex-kramer"]).unwrap(),
            invocation: ModelInvocation {
                model_id: "local-test-model".to_owned(),
                adapter: ExecutionAdapterId("echo".to_owned()),
                prompt_ref: playful_ref(21),
                input_ref: None,
                max_tokens: Some(16),
                temperature_milli: None,
            },
            budget: sats(100),
            settlement: SettlementIntent {
                max_price: sats(100),
                escrow_required: true,
            },
            delivery: DeliveryPolicy {
                max_attempts: 3,
                timeout_ms: 1_000,
                require_chronicle_receipt: false,
            },
            admission: QueueAdmission {
                max_depth: 4,
                max_concurrent_jobs: 1,
                reject_when_over_budget: true,
            },
            dispatch: DispatchPolicy {
                latency_preference: LatencyPreference::Balanced,
                require_reputation: false,
                max_price: sats(100),
            },
        };
        let envelope = NeutralComputeEnvelope {
            job,
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content_ref: None,
                content_inline: Some("ping".to_owned()),
            }],
            request_id: IdempotencyKey::new("neutral-1").unwrap(),
        };
        let adapter = EchoAdapter {
            id: ExecutionAdapterId("echo".to_owned()),
        };

        let output = adapter.execute(&envelope).unwrap();

        assert_eq!(adapter.adapter_id(), &ExecutionAdapterId("echo".to_owned()));
        assert_eq!(output.result_ref, playful_ref(20));
        assert_eq!(output.prompt_tokens, 1);
    }
}
