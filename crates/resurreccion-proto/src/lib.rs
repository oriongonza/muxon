//! Wire protocol for the Resurreccion daemon.
//!
//! All communication between CLI, plugins, and the daemon goes through
//! the types defined here. The envelope schema is the stability boundary:
//! once 1.0 ships, no field may be removed or semantically changed.

pub mod verbs;

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// Protocol version. Increment on any breaking envelope change.
pub const PROTO_VERSION: u32 = 1;

/// The wire envelope wrapping every request and response.
///
/// Framed on the wire as a length-delimited JSON line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// Caller-chosen correlation ID; echoed in the matching response.
    pub id: String,
    /// Verb name (e.g. `"doctor.ping"`, `"workspace.open"`).
    pub verb: String,
    /// Protocol version of the sender.
    pub proto: u32,
    /// `true` for success, `false` for error.
    pub ok: bool,
    /// Verb-specific payload. On error, contains `{ "code": "...", "message": "..." }`.
    pub body: serde_json::Value,
    /// Unix timestamp (ms) when this envelope was created.
    pub ts: u64,
}

impl Envelope {
    /// Create a success envelope.
    pub fn ok(id: impl Into<String>, verb: impl Into<String>, body: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            verb: verb.into(),
            proto: PROTO_VERSION,
            ok: true,
            body,
            ts: now_ms(),
        }
    }

    /// Create an error envelope.
    pub fn err(
        id: impl Into<String>,
        verb: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            verb: verb.into(),
            proto: PROTO_VERSION,
            ok: false,
            body: serde_json::json!({ "code": code.into(), "message": message.into() }),
            ts: now_ms(),
        }
    }
}

#[allow(clippy::cast_possible_truncation)]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// A typed request from CLI or plugin to the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// Verb to invoke.
    pub verb: String,
    /// Verb-specific arguments.
    pub args: serde_json::Value,
}

impl Request {
    /// Build a raw request.
    pub fn new(verb: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            verb: verb.into(),
            args,
        }
    }

    /// `doctor.ping` — health check.
    pub fn doctor_ping() -> Self {
        Self::new(verbs::DOCTOR_PING, serde_json::Value::Null)
    }

    /// `workspace.open` — open or create a workspace.
    pub fn workspace_open(args: WorkspaceOpenArgs) -> Self {
        Self::new(
            verbs::WORKSPACE_OPEN,
            serde_json::to_value(args).unwrap_or_default(),
        )
    }

    /// `workspace.list` — list all workspaces.
    pub fn workspace_list() -> Self {
        Self::new(verbs::WORKSPACE_LIST, serde_json::Value::Null)
    }

    /// `snapshot.create` — create a snapshot of the current workspace state.
    pub fn snapshot_create(args: SnapshotCreateArgs) -> Self {
        Self::new(
            verbs::SNAPSHOT_CREATE,
            serde_json::to_value(args).unwrap_or_default(),
        )
    }

    /// `snapshot.restore` — restore a workspace from a snapshot.
    pub fn snapshot_restore(args: SnapshotRestoreArgs) -> Self {
        Self::new(
            verbs::SNAPSHOT_RESTORE,
            serde_json::to_value(args).unwrap_or_default(),
        )
    }
}

/// A typed response carrying success data or an error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// Successful response with verb-specific data.
    Ok {
        /// Verb-specific response payload.
        data: serde_json::Value,
    },
    /// Error response.
    Err {
        /// Machine-readable error code.
        code: String,
        /// Human-readable message.
        message: String,
    },
}

// ── Verb-specific arg/result types ─────────────────────────────────────────

/// Arguments for `workspace.open`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceOpenArgs {
    /// Canonical filesystem path to bind to.
    pub path: String,
}

/// Arguments for `snapshot.create`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotCreateArgs {
    /// ID of the workspace to snapshot (as string).
    pub workspace_id: String,
}

/// Arguments for `snapshot.restore`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRestoreArgs {
    /// ID of the snapshot to restore (as string).
    pub snapshot_id: String,
}

/// Request for capability negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRequest {
    /// Protocol version supported by the client.
    pub client_proto: u32,
    /// Capabilities requested by the client.
    pub client_capabilities: Vec<String>,
}

/// Response to capability negotiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResponse {
    /// Protocol version supported by the server.
    pub server_proto: u32,
    /// Capabilities agreed upon by the server.
    pub agreed_capabilities: Vec<String>,
}

