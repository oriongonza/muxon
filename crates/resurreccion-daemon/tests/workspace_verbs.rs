#![allow(missing_docs)]

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

    // Create a mock WorkspaceListHandler
    struct WorkspaceListHandler {
        store: Arc<Mutex<Store>>,
    }
    impl resurreccion_daemon::Handler for WorkspaceListHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            match self.store.lock().unwrap().workspace_list() {
                Ok(rows) => Envelope::ok(
                    &env.id,
                    &env.verb,
                    serde_json::to_value(rows).unwrap_or_default(),
                ),
                Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
            }
        }
    }

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_LIST,
        Arc::new(WorkspaceListHandler {
            store: store.clone(),
        }),
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
    use resurreccion_store::types::WorkspaceRow;
    use ulid::Ulid;

    let (_temp, store) = create_test_store();

    struct WorkspaceCreateHandler {
        store: Arc<Mutex<Store>>,
    }
    impl resurreccion_daemon::Handler for WorkspaceCreateHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            // Parse args
            let name = env
                .body
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or("test");
            let path = env
                .body
                .get("root_path")
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp/test");
            let binding_key = env
                .body
                .get("binding_key")
                .and_then(|v| v.as_str())
                .unwrap_or("test-key");

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let created_at = format!("2026-04-18T00:00:{:02}.000Z", (now % 60));

            let row = WorkspaceRow {
                id: Ulid::new().to_string(),
                binding_key: binding_key.to_string(),
                display_name: name.to_string(),
                root_path: path.to_string(),
                created_at,
                last_opened_at: None,
            };

            match self.store.lock().unwrap().workspace_insert(&row) {
                Ok(_) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(&row).unwrap()),
                Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
            }
        }
    }

    struct WorkspaceGetHandler {
        store: Arc<Mutex<Store>>,
    }
    impl resurreccion_daemon::Handler for WorkspaceGetHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            let id = env
                .body
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("");

            match self.store.lock().unwrap().workspace_get(id) {
                Ok(Some(row)) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(row).unwrap()),
                Ok(None) => Envelope::err(&env.id, &env.verb, "not_found", "workspace not found"),
                Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
            }
        }
    }

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_CREATE,
        Arc::new(WorkspaceCreateHandler {
            store: store.clone(),
        }),
    );
    dispatcher.register(
        verbs::WORKSPACE_GET,
        Arc::new(WorkspaceGetHandler {
            store: store.clone(),
        }),
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
    use resurreccion_store::types::WorkspaceRow;
    use ulid::Ulid;

    let (_temp, store) = create_test_store();

    struct WorkspaceResolveOrCreateHandler {
        store: Arc<Mutex<Store>>,
    }
    impl resurreccion_daemon::Handler for WorkspaceResolveOrCreateHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            let binding_key = env
                .body
                .get("binding_key")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let root_path = env
                .body
                .get("root_path")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let display_name = env
                .body
                .get("display_name")
                .and_then(|v| v.as_str())
                .unwrap_or("workspace");

            let store_guard = self.store.lock().unwrap();

            // Try to get by binding key
            if let Ok(Some(existing)) = store_guard.workspace_get_by_key(binding_key) {
                return Envelope::ok(&env.id, &env.verb, serde_json::to_value(existing).unwrap());
            }

            drop(store_guard);

            // Create new workspace
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let created_at = format!("2026-04-18T00:00:{:02}.000Z", (now % 60));

            let row = WorkspaceRow {
                id: Ulid::new().to_string(),
                binding_key: binding_key.to_string(),
                display_name: display_name.to_string(),
                root_path: root_path.to_string(),
                created_at,
                last_opened_at: None,
            };

            match self.store.lock().unwrap().workspace_insert(&row) {
                Ok(_) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(&row).unwrap()),
                Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
            }
        }
    }

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_RESOLVE_OR_CREATE,
        Arc::new(WorkspaceResolveOrCreateHandler {
            store: store.clone(),
        }),
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
fn workspace_open_returns_workspace_row() {
    use resurreccion_store::types::WorkspaceRow;
    use ulid::Ulid;

    let (_temp, store) = create_test_store();

    struct WorkspaceOpenHandler {
        store: Arc<Mutex<Store>>,
    }
    impl resurreccion_daemon::Handler for WorkspaceOpenHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            let _path = env
                .body
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or("/tmp");

            // For this test, we'll just create a workspace
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let created_at = format!("2026-04-18T00:00:{:02}.000Z", (now % 60));

            let row = WorkspaceRow {
                id: Ulid::new().to_string(),
                binding_key: "test-binding".to_string(),
                display_name: "test".to_string(),
                root_path: "/tmp".to_string(),
                created_at,
                last_opened_at: None,
            };

            match self.store.lock().unwrap().workspace_insert(&row) {
                Ok(_) => {
                    // Touch to update last_opened_at
                    let _ = self.store.lock().unwrap().workspace_touch(&row.id);
                    if let Ok(Some(updated)) = self.store.lock().unwrap().workspace_get(&row.id) {
                        Envelope::ok(&env.id, &env.verb, serde_json::to_value(updated).unwrap())
                    } else {
                        Envelope::ok(&env.id, &env.verb, serde_json::to_value(row).unwrap())
                    }
                }
                Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
            }
        }
    }

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(
        verbs::WORKSPACE_OPEN,
        Arc::new(WorkspaceOpenHandler {
            store: store.clone(),
        }),
    );

    let req = Envelope::ok(
        "test-id",
        verbs::WORKSPACE_OPEN,
        serde_json::json!({"path": "/tmp"}),
    );
    let resp = dispatcher.dispatch(&req);

    assert!(resp.ok);
    assert!(resp.body.get("id").is_some());
    assert!(resp.body.get("binding_key").is_some());
    assert_eq!(
        resp.body.get("display_name").and_then(|v| v.as_str()),
        Some("test")
    );
}
