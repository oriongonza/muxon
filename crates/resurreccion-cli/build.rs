#![allow(missing_docs)]

use clap::{CommandFactory, Parser};
use clap_complete::{generate_to, Shell};
use std::env;

// Inline the Cli struct to avoid circular dependency
#[derive(Parser, Debug)]
#[command(name = "muxon", about = "Resurreccion workspace manager")]
struct Cli {
    #[arg(long, global = true)]
    dir: Option<String>,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    socket: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    Doctor,
    Save,
    Restore,
    Tree,
    Events {
        #[command(subcommand)]
        cmd: EventsCmd,
    },
    Workspace {
        #[command(subcommand)]
        cmd: WorkspaceCmd,
    },
}

#[derive(clap::Subcommand, Debug)]
enum EventsCmd {
    Tail,
}

#[derive(clap::Subcommand, Debug)]
enum WorkspaceCmd {
    Create,
    Get,
    List,
}

fn main() {
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR not set");

    let mut cmd = Cli::command();

    for shell in &[Shell::Bash, Shell::Zsh, Shell::Fish] {
        generate_to(*shell, &mut cmd, "muxon", &out_dir).expect("failed to generate completions");
    }
}
