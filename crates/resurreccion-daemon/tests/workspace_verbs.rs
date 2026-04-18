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

#[test]
fn workspace_list_handler_returns_empty_list() {
    let (_temp, store) = create_test_store();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_LIST,
        Arc::new(resurreccion_daemon::WorkspaceListHandler::new(
            store.clone(),
        )),
    );

    let request = Envelope::ok("test-id", verbs::WORKSPACE_LIST, serde_json::json!(null));
    let response = dispatcher.dispatch(&request);

    assert!(response.ok, "response should be ok");
    assert_eq!(response.verb, verbs::WORKSPACE_LIST);
    assert_eq!(response.id, "test-id");
    assert!(
        response.body.is_array(),
        "body should be an array, got: {:?}",
        response.body
    );
    assert_eq!(
        response.body.as_array().unwrap().len(),
        0,
        "empty store should return empty list"
    );
}

#[test]
fn workspace_create_then_get() {
    let (_temp, store) = create_test_store();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_CREATE,
        Arc::new(resurreccion_daemon::WorkspaceCreateHandler::new(
            store.clone(),
        )),
    );
    dispatcher.register(
        verbs::WORKSPACE_GET,
        Arc::new(resurreccion_daemon::WorkspaceGetHandler::new(store.clone())),
    );

    // Create workspace
    let create_req = Envelope::ok(
        "test-id-1",
        verbs::WORKSPACE_CREATE,
        serde_json::json!({
            "display_name": "test-ws",
            "root_path": "/tmp/test-path",
            "binding_key": "test-binding-key"
        }),
    );
    let create_resp = dispatcher.dispatch(&create_req);
    assert!(create_resp.ok, "create should succeed");

    let workspace_id = create_resp
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have id in response");

    // Get workspace
    let get_req = Envelope::ok(
        "test-id-2",
        verbs::WORKSPACE_GET,
        serde_json::json!({"id": workspace_id}),
    );
    let get_resp = dispatcher.dispatch(&get_req);
    assert!(get_resp.ok, "get should succeed");

    assert_eq!(
        get_resp.body.get("display_name").and_then(|v| v.as_str()),
        Some("test-ws")
    );
    assert_eq!(
        get_resp.body.get("root_path").and_then(|v| v.as_str()),
        Some("/tmp/test-path")
    );
}

#[test]
fn workspace_resolve_or_create_is_idempotent() {
    let (_temp, store) = create_test_store();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_RESOLVE_OR_CREATE,
        Arc::new(resurreccion_daemon::WorkspaceResolveOrCreateHandler::new(
            store.clone(),
        )),
    );

    // First call
    let req1 = Envelope::ok(
        "test-id-1",
        verbs::WORKSPACE_RESOLVE_OR_CREATE,
        serde_json::json!({
            "binding_key": "my-binding-key",
            "root_path": "/tmp/workspace",
            "display_name": "my-workspace"
        }),
    );
    let resp1 = dispatcher.dispatch(&req1);
    assert!(resp1.ok);
    let ws_id_1 = resp1
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have id");

    // Second call with same binding key
    let req2 = Envelope::ok(
        "test-id-2",
        verbs::WORKSPACE_RESOLVE_OR_CREATE,
        serde_json::json!({
            "binding_key": "my-binding-key",
            "root_path": "/tmp/workspace",
            "display_name": "my-workspace"
        }),
    );
    let resp2 = dispatcher.dispatch(&req2);
    assert!(resp2.ok);
    let ws_id_2 = resp2
        .body
        .get("id")
        .and_then(|v| v.as_str())
        .expect("should have id");

    assert_eq!(ws_id_1, ws_id_2, "should return same workspace id");
}

#[test]
#[ignore] // git2::Repository::discover can hang on some systems; integration tests cover this
fn workspace_open_returns_workspace_row() {
    let (_temp, store) = create_test_store();
    let work_dir = TempDir::new().expect("failed to create temp dir");
    let work_path = work_dir.path().to_string_lossy().to_string();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_OPEN,
        Arc::new(resurreccion_daemon::WorkspaceOpenHandler::new(
            store.clone(),
        )),
    );

    let req = Envelope::ok(
        "test-id",
        verbs::WORKSPACE_OPEN,
        serde_json::json!({"path": work_path}),
    );
    let resp = dispatcher.dispatch(&req);

    assert!(resp.ok);
    assert!(resp.body.get("id").is_some());
    assert!(resp.body.get("binding_key").is_some());
}
