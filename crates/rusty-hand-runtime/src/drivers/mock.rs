//! Deterministic mock LLM driver for integration tests and offline demos.
//!
//! `MockDriver` is selected by setting `default_model.provider = "mock"` in the
//! kernel config. It accepts any model name, requires no API key, and never
//! makes a network call. The response is a constant text echo of the user's
//! last message (so message round-trip flows can be tested end-to-end without
//! a real LLM).
//!
//! Out of scope: tool calling, structured output, streaming, anything that
//! would make this driver "smart" enough to drive multi-turn agent reasoning.
//! Tests that need branching behaviour should impl `LlmDriver` directly with
//! test-only state — see the `agent_loop::tests` fakes for examples.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError};
use rusty_hand_types::message::{ContentBlock, MessageContent, Role, StopReason, TokenUsage};

/// A deterministic LLM driver that echoes the user's last message.
///
/// Boot a kernel with `provider = "mock"` to use it. No API key needed;
/// every call returns immediately with a synthesized "[mock] <prompt>" reply.
#[derive(Debug, Default)]
pub struct MockDriver;

impl MockDriver {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LlmDriver for MockDriver {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        // Find the most recent user message and use it as the echo source.
        let last_user_text = request
            .messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, Role::User))
            .map(|m| match &m.content {
                MessageContent::Text(t) => t.clone(),
                MessageContent::Blocks(blocks) => blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
            })
            .unwrap_or_default();

        // Mirror approximate token accounting so usage/budget gauges still
        // move during tests. One token per ~4 characters is the usual rule
        // of thumb; close enough for assertions about "non-zero usage".
        let input_tokens = (request
            .messages
            .iter()
            .map(|m| match &m.content {
                MessageContent::Text(t) => t.len(),
                MessageContent::Blocks(b) => b
                    .iter()
                    .map(|blk| match blk {
                        ContentBlock::Text { text } => text.len(),
                        _ => 0,
                    })
                    .sum(),
            })
            .sum::<usize>()
            / 4) as u64
            + 1;

        let reply_text = if last_user_text.is_empty() {
            "[mock] (no user message)".to_string()
        } else {
            format!("[mock] {last_user_text}")
        };
        let output_tokens = (reply_text.len() / 4) as u64 + 1;

        Ok(CompletionResponse {
            content: vec![ContentBlock::Text { text: reply_text }],
            stop_reason: StopReason::EndTurn,
            tool_calls: Vec::new(),
            usage: TokenUsage {
                input_tokens,
                output_tokens,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::agent::ResponseFormat;
    use rusty_hand_types::message::Message;

    fn user_msg(text: &str) -> Message {
        Message {
            role: Role::User,
            content: MessageContent::Text(text.to_string()),
        }
    }

    fn assistant_msg(text: &str) -> Message {
        Message {
            role: Role::Assistant,
            content: MessageContent::Text(text.to_string()),
        }
    }

    fn req(messages: Vec<Message>) -> CompletionRequest {
        CompletionRequest {
            model: "mock-model".to_string(),
            messages,
            tools: Vec::new(),
            max_tokens: 1024,
            temperature: 0.5,
            system: None,
            thinking: None,
            response_format: ResponseFormat::Text,
        }
    }

    #[tokio::test]
    async fn echoes_last_user_message() {
        let driver = MockDriver::new();
        let response = driver
            .complete(req(vec![user_msg("hello world")]))
            .await
            .expect("mock driver always succeeds");
        assert_eq!(response.text(), "[mock] hello world");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        assert!(response.tool_calls.is_empty());
        assert!(response.usage.input_tokens > 0);
        assert!(response.usage.output_tokens > 0);
    }

    #[tokio::test]
    async fn picks_the_most_recent_user_message() {
        let driver = MockDriver::new();
        let response = driver
            .complete(req(vec![
                user_msg("first"),
                assistant_msg("[mock] first"),
                user_msg("second"),
            ]))
            .await
            .unwrap();
        assert_eq!(response.text(), "[mock] second");
    }

    #[tokio::test]
    async fn handles_empty_message_list() {
        let driver = MockDriver::new();
        let response = driver.complete(req(Vec::new())).await.unwrap();
        assert_eq!(response.text(), "[mock] (no user message)");
    }
}
