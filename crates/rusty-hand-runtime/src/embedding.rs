//! Embedding driver for vector-based semantic memory.
//!
//! Provides an `EmbeddingDriver` trait and an OpenAI-compatible implementation
//! that works with any provider offering a `/v1/embeddings` endpoint. Built-in
//! targets: Ollama (local), Voyage (specialized), and any OpenAI-compatible
//! endpoint via an explicit `base_url`.

use async_trait::async_trait;
use rusty_hand_types::model_catalog::{
    OLLAMA_BASE_URL, OPENAI_EMBEDDING_BASE_URL, VOYAGE_BASE_URL,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};
use zeroize::Zeroizing;

/// How long the driver short-circuits embed() calls after a failure.
/// On a fresh install where Ollama isn't running, this prevents every
/// agent message from spamming an `error sending request` warning and
/// from issuing a real HTTP attempt that's certain to fail.
const FAILURE_COOLDOWN: Duration = Duration::from_secs(30);

/// Error type for embedding operations.
#[derive(Debug, thiserror::Error)]
pub enum EmbeddingError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Missing API key: {0}")]
    MissingApiKey(String),
    /// Last call failed and we're still within the cooldown window —
    /// no HTTP attempt was made. Callers should treat this as a normal
    /// "embedding unavailable, fall back to text search" signal and
    /// log at `debug!` level only.
    #[error("Embedding endpoint unavailable (last error: {0})")]
    Unavailable(String),
}

/// Configuration for creating an embedding driver.
#[derive(Debug, Clone)]
pub struct EmbeddingConfig {
    /// Provider name (openai, groq, together, ollama, etc.).
    pub provider: String,
    /// Model name (e.g., "text-embedding-3-small", "all-MiniLM-L6-v2").
    pub model: String,
    /// API key (resolved from env var).
    pub api_key: String,
    /// Base URL for the API.
    pub base_url: String,
}

/// Trait for computing text embeddings.
#[async_trait]
pub trait EmbeddingDriver: Send + Sync {
    /// Compute embedding vectors for a batch of texts.
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError>;

    /// Compute embedding for a single text.
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbeddingError> {
        let results = self.embed(&[text]).await?;
        results
            .into_iter()
            .next()
            .ok_or_else(|| EmbeddingError::Parse("Empty embedding response".to_string()))
    }

    /// Return the dimensionality of embeddings produced by this driver.
    fn dimensions(&self) -> usize;
}

/// State machine for the per-driver failure circuit breaker.
struct CircuitState {
    /// Embed() short-circuits to `Unavailable` until this instant.
    open_until: Instant,
    /// Last error message, returned in the short-circuited error.
    last_error: String,
    /// Whether we already logged the WARN line for this outage. Cleared
    /// on first successful call.
    warned: bool,
}

/// OpenAI-compatible embedding driver.
///
/// Works with any provider that implements the `/v1/embeddings` endpoint:
/// OpenAI, Groq, Together, Fireworks, Ollama, vLLM, LM Studio, etc.
pub struct OpenAIEmbeddingDriver {
    api_key: Zeroizing<String>,
    base_url: String,
    model: String,
    client: reqwest::Client,
    dims: usize,
    /// Failure circuit breaker — see `FAILURE_COOLDOWN`.
    circuit: Mutex<Option<CircuitState>>,
}

#[derive(Serialize)]
struct EmbedRequest<'a> {
    model: &'a str,
    input: &'a [&'a str],
}

#[derive(Deserialize)]
struct EmbedResponse {
    data: Vec<EmbedData>,
}

#[derive(Deserialize)]
struct EmbedData {
    embedding: Vec<f32>,
    /// Position of this embedding in the input batch. OpenAI-compatible
    /// endpoints carry it precisely so consumers don't rely on array order.
    ///
    /// `Option<i64>` rather than `usize` on purpose: `#[serde(default)]` only
    /// applies when the key is ABSENT, so a provider that sends `"index": null`
    /// (or a float, or -1) would fail to deserialize the ENTIRE response and
    /// break embedding for that endpoint outright — where before this field
    /// existed the value was simply ignored. Tolerate anything unusable and
    /// fall back to array order.
    #[serde(default, deserialize_with = "deserialize_opt_index")]
    index: Option<i64>,
}

