//! Compile-time test: all public types exist and are usable.
use resurreccion_core::events::{
    DaemonEvent, FocusChanged, LayoutChanged, PaneClosed, PaneOpened, RuntimeAttached,
    RuntimeDetached, SnapshotCreated, SnapshotRestored, WorkspaceClosed, WorkspaceOpened,
};
use resurreccion_core::{
    BindingKey, BlobId, ErrorCode, EventId, PaneId, PartialRestore, RestoreFidelity, RuntimeId,
    SessionId, SnapshotId, TabId, WorkspaceId,
};

#[test]
fn all_id_types_are_distinct() {
    let workspace = WorkspaceId::new();
    let runtime = RuntimeId::new();
    let snapshot = SnapshotId::new();
    let pane = PaneId::new();
    let tab = TabId::new();
    let session = SessionId::new();
    let event = EventId::new();
    let blob = BlobId::new();
    // IDs are time-sortable ULIDs
    assert!(workspace.to_string().len() == 26);
    assert!(runtime.to_string().len() == 26);
    assert!(snapshot.to_string().len() == 26);
    assert!(pane.to_string().len() == 26);
    assert!(tab.to_string().len() == 26);
    assert!(session.to_string().len() == 26);
    assert!(event.to_string().len() == 26);
    assert!(blob.to_string().len() == 26);
}

#[test]
fn binding_key_is_deterministic() {
    let key1 = BindingKey::from_bytes([0u8; 32]);
    let key2 = BindingKey::from_bytes([0u8; 32]);
    assert_eq!(key1, key2);
}

#[test]
fn restore_fidelity_ordering() {
    assert!(RestoreFidelity::Exact > RestoreFidelity::Stateful);
    assert!(RestoreFidelity::Stateful > RestoreFidelity::Structural);
    assert!(RestoreFidelity::Structural > RestoreFidelity::Historical);
}

#[test]
fn event_types_are_usable() {
    // Verify that all event types are constructable and implement DaemonEvent
    let events: Vec<Box<dyn DaemonEvent>> = vec![
        Box::new(WorkspaceOpened {
            workspace_id: WorkspaceId::new(),
        }),
        Box::new(WorkspaceClosed {
            workspace_id: WorkspaceId::new(),
        }),
        Box::new(RuntimeAttached {
            workspace_id: WorkspaceId::new(),
            runtime_id: RuntimeId::new(),
        }),
        Box::new(RuntimeDetached {
            workspace_id: WorkspaceId::new(),
            runtime_id: RuntimeId::new(),
        }),
        Box::new(PaneOpened {
            runtime_id: RuntimeId::new(),
            pane_id: PaneId::new(),
        }),
        Box::new(PaneClosed {
            runtime_id: RuntimeId::new(),
            pane_id: PaneId::new(),
        }),
        Box::new(FocusChanged {
            runtime_id: RuntimeId::new(),
            pane_id: PaneId::new(),
        }),
        Box::new(LayoutChanged {
            runtime_id: RuntimeId::new(),
        }),
        Box::new(SnapshotCreated {
            workspace_id: WorkspaceId::new(),
            snapshot_id: SnapshotId::new(),
        }),
        Box::new(SnapshotRestored {
            workspace_id: WorkspaceId::new(),
            snapshot_id: SnapshotId::new(),
        }),
    ];
    assert_eq!(events.len(), 10);
}

#[test]
fn error_code_and_partial_restore() {
    let error_code: ErrorCode = ErrorCode::NotFound;
    assert_eq!(error_code, ErrorCode::NotFound);
    let restore = PartialRestore {
        fidelity: RestoreFidelity::Exact,
        failures: vec![],
    };
    assert_eq!(restore.fidelity, RestoreFidelity::Exact);
}
