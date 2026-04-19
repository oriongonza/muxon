//! Daemon event topology — the executable architecture diagram.
//!
//! Every edge here simultaneously declares:
//! - Which Rust type flows along that arc (the event struct)
//! - Which node emits it (`from`)
//! - Which node receives it (`to`)
//!
//! # Nodes
//!
//! | Name       | Role                                         |
//! |------------|----------------------------------------------|
//! | `Protocol` | Protocol/verb layer — receives CLI commands  |
//! | `Mux`      | Multiplexer backend (Zellij)                 |
//! | `Store`    | Persistent storage for workspaces/snapshots  |
//! | `Recorder` | Durable event log (appends to the event table) |
//!
//! # Structural guarantees
//!
//! Call [`verify`] once at startup before accepting any connections.
//! A wiring error is always a programming mistake, never a runtime condition.

use resurreccion_core::events::{
    FocusChanged, LayoutChanged, PaneClosed, PaneOpened, RuntimeAttached, RuntimeDetached,
    SnapshotCreated, SnapshotRestored, WorkspaceClosed, WorkspaceOpened,
};
use rt_events_dag::{check_completeness, check_termination, edges, node, node_id, DagEdge, NodeId};

node!(Protocol);
node!(Mux);
node!(Store);
node!(Recorder);

/// All event arcs in the daemon.
///
/// Each line reads: `EventType: emitting_node -> receiving_node`.
/// Removing any event type from resurreccion-core will produce a
/// compile error at the edge that references it.
pub const EDGES: &[DagEdge] = edges![
    // Workspace lifecycle — protocol layer emits; store records
    WorkspaceOpened:  Protocol -> Recorder,
    WorkspaceClosed:  Protocol -> Recorder,
    // Multiplexer lifecycle — mux backend emits; store records
    RuntimeAttached:  Mux      -> Recorder,
    RuntimeDetached:  Mux      -> Recorder,
    PaneOpened:       Mux      -> Recorder,
    PaneClosed:       Mux      -> Recorder,
    FocusChanged:     Mux      -> Recorder,
    LayoutChanged:    Mux      -> Recorder,
    // Snapshot outcomes — store emits after commit; recorder logs them
    SnapshotCreated:  Store    -> Recorder,
    SnapshotRestored: Store    -> Recorder,
];

/// Node identities explicitly declared in this topology.
const NODES: &[NodeId] = &[
    node_id::<Protocol>(),
    node_id::<Mux>(),
    node_id::<Store>(),
    node_id::<Recorder>(),
];

/// Verify structural integrity of the event topology.
///
/// Checks:
/// 1. **Termination** — no event can transitively trigger itself (no cycles).
/// 2. **Completeness** — every declared node lies on a source-to-sink path.
///
/// # Errors
///
/// Returns the first structural violation found. A failed check is a fatal
/// programming error — abort startup rather than proceeding with a broken wiring.
///
/// # Panics
///
/// Does not panic. Errors are returned, not panicked.
pub fn verify() -> anyhow::Result<()> {
    check_termination(EDGES).map_err(|e| anyhow::anyhow!("event wiring: {e}"))?;
    check_completeness(NODES, EDGES).map_err(|e| anyhow::anyhow!("event wiring: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_is_acyclic() {
        check_termination(EDGES).expect("event topology must not contain cycles");
    }

    #[test]
    fn all_declared_nodes_are_wired() {
        check_completeness(NODES, EDGES)
            .expect("every declared node must lie on a source-to-sink path");
    }

    #[test]
    fn edge_count_matches_domain_events() {
        assert_eq!(EDGES.len(), 10, "one edge per domain event type");
    }
}
