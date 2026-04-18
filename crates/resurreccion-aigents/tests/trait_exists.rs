//! Tests for Aigent trait seam.

use resurreccion_aigents::{Aigent, AigentCapability, Message, Role};

/// Verifies that dyn Aigent can be created and is object-safe.
#[test]
fn assert_aigent_is_object_safe() {
    // This test verifies that dyn Aigent can be created.
    // If this doesn't compile, the trait is not object-safe.
    struct MockAigent;

    impl Aigent for MockAigent {
        fn model_id(&self) -> &'static str {
            "mock-model"
        }

        fn generate(&self, _messages: &[Message]) -> anyhow::Result<String> {
            Ok("response".to_string())
        }

        fn capabilities(&self) -> AigentCapability {
            AigentCapability::empty()
        }
    }

    let aigent: Box<dyn Aigent> = Box::new(MockAigent);
    assert_eq!(aigent.model_id(), "mock-model");
}

/// Verifies that capability flags are properly implemented as bitflags.
#[test]
fn capability_flags_are_bitflags() {
    let streaming = AigentCapability::STREAMING;
    let function_calling = AigentCapability::FUNCTION_CALLING;
    let combined = streaming | function_calling;

    assert_eq!(streaming.bits(), 1);
    assert_eq!(function_calling.bits(), 2);
    assert_eq!(combined.bits(), 3);

    // Test empty
    assert_eq!(AigentCapability::empty().bits(), 0);

    // Test all three
    let image = AigentCapability::IMAGE_INPUT;
    let all = streaming | function_calling | image;
    assert_eq!(all.bits(), 7);
}

/// Verifies that Role can be serialized and deserialized correctly.
#[test]
fn message_role_roundtrips() {
    let user_role = Role::User;
    let assistant_role = Role::Assistant;
    let system_role = Role::System;

    // Just verify they exist and can be compared
    assert_eq!(user_role, Role::User);
    assert_eq!(assistant_role, Role::Assistant);
    assert_eq!(system_role, Role::System);
    assert_ne!(user_role, assistant_role);

    // Test serde: serialize and deserialize
    let user_json = serde_json::to_string(&user_role).expect("should serialize");
    let user_deserialized: Role = serde_json::from_str(&user_json).expect("should deserialize");
    assert_eq!(user_deserialized, Role::User);
}
