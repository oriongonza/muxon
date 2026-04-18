use resurreccion_proto::{default_socket_path, Request, Response};
use rt_events::EventBus;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
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
        Command::Serve => serve(&args.socket_path),
        Command::Healthcheck => {
            let response = request_health(&args.socket_path)?;
            println!("{}", response.to_json());
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Serve,
    Healthcheck,
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
        let mut command = Command::Serve;
        let mut socket_path = default_socket_path();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "serve" => command = Command::Serve,
                "healthcheck" => command = Command::Healthcheck,
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

        Ok(Self { command, socket_path })
    }
}

fn help_text() -> String {
    [
        "Usage:",
        "  resurreccion-daemon [serve] [--socket PATH]",
        "  resurreccion-daemon healthcheck [--socket PATH]",
    ]
    .join("\n")
}

fn serve(socket_path: &Path) -> Result<(), String> {
    let bus = daemon_event_bus();
    remove_stale_socket(socket_path)?;
    let listener = UnixListener::bind(socket_path)
        .map_err(|error| format!("failed to bind {}: {error}", socket_path.display()))?;

    bus.emit(SocketBound {
        socket_path: socket_path.display().to_string(),
    });

    for stream in listener.incoming() {
        let mut stream = stream.map_err(|error| {
            format!(
                "failed to accept connection on {}: {error}",
                socket_path.display()
            )
        })?;
        handle_connection(&mut stream, socket_path, &bus)?;
    }

    Ok(())
}

fn remove_stale_socket(socket_path: &Path) -> Result<(), String> {
    if !socket_path.exists() {
        return Ok(());
    }

    match UnixStream::connect(socket_path) {
        Ok(_) => Err(format!(
            "refusing to replace active socket at {}",
            socket_path.display()
        )),
        Err(_) => fs::remove_file(socket_path)
            .map_err(|error| format!("failed to remove stale socket {}: {error}", socket_path.display())),
    }
}

fn handle_connection(
    stream: &mut UnixStream,
    socket_path: &Path,
    bus: &EventBus,
) -> Result<(), String> {
    let mut request_line = String::new();
    let mut reader = BufReader::new(stream.try_clone().map_err(|error| error.to_string())?);
    reader
        .read_line(&mut request_line)
        .map_err(|error| format!("failed to read request: {error}"))?;

    let response = response_for_request(&request_line, socket_path, bus);

    stream
        .write_all(response.to_json().as_bytes())
        .and_then(|_| stream.write_all(b"\n"))
        .map_err(|error| format!("failed to write response: {error}"))
}

fn response_for_request(request_line: &str, socket_path: &Path, bus: &EventBus) -> Response {
    match Request::parse(request_line) {
        Ok(Request::Health) => {
            bus.emit(HealthRequestHandled {
                socket_path: socket_path.display().to_string(),
            });
            Response::health(socket_path)
        }
        Err(error) => {
            bus.emit(BadRequestObserved {
                socket_path: socket_path.display().to_string(),
                message: error.clone(),
            });
            Response::error("bad_request", error)
        }
    }
}

fn request_health(socket_path: &Path) -> Result<Response, String> {
    let mut stream = UnixStream::connect(socket_path)
        .map_err(|error| format!("failed to connect to {}: {error}", socket_path.display()))?;
    stream
        .write_all(Request::Health.as_wire().as_bytes())
        .map_err(|error| format!("failed to send health request: {error}"))?;

    let mut buffer = String::new();
    stream
        .read_to_string(&mut buffer)
        .map_err(|error| format!("failed to read health response: {error}"))?;

    if buffer.contains("\"ok\":true") {
        Ok(Response::health(socket_path))
    } else {
        Err(format!("daemon returned unexpected response: {}", buffer.trim()))
    }
}

fn daemon_event_bus() -> EventBus {
    let mut bus = EventBus::new();
    register_default_observers(&mut bus);
    bus
}

fn register_default_observers(bus: &mut EventBus) {
    bus.on(|event: &SocketBound| {
        eprintln!("resurreccion-daemon listening on {}", event.socket_path);
    });
    bus.on(|event: &HealthRequestHandled| {
        eprintln!("resurreccion-daemon handled health request on {}", event.socket_path);
    });
    bus.on(|event: &BadRequestObserved| {
        eprintln!(
            "resurreccion-daemon bad request on {}: {}",
            event.socket_path, event.message
        );
    });
}

#[derive(Debug, Clone)]
struct SocketBound {
    socket_path: String,
}

#[derive(Debug, Clone)]
struct HealthRequestHandled {
    socket_path: String,
}

#[derive(Debug, Clone)]
struct BadRequestObserved {
    socket_path: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::{response_for_request, BadRequestObserved, HealthRequestHandled};
    use resurreccion_proto::Response;
    use rt_events::EventBus;
    use std::cell::RefCell;
    use std::path::Path;
    use std::rc::Rc;

    #[test]
    fn emits_health_event_for_health_requests() {
        let events = Rc::new(RefCell::new(0usize));
        let mut bus = EventBus::new();
        let seen = Rc::clone(&events);
        bus.on(move |_: &HealthRequestHandled| {
            *seen.borrow_mut() += 1;
        });

        let response = response_for_request("health\n", Path::new("/tmp/test.sock"), &bus);

        assert_eq!(*events.borrow(), 1);
        assert!(matches!(response, Response::Health(_)));
    }

    #[test]
    fn emits_bad_request_event_for_unknown_requests() {
        let errors = Rc::new(RefCell::new(Vec::<String>::new()));
        let mut bus = EventBus::new();
        let seen = Rc::clone(&errors);
        bus.on(move |event: &BadRequestObserved| {
            seen.borrow_mut().push(event.message.clone());
        });

        let response = response_for_request("bogus\n", Path::new("/tmp/test.sock"), &bus);

        assert_eq!(errors.borrow().len(), 1);
        assert!(matches!(response, Response::Error(_)));
    }
}
