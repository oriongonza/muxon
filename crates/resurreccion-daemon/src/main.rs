//! Resurreccion daemon — the async runtime for the Resurreccion system.

use resurreccion_daemon::{
    Dispatcher, EventsTailHandler, Handler, SnapshotCreateHandler, SnapshotGetHandler,
    SnapshotListHandler, SnapshotRestoreHandler, WorkspaceCreateHandler, WorkspaceGetHandler,
    WorkspaceListHandler, WorkspaceOpenHandler, WorkspaceResolveOrCreateHandler,
};
use resurreccion_mux::Mux;
use resurreccion_proto::verbs;
use resurreccion_store::Store;
use resurreccion_zellij::ZellijMux;
use std::sync::{Arc, Mutex};

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

    // Initialize store
    let store_path = get_store_path()?;
    let store = Store::open(&store_path)?;
    let store = Arc::new(Mutex::new(store));

    // Initialize Mux
    let mux: Arc<dyn Mux> = Arc::new(ZellijMux);

    // Create dispatcher
    let mut dispatcher = Dispatcher::new();

    // Register handlers
    dispatcher.register(verbs::DOCTOR_PING, Arc::new(DoctorPingHandler));
    dispatcher.register(verbs::HANDSHAKE, Arc::new(HandshakeHandler));

    // Register workspace handlers
    dispatcher.register(
        verbs::WORKSPACE_LIST,
        Arc::new(WorkspaceListHandler::new(store.clone())),
    );
    dispatcher.register(
        verbs::WORKSPACE_CREATE,
        Arc::new(WorkspaceCreateHandler::new(store.clone())),
    );
    dispatcher.register(
        verbs::WORKSPACE_GET,
        Arc::new(WorkspaceGetHandler::new(store.clone())),
    );
    dispatcher.register(
        verbs::WORKSPACE_RESOLVE_OR_CREATE,
        Arc::new(WorkspaceResolveOrCreateHandler::new(store.clone())),
    );
    dispatcher.register(
        verbs::WORKSPACE_OPEN,
        Arc::new(WorkspaceOpenHandler::new(store.clone())),
    );

    // Register snapshot handlers
    dispatcher.register(
        verbs::SNAPSHOT_CREATE,
        Arc::new(SnapshotCreateHandler::new(store.clone(), mux.clone())),
    );
    dispatcher.register(
        verbs::SNAPSHOT_RESTORE,
        Arc::new(SnapshotRestoreHandler::new(store.clone(), mux.clone())),
    );
    dispatcher.register(
        verbs::SNAPSHOT_LIST,
        Arc::new(SnapshotListHandler::new(store.clone())),
    );
    dispatcher.register(
        verbs::SNAPSHOT_GET,
        Arc::new(SnapshotGetHandler::new(store.clone())),
    );

    // Register events handlers
    dispatcher.register(
        verbs::EVENTS_TAIL,
        Arc::new(EventsTailHandler::new(store.clone())),
    );

    let dispatcher = Arc::new(dispatcher);

    // Get socket path
    let socket_path = resurreccion_proto::default_socket_path();

    // Run daemon
    resurreccion_daemon::runtime::run(socket_path, dispatcher).await
}

/// Get the store database path from environment or use default.
fn get_store_path() -> anyhow::Result<String> {
    if let Ok(path) = std::env::var("RESURRECCION_STORE_PATH") {
        return Ok(path);
    }

    let data_dir = directories::ProjectDirs::from("dev", "orion", "resurreccion")
        .ok_or_else(|| anyhow::anyhow!("could not determine project directory"))?;

    let data_path = data_dir.data_dir();
    std::fs::create_dir_all(data_path)?;

    Ok(data_path.join("store.db").to_string_lossy().to_string())
}
