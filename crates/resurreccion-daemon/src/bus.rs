//! Event bus, emitter, and subscribers for the daemon.
//!
//! # Architecture
//!
//! `EventBus` is single-threaded and not `Send`. All bus operations live on a
//! dedicated bus thread. A single channel bridges the Tokio world to the bus:
//!
//! ```text
//! handler ──emit_tx──> BUS THREAD ──calls──> bus.emit::<E>(event)
//!                                                ↓ subscribers fire
//!                                            store_tx ──> store writer
//! ```
//!
//! `EventEmitter` is a `Clone + Send` handle. Each call to `emit<E>` boxes a
//! closure that captures the typed event and calls `bus.emit(event)` on the
//! bus thread. No enum dispatch, no TypeId matching — rt-events handles
//! dispatch internally via `TypeId::of::<E>()`.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use resurreccion_core::events::{
    FocusChanged, LayoutChanged, PaneClosed, PaneOpened, RuntimeAttached, RuntimeDetached,
    SnapshotCreated, SnapshotRestored, WorkspaceClosed, WorkspaceOpened,
};
use resurreccion_store::{types::EventRow, Store};
use rt_events::EventBus;

// ── Emit channel ──────────────────────────────────────────────────────────────

/// A closure that emits one event onto the bus thread's `EventBus`.
type EmitFn = Box<dyn Fn(&mut EventBus) + Send>;

// ── EventEmitter ──────────────────────────────────────────────────────────────

/// A `Clone + Send` handle for emitting typed domain events from any thread.
///
/// Obtained from [`setup_event_bus`]. Clone freely — all clones share the
/// same bus thread.
///
/// # Usage
///
/// ```ignore
/// emitter.emit(WorkspaceOpened { workspace_id });
/// emitter.emit(SnapshotCreated { workspace_id, snapshot_id });
/// ```
#[derive(Clone)]
pub struct EventEmitter {
    tx: mpsc::SyncSender<EmitFn>,
}

impl EventEmitter {
    /// Create a no-op emitter for use in tests — all emits are silently dropped.
    pub fn no_op() -> Self {
        let (tx, _rx) = mpsc::sync_channel(1);
        Self { tx }
    }

    /// Emit any event type that rt-events can dispatch.
    ///
    /// The event is boxed into a closure and sent to the bus thread, which
    /// calls `bus.emit(event)`. Fires all registered subscribers synchronously
    /// on the bus thread. If the channel is full the event is silently dropped.
    pub fn emit<E: Clone + Send + 'static>(&self, event: E) {
        let _ = self.tx.try_send(Box::new(move |bus: &mut EventBus| {
            bus.emit(event.clone());
        }));
    }
}

// ── Setup ─────────────────────────────────────────────────────────────────────

/// Set up the event bus with the durable store subscriber.
///
/// Starts two threads:
/// - **Bus thread**: owns the `EventBus`, runs emit closures, fires subscribers.
/// - **Store writer thread**: receives `EventRow`s and appends to the store.
///
/// Returns `(handle, emitter)`. Join `handle` to wait for the bus thread to
/// drain on shutdown. All `EventEmitter` clones must be dropped first.
pub fn setup_event_bus(store: Arc<Mutex<Store>>) -> (thread::JoinHandle<()>, EventEmitter) {
    let (emit_tx, emit_rx) = mpsc::sync_channel::<EmitFn>(4096);
    let (store_tx, store_rx) = mpsc::channel::<EventRow>();

    let emitter = EventEmitter { tx: emit_tx };

    let handle = thread::spawn(move || {
        let mut bus = EventBus::new();

        // Register store subscriber for each event type.
        // Each subscriber converts the typed event to an EventRow and forwards
        // to the store writer thread via the channel.
        macro_rules! subscribe {
            ($event_ty:ty, $row_fn:expr) => {{
                let tx = store_tx.clone();
                bus.on(move |e: &$event_ty| {
                    let _ = tx.send($row_fn(e));
                });
            }};
        }

        subscribe!(WorkspaceOpened, |e: &WorkspaceOpened| {
            event_row("WorkspaceOpened", serde_json::json!({
                "workspace_id": e.workspace_id.to_string()
            }))
        });
        subscribe!(WorkspaceClosed, |e: &WorkspaceClosed| {
            event_row("WorkspaceClosed", serde_json::json!({
                "workspace_id": e.workspace_id.to_string()
            }))
        });
        subscribe!(RuntimeAttached, |e: &RuntimeAttached| {
            event_row("RuntimeAttached", serde_json::json!({
                "workspace_id": e.workspace_id.to_string(),
                "runtime_id": e.runtime_id.to_string()
            }))
        });
        subscribe!(RuntimeDetached, |e: &RuntimeDetached| {
            event_row("RuntimeDetached", serde_json::json!({
                "workspace_id": e.workspace_id.to_string(),
                "runtime_id": e.runtime_id.to_string()
            }))
        });
        subscribe!(PaneOpened, |e: &PaneOpened| {
            event_row("PaneOpened", serde_json::json!({
                "runtime_id": e.runtime_id.to_string(),
                "pane_id": e.pane_id.to_string()
            }))
        });
        subscribe!(PaneClosed, |e: &PaneClosed| {
            event_row("PaneClosed", serde_json::json!({
                "runtime_id": e.runtime_id.to_string(),
                "pane_id": e.pane_id.to_string()
            }))
        });
        subscribe!(FocusChanged, |e: &FocusChanged| {
            event_row("FocusChanged", serde_json::json!({
                "runtime_id": e.runtime_id.to_string(),
                "pane_id": e.pane_id.to_string()
            }))
        });
        subscribe!(LayoutChanged, |e: &LayoutChanged| {
            event_row("LayoutChanged", serde_json::json!({
                "runtime_id": e.runtime_id.to_string()
            }))
        });
        subscribe!(SnapshotCreated, |e: &SnapshotCreated| {
            event_row("SnapshotCreated", serde_json::json!({
                "workspace_id": e.workspace_id.to_string(),
                "snapshot_id": e.snapshot_id.to_string()
            }))
        });
        subscribe!(SnapshotRestored, |e: &SnapshotRestored| {
            event_row("SnapshotRestored", serde_json::json!({
                "workspace_id": e.workspace_id.to_string(),
                "snapshot_id": e.snapshot_id.to_string()
            }))
        });

        drop(store_tx);

        // Spawn store writer.
        let writer = thread::spawn(move || {
            for row in store_rx {
                if let Ok(s) = store.lock() {
                    if let Err(e) = s.event_append(&row) {
                        eprintln!("event_append: {e}");
                    }
                }
            }
        });

        // Run each emit closure on the bus (fires subscribers synchronously).
        for f in emit_rx {
            f(&mut bus);
        }

        let _ = writer.join();
    });

    (handle, emitter)
}

