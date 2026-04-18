//! Multi-daemon federation layer for Resurreccion.
//!
//! Discovers and routes verbs across multiple Resurreccion daemon instances,
//! both local and remote (via SSH). Depends on `resurreccion-ssh` for the
//! underlying SSH transport.

use resurreccion_proto::{SshConnectRequest, SshForwardRequest};
use resurreccion_ssh::SshConnection;

/// A peer in the federation (local or remote daemon).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FederationPeer {
    /// Unique identifier for this peer (ULID).
    pub peer_id: String,
    /// Whether this peer is local or remote.
    pub kind: PeerKind,
    /// Human-readable label for this peer.
    pub label: String,
}

/// Discriminates local versus remote SSH peers.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PeerKind {
    /// A locally running daemon instance.
    Local,
    /// A remote daemon reachable over SSH.
    Remote {
        /// Hostname or IP address of the remote host.
        host: String,
        /// SSH port on the remote host.
        port: u16,
        /// Remote username.
        user: String,
    },
}

/// Federation registry — tracks known daemon peers.
pub struct Federation {
    peers: Vec<FederationPeer>,
}

impl Federation {
    /// Create a new, empty federation registry.
    pub const fn new() -> Self {
        Self { peers: vec![] }
    }

    /// Register a local peer.
    pub fn add_local(&mut self, label: impl Into<String>) -> &mut Self {
        self.peers.push(FederationPeer {
            peer_id: ulid::Ulid::new().to_string(),
            kind: PeerKind::Local,
            label: label.into(),
        });
        self
    }

    /// Register a remote SSH peer.
    pub fn add_remote(
        &mut self,
        label: impl Into<String>,
        host: impl Into<String>,
        port: u16,
        user: impl Into<String>,
    ) -> &mut Self {
        self.peers.push(FederationPeer {
            peer_id: ulid::Ulid::new().to_string(),
            kind: PeerKind::Remote {
                host: host.into(),
                port,
                user: user.into(),
            },
            label: label.into(),
        });
        self
    }

    /// Return all registered peers.
    pub fn peers(&self) -> &[FederationPeer] {
        &self.peers
    }

    /// Return the number of registered peers.
    pub const fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Generate the SSH forward command string for a remote peer verb (dry-run).
    ///
    /// Returns `None` if `peer_id` is not found or the peer is `Local`.
    pub fn forward_dry_run(
        &self,
        peer_id: &str,
        verb: &str,
        body: serde_json::Value,
    ) -> Option<String> {
        let peer = self.peers.iter().find(|p| p.peer_id == peer_id)?;
        match &peer.kind {
            PeerKind::Local => None,
            PeerKind::Remote { host, port, user } => {
                let req = SshConnectRequest {
                    host: host.clone(),
                    port: *port,
                    user: user.clone(),
                    identity_file: None,
                };
                let (conn, _) = SshConnection::connect(&req).ok()?;
                let fwd = SshForwardRequest {
                    connection_id: conn.connection_id().to_string(),
                    verb: verb.to_string(),
                    body,
                };
                Some(conn.forward_dry_run(&fwd))
            }
        }
    }
}

impl Default for Federation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_federation_is_empty() {
        let fed = Federation::new();
        assert_eq!(fed.peer_count(), 0);
        assert!(fed.peers().is_empty());
    }

    #[test]
    fn add_local_peer_increments_count() {
        let mut fed = Federation::new();
        fed.add_local("local-dev");
        assert_eq!(fed.peer_count(), 1);
    }

    #[test]
    fn add_remote_peer_increments_count() {
        let mut fed = Federation::new();
        fed.add_remote("remote-prod", "prod.example.com", 22, "deploy");
        assert_eq!(fed.peer_count(), 1);
    }

    #[test]
    fn peers_returns_all() {
        let mut fed = Federation::new();
        fed.add_local("local");
        fed.add_remote("remote-a", "a.example.com", 22, "alice");
        fed.add_remote("remote-b", "b.example.com", 2222, "bob");
        let peers = fed.peers();
        assert_eq!(peers.len(), 3);
        assert_eq!(peers[0].label, "local");
        assert_eq!(peers[1].label, "remote-a");
        assert_eq!(peers[2].label, "remote-b");
    }

    #[test]
    fn forward_dry_run_local_peer_returns_none() {
        let mut fed = Federation::new();
        fed.add_local("local-dev");
        let peer_id = fed.peers()[0].peer_id.clone();
        let result = fed.forward_dry_run(&peer_id, "doctor.ping", serde_json::Value::Null);
        assert!(result.is_none(), "local peer should return None");
    }

    #[test]
    fn forward_dry_run_remote_peer_returns_some_with_host() {
        let mut fed = Federation::new();
        fed.add_remote("remote-prod", "prod.example.com", 22, "deploy");
        let peer_id = fed.peers()[0].peer_id.clone();
        let result = fed.forward_dry_run(&peer_id, "doctor.ping", serde_json::Value::Null);
        assert!(result.is_some(), "remote peer should return Some");
        let cmd = result.unwrap();
        assert!(
            cmd.contains("prod.example.com"),
            "command should contain the host: {cmd}"
        );
        assert!(
            cmd.contains("doctor.ping"),
            "command should contain the verb: {cmd}"
        );
    }

    #[test]
    fn forward_dry_run_unknown_peer_returns_none() {
        let fed = Federation::new();
        let result = fed.forward_dry_run(
            "nonexistent-peer-id",
            "doctor.ping",
            serde_json::Value::Null,
        );
        assert!(result.is_none(), "unknown peer_id should return None");
    }
}
