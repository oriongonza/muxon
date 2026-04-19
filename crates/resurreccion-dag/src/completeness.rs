use std::collections::{HashMap, HashSet, VecDeque};

use crate::DagEdge;

/// The completeness guarantee was violated: one or more nodes are not
/// on any source-to-sink path.
///
/// Both failure classes are reported together so a single startup check
/// surfaces all wiring gaps at once.
#[derive(Debug)]
pub struct CompletenessViolation {
    /// Nodes with no path from any source — nothing can ever reach them.
    pub unreachable: Vec<String>,
    /// Nodes with no path to any sink — their events propagate into a void.
    pub dead_ends: Vec<String>,
}

impl std::fmt::Display for CompletenessViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "completeness violated:")?;
        if !self.unreachable.is_empty() {
            write!(f, " unreachable: [{}]", self.unreachable.join(", "))?;
        }
        if !self.dead_ends.is_empty() {
            if !self.unreachable.is_empty() {
                write!(f, ";")?;
            }
            write!(f, " dead ends: [{}]", self.dead_ends.join(", "))?;
        }
        Ok(())
    }
}

impl std::error::Error for CompletenessViolation {}

/// Verify the **completeness guarantee**: every declared node lies on at
/// least one path from a source to a sink.
///
/// `declared` lists every node name that must be wired. Nodes that appear
/// in `declared` but in no edge are isolated — they are reported as both
/// unreachable and dead ends. When `declared` is empty, the check operates
/// only over nodes derived from `edges`.
///
/// Algorithm: forward BFS from all sources, backward BFS from all sinks
/// (on the reversed graph). Every node must appear in both reachable sets.
///
/// # Errors
///
/// Returns [`CompletenessViolation`] listing every node that fails either
/// check. Multiple failures are accumulated so the caller sees all gaps
/// at once. Treat this as a fatal startup error.
pub fn check_completeness(declared: &[&str], edges: &[DagEdge]) -> Result<(), CompletenessViolation> {
    let mut all_nodes: HashSet<&str> = declared.iter().copied().collect();
    for e in edges {
        all_nodes.insert(e.from);
        all_nodes.insert(e.to);
    }

    if all_nodes.is_empty() {
        return Ok(());
    }

    let mut fwd: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut bwd: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut edge_nodes: HashSet<&str> = HashSet::new();

    for e in edges {
        edge_nodes.insert(e.from);
        edge_nodes.insert(e.to);
        fwd.entry(e.from).or_default().push(e.to);
        bwd.entry(e.to).or_default().push(e.from);
    }

    // Nodes in `declared` but absent from every edge — isolated, caught before
    // BFS since they would trivially pass (simultaneously source and sink).
    let mut isolated: Vec<String> = all_nodes
        .iter()
        .filter(|n| !edge_nodes.contains(*n))
        .map(|n| n.to_string())
        .collect();
    isolated.sort();

    let connected: HashSet<&str> = all_nodes
        .iter()
        .copied()
        .filter(|n| edge_nodes.contains(n))
        .collect();

    let sources: Vec<&str> = connected.iter().copied().filter(|n| !bwd.contains_key(n)).collect();
    let sinks: Vec<&str> = connected.iter().copied().filter(|n| !fwd.contains_key(n)).collect();

    let fwd_reachable = bfs(&sources, &fwd);
    let bwd_reachable = bfs(&sinks, &bwd);

    let mut unreachable: Vec<String> = connected
        .iter()
        .filter(|n| !fwd_reachable.contains(*n))
        .map(|n| n.to_string())
        .collect();
    unreachable.extend(isolated.iter().cloned());
    unreachable.sort();
    unreachable.dedup();

    let mut dead_ends: Vec<String> = connected
        .iter()
        .filter(|n| !bwd_reachable.contains(*n))
        .map(|n| n.to_string())
        .collect();
    dead_ends.extend(isolated.iter().cloned());
    dead_ends.sort();
    dead_ends.dedup();

    if unreachable.is_empty() && dead_ends.is_empty() {
        Ok(())
    } else {
        Err(CompletenessViolation { unreachable, dead_ends })
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

    #[test]
    fn empty_is_complete() {
        assert!(check_completeness(&[], &[]).is_ok());
    }

    #[test]
    fn single_edge_is_complete() {
        assert!(check_completeness(&[], &[e("a", "E", "b")]).is_ok());
    }

    #[test]
    fn linear_chain_is_complete() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c"), e("c", "E3", "d")];
        assert!(check_completeness(&[], &edges).is_ok());
    }

    #[test]
    fn diamond_is_complete() {
        let edges = [
            e("a", "E1", "b"),
            e("a", "E2", "c"),
            e("b", "E3", "d"),
            e("c", "E4", "d"),
        ];
        assert!(check_completeness(&[], &edges).is_ok());
    }

    #[test]
    fn fan_out_is_complete() {
        let edges = [e("source", "E1", "sink_a"), e("source", "E2", "sink_b")];
        assert!(check_completeness(&[], &edges).is_ok());
    }

    #[test]
    fn fan_in_is_complete() {
        let edges = [e("source_a", "E1", "sink"), e("source_b", "E2", "sink")];
        assert!(check_completeness(&[], &edges).is_ok());
    }

    #[test]
    fn declared_node_absent_from_edges_violates_completeness() {
        let edges = [e("protocol", "WorkspaceListCmd", "store")];
        let err = check_completeness(&["protocol", "store", "orphan"], &edges).unwrap_err();
        assert!(err.unreachable.contains(&"orphan".to_string()));
        assert!(err.dead_ends.contains(&"orphan".to_string()));
    }

    #[test]
    fn all_declared_nodes_in_edges_is_complete() {
        let edges = [e("protocol", "WorkspaceListCmd", "store")];
        assert!(check_completeness(&["protocol", "store"], &edges).is_ok());
    }

    #[test]
    fn multiple_unwired_nodes_all_reported() {
        let edges = [e("a", "E", "b")];
        let err = check_completeness(&["a", "b", "x", "y"], &edges).unwrap_err();
        assert!(err.unreachable.contains(&"x".to_string()));
        assert!(err.unreachable.contains(&"y".to_string()));
    }

    #[test]
    fn disconnected_components_are_complete() {
        let edges = [e("a", "E1", "b"), e("c", "E2", "d")];
        assert!(check_completeness(&[], &edges).is_ok());
    }

    #[test]
    fn display_shows_completeness_violated_prefix() {
        let err = CompletenessViolation {
            unreachable: vec!["ghost".into()],
            dead_ends: vec!["void".into()],
        };
        let s = err.to_string();
        assert!(s.starts_with("completeness violated:"));
        assert!(s.contains("ghost"));
        assert!(s.contains("void"));
    }
}
