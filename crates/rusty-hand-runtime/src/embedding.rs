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

        // Update dimensions from actual response if available
        let embeddings: Vec<Vec<f32>> = data.data.into_iter().map(|d| d.embedding).collect();

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
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
