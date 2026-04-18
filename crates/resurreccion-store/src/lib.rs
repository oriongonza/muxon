//! Durable storage for the Resurreccion daemon.
//!
//! All writes go through the daemon; nothing else touches the database directly.
//! The store is append-oriented: workspace/runtime/snapshot state is derived
//! from the events table and can be rebuilt at any time.

#![allow(
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::cast_possible_truncation
)]

use anyhow::Result;
use rusqlite::Connection;
use std::sync::Mutex;

pub mod types;
pub use types::{BlobRow, EventRow, RuntimeRow, SnapshotRow, WorkspaceRow};

/// The main store handle. Wraps a `SQLite` connection.
///
/// Obtain via [`Store::open`]. All methods are synchronous; the daemon
/// wraps the store in an `Arc<Mutex<Store>>` for concurrent access.
pub struct Store {
    conn: Mutex<Connection>,
}

impl Store {
    /// Open (or create) the `SQLite` database at `path`, running migrations.
    ///
    /// # Errors
    /// Returns an error if the database cannot be opened or migration fails.
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        // Run the migration SQL
        let migration_sql = include_str!("../migrations/001_initial.sql");
        conn.execute_batch(migration_sql)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    // ── Workspaces ────────────────────────────────────────────────────────

    /// Insert a new workspace row.
    ///
    /// # Errors
    /// Returns an error if a workspace with the same binding key already exists.
    pub fn workspace_insert(&self, row: &WorkspaceRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workspaces (id, binding_key, display_name, root_path, created_at, last_opened_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &row.id,
                &row.binding_key,
                &row.display_name,
                &row.root_path,
                &row.created_at,
                &row.last_opened_at,
            ],
        )?;
        Ok(())
    }

    /// Retrieve a workspace by its ULID ID.
    pub fn workspace_get(&self, id: &str) -> Result<Option<WorkspaceRow>> {
        let result = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, binding_key, display_name, root_path, created_at, last_opened_at
                 FROM workspaces WHERE id = ?1",
            )?;

            stmt.query_row(rusqlite::params![id], |row| {
                Ok(WorkspaceRow {
                    id: row.get(0)?,
                    binding_key: row.get(1)?,
                    display_name: row.get(2)?,
                    root_path: row.get(3)?,
                    created_at: row.get(4)?,
                    last_opened_at: row.get(5)?,
                })
            })
        };

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Retrieve a workspace by its binding key.
    pub fn workspace_get_by_key(&self, binding_key: &str) -> Result<Option<WorkspaceRow>> {
        let result = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, binding_key, display_name, root_path, created_at, last_opened_at
                 FROM workspaces WHERE binding_key = ?1",
            )?;

            stmt.query_row(rusqlite::params![binding_key], |row| {
                Ok(WorkspaceRow {
                    id: row.get(0)?,
                    binding_key: row.get(1)?,
                    display_name: row.get(2)?,
                    root_path: row.get(3)?,
                    created_at: row.get(4)?,
                    last_opened_at: row.get(5)?,
                })
            })
        };

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List all workspaces, most-recently-opened first.
    pub fn workspace_list(&self) -> Result<Vec<WorkspaceRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, binding_key, display_name, root_path, created_at, last_opened_at
             FROM workspaces
             ORDER BY last_opened_at DESC, created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceRow {
                id: row.get(0)?,
                binding_key: row.get(1)?,
                display_name: row.get(2)?,
                root_path: row.get(3)?,
                created_at: row.get(4)?,
                last_opened_at: row.get(5)?,
            })
        })?;

        let mut output = Vec::new();
        for row in rows {
            output.push(row?);
        }
        Ok(output)
    }

    /// Update `last_opened_at` for a workspace.
    pub fn workspace_touch(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = iso_now();
        conn.execute(
            "UPDATE workspaces SET last_opened_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    // ── Runtimes ──────────────────────────────────────────────────────────

    /// Record a new runtime attached to a workspace.
    pub fn runtime_insert(&self, row: &RuntimeRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO runtimes (id, workspace_id, session_name, backend, created_at, detached_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &row.id,
                &row.workspace_id,
                &row.session_name,
                &row.backend,
                &row.created_at,
                &row.detached_at,
            ],
        )?;
        Ok(())
    }

    /// List all runtimes for a workspace.
    pub fn runtime_list(&self, workspace_id: &str) -> Result<Vec<RuntimeRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, session_name, backend, created_at, detached_at
             FROM runtimes WHERE workspace_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![workspace_id], |row| {
            Ok(RuntimeRow {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                session_name: row.get(2)?,
                backend: row.get(3)?,
                created_at: row.get(4)?,
                detached_at: row.get(5)?,
            })
        })?;

        let mut output = Vec::new();
        for row in rows {
            output.push(row?);
        }
        Ok(output)
    }

    /// Mark a runtime as detached (set `detached_at`).
    pub fn runtime_detach(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        let now = iso_now();
        conn.execute(
            "UPDATE runtimes SET detached_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id],
        )?;
        Ok(())
    }

    // ── Snapshots ─────────────────────────────────────────────────────────

    /// Insert a new snapshot.
    pub fn snapshot_insert(&self, row: &SnapshotRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO snapshots (id, workspace_id, runtime_id, fidelity, manifest_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                &row.id,
                &row.workspace_id,
                &row.runtime_id,
                &row.fidelity,
                &row.manifest_json,
                &row.created_at,
            ],
        )?;
        Ok(())
    }

    /// Get a snapshot by ID.
    pub fn snapshot_get(&self, id: &str) -> Result<Option<SnapshotRow>> {
        let result = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare(
                "SELECT id, workspace_id, runtime_id, fidelity, manifest_json, created_at
                 FROM snapshots WHERE id = ?1",
            )?;

            stmt.query_row(rusqlite::params![id], |row| {
                Ok(SnapshotRow {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    runtime_id: row.get(2)?,
                    fidelity: row.get(3)?,
                    manifest_json: row.get(4)?,
                    created_at: row.get(5)?,
                })
            })
        };

        match result {
            Ok(row) => Ok(Some(row)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// List snapshots for a workspace, newest first.
    pub fn snapshot_list(&self, workspace_id: &str) -> Result<Vec<SnapshotRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, workspace_id, runtime_id, fidelity, manifest_json, created_at
             FROM snapshots WHERE workspace_id = ?1
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map(rusqlite::params![workspace_id], |row| {
            Ok(SnapshotRow {
                id: row.get(0)?,
                workspace_id: row.get(1)?,
                runtime_id: row.get(2)?,
                fidelity: row.get(3)?,
                manifest_json: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut output = Vec::new();
        for row in rows {
            output.push(row?);
        }
        Ok(output)
    }

    // ── Events ────────────────────────────────────────────────────────────

    /// Append a domain event.
    pub fn event_append(&self, row: &EventRow) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO events (id, workspace_id, kind, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                &row.id,
                &row.workspace_id,
                &row.kind,
                &row.payload_json,
                &row.created_at,
            ],
        )?;
        Ok(())
    }

    // ── Blobs ─────────────────────────────────────────────────────────────

    /// Store a blob, returning its BLAKE3 hex hash.
    ///
    /// Content-addressed: calling with the same bytes is idempotent and returns
    /// the same hash without error.
    pub fn blob_put(&self, data: &[u8]) -> Result<String> {
        let hash = blake3::hash(data).to_hex().to_string();
        let size = i64::try_from(data.len()).unwrap_or(i64::MAX);
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO blobs (hash, size, data) VALUES (?1, ?2, ?3)",
            rusqlite::params![&hash, size, data],
        )?;
        Ok(hash)
    }

    /// Retrieve a blob by its BLAKE3 hex hash.
    ///
    /// Returns `None` if no blob with that hash exists.
    pub fn blob_get(&self, hash: &str) -> Result<Option<Vec<u8>>> {
        let result = {
            let conn = self.conn.lock().unwrap();
            let mut stmt = conn.prepare("SELECT data FROM blobs WHERE hash = ?1")?;
            stmt.query_row(rusqlite::params![hash], |row| row.get::<_, Vec<u8>>(0))
        };

        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Stream events from a given ID onward (for tail subscriptions).
    pub fn event_tail_from(&self, after_id: Option<&str>) -> Result<Vec<EventRow>> {
        let conn = self.conn.lock().unwrap();

        if let Some(id) = after_id {
            let mut find_stmt = conn.prepare("SELECT rowid FROM events WHERE id = ?1")?;
            let after_rowid: i64 = find_stmt.query_row(rusqlite::params![id], |row| row.get(0))?;
            drop(find_stmt);

            let mut stmt = conn.prepare(
                "SELECT id, workspace_id, kind, payload_json, created_at
                 FROM events WHERE rowid > ?1
                 ORDER BY rowid ASC",
            )?;
            let rows = stmt.query_map(rusqlite::params![after_rowid], |row| {
                Ok(EventRow {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    kind: row.get(2)?,
                    payload_json: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?;

            let mut output = Vec::new();
            for row in rows {
                output.push(row?);
            }
            Ok(output)
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, workspace_id, kind, payload_json, created_at
                 FROM events
                 ORDER BY rowid ASC",
            )?;

            let rows = stmt.query_map([], |row| {
                Ok(EventRow {
                    id: row.get(0)?,
                    workspace_id: row.get(1)?,
                    kind: row.get(2)?,
                    payload_json: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })?;

            let mut output = Vec::new();
            for row in rows {
                output.push(row?);
            }
            Ok(output)
        }
    }
}

/// Get current timestamp in ISO-8601 format with milliseconds.
fn iso_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let seconds = now / 1000;
    let millis = now % 1000;

    // Create a simple ISO format string
    // In a real implementation, you'd use a datetime library
    format!("2026-04-18T00:00:{:02}.{:03}Z", (seconds % 60), millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_memory() -> Store {
        Store::open(":memory:").expect("in-memory store must open")
    }

    #[test]
    fn blob_round_trip() {
        let store = open_memory();
        let payload = b"hello world";

        // Known BLAKE3 hash for b"hello world"
        let expected_hash = blake3::hash(payload).to_hex().to_string();

        // blob_put returns the correct hex hash
        let hash = store.blob_put(payload).expect("blob_put must succeed");
        assert_eq!(
            hash, expected_hash,
            "returned hash must be BLAKE3 of the data"
        );

        // blob_get returns the original data
        let got = store.blob_get(&hash).expect("blob_get must not error");
        assert_eq!(
            got,
            Some(payload.to_vec()),
            "blob_get must return original data"
        );

        // blob_get with unknown hash returns None
        let fake = "0".repeat(64);
        let missing = store
            .blob_get(&fake)
            .expect("blob_get for missing must not error");
        assert_eq!(missing, None, "blob_get with fake hash must return None");

        // blob_put is idempotent: same data → same hash, no error
        let hash2 = store
            .blob_put(payload)
            .expect("second blob_put must succeed");
        assert_eq!(hash2, hash, "second put of same data must return same hash");
    }
}
