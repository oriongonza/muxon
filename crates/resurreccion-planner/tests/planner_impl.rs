//! Test suite for planner implementation (Lane F).

use resurreccion_planner::{
    execute, plan_capture, plan_restore, verbs, Capability, Plan, SnapshotManifest, Store,
};
use tempfile::TempDir;

/// Test: `plan_capture` returns a single node with `CAPTURE_LAYOUT` verb.
#[test]
fn test_plan_capture_returns_single_node() {
    let cap = Capability::empty();
    let plan = plan_capture(&cap);

    assert_eq!(plan.nodes.len(), 1, "plan should have exactly one node");
    assert_eq!(
        plan.nodes[0].verb,
        verbs::CAPTURE_LAYOUT,
        "node verb should be CAPTURE_LAYOUT"
    );
    assert_eq!(
        plan.nodes[0].depends_on.len(),
        0,
        "CAPTURE_LAYOUT node should have no dependencies"
    );
}

/// Test: `plan_restore` returns a single node with `RESTORE_LAYOUT` verb.
#[test]
fn test_plan_restore_returns_single_node() {
    let manifest = SnapshotManifest {
        workspace_id: "test-workspace".to_string(),
        layout: serde_json::json!({}),
    };
    let cap = Capability::empty();
    let plan = plan_restore(&manifest, &cap);

    assert_eq!(plan.nodes.len(), 1, "plan should have exactly one node");
    assert_eq!(
        plan.nodes[0].verb,
        verbs::RESTORE_LAYOUT,
        "node verb should be RESTORE_LAYOUT"
    );
    assert_eq!(
        plan.nodes[0].depends_on.len(),
        0,
        "RESTORE_LAYOUT node should have no dependencies"
    );
}

/// Test: `dry_run` plan returns empty result without executing.
#[test]
fn test_dry_run_plan_returns_without_executing() {
    let plan = Plan::empty().as_dry_run();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = Store::open(db_path.to_str().unwrap()).unwrap();

    // For dry_run plans, execute should return an empty result without calling mux/store.
    let result = execute(&plan, &MockMux, &store);
    assert!(result.is_ok(), "execute should succeed on dry_run plan");

    let result = result.unwrap();
    assert_eq!(
        result.node_results.len(),
        0,
        "dry_run plan should return empty node results"
    );
}

/// Test: `empty` plan executes successfully.
#[test]
fn test_empty_plan_executes_successfully() {
    let plan = Plan::empty();
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let store = Store::open(db_path.to_str().unwrap()).unwrap();

    let result = execute(&plan, &MockMux, &store);
    assert!(result.is_ok(), "execute should succeed on empty plan");

    let result = result.unwrap();
    assert_eq!(
        result.node_results.len(),
        0,
        "empty plan should return empty node results"
    );
}

// ── Mocks for testing ──────────────────────────────────────────────

struct MockMux;

impl resurreccion_mux::Mux for MockMux {
    fn discover(&self) -> Result<Vec<String>, resurreccion_mux::MuxError> {
        Ok(vec![])
    }

    fn create(
        &self,
        _session_name: &str,
        _layout: &resurreccion_mux::LayoutSpec,
    ) -> Result<(), resurreccion_mux::MuxError> {
        Ok(())
    }

    fn attach(&self, _session_name: &str) -> Result<(), resurreccion_mux::MuxError> {
        Ok(())
    }

    fn capture(
        &self,
        _session_name: &str,
    ) -> Result<resurreccion_mux::LayoutCapture, resurreccion_mux::MuxError> {
        Ok(resurreccion_mux::LayoutCapture {
            session_name: "test".to_string(),
            panes: vec![],
            tabs: vec![],
            capabilities: Capability::empty(),
        })
    }

    fn apply_layout(
        &self,
        _session_name: &str,
        _layout: &resurreccion_mux::LayoutSpec,
    ) -> Result<(), resurreccion_mux::MuxError> {
        Ok(())
    }

    fn send_keys(
        &self,
        _session_name: &str,
        _keys: &str,
    ) -> Result<(), resurreccion_mux::MuxError> {
        Ok(())
    }

    fn subscribe_topology(
        &self,
        _session_name: &str,
    ) -> Result<
        std::sync::mpsc::Receiver<resurreccion_mux::TopologyEvent>,
        resurreccion_mux::MuxError,
    > {
        let (_tx, rx) = std::sync::mpsc::channel();
        Ok(rx)
    }

    fn capabilities(&self) -> Capability {
        Capability::empty()
    }
}
