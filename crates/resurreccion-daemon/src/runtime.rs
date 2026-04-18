//! Async daemon runtime with graceful shutdown and connection handling.

use crate::dispatch::Dispatcher;
use anyhow::{anyhow, Result};
use resurreccion_proto::Envelope;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{error, info};

/// Check if a daemon is already running at the given socket path.
/// Attempts to connect to the socket; if successful, returns an error.
pub fn single_instance_guard(socket_path: &Path) -> Result<()> {
    // This is a blocking call, which is fine since it's only used at startup
    match std::os::unix::net::UnixStream::connect(socket_path) {
        Ok(_) => Err(anyhow!(
            "daemon already running at {}",
            socket_path.display()
        )),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Socket doesn't exist, which is good
            Ok(())
        }
        Err(e) => Err(anyhow!("failed to check socket: {e}")),
    }
}

/// Remove a stale socket file if no daemon is running.
fn remove_stale_socket(socket_path: &Path) -> Result<()> {
    if !socket_path.exists() {
        return Ok(());
    }

    // Check if a daemon is running
    single_instance_guard(socket_path)?;

    // Socket exists but no daemon is running, remove it
    std::fs::remove_file(socket_path)?;
    Ok(())
}

/// Start the daemon runtime.
///
/// This function:
/// - Checks for an existing daemon (single-instance guard)
/// - Removes any stale socket
/// - Binds a Unix socket
/// - Accepts connections and spawns a tokio task per connection
/// - Handles graceful shutdown on SIGTERM/SIGINT
pub async fn run(socket_path: PathBuf, dispatcher: Arc<Dispatcher>) -> Result<()> {
    // Single-instance guard
    single_instance_guard(&socket_path)?;

    // Remove stale socket
    remove_stale_socket(&socket_path)?;

    // Bind socket (tokio's UnixListener)
    let listener = tokio::net::UnixListener::bind(&socket_path)?;
    info!("daemon listening on {}", socket_path.display());

    // Set up graceful shutdown signal handling
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);

    // Spawn signal handler
    tokio::spawn(async move {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to setup SIGTERM handler");
        let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
            .expect("failed to setup SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("received SIGINT, initiating graceful shutdown");
            }
        }

        shutdown_clone.store(true, Ordering::Relaxed);
    });

    // Accept connections
    let active_tasks = Arc::new(Mutex::new(0usize));

    loop {
        // Check for shutdown signal
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        // Accept connection with timeout
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                error!("failed to accept connection: {}", e);
                continue;
            }
        };

        let dispatcher_clone = Arc::clone(&dispatcher);
        let active_tasks_clone = Arc::clone(&active_tasks);

        // Spawn task to handle connection
        tokio::spawn(async move {
            let _ = handle_connection(stream, dispatcher_clone).await;
            let mut count = active_tasks_clone.lock().await;
            *count = count.saturating_sub(1);
        });

        // Increment active task count
        let mut count = active_tasks.lock().await;
        *count += 1;
    }

    // Graceful shutdown: wait up to 2 seconds for connections to drain
    let drain_start = std::time::Instant::now();
    let drain_timeout = std::time::Duration::from_secs(2);

    loop {
        let count = active_tasks.lock().await;
        if *count == 0 {
            break;
        }

        if drain_start.elapsed() > drain_timeout {
            error!("drain timeout exceeded, {} tasks still active", count);
            break;
        }

        drop(count);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    // Remove socket file
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    info!("daemon shutdown complete");
    Ok(())
}

/// Handle a single connection by reading envelopes, dispatching them, and writing responses.
async fn handle_connection(
    stream: tokio::net::UnixStream,
    dispatcher: Arc<Dispatcher>,
) -> Result<()> {
    let (reader, writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut writer = writer;

    let mut line = String::new();
    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 {
            // Connection closed
            break;
        }

        // Parse the line as JSON envelope
        let envelope: Envelope = match serde_json::from_str(&line) {
            Ok(env) => env,
            Err(e) => {
                error!("failed to parse envelope: {e}");
                let error_env = Envelope::err(
                    "parse-error",
                    "unknown",
                    "parse_error",
                    format!("failed to parse JSON: {e}"),
                );
                let response = format!("{}\n", serde_json::to_string(&error_env)?);
                writer.write_all(response.as_bytes()).await?;
                continue;
            }
        };

        // Dispatch the envelope
        let response = dispatcher.dispatch(&envelope);

        // Write response
        let response_json = format!("{}\n", serde_json::to_string(&response)?);
        writer.write_all(response_json.as_bytes()).await?;
    }

    Ok(())
}
