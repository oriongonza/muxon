//! Row types matching the SQL schema in `migrations/001_initial.sql`.

use serde::{Deserialize, Serialize};

/// A workspace row from the `workspaces` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRow {
    /// ULID identifier.
    pub id: String,
    /// BLAKE3 hex binding key.
    pub binding_key: String,
    /// Human-readable name (usually the directory basename).
    pub display_name: String,
    /// Canonical filesystem path.
    pub root_path: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 timestamp of last open, if any.
    pub last_opened_at: Option<String>,
}

/// A runtime row from the `runtimes` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRow {
    /// ULID identifier.
    pub id: String,
    /// Owning workspace ULID.
    pub workspace_id: String,
    /// Multiplexer session name.
    pub session_name: String,
    /// Backend name (e.g. `"zellij"`).
    pub backend: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// ISO-8601 detach timestamp, if detached.
    pub detached_at: Option<String>,
}

/// A snapshot row from the `snapshots` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRow {
    /// ULID identifier.
    pub id: String,
    /// Owning workspace ULID.
    pub workspace_id: String,
    /// Owning runtime ULID, if any.
    pub runtime_id: Option<String>,
    /// Fidelity level string.
    pub fidelity: String,
    /// JSON-encoded manifest describing what was captured.
    pub manifest_json: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}

/// A blob row from the `blobs` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRow {
    /// BLAKE3 hex hash (content-addressed primary key).
    pub hash: String,
    /// Size in bytes.
    pub size: i64,
    /// Raw blob data.
    pub data: Vec<u8>,
}

/// An event row from the `events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    /// ULID identifier (time-sortable).
    pub id: String,
    /// Associated workspace ULID, if any.
    pub workspace_id: Option<String>,
    /// Event kind discriminator string.
    pub kind: String,
    /// JSON-encoded event payload.
    pub payload_json: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
}
