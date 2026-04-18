//! Common multiplexer interface.
//!
//! Every multiplexer backend (Zellij, tmux, kitty) implements the [`Mux`] trait.
//! Resurreccion logic targets this trait exclusively — no crate other than
//! `resurreccion-zellij` (and future backends) names any specific multiplexer.

pub mod conformance;

use serde::{Deserialize, Serialize};
use thiserror::Error;

// Re-export bitflags so callers don't need to add it as a direct dep.
pub use bitflags::bitflags;

bitflags! {
    /// Capability flags advertised by a multiplexer backend.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
    pub struct Capability: u32 {
        /// Backend supports embedding plugin UIs inside panes.
        const PLUGIN_EMBEDDING = 0b0001;
        /// Backend supports copy mode (scrollback selection).
        const COPY_MODE        = 0b0010;
        /// Backend exposes raw scrollback as text.
        const SCROLLBACK_TEXT  = 0b0100;
    }
}

/// A pane within the multiplexer layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaneSpec {
    /// Stable pane identifier (backend-specific string).
    pub id: String,
    /// Working directory of the pane process, if known.
    pub cwd: Option<String>,
    /// Current command/title shown in the pane.
    pub title: Option<String>,
}

/// A description of the desired layout to apply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LayoutSpec {
    /// Panes to create, in order.
    pub panes: Vec<PaneSpec>,
    /// Tab names, if the backend supports tabs.
    pub tabs: Vec<String>,
}

/// A snapshot of the current multiplexer layout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutCapture {
    /// Session name in the multiplexer.
    pub session_name: String,
    /// Captured panes.
    pub panes: Vec<PaneSpec>,
    /// Tab names.
    pub tabs: Vec<String>,
    /// Backend capability flags at capture time.
    pub capabilities: Capability,
}

/// A topology change event emitted by [`Mux::subscribe_topology`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TopologyEvent {
    /// A pane was created.
    PaneOpened {
        /// The ID of the pane that opened.
        pane_id: String,
    },
    /// A pane was closed.
    PaneClosed {
        /// The ID of the pane that closed.
        pane_id: String,
    },
    /// Keyboard focus moved to a different pane.
    FocusChanged {
        /// The ID of the pane that now has focus.
        pane_id: String,
    },
    /// Tabs or splits changed.
    LayoutChanged,
}

/// Errors returned by multiplexer operations.
#[derive(Debug, Error)]
pub enum MuxError {
    /// The multiplexer binary was not found or not runnable.
    #[error("backend not available: {0}")]
    NotAvailable(String),
    /// A session with this name already exists.
    #[error("session already exists: {0}")]
    SessionExists(String),
    /// The session was not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),
    /// The backend returned output that could not be parsed.
    #[error("parse error: {0}")]
    ParseError(String),
    /// An I/O error communicating with the backend.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// A fatal error that cannot be retried.
    #[error("fatal: {0}")]
    Fatal(String),
}

impl MuxError {
    /// Returns `true` if the operation may be retried after a delay.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::Io(_) | Self::NotAvailable(_))
    }
}

/// The common multiplexer interface.
///
/// Implementors: `resurreccion_zellij::ZellijMux` (and future tmux/kitty backends).
/// All Resurreccion planner and daemon logic targets this trait.
pub trait Mux: Send + Sync {
    /// Discover all currently running sessions managed by this backend.
    ///
    /// # Errors
    /// Returns [`MuxError::NotAvailable`] if the backend is not running.
    fn discover(&self) -> Result<Vec<String>, MuxError>;

    /// Create a new session with the given name and layout.
    ///
    /// # Errors
    /// Returns [`MuxError::SessionExists`] if a session with this name already exists.
    fn create(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError>;

    /// Attach the current process to an existing session.
    ///
    /// # Errors
    /// Returns [`MuxError::SessionNotFound`] if the session does not exist.
    fn attach(&self, session_name: &str) -> Result<(), MuxError>;

    /// Capture the current layout of a session.
    ///
    /// # Errors
    /// Returns [`MuxError::SessionNotFound`] if the session does not exist.
    fn capture(&self, session_name: &str) -> Result<LayoutCapture, MuxError>;

    /// Apply a layout to an existing session, restructuring panes to match.
    ///
    /// # Errors
    /// Returns an error if the session is unavailable or the layout cannot be applied.
    fn apply_layout(&self, session_name: &str, layout: &LayoutSpec) -> Result<(), MuxError>;

    /// Send a key sequence to the focused pane in a session.
    ///
    /// # Errors
    /// Returns an error if the session or pane is unavailable.
    fn send_keys(&self, session_name: &str, keys: &str) -> Result<(), MuxError>;

    /// Subscribe to topology events from a session.
    ///
    /// Returns a channel receiver. The backend spawns a background task that
    /// emits [`TopologyEvent`]s as the layout changes.
    ///
    /// # Errors
    /// Returns an error if the backend cannot start topology monitoring.
    fn subscribe_topology(
        &self,
        session_name: &str,
    ) -> Result<std::sync::mpsc::Receiver<TopologyEvent>, MuxError>;

    /// Return the capability flags this backend supports.
    fn capabilities(&self) -> Capability;
}
