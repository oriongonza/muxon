//! resurreccion-claude — Claude AI model agent implementation.

use std::env;

use anyhow::anyhow;
use resurreccion_aigents::{Aigent, AigentCapability, Message, Role};
use serde::{Deserialize, Serialize};

/// Claude AI model agent.
pub struct ClaudeAigent {
    /// The model identifier.
    model: String,
    /// The API key for authentication.
    api_key: String,
}

impl ClaudeAigent {
    /// Create a new Claude Aigent with the given model.
    ///
    /// Reads the `ANTHROPIC_API_KEY` environment variable for authentication.
    /// If the environment variable is not set, an empty string is used.
    pub fn new(model: impl Into<String>) -> Self {
        let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_default();

        Self {
            model: model.into(),
            api_key,
        }
    }
}

impl Aigent for ClaudeAigent {
    fn model_id(&self) -> &'static str {
        Box::leak(self.model.clone().into_boxed_str())
    }

    fn generate(&self, messages: &[Message]) -> anyhow::Result<String> {
        let client = reqwest::blocking::Client::new();

        // Convert messages to Anthropic format
        let api_messages: Vec<AnthropicMessage> = messages
            .iter()
            .map(|msg| AnthropicMessage {
                role: match msg.role {
                    Role::Assistant => "assistant".to_string(),
                    Role::User | Role::System => "user".to_string(), // Map System to user for Anthropic API
                },
                content: msg.content.clone(),
            })
            .collect();

        let request = AnthropicRequest {
            model: self.model.clone(),
            max_tokens: 1024,
            messages: api_messages,
        };

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()?;

        let response_body: AnthropicResponse = response.json()?;

        // Extract the text content from the response
        match response_body.content.first() {
            Some(AnthropicContent::Text { text }) => Ok(text.clone()),
            None => Err(anyhow!("No content in response")),
        }
    }

    fn capabilities(&self) -> AigentCapability {
        AigentCapability::STREAMING
    }
}

/// Request structure for Anthropic Messages API.
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<AnthropicMessage>,
}

/// Message structure for Anthropic API.
#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

/// Response structure from Anthropic Messages API.
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

/// Content structure in Anthropic response.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicContent {
    #[serde(rename = "text")]
    Text { text: String },
}
