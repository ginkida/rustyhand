//! OpenAI-compatible API driver.
//!
//! Works with OpenAI, Ollama, vLLM, and any other OpenAI-compatible endpoint.

use crate::llm_driver::{CompletionRequest, CompletionResponse, LlmDriver, LlmError, StreamEvent};
use async_trait::async_trait;
use futures::StreamExt;
use rusty_hand_types::message::{ContentBlock, MessageContent, Role, StopReason, TokenUsage};
use rusty_hand_types::tool::ToolCall;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use zeroize::Zeroizing;

/// OpenAI-compatible API driver.
pub struct OpenAIDriver {
    api_key: Zeroizing<String>,
    base_url: String,
    client: reqwest::Client,
}

impl OpenAIDriver {
    /// Create a new OpenAI-compatible driver.
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            api_key: Zeroizing::new(api_key),
            base_url,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(300))
                .build()
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Serialize)]
struct OaiRequest {
    model: String,
    messages: Vec<OaiMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<OaiTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
    /// JSON mode. `{"type":"json_object"}` forces the model to output valid JSON.
    /// Omitted when not needed (most calls) to avoid breaking providers that
    /// don't support the field (e.g. older Ollama models).
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct OaiMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<OaiMessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OaiToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    /// Reasoning content echoed back to the provider. DeepSeek V4 reasoning
    /// models reject multi-turn requests when the previous assistant turn had
    /// a reasoning trace and we omit this field ("The `reasoning_content` in
    /// the thinking mode must be passed back to the API."). Other OpenAI-
    /// compat providers ignore unknown fields, so keeping this on all
    /// outbound assistant messages is safe.
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_content: Option<String>,
}

/// Content can be a plain string or an array of content parts (for images).
#[derive(Debug, Serialize)]
#[serde(untagged)]
enum OaiMessageContent {
    Text(String),
    Parts(Vec<OaiContentPart>),
}

/// A content part for multi-modal messages.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum OaiContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: OaiImageUrl },
}

#[derive(Debug, Serialize)]
struct OaiImageUrl {
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct OaiToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OaiFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OaiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct OaiTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OaiToolDef,
}

