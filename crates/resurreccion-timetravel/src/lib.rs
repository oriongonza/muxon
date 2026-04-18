//! Time-travel snapshot browsing for Resurreccion.
//!
//! Provides a pure query layer for browsing snapshot history stored in
//! the [`resurreccion_store::Store`]. No UI — callers supply a live store
//! reference and receive typed [`SnapshotPoint`] values back.

use anyhow::Result;
use resurreccion_store::{SnapshotRow, Store};

/// A point-in-time view of workspace state.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotPoint {
    /// ULID identifier for this snapshot.
    pub snapshot_id: String,
    /// Owning workspace ULID.
    pub workspace_id: String,
    /// ISO-8601 creation timestamp.
    pub created_at: String,
    /// Fidelity level string.
    pub fidelity: String,
    /// Decoded manifest (null if the stored JSON was malformed).
    pub manifest: serde_json::Value,
}

impl SnapshotPoint {
    fn from_row(r: SnapshotRow) -> Self {
        Self {
            snapshot_id: r.id,
            workspace_id: r.workspace_id,
            created_at: r.created_at,
            fidelity: r.fidelity,
            manifest: serde_json::from_str(&r.manifest_json).unwrap_or(serde_json::Value::Null),
        }
    }
}

/// Query interface for snapshot history.
pub struct TimeTravelQuery<'a> {
    store: &'a Store,
}

impl<'a> TimeTravelQuery<'a> {
    /// Wrap a store reference with the time-travel query API.
    pub const fn new(store: &'a Store) -> Self {
        Self { store }
    }

    /// List all snapshots for a workspace, newest first.
    pub fn list_snapshots(&self, workspace_id: &str) -> Result<Vec<SnapshotPoint>> {
        let rows = self.store.snapshot_list(workspace_id)?;
        Ok(rows.into_iter().map(SnapshotPoint::from_row).collect())
    }

    /// Get a specific snapshot by ID.
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<SnapshotPoint>> {
        Ok(self
            .store
            .snapshot_get(snapshot_id)?
            .map(SnapshotPoint::from_row))
    }

