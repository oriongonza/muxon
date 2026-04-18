//! Events subscription handler for the Resurreccion daemon.
//!
//! Implements the `EVENTS_SUBSCRIBE` verb as a single-shot response:
//! the handler calls `event_tail_from(after_id)` once and returns all
//! matching events as a JSON array.  True streaming (polling loop) is
//! deferred to a later sprint; the wire contract is identical either way.

use crate::dispatch::Handler;
use resurreccion_proto::{Envelope, SubscribeRequest};
use resurreccion_store::Store;
use std::sync::{Arc, Mutex};

/// Handler for `events.subscribe` — return events since `after_id`.
///
/// Reads `SubscribeRequest { workspace_id, after_id }` from the envelope body,
/// queries the store's `event_tail_from(after_id)`, and returns all matching
/// events as a JSON array under the `events.push` verb.
pub struct EventsSubscribeHandler {
    store: Arc<Mutex<Store>>,
}

impl EventsSubscribeHandler {
    /// Create a new events-subscribe handler backed by `store`.
    pub const fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for EventsSubscribeHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        // Parse the SubscribeRequest from the envelope body.
        let req: SubscribeRequest = match serde_json::from_value(env.body.clone()) {
            Ok(r) => r,
            Err(e) => {
                return Envelope::err(
                    &env.id,
                    &env.verb,
                    "invalid_request",
                    format!("could not parse SubscribeRequest: {e}"),
                );
            }
        };

