//! Zellij plugin renderer for Resurreccion.
//!
//! Renders workspace/runtime/snapshot state as a Zellij pane.
//! Compiled to WebAssembly for Zellij plugin loading.

/// Plugin state.
#[derive(Default)]
pub struct ResurrectionPlugin {
    /// Workspace entries to display.
    pub workspaces: Vec<WorkspaceEntry>,
}

/// A single workspace entry.
#[derive(Debug, Clone)]
pub struct WorkspaceEntry {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Active session name, if any.
    pub session_name: Option<String>,
}

impl ResurrectionPlugin {
    /// Create a new plugin instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the current workspace list.
    pub fn update_workspaces(&mut self, entries: Vec<WorkspaceEntry>) {
        self.workspaces = entries;
    }

    /// Render current state as a string (for testing without Zellij runtime).
    pub fn render_text(&self) -> String {
        if self.workspaces.is_empty() {
            return "No workspaces".to_string();
        }
        self.workspaces
            .iter()
            .map(|w| {
                format!(
                    "{} [{}]",
                    w.name,
                    w.session_name.as_deref().unwrap_or("detached")
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_plugin() {
        let plugin = ResurrectionPlugin::new();
        assert!(plugin.workspaces.is_empty());
    }

    #[test]
    fn update_workspaces_replaces_entries() {
        let mut plugin = ResurrectionPlugin::new();
        let entries = vec![
            WorkspaceEntry {
                id: "1".to_string(),
                name: "alpha".to_string(),
                session_name: Some("session-a".to_string()),
            },
            WorkspaceEntry {
                id: "2".to_string(),
                name: "beta".to_string(),
                session_name: None,
            },
        ];
        plugin.update_workspaces(entries);
        assert_eq!(plugin.workspaces.len(), 2);
        assert_eq!(plugin.workspaces[0].name, "alpha");
        assert_eq!(plugin.workspaces[1].name, "beta");
    }

    #[test]
    fn render_text_with_no_workspaces() {
        let plugin = ResurrectionPlugin::new();
        assert_eq!(plugin.render_text(), "No workspaces");
    }

    #[test]
    fn render_text_with_entries() {
        let mut plugin = ResurrectionPlugin::new();
        plugin.update_workspaces(vec![
            WorkspaceEntry {
                id: "a".to_string(),
                name: "work".to_string(),
                session_name: Some("zellij-main".to_string()),
            },
            WorkspaceEntry {
                id: "b".to_string(),
                name: "personal".to_string(),
                session_name: None,
            },
        ]);
        let text = plugin.render_text();
        assert_eq!(text, "work [zellij-main]\npersonal [detached]");
    }
}
