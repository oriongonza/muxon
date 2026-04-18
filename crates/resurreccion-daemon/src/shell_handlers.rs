//! Shell capture and restore verb handlers for the Resurreccion daemon.
#![allow(
    clippy::ignored_unit_patterns,
    clippy::significant_drop_in_scrutinee,
    clippy::redundant_pattern_matching,
    clippy::missing_const_for_fn
)]

use crate::dispatch::Handler;
use resurreccion_proto::Envelope;
use resurreccion_shell::{ProcShellAdapter, ShellAdapter};
use resurreccion_store::types::EventRow;
use resurreccion_store::Store;
use std::sync::{Arc, Mutex};
use ulid::Ulid;

/// Handler for `shell.capture` — captures shell state for workspace.
pub struct ShellCaptureHandler {
    store: Arc<Mutex<Store>>,
}

impl ShellCaptureHandler {
    /// Create a new shell capture handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for ShellCaptureHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        // Parse workspace_id (required)
        let workspace_id = match env.body.get("workspace_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => {
                return Envelope::err(
                    &env.id,
                    &env.verb,
                    "missing_workspace_id",
                    "workspace_id required",
                )
            }
        };

        // Parse pids (optional, defaults to empty)
        let pids: Vec<u64> = env
            .body
            .get("pids")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_u64()).collect())
            .unwrap_or_default();

        // Capture each pid
        let adapter = ProcShellAdapter;
        let mut captures = vec![];

        for pid in pids {
            match adapter.capture(pid as u32) {
                Ok(capture) => captures.push(capture),
                Err(_) => {
                    // Skip PIDs that can't be captured (permission denied, process gone, etc.)
                }
            }
        }

        // Generate current timestamp
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64;

        // Store event
        let event = EventRow {
            id: Ulid::new().to_string(),
            workspace_id: Some(workspace_id),
            kind: "shell.capture".to_string(),
            payload_json: match serde_json::to_string(&captures) {
                Ok(json) => json,
                Err(e) => {
                    return Envelope::err(&env.id, &env.verb, "serialization_error", e.to_string())
                }
            },
            created_at: format_timestamp_ms(now_ms),
        };

        if let Err(e) = self.store.lock().unwrap().event_append(&event) {
            return Envelope::err(&env.id, &env.verb, "internal", e.to_string());
        }

        // Return success response
        Envelope::ok(
            &env.id,
            &env.verb,
            serde_json::json!({
                "captures": captures,
                "count": captures.len()
            }),
        )
    }
}

/// Handler for `shell.restore` — reads latest shell captures from events.
pub struct ShellRestoreHandler {
    store: Arc<Mutex<Store>>,
}

impl ShellRestoreHandler {
    /// Create a new shell restore handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for ShellRestoreHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        // Parse workspace_id (required)
        let workspace_id = match env.body.get("workspace_id").and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => {
                return Envelope::err(
                    &env.id,
                    &env.verb,
                    "missing_workspace_id",
                    "workspace_id required",
                )
            }
        };

        // Read all events from store
        let events = match self.store.lock().unwrap().event_tail_from(None) {
            Ok(events) => events,
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "internal", e.to_string());
            }
        };

        // Find the latest shell.capture event for this workspace
        let latest_capture = events.iter().rev().find(|event| {
            event.kind == "shell.capture"
                && event.workspace_id.as_ref().map(|id| id.as_str()) == Some(workspace_id.as_str())
        });

        let captures: Vec<_> = match latest_capture {
            Some(event) => {
                // Parse the payload JSON
                match serde_json::from_str::<Vec<resurreccion_shell::ShellCapture>>(
                    &event.payload_json,
                ) {
                    Ok(caps) => caps,
                    Err(e) => {
                        return Envelope::err(&env.id, &env.verb, "invalid_payload", e.to_string())
                    }
                }
            }
            None => vec![],
        };

        // Return success response with captures
        Envelope::ok(
            &env.id,
            &env.verb,
            serde_json::json!({
                "captures": captures,
                "workspace_id": workspace_id
            }),
        )
    }
}

/// Format a timestamp in milliseconds as ISO-8601 string.
fn format_timestamp_ms(ms: i64) -> String {
    let seconds = ms / 1000;
    let millis = ms % 1000;
    let days_since_epoch = seconds / 86400;
    let secs_in_day = seconds % 86400;

    // Simple calculation: convert seconds since epoch to date
    // This is a simplified version; for production, use chrono or similar
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let secs = secs_in_day % 60;

    // Base date: 1970-01-01 + days_since_epoch
    let year = 1970 + days_since_epoch / 365;
    let day_of_year = days_since_epoch % 365;
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, month, day, hours, minutes, secs, millis
    )
}
