//! Capability verb constants consumed by plan nodes.
//!
//! These strings are the keys the planner uses to match intent to capability.
//! Lane F (planner implementation) and Lane B3 (snapshot verbs) reference these.

/// Capture the multiplexer layout (tabs, panes, splits, cwds).
pub const CAPTURE_LAYOUT: &str = "capture.layout";

/// Restore the multiplexer layout from a snapshot manifest.
pub const RESTORE_LAYOUT: &str = "restore.layout";

/// Capture shell state (cwd, last command, env snapshot).
pub const CAPTURE_SHELL: &str = "capture.shell";

/// Restore shell cwd and environment in a pane.
pub const RESTORE_SHELL: &str = "restore.shell";

/// Capture aigent session metadata (external ID, transcript reference).
pub const CAPTURE_AIGENT: &str = "capture.aigent";

/// Resume an aigent session from captured metadata.
pub const RESTORE_AIGENT: &str = "restore.aigent";

/// Capture editor state (open buffers, cursor positions).
pub const CAPTURE_EDITOR: &str = "capture.editor";

/// Restore editor state from a snapshot.
pub const RESTORE_EDITOR: &str = "restore.editor";
