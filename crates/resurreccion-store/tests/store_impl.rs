use anyhow::Result;
use resurreccion_store::{EventRow, RuntimeRow, SnapshotRow, Store, WorkspaceRow};
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn temp_store() -> Result<(TempDir, Store)> {
    let dir = TempDir::new()?;
    let path = dir.path().join("test.db");
    let store = Store::open(path.to_str().unwrap())?;
    Ok((dir, store))
}

fn now_iso() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    // For tests, we'll use a simple ISO-like format with milliseconds
    format!("2026-04-18T00:00:{:02}.{:03}Z", (now / 1000) % 60, now % 1000)
}

#[test]
fn workspace_insert_and_get() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let ws = WorkspaceRow {
        id: "ws-test-001".to_string(),
        binding_key: "key-001".to_string(),
        display_name: "Test Workspace".to_string(),
        root_path: "/tmp/test".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };

    store.workspace_insert(&ws)?;
    let retrieved = store.workspace_get("ws-test-001")?;

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "ws-test-001");
    assert_eq!(retrieved.binding_key, "key-001");
    assert_eq!(retrieved.display_name, "Test Workspace");
    assert_eq!(retrieved.root_path, "/tmp/test");

    Ok(())
}

#[test]
fn workspace_list() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let ws1 = WorkspaceRow {
        id: "ws-list-001".to_string(),
        binding_key: "key-list-001".to_string(),
        display_name: "Workspace 1".to_string(),
        root_path: "/tmp/test1".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };

    let ws2 = WorkspaceRow {
        id: "ws-list-002".to_string(),
        binding_key: "key-list-002".to_string(),
        display_name: "Workspace 2".to_string(),
        root_path: "/tmp/test2".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };

    store.workspace_insert(&ws1)?;
    store.workspace_insert(&ws2)?;

    let all = store.workspace_list()?;
    assert_eq!(all.len(), 2);

    Ok(())
}

#[test]
fn workspace_touch() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let ws = WorkspaceRow {
        id: "ws-touch-001".to_string(),
        binding_key: "key-touch-001".to_string(),
        display_name: "Touch Test".to_string(),
        root_path: "/tmp/test".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };

    store.workspace_insert(&ws)?;

    // Initially no last_opened_at
    let before = store.workspace_get("ws-touch-001")?.unwrap();
    assert!(before.last_opened_at.is_none());

    // Touch it
    store.workspace_touch("ws-touch-001")?;

    // Now it should have last_opened_at set
    let after = store.workspace_get("ws-touch-001")?.unwrap();
    assert!(after.last_opened_at.is_some());

    Ok(())
}

#[test]
fn workspace_get_by_key() -> Result<()> {
    let (_dir, store) = temp_store()?;

    let ws = WorkspaceRow {
        id: "ws-key-001".to_string(),
        binding_key: "key-unique-001".to_string(),
        display_name: "Key Test".to_string(),
        root_path: "/tmp/test".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };

    store.workspace_insert(&ws)?;
    let retrieved = store.workspace_get_by_key("key-unique-001")?;

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "ws-key-001");
    assert_eq!(retrieved.binding_key, "key-unique-001");

    Ok(())
}

#[test]
fn runtime_insert_list_detach() -> Result<()> {
    let (_dir, store) = temp_store()?;

    // First insert a workspace
    let ws = WorkspaceRow {
        id: "ws-runtime-001".to_string(),
        binding_key: "key-runtime-001".to_string(),
        display_name: "Runtime Test WS".to_string(),
        root_path: "/tmp/test".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };
    store.workspace_insert(&ws)?;

    // Insert a runtime
    let rt = RuntimeRow {
        id: "rt-001".to_string(),
        workspace_id: "ws-runtime-001".to_string(),
        session_name: "my-session".to_string(),
        backend: "zellij".to_string(),
        created_at: now_iso(),
        detached_at: None,
    };

    store.runtime_insert(&rt)?;

    // List runtimes for the workspace
    let runtimes = store.runtime_list("ws-runtime-001")?;
    assert_eq!(runtimes.len(), 1);
    assert_eq!(runtimes[0].id, "rt-001");
    assert!(runtimes[0].detached_at.is_none());

    // Detach the runtime
    store.runtime_detach("rt-001")?;

    // Verify it's marked as detached
    let runtimes = store.runtime_list("ws-runtime-001")?;
    assert_eq!(runtimes.len(), 1);
    assert!(runtimes[0].detached_at.is_some());

    Ok(())
}

#[test]
fn snapshot_insert_get_list() -> Result<()> {
    let (_dir, store) = temp_store()?;

    // Insert workspace
    let ws = WorkspaceRow {
        id: "ws-snap-001".to_string(),
        binding_key: "key-snap-001".to_string(),
        display_name: "Snapshot Test WS".to_string(),
        root_path: "/tmp/test".to_string(),
        created_at: now_iso(),
        last_opened_at: None,
    };
    store.workspace_insert(&ws)?;

    // Insert a snapshot
    let snap = SnapshotRow {
        id: "snap-001".to_string(),
        workspace_id: "ws-snap-001".to_string(),
        runtime_id: None,
        fidelity: "structural".to_string(),
        manifest_json: r#"{"version": 1}"#.to_string(),
        created_at: now_iso(),
    };

    store.snapshot_insert(&snap)?;

    // Get by ID
    let retrieved = store.snapshot_get("snap-001")?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "snap-001");
    assert_eq!(retrieved.fidelity, "structural");

    // List by workspace
    let snaps = store.snapshot_list("ws-snap-001")?;
    assert_eq!(snaps.len(), 1);
    assert_eq!(snaps[0].id, "snap-001");

    Ok(())
}

#[test]
fn event_append_tail() -> Result<()> {
    let (_dir, store) = temp_store()?;

    // Append 3 events
    let e1 = EventRow {
        id: "e1".to_string(),
        workspace_id: None,
        kind: "workspace.created".to_string(),
        payload_json: r#"{"id":"ws1"}"#.to_string(),
        created_at: now_iso(),
    };

    let e2 = EventRow {
        id: "e2".to_string(),
        workspace_id: None,
        kind: "workspace.created".to_string(),
        payload_json: r#"{"id":"ws2"}"#.to_string(),
        created_at: now_iso(),
    };

    let e3 = EventRow {
        id: "e3".to_string(),
        workspace_id: None,
        kind: "runtime.attached".to_string(),
        payload_json: r#"{"id":"rt1"}"#.to_string(),
        created_at: now_iso(),
    };

    store.event_append(&e1)?;
    store.event_append(&e2)?;
    store.event_append(&e3)?;

    // Tail from beginning should return all 3
    let all = store.event_tail_from(None)?;
    assert_eq!(all.len(), 3);

    // Tail from after e1 should return e2 and e3
    let from_e2 = store.event_tail_from(Some("e1"))?;
    assert_eq!(from_e2.len(), 2);
    assert_eq!(from_e2[0].id, "e2");
    assert_eq!(from_e2[1].id, "e3");

    Ok(())
}
