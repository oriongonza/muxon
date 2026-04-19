//! ULID-based identifier newtypes and the BLAKE3 binding key.

use serde::{Deserialize, Serialize};
use std::fmt;
use ulid::Ulid;

macro_rules! ulid_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(Ulid);

        impl $name {
            /// Generate a new random ID.
            pub fn new() -> Self {
                Self(Ulid::new())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = ulid::DecodeError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse::<Ulid>().map(Self)
            }
        }
    };
}

ulid_id!(WorkspaceId, "Unique identifier for a workspace.");
ulid_id!(
    RuntimeId,
    "Unique identifier for a runtime (multiplexer session)."
);
ulid_id!(SnapshotId, "Unique identifier for a snapshot.");
ulid_id!(PaneId, "Unique identifier for a pane.");
ulid_id!(SessionId, "Unique identifier for a multiplexer session.");
ulid_id!(TabId, "Unique identifier for a tab.");
ulid_id!(EventId, "Unique identifier for a durable event.");
ulid_id!(BlobId, "Unique identifier for a content-addressed blob.");

/// A stable key identifying a workspace by its directory and optional git identity.
///
/// Computed as `BLAKE3(canonical_path + optional_git_remote + optional_worktree)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BindingKey([u8; 32]);

impl BindingKey {
    /// Construct from raw bytes (e.g. a BLAKE3 hash output).
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Compute from a canonical path string and optional git metadata.
    pub fn compute(canonical_path: &str, git_remote: Option<&str>, worktree: Option<&str>) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(canonical_path.as_bytes());
        if let Some(remote) = git_remote {
            hasher.update(b"\x00");
            hasher.update(remote.as_bytes());
        }
        if let Some(wt) = worktree {
            hasher.update(b"\x00");
            hasher.update(wt.as_bytes());
        }
        Self(*hasher.finalize().as_bytes())
    }

    /// Raw bytes of the key.
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for BindingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in &self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
