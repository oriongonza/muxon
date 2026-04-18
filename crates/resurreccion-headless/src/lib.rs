//! Headless/CI mode for Resurreccion.
//!
//! Reads a JSON command file or stdin, executes verbs against a store,
//! and writes JSON results to stdout or a file.

use serde_json::Value;

/// A single command in headless mode.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadlessCommand {
    /// The verb identifying the operation to perform.
    pub verb: String,
    /// The JSON body payload for the command.
    pub body: Value,
}

/// Result of executing a headless command.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadlessResult {
    /// The verb that was executed.
    pub verb: String,
    /// Whether the command succeeded.
    pub success: bool,
    /// The JSON body of the result.
    pub body: Value,
    /// An optional error message if the command failed.
    pub error: Option<String>,
}

/// A batch of headless commands.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeadlessBatch {
    /// The list of commands to execute.
    pub commands: Vec<HeadlessCommand>,
}

impl HeadlessBatch {
    /// Parse a batch from a JSON string.
    pub fn from_json(s: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(s)?)
    }

    /// Construct a batch from a single verb and body.
    pub fn from_single(verb: impl Into<String>, body: Value) -> Self {
        Self {
            commands: vec![HeadlessCommand {
                verb: verb.into(),
                body,
            }],
        }
    }
}

/// Execute a batch of headless commands and return results.
///
/// For 0.x: only handles verbs that don't require a live mux (store-only operations).
/// Unknown verbs return a [`HeadlessResult`] with `success=false`.
pub fn execute_batch(
    batch: &HeadlessBatch,
    store: &resurreccion_store::Store,
) -> Vec<HeadlessResult> {
    batch
        .commands
        .iter()
        .map(|cmd| execute_one(cmd, store))
        .collect()
}

fn execute_one(cmd: &HeadlessCommand, store: &resurreccion_store::Store) -> HeadlessResult {
    match cmd.verb.as_str() {
        "workspace.list" => match store.workspace_list() {
            Ok(rows) => HeadlessResult {
                verb: cmd.verb.clone(),
                success: true,
                body: serde_json::to_value(&rows).unwrap_or(Value::Null),
                error: None,
            },
            Err(e) => HeadlessResult {
                verb: cmd.verb.clone(),
                success: false,
                body: Value::Null,
                error: Some(e.to_string()),
            },
        },
        other => HeadlessResult {
            verb: other.to_string(),
            success: false,
            body: Value::Null,
            error: Some(format!("verb not supported in headless mode: {other}")),
        },
    }
}

/// Format results as newline-delimited JSON (NDJSON) for stdout.
pub fn results_to_ndjson(results: &[HeadlessResult]) -> String {
    results
        .iter()
        .map(|r| serde_json::to_string(r).unwrap_or_default())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_store() -> (tempfile::TempDir, resurreccion_store::Store) {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("test.db");
        let store = resurreccion_store::Store::open(path.to_str().unwrap()).expect("store");
        (dir, store)
    }

    #[test]
    fn from_json_parses_correctly() {
        let input = r#"{"commands":[{"verb":"workspace.list","body":{}}]}"#;
        let batch = HeadlessBatch::from_json(input).expect("parse");
        assert_eq!(batch.commands.len(), 1);
        assert_eq!(batch.commands[0].verb, "workspace.list");
        assert_eq!(batch.commands[0].body, json!({}));
    }

    #[test]
    fn from_json_multi_command() {
        let input = r#"{"commands":[{"verb":"workspace.list","body":{}},{"verb":"unknown","body":{"x":1}}]}"#;
        let batch = HeadlessBatch::from_json(input).expect("parse");
        assert_eq!(batch.commands.len(), 2);
        assert_eq!(batch.commands[1].verb, "unknown");
    }

    #[test]
    fn unknown_verb_returns_error_result() {
        let (_dir, store) = make_store();
        let batch = HeadlessBatch::from_single("workspace.nope", json!({}));
        let results = execute_batch(&batch, &store);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0]
            .error
            .as_deref()
            .unwrap_or("")
            .contains("not supported"));
    }

    #[test]
    fn workspace_list_on_empty_store_returns_success_with_empty_list() {
        let (_dir, store) = make_store();
        let batch = HeadlessBatch::from_single("workspace.list", json!({}));
        let results = execute_batch(&batch, &store);
        assert_eq!(results.len(), 1);
        assert!(results[0].success);
        assert_eq!(results[0].body, json!([]));
        assert!(results[0].error.is_none());
    }

    #[test]
    fn results_to_ndjson_formats_correctly() {
        let results = vec![
            HeadlessResult {
                verb: "workspace.list".to_string(),
                success: true,
                body: json!([]),
                error: None,
            },
            HeadlessResult {
                verb: "unknown".to_string(),
                success: false,
                body: Value::Null,
                error: Some("verb not supported".to_string()),
            },
        ];
        let ndjson = results_to_ndjson(&results);
        let lines: Vec<&str> = ndjson.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line must be valid JSON
        let first: Value = serde_json::from_str(lines[0]).expect("valid json line 0");
        let second: Value = serde_json::from_str(lines[1]).expect("valid json line 1");
        assert_eq!(first["success"], json!(true));
        assert_eq!(second["success"], json!(false));
        assert_eq!(second["error"], json!("verb not supported"));
    }
}
