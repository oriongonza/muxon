//! Tests for event subscription protocol.
use resurreccion_proto::{verbs, SubscribeRequest};

#[test]
fn subscribe_request_serializes() {
    let req = SubscribeRequest {
        workspace_id: Some("ws1".into()),
        after_id: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    let decoded: SubscribeRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.workspace_id, Some("ws1".to_string()));
    assert_eq!(decoded.after_id, None);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn events_subscribe_verb_nonempty() {
    assert!(!verbs::EVENTS_SUBSCRIBE.is_empty());
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn events_push_verb_nonempty() {
    assert!(!verbs::EVENTS_PUSH.is_empty());
}