/// Deserialize `index` leniently: any value that isn't a non-negative integer
/// becomes `None` so a single odd field can't fail the whole response.
fn deserialize_opt_index<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = serde_json::Value::deserialize(deserializer)?;
    Ok(v.as_i64().filter(|i| *i >= 0))
}

/// Return the embeddings in INPUT order, using each item's `index` when the set
/// of indices is trustworthy.
///
/// Placement rather than `sort_by_key`: a sort silently accepts duplicate or
/// out-of-range indices and still produces a full-length vector, so a
/// non-conformant provider would yield a confidently wrong chunk↔vector pairing.
/// Here the indices must form an exact permutation of `0..expected` before they
/// are trusted; anything else (missing, duplicated, out of range, wrong count)
/// falls back to array order, which is what every provider that omits `index`
/// relies on anyway.
fn order_embeddings(items: Vec<EmbedData>, expected: usize) -> Vec<Vec<f32>> {
    let usable = items.len() == expected && items.iter().all(|d| d.index.is_some());
    if usable {
        let mut slots: Vec<Option<Vec<f32>>> = (0..expected).map(|_| None).collect();
        let mut ok = true;
        for item in &items {
            match item.index.and_then(|i| usize::try_from(i).ok()) {
                Some(i) if i < expected && slots[i].is_none() => {
                    slots[i] = Some(item.embedding.clone());
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            // Every slot is filled: `expected` distinct in-range indices over
            // `expected` slots leaves no gaps.
            return slots.into_iter().flatten().collect();
        }
        warn!(
            expected,
            "Embedding response indices are not a permutation of 0..n — \
             falling back to array order"
        );
    }
    items.into_iter().map(|d| d.embedding).collect()
}

impl OpenAIEmbeddingDriver {
    /// Create a new OpenAI-compatible embedding driver.
    pub fn new(config: EmbeddingConfig) -> Result<Self, EmbeddingError> {
        // Infer dimensions from model name (common models)
        let dims = infer_dimensions(&config.model);

        Ok(Self {
            api_key: Zeroizing::new(config.api_key),
            base_url: config.base_url,
            model: config.model,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap_or_default(),
            dims,
            circuit: Mutex::new(None),
        })
    }

    /// If the circuit is currently open, return the cached `Unavailable`
    /// error without making an HTTP call.
    fn check_circuit(&self) -> Option<EmbeddingError> {
        let guard = self.circuit.lock().ok()?;
        let state = guard.as_ref()?;
        if Instant::now() < state.open_until {
            Some(EmbeddingError::Unavailable(state.last_error.clone()))
        } else {
            None
        }
    }

    /// Trip the circuit breaker. Returns `true` on the first failure of
    /// an outage so the caller can emit a single WARN; subsequent calls
    /// inside the cooldown window return `false` for `debug!`-level
    /// logging.
    fn record_failure(&self, msg: String) -> bool {
        let mut guard = match self.circuit.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let already_warned = guard.as_ref().map(|s| s.warned).unwrap_or(false);
        *guard = Some(CircuitState {
            open_until: Instant::now() + FAILURE_COOLDOWN,
            last_error: msg,
            warned: true,
        });
        !already_warned
    }

    /// Reset the circuit on a successful call. If we previously warned
    /// the user about an outage, log the recovery once.
    fn record_success(&self) {
        let mut guard = match self.circuit.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(state) = guard.take() {
            if state.warned {
                info!(
                    base_url = %self.base_url,
                    "Embedding endpoint recovered"
                );
            }
        }
    }
}

/// Infer embedding dimensions from model name.
fn infer_dimensions(model: &str) -> usize {
    match model {
        // OpenAI
        "text-embedding-3-small" => 1536,
        "text-embedding-3-large" => 3072,
        "text-embedding-ada-002" => 1536,
        // Sentence Transformers / local models
        "all-MiniLM-L6-v2" => 384,
        "all-MiniLM-L12-v2" => 384,
        "all-mpnet-base-v2" => 768,
        "nomic-embed-text" => 768,
        "mxbai-embed-large" => 1024,
        // Voyage AI
        "voyage-3" => 1024,
        "voyage-3-lite" => 512,
        "voyage-code-3" => 1024,
        "voyage-finance-2" => 1024,
        "voyage-law-2" => 1024,
        "voyage-multilingual-2" => 1024,
        // Default to 1536 (most common)
        _ => 1536,
    }
}

#[async_trait]
impl EmbeddingDriver for OpenAIEmbeddingDriver {
    async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbeddingError> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        // Short-circuit while the breaker is open — avoids spamming the
        // dead endpoint and lets the caller fall back to text search
        // immediately.
        if let Some(err) = self.check_circuit() {
            return Err(err);
        }

        let url = format!("{}/embeddings", self.base_url);
        let body = EmbedRequest {
            model: &self.model,
            input: texts,
        };

        let mut req = self.client.post(&url).json(&body);
        if !self.api_key.as_str().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", self.api_key.as_str()));
        }

        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                let msg = e.to_string();
                let first = self.record_failure(msg.clone());
                if first {
                    warn!(
                        base_url = %self.base_url,
                        error = %msg,
                        cooldown_secs = FAILURE_COOLDOWN.as_secs(),
                        "Embedding endpoint unreachable; suppressing further warnings during cooldown"
                    );
                } else {
                    debug!(error = %msg, "Embedding endpoint still unreachable");
                }
                return Err(EmbeddingError::Http(msg));
            }
        };
        let status = resp.status().as_u16();

        if status != 200 {
            let body_text = resp
                .text()
                .await
                .unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            let first = self.record_failure(format!("status {status}"));
            if first {
                warn!(
                    base_url = %self.base_url,
                    status,
                    body = %body_text,
                    cooldown_secs = FAILURE_COOLDOWN.as_secs(),
                    "Embedding endpoint returned error; suppressing further warnings during cooldown"
                );
            } else {
                debug!(status, body = %body_text, "Embedding endpoint still erroring");
            }
            return Err(EmbeddingError::Api {
                status,
                message: body_text,
            });
        }

        let data: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| EmbeddingError::Parse(e.to_string()))?;

        // Restore input order from the response `index` before collecting: some
        // OpenAI-compatible endpoints return embeddings out of input order, and
        // the ingest path pairs them positionally with chunks — a mismatch
        // silently attaches each chunk to the wrong vector (corrupt RAG search
        // that no error surface reveals).
        let embeddings = order_embeddings(data.data, texts.len());

        debug!(
            "Embedded {} texts (dims={})",
            embeddings.len(),
            embeddings.first().map(|e| e.len()).unwrap_or(0)
        );

        // Successful round-trip — clear the breaker if it was tripped.
        self.record_success();

        Ok(embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dims
    }
}

