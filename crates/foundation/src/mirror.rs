use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::namespace::validate_slug;
use crate::{CrossGraphRef, FoundationError};

pub const MAX_MIRROR_PATH_BYTES: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorDirection {
    Upload,
    Download,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MirrorFileStatus {
    InSync,
    NeedsPull,
    NeedsPush,
    Pulling,
    Pushing,
    Skipped,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CorpusSlug(String);

impl CorpusSlug {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        validate_slug("corpus_slug", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for CorpusSlug {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<CorpusSlug> for String {
    fn from(value: CorpusSlug) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MirrorPath(String);

impl MirrorPath {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(FoundationError::EmptyField {
                field: "mirror_path",
            });
        }
        if value.len() > MAX_MIRROR_PATH_BYTES {
            return Err(FoundationError::OutOfRange {
                field: "mirror_path",
            });
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for MirrorPath {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<MirrorPath> for String {
    fn from(value: MirrorPath) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ContentHash(String);

impl ContentHash {
    pub fn new(value: impl Into<String>) -> Result<Self, FoundationError> {
        let value = value.into();
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(FoundationError::InvalidFormat {
                field: "content_hash",
            });
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn sha256_text(value: &str) -> Self {
        let digest = Sha256::digest(value.as_bytes());
        Self(bytes_to_hex(&digest))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ContentHash {
    type Error = FoundationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<ContentHash> for String {
    fn from(value: ContentHash) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorLink {
    pub local_root: MirrorPath,
    pub corpus_slug: CorpusSlug,
    pub direction: MirrorDirection,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedDocument {
    pub document_ref: CrossGraphRef,
    pub corpus_slug: CorpusSlug,
    pub source_path: MirrorPath,
    pub body_hash: ContentHash,
    pub file_size_bytes: u64,
    pub sync_status: MirrorFileStatus,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteDocumentInfo {
    pub document_ref: CrossGraphRef,
    pub corpus_slug: CorpusSlug,
    pub source_path: Option<MirrorPath>,
    pub title: Option<String>,
    pub format: Option<String>,
    pub body_hash: ContentHash,
    pub updated_at: Option<String>,
}

impl RemoteDocumentInfo {
    pub fn effective_path(&self) -> Result<MirrorPath, FoundationError> {
        if let Some(path) = &self.source_path {
            return Ok(path.clone());
        }
        let base = self.title.as_deref().unwrap_or("document");
        let slug = slugify_path_component(base);
        let extension = match self.format.as_deref() {
            Some("text/html") => ".html",
            Some("text/plain") => ".txt",
            Some("application/pdf") => ".pdf",
            _ => ".md",
        };
        if slug.ends_with(extension) {
            MirrorPath::new(slug)
        } else {
            MirrorPath::new(format!("{slug}{extension}"))
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalDocumentSnapshot {
    pub relative_path: MirrorPath,
    pub body_hash: ContentHash,
    pub size_bytes: u64,
}

impl LocalDocumentSnapshot {
    pub fn from_text(relative_path: MirrorPath, body: &str) -> Result<Self, FoundationError> {
        Ok(Self {
            relative_path,
            body_hash: ContentHash::sha256_text(body),
            size_bytes: body.len() as u64,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorConflict {
    pub source_path: MirrorPath,
    pub corpus_slug: CorpusSlug,
    pub local_hash: ContentHash,
    pub remote_hash: ContentHash,
    pub remote_updated_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorDiff {
    pub to_push: Vec<LocalDocumentSnapshot>,
    pub to_pull: Vec<RemoteDocumentInfo>,
    pub to_update: Vec<(LocalDocumentSnapshot, RemoteDocumentInfo)>,
    pub hash_matched: Vec<(LocalDocumentSnapshot, RemoteDocumentInfo)>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorState {
    pub links: Vec<MirrorLink>,
    pub cached_documents: Vec<CachedDocument>,
    pub total_size_bytes: u64,
    pub last_sync_at: Option<String>,
    pub is_syncing: bool,
    pub conflicts: Vec<MirrorConflict>,
}

impl MirrorState {
    pub fn link(&mut self, link: MirrorLink) {
        self.links
            .retain(|existing| existing.local_root != link.local_root);
        self.links.push(link);
    }

    pub fn unlink(&mut self, local_root: &MirrorPath) -> bool {
        let before = self.links.len();
        self.links
            .retain(|existing| &existing.local_root != local_root);
        self.links.len() != before
    }
}

pub fn compute_mirror_diff(
    local_docs: &[LocalDocumentSnapshot],
    remote_docs: &[RemoteDocumentInfo],
) -> Result<MirrorDiff, FoundationError> {
    let remote_paths = remote_docs
        .iter()
        .map(|doc| doc.effective_path().map(|path| (path, doc)))
        .collect::<Result<Vec<_>, _>>()?;
    let remote_by_path = remote_paths
        .iter()
        .map(|(path, doc)| (path.as_str(), *doc))
        .collect::<HashMap<_, _>>();
    let local_by_path = local_docs
        .iter()
        .map(|doc| (doc.relative_path.as_str(), doc))
        .collect::<HashMap<_, _>>();

    let mut to_push = Vec::new();
    let mut to_update = Vec::new();
    let mut matched_remote_refs = HashSet::new();
    let mut unmatched_local = Vec::new();

    for local_doc in local_docs {
        match remote_by_path.get(local_doc.relative_path.as_str()) {
            Some(remote_doc) => {
                matched_remote_refs.insert(remote_doc.document_ref.to_string());
                if local_doc.body_hash != remote_doc.body_hash {
                    to_update.push((local_doc.clone(), (*remote_doc).clone()));
                }
            }
            None => unmatched_local.push(local_doc.clone()),
        }
    }

    let mut remote_by_hash = HashMap::new();
    for remote_doc in remote_docs {
        if !matched_remote_refs.contains(&remote_doc.document_ref.to_string()) {
            remote_by_hash
                .entry(remote_doc.body_hash.as_str())
                .or_insert(remote_doc);
        }
    }

    let local_hashes = local_docs
        .iter()
        .map(|doc| doc.body_hash.as_str())
        .collect::<HashSet<_>>();
    let mut hash_matched_remote_refs = HashSet::new();
    let mut hash_matched = Vec::new();

    for local_doc in unmatched_local {
        if let Some(remote_doc) = remote_by_hash.get(local_doc.body_hash.as_str()) {
            hash_matched_remote_refs.insert(remote_doc.document_ref.to_string());
            hash_matched.push((local_doc, (*remote_doc).clone()));
        } else {
            to_push.push(local_doc);
        }
    }

    let mut to_pull = Vec::new();
    for remote_doc in remote_docs {
        let path = remote_doc.effective_path()?;
        if !local_by_path.contains_key(path.as_str())
            && !hash_matched_remote_refs.contains(&remote_doc.document_ref.to_string())
            && !local_hashes.contains(remote_doc.body_hash.as_str())
        {
            to_pull.push(remote_doc.clone());
        }
    }

    Ok(MirrorDiff {
        to_push,
        to_pull,
        to_update,
        hash_matched,
    })
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(nibble_to_hex(byte >> 4));
        output.push(nibble_to_hex(byte & 0x0f));
    }
    output
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => unreachable!("nibble out of range"),
    }
}

fn slugify_path_component(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if slug.is_empty() {
        "document".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> CorpusSlug {
        CorpusSlug::new("playful-docs").unwrap()
    }

    fn path(value: &str) -> MirrorPath {
        MirrorPath::new(value).unwrap()
    }

    fn remote(
        sequence: u32,
        source_path: Option<&str>,
        title: Option<&str>,
        body: &str,
    ) -> RemoteDocumentInfo {
        RemoteDocumentInfo {
            document_ref: format!("playful/124/doc/{sequence}").parse().unwrap(),
            corpus_slug: corpus(),
            source_path: source_path.map(path),
            title: title.map(str::to_owned),
            format: Some("text/markdown".to_owned()),
            body_hash: ContentHash::sha256_text(body),
            updated_at: None,
        }
    }

    #[test]
    fn content_hash_is_stable_sha256_hex() {
        assert_eq!(
            ContentHash::sha256_text("abc").as_str(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn mirror_diff_tracks_push_pull_update_and_hash_match() {
        let local = vec![
            LocalDocumentSnapshot::from_text(path("same.md"), "same").unwrap(),
            LocalDocumentSnapshot::from_text(path("changed.md"), "local").unwrap(),
            LocalDocumentSnapshot::from_text(path("new.md"), "new").unwrap(),
            LocalDocumentSnapshot::from_text(path("renamed-local.md"), "same-body").unwrap(),
        ];
        let remote = vec![
            remote(1, Some("same.md"), None, "same"),
            remote(2, Some("changed.md"), None, "remote"),
            remote(3, Some("missing.md"), None, "missing"),
            remote(4, Some("renamed-remote.md"), None, "same-body"),
        ];

        let diff = compute_mirror_diff(&local, &remote).unwrap();

        assert_eq!(diff.to_push[0].relative_path.as_str(), "new.md");
        assert_eq!(
            diff.to_pull[0].effective_path().unwrap().as_str(),
            "missing.md"
        );
        assert_eq!(diff.to_update[0].0.relative_path.as_str(), "changed.md");
        assert_eq!(
            diff.hash_matched[0].0.relative_path.as_str(),
            "renamed-local.md"
        );
    }

    #[test]
    fn remote_effective_path_falls_back_to_title_and_format() {
        let doc = RemoteDocumentInfo {
            document_ref: "playful/124/doc/5".parse().unwrap(),
            corpus_slug: corpus(),
            source_path: None,
            title: Some("Hello Wire".to_owned()),
            format: Some("text/plain".to_owned()),
            body_hash: ContentHash::sha256_text("hello"),
            updated_at: None,
        };

        assert_eq!(doc.effective_path().unwrap().as_str(), "Hello-Wire.txt");
    }

    #[test]
    fn mirror_state_replaces_link_for_same_root() {
        let mut state = MirrorState::default();
        state.link(MirrorLink {
            local_root: path("/tmp/docs"),
            corpus_slug: corpus(),
            direction: MirrorDirection::Upload,
        });
        state.link(MirrorLink {
            local_root: path("/tmp/docs"),
            corpus_slug: CorpusSlug::new("other-docs").unwrap(),
            direction: MirrorDirection::Download,
        });

        assert_eq!(state.links.len(), 1);
        assert_eq!(state.links[0].corpus_slug.as_str(), "other-docs");
        assert!(state.unlink(&path("/tmp/docs")));
        assert!(state.links.is_empty());
    }
}
