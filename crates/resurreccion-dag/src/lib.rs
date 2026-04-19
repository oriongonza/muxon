//! DAG topology for event-driven architectures.
//!
//! This is the seed of the future `rt-event-dag` crate.
//!
//! # Core model
//!
//! An **edge is an event type**. [`DagEdge`] declares both the data that
//! flows and the arc in the execution graph — one concept, two descriptions.
//!
//! # Structural guarantees via walks
//!
//! Every guarantee is a [`walk`] over the topology with a specific visitor:
//!
//! ```text
//! walk(t, v) -> a
//! ```
//!
//! - `t` — the topology (the program's wiring, derived from edges)
//! - `v` — the visitor (what to do at each node; the programmer's only concern)
//! - `a` — the answer (what the caller cares about)
//!
//! The two built-in guarantees:
//!
//! ```text
//! check_termination(edges)?   // CycleVisitor    → no event can trigger itself
//! check_completeness(d, edges)? // ReachabilityVisitor → every node is live
//! ```
//!
//! New guarantees are new visitors. The walk and the topology never change.
//!
//! # Runtime
//!
//! The event-dispatch loop is the same pattern at runtime:
//! each emitted event is an edge traversal, each handler is a visit.

mod completeness;
mod termination;
mod walk;

pub use completeness::{check_completeness, CompletenessViolation};
pub use termination::{check_termination, TerminationViolation};
pub use walk::{Topology, Visitor, walk};

/// A directed edge in the event DAG.
///
/// An edge **is** an event type. `from --[event]--> to` simultaneously:
/// - names the event type flowing along this arc (`event`)
/// - declares the emitting node (`from`)
/// - declares the receiving node (`to`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagEdge {
    pub from: &'static str,
    pub event: &'static str,
    pub to: &'static str,
}

impl DagEdge {
    pub const fn new(from: &'static str, event: &'static str, to: &'static str) -> Self {
        Self { from, event, to }
    }
}
