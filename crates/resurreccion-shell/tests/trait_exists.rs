#![allow(missing_docs)]

use resurreccion_shell::{ProcShellAdapter, ShellCapture};

#[test]
fn shell_capture_serializes() {
    use std::collections::HashMap;

    let capture = ShellCapture {
        pid: 1234,
        cwd: "/home/user".to_string(),
        cmdline: vec!["bash".to_string(), "-i".to_string()],
        env: HashMap::from([("PATH".to_string(), "/usr/bin".to_string())]),
        shell_name: "bash".to_string(),
    };

    let json = serde_json::to_string(&capture).expect("serialization failed");
    assert!(!json.is_empty());
}

#[test]
fn proc_shell_adapter_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProcShellAdapter>();
}
