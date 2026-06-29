//! Provider health probing — lightweight HTTP checks for local LLM providers.
//!
//! Probes local providers (Ollama, vLLM, LM Studio) for reachability and
//! dynamically discovers which models they currently serve.

use std::time::Instant;

/// Result of probing a provider endpoint.
#[derive(Debug, Clone, Default)]
pub struct ProbeResult {
    /// Whether the provider responded successfully.
    pub reachable: bool,
    /// Round-trip latency in milliseconds.
    pub latency_ms: u64,
    /// Model IDs discovered from the provider's listing endpoint.
    pub discovered_models: Vec<String>,
    /// Error message if the probe failed.
    pub error: Option<String>,
}

/// Check if a provider is a local provider (no key required, localhost URL).
///
/// Returns true for `"ollama"`. RustyHand v0.7.0 dropped vLLM and LM Studio as
/// first-class providers; users can still point `base_url` at them manually.
pub fn is_local_provider(provider: &str) -> bool {
    matches!(provider.to_lowercase().as_str(), "ollama")
}

/// Probe timeout for local provider health checks.
const PROBE_TIMEOUT_SECS: u64 = 5;

/// Maximum size (in bytes) of a provider health-probe response body we will
/// buffer and parse. A model listing (or a tiny probe completion) is small, so
/// this cap is safe — it exists only to stop a rogue/compromised local provider
/// from exhausting kernel memory with an unbounded body.
const MAX_PROBE_BODY_BYTES: usize = 4 * 1024 * 1024;

/// Read an HTTP response body into a `Vec<u8>`, enforcing a hard size cap.
///
/// Rejects early if a known `Content-Length` exceeds `max`, then streams the
/// body while accumulating a running total, aborting if it exceeds `max`
/// (covers chunked responses that omit `Content-Length`).
async fn read_capped_bytes(resp: reqwest::Response, max: usize) -> Result<Vec<u8>, String> {
    if let Some(len) = resp.content_length() {
        if len > max as u64 {
            return Err(format!("response too large: {len} bytes (max {max})"));
        }
    }

    let mut bytes = Vec::new();
    let mut stream = resp.bytes_stream();
    use futures::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("failed to read response body: {e}"))?;
        bytes.extend_from_slice(&chunk);
        if bytes.len() > max {
            return Err(format!("response body exceeded {max} byte limit"));
        }
    }

    Ok(bytes)
}

/// Probe a provider's health by hitting its model listing endpoint.
///
/// - **Ollama**: `GET {base_url_root}/api/tags` → parses `.models[].name`
/// - **OpenAI-compat** (vLLM, LM Studio): `GET {base_url}/models` → parses `.data[].id`
///
/// `base_url` should be the provider's base URL from the catalog (e.g.,
/// `http://localhost:11434/v1` for Ollama, `http://localhost:8000/v1` for vLLM).
pub async fn probe_provider(provider: &str, base_url: &str) -> ProbeResult {
    let start = Instant::now();

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(PROBE_TIMEOUT_SECS))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProbeResult {
                error: Some(format!("Failed to build HTTP client: {e}")),
                ..Default::default()
            };
        }
    };

    let lower = provider.to_lowercase();

    // Ollama uses a non-OpenAI endpoint for model listing
    let (url, is_ollama) = if lower == "ollama" {
        // base_url is typically "http://localhost:11434/v1" — strip /v1 for the tags endpoint
        let root = base_url
            .trim_end_matches('/')
            .trim_end_matches("/v1")
            .trim_end_matches("/v1/");
        (format!("{root}/api/tags"), true)
    } else {
        // OpenAI-compatible: GET {base_url}/models
        let trimmed = base_url.trim_end_matches('/');
        (format!("{trimmed}/models"), false)
    };

    let resp = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            return ProbeResult {
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("{e}")),
                ..Default::default()
            };
        }
    };

    if !resp.status().is_success() {
        return ProbeResult {
            latency_ms: start.elapsed().as_millis() as u64,
            error: Some(format!("HTTP {}", resp.status())),
            ..Default::default()
        };
    }

    // Cap the body before parsing — a rogue/compromised local provider must not
    // be able to OOM the kernel with an unbounded model listing.
    let bytes = match read_capped_bytes(resp, MAX_PROBE_BODY_BYTES).await {
        Ok(b) => b,
        Err(e) => {
            return ProbeResult {
                reachable: true, // server responded, body read/size failed
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(e),
                ..Default::default()
            };
        }
    };

    let body: serde_json::Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(e) => {
            return ProbeResult {
                reachable: true, // server responded, just bad JSON
                latency_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Invalid JSON: {e}")),
                ..Default::default()
            };
        }
    };

    let latency_ms = start.elapsed().as_millis() as u64;

    // Parse model names
    let models = if is_ollama {
        // Ollama: { "models": [ { "name": "llama3.2:latest", ... }, ... ] }
        body.get("models")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| {
                        m.get("name")
                            .and_then(|n| n.as_str())
                            .map(|s| s.to_string())
                    })
                    .collect()
            })
            .unwrap_or_default()
    } else {
        // OpenAI-compatible: { "data": [ { "id": "model-name", ... }, ... ] }
        body.get("data")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m.get("id").and_then(|n| n.as_str()).map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default()
    };

    ProbeResult {
        reachable: true,
        latency_ms,
        discovered_models: models,
        error: None,
    }
}

