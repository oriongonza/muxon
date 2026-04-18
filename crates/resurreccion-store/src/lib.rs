//! Durable storage for the Resurreccion daemon.
//!
//! All writes go through the daemon; nothing else touches the database directly.
//! The store is append-oriented: workspace/runtime/snapshot state is derived
//! from the events table and can be rebuilt at any time.
//!
//! # Implementation note
//! Method bodies are stubs (`unimplemented!`). Lane A fills them in.

use anyhow::Result;

pub mod types;
pub use types::{EventRow, RuntimeRow, SnapshotRow, WorkspaceRow};

/// The main store handle. Wraps a `SQLite` connection.
///
/// Obtain via [`Store::open`]. All methods are synchronous; the daemon
/// wraps the store in an `Arc<Mutex<Store>>` for concurrent access.
pub struct Store {
    #[allow(dead_code)]
    path: std::path::PathBuf,
    // conn: rusqlite::Connection  — Lane A adds this field
}

impl Store {
    /// Open (or create) the `SQLite` database at `path`, running migrations.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or migration fails.
    pub fn open(_path: &str) -> Result<Self> {
        // Lane A: run migrations/001_initial.sql and return a live connection
        unimplemented!("Store::open — implemented by Lane A")
    }

    // ── Workspaces ────────────────────────────────────────────────────────

    /// Insert a new workspace row.
    ///
    /// # Errors
    /// Returns an error if a workspace with the same binding key already exists.
    pub fn workspace_insert(&self, _row: &WorkspaceRow) -> Result<()> {
        unimplemented!("Lane A")
    }

    /// Retrieve a workspace by its ULID ID.
    pub fn workspace_get(&self, _id: &str) -> Result<Option<WorkspaceRow>> {
        unimplemented!("Lane A")
    }

    /// Retrieve a workspace by its binding key.
    pub fn workspace_get_by_key(&self, _binding_key: &str) -> Result<Option<WorkspaceRow>> {
        unimplemented!("Lane A")
    }

    /// List all workspaces, most-recently-opened first.
    pub fn workspace_list(&self) -> Result<Vec<WorkspaceRow>> {
        unimplemented!("Lane A")
    }

    /// Update `last_opened_at` for a workspace.
    pub fn workspace_touch(&self, _id: &str) -> Result<()> {
        unimplemented!("Lane A")
    }

    // ── Runtimes ──────────────────────────────────────────────────────────

    /// Record a new runtime attached to a workspace.
    pub fn runtime_insert(&self, _row: &RuntimeRow) -> Result<()> {
        unimplemented!("Lane A")
    }

    /// List all runtimes for a workspace.
    pub fn runtime_list(&self, _workspace_id: &str) -> Result<Vec<RuntimeRow>> {
        unimplemented!("Lane A")
    }

    /// Mark a runtime as detached (set `detached_at`).
    pub fn runtime_detach(&self, _id: &str) -> Result<()> {
        unimplemented!("Lane A")
    }

    // ── Snapshots ─────────────────────────────────────────────────────────

    /// Insert a new snapshot.
    pub fn snapshot_insert(&self, _row: &SnapshotRow) -> Result<()> {
        unimplemented!("Lane A")
    }

    /// Get a snapshot by ID.
    pub fn snapshot_get(&self, _id: &str) -> Result<Option<SnapshotRow>> {
        unimplemented!("Lane A")
    }

    /// List snapshots for a workspace, newest first.
    pub fn snapshot_list(&self, _workspace_id: &str) -> Result<Vec<SnapshotRow>> {
        unimplemented!("Lane A")
    }

    // ── Events ────────────────────────────────────────────────────────────

    /// Append a domain event.
    pub fn event_append(&self, _row: &EventRow) -> Result<()> {
        unimplemented!("Lane A")
    }

    /// Stream events from a given ID onward (for tail subscriptions).
    pub fn event_tail_from(&self, _after_id: Option<&str>) -> Result<Vec<EventRow>> {
        unimplemented!("Lane A")
    }
}
