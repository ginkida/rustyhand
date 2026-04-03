//! LLM conversation message types.

use serde::{Deserialize, Serialize};

/// A message in an LLM conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// The role of the sender.
    pub role: Role,
    /// The content of the message.
    pub content: MessageContent,
}

/// The role of a message sender in an LLM conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System prompt.
    System,
    /// Human user.
    User,
    /// AI assistant.
    Assistant,
}

/// Content of a message — can be simple text or structured blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    /// Simple text content.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<ContentBlock>),
}

/// A content block within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// A text block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
    /// An inline base64-encoded image.
    #[serde(rename = "image")]
    Image {
        /// MIME type (e.g. "image/png", "image/jpeg").
        media_type: String,
        /// Base64-encoded image data.
        data: String,
    },
    /// A tool use request from the assistant.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique ID for this tool use.
        id: String,
        /// The tool name.
        name: String,
        /// The tool input parameters.
        input: serde_json::Value,
    },
    /// A tool result from executing a tool.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// The tool_use ID this result corresponds to.
        tool_use_id: String,
        /// The result content.
        content: String,
        /// Whether the tool execution errored.
        is_error: bool,
    },
    /// Extended thinking content block (model's reasoning trace).
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking/reasoning text.
        thinking: String,
    },
    /// Catch-all for unrecognized content block types (forward compatibility).
    #[serde(other)]
    Unknown,
}

impl MessageContent {
    /// Create simple text content.
    pub fn text(content: impl Into<String>) -> Self {
        MessageContent::Text(content.into())
    }

    /// Get the total character length of text in this content.
    pub fn text_length(&self) -> usize {
        match self {
            MessageContent::Text(s) => s.len(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .map(|b| match b {
                    ContentBlock::Text { text } => text.len(),
                    ContentBlock::ToolResult { content, .. } => content.len(),
                    ContentBlock::Thinking { thinking } => thinking.len(),
                    ContentBlock::ToolUse { .. }
                    | ContentBlock::Image { .. }
                    | ContentBlock::Unknown => 0,
                })
                .sum(),
        }
    }

    /// Extract all text content as a single string.
    pub fn text_content(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

impl Message {
    /// Create a system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create a user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }
}

/// Why the LLM stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// The model finished its turn.
    EndTurn,
    /// The model wants to use a tool.
    ToolUse,
    /// The model hit the token limit.
    MaxTokens,
    /// The model hit a stop sequence.
    StopSequence,
}

/// Token usage information from an LLM call.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens used for the input/prompt.
    pub input_tokens: u64,
    /// Tokens generated in the output.
    pub output_tokens: u64,
}

impl TokenUsage {
    /// Total tokens used.
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

/// Reply directives extracted from agent output.
///
/// These control how the response is delivered back to the user/channel:
/// - `reply_to`: reply to a specific message ID
/// - `current_thread`: reply in the current thread
/// - `silent`: suppress the response entirely
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReplyDirectives {
    /// Reply to a specific message ID.
    pub reply_to: Option<String>,
    /// Reply in the current thread.
    pub current_thread: bool,
    /// Suppress the response from being sent.
    pub silent: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        match msg.content {
            MessageContent::Text(text) => assert_eq!(text, "Hello"),
            _ => panic!("Expected text content"),
        }
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn test_content_block_image_serde() {
        let block = ContentBlock::Image {
            media_type: "image/png".to_string(),
            data: "base64data".to_string(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["media_type"], "image/png");
    }

    #[test]
    fn test_content_block_unknown_deser() {
        let json = serde_json::json!({"type": "future_block_type"});
        let block: ContentBlock = serde_json::from_value(json).unwrap();
        assert!(matches!(block, ContentBlock::Unknown));
    }
}
