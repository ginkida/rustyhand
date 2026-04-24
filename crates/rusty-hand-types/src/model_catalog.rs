//! Model catalog types — shared data structures for the model registry.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Canonical provider base URLs — single source of truth.
// Referenced by rusty-hand-runtime drivers, model catalog, and embedding modules.
// ---------------------------------------------------------------------------

// RustyHand ships with seven LLM completion providers. Auxiliary services
// (OpenAI TTS, DALL-E image gen, Whisper audio transcription, Perplexity web
// search) talk to their upstreams directly via HTTP and don't need entries
// here — they read env vars at request time.

// ── Anthropic (native Messages API) ───────────────────────────────
pub const ANTHROPIC_BASE_URL: &str = "https://api.anthropic.com";

// ── Kimi Code (Moonshot's Anthropic-compatible coding endpoint) ──
// Uses Anthropic Messages API format. Driven by `AnthropicDriver`, not `OpenAIDriver`.
pub const KIMI_CODE_BASE_URL: &str = "https://api.kimi.com/coding";

// ── OpenAI-compatible clouds ──────────────────────────────────────
pub const DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";
// DeepSeek also exposes an Anthropic-compatible endpoint at the same hostname
// (same DEEPSEEK_API_KEY, Messages API wire format). Useful for clients that
// only speak Anthropic's wire, e.g. Claude Code pointed at DeepSeek backends.
// Drive through `AnthropicDriver` by setting `provider = "anthropic"` +
// `base_url = DEEPSEEK_ANTHROPIC_BASE_URL` + `api_key_env = "DEEPSEEK_API_KEY"`.
pub const DEEPSEEK_ANTHROPIC_BASE_URL: &str = "https://api.deepseek.com/anthropic";
pub const OPENROUTER_BASE_URL: &str = "https://openrouter.ai/api/v1";
pub const MINIMAX_BASE_URL: &str = "https://api.minimax.io/v1";
pub const ZHIPU_BASE_URL: &str = "https://open.bigmodel.cn/api/paas/v4";

// ── Local ─────────────────────────────────────────────────────────
pub const OLLAMA_BASE_URL: &str = "http://localhost:11434/v1";

// ── Embedding-only upstreams ──────────────────────────────────────
// These are NOT completion providers — RustyHand does not offer GPT/OpenAI
// LLMs as of v0.7.0. They are kept only for `embedding.rs` so users can still
// use high-quality text embedders for RAG / memory. Set VOYAGE_API_KEY or
// OPENAI_API_KEY independently of the LLM provider config.
pub const VOYAGE_BASE_URL: &str = "https://api.voyageai.com/v1";
pub const OPENAI_EMBEDDING_BASE_URL: &str = "https://api.openai.com/v1";

/// A model's capability tier.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    /// Cutting-edge, most capable models (e.g. Claude Opus, GPT-4.1).
    Frontier,
    /// Smart, cost-effective models (e.g. Claude Sonnet, Gemini 2.5 Flash).
    Smart,
    /// Balanced speed/cost models (e.g. GPT-4o-mini, Groq Llama).
    #[default]
    Balanced,
    /// Fastest, cheapest models for simple tasks.
    Fast,
    /// Local models (Ollama, vLLM, LM Studio).
    Local,
}

impl fmt::Display for ModelTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModelTier::Frontier => write!(f, "frontier"),
            ModelTier::Smart => write!(f, "smart"),
            ModelTier::Balanced => write!(f, "balanced"),
            ModelTier::Fast => write!(f, "fast"),
            ModelTier::Local => write!(f, "local"),
        }
    }
}

/// Provider authentication status.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStatus {
    /// API key is present in the environment.
    Configured,
    /// API key is missing.
    #[default]
    Missing,
    /// No API key required (local providers).
    NotRequired,
}

impl fmt::Display for AuthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthStatus::Configured => write!(f, "configured"),
            AuthStatus::Missing => write!(f, "missing"),
            AuthStatus::NotRequired => write!(f, "not_required"),
        }
    }
}

