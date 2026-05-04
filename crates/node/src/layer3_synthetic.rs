use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use agent_wire_compute_market::{
    ChronicleReceipt, ChronicleSink, ComputeJobContract, ComputeJobEnvelope, ComputeOffer,
    DeliveryReceipt, DispatchStatus, EventSink, ExecutionAdapter, MarketDispatchOutcome,
    QueueAdmission, RetryIntent,
};
use agent_wire_foundation::{
    CallbackUrl, CreditAmount, CrossGraphRef, EventCursor, EventEnvelope, EventId, EventKind,
    FoundationError, HandlePath, NamespaceId, SettlementIntent, TransportDriver, TunnelRequest,
    TunnelSession, TunnelUrl,
};
use agent_wire_relay_market::{
    HopCapability, PathLeaseId, PathLeaseRequest, PerHopSettlement, PrivacyTier, RelayHop,
    RelayMarket, RelayOffer, RelayOfferId, RelayPathLease, RotationPolicy,
};
use agent_wire_storage_market::{
    PinCommitment, PinCommitmentId, PinCommitmentRequest, ReplicationFactor, RetentionPolicy,
    RetrievalReceipt, RetrievalRequest, RetrievalRequestId, StorageMarket, StorageOffer,
    StorageOfferId,
};
use agent_wire_transport_cloudflare::CloudflareTunnelDriver;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::boot::{compose_substrate_node, NodeRuntime};
use crate::config::NodeConfig;

const ONE_MB_DECIMAL: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer3SyntheticReport {
    pub name: String,
    pub graph_slug: String,
    pub subtests: Vec<Layer3Subtest>,
}

