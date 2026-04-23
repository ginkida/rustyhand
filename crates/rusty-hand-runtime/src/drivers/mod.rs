//! LLM driver implementations.
//!
//! RustyHand ships with a deliberately lean set of seven LLM providers, driven
//! by two wire protocols:
//!
//! * **AnthropicDriver** — speaks the Anthropic Messages API. Used by
//!   `anthropic` and by `kimi` (Kimi Code, `api.kimi.com/coding`, which is
//!   Anthropic-compatible).
//! * **OpenAIDriver** — speaks the OpenAI Chat Completions API. Used by
//!   `deepseek`, `minimax`, `zhipu` (GLM), `ollama`, and `openrouter`, plus
//!   any custom OpenAI-compatible endpoint supplied via `base_url`.
//!
//! Historically RustyHand supported 27 providers (OpenAI, Gemini, Groq, xAI,
//! GitHub Copilot, …). They were removed in v0.7.0 to shrink scope: see
//! `openrouter` for a universal gateway to any model still not covered here.

pub mod anthropic;
pub mod fallback;
pub mod openai;

use crate::llm_driver::{CompletionRequest, CompletionResponse, DriverConfig, LlmDriver, LlmError};
use rusty_hand_types::model_catalog::{
    ANTHROPIC_BASE_URL, DEEPSEEK_BASE_URL, KIMI_CODE_BASE_URL, MINIMAX_BASE_URL, OLLAMA_BASE_URL,
    OPENROUTER_BASE_URL, ZHIPU_BASE_URL,
};
use std::sync::Arc;

/// Provider metadata: base URL and env var name for the API key.
struct ProviderDefaults {
    base_url: &'static str,
    api_key_env: &'static str,
    /// If true, the API key is required (error if missing).
    key_required: bool,
}

/// Get defaults for known providers.
fn provider_defaults(provider: &str) -> Option<ProviderDefaults> {
    match provider {
        // Premium cloud, Anthropic wire protocol.
        "anthropic" => Some(ProviderDefaults {
            base_url: ANTHROPIC_BASE_URL,
            api_key_env: "ANTHROPIC_API_KEY",
            key_required: true,
        }),
        // Kimi Code — Moonshot's Anthropic-compatible coding endpoint.
        // Routed through `AnthropicDriver` in `create_driver`; surfaced here so
        // introspection, catalog, and the `/api/providers/{name}/test` route
        // can resolve the base URL and env var.
        "kimi" => Some(ProviderDefaults {
            base_url: KIMI_CODE_BASE_URL,
            api_key_env: "KIMI_API_KEY",
            key_required: true,
        }),
        // OpenAI-compatible cloud providers.
        "deepseek" => Some(ProviderDefaults {
            base_url: DEEPSEEK_BASE_URL,
            api_key_env: "DEEPSEEK_API_KEY",
            key_required: true,
        }),
        "minimax" => Some(ProviderDefaults {
            base_url: MINIMAX_BASE_URL,
            api_key_env: "MINIMAX_API_KEY",
            key_required: true,
        }),
        "zhipu" | "glm" => Some(ProviderDefaults {
            base_url: ZHIPU_BASE_URL,
            api_key_env: "ZHIPU_API_KEY",
            key_required: true,
        }),
        "openrouter" => Some(ProviderDefaults {
            base_url: OPENROUTER_BASE_URL,
            api_key_env: "OPENROUTER_API_KEY",
            key_required: true,
        }),
        // Local.
        "ollama" => Some(ProviderDefaults {
            base_url: OLLAMA_BASE_URL,
            api_key_env: "OLLAMA_API_KEY",
            key_required: false,
        }),
        _ => None,
    }
}

