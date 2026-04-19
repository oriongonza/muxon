use std::collections::{HashMap, HashSet};

/// A directed edge in the event DAG.
///
/// An edge **is** an event type. `from --[event]--> to` declares:
/// - the node that emits the event (`from`)
/// - the event type name, which is the edge's identity (`event`)
/// - the node that handles it (`to`)
///
/// One event type may appear on multiple edges (fan-out), but each
/// `(from, event, to)` triple must be unique.
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

/// A cycle was detected in the event DAG.
///
/// `trace` is an alternating sequence of node names and event names,
/// starting and ending at the same node:
///
/// ```text
/// [node₀, event₀, node₁, event₁, …, nodeₙ]   where nodeₙ == node₀
/// ```
///
/// This encodes the minimal detected cycle path. The [`Display`] impl
/// renders it as:
/// ```text
/// store -[WorkspaceCreated]-> tui -[RefreshCmd]-> store
/// ```
///
/// [`Display`]: std::fmt::Display
#[derive(Debug)]
pub struct CycleError {
    pub trace: Vec<String>,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let t = &self.trace;
        if t.is_empty() {
            return write!(f, "cycle detected (empty trace)");
        }
        write!(f, "{}", t[0])?;
        let mut i = 1;
        while i < t.len() {
            // t[i] = event name, t[i+1] = next node
            if i + 1 < t.len() {
                write!(f, " -[{}]-> {}", t[i], t[i + 1])?;
                i += 2;
            } else {
                // malformed trace; render gracefully
                write!(f, " → {}", t[i])?;
                i += 1;
            }
        }
        Ok(())
    }
}

impl std::error::Error for CycleError {}

/// Verify that `edges` form an acyclic directed graph.
///
/// Uses DFS with three-color marking (unvisited → in-stack → done).
/// Discovers cycles in O(V + E) time.
///
/// # Errors
///
/// Returns [`CycleError`] with the detected cycle's node/event trace
/// if any cycle exists. Caller should treat this as a fatal startup error:
/// a cyclic event graph has no termination guarantee.
pub fn check_acyclic(edges: &[DagEdge]) -> Result<(), CycleError> {
    // Build adjacency list: from → [(to, event), …]
    let mut adj: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    let mut all_nodes: HashSet<&str> = HashSet::new();

    for e in edges {
        all_nodes.insert(e.from);
        all_nodes.insert(e.to);
        adj.entry(e.from).or_default().push((e.to, e.event));
    }

    let mut visited: HashMap<&str, Color> = HashMap::new();
    // Current DFS path as (node, outgoing_event) pairs.
    // path[i] = (node_i, event_that_goes_to_node_i+1)
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
    /// Currently on the DFS stack — a back-edge here means a cycle.
    InStack,
    /// Fully explored — safe to revisit without re-expanding.
    Done,
}

