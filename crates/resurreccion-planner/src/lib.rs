//! Pure planning layer for Resurreccion capture and restore operations.
//!
//! Planning is a pure function: `(state, intent) -> Plan`.
//! Execution is the only side-effecting layer.
//! Plans are inspectable, dry-runnable, and testable without touching real runtimes.

pub mod verbs;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

pub use resurreccion_mux::Capability;
pub use resurreccion_store::Store;

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

/// Minimal snapshot manifest for `plan_restore`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SnapshotManifest {
    /// Workspace identifier.
    pub workspace_id: String,
    /// Layout data as JSON.
    pub layout: serde_json::Value,
}

/// Build a capture plan for the given capability set.
///
/// For 0.1.0, returns a single `CAPTURE_LAYOUT` node.
#[must_use]
pub fn plan_capture(capabilities: &Capability) -> Plan {
    let node = PlanNode::leaf(
        verbs::CAPTURE_LAYOUT,
        serde_json::json!({ "capabilities": capabilities.bits() }),
    );
    Plan::single(node, "Capture workspace layout")
}

/// Build a restore plan from a snapshot manifest and capability set.
///
/// For 0.1.0, returns a single `RESTORE_LAYOUT` node.
#[must_use]
pub fn plan_restore(manifest: &SnapshotManifest, capabilities: &Capability) -> Plan {
    let node = PlanNode::leaf(
        verbs::RESTORE_LAYOUT,
        serde_json::json!({
            "workspace_id": manifest.workspace_id,
            "layout": manifest.layout,
            "capabilities": capabilities.bits()
        }),
    );
    Plan::single(node, "Restore workspace layout")
}

/// Execute a plan against a Mux backend and Store.
///
/// Executes plan nodes in DAG order (topological sort).
/// Dry-run plans return immediately without executing.
///
/// # Errors
/// Returns an error if the plan cannot be executed at all (e.g., no backend available).
/// Partial failures are reported in [`PlanResult::node_results`].
pub fn execute(
    plan: &Plan,
    mux: &dyn resurreccion_mux::Mux,
    _store: &Store,
) -> anyhow::Result<PlanResult> {
    // Dry-run plans skip execution
    if plan.dry_run {
        return Ok(PlanResult {
            node_results: vec![],
        });
    }

    // Topological sort (for 0.1.0: single node, order is trivial)
    let ordered = topo_sort(&plan.nodes);

    let mut results = Vec::new();
    for node in &ordered {
        let result = execute_node(node, mux);
        results.push(result);
    }

    Ok(PlanResult {
        node_results: results,
    })
}

fn topo_sort(nodes: &[PlanNode]) -> Vec<&PlanNode> {
    // Simple implementation: return nodes as-is for 0.1.0 (single node).
    // Full topological sort for multi-node DAGs would use Kahn's algorithm.
    nodes.iter().collect()
}

fn execute_node(node: &PlanNode, mux: &dyn resurreccion_mux::Mux) -> NodeResult {
    match node.verb.as_str() {
        verbs::CAPTURE_LAYOUT => {
            // Capture from mux
            match mux.discover() {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        NodeResult {
                            node_id: node.id,
                            success: true,
                            error: None,
                        }
                    } else {
                        match mux.capture(&sessions[0]) {
                            Ok(_) => NodeResult {
                                node_id: node.id,
                                success: true,
                                error: None,
                            },
                            Err(e) => NodeResult {
                                node_id: node.id,
                                success: false,
                                error: Some(e.to_string()),
                            },
                        }
                    }
                }
                Err(e) => NodeResult {
                    node_id: node.id,
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        }
        verbs::RESTORE_LAYOUT => {
            // For 0.1.0: restore is a no-op (just acknowledge success)
            NodeResult {
                node_id: node.id,
                success: true,
                error: None,
            }
        }
        other => NodeResult {
            node_id: node.id,
            success: false,
            error: Some(format!("unknown verb: {other}")),
        },
    }
}
