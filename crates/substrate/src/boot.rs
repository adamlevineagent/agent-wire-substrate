use agent_wire_compiler::CompilerOpManifest;
use agent_wire_compute_market::{
    ComputeJobContract, ComputeJobEnvelope, ComputeOffer, ComputeOfferId, DispatchPolicy,
    ExecutionAdapterId, LatencyPreference, ModelInvocation, ProviderNodeId, ProviderType,
    QueueAdmission, QueueDiscount,
};
use agent_wire_contracts::ContractVerb;
use agent_wire_foundation::{
    CallbackUrl, CreditAmount, CrossGraphRef, EndpointUrl, FoundationError, GraphSlug, HandlePath,
    PriceCurve, SettlementIntent, TransportDriver, TunnelRequest, TunnelSession, VocabularyEntry,
    VocabularyNamespace,
};
use agent_wire_relay_market::{HopCapability, PrivacyTier, RelayOffer, RelayOfferId};
use agent_wire_storage_market::{
    CapacityAllocation, ReplicationFactor, RetentionPolicy, StorageOffer, StorageOfferId,
};
use agent_wire_transport_cloudflare::CloudflareTunnelDriver;
use serde::{Deserialize, Serialize};

use crate::config::NodeConfig;
use crate::lifecycle::BackgroundWorkerLifecycle;
use crate::server::OperatorApiSurface;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRuntime {
    pub config: NodeConfig,
    pub transport: TunnelSession,
    pub lifecycle: BackgroundWorkerLifecycle,
    pub api: OperatorApiSurface,
    pub markets: MarketComposition,
    pub compiler: CompilerOpManifest,
    pub vocabulary: VocabularyEntry,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketComposition {
    pub contract_verb: ContractVerb,
    pub compute_job: ComputeJobContract,
    pub compute_offer: ComputeOffer,
    pub storage_offer: StorageOffer,
    pub relay_offer: RelayOffer,
}

pub fn compose_substrate_node(config: NodeConfig) -> Result<NodeRuntime, FoundationError> {
    let driver = CloudflareTunnelDriver::with_static_tunnel(config.requested_tunnel.clone());
    let transport = driver.open_tunnel(
        TunnelRequest::new(config.local_api_endpoint.clone())
            .with_callback(CallbackUrl::parse("https://node2-demo.example/callback")?),
    )?;

    Ok(NodeRuntime {
        api: config.surfaces.clone(),
        lifecycle: BackgroundWorkerLifecycle::substrate_default(),
        markets: compose_market_bundle(&config.operator)?,
        compiler: CompilerOpManifest::v1(),
        vocabulary: compose_vocabulary_entry()?,
        config,
        transport,
    })
}

fn compose_market_bundle(operator: &HandlePath) -> Result<MarketComposition, FoundationError> {
    let compute_offer = ComputeOffer {
        offer_id: ComputeOfferId("compute-offer-demo".to_owned()),
        provider: operator.clone(),
        provider_node_id: ProviderNodeId("node2-demo".to_owned()),
        provider_type: ProviderType::Local,
        model_id: "wire-demo-model".to_owned(),
        adapter: ExecutionAdapterId("local-adapter".to_owned()),
        price: PriceCurve {
            base: CreditAmount::from_sats(10),
            per_unit: CreditAmount::from_sats(2),
        },
        reservation_fee: CreditAmount::from_sats(1),
        queue_discount_curve: vec![QueueDiscount {
            queue_depth: 8,
            discount_bps: 200,
        }],
        max_queue_depth: 64,
        settlement: settlement(1_000),
    };

    let compute_job = ComputeJobContract::wrap(ComputeJobEnvelope {
        job_ref: demo_ref("compute", 2)?,
        requester: "agent/playful/kramer".to_owned(),
        requester_handle: operator.clone(),
        invocation: ModelInvocation {
            model_id: "wire-demo-model".to_owned(),
            adapter: ExecutionAdapterId("local-adapter".to_owned()),
            prompt_ref: demo_ref("prompt", 1)?,
            input_ref: Some(demo_ref("input", 1)?),
            max_tokens: Some(128),
            temperature_milli: Some(700),
        },
        budget: CreditAmount::from_sats(500),
        settlement: settlement(500),
        delivery: agent_wire_compute_market::DeliveryPolicy {
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
            latency_preference: LatencyPreference::Balanced,
            require_reputation: true,
            max_price: CreditAmount::from_sats(450),
        },
    });

    let storage_offer = StorageOffer {
        offer_id: StorageOfferId("storage-offer-demo".to_owned()),
        provider: operator.clone(),
        capacity_bytes: 1_000_000,
        capacity_allocation: vec![CapacityAllocation {
            graph: GraphSlug::new("kitty")?,
            reserved_bytes: 250_000,
        }],
        price: PriceCurve {
            base: CreditAmount::from_sats(5),
            per_unit: CreditAmount::from_sats(1),
        },
        replication: ReplicationFactor(3),
        retention: RetentionPolicy {
            minimum_seconds: 86_400,
            renew_before_expiry_seconds: Some(3_600),
        },
        settlement: settlement(500),
    };

    let relay_offer = RelayOffer {
        offer_id: RelayOfferId("relay-offer-demo".to_owned()),
        operator: operator.clone(),
        ingress: EndpointUrl::parse("https://node2-demo.example/ingress")?,
        egress: agent_wire_foundation::TunnelUrl::parse("https://node2-demo.example/tunnel")?,
        capabilities: vec![HopCapability::HttpTunnel, HopCapability::EventStream],
        privacy_tiers: vec![PrivacyTier::Direct, PrivacyTier::Shielded],
        price_per_hop: CreditAmount::from_sats(3),
        settlement: settlement(250),
    };

    Ok(MarketComposition {
        contract_verb: compute_job.verb,
        compute_job,
        compute_offer,
        storage_offer,
        relay_offer,
    })
}

fn compose_vocabulary_entry() -> Result<VocabularyEntry, FoundationError> {
    let vocabulary = VocabularyNamespace::new(
        agent_wire_foundation::NamespaceId::new("playful")?,
        "wire-v2",
    )?;
    VocabularyEntry::compute_primitive_entry(vocabulary, demo_ref("vocabulary", 1)?)
}

fn demo_ref(slug: &str, sequence: u32) -> Result<CrossGraphRef, FoundationError> {
    format!("playful/122/{slug}/{sequence}").parse()
}

fn settlement(max_price: u128) -> SettlementIntent {
    SettlementIntent {
        max_price: CreditAmount::from_sats(max_price),
        escrow_required: true,
    }
}
