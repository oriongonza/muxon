#![allow(
    missing_docs,
    clippy::ignored_unit_patterns,
    clippy::significant_drop_in_scrutinee,
    clippy::redundant_pattern_matching,
    clippy::missing_const_for_fn,
    clippy::items_after_statements,
    clippy::redundant_clone
)]

use resurreccion_daemon::Dispatcher;
use resurreccion_mux::{
    Capability, LayoutCapture, LayoutSpec, Mux, MuxError, PaneSpec, TopologyEvent,
};
use resurreccion_proto::{verbs, Envelope};
use resurreccion_store::Store;
use std::sync::Arc;
use std::sync::Mutex;
use tempfile::TempDir;

// Helper to create a test store in a temp directory
fn create_test_store() -> (TempDir, Arc<Mutex<Store>>) {
    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let store = Store::open(db_path.to_str().unwrap()).expect("failed to open store");
    (temp_dir, Arc::new(Mutex::new(store)))
}

// Minimal test Mux stub implementation
struct TestMux;

impl Mux for TestMux {
    fn discover(&self) -> Result<Vec<String>, MuxError> {
        Ok(vec!["test-session".to_string()])
    }

    fn create(&self, _session_name: &str, _layout: &LayoutSpec) -> Result<(), MuxError> {
        Ok(())
    }

    fn attach(&self, _session_name: &str) -> Result<(), MuxError> {
        Ok(())
    }

    fn capture(&self, _session_name: &str) -> Result<LayoutCapture, MuxError> {
        Ok(LayoutCapture {
            session_name: "test-session".to_string(),
            panes: vec![PaneSpec {
                id: "pane-1".to_string(),
                cwd: Some("/tmp".to_string()),
                title: Some("shell".to_string()),
            }],
            tabs: vec!["tab-1".to_string()],
            capabilities: Capability::empty(),
        })
    }

    fn apply_layout(&self, _session_name: &str, _layout: &LayoutSpec) -> Result<(), MuxError> {
        Ok(())
    }

    fn send_keys(&self, _session_name: &str, _keys: &str) -> Result<(), MuxError> {
        Ok(())
    }

    fn subscribe_topology(
        &self,
        _session_name: &str,
    ) -> Result<std::sync::mpsc::Receiver<TopologyEvent>, MuxError> {
        let (_tx, rx) = std::sync::mpsc::channel();
        Ok(rx)
    }

    fn capabilities(&self) -> Capability {
        Capability::empty()
    }
}

#[test]
fn snapshot_create_and_list() {
    let (_temp, store) = create_test_store();
    let mux = Arc::new(TestMux) as Arc<dyn Mux>;

    // First, create a workspace
    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_CREATE,
        Arc::new(resurreccion_daemon::WorkspaceCreateHandler::new(
            store.clone(),
        )),
    );
    dispatcher.register(
        verbs::SNAPSHOT_CREATE,
        Arc::new(resurreccion_daemon::SnapshotCreateHandler::new(
            store.clone(),
            mux.clone(),
        )),
    );
    dispatcher.register(
        verbs::SNAPSHOT_LIST,
        Arc::new(resurreccion_daemon::SnapshotListHandler::new(store.clone())),
    );

    let create_ws_req = Envelope::ok(
        "create-ws",
        verbs::WORKSPACE_CREATE,
        serde_json::json!({
            "display_name": "test-workspace",
            "root_path": "/tmp/test",
            "binding_key": "test-key"
        }),
    );
    let create_ws_resp = dispatcher.dispatch(&create_ws_req);
    assert!(create_ws_resp.ok, "workspace creation failed");

    let workspace_id = create_ws_resp
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have workspace id");

    // Create a snapshot
    let create_snap_req = Envelope::ok(
        "create-snap",
        verbs::SNAPSHOT_CREATE,
        serde_json::json!({
            "workspace_id": workspace_id,
        }),
    );
    let create_snap_resp = dispatcher.dispatch(&create_snap_req);
    assert!(
        create_snap_resp.ok,
        "snapshot creation failed: {}",
        create_snap_resp.body
    );

    let snapshot_id = create_snap_resp
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have snapshot id");

    // List snapshots for the workspace
    let list_req = Envelope::ok(
        "list-snaps",
        verbs::SNAPSHOT_LIST,
        serde_json::json!({
            "workspace_id": workspace_id,
        }),
    );
    let list_resp = dispatcher.dispatch(&list_req);
    assert!(list_resp.ok, "snapshot list failed");

    let snapshots = list_resp.body.as_array().expect("body should be array");
    assert_eq!(snapshots.len(), 1, "should have one snapshot");
    assert_eq!(
        snapshots[0]
            .get("id")
            .and_then(|v| v.as_str())
            .expect("snapshot should have id"),
        snapshot_id
    );
}

