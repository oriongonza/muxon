//! Integration tests for `ClaudeAigent` implementation.

use std::env;

use resurreccion_aigents::{Aigent, AigentCapability, Message, Role};
use resurreccion_claude::ClaudeAigent;

#[test]
fn claude_aigent_model_id_correct() {
    let aigent = ClaudeAigent::new("claude-haiku-4-5-20251001");
    assert_eq!(aigent.model_id(), "claude-haiku-4-5-20251001");
}

#[test]
fn claude_aigent_capabilities() {
    let aigent = ClaudeAigent::new("claude-haiku-4-5-20251001");
    assert!(aigent.capabilities().contains(AigentCapability::STREAMING));
}

#[test]
fn claude_generate_skips_without_api_key() {
    if env::var("ANTHROPIC_API_KEY").is_err() {
        return;
    }

    let aigent = ClaudeAigent::new("claude-haiku-4-5-20251001");
    let messages = vec![Message {
        role: Role::User,
        content: "Hello".to_string(),
    }];

    let result = aigent.generate(&messages);
    // This should succeed if we have the API key
    result.expect("generate should succeed with valid API key");
}