/// Create an embedding driver from kernel config.
pub fn create_embedding_driver(
    provider: &str,
    model: &str,
    api_key_env: &str,
) -> Result<Box<dyn EmbeddingDriver + Send + Sync>, EmbeddingError> {
    let api_key = if api_key_env.is_empty() {
        String::new()
    } else {
        std::env::var(api_key_env).unwrap_or_default()
    };

    let base_url = match provider {
        "ollama" => OLLAMA_BASE_URL.to_string(),
        "voyage" | "voyageai" => VOYAGE_BASE_URL.to_string(),
        // OpenAI embeddings (text-embedding-3-*) are kept available even though
        // OpenAI is not a first-class LLM provider in v0.7.0 — RAG / memory
        // quality is worth the separate env var.
        "openai" => OPENAI_EMBEDDING_BASE_URL.to_string(),
        other => {
            warn!("Unknown embedding provider '{other}', using OpenAI-compatible format");
            format!("https://{other}/v1")
        }
    };

    // SECURITY: Warn when embedding requests will be sent to an external API
    let is_local = base_url.contains("localhost")
        || base_url.contains("127.0.0.1")
        || base_url.contains("[::1]");
    if !is_local {
        warn!(
            provider = %provider,
            base_url = %base_url,
            "Embedding driver configured to send data to external API — text content will leave this machine"
        );
    }

    let config = EmbeddingConfig {
        provider: provider.to_string(),
        model: model.to_string(),
        api_key,
        base_url,
    };

    let driver = OpenAIEmbeddingDriver::new(config)?;
    Ok(Box::new(driver))
}

