//! resurreccion-aigents — AI model agent trait and conformance suite.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

/// Message structure for passing to AI models.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message sender.
    pub role: Role,
    /// The content of the message.
    pub content: String,
}

/// Role in a conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// System message.
    System,
}

bitflags! {
    /// Capabilities supported by an Aigent.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AigentCapability: u32 {
        /// Streaming output support.
        const STREAMING = 1;
        /// Function calling support.
        const FUNCTION_CALLING = 2;
        /// Image input support.
        const IMAGE_INPUT = 4;
    }
}

impl Serialize for AigentCapability {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(self.bits())
    }
}

impl<'de> Deserialize<'de> for AigentCapability {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bits = u32::deserialize(deserializer)?;
        // SAFETY: from_bits_retain is always safe; it preserves any bit pattern
        Ok(Self::from_bits_retain(bits))
    }
}

/// Trait defining an AI model agent.
///
/// Implementors must be `Send + Sync + 'static` to allow use across
/// thread boundaries and in dynamic dispatch contexts.
pub trait Aigent: Send + Sync + 'static {
    /// Return the model identifier.
    fn model_id(&self) -> &'static str;

    /// Generate a response from the given messages.
    fn generate(&self, messages: &[Message]) -> anyhow::Result<String>;

    /// Return the capabilities of this Aigent.
    fn capabilities(&self) -> AigentCapability;
}

/// Conformance testing module for Aigent implementations.
pub mod conformance {
    use super::{Aigent, Message, Role};

    /// Run conformance checks on an Aigent implementation.
    ///
    /// Verifies:
    /// - `model_id` is non-empty
    /// - `generate` with a single user message returns Ok
    pub fn run<A: Aigent>(aigent: &A) -> anyhow::Result<()> {
        let model_id = aigent.model_id();
        anyhow::ensure!(!model_id.is_empty(), "model_id must not be empty");

        let messages = vec![Message {
            role: Role::User,
            content: "Hello".to_string(),
        }];

        let _response = aigent.generate(&messages)?;

        Ok(())
    }
}
