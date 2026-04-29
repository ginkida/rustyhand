//! End-to-end wire-level tests for the Kimi provider.
//!
//! These tests verify that `create_driver("kimi", ...)` routes through the
//! Anthropic driver with the correct URL construction, headers, and payload
//! shape. The tests spin up a minimal raw-TCP HTTP server that pretends to be
//! `api.kimi.com/coding`, captures the exact request the driver sends, and
//! returns a canned Anthropic-format response so the response-parsing path is
//! exercised too.
//!
//! No network, no external dependencies beyond what the crate already uses.

use rusty_hand_runtime::drivers::create_driver;
use rusty_hand_runtime::llm_driver::{CompletionRequest, DriverConfig};
use rusty_hand_types::message::Message;
use rusty_hand_types::tool::ToolDefinition;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

/// Captured request details from the mock server.
#[derive(Debug, Default, Clone)]
struct CapturedRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: String,
}

impl CapturedRequest {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// Spawn a one-shot mock server on localhost that captures the first request,
/// returns a canned Anthropic-format response, and records the request for
/// assertions. Returns `(base_url, captured)` where `base_url` can be fed
/// straight into `DriverConfig::base_url` and `captured` is filled once the
/// server thread completes.
fn spawn_mock_server(response_body: &'static str) -> (String, Arc<Mutex<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    let captured = Arc::new(Mutex::new(CapturedRequest::default()));
    let captured_clone = captured.clone();

    thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => return,
        };

        // Read until headers end. Using a fixed buffer is fine for test-sized
        // requests — agent completion bodies here are well under 16 KB.
        let mut buf = vec![0u8; 16 * 1024];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => return,
        };
        let raw = String::from_utf8_lossy(&buf[..n]).into_owned();

        let (head, body_prefix) = match raw.split_once("\r\n\r\n") {
            Some((h, b)) => (h.to_string(), b.to_string()),
            None => (raw, String::new()),
        };

        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or("").to_string();
        let mut parts = request_line.splitn(3, ' ');
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();

        let mut headers = Vec::new();
        let mut content_length: usize = 0;
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim().to_string();
                let v = v.trim().to_string();
                if k.eq_ignore_ascii_case("content-length") {
                    content_length = v.parse().unwrap_or(0);
                }
                headers.push((k, v));
            }
        }

        // If body arrived in pieces, read the rest.
        let mut body = body_prefix;
        while body.len() < content_length {
            let mut chunk = vec![0u8; 4096];
            match stream.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => body.push_str(&String::from_utf8_lossy(&chunk[..n])),
                Err(_) => break,
            }
        }

        {
            let mut slot = captured_clone.lock().unwrap();
            slot.method = method;
            slot.path = path;
            slot.headers = headers;
            slot.body = body;
        }

        // Canned response.
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    (base_url, captured)
}

const KIMI_CANNED_RESPONSE: &str = r#"{
    "id": "msg_test_kimi",
    "type": "message",
    "role": "assistant",
    "model": "kimi-k2-thinking",
    "content": [
        {"type": "text", "text": "Hello from Kimi K2 Thinking!"}
    ],
    "stop_reason": "end_turn",
    "stop_sequence": null,
    "usage": {"input_tokens": 42, "output_tokens": 7}
}"#;

