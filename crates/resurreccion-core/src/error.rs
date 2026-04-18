//! Shared error code taxonomy.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Canonical error codes returned by the daemon.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Error)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    /// The daemon is starting up and not yet ready.
    #[error("not_ready")]
    NotReady,
    /// The requested resource does not exist.
    #[error("not_found")]
    NotFound,
    /// A conflicting operation is in progress.
    #[error("conflict")]
    Conflict,
    /// An internal daemon error occurred.
    #[error("internal")]
    Internal,
    /// The requested operation is not supported by this host.
    #[error("unsupported")]
    Unsupported,
    /// The client and daemon speak incompatible protocol versions.
    #[error("version_mismatch")]
    VersionMismatch,
    /// The operation timed out.
    #[error("timeout")]
    Timeout,
    /// A resource is locked by another operation.
    #[error("busy")]
    Busy,
}
