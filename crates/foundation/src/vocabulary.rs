use serde::{Deserialize, Serialize};

use crate::namespace::{validate_slug, NamespaceId};
use crate::refs::CrossGraphRef;
use crate::FoundationError;

pub const MAX_VOCABULARY_LABEL_BYTES: usize = 120;
pub const MAX_VOCABULARY_DESCRIPTION_BYTES: usize = 2_000;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VocabularyNamespace {
    pub namespace: NamespaceId,
    pub name: String,
}

impl VocabularyNamespace {
    pub fn new(namespace: NamespaceId, name: impl Into<String>) -> Result<Self, FoundationError> {
        let name = name.into();
        validate_slug("vocabulary_namespace", &name)?;
        Ok(Self { namespace, name })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VocabularyKey(String);

impl VocabularyKey {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_slug("vocabulary_key", &value)?;
        if is_reserved_primitive_name(&value)
            || canonical_ops::is_foundation_registered_name(&value)
        {
            return Err(FoundationError::ReservedName {
                field: "vocabulary_key",
            });
        }
        Ok(Self(value))
    }

    pub fn system(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_slug("vocabulary_key", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyTermRef {
    pub vocabulary: VocabularyNamespace,
    pub key: VocabularyKey,
    pub definition_ref: CrossGraphRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VocabularyEntry {
    term: VocabularyTermRef,
    label: String,
    description: Option<String>,
}

impl VocabularyEntry {
    pub fn new(
        term: VocabularyTermRef,
        label: impl Into<String>,
        description: Option<impl Into<String>>,
    ) -> Result<Self, FoundationError> {
        let label = label.into();
        if label.trim().is_empty() {
            return Err(FoundationError::EmptyField {
                field: "vocabulary_label",
            });
        }
        if label.len() > MAX_VOCABULARY_LABEL_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "vocabulary_label",
            });
        }
        let description = description.map(Into::into);
        if description
            .as_ref()
            .is_some_and(|value| value.len() > MAX_VOCABULARY_DESCRIPTION_BYTES)
        {
            return Err(FoundationError::OutOfRange {
                field: "vocabulary_description",
            });
        }
        Ok(Self {
            term,
            label,
            description,
        })
    }

    pub fn term(&self) -> &VocabularyTermRef {
        &self.term
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub trait VocabularyResolver {
    type Error;

    fn resolve(&self, term: &VocabularyTermRef) -> Result<VocabularyEntry, Self::Error>;
}

pub mod canonical_ops {
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum CanonicalOpFamily {
        Compiler,
        LlmPrimitive,
        WirePrimitive,
        StepModifier,
        InvocationMode,
        MaintenanceTask,
    }

    pub trait CanonicalOp: sealed::Sealed {
        fn name(&self) -> &'static str;
        fn family(&self) -> CanonicalOpFamily;
    }

    pub trait CompilerOperation: CanonicalOp {}
    pub trait LlmOperation: CanonicalOp {}
    pub trait WireOperation: CanonicalOp {}
    pub trait StepOperation: CanonicalOp {}
    pub trait InvocationOperation: CanonicalOp {}
    pub trait MaintenanceOperation: CanonicalOp {
        fn cron_hint(&self) -> &'static str;
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum CompilerOp {
        Llm,
        Wire,
        Task,
        Game,
    }

    impl CompilerOp {
        pub const ALL: [Self; 4] = [Self::Llm, Self::Wire, Self::Task, Self::Game];
    }

    impl sealed::Sealed for CompilerOp {}
    impl CompilerOperation for CompilerOp {}
    impl CanonicalOp for CompilerOp {
        fn name(&self) -> &'static str {
            match self {
                Self::Llm => "llm",
                Self::Wire => "wire",
                Self::Task => "task",
                Self::Game => "game",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::Compiler
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum LlmPrimitive {
        Ingest,
        Extract,
        Classify,
        Detect,
        Evaluate,
        Compare,
        Verify,
        Calibrate,
        Interrogate,
        Pitch,
        Draft,
        Synthesize,
        Translate,
        Analogize,
        Compress,
        Fuse,
        Review,
        FactCheck,
        Rebut,
        Steelman,
        Strawman,
        Timeline,
        Diff,
        Relate,
        CrossReference,
        Map,
        Price,
        Embody,
        Custom,
    }

    impl LlmPrimitive {
        pub const ALL: [Self; 29] = [
            Self::Ingest,
            Self::Extract,
            Self::Classify,
            Self::Detect,
            Self::Evaluate,
            Self::Compare,
            Self::Verify,
            Self::Calibrate,
            Self::Interrogate,
            Self::Pitch,
            Self::Draft,
            Self::Synthesize,
            Self::Translate,
            Self::Analogize,
            Self::Compress,
            Self::Fuse,
            Self::Review,
            Self::FactCheck,
            Self::Rebut,
            Self::Steelman,
            Self::Strawman,
            Self::Timeline,
            Self::Diff,
            Self::Relate,
            Self::CrossReference,
            Self::Map,
            Self::Price,
            Self::Embody,
            Self::Custom,
        ];
    }

    impl sealed::Sealed for LlmPrimitive {}
    impl LlmOperation for LlmPrimitive {}
    impl CanonicalOp for LlmPrimitive {
        fn name(&self) -> &'static str {
            match self {
                Self::Ingest => "ingest",
                Self::Extract => "extract",
                Self::Classify => "classify",
                Self::Detect => "detect",
                Self::Evaluate => "evaluate",
                Self::Compare => "compare",
                Self::Verify => "verify",
                Self::Calibrate => "calibrate",
                Self::Interrogate => "interrogate",
                Self::Pitch => "pitch",
                Self::Draft => "draft",
                Self::Synthesize => "synthesize",
                Self::Translate => "translate",
                Self::Analogize => "analogize",
                Self::Compress => "compress",
                Self::Fuse => "fuse",
                Self::Review => "review",
                Self::FactCheck => "fact_check",
                Self::Rebut => "rebut",
                Self::Steelman => "steelman",
                Self::Strawman => "strawman",
                Self::Timeline => "timeline",
                Self::Diff => "diff",
                Self::Relate => "relate",
                Self::CrossReference => "cross_reference",
                Self::Map => "map",
                Self::Price => "price",
                Self::Embody => "embody",
                Self::Custom => "custom",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::LlmPrimitive
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum WirePrimitive {
        Query,
        Contribute,
        Access,
        Rate,
        Flag,
        Browse,
        ListCreate,
        ListPin,
        ListSubscribe,
        ListQuery,
        MessageSend,
        MessageBroadcast,
        MarketCreate,
        MarketSeed,
        MarketStake,
        MarketResolve,
        MarketClaim,
        Supersede,
        SubscribeChain,
        CircleCreate,
        CircleInvite,
        CircleAssign,
        MonitorTopic,
        MonitorEntity,
        MonitorChain,
    }

    impl WirePrimitive {
        pub const ALL: [Self; 25] = [
            Self::Query,
            Self::Contribute,
            Self::Access,
            Self::Rate,
            Self::Flag,
            Self::Browse,
            Self::ListCreate,
            Self::ListPin,
            Self::ListSubscribe,
            Self::ListQuery,
            Self::MessageSend,
            Self::MessageBroadcast,
            Self::MarketCreate,
            Self::MarketSeed,
            Self::MarketStake,
            Self::MarketResolve,
            Self::MarketClaim,
            Self::Supersede,
            Self::SubscribeChain,
            Self::CircleCreate,
            Self::CircleInvite,
            Self::CircleAssign,
            Self::MonitorTopic,
            Self::MonitorEntity,
            Self::MonitorChain,
        ];
    }

    impl sealed::Sealed for WirePrimitive {}
    impl WireOperation for WirePrimitive {}
    impl CanonicalOp for WirePrimitive {
        fn name(&self) -> &'static str {
            match self {
                Self::Query => "wire.query",
                Self::Contribute => "wire.contribute",
                Self::Access => "wire.access",
                Self::Rate => "wire.rate",
                Self::Flag => "wire.flag",
                Self::Browse => "wire.browse",
                Self::ListCreate => "wire.list.create",
                Self::ListPin => "wire.list.pin",
                Self::ListSubscribe => "wire.list.subscribe",
                Self::ListQuery => "wire.list.query",
                Self::MessageSend => "wire.message.send",
                Self::MessageBroadcast => "wire.message.broadcast",
                Self::MarketCreate => "wire.market.create",
                Self::MarketSeed => "wire.market.seed",
                Self::MarketStake => "wire.market.stake",
                Self::MarketResolve => "wire.market.resolve",
                Self::MarketClaim => "wire.market.claim",
                Self::Supersede => "wire.supersede",
                Self::SubscribeChain => "wire.subscribe_chain",
                Self::CircleCreate => "wire.circle.create",
                Self::CircleInvite => "wire.circle.invite",
                Self::CircleAssign => "wire.circle.assign",
                Self::MonitorTopic => "wire.monitor.topic",
                Self::MonitorEntity => "wire.monitor.entity",
                Self::MonitorChain => "wire.monitor.chain",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::WirePrimitive
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum StepModifier {
        When,
        ForEach,
        ActionId,
        OnError,
        WaitFor,
        OutputSchema,
        ModelTier,
        GameType,
        Formation,
        Duration,
        EntryFee,
        Bounty,
    }

    impl StepModifier {
        pub const ALL: [Self; 12] = [
            Self::When,
            Self::ForEach,
            Self::ActionId,
            Self::OnError,
            Self::WaitFor,
            Self::OutputSchema,
            Self::ModelTier,
            Self::GameType,
            Self::Formation,
            Self::Duration,
            Self::EntryFee,
            Self::Bounty,
        ];
    }

    impl sealed::Sealed for StepModifier {}
    impl StepOperation for StepModifier {}
    impl CanonicalOp for StepModifier {
        fn name(&self) -> &'static str {
            match self {
                Self::When => "when",
                Self::ForEach => "forEach",
                Self::ActionId => "actionId",
                Self::OnError => "onError",
                Self::WaitFor => "waitFor",
                Self::OutputSchema => "outputSchema",
                Self::ModelTier => "modelTier",
                Self::GameType => "gameType",
                Self::Formation => "formation",
                Self::Duration => "duration",
                Self::EntryFee => "entryFee",
                Self::Bounty => "bounty",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::StepModifier
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum InvocationMode {
        Quote,
        Review,
        Trusted,
    }

    impl InvocationMode {
        pub const ALL: [Self; 3] = [Self::Quote, Self::Review, Self::Trusted];
    }

    impl sealed::Sealed for InvocationMode {}
    impl InvocationOperation for InvocationMode {}
    impl CanonicalOp for InvocationMode {
        fn name(&self) -> &'static str {
            match self {
                Self::Quote => "quote",
                Self::Review => "review",
                Self::Trusted => "trusted",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::InvocationMode
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum MaintenanceTask {
        CallbackSecretRetention,
        CoordinationEventRetention,
        FillIdempotencyRetention,
        ProviderSettlementExpiry,
        PurchaseExpiry,
        MarketSnapshot,
        MarketSnapshotRetention,
        WorkerLivenessCheck,
        Sweep,
        StaleOffers,
        ChronicleRetention,
        ObservationRetention,
    }

    impl MaintenanceTask {
        pub const ALL: [Self; 12] = [
            Self::CallbackSecretRetention,
            Self::CoordinationEventRetention,
            Self::FillIdempotencyRetention,
            Self::ProviderSettlementExpiry,
            Self::PurchaseExpiry,
            Self::MarketSnapshot,
            Self::MarketSnapshotRetention,
            Self::WorkerLivenessCheck,
            Self::Sweep,
            Self::StaleOffers,
            Self::ChronicleRetention,
            Self::ObservationRetention,
        ];
    }

    impl sealed::Sealed for MaintenanceTask {}
    impl MaintenanceOperation for MaintenanceTask {
        fn cron_hint(&self) -> &'static str {
            match self {
                Self::CallbackSecretRetention => "daily",
                Self::CoordinationEventRetention => "daily",
                Self::FillIdempotencyRetention => "hourly",
                Self::ProviderSettlementExpiry => "hourly",
                Self::PurchaseExpiry => "hourly",
                Self::MarketSnapshot => "hourly",
                Self::MarketSnapshotRetention => "daily",
                Self::WorkerLivenessCheck => "every_5_minutes",
                Self::Sweep => "deferred",
                Self::StaleOffers => "deferred",
                Self::ChronicleRetention => "deferred",
                Self::ObservationRetention => "deferred",
            }
        }
    }

    impl CanonicalOp for MaintenanceTask {
        fn name(&self) -> &'static str {
            match self {
                Self::CallbackSecretRetention => "callback_secret_retention",
                Self::CoordinationEventRetention => "coordination_event_retention",
                Self::FillIdempotencyRetention => "fill_idempotency_retention",
                Self::ProviderSettlementExpiry => "provider_settlement_expiry",
                Self::PurchaseExpiry => "purchase_expiry",
                Self::MarketSnapshot => "market_snapshot",
                Self::MarketSnapshotRetention => "market_snapshot_retention",
                Self::WorkerLivenessCheck => "worker_liveness_check",
                Self::Sweep => "sweep",
                Self::StaleOffers => "stale_offers",
                Self::ChronicleRetention => "chronicle_retention",
                Self::ObservationRetention => "observation_retention",
            }
        }

        fn family(&self) -> CanonicalOpFamily {
            CanonicalOpFamily::MaintenanceTask
        }
    }

    pub fn is_foundation_registered_name(value: &str) -> bool {
        CompilerOp::ALL.iter().any(|op| op.name() == value)
            || LlmPrimitive::ALL.iter().any(|op| op.name() == value)
            || WirePrimitive::ALL.iter().any(|op| op.name() == value)
            || StepModifier::ALL.iter().any(|op| op.name() == value)
            || InvocationMode::ALL.iter().any(|op| op.name() == value)
            || MaintenanceTask::ALL.iter().any(|op| op.name() == value)
    }

    mod sealed {
        pub trait Sealed {}
    }
}

pub fn is_reserved_primitive_name(value: &str) -> bool {
    matches!(
        value,
        "compute-market"
            | "storage-market"
            | "relay-market"
            | "transport-cloudflare"
            | "identity"
            | "reputation"
            | "sandbox"
            | "contracts"
            | "foundation"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_vocabulary_cannot_hijack_reserved_primitives() {
        assert_eq!(
            VocabularyKey::new("compute-market"),
            Err(FoundationError::ReservedName {
                field: "vocabulary_key"
            })
        );
        assert_eq!(
            VocabularyKey::new("llm"),
            Err(FoundationError::ReservedName {
                field: "vocabulary_key"
            })
        );
        assert!(VocabularyKey::system("compute-market").is_ok());
    }

    #[test]
    fn canonical_ops_are_foundation_registered() {
        use canonical_ops::{
            CanonicalOp, CompilerOp, MaintenanceOperation, MaintenanceTask, WirePrimitive,
        };

        assert_eq!(CompilerOp::Llm.name(), "llm");
        assert_eq!(WirePrimitive::Contribute.name(), "wire.contribute");
        assert_eq!(
            MaintenanceTask::WorkerLivenessCheck.cron_hint(),
            "every_5_minutes"
        );
        assert!(canonical_ops::is_foundation_registered_name(
            "fill_idempotency_retention"
        ));
    }

    #[test]
    fn vocabulary_entry_caps_user_payloads() {
        let vocabulary =
            VocabularyNamespace::new(NamespaceId::new("playful").unwrap(), "wire-v2").unwrap();
        let term = VocabularyTermRef {
            vocabulary,
            key: VocabularyKey::new("safe-term").unwrap(),
            definition_ref: "playful/123/vocabulary/1".parse().unwrap(),
        };

        assert_eq!(
            VocabularyEntry::new(term.clone(), "", None::<String>),
            Err(FoundationError::EmptyField {
                field: "vocabulary_label"
            })
        );
        assert_eq!(
            VocabularyEntry::new(
                term.clone(),
                "x".repeat(MAX_VOCABULARY_LABEL_BYTES + 1),
                None::<String>
            ),
            Err(FoundationError::OutOfRange {
                field: "vocabulary_label"
            })
        );
        assert!(VocabularyEntry::new(
            term,
            "Safe Term",
            Some("bounded definition anchored by typed CrossGraphRef")
        )
        .is_ok());
    }
}
