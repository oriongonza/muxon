#![allow(missing_docs)]

use resurreccion_proto::{default_socket_path, Request};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(std::env::args().skip(1))?;
    match args.command {
        Command::Doctor { json } => doctor(&args.socket_path, json),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Doctor { json: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Args {
    command: Command,
    socket_path: PathBuf,
}

impl Args {
    fn parse<I>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = String>,
    {
        let mut command = Command::Doctor { json: false };
        let mut socket_path = default_socket_path();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "doctor" => {}
                "--json" => command = Command::Doctor { json: true },
                "--socket" => {
                    let value = args
                        .next()
                        .ok_or_else(|| "missing value for --socket".to_string())?;
                    socket_path = PathBuf::from(value);
                }
                "--help" | "-h" => return Err(help_text()),
                other => return Err(format!("unknown argument: {other}\n\n{}", help_text())),
            }
        }

        Ok(Self {
            command,
            socket_path,
        })
    }
}

fn help_text() -> String {
    [
        "Usage:",
        "  resurreccion-cli doctor [--socket PATH] [--json]",
    ]
    .join("\n")
}

fn doctor(socket_path: &PathBuf, json: bool) -> Result<(), String> {
    let response = request_health(socket_path)?;
    if json {
        println!("{response}");
    } else {
        println!(
            "resurreccion-daemon is healthy at {}",
            socket_path.display()
        );
    }
    Ok(())
}

fn request_health(socket_path: &PathBuf) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("failed to connect to {}: {error}", socket_path.display()))?;
    stream
        .write_all(Request::Health.as_wire().as_bytes())
        .map_err(|error| format!("failed to send health request: {error}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read health response: {error}"))?;

    if response.contains("\"ok\":true") {
        Ok(response.trim().to_string())
    } else {
        Err(format!(
            "daemon returned unexpected response: {}",
            response.trim()
        ))
    }
}