/// A single model entry in the catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCatalogEntry {
    /// Canonical model identifier (e.g. "claude-sonnet-4-20250514").
    pub id: String,
    /// Human-readable display name (e.g. "Claude Sonnet 4").
    pub display_name: String,
    /// Provider identifier (e.g. "anthropic").
    pub provider: String,
    /// Capability tier.
    pub tier: ModelTier,
    /// Context window size in tokens.
    pub context_window: u64,
    /// Maximum output tokens.
    pub max_output_tokens: u64,
    /// Cost per million input tokens (USD).
    pub input_cost_per_m: f64,
    /// Cost per million output tokens (USD).
    pub output_cost_per_m: f64,
    /// Whether the model supports tool/function calling.
    pub supports_tools: bool,
    /// Whether the model supports vision/image inputs.
    pub supports_vision: bool,
    /// Whether the model supports streaming responses.
    pub supports_streaming: bool,
    /// Aliases for this model (e.g. ["sonnet", "claude-sonnet"]).
    #[serde(default)]
    pub aliases: Vec<String>,
}

impl Default for ModelCatalogEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            provider: String::new(),
            tier: ModelTier::default(),
            context_window: 0,
            max_output_tokens: 0,
            input_cost_per_m: 0.0,
            output_cost_per_m: 0.0,
            supports_tools: false,
            supports_vision: false,
            supports_streaming: false,
            aliases: Vec::new(),
        }
    }
}

/// Provider metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    /// Provider identifier (e.g. "anthropic").
    pub id: String,
    /// Human-readable display name (e.g. "Anthropic").
    pub display_name: String,
    /// Environment variable name for the API key.
    pub api_key_env: String,
    /// Default base URL.
    pub base_url: String,
    /// Whether an API key is required (false for local providers).
    pub key_required: bool,
    /// Runtime-detected authentication status.
    pub auth_status: AuthStatus,
    /// Number of models from this provider in the catalog.
    pub model_count: usize,
}

impl Default for ProviderInfo {
    fn default() -> Self {
        Self {
            id: String::new(),
            display_name: String::new(),
            api_key_env: String::new(),
            base_url: String::new(),
            key_required: true,
            auth_status: AuthStatus::default(),
            model_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_tier_display() {
        assert_eq!(ModelTier::Frontier.to_string(), "frontier");
        assert_eq!(ModelTier::Smart.to_string(), "smart");
        assert_eq!(ModelTier::Balanced.to_string(), "balanced");
        assert_eq!(ModelTier::Fast.to_string(), "fast");
        assert_eq!(ModelTier::Local.to_string(), "local");
    }

    #[test]
    fn test_auth_status_display() {
        assert_eq!(AuthStatus::Configured.to_string(), "configured");
        assert_eq!(AuthStatus::Missing.to_string(), "missing");
        assert_eq!(AuthStatus::NotRequired.to_string(), "not_required");
    }

    #[test]
    fn test_model_tier_default() {
        assert_eq!(ModelTier::default(), ModelTier::Balanced);
    }

    #[test]
    fn test_auth_status_default() {
        assert_eq!(AuthStatus::default(), AuthStatus::Missing);
    }

    #[test]
    fn test_model_catalog_entry_default() {
        let entry = ModelCatalogEntry::default();
        assert!(entry.id.is_empty());
        assert_eq!(entry.tier, ModelTier::Balanced);
        assert!(entry.aliases.is_empty());
    }

    #[test]
    fn test_provider_info_default() {
        let info = ProviderInfo::default();
        assert!(info.id.is_empty());
        assert!(info.key_required);
        assert_eq!(info.auth_status, AuthStatus::Missing);
    }

    #[test]
    fn test_model_tier_serde_roundtrip() {
        let tier = ModelTier::Frontier;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"frontier\"");
        let parsed: ModelTier = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, tier);
    }

    #[test]
    fn test_auth_status_serde_roundtrip() {
        let status = AuthStatus::Configured;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"configured\"");
        let parsed: AuthStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_model_entry_serde_roundtrip() {
        let entry = ModelCatalogEntry {
            id: "claude-sonnet-4-20250514".to_string(),
            display_name: "Claude Sonnet 4".to_string(),
            provider: "anthropic".to_string(),
            tier: ModelTier::Smart,
            context_window: 200_000,
            max_output_tokens: 64_000,
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec!["sonnet".to_string(), "claude-sonnet".to_string()],
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: ModelCatalogEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, entry.id);
        assert_eq!(parsed.tier, ModelTier::Smart);
        assert_eq!(parsed.aliases.len(), 2);
    }

    #[test]
    fn test_provider_info_serde_roundtrip() {
        let info = ProviderInfo {
            id: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: "https://api.anthropic.com".to_string(),
            key_required: true,
            auth_status: AuthStatus::Configured,
            model_count: 3,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: ProviderInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "anthropic");
        assert_eq!(parsed.auth_status, AuthStatus::Configured);
        assert_eq!(parsed.model_count, 3);
    }
}
