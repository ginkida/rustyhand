//! Cross-driver SSE wire-format conformance tests.
//!
//! Both `AnthropicDriver` and `OpenAIDriver` consume Server-Sent Events
//! from third-party providers. The SSE spec (https://html.spec.whatwg.org/
//! multipage/server-sent-events.html#parsing-an-event-stream) treats the
//! single space after a field colon as OPTIONAL — `event:foo` and
//! `event: foo` are both valid, and conformant parsers must accept either.
//!
//! Pre-v0.7.15 our parsers strict-matched the form *with* the space, so
//! every event from minifying upstreams (Kimi behind Cloudflare; some
//! OpenAI-compat proxies behind Fastly) was silently dropped. The
//! single-test-per-driver coverage that came in with the v0.7.15 fix is
//! good — but it doesn't make the conformance contract obvious to a
//! future reader. This file pins it explicitly:
//!
//!   matrix = (driver) × (with-space, without-space, mixed)
//!   for every pair: text content + token usage must round-trip cleanly.
//!
//! When a third driver lands (or when someone touches the SSE parser
//! again), this is the file that should grow a column / row, not a
//! one-off integration test.

use rusty_hand_runtime::drivers::create_driver;
use rusty_hand_runtime::llm_driver::{CompletionRequest, DriverConfig, LlmDriver, StreamEvent};
use rusty_hand_types::message::Message;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

// ── Helpers ───────────────────────────────────────────────────────────────

/// Spawn a one-shot HTTP server that returns the given SSE body on the
/// first incoming request. The body is sent verbatim — caller controls
/// the wire format down to whitespace.
fn spawn_sse_server(body: &'static str) -> String {
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

fn make_driver(provider: &str, base_url: String) -> Arc<dyn LlmDriver> {
    let config = DriverConfig {
        provider: provider.to_string(),
        api_key: Some("sk-conformance-test".to_string()),
        base_url: Some(base_url),
    };
    create_driver(&config).expect("driver must build")
}

fn make_request(model: &str) -> CompletionRequest {
    CompletionRequest {
        model: model.to_string(),
        messages: vec![Message::user("ping")],
        tools: vec![],
        max_tokens: 32,
        temperature: 0.0,
        system: None,
        thinking: None,
        response_format: Default::default(),
    }
}

/// Drain text deltas from the stream rx until the channel closes. Used
/// to prove that the parser actually emitted events to the caller, not
/// just accumulated silently.
async fn drain_text(mut rx: tokio::sync::mpsc::Receiver<StreamEvent>) -> String {
    let mut text = String::new();
    while let Ok(Some(evt)) =
        tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await
    {
        if let StreamEvent::TextDelta { text: t } = evt {
            text.push_str(&t);
        }
    }
    text
}

// ── Anthropic-shaped wire format (used by anthropic + kimi providers) ──

/// Canonical SSE format from api.anthropic.com — every field has a
/// space after the colon. This is the "happy path" wire format.
const ANTHROPIC_WITH_SPACE: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_a\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4-20250514\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello world\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n";

/// Minified SSE format from api.kimi.com/coding (Cloudflare in front
/// strips the optional space). Same payload as ANTHROPIC_WITH_SPACE,
/// no space after `event:` / `data:`.
const ANTHROPIC_WITHOUT_SPACE: &str = "event:message_start\n\
data:{\"type\":\"message_start\",\"message\":{\"id\":\"msg_a\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"kimi-for-coding\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event:content_block_start\n\
data:{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event:content_block_delta\n\
data:{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello world\"}}\n\n\
event:content_block_stop\n\
data:{\"type\":\"content_block_stop\",\"index\":0}\n\n\
event:message_delta\n\
data:{\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n\n\
event:message_stop\n\
data:{\"type\":\"message_stop\"}\n\n";

/// Mixed wire format — some lines with the space, some without. Real
/// CDNs sometimes strip whitespace inconsistently across chunks (e.g.
/// during proxy-level buffering), so the parser must handle the
/// transition mid-stream.
const ANTHROPIC_MIXED: &str = "event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_a\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"claude-sonnet-4\",\"content\":[],\"stop_reason\":null,\"usage\":{\"input_tokens\":8,\"output_tokens\":0}}}\n\n\
event:content_block_start\n\
data:{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hello world\"}}\n\n\
event:content_block_stop\n\
data:{\"type\":\"content_block_stop\",\"index\":0}\n\n\
event:message_delta\n\
data:{\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\",\"stop_sequence\":null},\"usage\":{\"output_tokens\":2}}\n\n";

#[tokio::test]
async fn anthropic_driver_with_space_canonical_format() {
    let base_url = spawn_sse_server(ANTHROPIC_WITH_SPACE);
    let drv = make_driver("anthropic", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("claude-sonnet-4-20250514"), tx)
        .await
        .expect("canonical with-space SSE must parse");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 8);
    assert_eq!(resp.usage.output_tokens, 2);
}

#[tokio::test]
async fn anthropic_driver_without_space_kimi_cloudflare_format() {
    let base_url = spawn_sse_server(ANTHROPIC_WITHOUT_SPACE);
    let drv = make_driver("kimi", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("kimi-for-coding"), tx)
        .await
        .expect("minified without-space SSE must parse — this is the v0.7.15 root cause fix");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 8);
    assert_eq!(resp.usage.output_tokens, 2);
}

#[tokio::test]
async fn anthropic_driver_mixed_format_partial_minification() {
    let base_url = spawn_sse_server(ANTHROPIC_MIXED);
    let drv = make_driver("anthropic", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("claude-sonnet-4"), tx)
        .await
        .expect("mixed-format SSE must parse — defensive against intermittent CDN behavior");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 8);
    assert_eq!(resp.usage.output_tokens, 2);
}

