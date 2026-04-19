use std::collections::HashMap;

use crate::DagEdge; // defined in lib.rs — shared by both guarantee modules

/// The termination guarantee was violated: a cycle exists in the event DAG.
///
/// `trace` is an alternating sequence of node names and event names,
/// starting and ending at the same node:
///
/// ```text
/// [node₀, event₀, node₁, event₁, …, nodeₙ]   where nodeₙ == node₀
/// ```
///
/// Rendered by [`Display`] as:
/// ```text
/// store -[WorkspaceCreated]-> tui -[RefreshCmd]-> store
/// ```
///
/// [`Display`]: std::fmt::Display
#[derive(Debug)]
pub struct TerminationViolation {
    pub trace: Vec<String>,
}

impl std::fmt::Display for TerminationViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = &self.trace;
        if t.is_empty() {
            return write!(f, "termination violated: cycle detected (empty trace)");
        }
        write!(f, "termination violated: {}", t[0])?;
        let mut i = 1;
        while i < t.len() {
            if i + 1 < t.len() {
                write!(f, " -[{}]-> {}", t[i], t[i + 1])?;
                i += 2;
            } else {
                write!(f, " → {}", t[i])?;
                i += 1;
            }
        }
        Ok(())
    }
}

impl std::error::Error for TerminationViolation {}

/// Verify the **termination guarantee**: no event can trigger itself,
/// directly or transitively.
///
/// A valid event DAG propagates every emission along a finite path that
/// reaches a sink. This check enforces that structurally using DFS
/// three-color marking (O(V + E)).
///
/// # Errors
///
/// Returns [`TerminationViolation`] with the detected cycle's full
/// node/event trace. Treat this as a fatal startup error.
pub fn check_termination(edges: &[DagEdge]) -> Result<(), TerminationViolation> {
    let mut adj: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    let mut all_nodes: std::collections::HashSet<&str> = std::collections::HashSet::new();

    for e in edges {
        all_nodes.insert(e.from);
        all_nodes.insert(e.to);
        adj.entry(e.from).or_default().push((e.to, e.event));
    }

    let mut visited: HashMap<&str, Color> = HashMap::new();
    let mut path: Vec<(&str, &str)> = Vec::new();

    for &node in &all_nodes {
        if !visited.contains_key(node) {
            if let Some(err) = dfs(node, &adj, &mut visited, &mut path) {
                return Err(err);
            }
        }
    }

    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    InStack,
    Done,
}

fn dfs<'a>(
    current: &'a str,
    adj: &HashMap<&'a str, Vec<(&'a str, &'a str)>>,
    visited: &mut HashMap<&'a str, Color>,
    path: &mut Vec<(&'a str, &'a str)>,
) -> Option<TerminationViolation> {
    visited.insert(current, Color::InStack);

    if let Some(neighbors) = adj.get(current) {
        for &(neighbor, event) in neighbors {
            match visited.get(neighbor).copied() {
                Some(Color::InStack) => {
                    return Some(build_violation(current, event, neighbor, path));
                }
                Some(Color::Done) => {}
                None => {
                    path.push((current, event));
                    if let Some(err) = dfs(neighbor, adj, visited, path) {
                        return Some(err);
                    }
                    path.pop();
                }
            }
        }
    }

    visited.insert(current, Color::Done);
    None
}

fn build_violation(
    current: &str,
    event: &str,
    cycle_entry: &str,
    path: &[(&str, &str)],
) -> TerminationViolation {
    let start = path
        .iter()
        .position(|(n, _)| *n == cycle_entry)
        .unwrap_or(0);

    let mut trace: Vec<String> = Vec::new();
    for (n, e) in &path[start..] {
        trace.push(n.to_string());
        trace.push(e.to_string());
    }
    trace.push(current.to_string());
    trace.push(event.to_string());
    trace.push(cycle_entry.to_string());

    TerminationViolation { trace }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(from: &'static str, event: &'static str, to: &'static str) -> DagEdge {
        DagEdge::new(from, event, to)
    }

    #[test]
    fn empty_graph_terminates() {
        assert!(check_termination(&[]).is_ok());
    }

    #[test]
    fn single_edge_terminates() {
        assert!(check_termination(&[e("a", "E", "b")]).is_ok());
    }

    #[test]
    fn linear_chain_terminates() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c"), e("c", "E3", "d")];
        assert!(check_termination(&edges).is_ok());
    }

    #[test]
    fn diamond_terminates() {
        let edges = [
            e("a", "E1", "b"),
            e("a", "E2", "c"),
            e("b", "E3", "d"),
            e("c", "E4", "d"),
        ];
        assert!(check_termination(&edges).is_ok());
    }

    #[test]
    fn disconnected_acyclic_terminates() {
        let edges = [e("a", "E1", "b"), e("c", "E2", "d")];
        assert!(check_termination(&edges).is_ok());
    }

    #[test]
    fn fan_out_terminates() {
        let edges = [
            e("protocol", "WorkspaceCreated", "tui"),
            e("protocol", "WorkspaceCreated", "search"),
            e("protocol", "WorkspaceCreated", "store"),
        ];
        assert!(check_termination(&edges).is_ok());
    }

    #[test]
    fn self_loop_violates_termination() {
        let err = check_termination(&[e("a", "E", "a")]).unwrap_err();
        assert_eq!(err.trace.len(), 3);
        assert_eq!(err.trace[0], "a");
        assert_eq!(err.trace[2], "a");
    }

    #[test]
    fn two_node_cycle_violates_termination() {
        let err =
            check_termination(&[e("a", "Ping", "b"), e("b", "Pong", "a")]).unwrap_err();
        let nodes: Vec<_> = err.trace.iter().step_by(2).collect();
        assert!(nodes.contains(&&"a".to_string()));
        assert!(nodes.contains(&&"b".to_string()));
        assert_eq!(err.trace.first(), err.trace.last());
    }

    #[test]
    fn three_node_cycle_violates_termination() {
        let err = check_termination(&[
            e("a", "E1", "b"),
            e("b", "E2", "c"),
            e("c", "E3", "a"),
        ])
        .unwrap_err();
        assert_eq!(err.trace.len(), 7);
        assert_eq!(err.trace.first(), err.trace.last());
    }

    #[test]
    fn cycle_among_valid_edges_violates_termination() {
        let err = check_termination(&[
            e("source", "E1", "a"),
            e("a", "E2", "b"),
            e("b", "E3", "c"),
            e("c", "E4", "b"),
            e("c", "E5", "sink"),
        ])
        .unwrap_err();
        assert_eq!(err.trace.first(), err.trace.last());
    }

    #[test]
    fn display_shows_termination_violated_prefix() {
        let err = TerminationViolation {
            trace: vec![
                "store".into(),
                "WorkspaceCreated".into(),
                "tui".into(),
                "RefreshCmd".into(),
                "store".into(),
            ],
        };
        let s = err.to_string();
        assert!(s.starts_with("termination violated:"));
        assert!(s.contains("store -[WorkspaceCreated]-> tui -[RefreshCmd]-> store"));
    }

    #[test]
    fn trace_contains_event_names() {
        let err =
            check_termination(&[e("p", "CycleEvent", "q"), e("q", "BackEvent", "p")])
                .unwrap_err();
        let events: Vec<_> = err.trace.iter().skip(1).step_by(2).collect();
        assert!(
            events.contains(&&"CycleEvent".to_string())
                || events.contains(&&"BackEvent".to_string())
        );
    }
}
