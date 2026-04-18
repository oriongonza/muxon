//! SSH remote client for the Resurreccion daemon.
//!
//! Connects to a remote Resurreccion daemon over SSH and forwards verbs to it
//! using the `SshConnectRequest`/`SshForwardRequest` types from the proto crate.
//!
//! For 0.x, each `forward()` call would open a fresh `ssh` subprocess (dry-run
//! mode returns the command string without executing it).

use resurreccion_proto::{SshConnectRequest, SshConnectResponse, SshForwardRequest};
use ulid::Ulid;

/// An active (or simulated) SSH connection to a remote daemon.
#[derive(Debug)]
pub struct SshConnection {
    /// ULID identifying this SSH connection session.
    pub connection_id: String,
    /// Hostname or IP address of the remote host.
    pub host: String,
    /// SSH port on the remote host.
    pub port: u16,
    /// Remote username.
    pub user: String,
    /// Optional path to a private-key identity file.
    pub identity_file: Option<String>,
}

impl SshConnection {
    /// Connect to a remote host (validates params, generates connection ID).
    ///
    /// Does NOT actually open a persistent SSH connection for 0.x —
    /// each `forward()` call opens a fresh `ssh` subprocess.
    pub fn connect(req: &SshConnectRequest) -> anyhow::Result<(Self, SshConnectResponse)> {
        anyhow::ensure!(!req.host.is_empty(), "host must not be empty");
        anyhow::ensure!(req.port > 0, "port must be > 0");
        anyhow::ensure!(!req.user.is_empty(), "user must not be empty");

        let connection_id = Ulid::new().to_string();
        let conn = Self {
            connection_id: connection_id.clone(),
            host: req.host.clone(),
            port: req.port,
            user: req.user.clone(),
            identity_file: req.identity_file.clone(),
        };
        let response = SshConnectResponse {
            connection_id,
            remote_capabilities: vec![], // populated on real connect
        };
        Ok((conn, response))
    }

    /// Forward a verb to the remote daemon via `ssh ... resurreccion-daemon --headless`.
    ///
    /// For 0.x: constructs the ssh command but does NOT execute it (dry-run mode).
    /// Returns the command that would be run as a string.
    pub fn forward_dry_run(&self, req: &SshForwardRequest) -> String {
        let body = serde_json::to_string(&req.body).unwrap_or_default();
        self.identity_file.as_ref().map_or_else(
            || {
                format!(
                    "ssh -p {} {}@{} resurreccion-daemon --headless --verb {} --body '{}'",
                    self.port, self.user, self.host, req.verb, body
                )
            },
            |identity| {
                format!(
                    "ssh -p {} -i {} {}@{} resurreccion-daemon --headless --verb {} --body '{}'",
                    self.port, identity, self.user, self.host, req.verb, body
                )
            },
        )
    }

    /// Connection ID for this session.
    pub fn connection_id(&self) -> &str {
        &self.connection_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use resurreccion_proto::SshConnectRequest;

    fn valid_req() -> SshConnectRequest {
        SshConnectRequest {
            host: "example.com".to_string(),
            port: 22,
            user: "alice".to_string(),
            identity_file: None,
        }
    }

    #[test]
    fn connect_with_valid_params_succeeds() {
        let req = valid_req();
        let result = SshConnection::connect(&req);
        assert!(result.is_ok(), "expected Ok, got {result:?}");
        let (conn, resp) = result.unwrap();
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 22);
        assert_eq!(conn.user, "alice");
        assert!(!conn.connection_id.is_empty());
        assert_eq!(conn.connection_id, resp.connection_id);
    }

    #[test]
    fn connect_with_empty_host_fails() {
        let req = SshConnectRequest {
            host: String::new(),
            port: 22,
            user: "alice".to_string(),
            identity_file: None,
        };
        let result = SshConnection::connect(&req);
        assert!(result.is_err(), "expected Err for empty host");
        assert!(result.unwrap_err().to_string().contains("host"));
    }

    #[test]
    fn connect_with_zero_port_fails() {
        let req = SshConnectRequest {
            host: "example.com".to_string(),
            port: 0,
            user: "alice".to_string(),
            identity_file: None,
        };
        let result = SshConnection::connect(&req);
        assert!(result.is_err(), "expected Err for port=0");
        assert!(result.unwrap_err().to_string().contains("port"));
    }

    #[test]
    fn forward_dry_run_contains_host_and_verb() {
        let req = valid_req();
        let (conn, _) = SshConnection::connect(&req).unwrap();
        let forward_req = SshForwardRequest {
            connection_id: conn.connection_id().to_string(),
            verb: "doctor.ping".to_string(),
            body: serde_json::Value::Null,
        };
        let cmd = conn.forward_dry_run(&forward_req);
        assert!(
            cmd.contains("example.com"),
            "expected host in command: {cmd}"
        );
        assert!(
            cmd.contains("doctor.ping"),
            "expected verb in command: {cmd}"
        );
    }

    #[test]
    fn forward_dry_run_with_identity_file_includes_i_flag() {
        let req = SshConnectRequest {
            host: "remote.host".to_string(),
            port: 2222,
            user: "bob".to_string(),
            identity_file: Some("/home/bob/.ssh/id_ed25519".to_string()),
        };
        let (conn, _) = SshConnection::connect(&req).unwrap();
        let forward_req = SshForwardRequest {
            connection_id: conn.connection_id().to_string(),
            verb: "workspace.list".to_string(),
            body: serde_json::Value::Null,
        };
        let cmd = conn.forward_dry_run(&forward_req);
        assert!(cmd.contains("-i "), "expected -i flag in command: {cmd}");
        assert!(
            cmd.contains("/home/bob/.ssh/id_ed25519"),
            "expected identity path in command: {cmd}"
        );
    }
}