/// Lightweight model probe -- sends a minimal completion request to verify a model is responsive.
///
/// Unlike `probe_provider` which checks the listing endpoint, this actually sends
/// a tiny prompt ("Hi") to verify the model can generate completions. Used by the
/// circuit breaker to re-test a provider during cooldown.
///
/// Returns `Ok(latency_ms)` if the model responds, or `Err(error_message)` if it fails.
pub async fn probe_model(
    provider: &str,
    base_url: &str,
    model: &str,
    api_key: Option<&str>,
) -> Result<u64, String> {
    let start = Instant::now();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "Hi"}],
        "max_tokens": 1,
        "temperature": 0.0
    });

    let mut req = client.post(&url).json(&body);
    if let Some(key) = api_key {
        // All v0.7.0 providers use Authorization: Bearer (OpenAI-compat) or
        // x-api-key (Anthropic/Kimi). This probe function is only called for
        // OpenAI-compat providers; Anthropic/Kimi health checks go through the
        // dedicated driver path in routes.rs.
        let _ = provider; // Kept in signature for callers that pass it for logging.
        req = req.header("Authorization", format!("Bearer {key}"));
    }

    let resp = req.send().await.map_err(|e| format!("{e}"))?;
    let latency = start.elapsed().as_millis() as u64;

    if resp.status().is_success() {
        Ok(latency)
    } else {
        let status = resp.status().as_u16();
        // Cap the error body before buffering — a rogue provider must not be
        // able to OOM the kernel via a giant error response.
        let body = match read_capped_bytes(resp, MAX_PROBE_BODY_BYTES).await {
            Ok(b) => String::from_utf8_lossy(&b).into_owned(),
            Err(e) => format!("<failed to read body: {e}>"),
        };
        Err(format!(
            "HTTP {status}: {}",
            rusty_hand_types::text::truncate_bytes(&body, 200)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_local_provider_true_for_ollama() {
        assert!(is_local_provider("ollama"));
        assert!(is_local_provider("Ollama"));
        assert!(is_local_provider("OLLAMA"));
    }

    #[test]
    fn test_is_local_provider_false_for_clouds() {
        assert!(!is_local_provider("anthropic"));
        assert!(!is_local_provider("kimi"));
        assert!(!is_local_provider("deepseek"));
        assert!(!is_local_provider("openrouter"));
        assert!(!is_local_provider("minimax"));
        assert!(!is_local_provider("zhipu"));
    }

    #[test]
    fn test_probe_result_default() {
        let result = ProbeResult::default();
        assert!(!result.reachable);
        assert_eq!(result.latency_ms, 0);
        assert!(result.discovered_models.is_empty());
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_probe_unreachable_returns_error() {
        // Probe a port that's almost certainly not running a server
        let result = probe_provider("ollama", "http://127.0.0.1:19999").await;
        assert!(!result.reachable);
        assert!(result.error.is_some());
    }

    #[test]
    fn test_probe_timeout_value() {
        assert_eq!(PROBE_TIMEOUT_SECS, 5);
    }

    #[test]
    fn test_probe_model_url_construction() {
        // Verify the URL format logic used inside probe_model.
        let url = format!(
            "{}/chat/completions",
            "http://localhost:8000/v1".trim_end_matches('/')
        );
        assert_eq!(url, "http://localhost:8000/v1/chat/completions");

        let url2 = format!(
            "{}/chat/completions",
            "http://localhost:8000/v1/".trim_end_matches('/')
        );
        assert_eq!(url2, "http://localhost:8000/v1/chat/completions");
    }

    #[tokio::test]
    async fn test_probe_model_unreachable() {
        let result = probe_model("test", "http://127.0.0.1:19998/v1", "test-model", None).await;
        assert!(result.is_err());
    }
}
