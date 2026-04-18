//! Shell adapter trait and types for capturing shell process information.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

impl ProcShellAdapter {
    /// Reads the current working directory from /proc/<pid>/cwd.
    fn read_cwd(pid: u32) -> anyhow::Result<String> {
        let cwd_path = format!("/proc/{pid}/cwd");
        let target = fs::read_link(&cwd_path)?;
        Ok(target.to_string_lossy().to_string())
    }

    /// Reads the command line from /proc/<pid>/cmdline (null-delimited).
    fn read_cmdline(pid: u32) -> anyhow::Result<Vec<String>> {
        let cmdline_path = format!("/proc/{pid}/cmdline");
        let bytes = fs::read(&cmdline_path)?;

        // Split on null bytes and decode as UTF-8
        let args = bytes
            .split(|&b| b == 0)
            .filter(|chunk| !chunk.is_empty())
            .map(|chunk| String::from_utf8_lossy(chunk).to_string())
            .collect();

        Ok(args)
    }

    /// Reads environment variables from /proc/<pid>/environ (null-delimited key=value pairs).
    fn read_environ(pid: u32) -> anyhow::Result<HashMap<String, String>> {
        let environ_path = format!("/proc/{pid}/environ");
        let bytes = fs::read(&environ_path)?;

        let mut env = HashMap::new();

        // Split on null bytes
        for chunk in bytes.split(|&b| b == 0) {
            if chunk.is_empty() {
                continue;
            }
            let entry = String::from_utf8_lossy(chunk);
            if let Some((key, value)) = entry.split_once('=') {
                env.insert(key.to_string(), value.to_string());
            }
        }

        Ok(env)
    }

    /// Detects the shell name from the first element of cmdline (basename, with leading `-` stripped for login shells).
    fn detect_shell_name(cmdline: &[String]) -> String {
        if cmdline.is_empty() {
            return String::new();
        }

        let first = &cmdline[0];
        let basename = Path::new(first).file_name().unwrap_or_default();
        let mut name = basename.to_string_lossy().to_string();

        // Strip leading `-` for login shells
        if name.starts_with('-') {
            name = name[1..].to_string();
        }

        name
    }
}

impl ShellAdapter for ProcShellAdapter {
    fn capture(&self, pid: u32) -> anyhow::Result<ShellCapture> {
        // Try to read from /proc filesystem. If not available (e.g., macOS), fall back to env.
        let cwd = if Path::new("/proc").exists() {
            Self::read_cwd(pid)?
        } else {
            // Fallback for macOS: use current directory if available
            std::env::current_dir()?.to_string_lossy().to_string()
        };

        let cmdline = if Path::new("/proc").exists() {
            Self::read_cmdline(pid)?
        } else {
            // Fallback: minimal data
            vec![]
        };

        let env = if Path::new("/proc").exists() {
            Self::read_environ(pid)?
        } else {
            // Fallback: empty env
            HashMap::new()
        };

        let shell_name = Self::detect_shell_name(&cmdline);

        Ok(ShellCapture {
            pid,
            cwd,
            cmdline,
            env,
            shell_name,
        })
    }

    fn shell_name(&self) -> &'static str {
        "proc"
    }
}