fn dfs<'a>(
    current: &'a str,
    adj: &HashMap<&'a str, Vec<(&'a str, &'a str)>>,
    visited: &mut HashMap<&'a str, Color>,
    path: &mut Vec<(&'a str, &'a str)>,
) -> Option<CycleError> {
    visited.insert(current, Color::InStack);

    if let Some(neighbors) = adj.get(current) {
        for &(neighbor, event) in neighbors {
            match visited.get(neighbor).copied() {
                Some(Color::InStack) => {
                    // Back-edge found: `current -[event]-> neighbor` closes a cycle.
                    // Reconstruct the cycle from where `neighbor` first appears in `path`.
                    return Some(build_cycle_error(current, event, neighbor, path));
                }
                Some(Color::Done) => {
                    // Cross-edge: already fully explored, no cycle through here.
                }
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

/// Build a [`CycleError`] from the back-edge `current -[event]-> cycle_entry`.
///
/// `path` contains the DFS path as `(node, outgoing_event)` pairs.
/// The cycle is the suffix of `path` starting where `cycle_entry` appears,
/// extended by the closing back-edge.
fn build_cycle_error(
    current: &str,
    event: &str,
    cycle_entry: &str,
    path: &[(&str, &str)],
) -> CycleError {
    // Find the position in `path` where cycle_entry first appears as a node.
    let start = path
        .iter()
        .position(|(n, _)| *n == cycle_entry)
        .unwrap_or(0);

    // Build alternating [node, event, node, event, …, closing_node] trace.
    let mut trace: Vec<String> = Vec::new();
    for (n, e) in &path[start..] {
        trace.push(n.to_string());
        trace.push(e.to_string());
    }
    // Close the cycle: current node, closing event, back to cycle_entry.
    trace.push(current.to_string());
    trace.push(event.to_string());
    trace.push(cycle_entry.to_string());

    CycleError { trace }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(from: &'static str, event: &'static str, to: &'static str) -> DagEdge {
        DagEdge::new(from, event, to)
    }

    // ── Acyclic cases (must return Ok) ────────────────────────────────────

    #[test]
    fn empty_graph_is_acyclic() {
        assert!(check_acyclic(&[]).is_ok());
    }

    #[test]
    fn single_edge_is_acyclic() {
        assert!(check_acyclic(&[edge("a", "E", "b")]).is_ok());
    }

    #[test]
    fn linear_chain_is_acyclic() {
        let edges = [edge("a", "E1", "b"), edge("b", "E2", "c"), edge("c", "E3", "d")];
        assert!(check_acyclic(&edges).is_ok());
    }

    #[test]
    fn diamond_is_acyclic() {
        // a → b, a → c, b → d, c → d  (diamond shape, not a cycle)
        let edges = [
            edge("a", "E1", "b"),
            edge("a", "E2", "c"),
            edge("b", "E3", "d"),
            edge("c", "E4", "d"),
        ];
        assert!(check_acyclic(&edges).is_ok());
    }

    #[test]
    fn disconnected_acyclic_components() {
        let edges = [
            edge("a", "E1", "b"),
            edge("c", "E2", "d"),
        ];
        assert!(check_acyclic(&edges).is_ok());
    }

    #[test]
    fn fan_out_is_acyclic() {
        // One event type going to multiple sinks is fine.
        let edges = [
            edge("protocol", "WorkspaceCreated", "tui"),
            edge("protocol", "WorkspaceCreated", "search"),
            edge("protocol", "WorkspaceCreated", "store"),
        ];
        assert!(check_acyclic(&edges).is_ok());
    }

    // ── Cyclic cases (must return Err) ────────────────────────────────────

    #[test]
    fn self_loop_is_cycle() {
        let err = check_acyclic(&[edge("a", "E", "a")]).unwrap_err();
        // trace: ["a", "E", "a"]
        assert_eq!(err.trace.len(), 3);
        assert_eq!(err.trace[0], "a");
        assert_eq!(err.trace[2], "a");
    }

    #[test]
    fn two_node_cycle() {
        let err = check_acyclic(&[
            edge("a", "Ping", "b"),
            edge("b", "Pong", "a"),
        ])
        .unwrap_err();
        // Must contain both nodes
        let nodes: Vec<_> = err.trace.iter().step_by(2).collect();
        assert!(nodes.contains(&&"a".to_string()));
        assert!(nodes.contains(&&"b".to_string()));
        // First and last node of trace are the same (cycle is closed)
        assert_eq!(err.trace.first(), err.trace.last());
    }

    #[test]
    fn three_node_cycle() {
        let err = check_acyclic(&[
            edge("a", "E1", "b"),
            edge("b", "E2", "c"),
            edge("c", "E3", "a"),
        ])
        .unwrap_err();
        // trace length for 3-node cycle: 3 nodes + 3 events + 1 closing = 7
        assert_eq!(err.trace.len(), 7);
        assert_eq!(err.trace.first(), err.trace.last());
    }

    #[test]
    fn cycle_among_otherwise_valid_edges() {
        // Most edges are fine; one sub-graph contains a cycle.
        let err = check_acyclic(&[
            edge("source", "E1", "a"),
            edge("a", "E2", "b"),
            edge("b", "E3", "c"),  // c → b is the cycle
            edge("c", "E4", "b"),
            edge("c", "E5", "sink"),
        ])
        .unwrap_err();
        // Cycle is b → c → b (or c → b → c); either way trace is closed
        assert_eq!(err.trace.first(), err.trace.last());
    }

    // ── Display format ────────────────────────────────────────────────────

    #[test]
    fn display_formats_as_arrow_chain() {
        let err = CycleError {
            trace: vec![
                "store".into(), "WorkspaceCreated".into(),
                "tui".into(),   "RefreshCmd".into(),
                "store".into(),
            ],
        };
        let s = err.to_string();
        assert_eq!(s, "store -[WorkspaceCreated]-> tui -[RefreshCmd]-> store");
    }

    #[test]
    fn display_self_loop() {
        let err = CycleError {
            trace: vec!["a".into(), "E".into(), "a".into()],
        };
        assert_eq!(err.to_string(), "a -[E]-> a");
    }

    // ── Trace content ─────────────────────────────────────────────────────

    #[test]
    fn trace_includes_event_names() {
        // Event names are the edges — they must appear in the trace.
        let err = check_acyclic(&[
            edge("p", "CycleEvent", "q"),
            edge("q", "BackEvent", "p"),
        ])
        .unwrap_err();
        let events: Vec<_> = err.trace.iter().skip(1).step_by(2).collect();
        // At least one of the two event names must appear
        assert!(
            events.contains(&&"CycleEvent".to_string())
                || events.contains(&&"BackEvent".to_string())
        );
    }
}
