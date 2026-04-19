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
///
/// Prefer [`edge_of`] over `DagEdge::new` when `event` is a Rust type that
/// implements [`DagEvent`] — it ties the string to the type at compile time.
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

/// Marker trait for event types that participate in the DAG.
///
/// Implementing this trait on a struct makes it a **first-class DAG event**:
/// - `NAME` is the canonical string used in [`DagEdge::event`]
/// - [`edge_of::<Self>`] creates edges that fail to compile when `Self` is removed
///
/// Use the [`dag_event!`] macro to define event types and implement this trait
/// together in one declaration:
///
/// ```rust
/// # use resurreccion_dag::dag_event;
/// dag_event!(WorkspaceCreated { id: u64 });
/// dag_event!(RefreshCmd);
/// ```
pub trait DagEvent: 'static {
    /// Canonical event name — used as [`DagEdge::event`] in the wiring.
    const NAME: &'static str;
}

/// Create a [`DagEdge`] whose event type is enforced at compile time.
///
/// `edge_of::<WorkspaceCreated>("protocol", "store")` will fail to compile if
/// `WorkspaceCreated` is removed or no longer implements [`DagEvent`].
///
/// ```rust
/// # use resurreccion_dag::{dag_event, edge_of, DagEdge};
/// dag_event!(Ping);
/// let edge: DagEdge = edge_of::<Ping>("a", "b");
/// assert_eq!(edge.event, "Ping");
/// ```
pub fn edge_of<E: DagEvent>(from: &'static str, to: &'static str) -> DagEdge {
    DagEdge::new(from, E::NAME, to)
}

/// Define an event struct and implement [`DagEvent`] for it in one step.
///
/// ```rust
/// # use resurreccion_dag::{dag_event, DagEvent};
/// dag_event!(MyEvent { x: u32, y: u32 });
/// dag_event!(UnitEvent);
///
/// assert_eq!(MyEvent::NAME, "MyEvent");
/// assert_eq!(UnitEvent::NAME, "UnitEvent");
/// ```
#[macro_export]
macro_rules! dag_event {
    ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
        pub struct $name { $(pub $field: $ty,)* }
        impl $crate::DagEvent for $name {
            const NAME: &'static str = stringify!($name);
        }
    };
    ($name:ident) => {
        pub struct $name;
        impl $crate::DagEvent for $name {
            const NAME: &'static str = stringify!($name);
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    dag_event!(Ping);
    dag_event!(Pong { seq: u32 });

    #[test]
    fn dag_event_name_matches_type_name() {
        assert_eq!(Ping::NAME, "Ping");
        assert_eq!(Pong::NAME, "Pong");
    }

    #[test]
    fn dag_event_struct_has_fields() {
        let p = Pong { seq: 42 };
        assert_eq!(p.seq, 42);
    }

    #[test]
    fn edge_of_produces_correct_dag_edge() {
        let e = edge_of::<Ping>("a", "b");
        assert_eq!(e.from, "a");
        assert_eq!(e.event, "Ping");
        assert_eq!(e.to, "b");
    }

    #[test]
    fn edge_of_and_dag_edge_new_are_equivalent() {
        let via_macro = edge_of::<Pong>("x", "y");
        let via_ctor = DagEdge::new("x", "Pong", "y");
        assert_eq!(via_macro, via_ctor);
    }

    #[test]
    fn edge_array_built_from_typed_edges() {
        let edges = [edge_of::<Ping>("a", "b"), edge_of::<Pong>("b", "c")];
        assert!(check_termination(&edges).is_ok());
        assert!(check_completeness(&[], &edges).is_ok());
    }
}
