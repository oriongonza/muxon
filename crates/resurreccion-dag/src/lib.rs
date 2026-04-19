//! DAG topology for event-driven architectures.
//!
//! This is the seed of the future `rt-event-dag` crate.
//!
//! # Core invariant
//!
//! An **edge is an event type** — declaring [`DagEdge`] declares both the
//! data that flows and the arc in the program's execution graph.
//! The two concepts are the same thing at different levels of description.
//!
//! # Termination guarantee
//!
//! A valid (acyclic) event DAG can never loop through event propagation.
//! This sidesteps the halting problem for the class of programs expressible
//! as this DAG: termination is structurally enforced, not reasoned about.
//!
//! [`check_acyclic`] enforces the invariant at startup.

mod cycle;
mod wiring;

pub use cycle::{check_acyclic, CycleError, DagEdge};
pub use wiring::{check_wiring, WiringError};
