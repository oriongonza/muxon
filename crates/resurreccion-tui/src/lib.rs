//! TUI data model for the Resurreccion workspace tree view.
//!
//! Provides a pure data model representing workspaces, runtimes, and
//! snapshots in a collapsible tree hierarchy. Rendering (ratatui/crossterm)
//! is intentionally deferred to a later crate.

use std::fmt::Write as _;

/// A node in the workspace tree hierarchy.
///
/// Nodes form a tree: `Workspace` nodes contain `Runtime` and `Snapshot`
/// children; `Runtime` nodes contain `Snapshot` children.
#[derive(Debug, Clone)]
pub enum TreeNode {
    /// A workspace node with nested runtimes and snapshots.
    Workspace {
        /// Unique workspace identifier.
        id: String,
        /// Human-readable workspace name.
        name: String,
        /// Child nodes (runtimes, snapshots) belonging to this workspace.
        children: Vec<Self>,
    },
    /// A runtime node representing an active multiplexer session.
    Runtime {
        /// Unique runtime identifier.
        id: String,
        /// Multiplexer session name.
        session_name: String,
        /// Backend name (e.g. "zellij").
        backend: String,
        /// Child nodes (snapshots) belonging to this runtime.
        children: Vec<Self>,
    },
    /// A snapshot node representing a captured workspace state.
    Snapshot {
        /// Unique snapshot identifier.
        id: String,
        /// Fidelity level of the snapshot (e.g. "layout").
        fidelity: String,
        /// ISO-8601 creation timestamp.
        created_at: String,
    },
}

/// The root container for the workspace tree.
///
/// `WorkspaceTree` owns a list of top-level workspace nodes and provides
/// mutation helpers for incrementally building the tree.
pub struct WorkspaceTree {
    /// Top-level workspace nodes.
    pub roots: Vec<TreeNode>,
}

impl WorkspaceTree {
    /// Create an empty workspace tree.
    pub const fn new() -> Self {
        Self { roots: Vec::new() }
    }

    /// Append a workspace node at the root level.
    ///
    /// Returns `&mut Self` for method chaining.
    pub fn add_workspace(&mut self, id: impl Into<String>, name: impl Into<String>) -> &mut Self {
        self.roots.push(TreeNode::Workspace {
            id: id.into(),
            name: name.into(),
            children: vec![],
        });
        self
    }

    /// Append a runtime node as a child of the given workspace.
    ///
    /// If `workspace_id` does not match any root workspace this is a no-op.
    /// Returns `&mut Self` for method chaining.
    pub fn add_runtime(
        &mut self,
        workspace_id: &str,
        id: impl Into<String>,
        session_name: impl Into<String>,
        backend: impl Into<String>,
    ) -> &mut Self {
        for node in &mut self.roots {
            if let TreeNode::Workspace {
                id: wid, children, ..
            } = node
            {
                if wid == workspace_id {
                    children.push(TreeNode::Runtime {
                        id: id.into(),
                        session_name: session_name.into(),
                        backend: backend.into(),
                        children: vec![],
                    });
                    return self;
                }
            }
        }
        self
    }

    /// Append a snapshot node as a direct child of the given workspace.
    ///
    /// Snapshots are added directly under their workspace. If `workspace_id`
    /// does not match any root workspace this is a no-op.
    /// Returns `&mut Self` for method chaining.
    pub fn add_snapshot(
        &mut self,
        workspace_id: &str,
        id: impl Into<String>,
        fidelity: impl Into<String>,
        created_at: impl Into<String>,
    ) -> &mut Self {
        for node in &mut self.roots {
            if let TreeNode::Workspace {
                id: wid, children, ..
            } = node
            {
                if wid == workspace_id {
                    children.push(TreeNode::Snapshot {
                        id: id.into(),
                        fidelity: fidelity.into(),
                        created_at: created_at.into(),
                    });
                    return self;
                }
            }
        }
        self
    }

    /// Render the tree as an indented ASCII text representation.
    ///
    /// Each level is indented with two spaces relative to its parent.
    ///
    /// # Example output
    ///
    /// ```text
    /// workspace: my-project
    ///   runtime: zellij/my-session (zellij)
    ///     snapshot: layout [2026-04-18T08:00:00Z]
    /// workspace: another-project
    /// ```
    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for root in &self.roots {
            render_node(&mut out, root, 0);
        }
        out
    }
}

