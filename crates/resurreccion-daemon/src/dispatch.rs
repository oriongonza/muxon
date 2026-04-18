//! Verb dispatch and handler registration.

use resurreccion_proto::Envelope;
use std::collections::HashMap;
use std::sync::Arc;

/// A handler for a specific verb in the daemon protocol.
pub trait Handler: Send + Sync {
    /// Handle a request envelope and return a response envelope.
    fn handle(&self, env: &Envelope) -> Envelope;
}

/// Maps verb names to handlers and dispatches envelopes to the appropriate handler.
pub struct Dispatcher {
    handlers: HashMap<&'static str, Arc<dyn Handler>>,
}

impl Dispatcher {
    /// Create a new empty dispatcher.
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a specific verb.
    pub fn register(&mut self, verb: &'static str, handler: Arc<dyn Handler>) {
        self.handlers.insert(verb, handler);
    }

    /// Dispatch an envelope to the appropriate handler, or return an error envelope.
    pub fn dispatch(&self, env: &Envelope) -> Envelope {
        self.handlers.get(env.verb.as_str()).map_or_else(
            || {
                Envelope::err(
                    &env.id,
                    &env.verb,
                    "unknown_verb",
                    format!("no handler for verb: {}", env.verb),
                )
            },
            |handler| handler.handle(env),
        )
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        Self::new()
    }
}