/// Create an LLM driver based on provider name and configuration.
///
/// Supported providers:
/// - `anthropic` — Anthropic Claude (Messages API)
/// - `kimi` — Moonshot Kimi Code (Anthropic-compatible endpoint)
/// - `deepseek` — DeepSeek V3 / R1 reasoning
/// - `minimax` — MiniMax M1 / M2 long-context
/// - `zhipu` (alias `glm`) — Zhipu AI GLM-4.6
/// - `openrouter` — OpenRouter (universal gateway)
/// - `ollama` — Ollama (local)
/// - Any custom provider with `base_url` set uses OpenAI-compatible format
pub fn create_driver(config: &DriverConfig) -> Result<Arc<dyn LlmDriver>, LlmError> {
    let provider = config.provider.as_str();

    // Anthropic uses its own API format — special case.
    if provider == "anthropic" {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::MissingApiKey("Set ANTHROPIC_API_KEY environment variable".to_string())
            })?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| ANTHROPIC_BASE_URL.to_string());
        return Ok(Arc::new(anthropic::AnthropicDriver::new(api_key, base_url)));
    }

    // Kimi Code — Moonshot's Anthropic-compatible endpoint.
    // Same wire protocol as Anthropic (Messages API, x-api-key, anthropic-version),
    // so we reuse AnthropicDriver with a different base URL. Env var is KIMI_API_KEY
    // (distinct from ANTHROPIC_API_KEY).
    if provider == "kimi" {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var("KIMI_API_KEY").ok())
            .ok_or_else(|| {
                LlmError::MissingApiKey(
                    "Set KIMI_API_KEY environment variable (get one at https://platform.moonshot.ai/console/code)".to_string(),
                )
            })?;
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| KIMI_CODE_BASE_URL.to_string());
        return Ok(Arc::new(anthropic::AnthropicDriver::new(api_key, base_url)));
    }

    // All other providers use OpenAI-compatible format.
    if let Some(defaults) = provider_defaults(provider) {
        let api_key = config
            .api_key
            .clone()
            .or_else(|| std::env::var(defaults.api_key_env).ok())
            .unwrap_or_default();

        if defaults.key_required && api_key.is_empty() {
            return Err(LlmError::MissingApiKey(format!(
                "Set {} environment variable for provider '{}'",
                defaults.api_key_env, provider
            )));
        }

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| defaults.base_url.to_string());

        return Ok(Arc::new(openai::OpenAIDriver::new(api_key, base_url)));
    }

    // Unknown provider — if base_url is set, treat as custom OpenAI-compatible.
    if let Some(ref base_url) = config.base_url {
        let api_key = config.api_key.clone().unwrap_or_default();
        return Ok(Arc::new(openai::OpenAIDriver::new(
            api_key,
            base_url.clone(),
        )));
    }

    Err(LlmError::Api {
        status: 0,
        message: format!(
            "Unknown provider '{}'. Supported: anthropic, kimi, deepseek, minimax, zhipu \
             (alias glm), openrouter, ollama. Or set base_url for a custom OpenAI-compatible \
             endpoint.",
            provider
        ),
    })
}

/// Placeholder driver returned when no LLM provider is configured.
///
/// Returns a clear error message on any completion request, allowing the
/// kernel to boot and serve the dashboard while prompting the user to
/// configure an API key.
pub struct NullDriver;

#[async_trait::async_trait]
impl LlmDriver for NullDriver {
    async fn complete(&self, _request: CompletionRequest) -> Result<CompletionResponse, LlmError> {
        Err(LlmError::Api {
            status: 0,
            message: "No LLM provider configured. Set ANTHROPIC_API_KEY, KIMI_API_KEY, \
                      DEEPSEEK_API_KEY, MINIMAX_API_KEY, ZHIPU_API_KEY, or OPENROUTER_API_KEY, \
                      or run `rustyhand init` to choose a provider."
                .to_string(),
        })
    }
}

/// Auto-detect an available LLM provider by scanning environment variables.
///
/// Returns `(provider, api_key_env, recommended_model)` for the first provider
/// whose API key is found in the environment. Providers are checked in priority
/// order (Anthropic + Kimi are the two first-class coding providers, then the
/// rest). Returns `None` if no keys are found.
pub fn detect_available_provider() -> Option<(&'static str, &'static str, &'static str)> {
    const PROBE_ORDER: &[(&str, &str, &str)] = &[
        ("anthropic", "ANTHROPIC_API_KEY", "claude-sonnet-4-20250514"),
        ("kimi", "KIMI_API_KEY", "kimi-for-coding"),
        ("deepseek", "DEEPSEEK_API_KEY", "deepseek-chat"),
        ("zhipu", "ZHIPU_API_KEY", "glm-4-plus"),
        ("minimax", "MINIMAX_API_KEY", "MiniMax-M2.7"),
        (
            "openrouter",
            "OPENROUTER_API_KEY",
            "anthropic/claude-sonnet-4",
        ),
    ];
    for &(provider, env_var, model) in PROBE_ORDER {
        if std::env::var(env_var)
            .ok()
            .filter(|v| !v.is_empty())
            .is_some()
        {
            return Some((provider, env_var, model));
        }
    }
    None
}

