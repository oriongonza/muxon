#![allow(missing_docs)]

use resurreccion_daemon::Dispatcher;
use resurreccion_proto::{verbs, Envelope};
use std::sync::Arc;

#[test]
fn handler_map_dispatches_ping() {
    // Create a mock handler for doctor.ping
    #[derive(Clone)]
    struct PingHandler;
    impl resurreccion_daemon::Handler for PingHandler {
        fn handle(&self, env: &Envelope) -> Envelope {
            Envelope::ok(&env.id, &env.verb, serde_json::json!({"proto": 1}))
        }
    }

    let mut dispatcher = Dispatcher::new();
    dispatcher.register(verbs::DOCTOR_PING, Arc::new(PingHandler));

    let request = Envelope::ok("test-id", verbs::DOCTOR_PING, serde_json::json!(null));
    let response = dispatcher.dispatch(&request);

    assert!(response.ok);
    assert_eq!(response.verb, verbs::DOCTOR_PING);
    assert_eq!(response.id, "test-id");
}

#[test]
fn handler_map_returns_error_for_unknown_verb() {
    let dispatcher = Dispatcher::new();

    let request = Envelope::ok("test-id", "unknown.verb", serde_json::json!(null));
    let response = dispatcher.dispatch(&request);

    assert!(!response.ok);
    assert_eq!(response.id, "test-id");
    let body = &response.body;
    assert_eq!(
        body.get("code").and_then(|v| v.as_str()),
        Some("unknown_verb")
    );
}

#[test]
fn single_instance_guard_detects_live_socket() {
    use std::os::unix::net::UnixListener;
    use tempfile::TempDir;

    let temp_dir = TempDir::new().expect("failed to create temp dir");
    let socket_path = temp_dir.path().join("test.sock");

    // Bind a socket to simulate a running daemon
    let _listener = UnixListener::bind(&socket_path).expect("failed to bind socket");

    // Try to run single_instance_guard on the same path
    let result = resurreccion_daemon::single_instance_guard(&socket_path);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("already running"));
}
