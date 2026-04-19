use std::collections::{HashMap, HashSet};

use crate::DagEdge;

/// The topology: precomputed structure derived from a set of [`DagEdge`]s.
///
/// `Topology` is the `t` in `walk(t, v) -> a`. It is computed once and
/// may be walked many times with different visitors.
pub struct Topology<'a> {
    edges: &'a [DagEdge],
    /// Forward adjacency: from → [(event, to), …]
    fwd: HashMap<&'a str, Vec<(&'a str, &'a str)>>,
    /// All node names appearing in any edge.
    nodes: HashSet<&'a str>,
}

impl<'a> Topology<'a> {
    pub fn new(edges: &'a [DagEdge]) -> Self {
        let mut fwd: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
        let mut nodes: HashSet<&str> = HashSet::new();
        for e in edges {
            nodes.insert(e.from);
            nodes.insert(e.to);
            fwd.entry(e.from).or_default().push((e.event, e.to));
        }
        Self { edges, fwd, nodes }
    }

    /// Edges as declared.
    pub fn edges(&self) -> &[DagEdge] {
        self.edges
    }

    /// All node names appearing in at least one edge.
    pub fn nodes(&self) -> impl Iterator<Item = &str> + '_ {
        self.nodes.iter().copied()
    }

    /// Source nodes: no incoming edges — the beginnings of all flows.
    pub fn sources(&self) -> impl Iterator<Item = &str> + '_ {
        let has_incoming: HashSet<&str> = self.edges.iter().map(|e| e.to).collect();
        self.nodes.iter().copied().filter(move |n| !has_incoming.contains(n))
    }

    /// Sink nodes: no outgoing edges — the ends of all flows.
    pub fn sinks(&self) -> impl Iterator<Item = &str> + '_ {
        self.nodes.iter().copied().filter(|n| !self.fwd.contains_key(n))
    }

    /// Outgoing edges from a node as `(event, to)` pairs.
    pub fn neighbors(&self, node: &str) -> &[(& str, &str)] {
        self.fwd.get(node).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// The visitor: what to do at each step of a walk.
///
/// This is the `v` in `walk(t, v) -> a` — the programmer's only concern.
/// The walk drives traversal; the visitor defines behavior.
///
/// # Event → Node → Event
///
/// Each visit reflects the runtime model: an event arrives at a node
/// (`enter`), the node processes it and emits into the graph. Back-edges
/// mark cycles (`back_edge`). Post-order cleanup (`leave`) mirrors the
/// node's emission completing.
pub trait Visitor {
    /// The answer produced when the walk finishes.
    type Answer;

    /// DFS pre-order: a node is entered for the first time.
    ///
    /// `via` = `(event_name, from_node)` that led here; `None` for sources.
    fn enter(&mut self, node: &str, via: Option<(&str, &str)>);

    /// A back-edge was found: `from` already on the current DFS stack,
    /// reached again via `event` from `current`.
    ///
    /// Default: no-op (completeness visitor ignores back-edges).
    fn back_edge(&mut self, current: &str, event: &str, to: &str) {
        let _ = (current, event, to);
    }

    /// DFS post-order: all nodes reachable from `node` have been explored.
    ///
    /// Default: no-op (termination visitor ignores post-order).
    fn leave(&mut self, node: &str) {
        let _ = node;
    }

    /// Consume the visitor and produce the answer.
    fn finish(self) -> Self::Answer;
}

/// Walk the topology depth-first, invoking the visitor at each step.
///
/// Starts from every source node (in-degree 0), then from any remaining
/// unvisited nodes (e.g., nodes in isolated cycles that have no source).
/// Every node in the topology is visited exactly once.
pub fn walk<'a, V: Visitor>(topology: &'a Topology<'a>, mut visitor: V) -> V::Answer {
    let mut state: HashMap<&str, DfsColor> = HashMap::new();

    for start in topology.sources() {
        if !state.contains_key(start) {
            dfs(start, None, topology, &mut visitor, &mut state);
        }
    }

    // Second pass: nodes unreachable from any source (e.g., isolated cycles).
    for node in topology.nodes() {
        if !state.contains_key(node) {
            dfs(node, None, topology, &mut visitor, &mut state);
        }
    }

    visitor.finish()
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum DfsColor {
    InStack,
    Done,
}

fn dfs<'a, V: Visitor>(
    node: &'a str,
    via: Option<(&'a str, &'a str)>, // (event, from_node)
    topology: &'a Topology<'a>,
    visitor: &mut V,
    state: &mut HashMap<&'a str, DfsColor>,
) {
    state.insert(node, DfsColor::InStack);
    visitor.enter(node, via);

    for &(event, neighbor) in topology.neighbors(node) {
        match state.get(neighbor).copied() {
            Some(DfsColor::InStack) => visitor.back_edge(node, event, neighbor),
            Some(DfsColor::Done) => {}
            None => dfs(neighbor, Some((event, node)), topology, visitor, state),
        }
    }

    state.insert(node, DfsColor::Done);
    visitor.leave(node);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DagEdge;

    fn e(from: &'static str, event: &'static str, to: &'static str) -> DagEdge {
        DagEdge::new(from, event, to)
    }

    /// Collect visited node names in DFS pre-order.
    struct Collector(Vec<String>);
    impl Visitor for Collector {
        type Answer = Vec<String>;
        fn enter(&mut self, node: &str, _: Option<(&str, &str)>) { self.0.push(node.into()); }
        fn finish(self) -> Vec<String> { self.0 }
    }

    #[test]
    fn walk_visits_all_nodes_in_linear_chain() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c")];
        let t = Topology::new(&edges);
        let visited = walk(&t, Collector(vec![]));
        assert_eq!(visited.len(), 3);
        assert_eq!(visited[0], "a"); // source first
    }

    #[test]
    fn walk_visits_every_node_in_diamond() {
        let edges = [e("a","E1","b"), e("a","E2","c"), e("b","E3","d"), e("c","E4","d")];
        let t = Topology::new(&edges);
        let visited = walk(&t, Collector(vec![]));
        assert_eq!(visited.len(), 4);
        assert!(visited.contains(&"a".into()));
        assert!(visited.contains(&"d".into()));
    }

    #[test]
    fn topology_sources_have_no_incoming_edges() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c")];
        let t = Topology::new(&edges);
        let sources: Vec<_> = t.sources().collect();
        assert_eq!(sources, ["a"]);
    }

    #[test]
    fn topology_sinks_have_no_outgoing_edges() {
        let edges = [e("a", "E1", "b"), e("b", "E2", "c")];
        let t = Topology::new(&edges);
        let sinks: Vec<_> = t.sinks().collect();
        assert_eq!(sinks, ["c"]);
    }

    #[test]
    fn back_edge_called_on_cycle() {
        struct CycleDetector(bool);
        impl Visitor for CycleDetector {
            type Answer = bool;
            fn enter(&mut self, _: &str, _: Option<(&str, &str)>) {}
            fn back_edge(&mut self, _: &str, _: &str, _: &str) { self.0 = true; }
            fn finish(self) -> bool { self.0 }
        }

        let edges = [e("a", "E1", "b"), e("b", "E2", "a")];
        let t = Topology::new(&edges);
        assert!(walk(&t, CycleDetector(false)));
    }

    #[test]
    fn via_carries_incoming_event_and_source() {
        struct EdgeRecorder(Vec<(Option<String>, Option<String>)>);
        impl Visitor for EdgeRecorder {
            type Answer = Vec<(Option<String>, Option<String>)>;
            fn enter(&mut self, _: &str, via: Option<(&str, &str)>) {
                self.0.push(via.map(|(ev, from)| (ev.to_string(), from.to_string()))
                    .map_or((None, None), |(ev, from)| (Some(ev), Some(from))));
            }
            fn finish(self) -> Self::Answer { self.0 }
        }

        let edges = [e("src", "MyEvent", "dst")];
        let t = Topology::new(&edges);
        let records = walk(&t, EdgeRecorder(vec![]));
        // src is a source: via = None
        assert!(records.iter().any(|(ev, _)| ev.is_none()));
        // dst is reached via MyEvent from src
        assert!(records.iter().any(|(ev, _)| ev.as_deref() == Some("MyEvent")));
    }
}
