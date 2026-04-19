use std::collections::{HashMap, HashSet, VecDeque};

use crate::DagEdge;

/// Every declared node must lie on at least one source-to-sink path.
///
/// A node that is not forward-reachable from any source is **unreachable** —
/// no event can ever arrive at it. A node that cannot reach any sink is a
/// **dead end** — its events propagate into a void.
///
/// Both conditions are reported together so a single validation pass surfaces
/// all wiring gaps at once.
#[derive(Debug)]
pub struct WiringError {
    /// Nodes with no path from any source — nothing can ever reach them.
    pub unreachable: Vec<String>,
    /// Nodes with no path to any sink — their events go nowhere.
    pub dead_ends: Vec<String>,
}

impl std::fmt::Display for WiringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.unreachable.is_empty() {
            writeln!(
                f,
                "unreachable (no path from any source): {}",
                self.unreachable.join(", ")
            )?;
        }
        if !self.dead_ends.is_empty() {
            write!(
                f,
                "dead ends (no path to any sink): {}",
                self.dead_ends.join(", ")
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for WiringError {}

/// Verify that every node in the graph is reachable from at least one source
/// and can reach at least one sink.
///
/// `declared` lists every node that must be wired — typically the full set
/// of node names in the DAG declaration. Nodes that appear only in `declared`
/// but never in `edges` are isolated and will be reported as both unreachable
/// and dead ends.
///
/// When `declared` is empty the check operates only over nodes derived from
/// `edges` (always passes for a valid acyclic graph; use with `declared` for
/// the meaningful completeness guarantee).
///
/// # Errors
///
/// Returns [`WiringError`] listing every node that fails either check.
/// Multiple failures are accumulated so the caller sees all gaps at once.
pub fn check_wiring(declared: &[&str], edges: &[DagEdge]) -> Result<(), WiringError> {
    // All nodes that must be validated.
    let mut all_nodes: HashSet<&str> = declared.iter().copied().collect();
    for e in edges {
        all_nodes.insert(e.from);
        all_nodes.insert(e.to);
    }

    if all_nodes.is_empty() {
        return Ok(());
    }

    // Build adjacency in both directions.
    let mut fwd: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut bwd: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut edge_nodes: HashSet<&str> = HashSet::new();

    for e in edges {
        edge_nodes.insert(e.from);
        edge_nodes.insert(e.to);
        fwd.entry(e.from).or_default().push(e.to);
        bwd.entry(e.to).or_default().push(e.from);
    }

    // Nodes that appear in `declared` but in no edge have no connections at all.
    // They are neither reachable nor can they reach anything — report immediately.
    // (They would trivially pass BFS since they'd be counted as both source and
    // sink, so we must handle them before the reachability pass.)
    let mut isolated: Vec<String> = all_nodes
        .iter()
        .filter(|n| !edge_nodes.contains(*n))
        .map(|n| n.to_string())
        .collect();
    isolated.sort();

    // For the reachability pass, only consider edge-connected nodes.
    let connected_nodes: HashSet<&str> = all_nodes
        .iter()
        .copied()
        .filter(|n| edge_nodes.contains(n))
        .collect();

    // Sources: edge-connected nodes with no incoming edges.
    let sources: Vec<&str> = connected_nodes
        .iter()
        .copied()
        .filter(|n| !bwd.contains_key(n))
        .collect();

    // Sinks: edge-connected nodes with no outgoing edges.
    let sinks: Vec<&str> = connected_nodes
        .iter()
        .copied()
        .filter(|n| !fwd.contains_key(n))
        .collect();

    let forward_reachable = bfs(&sources, &fwd);
    let backward_reachable = bfs(&sinks, &bwd);

    let mut unreachable: Vec<String> = connected_nodes
        .iter()
        .filter(|n| !forward_reachable.contains(*n))
        .map(|n| n.to_string())
        .collect();
    // Isolated nodes are also unreachable — merge and sort once.
    unreachable.extend(isolated.iter().cloned());
    unreachable.sort();
    unreachable.dedup();

    let mut dead_ends: Vec<String> = connected_nodes
        .iter()
        .filter(|n| !backward_reachable.contains(*n))
        .map(|n| n.to_string())
        .collect();
    dead_ends.extend(isolated.iter().cloned());
    dead_ends.sort();
    dead_ends.dedup();

    if unreachable.is_empty() && dead_ends.is_empty() {
        Ok(())
    } else {
        Err(WiringError {
            unreachable,
            dead_ends,
        })
    }
}

fn bfs<'a>(starts: &[&'a str], adj: &HashMap<&'a str, Vec<&'a str>>) -> HashSet<&'a str> {
    let mut visited: HashSet<&'a str> = HashSet::new();
    let mut queue: VecDeque<&'a str> = VecDeque::new();

    for &s in starts {
        if visited.insert(s) {
            queue.push_back(s);
        }
    }

    while let Some(node) = queue.pop_front() {
        if let Some(neighbors) = adj.get(node) {
            for &nb in neighbors {
                if visited.insert(nb) {
                    queue.push_back(nb);
                }
            }
        }
    }

    visited
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DagEdge;

    fn e(from: &'static str, event: &'static str, to: &'static str) -> DagEdge {
        DagEdge::new(from, event, to)
    }

    // ── Empty / trivial ───────────────────────────────────────────────────

    #[test]
    fn empty_edges_no_declared_nodes_is_ok() {
        assert!(check_wiring(&[], &[]).is_ok());
    }

    #[test]
    fn single_edge_is_fully_wired() {
        assert!(check_wiring(&[], &[e("a", "E", "b")]).is_ok());
    }

    // ── Valid DAG topologies ───────────────────────────────────────────────

    #[test]
    fn linear_chain_is_wired() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c"), e("c", "E3", "d")];
        assert!(check_wiring(&[], &edges).is_ok());
    }

    #[test]
    fn diamond_is_wired() {
        let edges = [
            e("a", "E1", "b"),
            e("a", "E2", "c"),
            e("b", "E3", "d"),
            e("c", "E4", "d"),
        ];
        assert!(check_wiring(&[], &edges).is_ok());
    }

    #[test]
    fn fan_out_is_wired() {
        let edges = [
            e("source", "E1", "sink_a"),
            e("source", "E2", "sink_b"),
        ];
        assert!(check_wiring(&[], &edges).is_ok());
    }

    #[test]
    fn fan_in_is_wired() {
        let edges = [
            e("source_a", "E1", "sink"),
            e("source_b", "E2", "sink"),
        ];
        assert!(check_wiring(&[], &edges).is_ok());
    }

    // ── Isolated declared nodes (the key wiring guarantee) ────────────────

    #[test]
    fn declared_node_missing_from_all_edges_is_unwired() {
        // "orphan" is declared in the DAG but wired to nothing.
        let edges = [e("protocol", "WorkspaceListCmd", "store")];
        let err = check_wiring(&["protocol", "store", "orphan"], &edges).unwrap_err();

        assert!(err.unreachable.contains(&"orphan".to_string()));
        assert!(err.dead_ends.contains(&"orphan".to_string()));
    }

    #[test]
    fn all_declared_nodes_present_in_edges_is_ok() {
        let edges = [e("protocol", "WorkspaceListCmd", "store")];
        assert!(check_wiring(&["protocol", "store"], &edges).is_ok());
    }

    #[test]
    fn extra_declared_sources_and_sinks_are_ok() {
        // Declaring more nodes than appear in edges, but all wired.
        let edges = [
            e("a", "E1", "b"),
            e("b", "E2", "c"),
        ];
        assert!(check_wiring(&["a", "b", "c"], &edges).is_ok());
    }

    // ── Multiple unwired nodes reported together ───────────────────────────

    #[test]
    fn multiple_unwired_nodes_all_reported() {
        let edges = [e("a", "E", "b")];
        let err = check_wiring(&["a", "b", "x", "y"], &edges).unwrap_err();

        assert!(err.unreachable.contains(&"x".to_string()));
        assert!(err.unreachable.contains(&"y".to_string()));
    }

    // ── Error display ─────────────────────────────────────────────────────

    #[test]
    fn wiring_error_display_lists_problem_nodes() {
        let err = WiringError {
            unreachable: vec!["ghost".into()],
            dead_ends: vec!["void".into()],
        };
        let s = err.to_string();
        assert!(s.contains("ghost"));
        assert!(s.contains("void"));
    }

    // ── Disconnected components ────────────────────────────────────────────

    #[test]
    fn disconnected_components_each_with_source_and_sink_are_ok() {
        // Two independent flows — each has its own beginning and end.
        let edges = [
            e("a", "E1", "b"),
            e("c", "E2", "d"),
        ];
        assert!(check_wiring(&[], &edges).is_ok());
    }
}
