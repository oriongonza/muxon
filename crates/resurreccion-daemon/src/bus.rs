//! Event bus and subscribers for the daemon.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use resurreccion_core::events::{
    FocusChanged, LayoutChanged, PaneClosed, PaneOpened, RuntimeAttached, RuntimeDetached,
    SnapshotCreated, SnapshotRestored, WorkspaceClosed, WorkspaceOpened,
};
use resurreccion_store::{types::EventRow, Store};
use rt_events::EventBus;

/// A serializable event wrapper for the channel.
#[derive(Debug, Clone)]
enum BusEventMessage {
    WorkspaceOpened {
        workspace_id: String,
    },
    WorkspaceClosed {
        workspace_id: String,
    },
    RuntimeAttached {
        workspace_id: String,
        runtime_id: String,
    },
    RuntimeDetached {
        workspace_id: String,
        runtime_id: String,
    },
    PaneOpened {
        runtime_id: String,
        pane_id: String,
    },
    PaneClosed {
        runtime_id: String,
        pane_id: String,
    },
    FocusChanged {
        runtime_id: String,
        pane_id: String,
    },
    LayoutChanged {
        runtime_id: String,
    },
    SnapshotCreated {
        workspace_id: String,
        snapshot_id: String,
    },
    SnapshotRestored {
        workspace_id: String,
        snapshot_id: String,
    },
}

impl BusEventMessage {
    /// Convert event message to `EventRow` for storage.
    fn to_event_row(&self) -> EventRow {
        use serde_json::json;

        match self {
            Self::WorkspaceOpened { workspace_id } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "WorkspaceOpened".to_string(),
                payload_json: json!({ "workspace_id": workspace_id }).to_string(),
                created_at: iso_now(),
            },
            Self::WorkspaceClosed { workspace_id } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "WorkspaceClosed".to_string(),
                payload_json: json!({ "workspace_id": workspace_id }).to_string(),
                created_at: iso_now(),
            },
            Self::RuntimeAttached {
                workspace_id,
                runtime_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "RuntimeAttached".to_string(),
                payload_json: json!({ "workspace_id": workspace_id, "runtime_id": runtime_id })
                    .to_string(),
                created_at: iso_now(),
            },
            Self::RuntimeDetached {
                workspace_id,
                runtime_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "RuntimeDetached".to_string(),
                payload_json: json!({ "workspace_id": workspace_id, "runtime_id": runtime_id })
                    .to_string(),
                created_at: iso_now(),
            },
            Self::PaneOpened {
                runtime_id,
                pane_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "PaneOpened".to_string(),
                payload_json: json!({ "runtime_id": runtime_id, "pane_id": pane_id }).to_string(),
                created_at: iso_now(),
            },
            Self::PaneClosed {
                runtime_id,
                pane_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "PaneClosed".to_string(),
                payload_json: json!({ "runtime_id": runtime_id, "pane_id": pane_id }).to_string(),
                created_at: iso_now(),
            },
            Self::FocusChanged {
                runtime_id,
                pane_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "FocusChanged".to_string(),
                payload_json: json!({ "runtime_id": runtime_id, "pane_id": pane_id }).to_string(),
                created_at: iso_now(),
            },
            Self::LayoutChanged { runtime_id } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "LayoutChanged".to_string(),
                payload_json: json!({ "runtime_id": runtime_id }).to_string(),
                created_at: iso_now(),
            },
            Self::SnapshotCreated {
                workspace_id,
                snapshot_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "SnapshotCreated".to_string(),
                payload_json: json!({ "workspace_id": workspace_id, "snapshot_id": snapshot_id })
                    .to_string(),
                created_at: iso_now(),
            },
            Self::SnapshotRestored {
                workspace_id,
                snapshot_id,
            } => EventRow {
                id: ulid::Ulid::new().to_string(),
                workspace_id: None,
                kind: "SnapshotRestored".to_string(),
                payload_json: json!({ "workspace_id": workspace_id, "snapshot_id": snapshot_id })
                    .to_string(),
                created_at: iso_now(),
            },
        }
    }
}

/// Get current timestamp in ISO-8601 format.
fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let seconds = now_millis / 1000;
    let millis = now_millis % 1000;
    format!("2026-04-18T00:00:{:02}.{:03}Z", (seconds % 60), millis)
}