#[tokio::test]
async fn kimi_driver_sends_anthropic_format_request() {
    let (base_url, captured) = spawn_mock_server(KIMI_CANNED_RESPONSE);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-test-key-12345".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("Hello, Kimi!")],
        tools: vec![],
        max_tokens: 64,
        temperature: 0.7,
        system: Some("You are a helpful coding assistant.".to_string()),
        thinking: None,
        response_format: Default::default(),
    };

    let response = driver
        .complete(request)
        .await
        .expect("complete() must succeed");

    // ── Wire-level assertions ─────────────────────────────────────────
    let req = captured.lock().unwrap().clone();

    assert_eq!(req.method, "POST", "Anthropic Messages API uses POST");
    assert_eq!(
        req.path, "/v1/messages",
        "AnthropicDriver must append /v1/messages to the base URL with no double slash"
    );
    assert_eq!(
        req.header("x-api-key"),
        Some("sk-kimi-test-key-12345"),
        "API key must be sent in x-api-key header (not Authorization: Bearer)"
    );
    assert_eq!(
        req.header("anthropic-version"),
        Some("2023-06-01"),
        "anthropic-version header is required by Kimi Code (Anthropic-compat)"
    );
    assert_eq!(
        req.header("content-type")
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("application/json"),
        "content-type must be application/json"
    );

    // ── Body shape (Anthropic Messages API) ───────────────────────────
    let body: serde_json::Value = serde_json::from_str(&req.body).expect("body must be JSON");
    assert_eq!(body["model"], "kimi-k2-thinking");
    assert_eq!(body["max_tokens"], 64);
    assert_eq!(body["system"], "You are a helpful coding assistant.");
    // `stream: false` is serialized as "omitted field" (skip_serializing_if = Not::not),
    // so it's null when absent — that's fine, as long as it isn't `true`.
    assert_ne!(body["stream"], serde_json::Value::Bool(true));
    assert!(body["messages"].is_array(), "messages must be an array");
    let messages = body["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");

    // ── Response parsing ──────────────────────────────────────────────
    assert_eq!(response.usage.input_tokens, 42);
    assert_eq!(response.usage.output_tokens, 7);
    assert!(
        response.content.iter().any(|c| matches!(
            c,
            rusty_hand_types::message::ContentBlock::Text { text } if text.contains("Kimi K2 Thinking")
        )),
        "response text must round-trip through AnthropicDriver parser"
    );
}

#[tokio::test]
async fn kimi_driver_forwards_tools_in_anthropic_shape() {
    let (base_url, captured) = spawn_mock_server(KIMI_CANNED_RESPONSE);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("Search for rust async")],
        tools: vec![ToolDefinition {
            name: "web_search".to_string(),
            description: "Search the web".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
        }],
        max_tokens: 32,
        temperature: 0.0,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let _ = driver
        .complete(request)
        .await
        .expect("tool-call request must succeed");

    let req = captured.lock().unwrap().clone();
    let body: serde_json::Value = serde_json::from_str(&req.body).expect("json body");

    // Anthropic tool shape: [{name, description, input_schema}, ...] with tool_choice=auto
    let tools = body["tools"].as_array().expect("tools must be an array");
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["name"], "web_search");
    assert!(tools[0]["input_schema"].is_object());
    assert_eq!(
        body["tool_choice"]["type"], "auto",
        "AnthropicDriver must send tool_choice=auto when tools are present"
    );
}

/// Minimal SSE response body in Anthropic's streaming event format.
/// Covers the main event types the streaming parser reads (message_start,
/// content_block_start/delta/stop, message_delta, message_stop).
///
/// Uses `\n\n` as the event separator (SSE spec requires \n\n; the anthropic.rs
/// parser splits on `\n\n`). The HTTP header framing still uses `\r\n`.
const KIMI_SSE_RESPONSE: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_k\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"kimi-k2-thinking\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":10,\"output_tokens\":0}}}\n\nevent: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\nevent: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Streaming from Kimi\"}}\n\nevent: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\nevent: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":5}}\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}\n\n";

/// Spawn a mock server that replies with an SSE stream (text/event-stream) so
/// the AnthropicDriver's streaming path gets exercised against a Kimi-shaped
/// base URL. Returns (base_url, captured).
fn spawn_mock_sse_server() -> (String, Arc<Mutex<CapturedRequest>>) {
    spawn_mock_sse_server_with_body(KIMI_SSE_RESPONSE)
}

