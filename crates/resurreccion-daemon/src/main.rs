//! Resurreccion daemon — the async runtime for the Resurreccion system.

use resurreccion_daemon::{Dispatcher, Handler};
use resurreccion_proto::verbs;
use std::sync::Arc;

struct DoctorPingHandler;

impl Handler for DoctorPingHandler {
    fn handle(&self, env: &resurreccion_proto::Envelope) -> resurreccion_proto::Envelope {
        resurreccion_proto::Envelope::ok(&env.id, &env.verb, serde_json::json!({"proto": 1}))
    }
}

struct HandshakeHandler;

impl Handler for HandshakeHandler {
    fn handle(&self, env: &resurreccion_proto::Envelope) -> resurreccion_proto::Envelope {
        resurreccion_proto::Envelope::ok(&env.id, &env.verb, serde_json::json!({"proto": 1}))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    // Create dispatcher
    let mut dispatcher = Dispatcher::new();

    // Register handlers
    dispatcher.register(verbs::DOCTOR_PING, Arc::new(DoctorPingHandler));
    dispatcher.register(verbs::HANDSHAKE, Arc::new(HandshakeHandler));

    let dispatcher = Arc::new(dispatcher);

    // Get socket path
    let socket_path = resurreccion_proto::default_socket_path();

    // Run daemon
    resurreccion_daemon::runtime::run(socket_path, dispatcher).await
}