/// Request to subscribe to the event stream.
///
/// Wire protocol for streaming:
/// 1. Client sends Envelope { verb: EVENTS_SUBSCRIBE, body: SubscribeRequest as JSON }
/// 2. Server responds with Envelope { verb: EVENTS_SUBSCRIBE, ok: true, body: {} } (ack)
/// 3. Server then sends zero or more Envelope { verb: EVENTS_PUSH, ok: true, body: EventRow as JSON }
/// 4. Loop exits when client closes connection (write error on server side)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    /// Optional workspace ID to filter events by. If None, subscribe to all workspaces.
    pub workspace_id: Option<String>,
    /// Optional event ID to resume streaming after. If None, start from the beginning.
    pub after_id: Option<String>,
}

// ── Legacy API (preserved for daemon compatibility) ───────────────────────

/// Legacy constant for backwards compatibility.
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/resurreccion.sock";

/// Legacy enum preserved for daemon compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyRequest {
    /// Health check request.
    Health,
}

impl LegacyRequest {
    /// Parse a request from a string (legacy API).
    pub fn parse(input: &str) -> Result<Self, String> {
        match input.trim() {
            "health" => Ok(Self::Health),
            other => Err(format!("unknown request: {other}")),
        }
    }

    /// Get the wire format representation of this request (legacy API).
    pub const fn as_wire(&self) -> &'static str {
        match self {
            Self::Health => "health\n",
        }
    }
}

/// Legacy response struct (preserved for daemon compatibility).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthResponse {
    /// Name of the service.
    pub service: &'static str,
    /// Status of the service.
    pub status: &'static str,
    /// Path to the socket.
    pub socket_path: String,
}

/// Legacy error response (preserved for daemon compatibility).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorResponse {
    /// Error code.
    pub code: String,
    /// Error message.
    pub message: String,
}

/// Legacy response enum preserved for daemon compatibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyResponse {
    /// Health check response.
    Health(HealthResponse),
    /// Error response.
    Error(ErrorResponse),
}

impl LegacyResponse {
    /// Create a successful health response (legacy API).
    pub fn health(socket_path: &std::path::Path) -> Self {
        Self::Health(HealthResponse {
            service: "resurreccion-daemon",
            status: "ok",
            socket_path: socket_path.display().to_string(),
        })
    }

    /// Create an error response (legacy API).
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error(ErrorResponse {
            code: code.into(),
            message: message.into(),
        })
    }

    /// Serialize this response to JSON (legacy API).
    pub fn to_json(&self) -> String {
        match self {
            Self::Health(health) => format!(
                "{{\"ok\":true,\"data\":{{\"service\":\"{}\",\"status\":\"{}\",\"socket_path\":\"{}\"}}}}",
                escape_json(health.service),
                escape_json(health.status),
                escape_json(&health.socket_path),
            ),
            Self::Error(error) => format!(
                "{{\"ok\":false,\"error\":{{\"code\":\"{}\",\"message\":\"{}\"}}}}",
                escape_json(&error.code),
                escape_json(&error.message),
            ),
        }
    }
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// Get the default socket path for the Resurreccion daemon (legacy API).
pub fn default_socket_path() -> std::path::PathBuf {
    std::path::PathBuf::from(DEFAULT_SOCKET_PATH)
}

// ── Skeleton for a daemon client.
///
/// Signatures only — Lane B1 fills in the implementation.
pub struct Client {
    #[allow(dead_code)]
    socket_path: std::path::PathBuf,
}

impl Client {
    /// Connect to the daemon socket at `path`.
    ///
    /// # Errors
    /// Returns an error if the socket is not available.
    pub async fn connect(path: impl Into<std::path::PathBuf>) -> std::io::Result<Self> {
        let socket_path = path.into();
        // Implementation: Lane B1
        let _ = tokio::net::UnixStream::connect(&socket_path).await?;
        Ok(Self { socket_path })
    }

    /// Send a request and await a single response.
    ///
    /// # Errors
    /// Returns an error if the request cannot be sent or the response is malformed.
    #[allow(clippy::unused_async)]
    pub async fn call(&self, _request: Request) -> std::io::Result<Envelope> {
        // Implementation: Lane B1
        unimplemented!("Client::call — implemented by Lane B1")
    }
}