// ── OpenAI-shaped wire format (used by deepseek, minimax, zhipu, etc.) ──

/// Canonical OpenAI streaming format — `data: {...}` per chunk plus
/// `data: [DONE]` terminator. Every line has the space after `data:`.
const OPENAI_WITH_SPACE: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"hello \"},\"index\":0}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n\
data: [DONE]\n";

/// Minified OpenAI streaming — same payload, no space after `data:`.
const OPENAI_WITHOUT_SPACE: &str = "data:{\"choices\":[{\"delta\":{\"content\":\"hello \"},\"index\":0}]}\n\
data:{\"choices\":[{\"delta\":{\"content\":\"world\"},\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n\
data:[DONE]\n";

/// Mixed: first chunk minified, second chunk canonical, [DONE] minified.
const OPENAI_MIXED: &str = "data:{\"choices\":[{\"delta\":{\"content\":\"hello \"},\"index\":0}]}\n\
data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"index\":0,\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":2}}\n\
data:[DONE]\n";

#[tokio::test]
async fn openai_driver_with_space_canonical_format() {
    let base_url = spawn_sse_server(OPENAI_WITH_SPACE);
    let drv = make_driver("deepseek", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("deepseek-v4-flash"), tx)
        .await
        .expect("canonical with-space OpenAI SSE must parse");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 7);
    assert_eq!(resp.usage.output_tokens, 2);
}

#[tokio::test]
async fn openai_driver_without_space_minified_cdn_format() {
    let base_url = spawn_sse_server(OPENAI_WITHOUT_SPACE);
    let drv = make_driver("deepseek", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("deepseek-v4-flash"), tx)
        .await
        .expect("minified without-space OpenAI SSE must parse");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 7);
    assert_eq!(resp.usage.output_tokens, 2);
}

#[tokio::test]
async fn openai_driver_mixed_format_partial_minification() {
    let base_url = spawn_sse_server(OPENAI_MIXED);
    let drv = make_driver("deepseek", base_url);

    let (tx, rx) = tokio::sync::mpsc::channel(64);
    let resp = drv
        .stream(make_request("deepseek-v4-flash"), tx)
        .await
        .expect("mixed-format OpenAI SSE must parse");

    assert_eq!(drain_text(rx).await, "hello world");
    assert_eq!(resp.usage.input_tokens, 7);
    assert_eq!(resp.usage.output_tokens, 2);
}
