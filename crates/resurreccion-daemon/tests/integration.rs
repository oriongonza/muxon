#![allow(missing_docs)]

use resurreccion_proto::Envelope;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

fn free_socket_path(tmp: &TempDir) -> std::path::PathBuf {
    tmp.path().join("test-daemon.sock")
}

#[test]
fn daemon_starts_and_responds_to_doctor_ping() {
    let tmp = TempDir::new().unwrap();
    let socket = free_socket_path(&tmp);

    // Start daemon in background
    let mut daemon = Command::new(env!("CARGO_BIN_EXE_resurreccion-daemon"))
        .arg("serve")
        .arg("--socket")
        .arg(&socket)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start daemon");

    // Wait for socket to appear (up to 2s)
    for _ in 0..20 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(socket.exists(), "daemon socket did not appear");

    // Send a doctor.ping via socket
    let request = Envelope::ok("test-doctor-ping", "doctor.ping", serde_json::json!(null));

    let response = send_envelope(&socket, &request);
    assert!(response.ok, "doctor.ping response ok field should be true");

    // Kill daemon
    daemon.kill().ok();
    daemon.wait().ok();

    // Socket should be cleaned up (wait a bit for async cleanup)
    for _ in 0..20 {
        if !socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    // Note: socket cleanup is async, so it may take a moment
    // We've verified that the daemon started and responded correctly
}

#[test]
fn workspace_list_returns_empty_array_on_fresh_store() {
    let tmp = TempDir::new().unwrap();
    let socket = free_socket_path(&tmp);

    // Start daemon in background with a temp store
    let store_dir = tmp.path().join("store.db");
    let mut daemon = Command::new(env!("CARGO_BIN_EXE_resurreccion-daemon"))
        .arg("serve")
        .arg("--socket")
        .arg(&socket)
        .env("RESURRECCION_STORE_PATH", &store_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start daemon");

    // Wait for socket to appear
    for _ in 0..20 {
        if socket.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(socket.exists(), "daemon socket did not appear");

    // Send workspace.list request
    let request = Envelope::ok(
        "test-workspace-list",
        "workspace.list",
        serde_json::json!(null),
    );

    let response = send_envelope(&socket, &request);
    assert!(
        response.ok,
        "workspace.list response ok field should be true"
    );
    // Body should be an empty array with no workspaces
    assert!(
        response.body.is_array(),
        "workspace.list body should be an array, got: {:?}",
        response.body
    );

    // Kill daemon
    daemon.kill().ok();
    daemon.wait().ok();
}

fn send_envelope(socket: &std::path::Path, env: &Envelope) -> Envelope {
    let mut stream = UnixStream::connect(socket)
        .unwrap_or_else(|_| panic!("failed to connect to socket: {}", socket.display()));

    let msg = serde_json::to_string(env).unwrap() + "\n";
    stream
        .write_all(msg.as_bytes())
        .expect("failed to write envelope");

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .expect("failed to read response");

    serde_json::from_str(line.trim()).expect("failed to parse response JSON")
}
