//! Verb-name constants — the single source of truth for all daemon verbs.
//!
//! Daemon dispatch, CLI commands, and protocol docs all reference these constants.
//! Adding a verb: add a constant here, add a handler in the daemon, add a subcommand in the CLI.

/// Health check. No arguments. Returns `{ "proto": <version> }`.
pub const DOCTOR_PING: &str = "doctor.ping";

/// Open or create a workspace bound to a directory.
pub const WORKSPACE_OPEN: &str = "workspace.open";

/// Create a workspace without opening it.
pub const WORKSPACE_CREATE: &str = "workspace.create";

/// Retrieve a single workspace by ID.
pub const WORKSPACE_GET: &str = "workspace.get";

/// List all known workspaces.
pub const WORKSPACE_LIST: &str = "workspace.list";

/// Resolve a directory to its workspace, creating one if absent.
pub const WORKSPACE_RESOLVE_OR_CREATE: &str = "workspace.resolve_or_create";

/// Create a snapshot of the current workspace state.
pub const SNAPSHOT_CREATE: &str = "snapshot.create";

/// Restore a workspace from a snapshot.
pub const SNAPSHOT_RESTORE: &str = "snapshot.restore";

/// Subscribe to the event stream (long-lived streaming response).
pub const EVENTS_TAIL: &str = "events.tail";

/// Protocol handshake — sent by daemon immediately on connect.
pub const HANDSHAKE: &str = "handshake";
