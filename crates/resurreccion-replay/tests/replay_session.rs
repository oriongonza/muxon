//! Integration tests for `resurreccion-replay`.

use resurreccion_replay::ReplaySession;
use resurreccion_store::{EventRow, Store, WorkspaceRow};

/// Build an [`EventRow`] with sensible defaults.
fn make_event(id: &str, workspace_id: Option<&str>, kind: &str, payload: &str) -> EventRow {
    EventRow {
        id: id.to_string(),
        workspace_id: workspace_id.map(str::to_string),
        kind: kind.to_string(),
        payload_json: payload.to_string(),
        created_at: format!("2024-01-01T00:00:00.{id}Z"),
    }
}

fn open_store() -> Store {
    Store::open(":memory:").expect("in-memory store")
}

fn insert_workspace(store: &Store, id: &str) {
    store
        .workspace_insert(&WorkspaceRow {
            id: id.to_string(),
            binding_key: id.to_string(),
            display_name: id.to_string(),
            root_path: format!("/tmp/{id}"),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            last_opened_at: None,
        })
        .expect("insert workspace");
}

// ── Basic: new() returns all events in order ──────────────────────────────

#[test]
fn new_returns_all_frames_in_order() {
    let store = open_store();
    store
        .event_append(&make_event("01", None, "shell.cmd", r#"{"cmd":"ls"}"#))
        .unwrap();
    store
        .event_append(&make_event(
            "02",
            None,
            "shell.out",
            r#"{"out":"file.txt"}"#,
        ))
        .unwrap();
    store
        .event_append(&make_event("03", None, "shell.cmd", r#"{"cmd":"pwd"}"#))
        .unwrap();

    let session = ReplaySession::new(&store).unwrap();
    assert_eq!(session.frame_count(), 3);

    let frames = session.all_frames();
    assert_eq!(frames[0].event_id, "01");
    assert_eq!(frames[1].event_id, "02");
    assert_eq!(frames[2].event_id, "03");
}

// ── for_workspace() filters to the requested workspace ───────────────────

#[test]
fn for_workspace_filters_correctly() {
    let store = open_store();

    // Insert workspaces first to satisfy FK constraint.
    insert_workspace(&store, "ws-a");
    insert_workspace(&store, "ws-b");

    store
        .event_append(&make_event("01", Some("ws-a"), "shell.cmd", r#"{}"#))
        .unwrap();
    store
        .event_append(&make_event("02", Some("ws-b"), "shell.cmd", r#"{}"#))
        .unwrap();
    store
        .event_append(&make_event("03", Some("ws-a"), "shell.out", r#"{}"#))
        .unwrap();
    store
        .event_append(&make_event("04", None, "daemon.start", r#"{}"#))
        .unwrap();

    let session_a = ReplaySession::for_workspace(&store, "ws-a").unwrap();
    assert_eq!(session_a.frame_count(), 2);
    assert_eq!(session_a.all_frames()[0].event_id, "01");
    assert_eq!(session_a.all_frames()[1].event_id, "03");

    let session_b = ReplaySession::for_workspace(&store, "ws-b").unwrap();
    assert_eq!(session_b.frame_count(), 1);
    assert_eq!(session_b.all_frames()[0].event_id, "02");
}

// ── Empty store ───────────────────────────────────────────────────────────

#[test]
fn new_on_empty_store_returns_empty_session() {
    let store = open_store();
    let session = ReplaySession::new(&store).unwrap();
    assert_eq!(session.frame_count(), 0);
    assert!(session.current_frame().is_none());
}

// ── Cursor: current_frame, next, seek ────────────────────────────────────

#[test]
fn cursor_navigation_works() {
    let store = open_store();
    for i in 1u8..=4 {
        store
            .event_append(&make_event(&format!("0{i}"), None, "shell.cmd", r#"{}"#))
            .unwrap();
    }

    let mut session = ReplaySession::new(&store).unwrap();
    assert_eq!(session.frame_count(), 4);

    // cursor starts at 0
    assert_eq!(
        session.current_frame().map(|f| f.event_id.as_str()),
        Some("01")
    );

    // next advances cursor to 1
    let f = session.next();
    assert_eq!(f.map(|f| f.event_id.as_str()), Some("02"));
    assert_eq!(
        session.current_frame().map(|f| f.event_id.as_str()),
        Some("02")
    );

    // seek to last frame (index 3)
    session.seek(3);
    assert_eq!(
        session.current_frame().map(|f| f.event_id.as_str()),
        Some("04")
    );

    // next from last returns None (cursor goes to 4 which is out of bounds)
    let after_last = session.next();
    assert!(after_last.is_none());
    assert!(session.current_frame().is_none());
}

#[test]
fn seek_past_end_clamps_to_frame_count() {
    let store = open_store();
    store
        .event_append(&make_event("01", None, "shell.cmd", r#"{}"#))
        .unwrap();

    let mut session = ReplaySession::new(&store).unwrap();
    session.seek(999);
    // cursor == frame_count (1), current_frame is None
    assert_eq!(session.frame_count(), 1);
    assert!(session.current_frame().is_none());
}

// ── Payload deserialization ───────────────────────────────────────────────

#[test]
fn payload_deserializes_json_correctly() {
    let store = open_store();
    store
        .event_append(&make_event("01", None, "shell.cmd", r#"{"cmd":"echo hi"}"#))
        .unwrap();

    let session = ReplaySession::new(&store).unwrap();
    let frame = &session.all_frames()[0];
    assert_eq!(frame.payload["cmd"], "echo hi");
}

#[test]
fn invalid_payload_json_becomes_null() {
    let store = open_store();
    store
        .event_append(&make_event("01", None, "shell.cmd", "not-json"))
        .unwrap();

    let session = ReplaySession::new(&store).unwrap();
    let frame = &session.all_frames()[0];
    assert!(frame.payload.is_null());
}

// ── workspace_id accessor ─────────────────────────────────────────────────

#[test]
fn workspace_id_accessor_reflects_filter() {
    let store = open_store();
    let unfiltered = ReplaySession::new(&store).unwrap();
    assert!(unfiltered.workspace_id().is_none());

    // for_workspace with no matching events still records the filter.
    let filtered = ReplaySession::for_workspace(&store, "ws-x").unwrap();
    assert_eq!(filtered.workspace_id(), Some("ws-x"));
    assert_eq!(filtered.frame_count(), 0);
}
