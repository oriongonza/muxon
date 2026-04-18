//! Core domain types for the Resurreccion system.
//!
//! Every crate in the workspace depends on this crate.
//! All shared identifiers, error codes, event types, and restoration
//! fidelity levels are defined here.

pub mod events;
pub mod ids;

pub use crate::error::ErrorCode;
pub use crate::fidelity::{PartialRestore, RestoreFidelity};
pub use ids::{
    BindingKey, BlobId, EventId, PaneId, RuntimeId, SessionId, SnapshotId, TabId, WorkspaceId,
};

mod error;
mod fidelity;
