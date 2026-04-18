#![allow(missing_docs, missing_crate_level_docs)]

//! CLI for resurreccion workspace manager.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// The muxon CLI application.
#[derive(Parser, Debug)]
#[command(name = "muxon", about = "Resurreccion workspace manager")]
pub struct Cli {
    /// Workspace directory
    #[arg(long, global = true)]
    pub dir: Option<PathBuf>,

    /// Output as JSON
    #[arg(long, global = true)]
    pub json: bool,

    /// Socket path for daemon communication
    #[arg(long, global = true)]
    pub socket: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available subcommands.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Health check
    Doctor,

    /// Save workspace state
    Save,

    /// Restore workspace state
    Restore,

    /// Show workspace tree (not yet implemented)
    Tree,

    /// Event streaming
    Events(EventsWrapper),

    /// Workspace management
    Workspace(WorkspaceWrapper),
}

/// Wrapper for events subcommands
#[derive(Parser, Debug)]
pub struct EventsWrapper {
    #[command(subcommand)]
    pub cmd: EventsCmd,
}

/// Wrapper for workspace subcommands
#[derive(Parser, Debug)]
pub struct WorkspaceWrapper {
    #[command(subcommand)]
    pub cmd: WorkspaceCmd,
}

/// Events subcommands.
#[derive(Subcommand, Debug)]
pub enum EventsCmd {
    /// Tail the event stream
    Tail,
}

/// Workspace subcommands.
#[derive(Subcommand, Debug)]
pub enum WorkspaceCmd {
    /// Create a new workspace
    Create,

    /// Get workspace info
    Get,

    /// List workspaces
    List,
}
