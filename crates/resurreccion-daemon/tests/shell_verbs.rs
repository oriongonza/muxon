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
use resurreccion_shell::ShellCapture;
use resurreccion_store::Store;
use std::collections::HashMap;
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
fn shell_capture_stores_event() {
    let (_temp, store) = create_test_store();

    // Create a workspace first (required by foreign key constraint)
    let workspace_row = resurreccion_store::types::WorkspaceRow {
        id: "test-workspace-1".to_string(),
        binding_key: "test-key-1".to_string(),
        display_name: "Test Workspace 1".to_string(),
        root_path: "/tmp/test-1".to_string(),
        created_at: "2026-04-18T12:00:00.000Z".to_string(),
        last_opened_at: None,
    };
    store
        .lock()
        .unwrap()
        .workspace_insert(&workspace_row)
        .expect("failed to create workspace");

    // Create dispatcher with shell capture handler
    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::SHELL_CAPTURE,
        Arc::new(resurreccion_daemon::ShellCaptureHandler::new(store.clone())),
    );

    // Send capture request with empty pids
    let capture_req = Envelope::ok(
        "capture-1",
        verbs::SHELL_CAPTURE,
        serde_json::json!({
            "workspace_id": "test-workspace-1",
            "pids": []
        }),
    );

    let capture_resp = dispatcher.dispatch(&capture_req);
    assert!(capture_resp.ok, "shell capture failed");
    assert_eq!(
        capture_resp
            .body
            .get("count")
            .and_then(|v| v.as_u64())
            .unwrap_or(999),
        0,
        "should have captured 0 shells"
    );

    // Verify event was stored
    let events = store
        .lock()
        .unwrap()
        .event_tail_from(None)
        .expect("failed to read events");

    let shell_event = events
        .iter()
        .find(|e| {
            e.kind == "shell.capture"
                && e.workspace_id.as_ref().map(|id| id.as_str()) == Some("test-workspace-1")
        })
        .expect("should have stored a shell.capture event");

    assert_eq!(shell_event.kind, "shell.capture");
    assert_eq!(
        shell_event.workspace_id,
        Some("test-workspace-1".to_string())
    );

    // Verify payload is valid JSON array
    let _captures: Vec<ShellCapture> =
        serde_json::from_str(&shell_event.payload_json).expect("payload should be valid JSON");
}

#[test]
fn shell_restore_returns_captures() {
    let (_temp, store) = create_test_store();

    // Create a workspace first (required by foreign key constraint)
    let workspace_row = resurreccion_store::types::WorkspaceRow {
        id: "test-workspace-2".to_string(),
        binding_key: "test-key-2".to_string(),
        display_name: "Test Workspace 2".to_string(),
        root_path: "/tmp/test-2".to_string(),
        created_at: "2026-04-18T12:00:00.000Z".to_string(),
        last_opened_at: None,
    };
    store
        .lock()
        .unwrap()
        .workspace_insert(&workspace_row)
        .expect("failed to create workspace");

    // Manually insert a shell capture event
    let test_capture = ShellCapture {
        pid: 12345,
        cwd: "/home/user".to_string(),
        cmdline: vec!["/bin/bash".to_string()],
        env: {
            let mut env = HashMap::new();
            env.insert("HOME".to_string(), "/home/user".to_string());
            env
        },
        shell_name: "bash".to_string(),
    };

    let captures = vec![test_capture];
    let payload_json = serde_json::to_string(&captures).expect("should serialize captures");

    let event_row = resurreccion_store::types::EventRow {
        id: "test-event-1".to_string(),
        workspace_id: Some("test-workspace-2".to_string()),
        kind: "shell.capture".to_string(),
        payload_json,
        created_at: "2026-04-18T12:00:00.000Z".to_string(),
    };

    store
        .lock()
        .unwrap()
        .event_append(&event_row)
        .expect("failed to append event");

    // Create dispatcher with shell restore handler
    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::SHELL_RESTORE,
        Arc::new(resurreccion_daemon::ShellRestoreHandler::new(store.clone())),
    );

    // Send restore request
    let restore_req = Envelope::ok(
        "restore-1",
        verbs::SHELL_RESTORE,
        serde_json::json!({
            "workspace_id": "test-workspace-2"
        }),
    );

    let restore_resp = dispatcher.dispatch(&restore_req);
    assert!(restore_resp.ok, "shell restore failed");

    // Verify response contains captures array
    let captures_value = restore_resp
        .body
        .get("captures")
        .expect("should have captures in response");

    let captures_array = captures_value
        .as_array()
        .expect("captures should be an array");

    assert_eq!(captures_array.len(), 1, "should have one capture");

    let first_capture = captures_array.first().expect("should have first capture");
    assert_eq!(
        first_capture
            .get("pid")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        12345
    );
    assert_eq!(
        first_capture
            .get("shell_name")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "bash"
    );

    // Verify workspace_id in response
    assert_eq!(
        restore_resp
            .body
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "test-workspace-2"
    );
}

#[test]
fn shell_restore_returns_empty_if_no_captures() {
    let (_temp, store) = create_test_store();

    // Create dispatcher with shell restore handler
    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::SHELL_RESTORE,
        Arc::new(resurreccion_daemon::ShellRestoreHandler::new(store.clone())),
    );

    // Send restore request for non-existent workspace
    let restore_req = Envelope::ok(
        "restore-empty",
        verbs::SHELL_RESTORE,
        serde_json::json!({
            "workspace_id": "non-existent-workspace"
        }),
    );

    let restore_resp = dispatcher.dispatch(&restore_req);
    assert!(
        restore_resp.ok,
        "shell restore should succeed even with no captures"
    );

    // Verify response contains empty captures array
    let captures_value = restore_resp
        .body
        .get("captures")
        .expect("should have captures in response");

    let captures_array = captures_value
        .as_array()
        .expect("captures should be an array");

    assert!(captures_array.is_empty(), "should have no captures");
}

#[test]
fn shell_capture_requires_workspace_id() {
    let (_temp, store) = create_test_store();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::SHELL_CAPTURE,
        Arc::new(resurreccion_daemon::ShellCaptureHandler::new(store.clone())),
    );

    // Send capture request without workspace_id
    let capture_req = Envelope::ok(
        "bad-capture",
        verbs::SHELL_CAPTURE,
        serde_json::json!({
            "pids": []
        }),
    );

    let capture_resp = dispatcher.dispatch(&capture_req);
    assert!(!capture_resp.ok, "should fail without workspace_id");
    assert_eq!(
        capture_resp
            .body
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "missing_workspace_id"
    );
}

#[test]
fn shell_restore_requires_workspace_id() {
    let (_temp, store) = create_test_store();

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::SHELL_RESTORE,
        Arc::new(resurreccion_daemon::ShellRestoreHandler::new(store.clone())),
    );

    // Send restore request without workspace_id
    let restore_req = Envelope::ok("bad-restore", verbs::SHELL_RESTORE, serde_json::json!({}));

    let restore_resp = dispatcher.dispatch(&restore_req);
    assert!(!restore_resp.ok, "should fail without workspace_id");
    assert_eq!(
        restore_resp
            .body
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
        "missing_workspace_id"
    );
}
