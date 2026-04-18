use resurreccion_shell::{ProcShellAdapter, ShellAdapter};
use std::process;

#[test]
fn capture_self_process() {
    let adapter = ProcShellAdapter;
    let pid = process::id();

    let capture = adapter
        .capture(pid)
        .expect("failed to capture self process");

    // Verify we captured something
    assert_eq!(capture.pid, pid);

    // cwd should match our current directory
    let expected_cwd = std::env::current_dir()
        .expect("failed to get current dir")
        .to_string_lossy()
        .to_string();
    assert_eq!(capture.cwd, expected_cwd);

    // cmdline should not be empty
    assert!(!capture.cmdline.is_empty());

    // shell_name should not be empty
    assert!(!capture.shell_name.is_empty());
}

#[test]
fn capture_detects_shell_name() {
    // Skip on macOS if /proc is not available
    if !std::path::Path::new("/proc").exists() {
        return;
    }

    let adapter = ProcShellAdapter;
    let pid = process::id();

    let capture = adapter.capture(pid).expect("failed to capture process");

    // shell_name should be non-empty
    assert!(!capture.shell_name.is_empty());
}

#[test]
fn capture_nonexistent_pid_returns_err() {
    let adapter = ProcShellAdapter;

    // Try to capture a non-existent PID (very high number unlikely to exist)
    let result = adapter.capture(9_999_999);

    // Should return an error
    assert!(result.is_err());
}

#[test]
fn proc_shell_adapter_name() {
    let adapter = ProcShellAdapter;
    assert_eq!(adapter.shell_name(), "proc");
}
