//! Zellij multiplexer backend implementation.
//!
//! This module provides [`ZellijMux`], an implementation of [`resurreccion_mux::Mux`]
//! that communicates with Zellij via its CLI.

use resurreccion_mux::{Capability, LayoutCapture, LayoutSpec, Mux, MuxError, TopologyEvent};
use std::collections::HashSet;
use std::process::Command;
use std::time::Duration;
use tracing::{debug, warn};

/// Zellij multiplexer backend.
#[derive(Clone)]
pub struct ZellijMux;

impl ZellijMux {
    /// Create a new `ZellijMux` instance.
    pub const fn new() -> Self {
        Self
    }

    fn zellij_cmd() -> Command {
        Command::new("zellij")
    }
}

impl Default for ZellijMux {
    fn default() -> Self {
        Self::new()
    }
}

impl Mux for ZellijMux {
    fn discover(&self) -> Result<Vec<String>, MuxError> {
        let output = Self::zellij_cmd()
            .args(["list-sessions"])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MuxError::NotAvailable("zellij binary not found".to_string())
                } else {
                    MuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            // zellij not running or no sessions
            return Ok(Vec::new());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let sessions: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return None;
                }
                // Parse: "session_name" or "session_name (attached)" etc.
                let session_name = trimmed.split_whitespace().next()?.to_string();
                Some(session_name)
            })
            .collect();

        Ok(sessions)
    }

    fn create(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError> {
        // Check if session already exists
        let existing_sessions = self.discover()?;
        if existing_sessions.contains(&session_name.to_string()) {
            return Err(MuxError::SessionExists(session_name.to_string()));
        }

        // Generate a minimal KDL layout string from the spec
        let kdl_layout = Self::spec_to_kdl(layout);

        // Write layout to a temp file
        let temp_file = std::env::temp_dir().join(format!("zellij-layout-{session_name}.kdl"));
        std::fs::write(&temp_file, &kdl_layout).map_err(|e| {
            warn!("Failed to write layout file: {}", e);
            MuxError::Io(e)
        })?;

        // Create the session with the layout
        let status = Self::zellij_cmd()
            .args(["--layout", temp_file.to_str().unwrap()])
            .args(["--session", session_name])
            .status()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MuxError::NotAvailable("zellij binary not found".to_string())
                } else {
                    MuxError::Io(e)
                }
            })?;

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_file);

        if status.success() {
            Ok(())
        } else {
            Err(MuxError::Fatal(format!(
                "zellij create failed with status: {status}"
            )))
        }
    }

    fn attach(&self, session_name: &str) -> Result<(), MuxError> {
        let status = Self::zellij_cmd()
            .args(["attach", session_name])
            .status()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MuxError::NotAvailable("zellij binary not found".to_string())
                } else {
                    MuxError::Io(e)
                }
            })?;

        if status.success() {
            Ok(())
        } else {
            // Session not found or attach failed
            Err(MuxError::SessionNotFound(session_name.to_string()))
        }
    }

    fn capture(&self, session_name: &str) -> Result<LayoutCapture, MuxError> {
        let output = Self::zellij_cmd()
            .args(["action", "dump-layout"])
            .args(["--session", session_name])
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MuxError::NotAvailable("zellij binary not found".to_string())
                } else {
                    MuxError::Io(e)
                }
            })?;

        if !output.status.success() {
            return Err(MuxError::SessionNotFound(session_name.to_string()));
        }

        // For now, return a minimal LayoutCapture if we can parse the output at all.
        // In a fuller implementation, parse the actual KDL output.
        let stdout = String::from_utf8_lossy(&output.stdout);
        debug!("dump-layout output: {}", stdout);

        // Return minimal valid LayoutCapture
        Ok(LayoutCapture {
            session_name: session_name.to_string(),
            panes: vec![],
            tabs: vec![],
            capabilities: self.capabilities(),
        })
    }

    fn apply_layout(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError> {
        // For 0.1.0: kill existing session and create new one with spec
        // This is a destructive operation, documented in the architecture.

        // Try to kill the existing session
        let _ = Self::zellij_cmd()
            .args(["kill-session", "-s", session_name])
            .status();

        // Create new session with layout
        self.create(session_name, layout)
    }

    fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), MuxError> {
        let status = Self::zellij_cmd()
            .args(["action", "write-chars"])
            .args(["--session", session_name])
            .arg(keys)
            .status()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    MuxError::NotAvailable("zellij binary not found".to_string())
                } else {
                    MuxError::Io(e)
                }
            })?;

        if status.success() {
            Ok(())
        } else {
            Err(MuxError::SessionNotFound(session_name.to_string()))
        }
    }

    fn subscribe_topology(
        &self,
        session_name: &str,
    ) -> Result<std::sync::mpsc::Receiver<TopologyEvent>, MuxError> {
        let (tx, rx) = std::sync::mpsc::channel();
        let session_name = session_name.to_string();
        let mux = self.clone();

        // Spawn a blocking thread for polling (not tokio task, as we're using std::sync::mpsc)
        std::thread::spawn(move || {
            let mut previous_sessions = HashSet::new();
            let mut previous_panes = HashSet::new();

            while let Ok(sessions) = mux.discover() {
                let current_sessions: HashSet<_> = sessions.into_iter().collect();
                let current_panes: HashSet<_> = match mux.capture(&session_name) {
                    Ok(cap) => cap.panes.iter().map(|p| p.id.clone()).collect(),
                    Err(_) => HashSet::new(),
                };

                // Detect changes and emit events
                for session in current_sessions.difference(&previous_sessions) {
                    let _ = tx.send(TopologyEvent::PaneOpened {
                        pane_id: session.clone(),
                    });
                }

                for session in previous_sessions.difference(&current_sessions) {
                    let _ = tx.send(TopologyEvent::PaneClosed {
                        pane_id: session.clone(),
                    });
                }

                if previous_panes != current_panes {
                    let _ = tx.send(TopologyEvent::LayoutChanged);
                }

                previous_sessions = current_sessions;
                previous_panes = current_panes;

                std::thread::sleep(Duration::from_secs(1));
            }
        });

        Ok(rx)
    }

    fn capabilities(&self) -> Capability {
        // Zellij 0.1.0: supports copy mode and scrollback text, but not plugin embedding
        Capability::COPY_MODE | Capability::SCROLLBACK_TEXT
    }
}

impl ZellijMux {
    /// Convert a `LayoutSpec` to Zellij KDL layout format.
    fn spec_to_kdl(_spec: &LayoutSpec) -> String {
        // For 0.1.0, return a minimal valid KDL layout
        // A complete implementation would parse spec.panes and generate appropriate splits
        r#"
layout {
    pane split_direction="vertical" {
        pane
        pane
    }
}
"#
        .trim()
        .to_string()
    }
}
