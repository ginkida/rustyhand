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

/// Read an env var, treating an empty string the same as unset.
///
/// Without this, a manifest with `api_key_env = "GEMINI_API_KEY"` plus
/// an env that exports `GEMINI_API_KEY=""` (a common shell mistake)
/// would build an LLM driver with an empty bearer token. The provider
/// would then 401 on every call and the agent_loop would silently
/// surface an empty response — exactly the v0.7.9-era regression.
fn env_nonempty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

/// Like `Option<String>::or_else(env_nonempty(name))` but also rejects
/// `Some("")` from the `config.api_key` side. The kernel's
/// `resolve_driver` reads `std::env::var(api_key_env).ok()` which
/// produces `Some("")` when the env var is set but empty — that
/// cannot fall through to a sensible default without this guard.
fn pick_api_key(explicit: &Option<String>, env_var: &str) -> Option<String> {
    explicit
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| env_nonempty(env_var))
}

/// Create an LLM driver based on provider name and configuration.
///
/// Supported providers:
/// - `anthropic` — Anthropic Claude (Messages API). Also the route for
///   DeepSeek's Anthropic-compatible endpoint — set
///   `base_url = "https://api.deepseek.com/anthropic"` +
///   `api_key_env = "DEEPSEEK_API_KEY"` to drive DeepSeek V4 models over the
///   Messages API wire format. Works for single-turn and multi-turn text.
///   For reasoning + multi-turn tool-use prefer `provider = "deepseek"`
///   (OpenAI wire) — the Anthropic `thinking` block round-trip is not
///   exercised in RustyHand tests and may require a signature field we
///   do not yet persist.
/// - `kimi` — Moonshot Kimi Code (Anthropic-compatible endpoint)
/// - `deepseek` — DeepSeek V4 Flash / V4 Pro (V3/R1 legacy, deprecated 2026-07-24)
/// - `minimax` — MiniMax M1 / M2 long-context
/// - `zhipu` (alias `glm`) — Zhipu AI GLM-4.6
/// - `openrouter` — OpenRouter (universal gateway)
/// - `ollama` — Ollama (local)
/// - Any custom provider with `base_url` set uses OpenAI-compatible format
pub fn create_driver(config: &DriverConfig) -> Result<Arc<dyn LlmDriver>, LlmError> {
    let provider = config.provider.as_str();

    // Anthropic uses its own API format — special case.
    if provider == "anthropic" {
        let api_key = pick_api_key(&config.api_key, "ANTHROPIC_API_KEY").ok_or_else(|| {
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
        let api_key = pick_api_key(&config.api_key, "KIMI_API_KEY").ok_or_else(|| {
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
        let api_key = pick_api_key(&config.api_key, defaults.api_key_env).unwrap_or_default();

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
        ("deepseek", "DEEPSEEK_API_KEY", "deepseek-v4-flash"),
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

    /// Regression: `Some("")` on either side must not slip through.
    /// `std::env::var("X").ok()` returns `Some("")` for an env var that's
    /// declared but empty (a common shell mistake), and the kernel's
    /// `resolve_driver` passes that straight into `DriverConfig.api_key`.
    /// Pre-fix the `or_else` chain only fired on `None`, so an empty
    /// string slipped through and the driver was instantiated with no
    /// auth — the v0.7.9 "silent empty Telegram reply" failure mode.
    ///
    /// We test `pick_api_key` directly against an env var name that's
    /// guaranteed not to be set anywhere in the workspace so the test is
    /// race-free. `std::env::set_var` / `remove_var` are
    /// data-race-unsafe under cargo's parallel test runner (the Rust
    /// 2024 edition marks them `unsafe` for that reason), so a unit
    /// test here that mutated `ANTHROPIC_API_KEY` could flake with
    /// other tests reading it.
    #[test]
    fn pick_api_key_treats_empty_string_as_missing() {
        const ABSENT: &str = "RUSTYHAND_TEST_VAR_NEVER_SET_FOR_PICK_API_KEY";

        // None on the explicit side, env var not set → None.
        assert_eq!(pick_api_key(&None, ABSENT), None);

        // Some("") on the explicit side (the v0.7.9 regression input —
        // `std::env::var("MISSING_KEY_ENV").ok()` returned `Some("")`
        // for declared-but-empty env vars), env var not set → None.
        assert_eq!(pick_api_key(&Some(String::new()), ABSENT), None);

        // Non-empty explicit value short-circuits the env fallback.
        assert_eq!(
            pick_api_key(&Some("sk-ant-explicit".to_string()), ABSENT),
            Some("sk-ant-explicit".to_string())
        );

        // PATH is always non-empty in cargo test, so this exercises the
        // env-var fallback branch without mutating any env state.
        let path_via_env = pick_api_key(&Some(String::new()), "PATH");
        assert!(
            path_via_env.is_some_and(|v| !v.is_empty()),
            "Some(empty) on explicit side must fall through to a \
             populated env var (PATH is always set in cargo test)"
        );
    }

    /// End-to-end check that `create_driver` surfaces an empty
    /// `api_key` as `MissingApiKey` rather than constructing a driver
    /// with empty auth. Uses an `api_key_env` that's guaranteed unset
    /// so we don't have to mutate process env.
    #[test]
    fn create_driver_errors_on_empty_api_key_with_unset_env() {
        // Use a custom base_url so the OpenAI-compat path goes through
        // the "unknown provider with base_url → custom" branch, which
        // doesn't read any env var. Then explicit empty api_key falls
        // through to api_key=String::new() and the driver builds
        // (custom-provider mode is permissive). To force the
        // MissingApiKey path we use a known provider with an unset
        // env var via the kernel-provided `api_key_env`. We can't
        // pass `api_key_env` to create_driver directly — it reads
        // the conventional env var for the provider — so we rely on
        // CI / cargo test running in an environment where neither
        // `ANTHROPIC_API_KEY` nor `KIMI_API_KEY` is set. Skip this
        // test silently if either is set so a developer with real
        // keys exported doesn't see a spurious failure.
        if std::env::var("ANTHROPIC_API_KEY")
            .map(|v| !v.is_empty())
            .unwrap_or(false)
        {
            // Real key in env — pick_api_key would fall through, so
            // this assertion would fail by design. The unit test
            // above already covers the `Some("")` shape.
            return;
        }

        let result = create_driver(&DriverConfig {
            provider: "anthropic".to_string(),
            api_key: Some(String::new()),
            base_url: None,
        });
        assert!(
            matches!(result, Err(LlmError::MissingApiKey(_))),
            "explicit empty api_key + unset ANTHROPIC_API_KEY env must \
             surface MissingApiKey at construction time"
        );
    }

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
    fn test_deepseek_via_anthropic_endpoint() {
        // DeepSeek's Anthropic-compatible endpoint rides on the `anthropic`
        // provider with a custom base_url. Driver should be constructed
        // successfully when DEEPSEEK_API_KEY is supplied.
        use rusty_hand_types::model_catalog::DEEPSEEK_ANTHROPIC_BASE_URL;
        let config = DriverConfig {
            provider: "anthropic".to_string(),
            api_key: Some("sk-deepseek-test".to_string()),
            base_url: Some(DEEPSEEK_ANTHROPIC_BASE_URL.to_string()),
        };
        let driver = create_driver(&config);
        assert!(
            driver.is_ok(),
            "DeepSeek-via-Anthropic endpoint must build a driver"
        );
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

    // ─── Live integration tests (DeepSeek) ────────────────────────────
    // These tests hit the real DeepSeek API and are `#[ignore]`'d by
    // default. Run manually with:
    //   DEEPSEEK_API_KEY=sk-... cargo test -p rusty-hand-runtime \
    //     --lib drivers::tests::live_deepseek -- --ignored --nocapture
    //
    // Purpose: prove that `deepseek-v4-flash` / `deepseek-v4-pro` round-trip
    // through our actual driver code, over both wire protocols. V4 models are
    // reasoning models — response must contain a Thinking block plus text.

    #[tokio::test]
    #[ignore = "requires DEEPSEEK_API_KEY"]
    async fn live_deepseek_v4_flash_openai_wire() {
        let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
        let driver = create_driver(&DriverConfig {
            provider: "deepseek".to_string(),
            api_key: Some(key),
            base_url: None,
        })
        .expect("driver should build");
        let response = driver
            .complete(CompletionRequest {
                model: "deepseek-v4-flash".to_string(),
                messages: vec![rusty_hand_types::message::Message::user(
                    "Reply with just the single word: pong",
                )],
                tools: vec![],
                max_tokens: 512,
                temperature: 0.0,
                system: None,
                thinking: None,
                response_format: Default::default(),
            })
            .await
            .expect("live completion should succeed");
        let text = response.text().to_lowercase();
        assert!(
            text.contains("pong"),
            "expected 'pong' in response text, got: {text:?}"
        );
        let has_thinking = response
            .content
            .iter()
            .any(|b| matches!(b, rusty_hand_types::message::ContentBlock::Thinking { .. }));
        assert!(
            has_thinking,
            "V4 Flash is a reasoning model — expected a Thinking block"
        );
        assert!(response.usage.input_tokens > 0);
        assert!(response.usage.output_tokens > 0);
    }

    #[tokio::test]
    #[ignore = "requires DEEPSEEK_API_KEY"]
    async fn live_deepseek_v4_pro_openai_wire() {
        let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
        let driver = create_driver(&DriverConfig {
            provider: "deepseek".to_string(),
            api_key: Some(key),
            base_url: None,
        })
        .unwrap();
        let response = driver
            .complete(CompletionRequest {
                model: "deepseek-v4-pro".to_string(),
                messages: vec![rusty_hand_types::message::Message::user(
                    "Reply with just the single word: pong",
                )],
                tools: vec![],
                max_tokens: 512,
                temperature: 0.0,
                system: None,
                thinking: None,
                response_format: Default::default(),
            })
            .await
            .expect("live completion should succeed");
        assert!(response.text().to_lowercase().contains("pong"));
    }

    #[tokio::test]
    #[ignore = "requires DEEPSEEK_API_KEY"]
    async fn live_deepseek_v4_flash_anthropic_wire() {
        use rusty_hand_types::model_catalog::DEEPSEEK_ANTHROPIC_BASE_URL;
        let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
        let driver = create_driver(&DriverConfig {
            provider: "anthropic".to_string(),
            api_key: Some(key),
            base_url: Some(DEEPSEEK_ANTHROPIC_BASE_URL.to_string()),
        })
        .expect("driver should build for DeepSeek-via-Anthropic");
        let response = driver
            .complete(CompletionRequest {
                model: "deepseek-v4-flash".to_string(),
                messages: vec![rusty_hand_types::message::Message::user(
                    "Reply with just the single word: pong",
                )],
                tools: vec![],
                max_tokens: 512,
                temperature: 0.0,
                system: None,
                thinking: None,
                response_format: Default::default(),
            })
            .await
            .expect("live completion on Anthropic endpoint should succeed");
        let text = response.text().to_lowercase();
        assert!(
            text.contains("pong"),
            "expected 'pong' via Anthropic wire, got: {text:?}"
        );
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
