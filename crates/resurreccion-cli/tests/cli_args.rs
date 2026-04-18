#![allow(missing_docs)]

use clap::Parser;
use resurreccion_cli::{Cli, Commands};

#[test]
fn doctor_subcommand_parses() {
    let cli = Cli::try_parse_from(["muxon", "doctor"]).expect("doctor should parse");
    assert!(matches!(cli.command, Commands::Doctor));
}

#[test]
fn workspace_list_parses() {
    let cli = Cli::try_parse_from(["muxon", "workspace", "list"]).expect("workspace list should parse");
    assert!(matches!(cli.command, Commands::Workspace(_)));
}

#[test]
fn json_flag_parses() {
    let cli = Cli::try_parse_from(["muxon", "doctor", "--json"]).expect("doctor --json should parse");
    assert!(cli.json);
    assert!(matches!(cli.command, Commands::Doctor));
}

#[test]
fn tree_subcommand_is_stub() {
    let cli = Cli::try_parse_from(["muxon", "tree"]).expect("tree should parse");
    assert!(matches!(cli.command, Commands::Tree));
}

#[test]
fn events_tail_parses() {
    let cli = Cli::try_parse_from(["muxon", "events", "tail"]).expect("events tail should parse");
    assert!(matches!(cli.command, Commands::Events(_)));
}