#[test]
fn snapshot_get_returns_row() {
    let (_temp, store) = create_test_store();
    let mux = Arc::new(TestMux) as Arc<dyn Mux>;

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_CREATE,
        Arc::new(resurreccion_daemon::WorkspaceCreateHandler::new(
            store.clone(),
        )),
    );
    dispatcher.register(
        verbs::SNAPSHOT_CREATE,
        Arc::new(resurreccion_daemon::SnapshotCreateHandler::new(
            store.clone(),
            mux.clone(),
        )),
    );
    dispatcher.register(
        verbs::SNAPSHOT_GET,
        Arc::new(resurreccion_daemon::SnapshotGetHandler::new(store.clone())),
    );

    // Create a workspace
    let create_ws_req = Envelope::ok(
        "create-ws",
        verbs::WORKSPACE_CREATE,
        serde_json::json!({
            "display_name": "test-workspace",
            "root_path": "/tmp/test",
            "binding_key": "test-key"
        }),
    );
    let create_ws_resp = dispatcher.dispatch(&create_ws_req);
    assert!(create_ws_resp.ok);

    let workspace_id = create_ws_resp
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have workspace id");

    // Create a snapshot
    let create_snap_req = Envelope::ok(
        "create-snap",
        verbs::SNAPSHOT_CREATE,
        serde_json::json!({
            "workspace_id": workspace_id,
        }),
    );
    let create_snap_resp = dispatcher.dispatch(&create_snap_req);
    assert!(create_snap_resp.ok);

    let snapshot_id = create_snap_resp
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have snapshot id");

    // Get the snapshot
    let get_req = Envelope::ok(
        "get-snap",
        verbs::SNAPSHOT_GET,
        serde_json::json!({"id": snapshot_id}),
    );
    let get_resp = dispatcher.dispatch(&get_req);
    assert!(get_resp.ok, "get snapshot failed");

    assert_eq!(
        get_resp
            .body
            .get("id")
            .and_then(|v| v.as_str())
            .expect("should have id"),
        snapshot_id
    );
    assert_eq!(
        get_resp
            .body
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .expect("should have workspace_id"),
        workspace_id
    );
    assert_eq!(
        get_resp
            .body
            .get("fidelity")
            .and_then(|v| v.as_str())
            .expect("should have fidelity"),
        "basic"
    );
}

#[test]
fn events_tail_returns_events() {
    let (_temp, store) = create_test_store();

    // First create a workspace
    let workspace = resurreccion_store::types::WorkspaceRow {
        id: "test-ws".to_string(),
        binding_key: "test-key".to_string(),
        display_name: "test".to_string(),
        root_path: "/tmp".to_string(),
        created_at: "2026-04-18T00:00:00Z".to_string(),
        last_opened_at: None,
    };
    store
        .lock()
        .unwrap()
        .workspace_insert(&workspace)
        .expect("failed to insert workspace");

    // Create test event
    let event = resurreccion_store::types::EventRow {
        id: ulid::Ulid::new().to_string(),
        workspace_id: Some("test-ws".to_string()),
        kind: "test-event".to_string(),
        payload_json: serde_json::json!({"test": "data"}).to_string(),
        created_at: "2026-04-18T00:00:00Z".to_string(),
    };

    store
        .lock()
        .unwrap()
        .event_append(&event)
        .expect("failed to append event");

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::EVENTS_TAIL,
        Arc::new(resurreccion_daemon::EventsTailHandler::new(store.clone())),
    );

    // Tail events with no filter
    let tail_req = Envelope::ok(
        "tail-events",
        verbs::EVENTS_TAIL,
        serde_json::json!({"after_id": null}),
    );
    let tail_resp = dispatcher.dispatch(&tail_req);
    assert!(tail_resp.ok, "events.tail failed");

    let events = tail_resp.body.as_array().expect("body should be array");
    assert_eq!(events.len(), 1, "should have one event");
    assert_eq!(
        events[0]
            .get("kind")
            .and_then(|v| v.as_str())
            .expect("should have kind"),
        "test-event"
    );
}