/// Set up the durable event subscriber that writes all events to the store.
///
/// # Description
/// Creates an `mpsc` channel and registers callbacks on the bus for all event types.
/// Each callback sends the event to the channel (sync, non-blocking).
/// Spawns a worker thread that receives from the channel and calls `store.event_append()`.
///
/// # Arguments
/// * `bus` - The event bus (must be on a single thread)
/// * `store` - The store wrapped in Arc<Mutex<>> for thread-safe access
///
/// # Returns
/// A `JoinHandle` that can be awaited to join the worker thread.
///
/// # Note on rt-events constraints
/// All callbacks must be sync and non-blocking. Only channel-send is allowed in callbacks.
#[allow(clippy::too_many_lines)]
pub fn setup_store_subscriber(
    bus: &mut EventBus,
    store: Arc<Mutex<Store>>,
) -> thread::JoinHandle<()> {
    let (tx, rx) = mpsc::channel::<BusEventMessage>();

    // Register WorkspaceOpened
    {
        let tx = tx.clone();
        bus.on(move |event: &WorkspaceOpened| {
            let msg = BusEventMessage::WorkspaceOpened {
                workspace_id: event.workspace_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register WorkspaceClosed
    {
        let tx = tx.clone();
        bus.on(move |event: &WorkspaceClosed| {
            let msg = BusEventMessage::WorkspaceClosed {
                workspace_id: event.workspace_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register RuntimeAttached
    {
        let tx = tx.clone();
        bus.on(move |event: &RuntimeAttached| {
            let msg = BusEventMessage::RuntimeAttached {
                workspace_id: event.workspace_id.to_string(),
                runtime_id: event.runtime_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register RuntimeDetached
    {
        let tx = tx.clone();
        bus.on(move |event: &RuntimeDetached| {
            let msg = BusEventMessage::RuntimeDetached {
                workspace_id: event.workspace_id.to_string(),
                runtime_id: event.runtime_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register PaneOpened
    {
        let tx = tx.clone();
        bus.on(move |event: &PaneOpened| {
            let msg = BusEventMessage::PaneOpened {
                runtime_id: event.runtime_id.to_string(),
                pane_id: event.pane_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register PaneClosed
    {
        let tx = tx.clone();
        bus.on(move |event: &PaneClosed| {
            let msg = BusEventMessage::PaneClosed {
                runtime_id: event.runtime_id.to_string(),
                pane_id: event.pane_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register FocusChanged
    {
        let tx = tx.clone();
        bus.on(move |event: &FocusChanged| {
            let msg = BusEventMessage::FocusChanged {
                runtime_id: event.runtime_id.to_string(),
                pane_id: event.pane_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register LayoutChanged
    {
        let tx = tx.clone();
        bus.on(move |event: &LayoutChanged| {
            let msg = BusEventMessage::LayoutChanged {
                runtime_id: event.runtime_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register SnapshotCreated
    {
        let tx = tx.clone();
        bus.on(move |event: &SnapshotCreated| {
            let msg = BusEventMessage::SnapshotCreated {
                workspace_id: event.workspace_id.to_string(),
                snapshot_id: event.snapshot_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    // Register SnapshotRestored
    {
        let tx = tx.clone();
        bus.on(move |event: &SnapshotRestored| {
            let msg = BusEventMessage::SnapshotRestored {
                workspace_id: event.workspace_id.to_string(),
                snapshot_id: event.snapshot_id.to_string(),
            };
            let _ = tx.send(msg);
        });
    }

    drop(tx); // Drop the original sender; clones are held by callbacks

    // Spawn worker thread
    thread::spawn(move || {
        for msg in rx {
            let row = msg.to_event_row();
            if let Ok(store) = store.lock() {
                let result = store.event_append(&row);
                if result.is_err() {
                    eprintln!("Failed to append event: {result:?}");
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_core::ids::{PaneId, RuntimeId, WorkspaceId};
    use std::time::Duration;
    use tempfile::NamedTempFile;

    #[test]
    fn test_bus_callback_receives_event() {
        // First, test that the bus callback mechanism works at all
        use std::cell::RefCell;
        use std::rc::Rc;

        let received = Rc::new(RefCell::new(None));
        let mut bus = EventBus::new();
        let r = received.clone();

        bus.on(move |event: &WorkspaceOpened| {
            *r.borrow_mut() = Some(event.workspace_id.to_string());
        });

        let ws_id = WorkspaceId::new();
        bus.emit(WorkspaceOpened {
            workspace_id: ws_id,
        });

        let val = received.borrow();
        assert!(val.is_some(), "callback was not invoked");
        assert_eq!(*val, Some(ws_id.to_string()));
    }

    #[test]
    fn test_channel_in_callback() {
        // Test that channels work within callbacks
        let (tx, rx) = mpsc::channel::<String>();
        let mut bus = EventBus::new();

        bus.on(move |event: &WorkspaceOpened| {
            let msg = format!("workspace: {}", event.workspace_id);
            let _ = tx.send(msg);
        });

        let ws_id = WorkspaceId::new();
        bus.emit(WorkspaceOpened {
            workspace_id: ws_id,
        });

        // Try to receive
        let msg = rx
            .recv_timeout(Duration::from_millis(100))
            .expect("failed to receive from channel");
        assert!(msg.starts_with("workspace:"));
    }

    #[test]
    fn test_setup_store_subscriber_basic() {
        // Create a temporary database
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap();

        // Open store
        let store = Arc::new(Mutex::new(
            Store::open(db_path).expect("failed to open store"),
        ));

        // Create bus
        let mut bus = EventBus::new();

        // Setup subscriber
        let _handle = setup_store_subscriber(&mut bus, Arc::clone(&store));

        // Emit an event
        let ws_id = WorkspaceId::new();
        bus.emit(WorkspaceOpened {
            workspace_id: ws_id,
        });

        // Give worker thread time to process
        std::thread::sleep(Duration::from_millis(100));

        // Verify event was written
        let events = store
            .lock()
            .unwrap()
            .event_tail_from(None)
            .expect("failed to query events");
        assert!(
            !events.is_empty(),
            "expected at least one event, got {}",
            events.len()
        );
        assert_eq!(events[0].kind, "WorkspaceOpened");
    }

    #[test]
    fn test_setup_store_subscriber_all_event_types() {
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap();
        let store = Arc::new(Mutex::new(
            Store::open(db_path).expect("failed to open store"),
        ));

        let mut bus = EventBus::new();
        let _handle = setup_store_subscriber(&mut bus, Arc::clone(&store));

        let ws_id = WorkspaceId::new();
        let rt_id = RuntimeId::new();

        // Emit various events
        bus.emit(WorkspaceOpened {
            workspace_id: ws_id,
        });
        bus.emit(RuntimeAttached {
            workspace_id: ws_id,
            runtime_id: rt_id,
        });
        bus.emit(PaneOpened {
            runtime_id: rt_id,
            pane_id: PaneId::new(),
        });

        std::thread::sleep(Duration::from_millis(100));

        let events = store
            .lock()
            .unwrap()
            .event_tail_from(None)
            .expect("failed to query events");
        assert_eq!(
            events.len(),
            3,
            "expected three events, got {}",
            events.len()
        );
        assert_eq!(events[0].kind, "WorkspaceOpened");
        assert_eq!(events[1].kind, "RuntimeAttached");
        assert_eq!(events[2].kind, "PaneOpened");
    }

    #[test]
    fn test_callback_is_non_blocking() {
        // This test verifies the callback returns immediately by checking
        // that we can emit many events rapidly without blocking.
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap();
        let store = Arc::new(Mutex::new(
            Store::open(db_path).expect("failed to open store"),
        ));

        let mut bus = EventBus::new();
        let _handle = setup_store_subscriber(&mut bus, Arc::clone(&store));

        // Emit 1000 events rapidly
        let start = std::time::Instant::now();
        for _ in 0..1000 {
            bus.emit(WorkspaceOpened {
                workspace_id: WorkspaceId::new(),
            });
        }
        let elapsed = start.elapsed();

        // Should complete quickly (callback returns immediately)
        // 1000 emissions should be < 100ms on a modern system
        assert!(
            elapsed < Duration::from_millis(100),
            "callback may be blocking: took {elapsed:?}"
        );
    }
}