/// Compute cosine similarity between two vectors.
///
/// Returns a value in [-1.0, 1.0] where 1.0 = identical direction.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom < f32::EPSILON {
        0.0
    } else {
        dot / denom
    }
}

/// Serialize an embedding vector to bytes (for SQLite BLOB storage).
pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(embedding.len() * 4);
    for &val in embedding {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    bytes
}

/// Deserialize an embedding vector from bytes.
pub fn embedding_from_bytes(bytes: &[u8]) -> Vec<f32> {
    // NB: written as an explicit split_at loop rather than `chunks_exact(4)` or
    // `as_chunks::<4>()`. clippy 1.98 rejects the former (chunks_exact_to_as_chunks)
    // and the latter needs Rust 1.88, above the workspace MSRV of 1.75.
    let mut out = Vec::with_capacity(bytes.len() / 4);
    let mut rest = bytes;
    while rest.len() >= 4 {
        let (head, tail) = rest.split_at(4);
        out.push(f32::from_le_bytes([head[0], head[1], head[2], head[3]]));
        rest = tail;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── order_embeddings / index handling ───────────────────────────────────

    fn item(index: Option<i64>, tag: f32) -> EmbedData {
        EmbedData {
            embedding: vec![tag],
            index,
        }
    }

    /// The bug this guards: some OpenAI-compatible endpoints return the data
    /// array out of input order, and the ingest path pairs it positionally with
    /// chunks — so every chunk silently gets the wrong vector and RAG search is
    /// corrupted with no error anywhere.
    #[test]
    fn reorders_by_index() {
        let items = vec![item(Some(2), 2.0), item(Some(0), 0.0), item(Some(1), 1.0)];
        let out = order_embeddings(items, 3);
        assert_eq!(out, vec![vec![0.0], vec![1.0], vec![2.0]]);
    }

    /// Providers that omit `index` rely on array order — it must be preserved.
    #[test]
    fn preserves_array_order_when_index_is_absent() {
        let items = vec![item(None, 0.0), item(None, 1.0), item(None, 2.0)];
        let out = order_embeddings(items, 3);
        assert_eq!(out, vec![vec![0.0], vec![1.0], vec![2.0]]);
    }

    /// Placement rather than a blind sort: duplicate or out-of-range indices
    /// must NOT produce a confidently-wrong pairing. Falling back to array order
    /// is the honest answer.
    #[test]
    fn falls_back_to_array_order_on_bad_indices() {
        // Duplicate index.
        let out = order_embeddings(vec![item(Some(0), 7.0), item(Some(0), 8.0)], 2);
        assert_eq!(out, vec![vec![7.0], vec![8.0]]);

        // Out of range.
        let out = order_embeddings(vec![item(Some(0), 7.0), item(Some(9), 8.0)], 2);
        assert_eq!(out, vec![vec![7.0], vec![8.0]]);

        // Partially present.
        let out = order_embeddings(vec![item(Some(1), 7.0), item(None, 8.0)], 2);
        assert_eq!(out, vec![vec![7.0], vec![8.0]]);
    }

    #[test]
    fn falls_back_when_count_does_not_match() {
        let out = order_embeddings(vec![item(Some(0), 7.0)], 2);
        assert_eq!(
            out,
            vec![vec![7.0]],
            "length is reported to the caller as-is"
        );
    }

    /// `index` is `Option<i64>` with a lenient deserializer on purpose:
    /// `#[serde(default)]` only fires when the key is ABSENT, so a provider
    /// sending `"index": null` (or a float, or -1) would otherwise fail to
    /// deserialize the ENTIRE response and break embedding for that endpoint.
    #[test]
    fn tolerates_unusable_index_values() {
        let parsed: EmbedResponse = serde_json::from_str(
            r#"{"data":[
                {"embedding":[1.0],"index":null},
                {"embedding":[2.0],"index":1.5},
                {"embedding":[3.0],"index":-1},
                {"embedding":[4.0]}
            ]}"#,
        )
        .expect("odd index values must not fail the whole response");
        assert_eq!(parsed.data.len(), 4);
        assert!(parsed.data.iter().all(|d| d.index.is_none()));
        // ...and the fallback keeps array order.
        let out = order_embeddings(parsed.data, 4);
        assert_eq!(out, vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]]);
    }

    #[test]
    fn parses_well_formed_indices() {
        let parsed: EmbedResponse = serde_json::from_str(
            r#"{"data":[{"embedding":[1.0],"index":1},{"embedding":[0.0],"index":0}]}"#,
        )
        .unwrap();
        let out = order_embeddings(parsed.data, 2);
        assert_eq!(out, vec![vec![0.0], vec![1.0]]);
    }

    /// Regression: when the embedding endpoint is down, the driver must
    /// short-circuit subsequent calls instead of issuing fresh HTTP
    /// attempts every message. Without this, a fresh install with no
    /// Ollama running floods the logs with two warnings per inbound
    /// Telegram message (one for memory recall, one for "remember")
    /// AND blocks the agent for the duration of two HTTP timeouts.
    #[tokio::test]
    async fn embed_short_circuits_after_failure() {
        // Point at a port that's guaranteed to refuse the connection.
        let driver = OpenAIEmbeddingDriver::new(EmbeddingConfig {
            provider: "ollama".to_string(),
            model: "nomic-embed-text".to_string(),
            api_key: String::new(),
            base_url: "http://127.0.0.1:1".to_string(),
        })
        .expect("driver");

        // First call: real HTTP attempt, fails with Http error, trips
        // the breaker and emits the WARN log.
        let first = driver.embed(&["hello"]).await;
        assert!(matches!(first, Err(EmbeddingError::Http(_))));

        // Second call within the cooldown window: no HTTP, returns
        // Unavailable immediately. agent_loop logs at debug! level.
        let second = driver.embed(&["world"]).await;
        match second {
            Err(EmbeddingError::Unavailable(msg)) => {
                assert!(!msg.is_empty(), "Unavailable carries the cached error msg");
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 0.0];
        let b = vec![-1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_real_vectors() {
        let a = vec![0.1, 0.2, 0.3, 0.4];
        let b = vec![0.1, 0.2, 0.3, 0.4];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-5);

        let c = vec![0.4, 0.3, 0.2, 0.1];
        let sim2 = cosine_similarity(&a, &c);
        assert!(sim2 > 0.0 && sim2 < 1.0); // Similar but not identical
    }

    #[test]
    fn test_cosine_similarity_empty() {
        let sim = cosine_similarity(&[], &[]);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_length_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        let sim = cosine_similarity(&a, &b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_embedding_roundtrip() {
        let embedding = vec![0.1, -0.5, 1.23456, 0.0, -1e10, 1e10];
        let bytes = embedding_to_bytes(&embedding);
        let recovered = embedding_from_bytes(&bytes);
        assert_eq!(embedding.len(), recovered.len());
        for (a, b) in embedding.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_embedding_bytes_empty() {
        let bytes = embedding_to_bytes(&[]);
        assert!(bytes.is_empty());
        let recovered = embedding_from_bytes(&bytes);
        assert!(recovered.is_empty());
    }

    #[test]
    fn test_infer_dimensions() {
        assert_eq!(infer_dimensions("text-embedding-3-small"), 1536);
        assert_eq!(infer_dimensions("all-MiniLM-L6-v2"), 384);
        assert_eq!(infer_dimensions("nomic-embed-text"), 768);
        assert_eq!(infer_dimensions("unknown-model"), 1536); // default
    }

    #[test]
    fn test_create_embedding_driver_ollama() {
        // Should succeed even without API key (ollama is local)
        let driver = create_embedding_driver("ollama", "all-MiniLM-L6-v2", "");
        assert!(driver.is_ok());
        assert_eq!(driver.unwrap().dimensions(), 384);
    }
}
