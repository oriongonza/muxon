#![warn(missing_docs)]

//! Resurreccion daemon library — provides the async runtime, dispatcher, and handlers.

/// Event bus and subscribers for the daemon.
pub mod bus;
/// Verb dispatch and handler trait.
pub mod dispatch;
/// Async daemon runtime with graceful shutdown.
pub mod runtime;

#[cfg(test)]
mod bus_test;

pub use bus::setup_store_subscriber;
pub use dispatch::{Dispatcher, Handler};
pub use runtime::single_instance_guard;