/// Variant that lets callers control the SSE body so we can exercise edge
/// cases like in-stream `event: error` payloads.
fn spawn_mock_sse_server_with_body(body: &'static str) -> (String, Arc<Mutex<CapturedRequest>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock sse server");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    let captured = Arc::new(Mutex::new(CapturedRequest::default()));
    let captured_clone = captured.clone();

    thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => return,
        };
        let mut buf = vec![0u8; 16 * 1024];
        let n = match stream.read(&mut buf) {
            Ok(n) => n,
            Err(_) => return,
        };
        let raw = String::from_utf8_lossy(&buf[..n]).into_owned();
        let (head, _) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
        let mut lines = head.lines();
        let request_line = lines.next().unwrap_or("").to_string();
        let mut parts = request_line.splitn(3, ' ');
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        let mut headers = Vec::new();
        for line in lines {
            if let Some((k, v)) = line.split_once(':') {
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
        {
            let mut slot = captured_clone.lock().unwrap();
            slot.method = method;
            slot.path = path;
            slot.headers = headers;
        }

        // Respond with text/event-stream (chunked would be more realistic, but the
        // AnthropicDriver accepts Content-Length too since reqwest handles both).
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    (base_url, captured)
}

#[tokio::test]
async fn kimi_driver_streams_anthropic_sse_format() {
    let (base_url, captured) = spawn_mock_sse_server();

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-stream-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("Stream hello")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.5,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let final_response = driver
        .stream(request, tx)
        .await
        .expect("stream() must succeed against Kimi-shaped endpoint");

    // Drain events and collect text deltas — prove the SSE parser ran.
    let mut text_deltas = Vec::new();
    while let Ok(evt) = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
    {
        match evt {
            Some(rusty_hand_runtime::llm_driver::StreamEvent::TextDelta { text }) => {
                text_deltas.push(text);
            }
            Some(_) => {}
            None => break,
        }
    }

    let req = captured.lock().unwrap().clone();
    assert_eq!(req.path, "/v1/messages");
    assert_eq!(req.header("x-api-key"), Some("sk-kimi-stream-test"));
    assert_eq!(req.header("anthropic-version"), Some("2023-06-01"));

    assert_eq!(
        text_deltas.join(""),
        "Streaming from Kimi",
        "SSE text_delta events must flow through to the caller"
    );
    assert_eq!(final_response.usage.input_tokens, 10);
    assert_eq!(final_response.usage.output_tokens, 5);
}

#[tokio::test]
async fn kimi_driver_rejects_missing_api_key() {
    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: None,
        // No base_url — if the env var isn't set either, this should fail at driver construction.
        base_url: None,
    };

    // Only assert when KIMI_API_KEY is NOT set in the environment; otherwise
    // env var satisfies the requirement and the test is inconclusive.
    if std::env::var("KIMI_API_KEY").is_err() {
        let result = create_driver(&config);
        assert!(
            matches!(
                result,
                Err(rusty_hand_runtime::llm_driver::LlmError::MissingApiKey(_))
            ),
            "kimi provider with no key must surface a MissingApiKey error"
        );
    }
}

// ── In-stream error event handling ────────────────────────────────────────
// These tests exercise the path where the upstream sends HTTP 200 + SSE
// `event: error` mid-stream (the path that v0.7.13 silently swallowed,
// causing 6 redundant retries before giving up with a generic message).
// The fix surfaces these as the matching `LlmError::*` variant immediately,
// and the agent_loop's existing handler reports the upstream's actual
// error message instead of a hardcoded "overloaded" string.

