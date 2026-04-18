//! Tests for the proto envelope serialization and verb constants.
use resurreccion_proto::{verbs, Envelope, Request, PROTO_VERSION};

#[test]
#[allow(clippy::assertions_on_constants)]
fn proto_version_is_nonzero() {
    assert!(PROTO_VERSION > 0);
}

#[test]
fn envelope_round_trips() {
    let env = Envelope::ok(
        "test-id",
        "doctor.ping",
        serde_json::json!({"status": "ok"}),
    );
    let json = serde_json::to_string(&env).unwrap();
    let decoded: Envelope = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.verb, "doctor.ping");
    assert!(decoded.ok);
}

#[test]
fn envelope_error_round_trips() {
    let env = Envelope::err("test-id", "doctor.ping", "not_found", "workspace missing");
    let json = serde_json::to_string(&env).unwrap();
    let decoded: Envelope = serde_json::from_str(&json).unwrap();
    assert!(!decoded.ok);
}

#[test]
fn verb_constants_are_nonempty() {
    assert!(!verbs::DOCTOR_PING.is_empty());
    assert!(!verbs::WORKSPACE_OPEN.is_empty());
    assert!(!verbs::SNAPSHOT_CREATE.is_empty());
}

#[test]
fn request_serializes() {
    let req = Request::doctor_ping();
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("doctor.ping"));
}