        // Pull events from the store.
        // Extract the result before the match so the MutexGuard is dropped promptly.
        let tail_result = self
            .store
            .lock()
            .unwrap()
            .event_tail_from(req.after_id.as_deref());
        let events = match tail_result {
            Ok(rows) => rows,
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "internal", e.to_string());
            }
        };

        // Optionally filter by workspace_id if present in the request.
        let events = if let Some(ref ws_id) = req.workspace_id {
            events
                .into_iter()
                .filter(|row| row.workspace_id.as_deref() == Some(ws_id.as_str()))
                .collect()
        } else {
            events
        };

        // Respond with EVENTS_PUSH verb containing the event array.
        Envelope::ok(
            &env.id,
            resurreccion_proto::verbs::EVENTS_PUSH,
            serde_json::to_value(events).unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_proto::{verbs, Envelope, SubscribeRequest, PROTO_VERSION};
    use resurreccion_store::{types::EventRow, Store, WorkspaceRow};
    use std::sync::{Arc, Mutex};
    use tempfile::NamedTempFile;

    /// Build an in-memory store backed by a temp `SQLite` file.
    ///
    /// Returns both the store handle and the `NamedTempFile` guard.  The caller
    /// must keep the guard alive for the duration of the test; dropping it early
    /// causes `SQLite` to lose the backing file and subsequent writes fail.
    fn make_store() -> (Arc<Mutex<Store>>, NamedTempFile) {
        let f = NamedTempFile::new().expect("tempfile");
        let store = Arc::new(Mutex::new(
            Store::open(f.path().to_str().unwrap()).expect("store::open"),
        ));
        (store, f)
    }

    /// Build a test envelope carrying a `SubscribeRequest`.
    fn make_envelope(req: &SubscribeRequest) -> Envelope {
        Envelope {
            id: "test-id".to_string(),
            verb: verbs::EVENTS_SUBSCRIBE.to_string(),
            proto: PROTO_VERSION,
            ok: true,
            body: serde_json::to_value(req).unwrap(),
            ts: 0,
        }
    }

    /// Insert one synthetic event row into the store.
    ///
    /// Use `workspace_id: None` unless a corresponding workspace row has already
    /// been inserted; the schema enforces a FK constraint on non-null values.
    fn insert_event(store: &Arc<Mutex<Store>>, id: &str, workspace_id: Option<&str>, kind: &str) {
        let row = EventRow {
            id: id.to_string(),
            workspace_id: workspace_id.map(ToString::to_string),
            kind: kind.to_string(),
            payload_json: "{}".to_string(),
            created_at: "2026-04-18T00:00:00.000Z".to_string(),
        };
        store.lock().unwrap().event_append(&row).expect("insert");
    }

    /// Insert a minimal workspace row so FK-constrained event inserts succeed.
    fn insert_workspace(store: &Arc<Mutex<Store>>, id: &str) {
        let row = WorkspaceRow {
            id: id.to_string(),
            binding_key: format!("key-{id}"),
            display_name: id.to_string(),
            root_path: "/tmp".to_string(),
            created_at: "2026-04-18T00:00:00.000Z".to_string(),
            last_opened_at: None,
        };
        store
            .lock()
            .unwrap()
            .workspace_insert(&row)
            .expect("insert workspace");
    }

    // ── no events ────────────────────────────────────────────────────────────

    #[test]
    fn empty_store_returns_empty_array() {
        let (store, _guard) = make_store();
        let handler = EventsSubscribeHandler::new(store);
        let req = SubscribeRequest {
            workspace_id: None,
            after_id: None,
        };
        let resp = handler.handle(&make_envelope(&req));

        assert!(resp.ok, "expected ok response");
        assert_eq!(resp.verb, verbs::EVENTS_PUSH);
        let arr = resp.body.as_array().expect("body is array");
        assert!(arr.is_empty());
    }

    // ── all events when after_id is None ────────────────────────────────────

    #[test]
    fn returns_all_events_when_no_after_id() {
        let (store, _guard) = make_store();
        insert_event(&store, "evt-001", None, "Kind1");
        insert_event(&store, "evt-002", None, "Kind2");

        let handler = EventsSubscribeHandler::new(store);
        let req = SubscribeRequest {
            workspace_id: None,
            after_id: None,
        };
        let resp = handler.handle(&make_envelope(&req));

        assert!(resp.ok);
        assert_eq!(resp.verb, verbs::EVENTS_PUSH);
        let arr = resp.body.as_array().expect("array");
        assert_eq!(arr.len(), 2);
    }

    // ── after_id returns only newer events ───────────────────────────────────

    #[test]
    fn returns_events_after_given_id() {
        let (store, _guard) = make_store();
        insert_event(&store, "evt-001", None, "Old");
        insert_event(&store, "evt-002", None, "New");

        let handler = EventsSubscribeHandler::new(store);
        let req = SubscribeRequest {
            workspace_id: None,
            after_id: Some("evt-001".to_string()),
        };
        let resp = handler.handle(&make_envelope(&req));

        assert!(resp.ok);
        let arr = resp.body.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["kind"], "New");
    }

    // ── workspace_id filter ──────────────────────────────────────────────────

    #[test]
    fn filters_by_workspace_id() {
        let (store, _guard) = make_store();

        // Pre-insert workspaces so FK constraint is satisfied.
        insert_workspace(&store, "ws-aaa");
        insert_workspace(&store, "ws-bbb");
        insert_event(&store, "evt-001", Some("ws-aaa"), "MatchingKind");
        insert_event(&store, "evt-002", Some("ws-bbb"), "OtherKind");

        let handler = EventsSubscribeHandler::new(store);
        let req = SubscribeRequest {
            workspace_id: Some("ws-aaa".to_string()),
            after_id: None,
        };
        let resp = handler.handle(&make_envelope(&req));

        assert!(resp.ok);
        let arr = resp.body.as_array().expect("array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["kind"], "MatchingKind");
    }

    // ── invalid request body ─────────────────────────────────────────────────

    #[test]
    fn invalid_body_returns_error() {
        let (store, _guard) = make_store();
        let handler = EventsSubscribeHandler::new(store);

        let bad_env = Envelope {
            id: "bad-id".to_string(),
            verb: verbs::EVENTS_SUBSCRIBE.to_string(),
            proto: PROTO_VERSION,
            ok: true,
            // An array is not a valid SubscribeRequest object.
            body: serde_json::json!([1, 2, 3]),
            ts: 0,
        };

        let resp = handler.handle(&bad_env);
        assert!(!resp.ok);
        assert_eq!(resp.body["code"].as_str().unwrap_or(""), "invalid_request");
    }

    // ── verb of reply is always EVENTS_PUSH ─────────────────────────────────

    #[test]
    fn reply_verb_is_events_push() {
        let (store, _guard) = make_store();
        let handler = EventsSubscribeHandler::new(store);
        let req = SubscribeRequest {
            workspace_id: None,
            after_id: None,
        };
        let resp = handler.handle(&make_envelope(&req));
        assert_eq!(resp.verb, verbs::EVENTS_PUSH);
    }
}
