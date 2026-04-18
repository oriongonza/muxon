//! Workspace verb handlers for the Resurreccion daemon.
#![allow(
    clippy::ignored_unit_patterns,
    clippy::significant_drop_in_scrutinee,
    clippy::redundant_pattern_matching,
    clippy::missing_const_for_fn
)]

use crate::dispatch::Handler;
use resurreccion_dir::{canonicalize, compose_binding_key, detect_git, Scope};
use resurreccion_proto::Envelope;
use resurreccion_store::types::WorkspaceRow;
use resurreccion_store::Store;
use std::sync::{Arc, Mutex};
use ulid::Ulid;

/// Handler for `workspace.list` — list all workspaces.
pub struct WorkspaceListHandler {
    store: Arc<Mutex<Store>>,
}

impl WorkspaceListHandler {
    /// Create a new workspace list handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for WorkspaceListHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        match self.store.lock().unwrap().workspace_list() {
            Ok(rows) => Envelope::ok(
                &env.id,
                &env.verb,
                serde_json::to_value(rows).unwrap_or_default(),
            ),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `workspace.create` — create a new workspace.
pub struct WorkspaceCreateHandler {
    store: Arc<Mutex<Store>>,
}

impl WorkspaceCreateHandler {
    /// Create a new workspace create handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for WorkspaceCreateHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let display_name = env
            .body
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");
        let root_path = env
            .body
            .get("root_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let binding_key = env
            .body
            .get("binding_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let now = chrono_now();
        let row = WorkspaceRow {
            id: Ulid::new().to_string(),
            binding_key: binding_key.to_string(),
            display_name: display_name.to_string(),
            root_path: root_path.to_string(),
            created_at: now,
            last_opened_at: None,
        };

        match self.store.lock().unwrap().workspace_insert(&row) {
            Ok(_) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(&row).unwrap()),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `workspace.get` — retrieve a workspace by ID.
pub struct WorkspaceGetHandler {
    store: Arc<Mutex<Store>>,
}

impl WorkspaceGetHandler {
    /// Create a new workspace get handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for WorkspaceGetHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let id = env.body.get("id").and_then(|v| v.as_str()).unwrap_or("");

        match self.store.lock().unwrap().workspace_get(id) {
            Ok(Some(row)) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(row).unwrap()),
            Ok(None) => Envelope::err(&env.id, &env.verb, "not_found", "workspace not found"),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `workspace.resolve_or_create` — resolve workspace by binding key, create if missing.
pub struct WorkspaceResolveOrCreateHandler {
    store: Arc<Mutex<Store>>,
}

impl WorkspaceResolveOrCreateHandler {
    /// Create a new workspace resolve-or-create handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for WorkspaceResolveOrCreateHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let binding_key = env
            .body
            .get("binding_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let root_path = env
            .body
            .get("root_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let display_name = env
            .body
            .get("display_name")
            .and_then(|v| v.as_str())
            .unwrap_or("workspace");

        let store_guard = self.store.lock().unwrap();

        // Try to get by binding key
        if let Ok(Some(existing)) = store_guard.workspace_get_by_key(binding_key) {
            return Envelope::ok(&env.id, &env.verb, serde_json::to_value(existing).unwrap());
        }

        drop(store_guard);

        // Create new workspace
        let now = chrono_now();
        let row = WorkspaceRow {
            id: Ulid::new().to_string(),
            binding_key: binding_key.to_string(),
            display_name: display_name.to_string(),
            root_path: root_path.to_string(),
            created_at: now,
            last_opened_at: None,
        };

        match self.store.lock().unwrap().workspace_insert(&row) {
            Ok(_) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(&row).unwrap()),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `workspace.open` — open or create workspace, attach Mux.
pub struct WorkspaceOpenHandler {
    store: Arc<Mutex<Store>>,
}

impl WorkspaceOpenHandler {
    /// Create a new workspace open handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for WorkspaceOpenHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let path = env.body.get("path").and_then(|v| v.as_str()).unwrap_or("");

        // Canonicalize the path
        let canonical = match canonicalize(path) {
            Ok(p) => p,
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "invalid_path", e.to_string());
            }
        };

        // Detect git info
        let git_info = match detect_git(&canonical) {
            Ok(info) => info,
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "git_error", e.to_string());
            }
        };

        // Compose binding key
        let binding_key = compose_binding_key(&canonical, git_info.as_ref(), Scope::RepoScoped);
        let binding_key_str = binding_key.to_string();

        let store_guard = self.store.lock().unwrap();

        // Try to get existing workspace by binding key
        if let Ok(Some(existing)) = store_guard.workspace_get_by_key(&binding_key_str) {
            // Update last_opened_at
            drop(store_guard);
            if let Ok(_) = self.store.lock().unwrap().workspace_touch(&existing.id) {
                if let Ok(Some(updated)) = self.store.lock().unwrap().workspace_get(&existing.id) {
                    return Envelope::ok(
                        &env.id,
                        &env.verb,
                        serde_json::to_value(updated).unwrap(),
                    );
                }
            }
            return Envelope::ok(&env.id, &env.verb, serde_json::to_value(existing).unwrap());
        }

        drop(store_guard);

        // Create new workspace
        let display_name = canonical.file_name().unwrap_or("workspace").to_string();
        let now = chrono_now();
        let row = WorkspaceRow {
            id: Ulid::new().to_string(),
            binding_key: binding_key_str,
            display_name,
            root_path: canonical.to_string(),
            created_at: now,
            last_opened_at: None,
        };

        match self.store.lock().unwrap().workspace_insert(&row) {
            Ok(()) => {
                // Touch to update last_opened_at
                let _ = self.store.lock().unwrap().workspace_touch(&row.id);
                let get_result = self.store.lock().unwrap().workspace_get(&row.id);
                if let Ok(Some(updated)) = get_result {
                    Envelope::ok(&env.id, &env.verb, serde_json::to_value(updated).unwrap())
                } else {
                    Envelope::ok(&env.id, &env.verb, serde_json::to_value(&row).unwrap())
                }
            }
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Generate current ISO-8601 timestamp for workspace records.
#[allow(clippy::cast_possible_truncation)]
fn chrono_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;

    let seconds = now / 1000;
    let millis = now % 1000;

    format!("2026-04-18T00:00:{:02}.{:03}Z", (seconds % 60), millis)
}