#[derive(Debug, Serialize)]
struct OaiToolDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OaiResponse {
    choices: Vec<OaiChoice>,
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
struct OaiChoice {
    message: OaiResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaiResponseMessage {
    content: Option<String>,
    /// Extended thinking content (DeepSeek R1, QwQ, etc.)
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OaiUsage {
    prompt_tokens: u64,
    completion_tokens: u64,
}

#[async_trait]
impl LlmDriver for OpenAIDriver {
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        let mut oai_messages: Vec<OaiMessage> = Vec::new();

        // Add system message if present
        if let Some(ref system) = request.system {
            oai_messages.push(OaiMessage {
                role: "system".to_string(),
                content: Some(OaiMessageContent::Text(system.clone())),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        // Convert messages
        for msg in &request.messages {
            match (&msg.role, &msg.content) {
                (Role::System, MessageContent::Text(text)) if request.system.is_none() => {
                    oai_messages.push(OaiMessage {
                        role: "system".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::User, MessageContent::Text(text)) => {
                    oai_messages.push(OaiMessage {
                        role: "user".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::Assistant, MessageContent::Text(text)) => {
                    oai_messages.push(OaiMessage {
                        role: "assistant".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::User, MessageContent::Blocks(blocks)) => {
                    // Handle tool results and images in user messages
                    let mut parts: Vec<OaiContentPart> = Vec::new();
                    let mut has_tool_results = false;
                    for block in blocks {
                        match block {
                            ContentBlock::ToolResult {
                                tool_use_id,
                                content,
                                ..
                            } => {
                                has_tool_results = true;
                                oai_messages.push(OaiMessage {
                                    role: "tool".to_string(),
                                    content: Some(OaiMessageContent::Text(content.clone())),
                                    tool_calls: None,
                                    tool_call_id: Some(tool_use_id.clone()),
                                    reasoning_content: None,
                                });
                            }
                            ContentBlock::Text { text } => {
                                parts.push(OaiContentPart::Text { text: text.clone() });
                            }
                            ContentBlock::Image { media_type, data } => {
                                parts.push(OaiContentPart::ImageUrl {
                                    image_url: OaiImageUrl {
                                        url: format!("data:{media_type};base64,{data}"),
                                    },
                                });
                            }
                            ContentBlock::Thinking { .. } => {}
                            _ => {}
                        }
                    }
                    if !parts.is_empty() && !has_tool_results {
                        oai_messages.push(OaiMessage {
                            role: "user".to_string(),
                            content: Some(OaiMessageContent::Parts(parts)),
                            tool_calls: None,
                            tool_call_id: None,
                            reasoning_content: None,
                        });
                    }
                }
                (Role::Assistant, MessageContent::Blocks(blocks)) => {
                    let mut text_parts = Vec::new();
                    let mut tool_calls = Vec::new();
                    let mut thinking_parts = Vec::new();
                    for block in blocks {
                        match block {
                            ContentBlock::Text { text } => text_parts.push(text.clone()),
                            ContentBlock::ToolUse { id, name, input } => {
                                tool_calls.push(OaiToolCall {
                                    id: id.clone(),
                                    call_type: "function".to_string(),
                                    function: OaiFunction {
                                        name: name.clone(),
                                        arguments: serde_json::to_string(input).unwrap_or_default(),
                                    },
                                });
                            }
                            ContentBlock::Thinking { thinking } => {
                                thinking_parts.push(thinking.clone());
                            }
                            _ => {}
                        }
                    }
                    oai_messages.push(OaiMessage {
                        role: "assistant".to_string(),
                        content: if text_parts.is_empty() {
                            None
                        } else {
                            Some(OaiMessageContent::Text(text_parts.join("")))
                        },
                        tool_calls: if tool_calls.is_empty() {
                            None
                        } else {
                            Some(tool_calls)
                        },
                        tool_call_id: None,
                        reasoning_content: if thinking_parts.is_empty() {
                            None
                        } else {
                            Some(thinking_parts.join("\n"))
                        },
                    });
                }
                _ => {}
            }
        }

        let oai_tools: Vec<OaiTool> = request
            .tools
            .iter()
            .map(|t| OaiTool {
                tool_type: "function".to_string(),
                function: OaiToolDef {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: rusty_hand_types::tool::normalize_schema_for_provider(
                        &t.input_schema,
                        "openai",
                    ),
                },
            })
            .collect();

        let tool_choice = if oai_tools.is_empty() {
            None
        } else {
            Some(serde_json::json!("auto"))
        };

        let json_mode = request.response_format == rusty_hand_types::agent::ResponseFormat::Json;
        let mut oai_request = OaiRequest {
            model: request.model.clone(),
            messages: oai_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            tools: oai_tools,
            tool_choice,
            stream: false,
            response_format: if json_mode {
                Some(serde_json::json!({"type": "json_object"}))
            } else {
                None
            },
        };

        let max_retries = 3;
        for attempt in 0..=max_retries {
            let url = format!("{}/chat/completions", self.base_url);
            debug!(url = %url, attempt, "Sending OpenAI API request");

            let mut req_builder = self
                .client
                .post(&url)
                .header("content-type", "application/json")
                .json(&oai_request);

            if !self.api_key.as_str().is_empty() {
                req_builder = req_builder
                    .header("authorization", format!("Bearer {}", self.api_key.as_str()));
            }

            let resp = req_builder
                .send()
                .await
                .map_err(|e| LlmError::Http(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 {
                if attempt < max_retries {
                    let retry_ms = (attempt + 1) as u64 * 2000;
                    warn!(status, retry_ms, "Rate limited, retrying");
                    tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                    continue;
                }
                return Err(LlmError::RateLimited {
                    retry_after_ms: 5000,
                });
            }

            if !resp.status().is_success() {
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

                // Groq "tool_use_failed": model generated tool call in XML format.
                // Parse the failed_generation and convert to a proper tool call response.
                if status == 400 && body.contains("tool_use_failed") {
                    if let Some(response) = parse_groq_failed_tool_call(&body) {
                        warn!("Recovered tool call from Groq failed_generation");
                        return Ok(response);
                    }
                    // If parsing fails, retry on next attempt
                    if attempt < max_retries {
                        let retry_ms = (attempt + 1) as u64 * 1500;
                        warn!(status, attempt, retry_ms, "tool_use_failed, retrying");
                        tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                        continue;
                    }
                }

                // Auto-cap max_tokens when model rejects our value (e.g. Groq Maverick limit 8192)
                if status == 400 && body.contains("max_tokens") && attempt < max_retries {
                    // Extract the limit from error: "must be less than or equal to `8192`"
                    let cap = extract_max_tokens_limit(&body).unwrap_or(oai_request.max_tokens / 2);
                    warn!(
                        old = oai_request.max_tokens,
                        new = cap,
                        "Auto-capping max_tokens to model limit"
                    );
                    oai_request.max_tokens = cap;
                    continue;
                }

                return Err(LlmError::Api {
                    status,
                    message: body,
                });
            }

            let body = resp
                .text()
                .await
                .map_err(|e| LlmError::Http(e.to_string()))?;
            let oai_response: OaiResponse =
                serde_json::from_str(&body).map_err(|e| LlmError::Parse(e.to_string()))?;

            let choice = oai_response
                .choices
                .into_iter()
                .next()
                .ok_or_else(|| LlmError::Parse("No choices in response".to_string()))?;

            let mut content = Vec::new();
            let mut tool_calls = Vec::new();

            // Handle reasoning_content field (DeepSeek R1, QwQ, etc.)
            if let Some(reasoning) = choice.message.reasoning_content {
                if !reasoning.is_empty() {
                    content.push(ContentBlock::Thinking {
                        thinking: reasoning,
                    });
                }
            }

            if let Some(text) = choice.message.content {
                if !text.is_empty() {
                    // Parse <think>...</think> tags (MiniMax, QwQ, etc.)
                    let (visible, thinking) = extract_think_tags(&text);
                    if !thinking.is_empty() {
                        content.push(ContentBlock::Thinking { thinking });
                    }
                    if !visible.trim().is_empty() {
                        content.push(ContentBlock::Text { text: visible });
                    }
                }
            }

            if let Some(calls) = choice.message.tool_calls {
                for call in calls {
                    let input: serde_json::Value =
                        serde_json::from_str(&call.function.arguments).unwrap_or_else(|e| {
                            warn!(tool = %call.function.name, error = %e, "Failed to parse tool call arguments, using empty object");
                            serde_json::Value::Object(Default::default())
                        });
                    content.push(ContentBlock::ToolUse {
                        id: call.id.clone(),
                        name: call.function.name.clone(),
                        input: input.clone(),
                    });
                    tool_calls.push(ToolCall {
                        id: call.id,
                        name: call.function.name,
                        input,
                    });
                }
            }

            let stop_reason = match choice.finish_reason.as_deref() {
                Some("stop") => StopReason::EndTurn,
                Some("tool_calls") => StopReason::ToolUse,
                Some("length") => StopReason::MaxTokens,
                _ => {
                    if !tool_calls.is_empty() {
                        StopReason::ToolUse
                    } else {
                        StopReason::EndTurn
                    }
                }
            };

            let usage = oai_response
                .usage
                .map(|u| TokenUsage {
                    input_tokens: u.prompt_tokens,
                    output_tokens: u.completion_tokens,
                })
                .unwrap_or_default();

            return Ok(CompletionResponse {
                content,
                stop_reason,
                tool_calls,
                usage,
            });
        }

        Err(LlmError::Api {
            status: 0,
            message: "Max retries exceeded".to_string(),
        })
    }

    async fn stream(
        &self,
        request: CompletionRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<CompletionResponse, LlmError> {
        // Build request (same as complete but with stream: true)
        let mut oai_messages: Vec<OaiMessage> = Vec::new();

        if let Some(ref system) = request.system {
            oai_messages.push(OaiMessage {
                role: "system".to_string(),
                content: Some(OaiMessageContent::Text(system.clone())),
                tool_calls: None,
                tool_call_id: None,
                reasoning_content: None,
            });
        }

        for msg in &request.messages {
            match (&msg.role, &msg.content) {
                (Role::System, MessageContent::Text(text)) if request.system.is_none() => {
                    oai_messages.push(OaiMessage {
                        role: "system".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::User, MessageContent::Text(text)) => {
                    oai_messages.push(OaiMessage {
                        role: "user".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::Assistant, MessageContent::Text(text)) => {
                    oai_messages.push(OaiMessage {
                        role: "assistant".to_string(),
                        content: Some(OaiMessageContent::Text(text.clone())),
                        tool_calls: None,
                        tool_call_id: None,
                        reasoning_content: None,
                    });
                }
                (Role::User, MessageContent::Blocks(blocks)) => {
                    for block in blocks {
                        if let ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                            ..
                        } = block
                        {
                            oai_messages.push(OaiMessage {
                                role: "tool".to_string(),
                                content: Some(OaiMessageContent::Text(content.clone())),
                                tool_calls: None,
                                tool_call_id: Some(tool_use_id.clone()),
                                reasoning_content: None,
                            });
                        }
                    }
                }
                (Role::Assistant, MessageContent::Blocks(blocks)) => {
                    let mut text_parts = Vec::new();
                    let mut tool_calls_out = Vec::new();
                    let mut thinking_parts = Vec::new();
                    for block in blocks {
                        match block {
                            ContentBlock::Text { text } => text_parts.push(text.clone()),
                            ContentBlock::ToolUse { id, name, input } => {
                                tool_calls_out.push(OaiToolCall {
                                    id: id.clone(),
                                    call_type: "function".to_string(),
                                    function: OaiFunction {
                                        name: name.clone(),
                                        arguments: serde_json::to_string(input).unwrap_or_default(),
                                    },
                                });
                            }
                            ContentBlock::Thinking { thinking } => {
                                thinking_parts.push(thinking.clone());
                            }
                            _ => {}
                        }
                    }
                    oai_messages.push(OaiMessage {
                        role: "assistant".to_string(),
                        content: if text_parts.is_empty() {
                            None
                        } else {
                            Some(OaiMessageContent::Text(text_parts.join("")))
                        },
                        tool_calls: if tool_calls_out.is_empty() {
                            None
                        } else {
                            Some(tool_calls_out)
                        },
                        tool_call_id: None,
                        reasoning_content: if thinking_parts.is_empty() {
                            None
                        } else {
                            Some(thinking_parts.join("\n"))
                        },
                    });
                }
                _ => {}
            }
        }

        let oai_tools: Vec<OaiTool> = request
            .tools
            .iter()
            .map(|t| OaiTool {
                tool_type: "function".to_string(),
                function: OaiToolDef {
                    name: t.name.clone(),
                    description: t.description.clone(),
                    parameters: rusty_hand_types::tool::normalize_schema_for_provider(
                        &t.input_schema,
                        "openai",
                    ),
                },
            })
            .collect();

        let tool_choice = if oai_tools.is_empty() {
            None
        } else {
            Some(serde_json::json!("auto"))
        };

        let json_mode_stream =
            request.response_format == rusty_hand_types::agent::ResponseFormat::Json;
        let mut oai_request = OaiRequest {
            model: request.model.clone(),
            messages: oai_messages,
            max_tokens: request.max_tokens,
            temperature: request.temperature,
            tools: oai_tools,
            tool_choice,
            stream: true,
            response_format: if json_mode_stream {
                Some(serde_json::json!({"type": "json_object"}))
            } else {
                None
            },
        };

        // Retry loop for the initial HTTP request
        let max_retries = 3;
        for attempt in 0..=max_retries {
            let url = format!("{}/chat/completions", self.base_url);
            debug!(url = %url, attempt, "Sending OpenAI streaming request");

            let mut req_builder = self
                .client
                .post(&url)
                .header("content-type", "application/json")
                .json(&oai_request);

            if !self.api_key.as_str().is_empty() {
                req_builder = req_builder
                    .header("authorization", format!("Bearer {}", self.api_key.as_str()));
            }

            let resp = req_builder
                .send()
                .await
                .map_err(|e| LlmError::Http(e.to_string()))?;

            let status = resp.status().as_u16();
            if status == 429 {
                if attempt < max_retries {
                    let retry_ms = (attempt + 1) as u64 * 2000;
                    warn!(status, retry_ms, "Rate limited (stream), retrying");
                    tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                    continue;
                }
                return Err(LlmError::RateLimited {
                    retry_after_ms: 5000,
                });
            }

            if !resp.status().is_success() {
                let body = resp
                    .text()
                    .await
                    .unwrap_or_else(|e| format!("<failed to read body: {e}>"));

                // Groq "tool_use_failed": parse and recover (streaming path)
                if status == 400 && body.contains("tool_use_failed") {
                    if let Some(response) = parse_groq_failed_tool_call(&body) {
                        warn!("Recovered tool call from Groq failed_generation (stream)");
                        return Ok(response);
                    }
                    if attempt < max_retries {
                        let retry_ms = (attempt + 1) as u64 * 1500;
                        warn!(
                            status,
                            attempt, retry_ms, "tool_use_failed (stream), retrying"
                        );
                        tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                        continue;
                    }
                }

                // Auto-cap max_tokens when model rejects our value
                if status == 400 && body.contains("max_tokens") && attempt < max_retries {
                    let cap = extract_max_tokens_limit(&body).unwrap_or(oai_request.max_tokens / 2);
                    warn!(
                        old = oai_request.max_tokens,
                        new = cap,
                        "Auto-capping max_tokens (stream)"
                    );
                    oai_request.max_tokens = cap;
                    continue;
                }

                return Err(LlmError::Api {
                    status,
                    message: body,
                });
            }

            // Parse the SSE stream
            let mut buffer = String::new();
            let mut text_content = String::new();
            let mut thinking_content = String::new();
            let mut in_thinking = false; // tracks <think> tag state for streaming
                                         // Track tool calls: index -> (id, name, arguments)
            let mut tool_accum: Vec<(String, String, String)> = Vec::new();
            let mut finish_reason: Option<String> = None;
            let mut usage = TokenUsage::default();

            let mut byte_stream = resp.bytes_stream();
            while let Some(chunk_result) = byte_stream.next().await {
                let chunk = chunk_result.map_err(|e| LlmError::Http(e.to_string()))?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                // Process complete lines
                while let Some(pos) = buffer.find('\n') {
                    let line = buffer[..pos].trim_end().to_string();
                    buffer = buffer[pos + 1..].to_string();

                    if line.is_empty() || line.starts_with(':') {
                        continue;
                    }

                    // SSE spec: the space after `data:` is optional. Some
                    // OpenAI-compat upstreams behind minifying CDNs (Cloudflare,
                    // Fastly) emit `data:{...}` with no space. Accept both.
                    let data = match line.strip_prefix("data:") {
                        Some(rest) => rest.strip_prefix(' ').unwrap_or(rest),
                        None => continue,
                    };

                    if data == "[DONE]" {
                        continue;
                    }

                    let json: serde_json::Value = match serde_json::from_str(data) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    // Some OpenAI-compat providers (DeepSeek, OpenRouter, Groq,
                    // local proxies, etc.) emit errors mid-stream as
                    // `data: {"error": {"message": ..., "type": ..., "code": ...}}`
                    // with HTTP 200. Without this branch the chunk has no
                    // `choices` field and we just `continue`d, ending up with
                    // empty content + zero usage — the truncation guard would
                    // then misclassify as Overloaded and the agent looped over
                    // a real, terminal error (auth, quota, content filter).
                    if let Some(err) = json.get("error") {
                        let err_type = err["type"].as_str().unwrap_or("unknown_error").to_string();
                        let err_code = err["code"].as_str().unwrap_or("").to_string();
                        let err_msg = err["message"]
                            .as_str()
                            .unwrap_or("(no message)")
                            .to_string();
                        warn!(
                            base_url = %self.base_url,
                            err_type = %err_type,
                            err_code = %err_code,
                            err_msg = %err_msg,
                            "Upstream sent error mid-stream as data event"
                        );
                        let combined = if err_code.is_empty() {
                            format!("{err_type}: {err_msg}")
                        } else {
                            format!("{err_type} ({err_code}): {err_msg}")
                        };
                        return Err(match err_type.as_str() {
                            "rate_limit_exceeded" | "rate_limit_error" => LlmError::RateLimited {
                                retry_after_ms: 5000,
                            },
                            "server_error" | "overloaded_error" | "service_unavailable" => {
                                LlmError::Overloaded {
                                    retry_after_ms: 5000,
                                }
                            }
                            "authentication_error" | "invalid_api_key" => LlmError::Api {
                                status: 401,
                                message: combined,
                            },
                            "permission_error" | "insufficient_quota" => LlmError::Api {
                                status: 403,
                                message: combined,
                            },
                            "not_found_error" | "model_not_found" => LlmError::Api {
                                status: 404,
                                message: combined,
                            },
                            "context_length_exceeded" | "request_too_large" => LlmError::Api {
                                status: 413,
                                message: combined,
                            },
                            "invalid_request_error" | "tool_use_failed" => LlmError::Api {
                                status: 400,
                                message: combined,
                            },
                            _ => LlmError::Api {
                                status: 502,
                                message: combined,
                            },
                        });
                    }

                    // Extract usage if present (some providers send it in the last chunk)
                    if let Some(u) = json.get("usage") {
                        if let Some(pt) = u["prompt_tokens"].as_u64() {
                            usage.input_tokens = pt;
                        }
                        if let Some(ct) = u["completion_tokens"].as_u64() {
                            usage.output_tokens = ct;
                        }
                    }

                    let choices = match json["choices"].as_array() {
                        Some(c) => c,
                        None => continue,
                    };

                    for choice in choices {
                        let delta = &choice["delta"];

                        // Reasoning content delta (DeepSeek R1, QwQ, etc.)
                        if let Some(reasoning) = delta["reasoning_content"].as_str() {
                            if !reasoning.is_empty() {
                                thinking_content.push_str(reasoning);
                                let _ = tx
                                    .send(StreamEvent::ThinkingDelta {
                                        text: reasoning.to_string(),
                                    })
                                    .await;
                            }
                        }

                        // Text content delta — with <think> tag tracking
                        if let Some(text) = delta["content"].as_str() {
                            if !text.is_empty() {
                                // Route content based on <think> tag state
                                let mut remaining = text;
                                while !remaining.is_empty() {
                                    if in_thinking {
                                        if let Some(end) = remaining.find("</think>") {
                                            let chunk = &remaining[..end];
                                            thinking_content.push_str(chunk);
                                            if !chunk.is_empty() {
                                                let _ = tx
                                                    .send(StreamEvent::ThinkingDelta {
                                                        text: chunk.to_string(),
                                                    })
                                                    .await;
                                            }
                                            remaining = &remaining[end + 8..];
                                            in_thinking = false;
                                        } else {
                                            thinking_content.push_str(remaining);
                                            let _ = tx
                                                .send(StreamEvent::ThinkingDelta {
                                                    text: remaining.to_string(),
                                                })
                                                .await;
                                            break;
                                        }
                                    } else if let Some(start) = remaining.find("<think>") {
                                        let before = &remaining[..start];
                                        if !before.is_empty() {
                                            text_content.push_str(before);
                                            let _ = tx
                                                .send(StreamEvent::TextDelta {
                                                    text: before.to_string(),
                                                })
                                                .await;
                                        }
                                        remaining = &remaining[start + 7..];
                                        in_thinking = true;
                                    } else {
                                        text_content.push_str(remaining);
                                        let _ = tx
                                            .send(StreamEvent::TextDelta {
                                                text: remaining.to_string(),
                                            })
                                            .await;
                                        break;
                                    }
                                }
                            }
                        }

                        // Tool call deltas
                        if let Some(calls) = delta["tool_calls"].as_array() {
                            for call in calls {
                                let idx = call["index"].as_u64().unwrap_or(0) as usize;

                                // Ensure tool_accum has enough entries
                                while tool_accum.len() <= idx {
                                    tool_accum.push((String::new(), String::new(), String::new()));
                                }

                                // ID (sent in first chunk for this tool)
                                if let Some(id) = call["id"].as_str() {
                                    tool_accum[idx].0 = id.to_string();
                                }

                                if let Some(func) = call.get("function") {
                                    // Name (sent in first chunk)
                                    if let Some(name) = func["name"].as_str() {
                                        tool_accum[idx].1 = name.to_string();
                                        let _ = tx
                                            .send(StreamEvent::ToolUseStart {
                                                id: tool_accum[idx].0.clone(),
                                                name: name.to_string(),
                                            })
                                            .await;
                                    }

                                    // Arguments delta
                                    if let Some(args) = func["arguments"].as_str() {
                                        tool_accum[idx].2.push_str(args);
                                        if !args.is_empty() {
                                            let _ = tx
                                                .send(StreamEvent::ToolInputDelta {
                                                    text: args.to_string(),
                                                })
                                                .await;
                                        }
                                    }
                                }
                            }
                        }

                        // Finish reason
                        if let Some(fr) = choice["finish_reason"].as_str() {
                            finish_reason = Some(fr.to_string());
                        }
                    }
                }
            }

            // Detect a truncated/empty SSE stream — same guard the
            // Anthropic driver got in v0.7.11. The OpenAI-compat
            // streaming pattern can also return HTTP 200 with zero
            // events when the upstream connection is dropped after
            // the headers, when the API silently closes the stream
            // for a quota/auth edge case, or when an OpenAI-compat
            // proxy is misconfigured. Without this guard the driver
            // returns `Ok(empty)` and the agent_loop's empty-response
            // guard fires with `input_tokens=0 output_tokens=0` —
            // the same silent-empty failure mode that bit Anthropic
            // users.
            if text_content.is_empty()
                && thinking_content.is_empty()
                && tool_accum.is_empty()
                && usage.input_tokens == 0
                && usage.output_tokens == 0
            {
                if attempt < max_retries {
                    let retry_ms = (attempt + 1) as u64 * 500;
                    warn!(
                        base_url = %self.base_url,
                        attempt,
                        retry_ms,
                        "OpenAI-compat stream returned no content and no usage; retrying"
                    );
                    tokio::time::sleep(std::time::Duration::from_millis(retry_ms)).await;
                    continue;
                }
                // Classify as Overloaded so agent_loop retries with the
                // longer 503-style backoff and surfaces the
                // user-friendly "AI provider is temporarily overloaded"
                // message. See the symmetric note in anthropic.rs.
                warn!(
                    base_url = %self.base_url,
                    "Stream returned no content + no usage after retries — \
                     surfacing as Overloaded so the higher-level retry kicks in"
                );
                return Err(LlmError::Overloaded {
                    retry_after_ms: 5000,
                });
            }

            // Build the final response
            let mut content = Vec::new();
            let mut tool_calls = Vec::new();

            if !thinking_content.is_empty() {
                content.push(ContentBlock::Thinking {
                    thinking: thinking_content,
                });
            }

            if !text_content.is_empty() {
                content.push(ContentBlock::Text { text: text_content });
            }

            for (id, name, arguments) in &tool_accum {
                let input: serde_json::Value =
                    serde_json::from_str(arguments).unwrap_or_else(|e| {
                        warn!(tool = %name, error = %e, "Failed to parse streamed tool call arguments");
                        serde_json::Value::Object(Default::default())
                    });
                content.push(ContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
                tool_calls.push(ToolCall {
                    id: id.clone(),
                    name: name.clone(),
                    input,
                });

                let _ = tx
                    .send(StreamEvent::ToolUseEnd {
                        id: id.clone(),
                        name: name.clone(),
                        input: serde_json::from_str(arguments)
                            .unwrap_or(serde_json::Value::Object(Default::default())),
                    })
                    .await;
            }

            let stop_reason = match finish_reason.as_deref() {
                Some("stop") => StopReason::EndTurn,
                Some("tool_calls") => StopReason::ToolUse,
                Some("length") => StopReason::MaxTokens,
                _ => {
                    if !tool_calls.is_empty() {
                        StopReason::ToolUse
                    } else {
                        StopReason::EndTurn
                    }
                }
            };

            let _ = tx
                .send(StreamEvent::ContentComplete { stop_reason, usage })
                .await;

            return Ok(CompletionResponse {
                content,
                stop_reason,
                tool_calls,
                usage,
            });
        }

        Err(LlmError::Api {
            status: 0,
            message: "Max retries exceeded".to_string(),
        })
    }
}

/// Parse Groq's `tool_use_failed` error and extract the tool call from `failed_generation`.
/// Extract the max_tokens limit from an API error message.
/// Looks for patterns like: `must be less than or equal to \`8192\``
fn extract_max_tokens_limit(body: &str) -> Option<u32> {
    // Pattern: "must be <= `N`" or "must be less than or equal to `N`"
    let patterns = [
        "less than or equal to `",
        "must be <= `",
        "maximum value for `max_tokens` is `",
    ];
    for pat in &patterns {
        if let Some(idx) = body.find(pat) {
            let after = &body[idx + pat.len()..];
            let end = after
                .find('`')
                .or_else(|| after.find('"'))
                .unwrap_or(after.len());
            if let Ok(n) = after[..end].trim().parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

///
/// Some models (e.g. Llama 3.3) generate tool calls as XML: `<function=NAME ARGS></function>`
/// instead of the proper JSON format. Groq rejects these with `tool_use_failed` but includes
/// the raw generation. We parse it and construct a proper CompletionResponse.
fn parse_groq_failed_tool_call(body: &str) -> Option<CompletionResponse> {
    let json_body: serde_json::Value = serde_json::from_str(body).ok()?;
    let failed = json_body
        .pointer("/error/failed_generation")
        .and_then(|v| v.as_str())?;

    // Parse all tool calls from the failed generation.
    // Format: <function=tool_name{"arg":"val"}></function> or <function=tool_name {"arg":"val"}></function>
    let mut tool_calls = Vec::new();
    let mut remaining = failed;

    while let Some(start) = remaining.find("<function=") {
        remaining = &remaining[start + 10..]; // skip "<function="
                                              // Find the end tag
        let end = remaining.find("</function>")?;
        let mut call_content = &remaining[..end];
        remaining = &remaining[end + 11..]; // skip "</function>"

        // Strip trailing ">" from the XML opening tag close
        call_content = call_content.strip_suffix('>').unwrap_or(call_content);

        // Split into name and args: "tool_name{"arg":"val"}" or "tool_name {"arg":"val"}"
        let (name, args) = if let Some(brace_pos) = call_content.find('{') {
            let name = call_content[..brace_pos].trim();
            let args = &call_content[brace_pos..];
            (name, args)
        } else {
            // No args — just a tool name
            (call_content.trim(), "{}")
        };

        // Parse args as JSON Value
        let args_value: serde_json::Value =
            serde_json::from_str(args).unwrap_or(serde_json::json!({}));

        tool_calls.push(ToolCall {
            id: format!("groq_recovered_{}", tool_calls.len()),
            name: name.to_string(),
            input: args_value,
        });
    }

    if tool_calls.is_empty() {
        // No tool calls found — the model generated plain text but Groq rejected it.
        // Return it as a normal text response instead of failing.
        if !failed.trim().is_empty() {
            warn!("Recovering plain text from Groq failed_generation (no tool calls)");
            return Some(CompletionResponse {
                content: vec![ContentBlock::Text {
                    text: failed.to_string(),
                }],
                tool_calls: vec![],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                },
            });
        }
        return None;
    }

    Some(CompletionResponse {
        content: vec![],
        tool_calls,
        stop_reason: StopReason::ToolUse,
        usage: TokenUsage {
            input_tokens: 0,
            output_tokens: 0,
        },
    })
}

/// Extract `<think>...</think>` blocks from text, returning (visible_text, thinking_text).
///
/// Handles multiple thinking blocks and nested content. Used for models like
/// MiniMax, QwQ, and others that embed reasoning in `<think>` XML tags within
/// the regular text response.
fn extract_think_tags(text: &str) -> (String, String) {
    let mut visible = String::new();
    let mut thinking = String::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("<think>") {
        // Everything before <think> is visible text
        visible.push_str(&remaining[..start]);
        remaining = &remaining[start + 7..]; // skip "<think>"

        // Find closing </think>
        if let Some(end) = remaining.find("</think>") {
            thinking.push_str(&remaining[..end]);
            remaining = &remaining[end + 8..]; // skip "</think>"
        } else {
            // Unclosed <think> — treat rest as thinking
            thinking.push_str(remaining);
            return (visible, thinking);
        }
    }

    // Remaining text after last </think>
    visible.push_str(remaining);
    (visible, thinking)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_driver_creation() {
        let driver = OpenAIDriver::new("test-key".to_string(), "http://localhost".to_string());
        assert_eq!(driver.api_key.as_str(), "test-key");
    }

    #[test]
    fn test_oai_message_serializes_reasoning_content() {
        // Regression: DeepSeek V4 reasoning models require that assistant
        // messages with tool_calls include the original `reasoning_content`
        // when echoed back. Prior to this fix, we dropped Thinking blocks on
        // the floor and DeepSeek returned HTTP 400
        // "The `reasoning_content` in the thinking mode must be passed back
        // to the API." on the second turn of a tool-use cycle.
        let msg = OaiMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(vec![OaiToolCall {
                id: "call_1".to_string(),
                call_type: "function".to_string(),
                function: OaiFunction {
                    name: "get_weather".to_string(),
                    arguments: "{\"city\":\"Paris\"}".to_string(),
                },
            }]),
            tool_call_id: None,
            reasoning_content: Some("Thinking about weather for Paris".to_string()),
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(
            json["reasoning_content"],
            serde_json::json!("Thinking about weather for Paris")
        );
        assert_eq!(json["role"], "assistant");
        assert_eq!(json["tool_calls"][0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_oai_message_omits_reasoning_content_when_none() {
        // Non-reasoning providers should not see a stray `reasoning_content`
        // field. `skip_serializing_if = Option::is_none` guards this.
        let msg = OaiMessage {
            role: "user".to_string(),
            content: Some(OaiMessageContent::Text("hello".to_string())),
            tool_calls: None,
            tool_call_id: None,
            reasoning_content: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert!(
            json.get("reasoning_content").is_none(),
            "reasoning_content must be omitted when None, got: {json}"
        );
    }

    #[test]
    fn test_parse_groq_failed_tool_call() {
        let body = r#"{"error":{"message":"Failed to call a function.","type":"invalid_request_error","code":"tool_use_failed","failed_generation":"<function=web_fetch{\"url\": \"https://example.com\"}></function>\n"}}"#;
        let result = parse_groq_failed_tool_call(body);
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].name, "web_fetch");
        assert!(resp.tool_calls[0]
            .input
            .to_string()
            .contains("https://example.com"));
    }

    #[test]
    fn test_parse_groq_failed_tool_call_with_space() {
        let body = r#"{"error":{"message":"Failed","type":"invalid_request_error","code":"tool_use_failed","failed_generation":"<function=shell_exec {\"command\": \"ls -la\"}></function>"}}"#;
        let result = parse_groq_failed_tool_call(body);
        assert!(result.is_some());
        let resp = result.unwrap();
        assert_eq!(resp.tool_calls[0].name, "shell_exec");
    }

    #[test]
    fn test_extract_think_tags_basic() {
        let (visible, thinking) =
            extract_think_tags("<think>Let me reason about this.</think>The answer is 42.");
        assert_eq!(thinking, "Let me reason about this.");
        assert_eq!(visible, "The answer is 42.");
    }

    #[test]
    fn test_extract_think_tags_no_tags() {
        let (visible, thinking) = extract_think_tags("Hello world");
        assert_eq!(visible, "Hello world");
        assert!(thinking.is_empty());
    }

    #[test]
    fn test_extract_think_tags_multiple() {
        let (visible, thinking) = extract_think_tags("<think>First</think>A<think>Second</think>B");
        assert_eq!(thinking, "FirstSecond");
        assert_eq!(visible, "AB");
    }

    #[test]
    fn test_extract_think_tags_unclosed() {
        let (visible, thinking) = extract_think_tags("Before<think>unclosed thinking");
        assert_eq!(visible, "Before");
        assert_eq!(thinking, "unclosed thinking");
    }

    #[test]
    fn test_extract_think_tags_empty_thinking() {
        let (visible, thinking) = extract_think_tags("<think></think>Result");
        assert!(thinking.is_empty());
        assert_eq!(visible, "Result");
    }

    /// Simulate exact MiniMax M2.7 non-streaming response with <think> tags.
    #[test]
    fn test_minimax_response_parsing_with_think() {
        let raw = r#"{
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "<think>\nLet me reason about this.\n</think>\n\nThe answer is 4.",
                    "role": "assistant"
                }
            }],
            "usage": {"prompt_tokens": 53, "completion_tokens": 326}
        }"#;

        let oai_response: OaiResponse = serde_json::from_str(raw).unwrap();
        let choice = &oai_response.choices[0];
        let text = choice.message.content.as_deref().unwrap();

        let (visible, thinking) = extract_think_tags(text);
        assert_eq!(thinking, "\nLet me reason about this.\n");
        assert_eq!(visible, "\n\nThe answer is 4.");
        assert!(!visible.trim().is_empty());
    }

    /// Simulate MiniMax M2.7 tool call response: <think> in content + tool_calls.
    #[test]
    fn test_minimax_tool_call_with_think() {
        let raw = r#"{
            "choices": [{
                "finish_reason": "tool_calls",
                "message": {
                    "content": "<think>The user wants weather data.</think>\n\n",
                    "role": "assistant",
                    "tool_calls": [{
                        "id": "call_function_abc123",
                        "type": "function",
                        "function": {
                            "name": "get_weather",
                            "arguments": "{\"city\":\"Tokyo\"}"
                        }
                    }]
                }
            }],
            "usage": {"prompt_tokens": 177, "completion_tokens": 55}
        }"#;

        let oai_response: OaiResponse = serde_json::from_str(raw).unwrap();
        let choice = &oai_response.choices[0];
        let text = choice.message.content.as_deref().unwrap();

        // Think tags should be extracted, leaving only whitespace
        let (visible, thinking) = extract_think_tags(text);
        assert_eq!(thinking, "The user wants weather data.");
        assert!(
            visible.trim().is_empty(),
            "Visible should be empty after trim, got: '{visible}'"
        );

        // Tool calls should parse correctly alongside thinking
        let tool_calls = choice.message.tool_calls.as_ref().unwrap();
        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].function.name, "get_weather");
    }

    /// Verify unknown MiniMax-specific fields don't break deserialization.
    #[test]
    fn test_minimax_extra_fields_ignored() {
        let raw = r#"{
            "choices": [{
                "finish_reason": "stop",
                "message": {
                    "content": "Hello!",
                    "role": "assistant",
                    "name": "MiniMax AI",
                    "audio_content": ""
                }
            }],
            "usage": {"prompt_tokens": 10, "completion_tokens": 5},
            "input_sensitive": false,
            "output_sensitive": false,
            "base_resp": {"status_code": 0, "status_msg": ""}
        }"#;

        let oai_response: OaiResponse = serde_json::from_str(raw).unwrap();
        assert_eq!(
            oai_response.choices[0].message.content.as_deref(),
            Some("Hello!")
        );
    }
}