/// Minified SSE format (`event:foo` / `data:{...}` — NO space after colon).
/// This is the exact wire format Kimi (api.kimi.com/coding) emits via its
/// Cloudflare front. Pre-fix the SSE parser required `"event: "` / `"data: "`
/// (with space) and silently dropped every event from this upstream, which
/// is why a fresh Docker install with KIMI_API_KEY produced the "agent
/// doesn't reply on Telegram" symptom.
const KIMI_SSE_MINIFIED_OK: &str = "event:message_start\ndata:{\"type\":\"message_start\",\"message\":{\"id\":\"msg_m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"kimi-for-coding\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":7,\"output_tokens\":0}}}\n\nevent:content_block_start\ndata:{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\nevent:content_block_delta\ndata:{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Pong!\"}}\n\nevent:content_block_stop\ndata:{\"type\":\"content_block_stop\",\"index\":0}\n\nevent:message_delta\ndata:{\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":3}}\n\nevent:message_stop\ndata:{\"type\":\"message_stop\"}\n\n";

const KIMI_SSE_OVERLOADED: &str = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_e\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"kimi-k2-thinking\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n\nevent: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Upstream is overloaded right now\"}}\n\n";

const KIMI_SSE_AUTH_FAILED: &str = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"authentication_error\",\"message\":\"invalid api key\"}}\n\n";

const KIMI_SSE_INVALID_REQ: &str = "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"message\":\"max_tokens too large\"}}\n\n";

/// Regression: the Kimi production endpoint emits SSE *without* the space
/// after `event:` / `data:`. Pre-fix our parser required the space and
/// dropped every event silently — this test pins the lenient parsing.
#[tokio::test]
async fn kimi_driver_parses_minified_sse_format_without_space_after_colon() {
    let (base_url, _captured) = spawn_mock_sse_server_with_body(KIMI_SSE_MINIFIED_OK);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-mini-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let request = CompletionRequest {
        model: "kimi-for-coding".to_string(),
        messages: vec![Message::user("ping")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.0,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let final_response = driver
        .stream(request, tx)
        .await
        .expect("stream() must parse the minified SSE format Kimi sends");

    let mut text_deltas = Vec::new();
    while let Ok(evt) = tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
    {
        match evt {
            Some(rusty_hand_runtime::llm_driver::StreamEvent::TextDelta { text }) => {
                text_deltas.push(text);
            }
            Some(_) => {}
            None => break,
        }
    }

    assert_eq!(
        text_deltas.join(""),
        "Pong!",
        "minified SSE (no space after colon) must produce text deltas — \
         this is the exact wire format the live Kimi API emits"
    );
    assert_eq!(final_response.usage.input_tokens, 7);
    assert_eq!(final_response.usage.output_tokens, 3);
}

#[tokio::test]
async fn kimi_driver_sse_error_event_overloaded_returns_overloaded() {
    let (base_url, _captured) = spawn_mock_sse_server_with_body(KIMI_SSE_OVERLOADED);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-err-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.5,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let err = driver
        .stream(request, tx)
        .await
        .expect_err("SSE error event must surface as LlmError, not Ok");

    assert!(
        matches!(
            err,
            rusty_hand_runtime::llm_driver::LlmError::Overloaded { .. }
        ),
        "overloaded_error type must map to LlmError::Overloaded, got: {err:?}"
    );
}

#[tokio::test]
async fn kimi_driver_sse_error_event_auth_returns_api_401() {
    let (base_url, _captured) = spawn_mock_sse_server_with_body(KIMI_SSE_AUTH_FAILED);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-err-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.5,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let err = driver
        .stream(request, tx)
        .await
        .expect_err("auth error event must surface as LlmError, not Ok");

    match err {
        rusty_hand_runtime::llm_driver::LlmError::Api { status, message } => {
            assert_eq!(status, 401, "auth error must map to HTTP 401");
            assert!(
                message.contains("invalid api key"),
                "user-facing message must include upstream detail, got: {message}"
            );
        }
        other => panic!("expected LlmError::Api(401), got {other:?}"),
    }
}

#[tokio::test]
async fn kimi_driver_sse_error_event_invalid_request_returns_api_400() {
    let (base_url, _captured) = spawn_mock_sse_server_with_body(KIMI_SSE_INVALID_REQ);

    let config = DriverConfig {
        provider: "kimi".to_string(),
        api_key: Some("sk-kimi-err-test".to_string()),
        base_url: Some(base_url),
    };
    let driver = create_driver(&config).expect("kimi driver must build");

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let request = CompletionRequest {
        model: "kimi-k2-thinking".to_string(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 99999,
        temperature: 0.5,
        system: None,
        thinking: None,
        response_format: Default::default(),
    };

    let err = driver
        .stream(request, tx)
        .await
        .expect_err("invalid_request error event must surface as LlmError");

    match err {
        rusty_hand_runtime::llm_driver::LlmError::Api { status, message } => {
            assert_eq!(status, 400);
            assert!(
                message.contains("max_tokens too large"),
                "user must see upstream's exact reason, got: {message}"
            );
        }
        other => panic!("expected LlmError::Api(400), got {other:?}"),
    }
}
