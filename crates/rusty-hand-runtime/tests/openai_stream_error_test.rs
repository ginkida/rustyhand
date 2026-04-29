//! Wire-level tests for OpenAI-compatible streaming error handling.
//!
//! Many OpenAI-compatible providers (DeepSeek, Groq, OpenRouter, MiniMax,
//! self-hosted vLLM/llama.cpp proxies) emit errors mid-stream as a single
//! SSE chunk shaped like `data: {"error": {...}}` with HTTP 200. Pre-fix
//! the driver silently skipped those chunks (no `choices` field → continue),
//! reported empty content + zero usage, and the agent loop spent six
//! retries on a terminal error.
//!
//! These tests pin the new behavior: the upstream's actual error type,
//! code, and message must be propagated through `LlmError` so the user
//! sees what's wrong instead of a generic "model overloaded" message.

use rusty_hand_runtime::drivers::create_driver;
use rusty_hand_runtime::llm_driver::{CompletionRequest, DriverConfig, LlmError};
use rusty_hand_types::message::Message;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

/// Spawn a one-shot mock SSE server that returns the given `data:` payload
/// followed by `[DONE]`. Returns the base URL.
fn spawn_mock_sse_server(payload: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock sse server");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => return,
        };
        let mut buf = vec![0u8; 16 * 1024];
        let _ = stream.read(&mut buf);

        let body = format!("data: {payload}\n\ndata: [DONE]\n\n");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    base_url
}

fn driver(base_url: String) -> Arc<dyn rusty_hand_runtime::llm_driver::LlmDriver> {
    let config = DriverConfig {
        provider: "openai-compat".to_string(),
        api_key: Some("sk-test".to_string()),
        base_url: Some(base_url),
    };
    create_driver(&config).expect("driver must build")
}

fn request() -> CompletionRequest {
    CompletionRequest {
        model: "gpt-4o-mini".to_string(),
        messages: vec![Message::user("hi")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.5,
        system: None,
        thinking: None,
        response_format: Default::default(),
    }
}

#[tokio::test]
async fn openai_stream_error_rate_limit_returns_rate_limited() {
    let payload = r#"{"error":{"message":"You hit the rate limit","type":"rate_limit_exceeded","code":"rate_limit_exceeded"}}"#;
    let base_url = spawn_mock_sse_server(payload);
    let drv = driver(base_url);

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let err = drv
        .stream(request(), tx)
        .await
        .expect_err("rate-limit error in stream must surface as LlmError");

    assert!(
        matches!(err, LlmError::RateLimited { .. }),
        "rate_limit_exceeded must map to LlmError::RateLimited, got: {err:?}"
    );
}

#[tokio::test]
async fn openai_stream_error_server_error_returns_overloaded() {
    let payload =
        r#"{"error":{"message":"upstream is at capacity","type":"server_error","code":"503"}}"#;
    let base_url = spawn_mock_sse_server(payload);
    let drv = driver(base_url);

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let err = drv.stream(request(), tx).await.expect_err("must error");

    assert!(
        matches!(err, LlmError::Overloaded { .. }),
        "server_error must map to LlmError::Overloaded, got: {err:?}"
    );
}

#[tokio::test]
async fn openai_stream_error_auth_returns_api_401_with_message() {
    let payload = r#"{"error":{"message":"Invalid API key provided","type":"invalid_api_key","code":"invalid_api_key"}}"#;
    let base_url = spawn_mock_sse_server(payload);
    let drv = driver(base_url);

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let err = drv.stream(request(), tx).await.expect_err("must error");

    match err {
        LlmError::Api { status, message } => {
            assert_eq!(status, 401);
            assert!(
                message.contains("Invalid API key"),
                "user must see upstream's exact reason, got: {message}"
            );
        }
        other => panic!("expected Api(401), got {other:?}"),
    }
}

#[tokio::test]
async fn openai_stream_error_quota_returns_api_403_with_message() {
    let payload = r#"{"error":{"message":"Free tier quota exhausted","type":"insufficient_quota","code":"quota_exceeded"}}"#;
    let base_url = spawn_mock_sse_server(payload);
    let drv = driver(base_url);

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let err = drv.stream(request(), tx).await.expect_err("must error");

    match err {
        LlmError::Api { status, message } => {
            assert_eq!(status, 403);
            assert!(
                message.contains("Free tier quota exhausted"),
                "user must see upstream's exact reason, got: {message}"
            );
        }
        other => panic!("expected Api(403), got {other:?}"),
    }
}

/// Regression: SSE spec allows `data:value` without the space after the
/// colon (the space is optional). Some OpenAI-compat upstreams behind
/// minifying CDNs emit this format. Pre-fix our parser strict-matched
/// `"data: "` (with space) and silently skipped every chunk.
#[tokio::test]
async fn openai_stream_parses_minified_data_lines() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    thread::spawn(move || {
        let (mut stream, _) = match listener.accept() {
            Ok(x) => x,
            Err(_) => return,
        };
        let mut buf = vec![0u8; 16 * 1024];
        let _ = stream.read(&mut buf);

        // No space after `data:` — this is what minifying CDNs emit.
        let body = "data:{\"choices\":[{\"delta\":{\"content\":\"Hi\"},\"index\":0}]}\n\
                    data:{\"choices\":[{\"delta\":{\"content\":\" there\"},\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":5,\"completion_tokens\":2}}\n\
                    data:[DONE]\n";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    });

    let drv = driver(base_url);
    let (tx, mut rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(request(), tx)
        .await
        .expect("minified data: must parse");

    let mut got = String::new();
    while let Ok(Some(evt)) =
        tokio::time::timeout(std::time::Duration::from_millis(50), rx.recv()).await
    {
        if let rusty_hand_runtime::llm_driver::StreamEvent::TextDelta { text } = evt {
            got.push_str(&text);
        }
    }

    assert_eq!(got, "Hi there", "minified SSE must produce text content");
    assert_eq!(resp.usage.output_tokens, 2);
}

#[tokio::test]
async fn openai_stream_error_unknown_type_falls_through_with_detail() {
    // Unknown error types still surface the upstream message verbatim instead
    // of being silently dropped.
    let payload = r#"{"error":{"message":"Stack overflow in tokenizer","type":"weird_provider_specific_error"}}"#;
    let base_url = spawn_mock_sse_server(payload);
    let drv = driver(base_url);

    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let err = drv.stream(request(), tx).await.expect_err("must error");

    match err {
        LlmError::Api { status, message } => {
            assert_eq!(status, 502, "unknown errors map to bad gateway by default");
            assert!(
                message.contains("Stack overflow in tokenizer"),
                "upstream message must propagate even for unknown types, got: {message}"
            );
        }
        other => panic!("expected Api(502), got {other:?}"),
    }
}