// ── Legacy adapter ────────────────────────────────────────────────────────────

/// Legacy adapter kept for existing call sites. Discards the emitter.
///
/// Migrate callers to [`setup_event_bus`] to obtain the `EventEmitter`.
pub fn setup_store_subscriber(
    _bus: &mut rt_events::EventBus,
    store: Arc<Mutex<Store>>,
) -> thread::JoinHandle<()> {
    let (handle, _emitter) = setup_event_bus(store);
    handle
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn event_row(kind: &str, payload: serde_json::Value) -> EventRow {
    EventRow {
        id: ulid::Ulid::new().to_string(),
        workspace_id: None,
        kind: kind.to_string(),
        payload_json: payload.to_string(),
        created_at: iso_now(),
    }
}

fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        2026, 4, 19,
        (ms / 3_600_000) % 24,
        (ms / 60_000) % 60,
        (ms / 1_000) % 60,
        ms % 1_000,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_core::events::WorkspaceOpened;
    use resurreccion_core::ids::{SnapshotId, WorkspaceId};
    use std::time::Duration;
    use tempfile::NamedTempFile;

    fn open_store() -> (NamedTempFile, Arc<Mutex<Store>>) {
        let tmp = NamedTempFile::new().unwrap();
        let store = Arc::new(Mutex::new(Store::open(tmp.path().to_str().unwrap()).unwrap()));
        (tmp, store) // caller must keep `tmp` alive for the duration of the test
    }

    #[test]
    fn emit_workspace_opened_flows_to_store() {
        let (_tmp, store) = open_store();
        let (_h, emitter) = setup_event_bus(Arc::clone(&store));

        emitter.emit(WorkspaceOpened { workspace_id: WorkspaceId::new() });

        thread::sleep(Duration::from_millis(50));
        let rows = store.lock().unwrap().event_tail_from(None).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].kind, "WorkspaceOpened");
    }

    #[test]
    fn emitter_is_send_clone_and_works_from_threads() {
        let (_tmp, store) = open_store();
        let (_h, emitter) = setup_event_bus(Arc::clone(&store));
        let e2 = emitter.clone();

        let ws = WorkspaceId::new();
        thread::spawn(move || e2.emit(WorkspaceOpened { workspace_id: ws }))
            .join().unwrap();

        emitter.emit(SnapshotCreated {
            workspace_id: ws,
            snapshot_id: SnapshotId::new(),
        });

        thread::sleep(Duration::from_millis(50));
        let rows = store.lock().unwrap().event_tail_from(None).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].kind, "WorkspaceOpened");
        assert_eq!(rows[1].kind, "SnapshotCreated");
    }

    #[test]
    fn generic_emit_requires_no_enum_boilerplate() {
        let (_tmp, store) = open_store();
        let (_h, emitter) = setup_event_bus(Arc::clone(&store));

        emitter.emit(SnapshotRestored {
            workspace_id: WorkspaceId::new(),
            snapshot_id: SnapshotId::new(),
        });

        thread::sleep(Duration::from_millis(50));
        let rows = store.lock().unwrap().event_tail_from(None).unwrap();
        assert_eq!(rows[0].kind, "SnapshotRestored");
    }
}
