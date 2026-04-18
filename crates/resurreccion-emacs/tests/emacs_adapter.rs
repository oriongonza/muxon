//! Integration tests for `EmacsAdapter`.

#![allow(missing_docs)]

use resurreccion_editor::{Editor, EditorCapture};
use resurreccion_emacs::EmacsAdapter;

#[test]
fn emacs_adapter_implements_editor_trait() {
    let adapter = EmacsAdapter::new();
    assert_eq!(adapter.editor_name(), "emacs");
}

#[test]
fn capture_graceful_without_emacsclient() {
    let adapter = EmacsAdapter::new();
    let result = adapter.capture();
    assert!(
        result.is_ok(),
        "capture must return Ok even without emacsclient"
    );
    let cap = result.unwrap();
    assert_eq!(cap.editor_name, "emacs");
    assert!(!cap.cwd.is_empty(), "cwd must be non-empty");
}

#[test]
fn restore_returns_ok_for_any_capture() {
    let adapter = EmacsAdapter::new();
    let capture = EditorCapture {
        editor_name: "emacs".to_string(),
        cwd: "/tmp".to_string(),
        open_files: vec![],
        raw_state: serde_json::json!({}),
    };
    assert!(adapter.restore(&capture).is_ok());
}

#[test]
fn with_socket_constructor() {
    let adapter = EmacsAdapter::with_socket("test");
    assert_eq!(adapter.editor_name(), "emacs");
}

#[test]
fn dyn_editor_works() {
    let adapter: Box<dyn Editor> = Box::new(EmacsAdapter::new());
    assert_eq!(adapter.editor_name(), "emacs");
}
