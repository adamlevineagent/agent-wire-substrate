use std::fmt;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use agent_wire_foundation::{
    FoundationError, LocalStateRecordId, LocalStateRecordKind, LocalStateSchema, LocalStateSubject,
    LocalStateSubjectKind, SecretMaterial, SecretString, TopicTag, TransportDriver, TunnelRequest,
    TunnelSession, TunnelUrl, WireNativeDocCodec,
};
use serde::{Deserialize, Serialize};

pub const TUNNEL_STATE_FILE: &str = "tunnel_state/tunnel.md";
pub const LEGACY_TUNNEL_STATE_FILE: &str = "tunnel.json";
pub const CLOUDFLARED_BINARY_NAME: &str = "cloudflared";

#[derive(Debug, PartialEq, Eq)]
pub enum CloudflareTunnelError {
    Foundation(FoundationError),
    Io(String),
    Http(String),
    Json(String),
    UnsupportedPlatform,
    MissingTunnelToken,
    Process(String),
    NoStderr,
}

impl fmt::Display for CloudflareTunnelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Foundation(error) => write!(f, "{error}"),
            Self::Io(error) => write!(f, "io error: {error}"),
            Self::Http(error) => write!(f, "http error: {error}"),
            Self::Json(error) => write!(f, "json error: {error}"),
            Self::UnsupportedPlatform => f.write_str("unsupported cloudflared platform"),
            Self::MissingTunnelToken => f.write_str("missing tunnel token"),
            Self::Process(error) => write!(f, "process error: {error}"),
            Self::NoStderr => f.write_str("cloudflared stderr was not piped"),
        }
    }
}

impl std::error::Error for CloudflareTunnelError {}

impl From<FoundationError> for CloudflareTunnelError {
    fn from(value: FoundationError) -> Self {
        Self::Foundation(value)
    }
}

impl From<std::io::Error> for CloudflareTunnelError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TunnelConnectionStatus {
    Disconnected,
    Provisioning,
    Downloading,
    Connecting,
    Connected,
    Error(String),
}

