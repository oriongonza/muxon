//! Domain event types.
//!
//! Each type here is a distinct `TypeId`, used as the key for `rt-events` subscriptions.
//! Adding a new event type here automatically creates a new subscription channel.

use crate::ids::{PaneId, RuntimeId, SnapshotId, WorkspaceId};
use serde::{Deserialize, Serialize};

/// Marker trait implemented by all daemon event types.
pub trait DaemonEvent: Send + Sync + 'static {}

macro_rules! daemon_event {
    ($(#[$attr:meta])* $name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Serialize, Deserialize)]
        pub struct $name {
            $(#[doc = concat!("The ", stringify!($field), " field.")]
            pub $field: $ty,)*
        }
        impl DaemonEvent for $name {}
    };
}

daemon_event!(
    /// A workspace was opened by a client.
    WorkspaceOpened { workspace_id: WorkspaceId }
);
daemon_event!(
    /// A workspace was closed (all clients detached).
    WorkspaceClosed { workspace_id: WorkspaceId }
);
daemon_event!(
    /// A multiplexer runtime was attached to a workspace.
    RuntimeAttached { workspace_id: WorkspaceId, runtime_id: RuntimeId }
);
daemon_event!(
    /// A multiplexer runtime was detached from a workspace.
    RuntimeDetached { workspace_id: WorkspaceId, runtime_id: RuntimeId }
);
daemon_event!(
    /// A pane was opened in the multiplexer.
    PaneOpened { runtime_id: RuntimeId, pane_id: PaneId }
);
daemon_event!(
    /// A pane was closed in the multiplexer.
    PaneClosed { runtime_id: RuntimeId, pane_id: PaneId }
);
daemon_event!(
    /// Keyboard focus moved to a different pane.
    FocusChanged { runtime_id: RuntimeId, pane_id: PaneId }
);
daemon_event!(
    /// The multiplexer layout changed (tabs/splits added or removed).
    LayoutChanged { runtime_id: RuntimeId }
);
daemon_event!(
    /// A snapshot was successfully created.
    SnapshotCreated { workspace_id: WorkspaceId, snapshot_id: SnapshotId }
);
daemon_event!(
    /// A snapshot was successfully restored.
    SnapshotRestored { workspace_id: WorkspaceId, snapshot_id: SnapshotId }
);