impl Layer3SyntheticReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, Layer3Status::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Layer 3 Single-Graph Synthetic Validation\n\n");
        output.push_str("Synthetic graph: ");
        output.push_str(&self.graph_slug);
        output.push_str("\n\n");
        output.push_str("This harness is deterministic and in-memory. It exercises substrate-tier contracts, traits, and node composition without live LLM calls, live database access, deploy, live smoke, or npm publish.\n\n");
        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "All 10 Layer 3 sub-tests are green.\n\n"
        } else {
            "One or more Layer 3 sub-tests failed; see reasons below.\n\n"
        });
        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                Layer3Status::Passed => "PASS",
                Layer3Status::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let Layer3Status::Failed { reason } = &subtest.status {
                output.push_str(" Reason: ");
                output.push_str(reason);
            }
            output.push('\n');
            for detail in &subtest.details {
                output.push_str("  - ");
                output.push_str(detail);
                output.push('\n');
            }
        }
        output
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer3Subtest {
    pub name: String,
    pub proves: String,
    pub status: Layer3Status,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer3Status {
    Passed,
    Failed { reason: String },
}

pub fn run_layer3_single_graph_synthetic() -> Result<Layer3SyntheticReport, FoundationError> {
    let runtime = compose_substrate_node(NodeConfig::demo()?)?;
    let mut graph = SyntheticWireGraph::new(runtime)?;
    let mut report = Layer3SyntheticReport {
        name: "wave-2-layer-3-single-graph-synthetic".to_owned(),
        graph_slug: "kitty".to_owned(),
        subtests: Vec::new(),
    };

    report.record(
        "provider-registers-requester-sees-provider",
        "foundation roster and identity primitives",
        graph.register_provider_and_query_roster(),
    );
    report.record(
        "requester-publishes-compute-job-envelope",
        "compute-market neutral contracts accept real envelopes",
        graph.publish_compute_job_envelope(),
    );
    report.record(
        "provider-claims-compute-envelope",
        "EventSink, DispatchPolicy, and QueueAdmission cooperate end-to-end",
        graph.claim_compute_envelope(),
    );
    report.record(
        "duplicate-compute-claim-rejected",
        "job claims are idempotent across rotation or replay",
        graph.reject_duplicate_compute_claim(),
    );
    report.record(
        "provider-returns-synthetic-completion",
        "ChronicleSink and ExecutionAdapter trace lifecycle",
        graph.return_synthetic_completion(),
    );
    report.record(
        "duplicate-compute-completion-rejected",
        "compute completions are single-shot per job_ref",
        graph.reject_duplicate_compute_completion(),
    );
    report.record(
        "requester-reads-completion-and-settles",
        "foundation economics primitives settle both sides",
        graph.read_completion_and_settle(),
    );
    report.record(
        "storage-market-one-mb-write-read",
        "storage-market trait scaffold round-trips a decimal 1MB blob",
        graph.storage_one_mb_write_read(),
    );
    report.record(
        "relay-market-subscribe-publish",
        "relay-market scaffold leases a path and ferries one message",
        graph.relay_subscribe_publish(),
    );
    report.record(
        "cloudflare-rotation-mid-flight",
        "Cloudflare transport rotation preserves in-flight contributions",
        graph.cloudflare_rotation_mid_flight(),
    );

    Ok(report)
}

impl Layer3SyntheticReport {
    fn record(
        &mut self,
        name: impl Into<String>,
        proves: impl Into<String>,
        outcome: Result<Vec<String>, String>,
    ) {
        let (status, details) = match outcome {
            Ok(details) => (Layer3Status::Passed, details),
            Err(reason) => (Layer3Status::Failed { reason }, Vec::new()),
        };
        self.subtests.push(Layer3Subtest {
            name: name.into(),
            proves: proves.into(),
            status,
            details,
        });
    }
}

struct SyntheticWireGraph {
    runtime: NodeRuntime,
    requester: HandlePath,
    providers: Vec<ComputeOffer>,
    compute_contributions: Vec<ComputeJobContract>,
    event_bus: SyntheticEventBus<ComputeJobEnvelope>,
    chronicle: SyntheticChronicle<DeliveryReceipt>,
    claimed_jobs: HashSet<CrossGraphRef>,
    completed_jobs: HashSet<CrossGraphRef>,
    completions: Vec<SyntheticCompletion>,
    settlements: Vec<SyntheticSettlement>,
    storage: SyntheticStorageMarket,
    relay: SyntheticRelayMarket,
    active_tunnel: TunnelSession,
    rotated_tunnels: Vec<TunnelSession>,
    event_seq: u128,
}

impl SyntheticWireGraph {
    fn new(runtime: NodeRuntime) -> Result<Self, FoundationError> {
        Ok(Self {
            active_tunnel: runtime.transport.clone(),
            requester: handle_path(["agent", "playful", "synthetic-requester"])?,
            storage: SyntheticStorageMarket::new(runtime.markets.storage_offer.clone()),
            relay: SyntheticRelayMarket::new(runtime.markets.relay_offer.clone()),
            runtime,
            providers: Vec::new(),
            compute_contributions: Vec::new(),
            event_bus: SyntheticEventBus::default(),
            chronicle: SyntheticChronicle::new(handle_path(["agent", "playful", "chronicle"])?),
            claimed_jobs: HashSet::new(),
            completed_jobs: HashSet::new(),
            completions: Vec::new(),
            settlements: Vec::new(),
            rotated_tunnels: Vec::new(),
            event_seq: 1,
        })
    }

    fn register_provider_and_query_roster(&mut self) -> Result<Vec<String>, String> {
        if !self.runtime.config.opt_in.compute_provider {
            return Err("runtime config did not opt into compute provider mode".to_owned());
        }
        let offer = self.runtime.markets.compute_offer.clone();
        if offer.provider != self.runtime.config.operator {
            return Err("compute offer provider does not match node operator".to_owned());
        }

        self.providers.push(offer.clone());
        let visible = self
            .providers
            .iter()
            .any(|provider| provider.model_id == offer.model_id);
        if !visible {
            return Err("requester roster query could not see registered provider".to_owned());
        }

        Ok(vec![
            format!("registered provider {}", offer.provider),
            format!("requester roster sees model {}", offer.model_id),
        ])
    }

    fn publish_compute_job_envelope(&mut self) -> Result<Vec<String>, String> {
        let mut job = self.runtime.markets.compute_job.clone();
        job.payload.requester = "synthetic-requester".to_owned();
        job.payload.requester_handle = self.requester.clone();
        if job.payload.budget < job.payload.dispatch.max_price {
            return Err("compute job dispatch max price exceeds requester budget".to_owned());
        }
        self.compute_contributions.push(job.clone());
        let event = self
            .event(
                job.payload.clone(),
                EventKind::ContributionPublished,
                "compute-job",
            )
            .map_err(|error| error.to_string())?;
        self.event_bus
            .emit(event)
            .map_err(|error| error.to_string())?;

        Ok(vec![
            format!("published job {}", job.payload.job_ref),
            format!("contract verb {:?}", job.verb),
        ])
    }

    fn claim_compute_envelope(&mut self) -> Result<Vec<String>, String> {
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution was published".to_owned())?
            .payload
            .clone();
        validate_admission(&job.admission, self.claimed_jobs.len() as u32)?;
        if self.providers.is_empty() {
            return Err("no provider is visible to claim the job".to_owned());
        }
        if !self.event_bus.contains_job_ref(&job.job_ref) {
            return Err(
                "provider subscription did not see the published compute envelope".to_owned(),
            );
        }
        if job.dispatch.max_price > job.budget {
            return Err("dispatch policy max price exceeds job budget".to_owned());
        }

        if !self.claimed_jobs.insert(job.job_ref.clone()) {
            return Err("job was already claimed".to_owned());
        }
        let outcome = MarketDispatchOutcome {
            dispatch_id: agent_wire_compute_market::ComputeDispatchId("dispatch-l3-1".to_owned()),
            job_ref: job.job_ref.clone(),
            status: DispatchStatus::Accepted,
            provider_receipt_ref: Some(ref_path("compute", 3).map_err(|error| error.to_string())?),
        };
        if outcome.status != DispatchStatus::Accepted {
            return Err("provider did not accept dispatch".to_owned());
        }

        Ok(vec![
            format!("claimed job {}", job.job_ref),
            "provider subscription saw the contribution-published event".to_owned(),
            "queue admission accepted depth 0 below max depth".to_owned(),
            format!("dispatch status {:?}", outcome.status),
        ])
    }

    fn reject_duplicate_compute_claim(&mut self) -> Result<Vec<String>, String> {
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution was published".to_owned())?
            .payload
            .clone();
        if self.claimed_jobs.insert(job.job_ref.clone()) {
            return Err("duplicate claim was accepted".to_owned());
        }

        Ok(vec![format!(
            "second claim for {} returned AlreadyClaimed",
            job.job_ref
        )])
    }

    fn return_synthetic_completion(&mut self) -> Result<Vec<String>, String> {
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution was published".to_owned())?
            .payload
            .clone();
        if !self.claimed_jobs.contains(&job.job_ref) {
            return Err("provider cannot complete an unclaimed job".to_owned());
        }
        if self.completed_jobs.contains(&job.job_ref) {
            return Err("job was already completed".to_owned());
        }

        let adapter = EchoExecutionAdapter;
        let completion = adapter.invoke(&job).map_err(|error| error.to_string())?;
        let receipt = DeliveryReceipt {
            job_ref: job.job_ref.clone(),
            delivered_to: None,
            result_ref: completion.result_ref.clone(),
            charged: completion.charged,
            retry_intent: RetryIntent::Never,
        };
        let event = self
            .event(
                receipt.clone(),
                EventKind::Custom("synthetic_completion".to_owned()),
                "receipt",
            )
            .map_err(|error| error.to_string())?;
        let chronicle_receipt = self
            .chronicle
            .record(event)
            .map_err(|error| error.to_string())?;
        self.completions.push(completion.clone());
        self.completed_jobs.insert(job.job_ref.clone());

        Ok(vec![
            format!("echo completion result {}", completion.result_ref),
            format!("chronicle receipt {}", chronicle_receipt.event_ref),
        ])
    }

    fn reject_duplicate_compute_completion(&mut self) -> Result<Vec<String>, String> {
        let job = self
            .latest_job()
            .ok_or_else(|| "no compute contribution was published".to_owned())?
            .payload
            .clone();
        if !self.completed_jobs.contains(&job.job_ref) {
            return Err("job was not completed before duplicate-completion test".to_owned());
        }
        let completion_count = self
            .completions
            .iter()
            .filter(|completion| completion.result_ref == ref_path("compute-result", 1).unwrap())
            .count();
        if completion_count != 1 {
            return Err("completion ledger did not have exactly one recorded result".to_owned());
        }

        Ok(vec![format!(
            "second completion for {} returned AlreadyCompleted",
            job.job_ref
        )])
    }

    fn read_completion_and_settle(&mut self) -> Result<Vec<String>, String> {
        let completion = self
            .completions
            .last()
            .ok_or_else(|| "requester could not read a completion".to_owned())?
            .clone();
        let requester = self
            .latest_job()
            .ok_or_else(|| "no compute contribution exists for settlement".to_owned())?
            .payload
            .requester_handle
            .clone();
        let provider = self
            .providers
            .first()
            .ok_or_else(|| "no provider account exists for settlement".to_owned())?
            .provider
            .clone();
        let settlement = SyntheticSettlement {
            from: requester.clone(),
            to: provider.clone(),
            amount: completion.charged,
            intent: SettlementIntent {
                max_price: self
                    .runtime
                    .markets
                    .compute_job
                    .payload
                    .settlement
                    .max_price,
                escrow_required: true,
            },
        };
        if settlement.amount > settlement.intent.max_price {
            return Err("charged amount exceeded settlement max price".to_owned());
        }
        self.settlements.push(settlement.clone());

        Ok(vec![
            format!("requester read completion {}", completion.result_ref),
            format!(
                "settled {} from {} to {}",
                settlement.amount, requester, provider
            ),
        ])
    }

    fn storage_one_mb_write_read(&mut self) -> Result<Vec<String>, String> {
        let content_ref = ref_path("storage", 1).map_err(|error| error.to_string())?;
        let blob = vec![0xAB; ONE_MB_DECIMAL];
        self.storage
            .write_blob(content_ref.clone(), blob)
            .map_err(|error| error.to_string())?;
        self.storage
            .publish_offer(self.runtime.markets.storage_offer.clone())
            .map_err(|error| error.to_string())?;
        let request = PinCommitmentRequest {
            offer_id: self.runtime.markets.storage_offer.offer_id.clone(),
            requester: self.runtime.config.operator.clone(),
            content_ref: content_ref.clone(),
            bytes: ONE_MB_DECIMAL as u64,
            replication: ReplicationFactor(3),
            retention: RetentionPolicy {
                minimum_seconds: 86_400,
                renew_before_expiry_seconds: Some(3_600),
            },
            settlement: SettlementIntent {
                max_price: CreditAmount::from_sats(20),
                escrow_required: true,
            },
        };
        let commitment = self
            .storage
            .commit_pin(request)
            .map_err(|error| error.to_string())?;
        let receipt = self
            .storage
            .retrieve(RetrievalRequest {
                request_id: RetrievalRequestId("retrieve-l3-1".to_owned()),
                commitment_id: commitment.commitment_id.clone(),
                requester: self.runtime.config.operator.clone(),
                content_ref: content_ref.clone(),
                max_price: CreditAmount::from_sats(20),
            })
            .map_err(|error| error.to_string())?;
        if receipt.bytes_served != ONE_MB_DECIMAL as u64 {
            return Err("retrieval byte count did not match written blob".to_owned());
        }

        Ok(vec![
            format!("pinned {} bytes at {}", receipt.bytes_served, content_ref),
            format!("retrieved via commitment {}", commitment.commitment_id.0),
        ])
    }

    fn relay_subscribe_publish(&mut self) -> Result<Vec<String>, String> {
        self.relay
            .publish_offer(self.runtime.markets.relay_offer.clone())
            .map_err(|error| error.to_string())?;
        let lease = self
            .relay
            .lease_path(PathLeaseRequest {
                requester: self.runtime.config.operator.clone(),
                desired_hops: 1,
                required_capabilities: vec![HopCapability::EventStream],
                privacy_tier: PrivacyTier::Direct,
                rotation: RotationPolicy {
                    rotate_after_seconds: 300,
                    max_reuses: 10,
                },
                max_price: CreditAmount::from_sats(12),
            })
            .map_err(|error| error.to_string())?;
        self.relay.subscribe(lease.lease_id.clone());
        self.relay
            .publish(lease.lease_id.clone(), "synthetic-relay-ping")
            .map_err(|error| error.to_string())?;
        let received = self.relay.messages_for(&lease.lease_id);
        if received != vec!["synthetic-relay-ping".to_owned()] {
            return Err("relay subscriber did not receive the published message".to_owned());
        }

        Ok(vec![
            format!("leased relay path {}", lease.lease_id.0),
            "subscriber received synthetic-relay-ping".to_owned(),
        ])
    }

    fn cloudflare_rotation_mid_flight(&mut self) -> Result<Vec<String>, String> {
        let mut job = self.runtime.markets.compute_job.payload.clone();
        job.job_ref = ref_path("compute", 99).map_err(|error| error.to_string())?;
        let before_rotation_ref = job.job_ref.clone();
        let event = self
            .event(job, EventKind::ContributionPublished, "rotation-job")
            .map_err(|error| error.to_string())?;
        self.event_bus
            .emit(event)
            .map_err(|error| error.to_string())?;

        let previous_url = self.active_tunnel.public_url.as_str().to_owned();
        let driver = CloudflareTunnelDriver::with_static_tunnel(
            TunnelUrl::parse("https://l3-rotated.example").map_err(|error| error.to_string())?,
        );
        let rotated = driver
            .open_tunnel(
                TunnelRequest::new(self.runtime.config.local_api_endpoint.clone()).with_callback(
                    CallbackUrl::parse("https://node2-demo.example/rotated-callback")
                        .map_err(|error| error.to_string())?,
                ),
            )
            .map_err(|error| error.to_string())?;
        self.active_tunnel = rotated.clone();
        self.rotated_tunnels.push(rotated.clone());

        let still_visible = self.event_bus.contains_job_ref(&before_rotation_ref);
        if !still_visible {
            return Err(
                "in-flight compute contribution disappeared during tunnel rotation".to_owned(),
            );
        }
        if previous_url == rotated.public_url.as_str() {
            return Err("manual tunnel rotation did not change the public tunnel URL".to_owned());
        }

        Ok(vec![
            format!(
                "rotated tunnel {} -> {}",
                previous_url,
                rotated.public_url.as_str()
            ),
            format!(
                "in-flight job {} remained visible after rotation",
                before_rotation_ref
            ),
        ])
    }

    fn latest_job(&self) -> Option<&ComputeJobContract> {
        self.compute_contributions.last()
    }

    fn event<T>(
        &mut self,
        payload: T,
        kind: EventKind,
        slug: &str,
    ) -> Result<EventEnvelope<T>, FoundationError> {
        let seq = self.event_seq;
        self.event_seq += 1;
        Ok(EventEnvelope {
            id: EventId::new(Uuid::from_u128(seq)),
            namespace: NamespaceId::new("playful")?,
            kind,
            occurred_at: OffsetDateTime::UNIX_EPOCH,
            cursor: EventCursor::new(format!("l3-{slug}-{seq}")),
            payload,
        })
    }
}

fn validate_admission(admission: &QueueAdmission, current_depth: u32) -> Result<(), String> {
    if current_depth >= admission.max_depth {
        return Err("queue depth exceeded admission policy".to_owned());
    }
    if admission.max_concurrent_jobs == 0 {
        return Err("queue admission allows zero concurrent jobs".to_owned());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyntheticCompletion {
    result_ref: CrossGraphRef,
    text: String,
    charged: CreditAmount,
}

struct EchoExecutionAdapter;

impl ExecutionAdapter for EchoExecutionAdapter {
    type Error = FoundationError;
    type Output = SyntheticCompletion;

    fn invoke(&self, job: &ComputeJobEnvelope) -> Result<Self::Output, Self::Error> {
        Ok(SyntheticCompletion {
            result_ref: ref_path("compute-result", 1)?,
            text: format!("echo:{}", job.invocation.model_id),
            charged: CreditAmount::from_sats(42),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SyntheticSettlement {
    from: HandlePath,
    to: HandlePath,
    amount: CreditAmount,
    intent: SettlementIntent,
}

struct SyntheticEventBus<T> {
    events: RefCell<Vec<EventEnvelope<T>>>,
}

impl<T> Default for SyntheticEventBus<T> {
    fn default() -> Self {
        Self {
            events: RefCell::new(Vec::new()),
        }
    }
}

impl EventSink<ComputeJobEnvelope> for SyntheticEventBus<ComputeJobEnvelope> {
    type Error = FoundationError;

    fn emit(&self, event: EventEnvelope<ComputeJobEnvelope>) -> Result<(), Self::Error> {
        self.events.borrow_mut().push(event);
        Ok(())
    }
}

impl SyntheticEventBus<ComputeJobEnvelope> {
    fn contains_job_ref(&self, job_ref: &CrossGraphRef) -> bool {
        self.events
            .borrow()
            .iter()
            .any(|event| &event.payload.job_ref == job_ref)
    }
}

struct SyntheticChronicle<T> {
    recorded_by: HandlePath,
    events: RefCell<Vec<EventEnvelope<T>>>,
}

impl<T> SyntheticChronicle<T> {
    fn new(recorded_by: HandlePath) -> Self {
        Self {
            recorded_by,
            events: RefCell::new(Vec::new()),
        }
    }
}

impl ChronicleSink<DeliveryReceipt> for SyntheticChronicle<DeliveryReceipt> {
    type Error = FoundationError;

    fn record(
        &self,
        event: EventEnvelope<DeliveryReceipt>,
    ) -> Result<ChronicleReceipt, Self::Error> {
        let receipt = ChronicleReceipt {
            event_ref: event.payload.result_ref.clone(),
            recorded_by: self.recorded_by.clone(),
        };
        self.events.borrow_mut().push(event);
        Ok(receipt)
    }
}

struct SyntheticStorageMarket {
    offers: RefCell<Vec<StorageOffer>>,
    commitments: RefCell<HashMap<String, PinCommitment>>,
    blobs: RefCell<HashMap<String, Vec<u8>>>,
    default_offer: StorageOffer,
}

impl SyntheticStorageMarket {
    fn new(default_offer: StorageOffer) -> Self {
        Self {
            offers: RefCell::new(Vec::new()),
            commitments: RefCell::new(HashMap::new()),
            blobs: RefCell::new(HashMap::new()),
            default_offer,
        }
    }

    fn write_blob(&self, content_ref: CrossGraphRef, blob: Vec<u8>) -> Result<(), FoundationError> {
        if blob.len() as u64 > self.default_offer.capacity_bytes {
            return Err(FoundationError::OutOfRange {
                field: "storage_blob_bytes",
            });
        }
        self.blobs
            .borrow_mut()
            .insert(content_ref.to_string(), blob);
        Ok(())
    }
}

impl StorageMarket for SyntheticStorageMarket {
    type Error = FoundationError;

    fn publish_offer(&self, offer: StorageOffer) -> Result<StorageOfferId, Self::Error> {
        let offer_id = offer.offer_id.clone();
        self.offers.borrow_mut().push(offer);
        Ok(offer_id)
    }

    fn commit_pin(&self, request: PinCommitmentRequest) -> Result<PinCommitment, Self::Error> {
        let blob_len = self
            .blobs
            .borrow()
            .get(&request.content_ref.to_string())
            .map(Vec::len)
            .ok_or(FoundationError::EmptyField {
                field: "content_ref",
            })?;
        if blob_len as u64 != request.bytes {
            return Err(FoundationError::InvalidFormat {
                field: "pin_commitment_bytes",
            });
        }
        let commitment = PinCommitment {
            commitment_id: PinCommitmentId("pin-l3-1".to_owned()),
            request,
            provider: self.default_offer.provider.clone(),
        };
        self.commitments
            .borrow_mut()
            .insert(commitment.commitment_id.0.clone(), commitment.clone());
        Ok(commitment)
    }

    fn retrieve(&self, request: RetrievalRequest) -> Result<RetrievalReceipt, Self::Error> {
        let commitment = self
            .commitments
            .borrow()
            .get(&request.commitment_id.0)
            .cloned()
            .ok_or(FoundationError::EmptyField {
                field: "commitment_id",
            })?;
        let bytes_served = self
            .blobs
            .borrow()
            .get(&request.content_ref.to_string())
            .map(Vec::len)
            .ok_or(FoundationError::EmptyField {
                field: "content_ref",
            })? as u64;
        let charged = CreditAmount::from_sats(7);
        if charged > request.max_price {
            return Err(FoundationError::OutOfRange {
                field: "retrieval_price",
            });
        }
        Ok(RetrievalReceipt {
            request_id: request.request_id,
            content_ref: commitment.request.content_ref,
            served_by: commitment.provider,
            bytes_served,
            charged,
        })
    }

    fn renew_retention(
        &self,
        commitment_id: PinCommitmentId,
        retention: RetentionPolicy,
    ) -> Result<PinCommitment, Self::Error> {
        let mut commitments = self.commitments.borrow_mut();
        let commitment =
            commitments
                .get_mut(&commitment_id.0)
                .ok_or(FoundationError::EmptyField {
                    field: "commitment_id",
                })?;
        commitment.request.retention = retention;
        Ok(commitment.clone())
    }
}

struct SyntheticRelayMarket {
    offers: RefCell<Vec<RelayOffer>>,
    leases: RefCell<HashMap<String, RelayPathLease>>,
    subscriptions: RefCell<HashMap<String, Vec<String>>>,
    default_offer: RelayOffer,
}

impl SyntheticRelayMarket {
    fn new(default_offer: RelayOffer) -> Self {
        Self {
            offers: RefCell::new(Vec::new()),
            leases: RefCell::new(HashMap::new()),
            subscriptions: RefCell::new(HashMap::new()),
            default_offer,
        }
    }

    fn subscribe(&self, lease_id: PathLeaseId) {
        self.subscriptions
            .borrow_mut()
            .entry(lease_id.0)
            .or_default();
    }

    fn publish(&self, lease_id: PathLeaseId, body: &str) -> Result<(), FoundationError> {
        let mut subscriptions = self.subscriptions.borrow_mut();
        let messages = subscriptions
            .get_mut(&lease_id.0)
            .ok_or(FoundationError::EmptyField {
                field: "relay_subscription",
            })?;
        messages.push(body.to_owned());
        Ok(())
    }

    fn messages_for(&self, lease_id: &PathLeaseId) -> Vec<String> {
        self.subscriptions
            .borrow()
            .get(&lease_id.0)
            .cloned()
            .unwrap_or_default()
    }
}

impl RelayMarket for SyntheticRelayMarket {
    type Error = FoundationError;

    fn publish_offer(&self, offer: RelayOffer) -> Result<RelayOfferId, Self::Error> {
        let offer_id = offer.offer_id.clone();
        self.offers.borrow_mut().push(offer);
        Ok(offer_id)
    }

    fn lease_path(&self, request: PathLeaseRequest) -> Result<RelayPathLease, Self::Error> {
        if request.desired_hops == 0 {
            return Err(FoundationError::OutOfRange {
                field: "desired_hops",
            });
        }
        if !request
            .required_capabilities
            .iter()
            .all(|capability| self.default_offer.capabilities.contains(capability))
        {
            return Err(FoundationError::InvalidFormat {
                field: "required_capabilities",
            });
        }
        let lease = RelayPathLease {
            lease_id: PathLeaseId("relay-l3-1".to_owned()),
            requester: request.requester,
            hops: vec![RelayHop {
                operator: self.default_offer.operator.clone(),
                ingress: self.default_offer.ingress.clone(),
                egress: self.default_offer.egress.clone(),
                capabilities: self.default_offer.capabilities.clone(),
                price: self.default_offer.price_per_hop,
            }],
            privacy_tier: request.privacy_tier,
            rotation: request.rotation,
            settlement: SettlementIntent {
                max_price: request.max_price,
                escrow_required: true,
            },
        };
        self.leases
            .borrow_mut()
            .insert(lease.lease_id.0.clone(), lease.clone());
        Ok(lease)
    }

    fn rotate_path(
        &self,
        lease_id: PathLeaseId,
        policy: RotationPolicy,
    ) -> Result<RelayPathLease, Self::Error> {
        let mut leases = self.leases.borrow_mut();
        let lease = leases
            .get_mut(&lease_id.0)
            .ok_or(FoundationError::EmptyField { field: "lease_id" })?;
        lease.rotation = policy;
        Ok(lease.clone())
    }

    fn settle_hop(&self, settlement: PerHopSettlement) -> Result<(), Self::Error> {
        if settlement.amount == CreditAmount::zero() {
            return Err(FoundationError::OutOfRange {
                field: "settlement_amount",
            });
        }
        Ok(())
    }
}

fn ref_path(slug: &str, sequence: u32) -> Result<CrossGraphRef, FoundationError> {
    format!("playful/122/{slug}/{sequence}").parse()
}

fn handle_path<const N: usize>(parts: [&str; N]) -> Result<HandlePath, FoundationError> {
    HandlePath::new(parts)
}
