#![allow(missing_docs)]

use clap::Parser;
use resurreccion_cli::{Cli, Commands, EventsCmd, WorkspaceCmd};
use resurreccion_proto::{default_socket_path, Envelope, Request};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;
use std::time::Duration;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> Result<(), String> {
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);

    match cli.command {
        Commands::Doctor => doctor(&socket_path, cli.json),
        Commands::Save => {
            println!("save: not yet implemented");
            Ok(())
        }
        Commands::Restore => {
            println!("restore: not yet implemented");
            Ok(())
        }
        Commands::Tree => {
            println!("tree: not yet implemented");
            Ok(())
        }
        Commands::Events(wrapper) => match wrapper.cmd {
            EventsCmd::Tail => {
                println!("events tail: not yet implemented");
                Ok(())
            }
        },
        Commands::Workspace(wrapper) => match wrapper.cmd {
            WorkspaceCmd::Create => workspace_create(&socket_path, cli.json),
            WorkspaceCmd::Get => workspace_get(&socket_path, cli.json),
            WorkspaceCmd::List => workspace_list(&socket_path, cli.json),
        },
    }
}

fn doctor(socket_path: &std::path::PathBuf, json: bool) -> Result<(), String> {
    let request = Request::doctor_ping();
    let envelope = send_request(socket_path, &request)?;

    if json {
        let json_str = serde_json::to_string(&envelope)
            .map_err(|e| format!("failed to serialize response: {e}"))?;
        println!("{json_str}");
    } else {
        println!("doctor: ok");
    }

    Ok(())
}

fn workspace_list(socket_path: &std::path::PathBuf, json: bool) -> Result<(), String> {
    let request = Request::workspace_list();
    let envelope = send_request(socket_path, &request)?;

    if json {
        let json_str = serde_json::to_string(&envelope)
            .map_err(|e| format!("failed to serialize response: {e}"))?;
        println!("{json_str}");
    } else {
        println!("workspaces: {}", envelope.body);
    }

    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn workspace_create(_socket_path: &std::path::PathBuf, _json: bool) -> Result<(), String> {
    println!("workspace create: not yet implemented");
    Ok(())
}

#[allow(clippy::unnecessary_wraps)]
fn workspace_get(_socket_path: &std::path::PathBuf, _json: bool) -> Result<(), String> {
    println!("workspace get: not yet implemented");
    Ok(())
}

fn send_request(socket_path: &std::path::PathBuf, request: &Request) -> Result<Envelope, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|error| {
        format!(
            "failed to connect to {}: {} (exit 3)",
            socket_path.display(),
            error
        )
    })?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("failed to set read timeout: {e}"))?;

    let request_json =
        serde_json::to_string(request).map_err(|e| format!("failed to serialize request: {e}"))?;

    stream
        .write_all(request_json.as_bytes())
        .map_err(|e| format!("failed to write to socket: {e}"))?;
    stream
        .write_all(b"\n")
        .map_err(|e| format!("failed to write newline: {e}"))?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader
        .read_line(&mut response_line)
        .map_err(|e| format!("failed to read from socket: {e}"))?;

    let envelope: Envelope = serde_json::from_str(&response_line)
        .map_err(|e| format!("failed to parse response: {e}"))?;

    Ok(envelope)
}
