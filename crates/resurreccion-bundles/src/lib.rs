//! Snapshot bundle packaging for the Resurreccion system.
//!
//! A [`Bundle`] groups multiple capture artifacts (layout, shell, editor, blob)
//! into a single portable unit that can be serialised to JSON or stored in the
//! content-addressed blob store.

#![deny(missing_docs)]

use anyhow::Result;

/// A bundle groups multiple capture artifacts into one portable unit.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Bundle {
    /// Schema version; currently always `1`.
    pub version: u32,
    /// The workspace this bundle belongs to.
    pub workspace_id: String,
    /// ISO-8601-ish timestamp of bundle creation.
    pub created_at: String,
    /// Ordered list of artifacts contained in this bundle.
    pub artifacts: Vec<BundleArtifact>,
}

/// A single artifact within a [`Bundle`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BundleArtifact {
    /// Artifact kind: `"layout"`, `"shell"`, `"editor"`, or `"blob"`.
    pub kind: String,
    /// Human-readable label for this artifact.
    pub label: String,
    /// The artifact data as an arbitrary JSON value.
    pub content: serde_json::Value,
    /// If the artifact data is stored in the blob store, its BLAKE3 hex hash.
    pub blob_hash: Option<String>,
}

impl Bundle {
    /// Create a new, empty bundle for the given workspace.
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            version: 1,
            workspace_id: workspace_id.into(),
            created_at: iso_now(),
            artifacts: vec![],
        }
    }

    /// Append an artifact to this bundle.
    ///
    /// Returns `&mut Self` so calls can be chained.
    pub fn add_artifact(
        &mut self,
        kind: impl Into<String>,
        label: impl Into<String>,
        content: serde_json::Value,
    ) -> &mut Self {
        self.artifacts.push(BundleArtifact {
            kind: kind.into(),
            label: label.into(),
            content,
            blob_hash: None,
        });
        self
    }

    /// Serialize bundle to pretty-printed JSON bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec_pretty(self)?)
    }

    /// Deserialize bundle from JSON bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        Ok(serde_json::from_slice(data)?)
    }

    /// Store this bundle as a blob in the store. Returns the blob hash.
    pub fn store_as_blob(&self, store: &resurreccion_store::Store) -> Result<String> {
        let bytes = self.to_bytes()?;
        store.blob_put(&bytes)
    }

    /// Load a bundle from the blob store by hash.
    ///
    /// Returns `None` if no blob with that hash exists.
    pub fn load_from_blob(store: &resurreccion_store::Store, hash: &str) -> Result<Option<Self>> {
        match store.blob_get(hash)? {
            Some(data) => Ok(Some(Self::from_bytes(&data)?)),
            None => Ok(None),
        }
    }
}

/// Return a Unix-epoch-seconds timestamp string, e.g. `"1713398400Z"`.
fn iso_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{now}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_store() -> resurreccion_store::Store {
        resurreccion_store::Store::open(":memory:").expect("in-memory store must open")
    }

    #[test]
    fn new_bundle_has_version_1() {
        let bundle = Bundle::new("ws-001");
        assert_eq!(bundle.version, 1);
        assert_eq!(bundle.workspace_id, "ws-001");
        assert!(bundle.artifacts.is_empty());
    }

    #[test]
    fn add_artifact_appends() {
        let mut bundle = Bundle::new("ws-002");
        bundle.add_artifact("layout", "main layout", serde_json::json!({"panes": 2}));
        bundle.add_artifact("shell", "bash capture", serde_json::json!({"rows": 24}));

        assert_eq!(bundle.artifacts.len(), 2);
        assert_eq!(bundle.artifacts[0].kind, "layout");
        assert_eq!(bundle.artifacts[0].label, "main layout");
        assert_eq!(bundle.artifacts[1].kind, "shell");
        assert!(bundle.artifacts[0].blob_hash.is_none());
    }

    #[test]
    fn round_trip_serialization() {
        let mut original = Bundle::new("ws-003");
        original.add_artifact(
            "editor",
            "nvim state",
            serde_json::json!({"file": "main.rs"}),
        );

        let bytes = original.to_bytes().expect("to_bytes must succeed");
        let recovered = Bundle::from_bytes(&bytes).expect("from_bytes must succeed");

        assert_eq!(recovered.version, original.version);
        assert_eq!(recovered.workspace_id, original.workspace_id);
        assert_eq!(recovered.artifacts.len(), original.artifacts.len());
        assert_eq!(recovered.artifacts[0].kind, original.artifacts[0].kind);
        assert_eq!(recovered.artifacts[0].label, original.artifacts[0].label);
        assert_eq!(
            recovered.artifacts[0].content,
            original.artifacts[0].content
        );
    }

    #[test]
    fn store_and_load_from_blob() {
        let store = open_store();
        let mut bundle = Bundle::new("ws-004");
        bundle.add_artifact("blob", "raw capture", serde_json::json!({"size": 1024}));

        let hash = bundle
            .store_as_blob(&store)
            .expect("store_as_blob must succeed");
        assert!(!hash.is_empty(), "hash must not be empty");

        let loaded = Bundle::load_from_blob(&store, &hash)
            .expect("load_from_blob must not error")
            .expect("bundle must be present");

        assert_eq!(loaded.workspace_id, bundle.workspace_id);
        assert_eq!(loaded.artifacts.len(), bundle.artifacts.len());
        assert_eq!(loaded.artifacts[0].content, bundle.artifacts[0].content);
    }

    #[test]
    fn load_nonexistent_hash_returns_none() {
        let store = open_store();
        let fake_hash = "0".repeat(64);
        let result = Bundle::load_from_blob(&store, &fake_hash)
            .expect("load_from_blob for missing hash must not error");
        assert!(result.is_none(), "must return None for unknown hash");
    }
}
