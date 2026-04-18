//! Tests for capability negotiation types and verb.
use resurreccion_proto::{verbs, CapabilityRequest, CapabilityResponse};

#[test]
fn capability_request_serializes() {
    let req = CapabilityRequest {
        client_proto: 1,
        client_capabilities: vec!["layout.capture".into()],
    };
    let json = serde_json::to_string(&req).unwrap();
    let decoded: CapabilityRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.client_proto, 1);
    assert_eq!(decoded.client_capabilities, vec!["layout.capture"]);
}

#[test]
fn capability_response_round_trips() {
    let resp = CapabilityResponse {
        server_proto: 1,
        agreed_capabilities: vec!["layout.capture".into()],
    };
    let json = serde_json::to_string(&resp).unwrap();
    let decoded: CapabilityResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.server_proto, 1);
    assert_eq!(decoded.agreed_capabilities, vec!["layout.capture"]);
}

#[test]
#[allow(clippy::assertions_on_constants)]
fn capability_negotiate_verb_nonempty() {
    assert!(!verbs::CAPABILITY_NEGOTIATE.is_empty());
}
