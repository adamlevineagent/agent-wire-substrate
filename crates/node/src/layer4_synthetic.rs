use std::collections::HashMap;

use agent_wire_compute_market::{
    ComputeJobEnvelope, DeliveryReceipt, ExecutionAdapter, RetryIntent,
};
use agent_wire_foundation::{
    CreditAmount, CrossGraphRef, FoundationError, GraphKind, GraphSlug, HandleClaim, HandlePath,
    MasterKeyId, MasterPublicKey, MasterSignature, MasterSigner, MasterVerifier, NamespaceId,
    OperatorEmail, PrivateAliasMapping, PrivateGraphRegistration, ReputationRegistryId,
    ReputationSnapshot, SignatureAlgorithm, SignedStatement,
};
use agent_wire_substrate::{compose_substrate_node, NodeConfig, NodeRuntime};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

const BRIDGE_FRICTION_BPS: u128 = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layer4SyntheticReport {
    pub name: String,
    pub graphs: Vec<String>,
    pub subtests: Vec<Layer4Subtest>,
}

impl Layer4SyntheticReport {
    pub fn all_green(&self) -> bool {
        self.subtests
            .iter()
            .all(|subtest| matches!(subtest.status, Layer4Status::Passed))
    }

    pub fn to_markdown(&self) -> String {
        let mut output = String::from("# Layer 4 Two-Graph Bridged Synthetic Validation\n\n");
        output.push_str("Synthetic graphs: ");
        output.push_str(&self.graphs.join(", "));
        output.push_str("\n\n");
        output.push_str("This harness is deterministic and in-memory. It exercises cross-graph identity, mid-slug references, one-shot reputation snapshots, bridge economics, and firewall asymmetry without live LLM calls, live database access, deploy, live smoke, or npm publish.\n\n");
        output.push_str("## Result\n\n");
        output.push_str(if self.all_green() {
            "All 9 Layer 4 sub-tests are green.\n\n"
        } else {
            "One or more Layer 4 sub-tests failed; see reasons below.\n\n"
        });
        output.push_str("## Sub-tests\n\n");
        for subtest in &self.subtests {
            output.push_str("- ");
            output.push_str(match &subtest.status {
                Layer4Status::Passed => "PASS",
                Layer4Status::Failed { .. } => "FAIL",
            });
            output.push_str(" `");
            output.push_str(&subtest.name);
            output.push_str("`: ");
            output.push_str(&subtest.proves);
            if let Layer4Status::Failed { reason } = &subtest.status {
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
pub struct Layer4Subtest {
    pub name: String,
    pub proves: String,
    pub status: Layer4Status,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layer4Status {
    Passed,
    Failed { reason: String },
}

pub fn run_layer4_two_graph_bridged_synthetic() -> Result<Layer4SyntheticReport, FoundationError> {
    let runtime = compose_substrate_node(NodeConfig::demo()?)?;
    let mut harness = TwoGraphHarness::new(runtime)?;
    let mut report = Layer4SyntheticReport {
        name: "wave-2-layer-4-two-graph-bridged-synthetic".to_owned(),
        graphs: vec!["mainnet".to_owned(), "kitty".to_owned()],
        subtests: Vec::new(),
    };

    report.record(
        "identity-claim-master-signature-both-graphs",
        "cross-graph identity primitives operate under one master public key",
        harness.identity_claim_with_master_signature(),
    );
    report.record(
        "reputation-snapshot-at-kitty-onboarding",
        "reputation firewall imports a one-shot mainnet snapshot",
        harness.reputation_snapshot_at_onboarding(),
    );
    report.record(
        "reputation-snapshot-import-is-one-shot",
        "reputation snapshots are signature-bound and cannot be overwritten",
        harness.reputation_snapshot_import_is_one_shot(),
    );
    report.record(
        "mainnet-reputation-evolution-does-not-propagate",
        "mainnet reputation changes do not mutate kitty reputation after snapshot",
        harness.mainnet_reputation_evolution_does_not_propagate(),
    );
    report.record(
        "kitty-release-to-mainnet-surfaces-via-bridge",
        "release_to_mainnet policy publishes a mid-slug contribution through the bridge",
        harness.kitty_release_to_mainnet_surfaces_via_bridge(),
    );
    report.record(
        "kitty-to-mainnet-credit-transfer-incurs-bridge-tax",
        "Sovereign-mode 2 percent bridge friction applies at par",
        harness.credit_transfer_incurs_bridge_tax(),
    );
    report.record(
        "mainnet-compute-job-fulfilled-by-kitty-provider",
        "cross-graph compute-market routing can fulfill a mainnet job through kitty",
        harness.mainnet_compute_job_fulfilled_by_kitty_provider(),
    );
    report.record(
        "mainnet-provider-reputation-does-not-steer-kitty-dispatch",
        "kitty dispatch policy uses kitty-local reputation, not mainnet reputation",
        harness.mainnet_provider_reputation_does_not_steer_kitty_dispatch(),
    );
    report.record(
        "bridge-severed-graphs-independent",
        "Sovereign graphs continue local operation when the bridge is severed",
        harness.bridge_severed_graphs_independent(),
    );

    Ok(report)
}

impl Layer4SyntheticReport {
    fn record(
        &mut self,
        name: impl Into<String>,
        proves: impl Into<String>,
        outcome: Result<Vec<String>, String>,
    ) {
        let (status, details) = match outcome {
            Ok(details) => (Layer4Status::Passed, details),
            Err(reason) => (Layer4Status::Failed { reason }, Vec::new()),
        };
        self.subtests.push(Layer4Subtest {
            name: name.into(),
            proves: proves.into(),
            status,
            details,
        });
    }
}

struct TwoGraphHarness {
    runtime: NodeRuntime,
    identity: SyntheticIdentity,
    signer: SyntheticMasterAuthority,
    mainnet: SyntheticGraph,
    kitty: SyntheticGraph,
    bridge: SyntheticBridge,
}

impl TwoGraphHarness {
    fn new(runtime: NodeRuntime) -> Result<Self, FoundationError> {
        let identity = SyntheticIdentity::new(runtime.config.keys.master_public_key.clone())?;
        let signer = SyntheticMasterAuthority::new(identity.master_key.clone());
        Ok(Self {
            runtime,
            mainnet: SyntheticGraph::new("mainnet", GraphKind::Public)?,
            kitty: SyntheticGraph::new("kitty", GraphKind::Sovereign)?,
            bridge: SyntheticBridge::sovereign_mode(),
            identity,
            signer,
        })
    }

    fn identity_claim_with_master_signature(&mut self) -> Result<Vec<String>, String> {
        let handle_claim = HandleClaim {
            handle: self.identity.public_handle.clone(),
            namespace: self.mainnet.namespace.clone(),
            master_key: self.identity.master_key.clone(),
            operator_email: Some(self.identity.operator_email.clone()),
            issued_at: OffsetDateTime::UNIX_EPOCH,
        };
        let signed_claim = self
            .signer
            .signed_statement(handle_claim)
            .map_err(|error| error.to_string())?;
        self.signer
            .verify(
                &self.identity.master_key,
                &signed_claim.statement,
                &signed_claim.signature,
            )
            .map_err(|error| error.to_string())?;
        self.mainnet.handle_claims.push(signed_claim);

        let graph_registration = PrivateGraphRegistration {
            namespace: self.mainnet.namespace.clone(),
            graph_slug: self.kitty.slug.clone(),
            graph_kind: GraphKind::Sovereign,
            master_key: self.identity.master_key.clone(),
            reputation_registry: Some(self.kitty.reputation_registry.clone()),
            registered_at: OffsetDateTime::UNIX_EPOCH,
        };
        if graph_registration.graph_kind != GraphKind::Sovereign {
            return Err("kitty registration is not sovereign".to_owned());
        }
        if self.kitty.graph_kind != GraphKind::Sovereign {
            return Err("kitty graph fixture is not sovereign".to_owned());
        }
        self.mainnet.graph_registrations.push(graph_registration);

        let alias_mapping = PrivateAliasMapping {
            private_alias: self.identity.private_alias.clone(),
            public_handle: Some(self.identity.public_handle.clone()),
            namespace: self.kitty.namespace.clone(),
            signed_ref: ref_path("kitty", "identity", 1).map_err(|error| error.to_string())?,
        };
        let signed_alias = self
            .signer
            .signed_statement(alias_mapping)
            .map_err(|error| error.to_string())?;
        self.signer
            .verify(
                &self.identity.master_key,
                &signed_alias.statement,
                &signed_alias.signature,
            )
            .map_err(|error| error.to_string())?;
        self.kitty.alias_mappings.push(signed_alias);

        let mid_slug_ref = ref_path("playful", "kitty", 7).map_err(|error| error.to_string())?;
        if mid_slug_ref.slug.as_deref() != Some("kitty") {
            return Err("mid-slug reference did not preserve kitty slug".to_owned());
        }

        Ok(vec![
            format!(
                "mainnet HandleClaim verified for {} with key {}",
                self.identity.public_handle,
                self.identity.master_key.key_id.as_str()
            ),
            format!(
                "kitty PrivateAliasMapping {} -> {} verified",
                self.identity.private_alias, self.identity.public_handle
            ),
            format!(
                "mainnet PrivateGraphRegistration owns slug {}",
                self.kitty.slug.as_str()
            ),
            format!("mid-slug reference parsed as {}", mid_slug_ref),
        ])
    }

    fn reputation_snapshot_at_onboarding(&mut self) -> Result<Vec<String>, String> {
        self.mainnet.set_reputation(&self.identity.master_key, 750);
        let snapshot = self
            .mainnet
            .export_reputation_snapshot(
                &self.identity.master_key,
                ref_path("playful", "mainnet", 1).map_err(|error| error.to_string())?,
                &self.signer,
            )
            .map_err(|error| error.to_string())?;
        self.kitty
            .import_reputation_snapshot(&self.identity.master_key, snapshot.clone(), &self.signer)
            .map_err(|error| error.to_string())?;
        let imported = self
            .kitty
            .snapshot_score(&self.identity.master_key)
            .ok_or_else(|| "kitty did not import mainnet reputation snapshot".to_owned())?;
        if imported != 750 {
            return Err("kitty imported the wrong snapshot score".to_owned());
        }

        Ok(vec![
            "mainnet reputation exported at 750".to_owned(),
            format!(
                "kitty imported snapshot {} from {}",
                snapshot.primitive.registry.as_str(),
                snapshot.primitive.source_ref
            ),
        ])
    }

    fn reputation_snapshot_import_is_one_shot(&mut self) -> Result<Vec<String>, String> {
        self.mainnet.set_reputation(&self.identity.master_key, 860);
        let duplicate = self
            .mainnet
            .export_reputation_snapshot(
                &self.identity.master_key,
                ref_path("playful", "mainnet", 2).map_err(|error| error.to_string())?,
                &self.signer,
            )
            .map_err(|error| error.to_string())?;
        let duplicate_import = self.kitty.import_reputation_snapshot(
            &self.identity.master_key,
            duplicate,
            &self.signer,
        );
        if !matches!(duplicate_import, Err(ref error) if error == "SnapshotAlreadyImported") {
            return Err("kitty accepted a second snapshot for the same master key".to_owned());
        }
        let imported = self
            .kitty
            .snapshot_score(&self.identity.master_key)
            .ok_or_else(|| "kitty snapshot disappeared after duplicate import".to_owned())?;
        if imported != 750 {
            return Err("duplicate snapshot overwrote the original score".to_owned());
        }

        let mut tampered = self
            .mainnet
            .export_reputation_snapshot(
                &self.identity.master_key,
                ref_path("playful", "mainnet", 3).map_err(|error| error.to_string())?,
                &self.signer,
            )
            .map_err(|error| error.to_string())?;
        tampered.score = 9_999;
        let tampered_import = SyntheticGraph::new("attacker", GraphKind::Sovereign)
            .map_err(|error| error.to_string())?
            .import_reputation_snapshot(&self.identity.master_key, tampered, &self.signer);
        if !matches!(tampered_import, Err(ref error) if error == "SnapshotSignatureInvalid") {
            return Err("tampered snapshot passed signature verification".to_owned());
        }

        Ok(vec![
            "duplicate import returned SnapshotAlreadyImported".to_owned(),
            format!("original kitty snapshot remained {imported}"),
            "tampered snapshot failed statement-bound signature verification".to_owned(),
        ])
    }

    fn mainnet_reputation_evolution_does_not_propagate(&mut self) -> Result<Vec<String>, String> {
        let before = self
            .kitty
            .snapshot_score(&self.identity.master_key)
            .ok_or_else(|| "snapshot must exist before testing reputation evolution".to_owned())?;
        self.mainnet.set_reputation(&self.identity.master_key, 910);
        self.kitty
            .add_local_reputation(&self.identity.master_key, -25);
        let after_snapshot = self
            .kitty
            .snapshot_score(&self.identity.master_key)
            .ok_or_else(|| "kitty snapshot disappeared".to_owned())?;
        let kitty_local = self.kitty.reputation_score(&self.identity.master_key);
        let mainnet_score = self.mainnet.reputation_score(&self.identity.master_key);
        if after_snapshot != before {
            return Err("kitty snapshot mutated after mainnet reputation changed".to_owned());
        }
        if kitty_local == mainnet_score {
            return Err("kitty local reputation collapsed into mainnet reputation".to_owned());
        }

        Ok(vec![
            format!("mainnet reputation evolved to {mainnet_score}"),
            format!("kitty one-shot snapshot remained {after_snapshot}"),
            format!("kitty local reputation independently became {kitty_local}"),
        ])
    }

    fn kitty_release_to_mainnet_surfaces_via_bridge(&mut self) -> Result<Vec<String>, String> {
        let contribution = SyntheticContribution {
            ref_id: ref_path("playful", "kitty", 7).map_err(|error| error.to_string())?,
            author: self.identity.private_alias.clone(),
            body: "synthetic kitty insight".to_owned(),
            release_to_mainnet: true,
            bridged_from: None,
        };
        self.kitty.publish_contribution(contribution.clone());
        let mirrored =
            self.bridge
                .release_to_mainnet(&self.kitty, &mut self.mainnet, &contribution.ref_id)?;
        if !self.mainnet.has_contribution(&contribution.ref_id) {
            return Err("mainnet did not receive released kitty contribution".to_owned());
        }
        if mirrored.bridged_from.as_ref() != Some(&self.kitty.slug) {
            return Err("bridged contribution did not record kitty source slug".to_owned());
        }

        Ok(vec![
            format!("kitty published {}", contribution.ref_id),
            format!("mainnet surfaced released contribution {}", mirrored.ref_id),
        ])
    }

    fn credit_transfer_incurs_bridge_tax(&mut self) -> Result<Vec<String>, String> {
        let transfer =
            self.bridge
                .transfer_credits("kitty", "mainnet", CreditAmount::from_sats(1_000))?;
        if transfer.gross.as_sats() != 1_000
            || transfer.bridge_tax.as_sats() != 20
            || transfer.net.as_sats() != 980
        {
            return Err("bridge transfer did not apply 2 percent tax at par".to_owned());
        }
        self.kitty.debit(CreditAmount::from_sats(1_000))?;
        self.mainnet.credit(transfer.net);

        Ok(vec![
            "kitty debited 1000 sats".to_owned(),
            "bridge tax retained 20 sats".to_owned(),
            format!(
                "{} credited 980 sats at par from {}",
                transfer.to_graph, transfer.from_graph
            ),
        ])
    }

    fn mainnet_compute_job_fulfilled_by_kitty_provider(&mut self) -> Result<Vec<String>, String> {
        let provider = SyntheticProvider {
            handle: self.identity.private_alias.clone(),
            master_key: self.identity.master_key.clone(),
            model_id: "wire-demo-model".to_owned(),
        };
        self.kitty.providers.push(provider.clone());
        self.kitty.set_reputation(&provider.master_key, 40);

        let mut job = self.runtime.markets.compute_job.payload.clone();
        job.job_ref = ref_path("playful", "mainnet", 20).map_err(|error| error.to_string())?;
        job.requester_handle = self.identity.public_handle.clone();
        let receipt = self
            .bridge
            .dispatch_compute_to_kitty(job, &mut self.kitty)
            .map_err(|error| error.to_string())?;
        if receipt.retry_intent != RetryIntent::Never {
            return Err("synthetic compute dispatch was not a terminal success".to_owned());
        }
        if !self.kitty.has_completion(&receipt.result_ref) {
            return Err("kitty did not record the bridged compute completion".to_owned());
        }

        Ok(vec![
            format!("mainnet job {} bridged to kitty provider", receipt.job_ref),
            format!("kitty completion recorded at {}", receipt.result_ref),
        ])
    }

    fn mainnet_provider_reputation_does_not_steer_kitty_dispatch(
        &mut self,
    ) -> Result<Vec<String>, String> {
        let high_mainnet_low_kitty = self.identity.master_key.clone();
        let kitty_local_key = MasterPublicKey::new(
            MasterKeyId::new("kitty-local-provider").map_err(|error| error.to_string())?,
            SignatureAlgorithm::Ed25519,
            vec![11; 32],
        )
        .map_err(|error| error.to_string())?;
        let high_mainnet_provider = SyntheticProvider {
            handle: self.identity.private_alias.clone(),
            master_key: high_mainnet_low_kitty.clone(),
            model_id: "wire-demo-model".to_owned(),
        };
        let kitty_local_provider = SyntheticProvider {
            handle: handle_path(["agent", "playful", "kitty-local-provider"])
                .map_err(|error| error.to_string())?,
            master_key: kitty_local_key.clone(),
            model_id: "wire-demo-model".to_owned(),
        };
        self.mainnet.set_reputation(&high_mainnet_low_kitty, 999);
        self.kitty.set_reputation(&high_mainnet_low_kitty, 5);
        self.mainnet.set_reputation(&kitty_local_key, 1);
        self.kitty.set_reputation(&kitty_local_key, 80);
        self.kitty.providers = vec![high_mainnet_provider, kitty_local_provider.clone()];

        let selected = self
            .kitty
            .select_provider_by_local_reputation("wire-demo-model")
            .ok_or_else(|| "kitty could not select a provider".to_owned())?;
        if selected.handle != kitty_local_provider.handle {
            return Err("kitty dispatch was steered by mainnet provider reputation".to_owned());
        }

        Ok(vec![
            "mainnet score 999 / kitty score 5 provider was not selected".to_owned(),
            format!(
                "kitty selected {} on kitty-local score 80",
                kitty_local_provider.handle
            ),
        ])
    }

    fn bridge_severed_graphs_independent(&mut self) -> Result<Vec<String>, String> {
        let mainnet_before = self.mainnet.reputation_score(&self.identity.master_key);
        self.bridge.connected = false;
        let kitty_local = SyntheticContribution {
            ref_id: ref_path("playful", "kitty", 30).map_err(|error| error.to_string())?,
            author: self.identity.private_alias.clone(),
            body: "private graph local success".to_owned(),
            release_to_mainnet: true,
            bridged_from: None,
        };
        let mainnet_local = SyntheticContribution {
            ref_id: ref_path("playful", "mainnet", 31).map_err(|error| error.to_string())?,
            author: self.identity.public_handle.clone(),
            body: "mainnet local success".to_owned(),
            release_to_mainnet: false,
            bridged_from: None,
        };
        self.kitty.publish_contribution(kitty_local.clone());
        self.mainnet.publish_contribution(mainnet_local.clone());
        self.kitty
            .add_local_reputation(&self.identity.master_key, 25);
        let release_result =
            self.bridge
                .release_to_mainnet(&self.kitty, &mut self.mainnet, &kitty_local.ref_id);
        if release_result.is_ok() {
            return Err("disconnected bridge unexpectedly released kitty contribution".to_owned());
        }
        if !self.kitty.has_contribution(&kitty_local.ref_id)
            || !self.mainnet.has_contribution(&mainnet_local.ref_id)
        {
            return Err(
                "one graph stopped accepting local contributions after bridge sever".to_owned(),
            );
        }
        let mainnet_after = self.mainnet.reputation_score(&self.identity.master_key);
        if mainnet_after != mainnet_before {
            return Err(
                "kitty-side success or bridge failure mutated mainnet reputation".to_owned(),
            );
        }

        Ok(vec![
            "bridge disconnected intentionally".to_owned(),
            format!("kitty continued locally with {}", kitty_local.ref_id),
            format!("mainnet continued locally with {}", mainnet_local.ref_id),
            format!("mainnet reputation stayed {mainnet_after} despite kitty-side changes"),
        ])
    }
}

#[derive(Debug, Clone)]
struct SyntheticIdentity {
    public_handle: HandlePath,
    private_alias: HandlePath,
    master_key: MasterPublicKey,
    operator_email: OperatorEmail,
}

impl SyntheticIdentity {
    fn new(master_key: MasterPublicKey) -> Result<Self, FoundationError> {
        Ok(Self {
            public_handle: HandlePath::new(["agent", "playful", "kramer"])?,
            private_alias: HandlePath::new(["agent", "playful", "scout-3"])?,
            operator_email: OperatorEmail::new("hello@callmeplayful.com")?,
            master_key,
        })
    }
}

struct SyntheticMasterAuthority {
    public_key: MasterPublicKey,
}

impl SyntheticMasterAuthority {
    fn new(public_key: MasterPublicKey) -> Self {
        Self { public_key }
    }

    fn signed_statement<T: Serialize>(
        &self,
        statement: T,
    ) -> Result<SignedStatement<T>, FoundationError> {
        let signature = self.sign(&statement)?;
        Ok(SignedStatement {
            statement,
            signed_at: OffsetDateTime::UNIX_EPOCH,
            signature,
        })
    }
}

impl MasterSigner for SyntheticMasterAuthority {
    type Error = FoundationError;

    fn sign<T: Serialize>(&self, statement: &T) -> Result<MasterSignature, Self::Error> {
        Ok(MasterSignature {
            key_id: self.public_key.key_id.clone(),
            algorithm: self.public_key.algorithm,
            bytes: statement_signature_bytes(&self.public_key, statement)?,
        })
    }
}

impl MasterVerifier for SyntheticMasterAuthority {
    type Error = FoundationError;

    fn verify<T: Serialize>(
        &self,
        public_key: &MasterPublicKey,
        statement: &T,
        signature: &MasterSignature,
    ) -> Result<(), Self::Error> {
        if public_key.key_id != signature.key_id || public_key.algorithm != signature.algorithm {
            return Err(FoundationError::InvalidFormat {
                field: "master_signature",
            });
        }
        if signature.bytes.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "master_signature",
            });
        }
        if signature.bytes != statement_signature_bytes(public_key, statement)? {
            return Err(FoundationError::InvalidFormat {
                field: "master_signature",
            });
        }
        Ok(())
    }
}

fn statement_signature_bytes<T: Serialize>(
    public_key: &MasterPublicKey,
    statement: &T,
) -> Result<Vec<u8>, FoundationError> {
    let mut bytes = serde_json::to_vec(statement).map_err(|_| FoundationError::InvalidFormat {
        field: "signed_statement",
    })?;
    bytes.extend_from_slice(public_key.key_id.as_str().as_bytes());
    bytes.push(match public_key.algorithm {
        SignatureAlgorithm::Ed25519 => 1,
        SignatureAlgorithm::Secp256k1 => 2,
    });
    bytes.extend_from_slice(&public_key.bytes);

    let mut acc = 0xA5A5_5A5A_D3C3_B2B2_u64;
    for byte in bytes {
        acc = acc.rotate_left(5) ^ u64::from(byte);
        acc = acc.wrapping_mul(0x100_0000_01B3);
    }

    let mut signature = Vec::with_capacity(64);
    for round in 0..8_u64 {
        signature.extend_from_slice(&acc.wrapping_add(round).to_le_bytes());
    }
    Ok(signature)
}

#[derive(Debug, Clone)]
struct SyntheticGraph {
    slug: GraphSlug,
    namespace: NamespaceId,
    graph_kind: GraphKind,
    reputation_registry: ReputationRegistryId,
    handle_claims: Vec<SignedStatement<HandleClaim>>,
    graph_registrations: Vec<PrivateGraphRegistration>,
    alias_mappings: Vec<SignedStatement<PrivateAliasMapping>>,
    reputation: HashMap<String, i64>,
    imported_snapshots: HashMap<String, SyntheticReputationSnapshot>,
    contributions: Vec<SyntheticContribution>,
    providers: Vec<SyntheticProvider>,
    completions: Vec<DeliveryReceipt>,
    credit_balance: CreditAmount,
}

impl SyntheticGraph {
    fn new(slug: &str, graph_kind: GraphKind) -> Result<Self, FoundationError> {
        Ok(Self {
            slug: GraphSlug::new(slug)?,
            namespace: NamespaceId::new("playful")?,
            graph_kind,
            reputation_registry: ReputationRegistryId::new(format!("{slug}_rep"))?,
            handle_claims: Vec::new(),
            graph_registrations: Vec::new(),
            alias_mappings: Vec::new(),
            reputation: HashMap::new(),
            imported_snapshots: HashMap::new(),
            contributions: Vec::new(),
            providers: Vec::new(),
            completions: Vec::new(),
            credit_balance: CreditAmount::from_sats(10_000),
        })
    }

    fn set_reputation(&mut self, key: &MasterPublicKey, score: i64) {
        self.reputation
            .insert(key.key_id.as_str().to_owned(), score);
    }

    fn add_local_reputation(&mut self, key: &MasterPublicKey, delta: i64) {
        let current = self.reputation_score(key);
        self.set_reputation(key, current + delta);
    }

    fn reputation_score(&self, key: &MasterPublicKey) -> i64 {
        self.reputation
            .get(key.key_id.as_str())
            .copied()
            .unwrap_or_default()
    }

    fn export_reputation_snapshot(
        &self,
        key: &MasterPublicKey,
        source_ref: CrossGraphRef,
        signer: &impl MasterSigner<Error = FoundationError>,
    ) -> Result<SyntheticReputationSnapshot, FoundationError> {
        let statement = ReputationSnapshotStatement {
            namespace: self.namespace.clone(),
            registry: self.reputation_registry.clone(),
            source_ref: source_ref.clone(),
            exported_at: OffsetDateTime::UNIX_EPOCH,
            score: self.reputation_score(key),
        };
        let signature = signer.sign(&statement)?;
        Ok(SyntheticReputationSnapshot {
            primitive: ReputationSnapshot {
                namespace: self.namespace.clone(),
                registry: self.reputation_registry.clone(),
                source_ref,
                exported_at: OffsetDateTime::UNIX_EPOCH,
                signature,
            },
            score: statement.score,
        })
    }

    fn import_reputation_snapshot(
        &mut self,
        key: &MasterPublicKey,
        snapshot: SyntheticReputationSnapshot,
        verifier: &impl MasterVerifier<Error = FoundationError>,
    ) -> Result<(), String> {
        if self.imported_snapshots.contains_key(key.key_id.as_str()) {
            return Err("SnapshotAlreadyImported".to_owned());
        }
        verifier
            .verify(key, &snapshot.statement(), &snapshot.primitive.signature)
            .map_err(|_| "SnapshotSignatureInvalid".to_owned())?;
        self.imported_snapshots
            .insert(key.key_id.as_str().to_owned(), snapshot);
        Ok(())
    }

    fn snapshot_score(&self, key: &MasterPublicKey) -> Option<i64> {
        self.imported_snapshots
            .get(key.key_id.as_str())
            .map(|snapshot| snapshot.score)
    }

    fn publish_contribution(&mut self, contribution: SyntheticContribution) {
        self.contributions.push(contribution);
    }

    fn has_contribution(&self, ref_id: &CrossGraphRef) -> bool {
        self.contributions
            .iter()
            .any(|contribution| &contribution.ref_id == ref_id)
    }

    fn has_completion(&self, ref_id: &CrossGraphRef) -> bool {
        self.completions
            .iter()
            .any(|completion| &completion.result_ref == ref_id)
    }

    fn select_provider_by_local_reputation(&self, model_id: &str) -> Option<SyntheticProvider> {
        self.providers
            .iter()
            .filter(|provider| provider.model_id == model_id)
            .max_by_key(|provider| self.reputation_score(&provider.master_key))
            .cloned()
    }

    fn credit(&mut self, amount: CreditAmount) {
        self.credit_balance = self
            .credit_balance
            .checked_add(amount)
            .unwrap_or(self.credit_balance);
    }

    fn debit(&mut self, amount: CreditAmount) -> Result<(), String> {
        self.credit_balance = self
            .credit_balance
            .checked_sub(amount)
            .ok_or_else(|| "insufficient graph balance".to_owned())?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SyntheticReputationSnapshot {
    primitive: ReputationSnapshot,
    score: i64,
}

impl SyntheticReputationSnapshot {
    fn statement(&self) -> ReputationSnapshotStatement {
        ReputationSnapshotStatement {
            namespace: self.primitive.namespace.clone(),
            registry: self.primitive.registry.clone(),
            source_ref: self.primitive.source_ref.clone(),
            exported_at: self.primitive.exported_at,
            score: self.score,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct ReputationSnapshotStatement {
    namespace: NamespaceId,
    registry: ReputationRegistryId,
    source_ref: CrossGraphRef,
    exported_at: OffsetDateTime,
    score: i64,
}

#[derive(Debug, Clone)]
struct SyntheticContribution {
    ref_id: CrossGraphRef,
    author: HandlePath,
    body: String,
    release_to_mainnet: bool,
    bridged_from: Option<GraphSlug>,
}

#[derive(Debug, Clone)]
struct SyntheticProvider {
    handle: HandlePath,
    master_key: MasterPublicKey,
    model_id: String,
}

struct SyntheticBridge {
    connected: bool,
    friction_bps: u128,
    exchange_rate_numerator: u128,
    exchange_rate_denominator: u128,
}

impl SyntheticBridge {
    fn sovereign_mode() -> Self {
        Self {
            connected: true,
            friction_bps: BRIDGE_FRICTION_BPS,
            exchange_rate_numerator: 1,
            exchange_rate_denominator: 1,
        }
    }

    fn release_to_mainnet(
        &self,
        source_graph: &SyntheticGraph,
        mainnet: &mut SyntheticGraph,
        ref_id: &CrossGraphRef,
    ) -> Result<SyntheticContribution, String> {
        if !self.connected {
            return Err("bridge is disconnected".to_owned());
        }
        let contribution = source_graph
            .contributions
            .iter()
            .find(|contribution| &contribution.ref_id == ref_id)
            .cloned()
            .ok_or_else(|| "source contribution not found".to_owned())?;
        if !contribution.release_to_mainnet {
            return Err("contribution is not releasable to mainnet".to_owned());
        }
        if contribution.body.trim().is_empty() {
            return Err("released contribution body is empty".to_owned());
        }
        if contribution.author.parts().is_empty() {
            return Err("released contribution author is empty".to_owned());
        }
        if contribution.ref_id.slug.as_deref() != Some(source_graph.slug.as_str()) {
            return Err("released contribution does not use the source graph mid-slug".to_owned());
        }
        let mut mirrored = contribution;
        mirrored.bridged_from = Some(source_graph.slug.clone());
        mainnet.publish_contribution(mirrored.clone());
        Ok(mirrored)
    }

    fn transfer_credits(
        &self,
        from_graph: &str,
        to_graph: &str,
        amount: CreditAmount,
    ) -> Result<BridgeTransfer, String> {
        if !self.connected {
            return Err("bridge is disconnected".to_owned());
        }
        let converted =
            amount.as_sats() * self.exchange_rate_numerator / self.exchange_rate_denominator;
        let bridge_tax = converted * self.friction_bps / 10_000;
        let net = converted
            .checked_sub(bridge_tax)
            .ok_or_else(|| "bridge tax exceeded converted transfer".to_owned())?;
        Ok(BridgeTransfer {
            from_graph: from_graph.to_owned(),
            to_graph: to_graph.to_owned(),
            gross: amount,
            bridge_tax: CreditAmount::from_sats(bridge_tax),
            net: CreditAmount::from_sats(net),
        })
    }

    fn dispatch_compute_to_kitty(
        &self,
        job: ComputeJobEnvelope,
        kitty: &mut SyntheticGraph,
    ) -> Result<DeliveryReceipt, String> {
        if !self.connected {
            return Err("bridge is disconnected".to_owned());
        }
        let provider = kitty
            .select_provider_by_local_reputation(&job.invocation.model_id)
            .ok_or_else(|| "no kitty provider can fulfill model".to_owned())?;
        let adapter = CrossGraphEchoAdapter {
            provider: provider.handle,
        };
        let receipt = adapter.invoke(&job).map_err(|error| error.to_string())?;
        kitty.completions.push(receipt.clone());
        Ok(receipt)
    }
}

#[derive(Debug, Clone)]
struct BridgeTransfer {
    from_graph: String,
    to_graph: String,
    gross: CreditAmount,
    bridge_tax: CreditAmount,
    net: CreditAmount,
}

struct CrossGraphEchoAdapter {
    provider: HandlePath,
}

impl ExecutionAdapter for CrossGraphEchoAdapter {
    type Error = FoundationError;
    type Output = DeliveryReceipt;

    fn invoke(&self, job: &ComputeJobEnvelope) -> Result<Self::Output, Self::Error> {
        let _provider_handle = self.provider.to_string();
        Ok(DeliveryReceipt {
            job_ref: job.job_ref.clone(),
            delivered_to: None,
            result_ref: ref_path("playful", "kitty", 21)?,
            charged: CreditAmount::from_sats(64),
            retry_intent: RetryIntent::Never,
        })
    }
}

fn ref_path(handle: &str, slug: &str, sequence: u32) -> Result<CrossGraphRef, FoundationError> {
    format!("{handle}/123/{slug}/{sequence}").parse()
}

fn handle_path<const N: usize>(parts: [&str; N]) -> Result<HandlePath, FoundationError> {
    HandlePath::new(parts)
}
