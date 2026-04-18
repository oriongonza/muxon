//! Workspace verb handlers for the Resurreccion daemon.
#![allow(
    clippy::ignored_unit_patterns,
    clippy::significant_drop_in_scrutinee,
    clippy::redundant_pattern_matching,
    clippy::missing_const_for_fn
)]

use crate::dispatch::Handler;
use resurreccion_dir::{canonicalize, compose_binding_key, detect_git, Scope};
use resurreccion_mux::Mux;
use resurreccion_planner::{plan_capture, plan_restore, SnapshotManifest};
use resurreccion_proto::Envelope;
use resurreccion_store::types::{SnapshotRow, WorkspaceRow};
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

/// Handler for `snapshot.create` — capture and store a snapshot.
pub struct SnapshotCreateHandler {
    store: Arc<Mutex<Store>>,
    mux: Arc<dyn Mux>,
}

impl SnapshotCreateHandler {
    /// Create a new snapshot create handler.
    pub fn new(store: Arc<Mutex<Store>>, mux: Arc<dyn Mux>) -> Self {
        Self { store, mux }
    }
}

impl Handler for SnapshotCreateHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let workspace_id = env
            .body
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if workspace_id.is_empty() {
            return Envelope::err(
                &env.id,
                &env.verb,
                "missing_workspace_id",
                "workspace_id required",
            );
        }

        // Get capabilities from Mux
        let capabilities = self.mux.capabilities();

        // Build capture plan
        let plan = plan_capture(&capabilities);

        // Execute plan
        match resurreccion_planner::execute(&plan, self.mux.as_ref(), &self.store.lock().unwrap()) {
            Ok(_) => {
                // Plan executed successfully
            }
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "plan_execution_failed", e.to_string());
            }
        }

        // For now, we use a minimal manifest JSON. In production, this would contain actual capture data.
        let manifest_json = serde_json::json!({
            "fidelity": "basic",
            "captured_at": chrono_now(),
        })
        .to_string();

        let snapshot = SnapshotRow {
            id: Ulid::new().to_string(),
            workspace_id: workspace_id.to_string(),
            runtime_id: None,
            fidelity: "basic".to_string(),
            manifest_json,
            created_at: chrono_now(),
        };

        match self.store.lock().unwrap().snapshot_insert(&snapshot) {
            Ok(()) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(&snapshot).unwrap()),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `snapshot.restore` — apply a snapshot to a workspace.
pub struct SnapshotRestoreHandler {
    store: Arc<Mutex<Store>>,
    mux: Arc<dyn Mux>,
}

impl SnapshotRestoreHandler {
    /// Create a new snapshot restore handler.
    pub fn new(store: Arc<Mutex<Store>>, mux: Arc<dyn Mux>) -> Self {
        Self { store, mux }
    }
}

impl Handler for SnapshotRestoreHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let snapshot_id = env
            .body
            .get("snapshot_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if snapshot_id.is_empty() {
            return Envelope::err(
                &env.id,
                &env.verb,
                "missing_snapshot_id",
                "snapshot_id required",
            );
        }

        let store_guard = self.store.lock().unwrap();

        let snapshot = match store_guard.snapshot_get(snapshot_id) {
            Ok(Some(snap)) => snap,
            Ok(None) => {
                return Envelope::err(&env.id, &env.verb, "not_found", "snapshot not found");
            }
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "internal", e.to_string());
            }
        };

        drop(store_guard);

        // Parse manifest and build restore plan
        let manifest_value: serde_json::Value = match serde_json::from_str(&snapshot.manifest_json)
        {
            Ok(m) => m,
            Err(e) => {
                return Envelope::err(&env.id, &env.verb, "invalid_manifest", e.to_string());
            }
        };

        // Create a manifest structure for the planner
        let manifest = SnapshotManifest {
            workspace_id: snapshot.workspace_id.clone(),
            layout: manifest_value
                .get("layout")
                .cloned()
                .unwrap_or(serde_json::json!({})),
        };

        let capabilities = self.mux.capabilities();
        let plan = plan_restore(&manifest, &capabilities);

        // Execute plan
        if let Err(e) =
            resurreccion_planner::execute(&plan, self.mux.as_ref(), &self.store.lock().unwrap())
        {
            return Envelope::err(&env.id, &env.verb, "plan_execution_failed", e.to_string());
        }

        Envelope::ok(
            &env.id,
            &env.verb,
            serde_json::json!({"snapshot_id": snapshot_id}),
        )
    }
}

/// Handler for `snapshot.list` — list snapshots for a workspace.
pub struct SnapshotListHandler {
    store: Arc<Mutex<Store>>,
}

impl SnapshotListHandler {
    /// Create a new snapshot list handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for SnapshotListHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let workspace_id = env
            .body
            .get("workspace_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if workspace_id.is_empty() {
            return Envelope::err(
                &env.id,
                &env.verb,
                "missing_workspace_id",
                "workspace_id required",
            );
        }

        match self.store.lock().unwrap().snapshot_list(workspace_id) {
            Ok(rows) => Envelope::ok(
                &env.id,
                &env.verb,
                serde_json::to_value(rows).unwrap_or_default(),
            ),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `snapshot.get` — retrieve a snapshot by ID.
pub struct SnapshotGetHandler {
    store: Arc<Mutex<Store>>,
}

impl SnapshotGetHandler {
    /// Create a new snapshot get handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for SnapshotGetHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let id = env.body.get("id").and_then(|v| v.as_str()).unwrap_or("");

        match self.store.lock().unwrap().snapshot_get(id) {
            Ok(Some(row)) => Envelope::ok(&env.id, &env.verb, serde_json::to_value(row).unwrap()),
            Ok(None) => Envelope::err(&env.id, &env.verb, "not_found", "snapshot not found"),
            Err(e) => Envelope::err(&env.id, &env.verb, "internal", e.to_string()),
        }
    }
}

/// Handler for `events.tail` — stream events from the store.
pub struct EventsTailHandler {
    store: Arc<Mutex<Store>>,
}

impl EventsTailHandler {
    /// Create a new events tail handler.
    pub fn new(store: Arc<Mutex<Store>>) -> Self {
        Self { store }
    }
}

impl Handler for EventsTailHandler {
    fn handle(&self, env: &Envelope) -> Envelope {
        let after_id = env
            .body
            .get("after_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        match self
            .store
            .lock()
            .unwrap()
            .event_tail_from(after_id.as_deref())
        {
            Ok(rows) => Envelope::ok(
                &env.id,
                &env.verb,
                serde_json::to_value(rows).unwrap_or_default(),
            ),
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