impl Default for TunnelConnectionStatus {
    fn default() -> Self {
        Self::Disconnected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TunnelState {
    pub tunnel_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_tunnel_url_tolerant")]
    pub tunnel_url: Option<TunnelUrl>,
    pub tunnel_token: Option<String>,
    pub status: TunnelConnectionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
struct TunnelStateDoc {
    pub tunnel_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_tunnel_url_tolerant")]
    pub tunnel_url: Option<TunnelUrl>,
    pub tunnel_token: Option<SecretMaterial>,
    pub status: TunnelConnectionStatus,
}

impl TunnelStateDoc {
    fn from_state(state: &TunnelState) -> Result<Self, CloudflareTunnelError> {
        Ok(Self {
            tunnel_id: state.tunnel_id.clone(),
            tunnel_url: state.tunnel_url.clone(),
            tunnel_token: state
                .tunnel_token
                .as_ref()
                .map(|token| {
                    SecretString::new(token.clone())
                        .map(SecretMaterial::Inline)
                        .map_err(CloudflareTunnelError::from)
                })
                .transpose()?,
            status: state.status.clone(),
        })
    }

    fn into_state(self) -> Result<TunnelState, CloudflareTunnelError> {
        let tunnel_token = match self.tunnel_token {
            Some(SecretMaterial::Inline(secret)) => Some(secret.expose_secret().to_owned()),
            Some(SecretMaterial::KeychainRef(handle)) => {
                return Err(CloudflareTunnelError::Json(format!(
                    "keychain tunnel token `{}` is not supported by this transport driver yet",
                    handle.as_str()
                )))
            }
            None => None,
        };
        Ok(TunnelState {
            tunnel_id: self.tunnel_id,
            tunnel_url: self.tunnel_url,
            tunnel_token,
            status: self.status,
        })
    }
}

fn deserialize_tunnel_url_tolerant<'de, D>(deserializer: D) -> Result<Option<TunnelUrl>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw.and_then(|value| TunnelUrl::parse(&value).ok()))
}

#[derive(Debug, Deserialize)]
struct ProvisionResponse {
    tunnel_token: String,
    tunnel_url: String,
    tunnel_id: String,
}

#[derive(Debug, Clone, Default)]
pub struct CloudflareTunnelDriver {
    tunnel_url: Option<TunnelUrl>,
}

impl CloudflareTunnelDriver {
    pub fn new(tunnel_url: Option<TunnelUrl>) -> Self {
        Self { tunnel_url }
    }

    pub fn from_state(state: &TunnelState) -> Self {
        Self {
            tunnel_url: state.tunnel_url.clone(),
        }
    }

    pub fn with_static_tunnel(tunnel_url: TunnelUrl) -> Self {
        Self {
            tunnel_url: Some(tunnel_url),
        }
    }
}

impl TransportDriver for CloudflareTunnelDriver {
    type Error = FoundationError;

    fn driver_name(&self) -> &'static str {
        "cloudflare"
    }

    fn tunnel_url(&self) -> Option<TunnelUrl> {
        self.tunnel_url.clone()
    }

    fn open_tunnel(&self, request: TunnelRequest) -> Result<TunnelSession, Self::Error> {
        let public_url = self
            .tunnel_url
            .clone()
            .or_else(|| request.requested_public_url.clone())
            .ok_or(FoundationError::EmptyField {
                field: "public_tunnel_url",
            })?;

        Ok(TunnelSession {
            driver_name: self.driver_name().to_owned(),
            public_url,
            local_endpoint: request.local_endpoint,
            callbacks: request.callbacks,
        })
    }
}

pub fn cloudflared_binary_path(data_dir: &Path) -> PathBuf {
    let binary_name = if cfg!(target_os = "windows") {
        "cloudflared.exe"
    } else {
        CLOUDFLARED_BINARY_NAME
    };
    data_dir.join("bin").join(binary_name)
}

pub fn is_cloudflared_installed(data_dir: &Path) -> bool {
    cloudflared_binary_path(data_dir).exists()
}

pub fn cloudflared_download_url() -> Result<String, CloudflareTunnelError> {
    cloudflared_download_url_for(std::env::consts::OS, std::env::consts::ARCH)
}

pub fn cloudflared_download_url_for(
    target_os: &str,
    target_arch: &str,
) -> Result<String, CloudflareTunnelError> {
    let base = "https://github.com/cloudflare/cloudflared/releases/latest/download";
    match (target_os, target_arch) {
        ("macos", "aarch64") => Ok(format!("{base}/cloudflared-darwin-arm64.tgz")),
        ("macos", _) => Ok(format!("{base}/cloudflared-darwin-amd64.tgz")),
        ("windows", _) => Ok(format!("{base}/cloudflared-windows-amd64.exe")),
        ("linux", "aarch64") => Ok(format!("{base}/cloudflared-linux-arm64")),
        ("linux", _) => Ok(format!("{base}/cloudflared-linux-amd64")),
        _ => Err(CloudflareTunnelError::UnsupportedPlatform),
    }
}

pub fn download_cloudflared(data_dir: &Path) -> Result<PathBuf, CloudflareTunnelError> {
    let binary_path = cloudflared_binary_path(data_dir);
    if binary_path.exists() && cloudflared_binary_is_runnable(&binary_path) {
        return Ok(binary_path);
    }
    if binary_path.exists() {
        fs::remove_file(&binary_path)?;
    }

    let download_url = cloudflared_download_url()?;
    let response = ureq::get(&download_url)
        .call()
        .map_err(|error| CloudflareTunnelError::Http(error.to_string()))?;
    if response.status() >= 400 {
        return Err(CloudflareTunnelError::Http(format!(
            "download failed with status {}",
            response.status()
        )));
    }

    let bin_dir = binary_path.parent().ok_or_else(|| {
        CloudflareTunnelError::Io(format!("missing parent for {}", binary_path.display()))
    })?;
    fs::create_dir_all(bin_dir)?;

    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    if download_url.ends_with(".tgz") {
        let archive_path = bin_dir.join("cloudflared.tgz");
        fs::write(&archive_path, bytes)?;
        let output = Command::new("tar")
            .args(["xzf", "cloudflared.tgz"])
            .current_dir(bin_dir)
            .output()?;
        let _ = fs::remove_file(&archive_path);
        if !output.status.success() {
            return Err(CloudflareTunnelError::Process(
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
    } else {
        fs::write(&binary_path, bytes)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&binary_path, fs::Permissions::from_mode(0o755))?;
    }

    Ok(binary_path)
}

fn cloudflared_binary_is_runnable(binary_path: &Path) -> bool {
    Command::new(binary_path)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

pub fn provision_tunnel(
    api_base_url: &str,
    access_token: &str,
    node_id: &str,
) -> Result<TunnelState, CloudflareTunnelError> {
    let url = format!("{}/api/v1/node/tunnel", api_base_url.trim_end_matches('/'));
    let response = ureq::post(&url)
        .set("Authorization", &format!("Bearer {access_token}"))
        .set("Content-Type", "application/json")
        .send_json(serde_json::json!({ "node_id": node_id }))
        .map_err(|error| CloudflareTunnelError::Http(error.to_string()))?;
    let provision: ProvisionResponse = response
        .into_json()
        .map_err(|error| CloudflareTunnelError::Json(error.to_string()))?;
    let tunnel_url = TunnelUrl::parse(&provision.tunnel_url)?;

    Ok(TunnelState {
        tunnel_id: Some(provision.tunnel_id),
        tunnel_url: Some(tunnel_url),
        tunnel_token: Some(provision.tunnel_token),
        status: TunnelConnectionStatus::Provisioning,
    })
}

pub fn load_tunnel_state(data_dir: &Path) -> Result<Option<TunnelState>, CloudflareTunnelError> {
    load_tunnel_state_from_path(&data_dir.join(TUNNEL_STATE_FILE))
        .or_else(|| load_tunnel_state_from_path(&data_dir.join(LEGACY_TUNNEL_STATE_FILE)))
        .transpose()
}

pub fn load_tunnel_state_for_node(
    data_dir: &Path,
    node_id: &str,
) -> Result<Option<TunnelState>, CloudflareTunnelError> {
    load_tunnel_state_from_path(&tunnel_state_path(data_dir, node_id))
        .or_else(|| load_tunnel_state_from_path(&data_dir.join(TUNNEL_STATE_FILE)))
        .or_else(|| load_tunnel_state_from_path(&data_dir.join(LEGACY_TUNNEL_STATE_FILE)))
        .transpose()
}

pub fn save_tunnel_state(
    data_dir: &Path,
    state: &TunnelState,
) -> Result<(), CloudflareTunnelError> {
    save_tunnel_state_to_path(&data_dir.join(TUNNEL_STATE_FILE), "tunnel", state)
}

pub fn save_tunnel_state_for_node(
    data_dir: &Path,
    node_id: &str,
    state: &TunnelState,
) -> Result<(), CloudflareTunnelError> {
    save_tunnel_state_to_path(&tunnel_state_path(data_dir, node_id), node_id, state)
}

pub fn tunnel_state_path(data_dir: &Path, node_id: &str) -> PathBuf {
    data_dir
        .join(LocalStateRecordKind::TunnelState.directory_name())
        .join(format!("{node_id}.md"))
}

fn load_tunnel_state_from_path(path: &Path) -> Option<Result<TunnelState, CloudflareTunnelError>> {
    if !path.exists() {
        return None;
    }
    Some(read_tunnel_state_file(path))
}

fn read_tunnel_state_file(path: &Path) -> Result<TunnelState, CloudflareTunnelError> {
    let data = fs::read_to_string(path)?;
    if data.starts_with("---\n") {
        let document = WireNativeDocCodec::new()
            .parse::<TunnelStateDoc>(&data)
            .map_err(|error| CloudflareTunnelError::Json(error.to_string()))?;
        return document.payload.into_state();
    }
    serde_json::from_str(&data).map_err(|error| CloudflareTunnelError::Json(error.to_string()))
}

fn save_tunnel_state_to_path(
    path: &Path,
    record_id: &str,
    state: &TunnelState,
) -> Result<(), CloudflareTunnelError> {
    let codec = WireNativeDocCodec::new();
    let mut document = codec
        .document(
            LocalStateRecordKind::TunnelState,
            LocalStateRecordId::new(record_id)?,
            LocalStateSchema::TunnelStateV1,
            TunnelStateDoc::from_state(state)?,
            "# Operator Notes\n\nCloudflare tunnel state for agent-wire-substrate transport.\n",
        )
        .map_err(|error| CloudflareTunnelError::Json(error.to_string()))?;
    document.frontmatter.topics = vec![
        TopicTag::new("agent-wire-substrate-node")?,
        TopicTag::new("cloudflare-tunnel")?,
    ];
    document.frontmatter.subjects = vec![LocalStateSubject {
        kind: LocalStateSubjectKind::NodeId,
        value: record_id.to_owned(),
    }];
    codec
        .write(path, &document)
        .map_err(|error| CloudflareTunnelError::Json(error.to_string()))
}

pub fn persisted_state_is_stale_for_node(state: &TunnelState, node_id: &str) -> bool {
    state
        .tunnel_url
        .as_ref()
        .map(|url| {
            let expected_prefix = format!("https://node-{node_id}.");
            !url.as_str().starts_with(&expected_prefix)
        })
        .unwrap_or(false)
}

pub fn resolve_or_provision_tunnel_state(
    data_dir: &Path,
    api_base_url: &str,
    access_token: &str,
    node_id: &str,
) -> Result<TunnelState, CloudflareTunnelError> {
    resolve_or_provision_tunnel_state_with(data_dir, node_id, || {
        provision_tunnel(api_base_url, access_token, node_id)
    })
}

pub fn resolve_or_provision_tunnel_state_with<F>(
    data_dir: &Path,
    node_id: &str,
    provision: F,
) -> Result<TunnelState, CloudflareTunnelError>
where
    F: FnOnce() -> Result<TunnelState, CloudflareTunnelError>,
{
    if let Some(mut persisted) = load_tunnel_state_for_node(data_dir, node_id)? {
        if persisted_state_is_stale_for_node(&persisted, node_id) {
            let _ = fs::remove_file(tunnel_state_path(data_dir, node_id));
            let _ = fs::remove_file(data_dir.join(TUNNEL_STATE_FILE));
            let _ = fs::remove_file(data_dir.join(LEGACY_TUNNEL_STATE_FILE));
        } else if persisted.tunnel_token.is_some() {
            persisted.status = TunnelConnectionStatus::Connecting;
            return Ok(persisted);
        }
    }

    let provisioned = provision()?;
    save_tunnel_state_for_node(data_dir, node_id, &provisioned)?;
    Ok(provisioned)
}

pub fn start_tunnel(data_dir: &Path, tunnel_token: &str) -> Result<Child, CloudflareTunnelError> {
    if tunnel_token.is_empty() {
        return Err(CloudflareTunnelError::MissingTunnelToken);
    }
    let binary_path = cloudflared_binary_path(data_dir);
    if !binary_path.exists() {
        return Err(CloudflareTunnelError::Process(
            "cloudflared binary not found; call download_cloudflared first".to_owned(),
        ));
    }

    kill_orphan_cloudflared();

    Command::new(binary_path)
        .arg("tunnel")
        .arg("run")
        .env("TUNNEL_TOKEN", tunnel_token)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(CloudflareTunnelError::from)
}

pub fn kill_orphan_cloudflared() {
    #[cfg(unix)]
    {
        let _ = Command::new("pkill")
            .arg("-f")
            .arg("cloudflared tunnel run")
            .output();
        thread::sleep(Duration::from_millis(500));
    }
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/F", "/IM", "cloudflared.exe"])
            .output();
        thread::sleep(Duration::from_millis(500));
    }
}

pub fn monitor_tunnel_output(
    child: &mut Child,
    max_lines: usize,
    line_timeout: Duration,
) -> Result<TunnelConnectionStatus, CloudflareTunnelError> {
    let stderr = child.stderr.take().ok_or(CloudflareTunnelError::NoStderr)?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });

    for _ in 0..max_lines {
        match receiver.recv_timeout(line_timeout) {
            Ok(Ok(line)) if cloudflared_line_marks_connected(&line) => {
                return Ok(TunnelConnectionStatus::Connected);
            }
            Ok(Ok(line)) if cloudflared_line_marks_error(&line) => {
                return Ok(TunnelConnectionStatus::Error(line));
            }
            Ok(Ok(_)) => {}
            Ok(Err(error)) => return Ok(TunnelConnectionStatus::Error(error.to_string())),
            Err(mpsc::RecvTimeoutError::Timeout) => break,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    Ok(TunnelConnectionStatus::Connecting)
}

pub fn cloudflared_line_marks_connected(line: &str) -> bool {
    line.contains("Registered tunnel connection")
        || line.contains("Connection registered")
        || line.contains("connIndex=")
}

pub fn cloudflared_line_marks_error(line: &str) -> bool {
    let lower = line.to_lowercase();
    if lower.contains("failed to sufficiently")
        || lower.contains("update check")
        || lower.contains("buffer size")
        || lower.contains("metrics server")
        || lower.contains("capacity")
        || (lower.contains(" inf ") && !lower.contains("tunnel connection failed"))
    {
        return false;
    }

    lower.contains(" err ")
        || lower.contains("\"level\":\"error\"")
        || lower.contains("failed to connect to edge")
        || lower.contains("tunnel connection failed")
        || lower.contains("authentication failed")
        || (lower.contains("credential") && lower.contains("error"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_wire_foundation::{CallbackUrl, EndpointUrl};

    #[test]
    fn driver_opens_session_from_static_url() {
        let driver = CloudflareTunnelDriver::with_static_tunnel(
            TunnelUrl::parse("https://tunnel.example").unwrap(),
        );
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap())
            .with_callback(CallbackUrl::parse("https://node.example/callback").unwrap());

        let session = driver.open_tunnel(request).unwrap();

        assert_eq!(session.driver_name, "cloudflare");
        assert_eq!(session.public_url.as_str(), "https://tunnel.example");
        assert_eq!(session.callbacks.len(), 1);
    }

    #[test]
    fn driver_uses_requested_public_url_when_static_url_absent() {
        let driver = CloudflareTunnelDriver::default();
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap())
            .with_requested_public_url(TunnelUrl::parse("https://requested.example").unwrap());

        let session = driver.open_tunnel(request).unwrap();

        assert_eq!(session.public_url.as_str(), "https://requested.example");
    }

    #[test]
    fn driver_errors_without_any_public_url() {
        let driver = CloudflareTunnelDriver::default();
        let request = TunnelRequest::new(EndpointUrl::parse("http://127.0.0.1:8787").unwrap());

        assert_eq!(
            driver.open_tunnel(request),
            Err(FoundationError::EmptyField {
                field: "public_tunnel_url"
            })
        );
    }

    #[test]
    fn tunnel_state_tolerates_bad_urls_without_losing_token() {
        let json = r#"{
            "tunnel_id": "tun-xyz",
            "tunnel_url": "not a url",
            "tunnel_token": "tok",
            "status": "connected"
        }"#;

        let decoded: TunnelState = serde_json::from_str(json).unwrap();

        assert_eq!(decoded.tunnel_id.as_deref(), Some("tun-xyz"));
        assert!(decoded.tunnel_url.is_none());
        assert_eq!(decoded.tunnel_token.as_deref(), Some("tok"));
        assert_eq!(decoded.status, TunnelConnectionStatus::Connected);
    }

    #[test]
    fn tunnel_state_detects_stale_node_url() {
        let state = TunnelState {
            tunnel_id: Some("tun-1".to_owned()),
            tunnel_url: Some(TunnelUrl::parse("https://node-other.agent-wire.com").unwrap()),
            tunnel_token: Some("tok".to_owned()),
            status: TunnelConnectionStatus::Connected,
        };

        assert!(persisted_state_is_stale_for_node(&state, "current"));
        assert!(!persisted_state_is_stale_for_node(&state, "other"));
    }

    #[test]
    fn tunnel_state_round_trips_to_disk() {
        let data_dir =
            std::env::temp_dir().join(format!("agent-wire-cloudflare-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data_dir);
        let state = TunnelState {
            tunnel_id: Some("tun-1".to_owned()),
            tunnel_url: Some(TunnelUrl::parse("https://node-demo.agent-wire.com").unwrap()),
            tunnel_token: Some("tok".to_owned()),
            status: TunnelConnectionStatus::Connected,
        };

        save_tunnel_state(&data_dir, &state).unwrap();
        let loaded = load_tunnel_state(&data_dir).unwrap().unwrap();

        assert_eq!(loaded, state);
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn resolver_replaces_stale_state_and_persists_new_tunnel() {
        let data_dir = std::env::temp_dir().join(format!(
            "agent-wire-cloudflare-resolve-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&data_dir);
        save_tunnel_state(
            &data_dir,
            &TunnelState {
                tunnel_id: Some("old".to_owned()),
                tunnel_url: Some(TunnelUrl::parse("https://node-other.agent-wire.com").unwrap()),
                tunnel_token: Some("old-token".to_owned()),
                status: TunnelConnectionStatus::Connected,
            },
        )
        .unwrap();

        let resolved = resolve_or_provision_tunnel_state_with(&data_dir, "current", || {
            Ok(TunnelState {
                tunnel_id: Some("new".to_owned()),
                tunnel_url: Some(TunnelUrl::parse("https://node-current.agent-wire.com").unwrap()),
                tunnel_token: Some("new-token".to_owned()),
                status: TunnelConnectionStatus::Provisioning,
            })
        })
        .unwrap();
        let persisted = load_tunnel_state_for_node(&data_dir, "current")
            .unwrap()
            .unwrap();

        assert_eq!(resolved.tunnel_id.as_deref(), Some("new"));
        assert_eq!(persisted.tunnel_id.as_deref(), Some("new"));
        assert_eq!(persisted.tunnel_token.as_deref(), Some("new-token"));
        let _ = fs::remove_dir_all(&data_dir);
    }

    #[test]
    fn cloudflared_download_url_selects_target_artifact() {
        assert!(cloudflared_download_url_for("macos", "aarch64")
            .unwrap()
            .ends_with("cloudflared-darwin-arm64.tgz"));
        assert!(cloudflared_download_url_for("linux", "x86_64")
            .unwrap()
            .ends_with("cloudflared-linux-amd64"));
        assert_eq!(
            cloudflared_download_url_for("plan9", "x86_64"),
            Err(CloudflareTunnelError::UnsupportedPlatform)
        );
    }

    #[test]
    fn cloudflared_line_classifier_matches_connection_and_errors() {
        assert!(cloudflared_line_marks_connected(
            "INF Registered tunnel connection connIndex=0"
        ));
        assert!(cloudflared_line_marks_error(
            "ERR tunnel connection failed: authentication failed"
        ));
        assert!(!cloudflared_line_marks_error(
            "INF metrics server stopped because capacity changed"
        ));
    }
}