impl Default for WorkspaceTree {
    fn default() -> Self {
        Self::new()
    }
}

/// Recursively render a single `TreeNode` at the given indentation depth.
fn render_node(out: &mut String, node: &TreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        TreeNode::Workspace { name, children, .. } => {
            writeln!(out, "{indent}workspace: {name}").unwrap();
            for child in children {
                render_node(out, child, depth + 1);
            }
        }
        TreeNode::Runtime {
            session_name,
            backend,
            children,
            ..
        } => {
            writeln!(out, "{indent}runtime: {backend}/{session_name} ({backend})").unwrap();
            for child in children {
                render_node(out, child, depth + 1);
            }
        }
        TreeNode::Snapshot {
            fidelity,
            created_at,
            ..
        } => {
            writeln!(out, "{indent}snapshot: {fidelity} [{created_at}]").unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    // --- TDD failing tests written first, implementation added after ---

    #[test]
    fn add_workspace_appears_in_render_text() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "my-project");
        let text = tree.render_text();
        assert!(
            text.contains("my-project"),
            "render_text should contain workspace name; got:\n{text}"
        );
        assert!(
            text.contains("workspace:"),
            "render_text should have 'workspace:' prefix; got:\n{text}"
        );
    }

    #[test]
    fn add_runtime_under_workspace_is_indented() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "my-project");
        tree.add_runtime("ws-1", "rt-1", "my-session", "zellij");
        let text = tree.render_text();
        let lines: Vec<&str> = text.lines().collect();
        // Workspace line must not be indented.
        assert!(
            lines[0].starts_with("workspace:"),
            "first line should be the workspace; got: {:?}",
            lines[0]
        );
        // Runtime line must be indented relative to workspace.
        assert!(
            lines[1].starts_with("  runtime:"),
            "runtime line should be indented; got: {:?}",
            lines[1]
        );
        assert!(
            text.contains("my-session"),
            "runtime session name must appear; got:\n{text}"
        );
        assert!(text.contains("zellij"), "backend must appear; got:\n{text}");
    }

    #[test]
    fn add_snapshot_under_workspace_is_further_indented() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "my-project");
        tree.add_snapshot("ws-1", "snap-1", "layout", "2026-04-18T08:00:00Z");
        let text = tree.render_text();
        let lines: Vec<&str> = text.lines().collect();
        assert!(
            lines[1].starts_with("  snapshot:"),
            "snapshot should be indented under workspace; got: {:?}",
            lines[1]
        );
        assert!(
            text.contains("layout"),
            "fidelity must appear; got:\n{text}"
        );
        assert!(
            text.contains("2026-04-18T08:00:00Z"),
            "created_at must appear; got:\n{text}"
        );
    }

    #[test]
    fn full_tree_render_matches_expected() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "my-project");
        tree.add_runtime("ws-1", "rt-1", "my-session", "zellij");
        // snapshot is added at the workspace level (workspace_id is the key)
        tree.add_snapshot("ws-1", "snap-1", "layout", "2026-04-18T08:00:00Z");
        tree.add_workspace("ws-2", "another-project");

        let text = tree.render_text();
        let expected = concat!(
            "workspace: my-project\n",
            "  runtime: zellij/my-session (zellij)\n",
            "  snapshot: layout [2026-04-18T08:00:00Z]\n",
            "workspace: another-project\n",
        );
        assert_eq!(text, expected);
    }

    #[test]
    fn default_creates_empty_tree() {
        let tree = WorkspaceTree::default();
        assert!(tree.roots.is_empty());
        assert_eq!(tree.render_text(), "");
    }

    #[test]
    fn add_runtime_to_unknown_workspace_is_noop() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "my-project");
        tree.add_runtime("ws-unknown", "rt-1", "session", "zellij");
        // Only the workspace line, no runtime.
        let text = tree.render_text();
        assert_eq!(text.lines().count(), 1);
    }

    #[test]
    fn method_chaining_works() {
        let mut tree = WorkspaceTree::new();
        tree.add_workspace("ws-1", "proj-a")
            .add_workspace("ws-2", "proj-b");
        assert_eq!(tree.roots.len(), 2);
    }
}
