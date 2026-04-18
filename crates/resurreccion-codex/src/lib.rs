//! resurreccion-codex — `OpenAI`-compatible AI model agent implementation.

use std::env;

use anyhow::anyhow;
use resurreccion_aigents::{Aigent, AigentCapability, Message, Role};
use serde::{Deserialize, Serialize};

/// `OpenAI`-compatible AI model agent.
///
/// Supports `OpenAI`, Azure `OpenAI`, local Ollama endpoints, and any
/// `OpenAI`-compatible API.
pub struct CodexAigent {
    /// The model identifier to use for generation.
    model: String,
    /// The API key for authentication.
    api_key: String,
    /// The base URL for the API endpoint.
    base_url: String,
}

impl CodexAigent {
    /// Create a new Codex Aigent with the given model.
    ///
    /// Reads the `OPENAI_API_KEY` environment variable for authentication.
    /// If the environment variable is not set, an empty string is used.
    /// The base URL defaults to `https://api.openai.com/v1`.
    pub fn new(model: impl Into<String>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        Self {
            model: model.into(),
            api_key,
            base_url: "https://api.openai.com/v1".to_string(),
        }
    }

    /// Create a new Codex Aigent with the given model and a custom base URL.
    ///
    /// Reads the `OPENAI_API_KEY` environment variable for authentication.
    /// If the environment variable is not set, an empty string is used.
    /// Useful for Azure `OpenAI`, Ollama, or any `OpenAI`-compatible endpoint.
    pub fn with_base_url(model: impl Into<String>, base_url: impl Into<String>) -> Self {
        let api_key = env::var("OPENAI_API_KEY").unwrap_or_default();
        Self {
            model: model.into(),
            api_key,
            base_url: base_url.into(),
        }
    }
}

impl Aigent for CodexAigent {
    fn model_id(&self) -> &'static str {
        "codex"
    }

    fn generate(&self, messages: &[Message]) -> anyhow::Result<String> {
        let client = reqwest::blocking::Client::new();

        let api_messages: Vec<OpenAiMessage> = messages
            .iter()
            .map(|msg| OpenAiMessage {
                role: match msg.role {
                    Role::User => "user".to_string(),
                    Role::Assistant => "assistant".to_string(),
                    Role::System => "system".to_string(),
                },
                content: msg.content.clone(),
            })
            .collect();

        let request = OpenAiRequest {
            model: self.model.clone(),
            messages: api_messages,
        };

        let url = format!("{}/chat/completions", self.base_url);

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()?;

        let response_body: OpenAiResponse = response.json()?;

        response_body
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| anyhow!("No choices in response"))
    }

    fn capabilities(&self) -> AigentCapability {
        AigentCapability::FUNCTION_CALLING
    }
}

/// Request structure for `OpenAI` Chat Completions API.
#[derive(Debug, Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
}

/// Message structure for `OpenAI` API.
#[derive(Debug, Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

/// Response structure from `OpenAI` Chat Completions API.
#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

/// A single choice in the `OpenAI` response.
#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

/// The message content within an `OpenAI` choice.
#[derive(Debug, Deserialize)]
struct OpenAiResponseMessage {
    content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_struct_construction() {
        let aigent = CodexAigent::new("gpt-4o");
        assert_eq!(aigent.model, "gpt-4o");
        assert_eq!(aigent.base_url, "https://api.openai.com/v1");
    }

    #[test]
    fn test_model_id_and_capabilities() {
        let aigent = CodexAigent::new("gpt-4o-mini");
        assert_eq!(aigent.model_id(), "codex");
        assert!(aigent
            .capabilities()
            .contains(AigentCapability::FUNCTION_CALLING));
    }

    #[test]
    fn test_with_base_url_sets_url() {
        let aigent = CodexAigent::with_base_url("llama3", "http://localhost:11434/v1");
        assert_eq!(aigent.model, "llama3");
        assert_eq!(aigent.base_url, "http://localhost:11434/v1");
    }
}