/// List all known provider names.
pub fn known_providers() -> &'static [&'static str] {
    &[
        "anthropic",
        "kimi",
        "deepseek",
        "minimax",
        "zhipu",
        "openrouter",
        "ollama",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_defaults_openrouter() {
        let d = provider_defaults("openrouter").unwrap();
        assert_eq!(d.base_url, "https://openrouter.ai/api/v1");
        assert_eq!(d.api_key_env, "OPENROUTER_API_KEY");
        assert!(d.key_required);
    }

    #[test]
    fn test_provider_defaults_ollama() {
        let d = provider_defaults("ollama").unwrap();
        assert!(!d.key_required);
    }

    #[test]
    fn test_provider_defaults_deepseek() {
        let d = provider_defaults("deepseek").unwrap();
        assert_eq!(d.base_url, "https://api.deepseek.com/v1");
        assert_eq!(d.api_key_env, "DEEPSEEK_API_KEY");
    }

    #[test]
    fn test_provider_defaults_minimax() {
        let d = provider_defaults("minimax").unwrap();
        assert_eq!(d.base_url, "https://api.minimax.io/v1");
    }

    #[test]
    fn test_provider_defaults_zhipu_and_glm_alias() {
        let zhipu = provider_defaults("zhipu").unwrap();
        let glm = provider_defaults("glm").unwrap();
        assert_eq!(zhipu.base_url, glm.base_url);
        assert_eq!(zhipu.api_key_env, "ZHIPU_API_KEY");
    }

    #[test]
    fn test_provider_defaults_kimi() {
        let d = provider_defaults("kimi").unwrap();
        assert_eq!(d.base_url, "https://api.kimi.com/coding");
        assert_eq!(d.api_key_env, "KIMI_API_KEY");
        assert!(d.key_required);
    }

    #[test]
    fn test_unknown_provider_returns_none() {
        // Deleted providers must no longer resolve.
        assert!(provider_defaults("openai").is_none());
        assert!(provider_defaults("gemini").is_none());
        assert!(provider_defaults("groq").is_none());
        assert!(provider_defaults("xai").is_none());
        assert!(provider_defaults("moonshot").is_none());
        assert!(provider_defaults("qwen").is_none());
        assert!(provider_defaults("github-copilot").is_none());
        assert!(provider_defaults("nonexistent").is_none());
    }

    #[test]
    fn test_custom_provider_with_base_url() {
        let config = DriverConfig {
            provider: "my-custom-llm".to_string(),
            api_key: Some("test".to_string()),
            base_url: Some("http://localhost:9999/v1".to_string()),
        };
        let driver = create_driver(&config);
        assert!(driver.is_ok());
    }

    #[test]
    fn test_unknown_provider_no_url_errors() {
        let config = DriverConfig {
            provider: "nonexistent".to_string(),
            api_key: None,
            base_url: None,
        };
        let driver = create_driver(&config);
        assert!(driver.is_err());
    }

    #[test]
    fn test_known_providers_list() {
        let providers = known_providers();
        assert_eq!(providers.len(), 7);
        for expected in [
            "anthropic",
            "kimi",
            "deepseek",
            "minimax",
            "zhipu",
            "openrouter",
            "ollama",
        ] {
            assert!(
                providers.contains(&expected),
                "missing provider: {expected}"
            );
        }
        // Deleted providers must not be present.
        for removed in [
            "openai",
            "gemini",
            "groq",
            "xai",
            "mistral",
            "together",
            "fireworks",
            "perplexity",
            "cohere",
            "ai21",
            "cerebras",
            "sambanova",
            "huggingface",
            "replicate",
            "github-copilot",
            "moonshot",
            "qwen",
            "qianfan",
            "vllm",
            "lmstudio",
        ] {
            assert!(
                !providers.contains(&removed),
                "deleted provider still present: {removed}"
            );
        }
    }

    #[test]
    fn test_create_kimi_driver_uses_anthropic_format() {
        // The kimi provider must route through AnthropicDriver (Messages API),
        // NOT the OpenAI-compat driver, since api.kimi.com/coding speaks Anthropic's wire format.
        let config = DriverConfig {
            provider: "kimi".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: None,
        };
        let driver = create_driver(&config);
        assert!(driver.is_ok(), "kimi provider should create a driver");
    }

    #[test]
    fn test_create_kimi_driver_respects_custom_base_url() {
        let config = DriverConfig {
            provider: "kimi".to_string(),
            api_key: Some("test-key".to_string()),
            base_url: Some("https://my-proxy.example.com/kimi".to_string()),
        };
        let driver = create_driver(&config);
        assert!(driver.is_ok());
    }
}
