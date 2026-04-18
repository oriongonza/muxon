//! Pure planning layer for Resurreccion capture and restore operations.
//!
//! Planning is a pure function: `(state, intent) -> Plan`.
//! Execution is the only side-effecting layer.
//! Plans are inspectable, dry-runnable, and testable without touching real runtimes.

pub mod verbs;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// Unique identifier for a plan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Ulid);

impl NodeId {
    /// Generate a new random node ID.
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// A single step in a plan DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNode {
    /// Unique ID for this node.
    pub id: NodeId,
    /// The capability verb this node invokes (e.g. `"capture.layout"`).
    pub verb: String,
    /// Verb-specific arguments as JSON.
    pub args: serde_json::Value,
    /// IDs of nodes that must complete before this node runs.
    pub depends_on: Vec<NodeId>,
}

impl PlanNode {
    /// Create a leaf node (no dependencies).
    #[must_use]
    pub fn leaf(verb: impl Into<String>, args: serde_json::Value) -> Self {
        Self {
            id: NodeId::new(),
            verb: verb.into(),
            args,
            depends_on: vec![],
        }
    }
}

/// A directed acyclic graph of [`PlanNode`]s.
///
/// Capture plans and restore plans are both represented as `Plan`s;
/// they are duals over the same capability graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// All nodes in the DAG. Execution order is determined by `depends_on`.
    pub nodes: Vec<PlanNode>,
    /// Human-readable description of what this plan does.
    pub description: String,
    /// If `true`, the plan should be displayed but not executed.
    pub dry_run: bool,
}

impl Plan {
    /// An empty plan with no nodes.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn empty() -> Self {
        Self {
            nodes: vec![],
            description: String::new(),
            dry_run: false,
        }
    }

    /// A plan with a single node.
    #[must_use]
    pub fn single(node: PlanNode, description: impl Into<String>) -> Self {
        Self {
            nodes: vec![node],
            description: description.into(),
            dry_run: false,
        }
    }

    /// Mark this plan as dry-run only.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

/// The outcome of executing a plan (or a partial execution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanResult {
    /// Results for each node, in execution order.
    pub node_results: Vec<NodeResult>,
}

/// The result of executing a single plan node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    /// The node that was executed.
    pub node_id: NodeId,
    /// Whether the node succeeded.
    pub success: bool,
    /// Error message if the node failed.
    pub error: Option<String>,
}

/// Execute a plan. Signature only — Lane F fills in the body.
///
/// # Errors
/// Returns an error if the plan cannot be executed at all (e.g., no backend available).
/// Partial failures are reported in [`PlanResult::node_results`].
pub fn execute(_plan: &Plan) -> anyhow::Result<PlanResult> {
    // Lane F: wire to Mux + Store
    unimplemented!("execute — implemented by Lane F")
}
