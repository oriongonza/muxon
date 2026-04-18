//! Neovim editor adapter for Resurreccion.
//!
//! Implements the [`Editor`] trait for Neovim via its RPC socket.
//! When a socket is not available the adapter degrades gracefully rather
//! than returning an error.

use resurreccion_editor::{Editor, EditorCapture};
use serde_json::json;

/// Neovim editor adapter.
///
/// Connects to a running Neovim instance via its Unix socket.  The socket
/// path is resolved from (in priority order):
///
/// 1. The path supplied at construction via [`NvimAdapter::with_socket`].
/// 2. The `NVIM` environment variable.
/// 3. The `NVIM_LISTEN_ADDRESS` environment variable (legacy name).
pub struct NvimAdapter {
    socket_path: Option<String>,
}

impl NvimAdapter {
    /// Create a new adapter, resolving the socket path from environment
    /// variables (`NVIM` then `NVIM_LISTEN_ADDRESS`).
    pub fn new() -> Self {
        let socket_path = std::env::var("NVIM")
            .ok()
            .or_else(|| std::env::var("NVIM_LISTEN_ADDRESS").ok());
        Self { socket_path }
    }

    /// Create an adapter that connects to a specific socket path.
    pub fn with_socket(path: impl Into<String>) -> Self {
        Self {
            socket_path: Some(path.into()),
        }
    }

    /// Return the resolved socket path, if any.
    pub fn socket_path(&self) -> Option<&str> {
        self.socket_path.as_deref()
    }

    /// Try to connect to the Neovim socket and run `:buffers` via the RPC
    /// protocol.  Returns `None` if the socket is unreachable or any IO
    /// error occurs, enabling graceful degradation in [`Editor::capture`].
    fn try_connect(&self) -> Option<()> {
        use std::os::unix::net::UnixStream;
        let path = self.socket_path.as_deref()?;
        UnixStream::connect(path).ok().map(|_| ())
    }
}

impl Editor for NvimAdapter {
    fn editor_name(&self) -> &'static str {
        "neovim"
    }

    fn capture(&self) -> anyhow::Result<EditorCapture> {
        // Try to connect to the socket.  If unavailable, return a minimal
        // capture with an error marker so callers can distinguish a real
        // capture from a degraded one without failing the whole pipeline.
        let unavailable = EditorCapture {
            editor_name: self.editor_name().to_owned(),
            cwd: String::new(),
            open_files: Vec::new(),
            raw_state: json!({ "error": "socket not available" }),
        };

        if self.try_connect().is_none() {
            return Ok(unavailable);
        }

        // Socket is reachable — a full implementation would send msgpack-RPC
        // calls (nvim_list_bufs, nvim_buf_get_name, nvim_call_function getcwd)
        // and decode the responses.  For 0.x we return a structural capture
        // indicating the socket is alive but detailed state was not fetched.
        Ok(EditorCapture {
            editor_name: self.editor_name().to_owned(),
            cwd: String::new(),
            open_files: Vec::new(),
            raw_state: json!({ "source": "nvim_rpc", "status": "connected" }),
        })
    }

    fn restore(&self, capture: &EditorCapture) -> anyhow::Result<()> {
        // For 0.x: log what would be done, then return Ok.
        // A full implementation would send nvim_command("cd <cwd>") and
        // nvim_command("edit <file>") for each entry in open_files.
        tracing::info!(
            editor = self.editor_name(),
            cwd = %capture.cwd,
            file_count = capture.open_files.len(),
            "restore: would reopen files (no-op in 0.x)"
        );
        for f in &capture.open_files {
            tracing::debug!(file = %f, "would open file");
        }
        Ok(())
    }
}

impl Default for NvimAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_editor::conformance;

    #[test]
    fn construction_default() {
        // NVIM / NVIM_LISTEN_ADDRESS are not set in test environment.
        let a = NvimAdapter::new();
        // May or may not have a path depending on test environment.
        let _ = a.socket_path();
    }

    #[test]
    fn construction_with_socket() {
        let a = NvimAdapter::with_socket("/tmp/nvim.sock");
        assert_eq!(a.socket_path(), Some("/tmp/nvim.sock"));
    }

    #[test]
    fn editor_name_is_neovim() {
        let a = NvimAdapter::new();
        assert_eq!(a.editor_name(), "neovim");
    }

    #[test]
    fn conformance_check() {
        let a = NvimAdapter::new();
        conformance::run(&a);
    }

    #[test]
    fn capture_without_socket_returns_ok() {
        // Construct with no socket path directly — avoids env var mutation.
        let a = NvimAdapter { socket_path: None };
        assert!(a.socket_path().is_none());
        let result = a.capture();
        assert!(result.is_ok(), "capture must not fail without a socket");
        let cap = result.unwrap();
        assert_eq!(cap.editor_name, "neovim");
        assert!(
            cap.raw_state.get("error").is_some(),
            "degraded capture should carry an error field"
        );
    }

    #[test]
    fn capture_with_bad_socket_path_degrades() {
        let a = NvimAdapter::with_socket("/nonexistent/nvim.sock");
        let result = a.capture();
        assert!(result.is_ok());
        let cap = result.unwrap();
        assert_eq!(cap.editor_name, "neovim");
        assert!(
            cap.raw_state.get("error").is_some(),
            "degraded capture should carry an error field"
        );
    }

    #[test]
    fn restore_returns_ok() {
        let a = NvimAdapter::new();
        let cap = EditorCapture {
            editor_name: "neovim".to_owned(),
            cwd: "/home/user/project".to_owned(),
            open_files: vec!["src/main.rs".to_owned(), "Cargo.toml".to_owned()],
            raw_state: serde_json::json!({}),
        };
        assert!(a.restore(&cap).is_ok());
    }

    #[test]
    fn restore_with_empty_capture_returns_ok() {
        let a = NvimAdapter::new();
        let cap = EditorCapture {
            editor_name: "neovim".to_owned(),
            cwd: String::new(),
            open_files: Vec::new(),
            raw_state: serde_json::json!({}),
        };
        assert!(a.restore(&cap).is_ok());
    }
}
