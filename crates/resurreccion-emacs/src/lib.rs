//! Emacs editor adapter for Resurreccion.
//!
//! Implements the [`Editor`] trait for Emacs via `emacsclient`.

use resurreccion_editor::{Editor, EditorCapture};

/// Timeout in seconds when waiting for `emacsclient` to respond.
const EMACSCLIENT_TIMEOUT_SECS: u64 = 3;

/// Emacs editor adapter that communicates with a running Emacs instance via `emacsclient`.
pub struct EmacsAdapter {
    socket_name: Option<String>,
}

impl EmacsAdapter {
    /// Creates a new `EmacsAdapter`, reading the socket name from the
    /// `EMACS_SOCKET_NAME` environment variable if set.
    #[must_use]
    pub fn new() -> Self {
        Self {
            socket_name: std::env::var("EMACS_SOCKET_NAME").ok(),
        }
    }

    /// Creates a new `EmacsAdapter` with an explicit socket name.
    pub fn with_socket(name: impl Into<String>) -> Self {
        Self {
            socket_name: Some(name.into()),
        }
    }

    /// Builds the base `emacsclient` command with optional socket argument.
    fn build_command(&self, extra_args: &[&str]) -> std::process::Command {
        let mut cmd = std::process::Command::new("emacsclient");
        if let Some(ref socket) = self.socket_name {
            cmd.arg("--socket-name").arg(socket);
        }
        for arg in extra_args {
            cmd.arg(arg);
        }
        cmd
    }

    /// Runs `emacsclient` with the given arguments, killing it after the timeout.
    ///
    /// Returns `None` if the command is not found, times out, or exits with failure.
    fn run_with_timeout(&self, args: &[&str]) -> Option<String> {
        let mut child = self
            .build_command(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .ok()?;

        let timeout = std::time::Duration::from_secs(EMACSCLIENT_TIMEOUT_SECS);
        let deadline = std::time::Instant::now() + timeout;

        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if status.success() {
                        let output = child
                            .wait_with_output()
                            .ok()
                            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
                        return output;
                    }
                    return None;
                }
                Ok(None) => {
                    if std::time::Instant::now() >= deadline {
                        let _ = child.kill();
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
                Err(_) => return None,
            }
        }
    }

    /// Parses an Emacs elisp list of strings into a `Vec<String>`.
    ///
    /// The input looks like: `("/path/to/file.rs" "/other/file.txt" nil)`
    /// `nil` entries (non-file buffers) are skipped.
    fn parse_elisp_list(input: &str) -> Vec<String> {
        let trimmed = input.trim();
        // Strip surrounding parens if present
        let inner = if trimmed.starts_with('(') && trimmed.ends_with(')') {
            &trimmed[1..trimmed.len() - 1]
        } else {
            trimmed
        };

        let mut result = Vec::new();
        let mut chars = inner.chars().peekable();

        while let Some(&c) = chars.peek() {
            if c.is_whitespace() {
                chars.next();
                continue;
            }
            if c == '"' {
                // Quoted string — collect until closing quote
                chars.next(); // consume opening "
                let mut s = String::new();
                let mut escaped = false;
                for ch in chars.by_ref() {
                    if escaped {
                        s.push(ch);
                        escaped = false;
                    } else if ch == '\\' {
                        escaped = true;
                    } else if ch == '"' {
                        break;
                    } else {
                        s.push(ch);
                    }
                }
                if !s.is_empty() {
                    result.push(s);
                }
            } else {
                // Bare token (e.g., `nil`) — skip
                while chars.peek().is_some_and(|&c| !c.is_whitespace()) {
                    chars.next();
                }
            }
        }

        result
    }
}

impl Default for EmacsAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl Editor for EmacsAdapter {
    fn editor_name(&self) -> &'static str {
        "emacs"
    }

    fn capture(&self) -> anyhow::Result<EditorCapture> {
        let cwd = std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let result = self.run_with_timeout(&["--eval", "(mapcar 'buffer-file-name (buffer-list))"]);

        match result {
            Some(raw) => {
                let open_files = Self::parse_elisp_list(raw.trim());
                Ok(EditorCapture {
                    editor_name: "emacs".to_string(),
                    cwd,
                    open_files,
                    raw_state: serde_json::json!({}),
                })
            }
            None => {
                // emacsclient not available or failed — return minimal capture
                Ok(EditorCapture {
                    editor_name: "emacs".to_string(),
                    cwd,
                    open_files: vec![],
                    raw_state: serde_json::json!({ "error": "emacsclient not available" }),
                })
            }
        }
    }

    fn restore(&self, capture: &EditorCapture) -> anyhow::Result<()> {
        for file in &capture.open_files {
            // --no-wait prevents blocking; silently ignore failures
            let _ = self.run_with_timeout(&["--no-wait", file.as_str()]);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_adapter_without_socket_when_env_unset() {
        std::env::remove_var("EMACS_SOCKET_NAME");
        let adapter = EmacsAdapter::new();
        assert!(adapter.socket_name.is_none());
    }

    #[test]
    fn new_reads_socket_from_env() {
        std::env::set_var("EMACS_SOCKET_NAME", "test-socket");
        let adapter = EmacsAdapter::new();
        assert_eq!(adapter.socket_name.as_deref(), Some("test-socket"));
        std::env::remove_var("EMACS_SOCKET_NAME");
    }

    #[test]
    fn with_socket_sets_name() {
        let adapter = EmacsAdapter::with_socket("my-socket");
        assert_eq!(adapter.socket_name.as_deref(), Some("my-socket"));
    }

    #[test]
    fn editor_name_returns_emacs() {
        let adapter = EmacsAdapter::new();
        assert_eq!(adapter.editor_name(), "emacs");
    }

    #[test]
    fn default_equals_new() {
        std::env::remove_var("EMACS_SOCKET_NAME");
        let a = EmacsAdapter::new();
        let b = EmacsAdapter::default();
        assert_eq!(a.socket_name, b.socket_name);
    }

    #[test]
    fn capture_without_emacsclient_returns_ok() {
        // emacsclient is not expected to be running; graceful degradation
        let adapter = EmacsAdapter::new();
        let result = adapter.capture();
        assert!(result.is_ok(), "capture must not return Err: {result:?}");
        let cap = result.unwrap();
        assert_eq!(cap.editor_name, "emacs");
    }

    #[test]
    fn restore_returns_ok() {
        let adapter = EmacsAdapter::new();
        let capture = EditorCapture {
            editor_name: "emacs".to_string(),
            cwd: "/tmp".to_string(),
            open_files: vec![],
            raw_state: serde_json::json!({}),
        };
        let result = adapter.restore(&capture);
        assert!(result.is_ok(), "restore must not return Err: {result:?}");
    }

    #[test]
    fn parse_elisp_list_extracts_paths() {
        let input = r#"("/home/user/foo.rs" "/home/user/bar.txt" nil)"#;
        let files = EmacsAdapter::parse_elisp_list(input);
        assert_eq!(files, vec!["/home/user/foo.rs", "/home/user/bar.txt"]);
    }

    #[test]
    fn parse_elisp_list_handles_empty() {
        let files = EmacsAdapter::parse_elisp_list("(nil)");
        assert!(files.is_empty());
    }

    #[test]
    fn emacs_adapter_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<EmacsAdapter>();
    }
}
