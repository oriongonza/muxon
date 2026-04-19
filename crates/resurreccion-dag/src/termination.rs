use crate::{DagEdge, Topology, Visitor, walk};

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
    let topology = Topology::new(edges);
    walk(&topology, CycleVisitor::default())
}

/// Visitor that detects cycles and records the first one found.
///
/// Maintains a DFS path stack (node + incoming event) so that when a
/// back-edge is found, the full cycle trace can be extracted immediately.
#[derive(Default)]
struct CycleVisitor {
    /// (node_name, incoming_event) — mirrors the live DFS stack.
    stack: Vec<(String, Option<String>)>,
    violation: Option<TerminationViolation>,
}

impl Visitor for CycleVisitor {
    type Answer = Result<(), TerminationViolation>;

    fn enter(&mut self, node: &str, via: Option<(&str, &str)>) {
        self.stack.push((node.to_string(), via.map(|(ev, _)| ev.to_string())));
    }

    fn back_edge(&mut self, _current: &str, event: &str, to: &str) {
        if self.violation.is_some() {
            return;
        }
        let start = self.stack.iter().position(|(n, _)| n == to).unwrap_or(0);
        let cycle_segment = &self.stack[start..];

        let mut trace: Vec<String> = Vec::with_capacity(cycle_segment.len() * 2 + 1);
        trace.push(cycle_segment[0].0.clone());
        for (node, incoming) in &cycle_segment[1..] {
            trace.push(incoming.clone().unwrap());
            trace.push(node.clone());
        }
        trace.push(event.to_string());
        trace.push(to.to_string());

        self.violation = Some(TerminationViolation { trace });
    }

    fn leave(&mut self, _node: &str) {
        self.stack.pop();
    }

    fn finish(self) -> Result<(), TerminationViolation> {
        match self.violation {
            Some(v) => Err(v),
            None => Ok(()),
        }
    }
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
