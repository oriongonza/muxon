#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use resurreccion_core::events::WorkspaceOpened;
    use resurreccion_core::ids::WorkspaceId;
    use resurreccion_store::{types::EventRow, Store};
    use rt_events::EventBus;
    use tempfile::NamedTempFile;

    #[test]
    fn test_direct_store_write() {
        // Test that we can write directly to the store
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap();
        let store = Arc::new(Mutex::new(
            Store::open(db_path).expect("failed to open store"),
        ));

        let row = EventRow {
            id: "test-id".to_string(),
            workspace_id: None, // NULL to avoid FK constraint
            kind: "TestEvent".to_string(),
            payload_json: "{}".to_string(),
            created_at: "2026-04-18T00:00:00.000Z".to_string(),
        };

        store
            .lock()
            .unwrap()
            .event_append(&row)
            .expect("failed to append event");

        let events = store
            .lock()
            .unwrap()
            .event_tail_from(None)
            .expect("failed to query events");
        assert!(!events.is_empty());
        assert_eq!(events[0].kind, "TestEvent");

        println!("Direct store write test passed!");
    }

    #[test]
    fn test_bus_with_manual_thread() {
        // Test that callbacks can send to channel and a thread can receive and write
        let temp_file = NamedTempFile::new().expect("failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap();
        let store = Arc::new(Mutex::new(
            Store::open(db_path).expect("failed to open store"),
        ));

        let (tx, rx) = mpsc::channel::<EventRow>();
        let mut bus = EventBus::new();

        #[allow(clippy::redundant_clone)]
        bus.on({
            let tx = tx.clone();
            move |event: &WorkspaceOpened| {
                let row = EventRow {
                    id: ulid::Ulid::new().to_string(),
                    workspace_id: None, // NULL to avoid FK constraint
                    kind: "WorkspaceOpened".to_string(),
                    payload_json: format!(r#"{{"workspace_id": "{}"}}"#, event.workspace_id),
                    created_at: "2026-04-18T00:00:00.000Z".to_string(),
                };
                let _ = tx.send(row);
            }
        });

        let store_clone = store.clone();
        let _handle = thread::spawn(move || {
            // Keep receiving until channel closes (which never happens for daemon)
            // For testing, we just let it run in the background
            while let Ok(event_row) = rx.recv() {
                if let Ok(store) = store_clone.lock() {
                    let _ = store.event_append(&event_row);
                    println!("Wrote event: {:?}", event_row.kind);
                }
            }
        });

        // Emit event
        let ws_id = WorkspaceId::new();
        println!("Emitting event for workspace: {ws_id}");
        bus.emit(WorkspaceOpened {
            workspace_id: ws_id,
        });

        // Give thread time to process
        std::thread::sleep(Duration::from_millis(100));

        // Check store
        let events = store
            .lock()
            .unwrap()
            .event_tail_from(None)
            .expect("failed to query events");
        println!("Events in store: {}", events.len());
        for e in &events {
            println!("  - {}: {}", e.kind, e.id);
        }
        assert!(!events.is_empty(), "expected at least one event");
        assert_eq!(events[0].kind, "WorkspaceOpened");
    }
}
