#![warn(missing_docs)]

//! Resurreccion daemon library — provides the async runtime, dispatcher, and handlers.

/// Verb dispatch and handler trait.
pub mod dispatch;
/// Async daemon runtime with graceful shutdown.
pub mod runtime;

pub use dispatch::{Dispatcher, Handler};
pub use runtime::single_instance_guard;
