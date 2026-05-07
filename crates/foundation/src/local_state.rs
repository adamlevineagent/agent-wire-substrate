use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::FoundationError;

macro_rules! impl_string_newtype_serde {
    ($name:ident, $field:literal) => {
        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.0.serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let value = String::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalStateRecordKind {
    IdentitySnapshot,
    MainnetAuthCredential,
    TunnelState,
    ComputeOffer,
    ComputeQuote,
    ComputePurchase,
    ComputeFulfillment,
    ComputeSettlement,
    ContributionCacheEntry,
    VocabularyRecord,
    MirrorRecord,
    SchedulerCursor,
}

impl LocalStateRecordKind {
    pub fn directory_name(&self) -> &'static str {
        match self {
            Self::IdentitySnapshot => "identity_snapshot",
            Self::MainnetAuthCredential => "mainnet_auth_credential",
            Self::TunnelState => "tunnel_state",
            Self::ComputeOffer => "compute_offer",
            Self::ComputeQuote => "compute_quote",
            Self::ComputePurchase => "compute_purchase",
            Self::ComputeFulfillment => "compute_fulfillment",
            Self::ComputeSettlement => "compute_settlement",
            Self::ContributionCacheEntry => "contribution_cache_entry",
            Self::VocabularyRecord => "vocabulary_record",
            Self::MirrorRecord => "mirror_record",
            Self::SchedulerCursor => "scheduler_cursor",
        }
    }

    pub fn secret_policy(&self) -> SecretPolicy {
        match self {
            Self::MainnetAuthCredential => SecretPolicy::SecretRefCapable,
            Self::TunnelState => SecretPolicy::PrivateFile,
            _ => SecretPolicy::Public,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalStateSchema {
    IdentitySnapshotV1,
    MainnetAuthCredentialV1,
    TunnelStateV1,
    ComputeOfferV1,
    ComputeQuoteV1,
    ComputePurchaseV1,
    ComputeFulfillmentV1,
    ComputeSettlementV1,
    ContributionCacheEntryV1,
    VocabularyRecordV1,
    MirrorRecordV1,
    SchedulerCursorV1,
}

impl LocalStateSchema {
    pub fn allowed_for_kind(kind: LocalStateRecordKind) -> &'static [Self] {
        match kind {
            LocalStateRecordKind::IdentitySnapshot => &[Self::IdentitySnapshotV1],
            LocalStateRecordKind::MainnetAuthCredential => &[Self::MainnetAuthCredentialV1],
            LocalStateRecordKind::TunnelState => &[Self::TunnelStateV1],
            LocalStateRecordKind::ComputeOffer => &[Self::ComputeOfferV1],
            LocalStateRecordKind::ComputeQuote => &[Self::ComputeQuoteV1],
            LocalStateRecordKind::ComputePurchase => &[Self::ComputePurchaseV1],
            LocalStateRecordKind::ComputeFulfillment => &[Self::ComputeFulfillmentV1],
            LocalStateRecordKind::ComputeSettlement => &[Self::ComputeSettlementV1],
            LocalStateRecordKind::ContributionCacheEntry => &[Self::ContributionCacheEntryV1],
            LocalStateRecordKind::VocabularyRecord => &[Self::VocabularyRecordV1],
            LocalStateRecordKind::MirrorRecord => &[Self::MirrorRecordV1],
            LocalStateRecordKind::SchedulerCursor => &[Self::SchedulerCursorV1],
        }
    }

    pub fn is_allowed_for_kind(self, kind: LocalStateRecordKind) -> bool {
        Self::allowed_for_kind(kind).contains(&self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretPolicy {
    Public,
    PrivateFile,
    SecretRefCapable,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SecretMaterial {
    Inline(SecretString),
    KeychainRef(KeyHandle),
}

#[derive(Clone, PartialEq, Eq)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(FoundationError::EmptyField {
                field: "secret_material",
            });
        }
        Ok(Self(value))
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString(<redacted>)")
    }
}

impl Serialize for SecretString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct KeyHandle(String);

impl KeyHandle {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_bounded_string("key_handle", &value).map(Self)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LocalStateRecordId(String);

impl LocalStateRecordId {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_record_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TopicTag(String);

impl TopicTag {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_topic_tag(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct WireHandle(String);

impl WireHandle {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_wire_handle(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl_string_newtype_serde!(KeyHandle, "key_handle");
impl_string_newtype_serde!(LocalStateRecordId, "local_state_record_id");
impl_string_newtype_serde!(TopicTag, "topic_tag");
impl_string_newtype_serde!(WireHandle, "wire_handle");

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalStateRef {
    pub kind: LocalStateRecordKind,
    pub id: LocalStateRecordId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handle: Option<WireHandle>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalStateSubjectKind {
    WireHandle,
    NodeId,
    ContributionRef,
    CrossGraphRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalStateSubject {
    pub kind: LocalStateSubjectKind,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalStateSensitivity {
    Public,
    Private,
    SecretRef,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalStateFrontmatter {
    pub schema_version: u32,
    pub record_kind: LocalStateRecordKind,
    pub record_id: LocalStateRecordId,
    pub state_schema: LocalStateSchema,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<LocalStateRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<TopicTag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<LocalStateRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subjects: Vec<LocalStateSubject>,
    pub sensitivity: LocalStateSensitivity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_handle: Option<WireHandle>,
}

impl LocalStateFrontmatter {
    pub fn validate(&self) -> Result<(), LocalStateDocError> {
        if self.schema_version != 1 {
            return Err(LocalStateDocError::UnsupportedSchemaVersion {
                version: self.schema_version,
            });
        }
        if !self.state_schema.is_allowed_for_kind(self.record_kind) {
            return Err(LocalStateDocError::KindSchemaMismatch {
                kind: self.record_kind,
                schema: self.state_schema,
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalStateDocument<T> {
    pub frontmatter: LocalStateFrontmatter,
    pub payload: T,
    #[serde(default)]
    pub prose: String,
}

impl<T> LocalStateDocument<T> {
    pub fn new(
        frontmatter: LocalStateFrontmatter,
        payload: T,
        prose: impl Into<String>,
    ) -> Result<Self, LocalStateDocError> {
        frontmatter.validate()?;
        Ok(Self {
            frontmatter,
            payload,
            prose: prose.into(),
        })
    }
}

#[derive(Debug, Error)]
pub enum LocalStateDocError {
    #[error(transparent)]
    Foundation(#[from] FoundationError),
    #[error("local-state doc has invalid yaml: {0}")]
    Yaml(String),
    #[error("local-state doc I/O failed: {0}")]
    Io(String),
    #[error("local-state doc must start with a frontmatter YAML document")]
    MissingFrontmatter,
    #[error("local-state doc is missing an authoritative payload YAML document")]
    MissingPayload,
    #[error("local-state doc schema version {version} is unsupported")]
    UnsupportedSchemaVersion { version: u32 },
    #[error("local-state schema {schema:?} is not allowed for kind {kind:?}")]
    KindSchemaMismatch {
        kind: LocalStateRecordKind,
        schema: LocalStateSchema,
    },
    #[error("private local-state docs require Unix private-file mode or a secure backend")]
    PrivateFileUnsupported,
}

impl From<io::Error> for LocalStateDocError {
    fn from(error: io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

pub trait Clock: Clone {
    fn now_rfc3339(&self) -> String;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
    }
}

#[derive(Clone, Debug)]
pub struct WireNativeDocCodec<C = SystemClock> {
    clock: C,
}

impl Default for WireNativeDocCodec<SystemClock> {
    fn default() -> Self {
        Self::new()
    }
}

impl WireNativeDocCodec<SystemClock> {
    pub fn new() -> Self {
        Self { clock: SystemClock }
    }
}

impl<C: Clock> WireNativeDocCodec<C> {
    pub fn with_clock(clock: C) -> Self {
        Self { clock }
    }

    pub fn frontmatter(
        &self,
        kind: LocalStateRecordKind,
        id: LocalStateRecordId,
        schema: LocalStateSchema,
    ) -> Result<LocalStateFrontmatter, LocalStateDocError> {
        let now = self.clock.now_rfc3339();
        let frontmatter = LocalStateFrontmatter {
            schema_version: 1,
            record_kind: kind,
            record_id: id,
            state_schema: schema,
            created_at: now.clone(),
            updated_at: now,
            supersedes: Vec::new(),
            topics: Vec::new(),
            derived_from: Vec::new(),
            subjects: Vec::new(),
            sensitivity: match kind.secret_policy() {
                SecretPolicy::Public => LocalStateSensitivity::Public,
                SecretPolicy::PrivateFile => LocalStateSensitivity::Private,
                SecretPolicy::SecretRefCapable => LocalStateSensitivity::SecretRef,
            },
            source_commit: None,
            source_handle: None,
        };
        frontmatter.validate()?;
        Ok(frontmatter)
    }

    pub fn document<T>(
        &self,
        kind: LocalStateRecordKind,
        id: LocalStateRecordId,
        schema: LocalStateSchema,
        payload: T,
        prose: impl Into<String>,
    ) -> Result<LocalStateDocument<T>, LocalStateDocError> {
        LocalStateDocument::new(self.frontmatter(kind, id, schema)?, payload, prose)
    }

    pub fn parse<T: DeserializeOwned>(
        &self,
        text: &str,
    ) -> Result<LocalStateDocument<T>, LocalStateDocError> {
        let split = split_multi_doc(text)?;
        let frontmatter: LocalStateFrontmatter = serde_yaml::from_str(split.frontmatter)
            .map_err(|error| LocalStateDocError::Yaml(error.to_string()))?;
        frontmatter.validate()?;
        let payload = serde_yaml::from_str(split.payload)
            .map_err(|error| LocalStateDocError::Yaml(error.to_string()))?;
        Ok(LocalStateDocument {
            frontmatter,
            payload,
            prose: split.prose.to_owned(),
        })
    }

    pub fn render<T: Serialize>(
        &self,
        document: &LocalStateDocument<T>,
    ) -> Result<String, LocalStateDocError> {
        document.frontmatter.validate()?;
        let frontmatter = yaml_without_document_marker(&document.frontmatter)?;
        let payload = yaml_without_document_marker(&document.payload)?;
        Ok(format!(
            "---\n{}---\n{}---\n{}",
            frontmatter, payload, document.prose
        ))
    }

    pub fn read<T: DeserializeOwned>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<LocalStateDocument<T>, LocalStateDocError> {
        let text = fs::read_to_string(path)?;
        self.parse(&text)
    }

    pub fn write<T: Serialize>(
        &self,
        path: impl AsRef<Path>,
        document: &LocalStateDocument<T>,
    ) -> Result<(), LocalStateDocError> {
        let rendered = self.render(document)?;
        write_atomically(
            path.as_ref(),
            rendered.as_bytes(),
            document.frontmatter.record_kind.secret_policy(),
        )
    }

    pub fn record_path(
        &self,
        state_dir: impl AsRef<Path>,
        kind: LocalStateRecordKind,
        id: &LocalStateRecordId,
    ) -> PathBuf {
        state_dir
            .as_ref()
            .join(kind.directory_name())
            .join(format!("{}.md", id.as_str()))
    }
}

struct SplitDoc<'a> {
    frontmatter: &'a str,
    payload: &'a str,
    prose: &'a str,
}

fn split_multi_doc(text: &str) -> Result<SplitDoc<'_>, LocalStateDocError> {
    if !text.starts_with("---\n") {
        return Err(LocalStateDocError::MissingFrontmatter);
    }
    let frontmatter_start = 4;
    let payload_boundary =
        find_boundary(text, frontmatter_start).ok_or(LocalStateDocError::MissingPayload)?;
    let payload_start = payload_boundary + 4;
    let prose_boundary =
        find_boundary(text, payload_start).ok_or(LocalStateDocError::MissingPayload)?;
    let prose_start = prose_boundary + 4;
    Ok(SplitDoc {
        frontmatter: &text[frontmatter_start..payload_boundary],
        payload: &text[payload_start..prose_boundary],
        prose: &text[prose_start..],
    })
}

fn find_boundary(text: &str, start: usize) -> Option<usize> {
    let mut index = start;
    while index < text.len() {
        let next_newline = text[index..]
            .find('\n')
            .map(|offset| index + offset)
            .unwrap_or(text.len());
        if &text[index..next_newline] == "---" {
            return Some(index);
        }
        if next_newline == text.len() {
            return None;
        }
        index = next_newline + 1;
    }
    None
}

fn yaml_without_document_marker<T: Serialize>(value: &T) -> Result<String, LocalStateDocError> {
    let mut yaml = serde_yaml::to_string(value)
        .map_err(|error| LocalStateDocError::Yaml(error.to_string()))?;
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_owned();
    }
    if !yaml.ends_with('\n') {
        yaml.push('\n');
    }
    Ok(yaml)
}

fn write_atomically(
    path: &Path,
    bytes: &[u8],
    secret_policy: SecretPolicy,
) -> Result<(), LocalStateDocError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("md.tmp");
    write_file(&tmp, bytes, secret_policy)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(unix)]
fn write_file(
    path: &Path,
    bytes: &[u8],
    secret_policy: SecretPolicy,
) -> Result<(), LocalStateDocError> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let mode = match secret_policy {
        SecretPolicy::Public => 0o644,
        SecretPolicy::PrivateFile | SecretPolicy::SecretRefCapable => 0o600,
    };
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(mode)
        .open(path)?;
    file.write_all(bytes)?;
    file.flush()?;
    Ok(())
}

#[cfg(not(unix))]
fn write_file(
    path: &Path,
    bytes: &[u8],
    secret_policy: SecretPolicy,
) -> Result<(), LocalStateDocError> {
    match secret_policy {
        SecretPolicy::Public => fs::write(path, bytes).map_err(LocalStateDocError::from),
        SecretPolicy::SecretRefCapable => fs::write(path, bytes).map_err(LocalStateDocError::from),
        SecretPolicy::PrivateFile => Err(LocalStateDocError::PrivateFileUnsupported),
    }
}

fn validate_record_id(value: &str) -> Result<(), FoundationError> {
    validate_bounded_string("local_state_record_id", value)?;
    if value.starts_with('.') || value.contains('/') || value.contains('\\') || value.contains("..")
    {
        return Err(FoundationError::InvalidFormat {
            field: "local_state_record_id",
        });
    }
    let upper = value.to_ascii_uppercase();
    let reserved = matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper[3..].bytes().all(|byte| (b'1'..=b'9').contains(&byte)));
    if reserved {
        return Err(FoundationError::ReservedName {
            field: "local_state_record_id",
        });
    }
    Ok(())
}

fn validate_topic_tag(value: &str) -> Result<(), FoundationError> {
    validate_bounded_string("topic_tag", value)?;
    if value.len() > 96
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || byte == b'-'
                || byte == b'_'
                || byte == b'.'
        })
    {
        return Err(FoundationError::InvalidCharacter { field: "topic_tag" });
    }
    Ok(())
}

fn validate_wire_handle(value: &str) -> Result<(), FoundationError> {
    validate_bounded_string("wire_handle", value)?;
    if value.split('/').any(str::is_empty)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
    {
        return Err(FoundationError::InvalidCharacter {
            field: "wire_handle",
        });
    }
    Ok(())
}

fn validate_bounded_string(field: &'static str, value: &str) -> Result<String, FoundationError> {
    if value.is_empty() {
        return Err(FoundationError::EmptyField { field });
    }
    if value
        .bytes()
        .any(|byte| byte == 0 || byte < 0x20 || byte == 0x7f)
    {
        return Err(FoundationError::InvalidCharacter { field });
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct FixedClock(&'static str);

    impl Clock for FixedClock {
        fn now_rfc3339(&self) -> String {
            self.0.to_owned()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct TestPayload {
        description: String,
    }

    #[test]
    fn rejects_unsafe_record_ids() {
        for value in [".hidden", "a/b", "..", "has\0nul", "CON", "LPT9"] {
            assert!(LocalStateRecordId::new(value).is_err(), "{value}");
        }
        assert_eq!(
            LocalStateRecordId::new("identity-main").unwrap().as_str(),
            "identity-main"
        );
    }

    #[test]
    fn rejects_kind_schema_mismatch() {
        let codec = WireNativeDocCodec::with_clock(FixedClock("2026-05-05T00:00:00Z"));
        let mut frontmatter = codec
            .frontmatter(
                LocalStateRecordKind::IdentitySnapshot,
                LocalStateRecordId::new("identity-main").unwrap(),
                LocalStateSchema::IdentitySnapshotV1,
            )
            .unwrap();
        frontmatter.state_schema = LocalStateSchema::TunnelStateV1;
        let document = LocalStateDocument {
            frontmatter,
            payload: TestPayload {
                description: "bad tuple".to_owned(),
            },
            prose: String::new(),
        };
        assert!(matches!(
            codec.render(&document),
            Err(LocalStateDocError::KindSchemaMismatch { .. })
        ));
    }

    #[test]
    fn parses_block_scalar_with_literal_doc_boundary_line() {
        let text = "---\nschema_version: 1\nrecord_kind: identity_snapshot\nrecord_id: identity-main\nstate_schema: identity_snapshot_v1\ncreated_at: \"2026-05-05T00:00:00Z\"\nupdated_at: \"2026-05-05T00:00:00Z\"\nsensitivity: public\n---\ndescription: |\n  ---\n  not a doc boundary\n---\n# notes\n";
        let parsed = WireNativeDocCodec::new()
            .parse::<TestPayload>(text)
            .unwrap();
        assert_eq!(parsed.payload.description, "---\nnot a doc boundary\n");
        assert_eq!(parsed.prose, "# notes\n");
    }

    #[test]
    fn render_parse_render_preserves_prose_tail_bytes() {
        let codec = WireNativeDocCodec::with_clock(FixedClock("2026-05-05T00:00:00Z"));
        let mut document = codec
            .document(
                LocalStateRecordKind::IdentitySnapshot,
                LocalStateRecordId::new("identity-main").unwrap(),
                LocalStateSchema::IdentitySnapshotV1,
                TestPayload {
                    description: "stable".to_owned(),
                },
                "# Operator Notes\n\nLeave my spacing alone.\n",
            )
            .unwrap();
        document.frontmatter.topics = vec![TopicTag::new("node-local-state").unwrap()];
        let rendered = codec.render(&document).unwrap();
        let reparsed = codec.parse::<TestPayload>(&rendered).unwrap();
        let rendered_again = codec.render(&reparsed).unwrap();
        assert_eq!(rendered_again, rendered);
    }

    #[test]
    fn local_state_refs_are_typed_at_decode() {
        let text = "---\nschema_version: 1\nrecord_kind: identity_snapshot\nrecord_id: identity-main\nstate_schema: identity_snapshot_v1\ncreated_at: \"2026-05-05T00:00:00Z\"\nupdated_at: \"2026-05-05T00:00:00Z\"\nsupersedes:\n  - kind: tunnel_state\n    id: tunnel-main\n    handle: agent/playful/kramer\nsensitivity: public\n---\ndescription: linked\n---\n";
        let parsed = WireNativeDocCodec::new()
            .parse::<TestPayload>(text)
            .unwrap();
        assert_eq!(
            parsed.frontmatter.supersedes[0].kind,
            LocalStateRecordKind::TunnelState
        );
        assert_eq!(parsed.frontmatter.supersedes[0].id.as_str(), "tunnel-main");
    }
}