    /// Return the N most recent snapshots for a workspace.
    pub fn latest_n(&self, workspace_id: &str, n: usize) -> Result<Vec<SnapshotPoint>> {
        Ok(self
            .list_snapshots(workspace_id)?
            .into_iter()
            .take(n)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_store::{SnapshotRow, Store, WorkspaceRow};

    fn make_store() -> Store {
        Store::open(":memory:").expect("in-memory store")
    }

    /// Insert a minimal workspace so foreign-key constraints are satisfied.
    fn insert_workspace(store: &Store, workspace_id: &str) {
        store
            .workspace_insert(&WorkspaceRow {
                id: workspace_id.to_string(),
                binding_key: format!("key-{workspace_id}"),
                display_name: workspace_id.to_string(),
                root_path: "/tmp/test".to_string(),
                created_at: "2026-04-18T00:00:00.000Z".to_string(),
                last_opened_at: None,
            })
            .expect("insert workspace");
    }

    fn insert_snapshot(store: &Store, id: &str, workspace_id: &str, created_at: &str) {
        store
            .snapshot_insert(&SnapshotRow {
                id: id.to_string(),
                workspace_id: workspace_id.to_string(),
                runtime_id: None,
                fidelity: "full".to_string(),
                manifest_json: r#"{"panes":1}"#.to_string(),
                created_at: created_at.to_string(),
            })
            .expect("insert snapshot");
    }

    #[test]
    fn list_snapshots_newest_first() {
        let store = make_store();
        insert_workspace(&store, "ws-1");
        insert_snapshot(&store, "snap-a", "ws-1", "2026-04-18T00:00:01.000Z");
        insert_snapshot(&store, "snap-b", "ws-1", "2026-04-18T00:00:02.000Z");
        insert_snapshot(&store, "snap-c", "ws-1", "2026-04-18T00:00:03.000Z");

        let q = TimeTravelQuery::new(&store);
        let points = q.list_snapshots("ws-1").expect("list");

        assert_eq!(points.len(), 3);
        // Newest (snap-c) must come first.
        assert_eq!(points[0].snapshot_id, "snap-c");
        assert_eq!(points[1].snapshot_id, "snap-b");
        assert_eq!(points[2].snapshot_id, "snap-a");
    }

    #[test]
    fn list_snapshots_only_own_workspace() {
        let store = make_store();
        insert_workspace(&store, "ws-2");
        insert_workspace(&store, "ws-3");
        insert_snapshot(&store, "snap-x", "ws-2", "2026-04-18T00:00:01.000Z");
        insert_snapshot(&store, "snap-y", "ws-3", "2026-04-18T00:00:01.000Z");

        let q = TimeTravelQuery::new(&store);
        let points = q.list_snapshots("ws-2").expect("list");

        assert_eq!(points.len(), 1);
        assert_eq!(points[0].snapshot_id, "snap-x");
    }

    #[test]
    fn get_snapshot_returns_one() {
        let store = make_store();
        insert_workspace(&store, "ws-1");
        insert_snapshot(&store, "snap-d", "ws-1", "2026-04-18T00:00:01.000Z");

        let q = TimeTravelQuery::new(&store);
        let point = q.get_snapshot("snap-d").expect("get").expect("some");

        assert_eq!(point.snapshot_id, "snap-d");
        assert_eq!(point.workspace_id, "ws-1");
        assert_eq!(point.fidelity, "full");
        assert_eq!(point.manifest, serde_json::json!({"panes": 1}));
    }

    #[test]
    fn get_snapshot_missing_returns_none() {
        let store = make_store();
        let q = TimeTravelQuery::new(&store);
        let result = q.get_snapshot("no-such-id").expect("no error");
        assert!(result.is_none());
    }

    #[test]
    fn latest_n_limits_count() {
        let store = make_store();
        insert_workspace(&store, "ws-4");
        insert_snapshot(&store, "snap-1", "ws-4", "2026-04-18T00:00:01.000Z");
        insert_snapshot(&store, "snap-2", "ws-4", "2026-04-18T00:00:02.000Z");
        insert_snapshot(&store, "snap-3", "ws-4", "2026-04-18T00:00:03.000Z");
        insert_snapshot(&store, "snap-4", "ws-4", "2026-04-18T00:00:04.000Z");

        let q = TimeTravelQuery::new(&store);
        let top2 = q.latest_n("ws-4", 2).expect("latest_n");

        assert_eq!(top2.len(), 2);
        // Should be the two newest.
        assert_eq!(top2[0].snapshot_id, "snap-4");
        assert_eq!(top2[1].snapshot_id, "snap-3");
    }

    #[test]
    fn latest_n_zero_returns_empty() {
        let store = make_store();
        insert_workspace(&store, "ws-5");
        insert_snapshot(&store, "snap-e", "ws-5", "2026-04-18T00:00:01.000Z");

        let q = TimeTravelQuery::new(&store);
        let empty = q.latest_n("ws-5", 0).expect("latest_n");

        assert!(empty.is_empty());
    }

    #[test]
    fn manifest_null_on_malformed_json() {
        let store = make_store();
        insert_workspace(&store, "ws-6");
        store
            .snapshot_insert(&SnapshotRow {
                id: "snap-bad".to_string(),
                workspace_id: "ws-6".to_string(),
                runtime_id: None,
                fidelity: "full".to_string(),
                manifest_json: "not-valid-json".to_string(),
                created_at: "2026-04-18T00:00:01.000Z".to_string(),
            })
            .expect("insert");

        let q = TimeTravelQuery::new(&store);
        let point = q.get_snapshot("snap-bad").expect("get").expect("some");
        assert_eq!(point.manifest, serde_json::Value::Null);
    }
}
