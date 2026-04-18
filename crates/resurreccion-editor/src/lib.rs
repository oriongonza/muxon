//! Editor adapter trait and capture types for Resurreccion.

use serde::{Deserialize, Serialize};

/// Snapshot of an editor's state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorCapture {
    /// Name of the editor (e.g., "vim", "emacs", "neovim").
    pub editor_name: String,
    /// Current working directory at the time of capture.
    pub cwd: String,
    /// List of open file paths.
    pub open_files: Vec<String>,
    /// Editor-specific state, serialized as JSON.
    pub raw_state: serde_json::Value,
}

/// Adapter trait for capturing and restoring editor sessions.
pub trait Editor: Send + Sync + 'static {
    /// Returns the name of the editor.
    fn editor_name(&self) -> &str;

    /// Captures the current editor state.
    fn capture(&self) -> anyhow::Result<EditorCapture>;

    /// Restores an editor to a previously captured state.
    fn restore(&self, capture: &EditorCapture) -> anyhow::Result<()>;
}

/// Conformance tests for Editor implementations.
pub mod conformance {
    use super::Editor;

    /// Runs basic conformance checks on an Editor implementation.
    pub fn run<E: Editor>(editor: &E) {
        assert!(
            !editor.editor_name().is_empty(),
            "editor_name must not be empty"
        );
    }
}
