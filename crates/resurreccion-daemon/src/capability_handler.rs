//! Handler for the `capability.negotiate` verb.
//!
//! Performs a capability handshake: the client declares its supported
//! capabilities, and the server responds with the intersection of client and
//! server capabilities together with the server protocol version.

use crate::dispatch::Handler;
use resurreccion_proto::{CapabilityRequest, CapabilityResponse, Envelope};

/// Capabilities this server supports.
pub const SERVER_CAPABILITIES: &[&str] = &[
    "capture.layout",
    "restore.layout",
    "shell.capture",
    "shell.restore",
    "blob.put",
    "blob.get",
    "events.subscribe",
];

/// Handler for `capability.negotiate`.
///
/// Reads a [`CapabilityRequest`] from the incoming envelope body, computes the
/// intersection of client-requested and server-supported capabilities, and
/// returns a [`CapabilityResponse`] with `server_proto = 1`.
pub struct CapabilityHandler;

impl Handler for CapabilityHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let req: CapabilityRequest = match serde_json::from_value(env.body.clone()) {
            Ok(r) => r,
            Err(e) => {
                return Envelope::err(
                    &env.id,
                    &env.verb,
                    "bad_request",
                    format!("failed to parse CapabilityRequest: {e}"),
                );
            }
        };

        let agreed_capabilities: Vec<String> = req
            .client_capabilities
            .iter()
            .filter(|cap| SERVER_CAPABILITIES.contains(&cap.as_str()))
            .cloned()
            .collect();

        let resp = CapabilityResponse {
            server_proto: 1,
            agreed_capabilities,
        };

        Envelope::ok(
            &env.id,
            &env.verb,
            serde_json::to_value(resp).unwrap_or_default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_proto::verbs;

    fn make_envelope(body: serde_json::Value) -> Envelope {
        Envelope::ok("test-id", verbs::CAPABILITY_NEGOTIATE, body)
    }

    // Build a CapabilityRequest envelope from a list of client capabilities.
    fn cap_request_envelope(client_caps: &[&str]) -> Envelope {
        let body = serde_json::to_value(CapabilityRequest {
            client_proto: 1,
            client_capabilities: client_caps.iter().map(ToString::to_string).collect(),
        })
        .unwrap();
        make_envelope(body)
    }

    #[test]
    fn returns_intersection_of_known_capabilities() {
        let env = cap_request_envelope(&["capture.layout", "shell.capture", "unknown.verb"]);
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert!(resp.ok, "response should be ok");

        let cap_resp: CapabilityResponse = serde_json::from_value(resp.body).unwrap();
        assert_eq!(cap_resp.server_proto, 1);

        let mut agreed = cap_resp.agreed_capabilities;
        agreed.sort();
        assert_eq!(agreed, vec!["capture.layout", "shell.capture"]);
    }

    #[test]
    fn unknown_capabilities_are_excluded() {
        let env = cap_request_envelope(&["does.not.exist", "also.unknown"]);
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert!(resp.ok);
        let cap_resp: CapabilityResponse = serde_json::from_value(resp.body).unwrap();
        assert!(
            cap_resp.agreed_capabilities.is_empty(),
            "no unknown caps should be agreed upon"
        );
    }

    #[test]
    fn all_server_capabilities_agreed_when_client_requests_all() {
        let env = cap_request_envelope(SERVER_CAPABILITIES);
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert!(resp.ok);
        let cap_resp: CapabilityResponse = serde_json::from_value(resp.body).unwrap();
        assert_eq!(cap_resp.server_proto, 1);

        let mut agreed = cap_resp.agreed_capabilities;
        agreed.sort();
        let mut expected: Vec<String> = SERVER_CAPABILITIES
            .iter()
            .map(ToString::to_string)
            .collect();
        expected.sort();
        assert_eq!(agreed, expected);
    }

    #[test]
    fn empty_client_capabilities_returns_empty_agreed() {
        let env = cap_request_envelope(&[]);
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert!(resp.ok);
        let cap_resp: CapabilityResponse = serde_json::from_value(resp.body).unwrap();
        assert!(cap_resp.agreed_capabilities.is_empty());
    }

    #[test]
    fn malformed_body_returns_error_envelope() {
        let env = make_envelope(serde_json::json!({"not_a_cap_request": true}));
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert!(!resp.ok, "malformed body should produce error envelope");
        assert_eq!(resp.body["code"], "bad_request");
    }

    #[test]
    fn correlation_id_is_echoed() {
        let body = serde_json::to_value(CapabilityRequest {
            client_proto: 1,
            client_capabilities: vec!["capture.layout".to_string()],
        })
        .unwrap();
        let env = Envelope::ok("my-correlation-id", verbs::CAPABILITY_NEGOTIATE, body);
        let handler = CapabilityHandler;
        let resp = handler.handle(&env);

        assert_eq!(resp.id, "my-correlation-id");
    }
}
