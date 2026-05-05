//! Bilateral Wire protocol DTOs.
//!
//! These are serialized contract shapes. Runtime semantics live in
//! `agent-wire-foundation` and cross this boundary through explicit
//! conversions rather than re-exports.

use serde::{Deserialize, Serialize};

pub trait WireDto: private::Sealed {}

mod private {
    pub trait Sealed {}
}

macro_rules! impl_wire_dto {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}
            impl WireDto for $ty {}
        )+
    };
}

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

impl<T: WireDto> private::Sealed for ContractWrap<T> {}
impl<T: WireDto> WireDto for ContractWrap<T> {}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody<T = serde_json::Value> {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<i64>,
}

impl<T> private::Sealed for ErrorBody<T> {}
impl<T> WireDto for ErrorBody<T> {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsufficientBalanceDetail {
    pub need: i64,
    pub have: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoOfferDetail {
    pub model_id: String,
    pub budget: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultipleNodesDetail {
    pub owned_node_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueDepthExceededDetail {
    pub x_wire_reason: String,
    pub observed_depth: i64,
    pub max_depth: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageDetail {
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InternalErrorDetail {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetExceededDetail {
    pub estimated_total: i64,
    pub max_budget: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuoteJwtInvalidReason {
    BadSignature,
    Malformed,
    WrongIssuer,
    WrongAudience,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteJwtInvalidDetail {
    pub reason: QuoteJwtInvalidReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteJwtExpiredDetail {
    pub exp: i64,
    pub now: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteAlreadyPurchasedDetail {
    pub first_purchased_at: String,
    pub existing_job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuoteNoLongerWinningDetail {
    pub offer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DispatchDeadlineExceededDetail {
    pub dispatch_deadline_at: String,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaxTokensExceedsQuoteDetail {
    pub requested: i64,
    pub quoted: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AllOffersSaturatedDetail {
    pub model_id: String,
    pub offer_count: i64,
    pub min_current_queue_depth: i64,
    pub max_queue_depth_across_offers: i64,
    pub min_expected_drain_ms: Option<f64>,
    pub median_typical_serve_ms_p50: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LatencyPreference {
    BestPrice,
    Balanced,
    LowestLatency,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeQuoteBody {
    pub model_id: String,
    pub input_tokens_est: i64,
    pub max_tokens: i64,
    pub latency_preference: LatencyPreference,
    pub max_budget: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requester_node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeQuotePriceBreakdown {
    pub reservation_fee: i64,
    pub matched_rate_in_per_m: i64,
    pub matched_rate_out_per_m: i64,
    pub matched_multiplier_bps: i64,
    pub estimated_input_cost: i64,
    pub estimated_output_cost: i64,
    pub estimated_deposit: i64,
    pub estimated_total: i64,
    pub queue_position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputeQuoteResponse {
    pub quote_jwt: String,
    pub quote_id: String,
    pub expires_at: String,
    pub price_breakdown: ComputeQuotePriceBreakdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputePurchaseTrigger {
    Immediate,
    Time,
    Cron,
    Event,
    Threshold,
    After,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputePurchaseBody {
    pub quote_jwt: String,
    pub trigger: ComputePurchaseTrigger,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComputePurchaseResponse {
    pub job_id: String,
    pub uuid_job_id: String,
    pub request_id: String,
    pub dispatch_deadline_at: String,
    pub queue_position: i64,
    pub matched_queue_depth: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryIntent {
    Never,
    Transient,
    Backoff,
}

pub const X_WIRE_RETRY_VALUES: &[RetryIntent] = &[
    RetryIntent::Never,
    RetryIntent::Transient,
    RetryIntent::Backoff,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeFailureCode {
    WorkerHeartbeatLost,
    ModelTimeout,
    Oom,
    InvalidMessages,
    ModelError,
}

pub const COMPUTE_FAILURE_CODES: &[ComputeFailureCode] = &[
    ComputeFailureCode::WorkerHeartbeatLost,
    ComputeFailureCode::ModelTimeout,
    ComputeFailureCode::Oom,
    ComputeFailureCode::InvalidMessages,
    ComputeFailureCode::ModelError,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeDeliveryFailureReason {
    MaxAttemptsContentOnly,
    MaxAttemptsSettlementOnly,
    MaxAttemptsBothFailed,
}

pub const COMPUTE_DELIVERY_FAILURE_REASONS: &[ComputeDeliveryFailureReason] = &[
    ComputeDeliveryFailureReason::MaxAttemptsContentOnly,
    ComputeDeliveryFailureReason::MaxAttemptsSettlementOnly,
    ComputeDeliveryFailureReason::MaxAttemptsBothFailed,
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceFloatPool {
    pub balance: i64,
    pub max: i64,
    pub inflow_24h: i64,
    pub outflow_24h: i64,
    pub destroyed_24h: i64,
    pub minted_24h: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceEconomic {
    pub float_pool: MarketSurfaceFloatPool,
    pub wire_take_24h: i64,
    pub graph_fund_24h: i64,
    pub reservation_fees_24h: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceVelocity1h {
    pub new_offers: i64,
    pub retired_offers: i64,
    pub rate_changes: i64,
    pub jobs_matched: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceSettled24h {
    pub jobs: i64,
    pub credits: i64,
    pub failure_rate: f64,
    pub median_latency_p95_ms: Option<i64>,
    pub median_tps: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceMarket {
    pub active_providers: i64,
    pub active_offers_total: i64,
    pub models_offered: i64,
    pub total_queue_capacity: i64,
    pub total_queue_depth: i64,
    pub capacity_utilization: f64,
    pub settled_24h: MarketSurfaceSettled24h,
    pub economic: MarketSurfaceEconomic,
    pub velocity_1h: MarketSurfaceVelocity1h,
    pub last_updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfacePriceTriple {
    pub min: Option<i64>,
    pub median: Option<i64>,
    pub max: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfacePrice {
    pub rate_per_m_input: MarketSurfacePriceTriple,
    pub rate_per_m_output: MarketSurfacePriceTriple,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceQueue {
    pub total_capacity: i64,
    pub current_depth: i64,
    pub unbounded_offers: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfacePerformance {
    pub p50_latency_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub median_tps: Option<f64>,
    pub success_rate_7d: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceTopOfBookEntry {
    pub offer_id: String,
    pub operator_handle: String,
    pub rate_per_m_input: i64,
    pub rate_per_m_output: i64,
    pub reservation_fee: i64,
    pub queue_position: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceTopOfBook {
    pub cheapest_with_headroom: Option<MarketSurfaceTopOfBookEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceDemand24h {
    pub jobs_matched: i64,
    pub jobs_settled: i64,
    pub queue_fill_events: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarketSurfaceProviderType {
    Local,
    Bridge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceQueueDiscountEntry {
    pub queue_depth: i64,
    pub discount_bps: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceOffer {
    pub offer_id: String,
    pub operator_handle: String,
    pub node_handle: String,
    pub provider_type: MarketSurfaceProviderType,
    pub rate_per_m_input: i64,
    pub rate_per_m_output: i64,
    pub reservation_fee: i64,
    pub queue_discount_curve: Vec<MarketSurfaceQueueDiscountEntry>,
    pub current_queue_depth: i64,
    pub max_queue_depth: i64,
    pub max_tokens_supported: i64,
    pub observed_median_tps_7d: Option<f64>,
    pub observed_p95_latency_ms_7d: Option<i64>,
    pub observed_success_rate_7d: Option<f64>,
    pub observed_job_count_7d: i64,
    pub last_heartbeat_at: Option<String>,
    pub operator_reputation_compute: Option<f64>,
    pub typical_serve_ms_p50_7d: Option<f64>,
    pub execution_concurrency: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceDepthBucket {
    pub rate_range: [i64; 2],
    pub offer_count: i64,
    pub queue_capacity: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceDepth {
    pub by_rate_input: Vec<MarketSurfaceDepthBucket>,
    pub by_rate_output: Vec<MarketSurfaceDepthBucket>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceModel {
    pub model_id: String,
    pub provider_count: i64,
    pub active_offers: i64,
    pub price: MarketSurfacePrice,
    pub queue: MarketSurfaceQueue,
    pub performance: MarketSurfacePerformance,
    pub top_of_book: MarketSurfaceTopOfBook,
    pub demand_24h: MarketSurfaceDemand24h,
    pub last_offer_update_at: Option<String>,
    pub model_typical_serve_ms_p50_7d: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offers: Option<Vec<MarketSurfaceOffer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<MarketSurfaceDepth>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceCatalog {
    pub model_ids_sorted: Vec<String>,
    pub by_provider_type: std::collections::HashMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceResponse {
    pub market: MarketSurfaceMarket,
    pub models: Vec<MarketSurfaceModel>,
    pub catalog: MarketSurfaceCatalog,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceStreamMarketEvent {
    pub active_providers: i64,
    pub float_pool: MarketSurfaceFloatPool,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceStreamOfferUpsertEvent {
    pub model_id: String,
    pub offer_id: String,
    pub operator_handle: String,
    pub rate_per_m_input: i64,
    pub rate_per_m_output: i64,
    pub reservation_fee: i64,
    pub current_queue_depth: i64,
    pub max_queue_depth: i64,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferRetiredReason {
    Superseded,
    Deactivated,
    NodeOffline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceStreamOfferRetiredEvent {
    pub model_id: String,
    pub offer_id: String,
    pub reason: OfferRetiredReason,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceStreamDepthTickEvent {
    pub model_id: String,
    pub offer_id: String,
    pub current_queue_depth: i64,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketSurfaceStreamHeartbeatExpiredEvent {
    pub node_id: String,
    pub affected_offers: Vec<String>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MarketSurfaceHistoryWindow {
    #[serde(rename = "1h")]
    OneHour,
    #[serde(rename = "24h")]
    TwentyFourHour,
    #[serde(rename = "7d")]
    SevenDay,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceHistoryModelSlice {
    pub median_rate_in: Option<i64>,
    pub active_offers: i64,
    pub jobs_settled: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceHistoryBucket {
    pub bucket_start: String,
    pub market: std::collections::HashMap<String, i64>,
    pub models: std::collections::HashMap<String, MarketSurfaceHistoryModelSlice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSurfaceHistoryResponse {
    pub window: MarketSurfaceHistoryWindow,
    pub bucket_seconds: i64,
    pub series: Vec<MarketSurfaceHistoryBucket>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeEventType {
    ComputeMatchResolved,
    ComputeMatchNoOffer,
    ComputeMatchInsufficientBalance,
    ComputeMatchContributionOrphan,
    ComputeMatchLinkFailed,
    ComputeMatchNoHandlePath,
    ComputeFillDispatched,
    ComputeFillProviderUnreachable,
    ComputeFillJwtRejected,
    ComputeFillProviderRejectedBody,
    ComputeFillProviderConflict,
    ComputeFillIdempotentInFlight,
    ComputeFillIdempotentReplay,
    ComputeFillSsrfRejected,
    ComputeFillRematchExhausted,
    #[serde(rename = "compute_fill_503_allow_market_visibility_false")]
    ComputeFill503AllowMarketVisibilityFalse,
    #[serde(rename = "compute_fill_503_market_compute_held")]
    ComputeFill503MarketComputeHeld,
    #[serde(rename = "compute_fill_503_market_disabled_globally")]
    ComputeFill503MarketDisabledGlobally,
    #[serde(rename = "compute_fill_503_market_serving_disabled")]
    ComputeFill503MarketServingDisabled,
    #[serde(rename = "compute_fill_503_negative_balance")]
    ComputeFill503NegativeBalance,
    #[serde(rename = "compute_fill_503_no_offer_for_model")]
    ComputeFill503NoOfferForModel,
    #[serde(rename = "compute_fill_503_queue_depth_exceeded")]
    ComputeFill503QueueDepthExceeded,
    ComputeOfferPublished,
    ComputeOfferDeactivated,
    ComputeOfferDeactivatedViaDispatchReject,
    ComputeOfferOwnershipRejected,
    ComputeOfferFloorRejected,
    ComputeOfferSupersessionChainBreak,
    ComputeQueueMirrorUpdated,
    ComputeQueueMirrorPrivacyRejected,
    ComputeQueueMirrorUnknownOffer,
    ComputeQueueMirrorRetractedOffer,
    ComputeQueueMirrorUpdateFailed,
    ComputeQueueSeqRegressed,
    ComputeQueueMirrorPushed,
    ComputeNodeSelfPaused,
    ComputeNodeSelfResumed,
    ComputeResultRelayed,
    ComputeResultRelayRejected,
    ComputeResultDelivered,
    ComputeResultForwardedToRequester,
    ComputeSettlementReceived,
    ComputeResultDeliveryFailed,
    ComputeResultDeliveryAttemptFailed,
    ComputeResultExpiredUndelivered,
    ComputeReservationRefunded,
    ComputeProviderSubsidyCompensated,
    ComputeQuoteRejected,
    ComputePurchaseCommitted,
    ComputePurchaseRejected,
    ComputePurchaseExpiredUnloaded,
    ComputeOvershootMinted,
    ComputeRpcError,
    AdminCreditGrantApplied,
    WelcomeBonusGranted,
    OnboardingPinSetDenied,
    OnboardingPinSetPublished,
    OnboardingPinSetSupersessionChainBreak,
    PyramidQueryDenied,
    PyramidQueryMinted,
    PyramidQuerySignError,
}

#[derive(Debug, Clone, Copy)]
pub struct SqlstateMapping {
    pub code: &'static str,
    pub http: u16,
    pub meaning: &'static str,
}

pub const COMPUTE_SQLSTATE: &[SqlstateMapping] = &[
    SqlstateMapping {
        code: "P0400",
        http: 400,
        meaning: "Input validation failure",
    },
    SqlstateMapping {
        code: "P0401",
        http: 401,
        meaning: "Token expired or otherwise unauthenticated (rev 2.1 quote JWT)",
    },
    SqlstateMapping {
        code: "P0403",
        http: 403,
        meaning: "Tenant/ownership mismatch",
    },
    SqlstateMapping {
        code: "P0404",
        http: 404,
        meaning: "No matching provider",
    },
    SqlstateMapping {
        code: "P0409",
        http: 409,
        meaning: "Budget exceeded / already-redeemed / balance race",
    },
    SqlstateMapping {
        code: "P0410",
        http: 409,
        meaning: "Race/retriable (quote_no_longer_winning, provider_queue_full)",
    },
    SqlstateMapping {
        code: "P0411",
        http: 409,
        meaning: "Job not in expected status",
    },
    SqlstateMapping {
        code: "P0412",
        http: 409,
        meaning: "Subsidy cap exceeded (retired rev 2.1; retained for legacy ledger)",
    },
    SqlstateMapping {
        code: "P0500",
        http: 500,
        meaning: "Unexpected internal error",
    },
    SqlstateMapping {
        code: "P0503",
        http: 503,
        meaning: "Platform operator missing",
    },
    SqlstateMapping {
        code: "P0504",
        http: 503,
        meaning: "Economic parameter missing",
    },
];

pub const COMPUTE_ERROR_EVENT_TYPES: &[ComputeEventType] = &[
    ComputeEventType::ComputeMatchNoOffer,
    ComputeEventType::ComputeMatchInsufficientBalance,
    ComputeEventType::ComputeMatchContributionOrphan,
    ComputeEventType::ComputeMatchLinkFailed,
    ComputeEventType::ComputeMatchNoHandlePath,
    ComputeEventType::ComputeFillProviderUnreachable,
    ComputeEventType::ComputeFillJwtRejected,
    ComputeEventType::ComputeFillProviderRejectedBody,
    ComputeEventType::ComputeFillProviderConflict,
    ComputeEventType::ComputeFillSsrfRejected,
    ComputeEventType::ComputeFillRematchExhausted,
    ComputeEventType::ComputeFill503AllowMarketVisibilityFalse,
    ComputeEventType::ComputeFill503MarketComputeHeld,
    ComputeEventType::ComputeFill503MarketDisabledGlobally,
    ComputeEventType::ComputeFill503MarketServingDisabled,
    ComputeEventType::ComputeFill503NegativeBalance,
    ComputeEventType::ComputeFill503NoOfferForModel,
    ComputeEventType::ComputeFill503QueueDepthExceeded,
    ComputeEventType::ComputeOfferOwnershipRejected,
    ComputeEventType::ComputeOfferFloorRejected,
    ComputeEventType::ComputeOfferSupersessionChainBreak,
    ComputeEventType::ComputeOfferDeactivatedViaDispatchReject,
    ComputeEventType::ComputeQueueMirrorPrivacyRejected,
    ComputeEventType::ComputeQueueMirrorUnknownOffer,
    ComputeEventType::ComputeQueueMirrorUpdateFailed,
    ComputeEventType::ComputeQueueSeqRegressed,
    ComputeEventType::ComputeResultRelayRejected,
    ComputeEventType::ComputeResultDeliveryFailed,
    ComputeEventType::ComputeResultDeliveryAttemptFailed,
    ComputeEventType::ComputeResultExpiredUndelivered,
    ComputeEventType::ComputeQuoteRejected,
    ComputeEventType::ComputePurchaseRejected,
    ComputeEventType::ComputePurchaseExpiredUnloaded,
    ComputeEventType::ComputeRpcError,
    ComputeEventType::OnboardingPinSetDenied,
    ComputeEventType::OnboardingPinSetSupersessionChainBreak,
    ComputeEventType::PyramidQueryDenied,
    ComputeEventType::PyramidQuerySignError,
];

impl_wire_dto!(
    AliasVisibilityDto,
    AllOffersSaturatedDetail,
    BudgetExceededDetail,
    ContractVerb,
    ComputeDeliveryFailureReason,
    ComputeEventType,
    ComputeFailureCode,
    ComputePurchaseBody,
    ComputePurchaseResponse,
    ComputePurchaseTrigger,
    ComputeQuoteBody,
    ComputeQuotePriceBreakdown,
    ComputeQuoteResponse,
    DispatchDeadlineExceededDetail,
    EventVisibilityDto,
    EventEnvelopeDto,
    HandleClaimDto,
    HandlePathDto,
    InsufficientBalanceDetail,
    InternalErrorDetail,
    LatencyPreference,
    MarketSurfaceCatalog,
    MarketSurfaceDemand24h,
    MarketSurfaceDepth,
    MarketSurfaceDepthBucket,
    MarketSurfaceEconomic,
    MarketSurfaceFloatPool,
    MarketSurfaceHistoryBucket,
    MarketSurfaceHistoryModelSlice,
    MarketSurfaceHistoryResponse,
    MarketSurfaceHistoryWindow,
    MarketSurfaceMarket,
    MarketSurfaceModel,
    MarketSurfaceOffer,
    MarketSurfacePerformance,
    MarketSurfacePrice,
    MarketSurfacePriceTriple,
    MarketSurfaceProviderType,
    MarketSurfaceQueue,
    MarketSurfaceQueueDiscountEntry,
    MarketSurfaceResponse,
    MarketSurfaceSettled24h,
    MarketSurfaceStreamDepthTickEvent,
    MarketSurfaceStreamHeartbeatExpiredEvent,
    MarketSurfaceStreamMarketEvent,
    MarketSurfaceStreamOfferRetiredEvent,
    MarketSurfaceStreamOfferUpsertEvent,
    MarketSurfaceTopOfBook,
    MarketSurfaceTopOfBookEntry,
    MarketSurfaceVelocity1h,
    MasterKeyRotationDto,
    MaxTokensExceedsQuoteDetail,
    MessageDetail,
    MoneyAmountDto,
    MultipleNodesDetail,
    NoOfferDetail,
    OfferRetiredReason,
    PrivateAliasMappingDto,
    PrivateGraphRegistrationDto,
    QueueDepthExceededDetail,
    QuoteAlreadyPurchasedDetail,
    QuoteJwtExpiredDetail,
    QuoteJwtInvalidDetail,
    QuoteJwtInvalidReason,
    QuoteNoLongerWinningDetail,
    ReputationSnapshotDto,
    RetryIntent,
    TunnelEndpointDto,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_event_type_serializes_snake_case() {
        let serialized = serde_json::to_string(&ComputeEventType::ComputeMatchNoOffer).unwrap();
        assert_eq!(serialized, "\"compute_match_no_offer\"");
    }

    #[test]
    fn compute_event_type_round_trips_explicit_names() {
        let event: ComputeEventType =
            serde_json::from_str("\"compute_queue_mirror_pushed\"").unwrap();
        assert_eq!(event, ComputeEventType::ComputeQueueMirrorPushed);

        let event: ComputeEventType =
            serde_json::from_str("\"compute_fill_503_market_compute_held\"").unwrap();
        assert_eq!(event, ComputeEventType::ComputeFill503MarketComputeHeld);
    }

    #[test]
    fn sqlstate_table_has_expected_codes() {
        let codes = COMPUTE_SQLSTATE
            .iter()
            .map(|mapping| mapping.code)
            .collect::<Vec<_>>();
        assert!(codes.contains(&"P0400"));
        assert!(codes.contains(&"P0504"));
    }

    #[test]
    fn settlement_received_event_round_trips() {
        let event: ComputeEventType =
            serde_json::from_str("\"compute_settlement_received\"").unwrap();
        assert_eq!(event, ComputeEventType::ComputeSettlementReceived);
        let serialized =
            serde_json::to_string(&ComputeEventType::ComputeSettlementReceived).unwrap();
        assert_eq!(serialized, "\"compute_settlement_received\"");
    }

    #[test]
    fn quote_event_types_round_trip() {
        for (variant, expected) in [
            (
                ComputeEventType::ComputeQuoteRejected,
                "\"compute_quote_rejected\"",
            ),
            (
                ComputeEventType::ComputePurchaseCommitted,
                "\"compute_purchase_committed\"",
            ),
            (
                ComputeEventType::ComputePurchaseRejected,
                "\"compute_purchase_rejected\"",
            ),
            (
                ComputeEventType::ComputePurchaseExpiredUnloaded,
                "\"compute_purchase_expired_unloaded\"",
            ),
            (
                ComputeEventType::ComputeOvershootMinted,
                "\"compute_overshoot_minted\"",
            ),
        ] {
            let serialized = serde_json::to_string(&variant).unwrap();
            assert_eq!(serialized, expected);
            let parsed: ComputeEventType = serde_json::from_str(expected).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn purchase_trigger_round_trips() {
        for (variant, expected) in [
            (ComputePurchaseTrigger::Immediate, "\"immediate\""),
            (ComputePurchaseTrigger::Time, "\"time\""),
            (ComputePurchaseTrigger::Cron, "\"cron\""),
            (ComputePurchaseTrigger::Event, "\"event\""),
            (ComputePurchaseTrigger::Threshold, "\"threshold\""),
            (ComputePurchaseTrigger::After, "\"after\""),
            (ComputePurchaseTrigger::Manual, "\"manual\""),
        ] {
            let serialized = serde_json::to_string(&variant).unwrap();
            assert_eq!(serialized, expected);
            let parsed: ComputePurchaseTrigger = serde_json::from_str(expected).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn latency_preference_round_trips() {
        for (variant, expected) in [
            (LatencyPreference::BestPrice, "\"best_price\""),
            (LatencyPreference::Balanced, "\"balanced\""),
            (LatencyPreference::LowestLatency, "\"lowest_latency\""),
        ] {
            let serialized = serde_json::to_string(&variant).unwrap();
            assert_eq!(serialized, expected);
            let parsed: LatencyPreference = serde_json::from_str(expected).unwrap();
            assert_eq!(parsed, variant);
        }
    }

    #[test]
    fn delivery_failure_reason_round_trips() {
        for (variant, expected) in [
            (
                ComputeDeliveryFailureReason::MaxAttemptsContentOnly,
                "\"max_attempts_content_only\"",
            ),
            (
                ComputeDeliveryFailureReason::MaxAttemptsSettlementOnly,
                "\"max_attempts_settlement_only\"",
            ),
            (
                ComputeDeliveryFailureReason::MaxAttemptsBothFailed,
                "\"max_attempts_both_failed\"",
            ),
        ] {
            let serialized = serde_json::to_string(&variant).unwrap();
            assert_eq!(serialized, expected);
            let parsed: ComputeDeliveryFailureReason = serde_json::from_str(expected).unwrap();
            assert_eq!(parsed, variant);
        }
        assert_eq!(COMPUTE_DELIVERY_FAILURE_REASONS.len(), 3);
    }

    #[test]
    fn error_body_allows_missing_optional_fields_without_detail_default() {
        let body: ErrorBody<MessageDetail> =
            serde_json::from_str(r#"{"error":"internal_error"}"#).unwrap();
        assert_eq!(body.error, "internal_error");
        assert!(body.detail.is_none());
        assert!(body.hint.is_none());
        assert!(body.retry_after_ms.is_none());
    }

    #[test]
    fn compute_contract_types_are_sealed_wire_dtos() {
        fn assert_wire_dto<T: WireDto>() {}

        assert_wire_dto::<ComputeQuoteBody>();
        assert_wire_dto::<ComputePurchaseResponse>();
        assert_wire_dto::<MarketSurfaceResponse>();
        assert_wire_dto::<ComputeEventType>();
        assert_wire_dto::<ErrorBody<serde_json::Value>>();
        assert_wire_dto::<ContractWrap<ComputeQuoteBody>>();
    }
}
