//! Shell adapter trait and types for capturing shell process information.

use std::collections::HashMap;

/// Information about a shell process captured from the system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ShellCapture {
    /// Process ID of the shell.
    pub pid: u32,
    /// Current working directory of the shell.
    pub cwd: String,
    /// Command line arguments of the shell process.
    pub cmdline: Vec<String>,
    /// Environment variables of the shell process.
    pub env: HashMap<String, String>,
    /// Name of the shell (e.g., "bash", "zsh").
    pub shell_name: String,
}

/// Trait for adapters that capture shell process information from the system.
pub trait ShellAdapter: Send + Sync + 'static {
    /// Capture shell information for a given process ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the shell information cannot be captured.
    fn capture(&self, pid: u32) -> anyhow::Result<ShellCapture>;

    /// Get the name of the shell adapter.
    fn shell_name(&self) -> &str;
}

/// Shell adapter that reads process information from the Linux /proc filesystem or macOS sysctl.
#[derive(Debug, Clone)]
pub struct ProcShellAdapter;

impl ShellAdapter for ProcShellAdapter {
    fn capture(&self, _pid: u32) -> anyhow::Result<ShellCapture> {
        unimplemented!("P1-A")
    }

    fn shell_name(&self) -> &str {
        unimplemented!("P1-A")
    }
}
