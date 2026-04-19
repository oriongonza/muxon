//! DAG topology for event-driven architectures.
//!
//! This is the seed of the future `rt-event-dag` crate.
//!
//! # Core invariant
//!
//! An **edge is an event type** — declaring [`DagEdge`] declares both the
//! data that flows and the arc in the program's execution graph.
//!
//! # Two structural guarantees
//!
//! ```text
//! check_termination(edges)?;             // no cycles  → event propagation halts
//! check_completeness(&declared, edges)?  // all wired  → every node is live
//! ```
//!
//! Together they give a program whose execution graph is fully described,
//! fully connected, and provably finite.

mod completeness;
mod termination;

pub use completeness::{check_completeness, CompletenessViolation};
pub use termination::{check_termination, TerminationViolation};

/// A directed edge in the event DAG.
///
/// An edge **is** an event type. `from --[event]--> to` declares:
/// - which node emits the event (`from`)
/// - the event type name — the edge's identity (`event`)
/// - which node handles it (`to`)
///
/// One event type may fan out to multiple sinks via multiple `DagEdge`
/// values sharing the same `event` name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagEdge {
    /// Emitting node name.
    pub from: &'static str,
    /// Event type name — the edge's identity.
    pub event: &'static str,
    /// Receiving node name.
    pub to: &'static str,
}

impl DagEdge {
    pub const fn new(from: &'static str, event: &'static str, to: &'static str) -> Self {
        Self { from, event, to }
    }
}
