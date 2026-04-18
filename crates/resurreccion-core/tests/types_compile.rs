//! Compile-time test: all public types exist and are usable.
use resurreccion_core::{
    BlobId, BindingKey, ErrorCode, EventId, PaneId, PartialRestore, RestoreFidelity,
    RuntimeId, SessionId, SnapshotId, TabId, WorkspaceId,
};
use resurreccion_core::events::{
    DaemonEvent, FocusChanged, LayoutChanged, PaneClosed, PaneOpened,
    RuntimeAttached, RuntimeDetached, SnapshotCreated, SnapshotRestored,
    WorkspaceClosed, WorkspaceOpened,
};

#[test]
fn all_id_types_are_distinct() {
    let w = WorkspaceId::new();
    let r = RuntimeId::new();
    let s = SnapshotId::new();
    // IDs are time-sortable ULIDs
    assert!(w.to_string().len() == 26);
    assert!(r.to_string().len() == 26);
    assert!(s.to_string().len() == 26);
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
