#![warn(missing_docs)]

//! Resurreccion daemon library — provides the async runtime, dispatcher, and handlers.

/// Event bus and subscribers for the daemon.
pub mod bus;
/// Event topology — the executable architecture diagram.
pub mod wiring;
/// Capability negotiation handler.
pub mod capability_handler;
/// Verb dispatch and handler trait.
pub mod dispatch;
/// Events subscription handler (`events.subscribe` / `events.push`).
pub mod events_handler;
/// Workspace and other verb handlers.
pub mod handlers;
/// Async daemon runtime with graceful shutdown.
pub mod runtime;

#[cfg(test)]
mod bus_test;

pub use bus::{setup_event_bus, setup_store_subscriber, EventEmitter};
pub use capability_handler::CapabilityHandler;
pub use dispatch::{Dispatcher, Handler};
pub use events_handler::EventsSubscribeHandler;
pub use handlers::{
    EventsTailHandler, SnapshotCreateHandler, SnapshotGetHandler, SnapshotListHandler,
    SnapshotRestoreHandler, WorkspaceCreateHandler, WorkspaceGetHandler, WorkspaceListHandler,
    WorkspaceOpenHandler, WorkspaceResolveOrCreateHandler,
};
pub use runtime::single_instance_guard;
