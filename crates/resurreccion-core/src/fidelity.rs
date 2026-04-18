//! Restoration fidelity lattice.

use serde::{Deserialize, Serialize};

/// How faithfully a restore operation reproduced the original state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestoreFidelity {
    /// Only historical metadata is available; no live state was restored.
    Historical = 0,
    /// Structure (tabs, panes, splits) was restored but no process state.
    Structural = 1,
    /// Process state (cwd, env, some history) was restored.
    Stateful = 2,
    /// Byte-exact reproduction of the original state.
    Exact = 3,
}

/// The result of a restore operation that may have partially succeeded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialRestore {
    /// Overall fidelity of the restore.
    pub fidelity: RestoreFidelity,
    /// Per-item results. Empty means full success.
    pub failures: Vec<RestoreFailure>,
}

/// A single item that could not be fully restored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreFailure {
    /// Human-readable description of what failed.
    pub description: String,
    /// Fidelity level that was achieved for this item.
    pub achieved: RestoreFidelity,
}
