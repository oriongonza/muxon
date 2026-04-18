//! Tests for the `Editor` trait and `EditorCapture` type.

use resurreccion_editor::{Editor, EditorCapture};
use serde_json::json;

#[test]
fn editor_capture_serializes() {
    let capture = EditorCapture {
        editor_name: "vim".to_string(),
        cwd: "/home/user".to_string(),
        open_files: vec!["main.rs".to_string(), "lib.rs".to_string()],
        raw_state: json!({ "buffers": 2 }),
    };

    let json = serde_json::to_string(&capture).expect("serialize failed");
    let deserialized: EditorCapture =
        serde_json::from_str(&json).expect("deserialize failed");

    assert_eq!(capture.editor_name, deserialized.editor_name);
    assert_eq!(capture.cwd, deserialized.cwd);
    assert_eq!(capture.open_files, deserialized.open_files);
    assert_eq!(capture.raw_state, deserialized.raw_state);
}

#[test]
fn conformance_run_calls_editor_name() {
    struct StubEditor;

    impl Editor for StubEditor {
        fn editor_name(&self) -> &'static str {
            "stub"
        }

        fn capture(&self) -> anyhow::Result<EditorCapture> {
            Ok(EditorCapture {
                editor_name: "stub".to_string(),
                cwd: "/tmp".to_string(),
                open_files: vec![],
                raw_state: json!({}),
            })
        }

        fn restore(&self, _capture: &EditorCapture) -> anyhow::Result<()> {
            Ok(())
        }
    }

    let editor = StubEditor;
    resurreccion_editor::conformance::run(&editor);
}

#[test]
fn assert_editor_is_object_safe() {
    fn _assert(_: &dyn crate::Editor) {}
}
