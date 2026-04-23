//! Model catalog — registry of known models with metadata, pricing, and auth detection.
//!
//! RustyHand v0.7.0 ships with seven first-class providers:
//! anthropic, kimi, deepseek, minimax, zhipu (GLM), openrouter, ollama.
//! OpenRouter is the universal gateway for any model not listed here.
//!
//! Alias resolution, auth-status detection, and pricing lookups all work off
//! this single registry. Pricing numbers are USD per million tokens; round
//! them to the upstream's published rates.

use rusty_hand_types::model_catalog::{
    AuthStatus, ModelCatalogEntry, ModelTier, ProviderInfo, ANTHROPIC_BASE_URL, DEEPSEEK_BASE_URL,
    KIMI_CODE_BASE_URL, MINIMAX_BASE_URL, OLLAMA_BASE_URL, OPENROUTER_BASE_URL, ZHIPU_BASE_URL,
};
use std::collections::HashMap;

/// The model catalog — registry of all known models and providers.
pub struct ModelCatalog {
    models: Vec<ModelCatalogEntry>,
    aliases: HashMap<String, String>,
    providers: Vec<ProviderInfo>,
}

impl ModelCatalog {
    /// Create a new catalog populated with builtin models and providers.
    pub fn new() -> Self {
        let models = builtin_models();
        let aliases = builtin_aliases();
        let mut providers = builtin_providers();

        // Set model counts on providers
        for provider in &mut providers {
            provider.model_count = models.iter().filter(|m| m.provider == provider.id).count();
        }

        Self {
            models,
            aliases,
            providers,
        }
    }

    /// Detect which providers have API keys configured.
    ///
    /// Checks `std::env::var()` for each provider's API key env var.
    /// Only checks presence — never reads or stores the actual secret.
    pub fn detect_auth(&mut self) {
        fn env_key_present(var: &str) -> bool {
            std::env::var(var)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        }
        for provider in &mut self.providers {
            if !provider.key_required {
                provider.auth_status = AuthStatus::NotRequired;
            } else if env_key_present(&provider.api_key_env) {
                provider.auth_status = AuthStatus::Configured;
            } else {
                provider.auth_status = AuthStatus::Missing;
            }
        }
    }

    /// List all models in the catalog.
    pub fn list_models(&self) -> &[ModelCatalogEntry] {
        &self.models
    }

    /// Find a model by its canonical ID or by alias.
    pub fn find_model(&self, id_or_alias: &str) -> Option<&ModelCatalogEntry> {
        let lower = id_or_alias.to_lowercase();
        if let Some(entry) = self.models.iter().find(|m| m.id.to_lowercase() == lower) {
            return Some(entry);
        }
        if let Some(canonical) = self.aliases.get(&lower) {
            return self.models.iter().find(|m| m.id == *canonical);
        }
        None
    }

    /// Resolve an alias to a canonical model ID, or None if not an alias.
    pub fn resolve_alias(&self, alias: &str) -> Option<&str> {
        self.aliases.get(&alias.to_lowercase()).map(|s| s.as_str())
    }

    /// List all providers.
    pub fn list_providers(&self) -> &[ProviderInfo] {
        &self.providers
    }

    /// Get a provider by ID.
    pub fn get_provider(&self, provider_id: &str) -> Option<&ProviderInfo> {
        self.providers.iter().find(|p| p.id == provider_id)
    }

    /// List models from a specific provider.
    pub fn models_by_provider(&self, provider: &str) -> Vec<&ModelCatalogEntry> {
        self.models
            .iter()
            .filter(|m| m.provider == provider)
            .collect()
    }

    /// List models that are available (from configured providers only).
    pub fn available_models(&self) -> Vec<&ModelCatalogEntry> {
        let configured: Vec<&str> = self
            .providers
            .iter()
            .filter(|p| p.auth_status != AuthStatus::Missing)
            .map(|p| p.id.as_str())
            .collect();
        self.models
            .iter()
            .filter(|m| configured.contains(&m.provider.as_str()))
            .collect()
    }

    /// Get pricing for a model: (input_cost_per_million, output_cost_per_million).
    pub fn pricing(&self, model_id: &str) -> Option<(f64, f64)> {
        self.find_model(model_id)
            .map(|m| (m.input_cost_per_m, m.output_cost_per_m))
    }

    /// List all alias mappings.
    pub fn list_aliases(&self) -> &HashMap<String, String> {
        &self.aliases
    }

    /// List models filtered by tier.
    pub fn models_by_tier(&self, tier: ModelTier) -> Vec<&ModelCatalogEntry> {
        self.models.iter().filter(|m| m.tier == tier).collect()
    }

    /// Merge dynamically discovered models from a local provider.
    pub fn merge_discovered_models(&mut self, provider: &str, model_ids: &[String]) {
        let existing_ids: std::collections::HashSet<String> = self
            .models
            .iter()
            .filter(|m| m.provider == provider)
            .map(|m| m.id.to_lowercase())
            .collect();

        let mut added = 0usize;
        for id in model_ids {
            if existing_ids.contains(&id.to_lowercase()) {
                continue;
            }
            let display = format!("{} ({})", id, provider);
            self.models.push(ModelCatalogEntry {
                id: id.clone(),
                display_name: display,
                provider: provider.to_string(),
                tier: ModelTier::Local,
                context_window: 32_768,
                max_output_tokens: 4_096,
                input_cost_per_m: 0.0,
                output_cost_per_m: 0.0,
                supports_tools: true,
                supports_vision: false,
                supports_streaming: true,
                aliases: Vec::new(),
            });
            added += 1;
        }

        if added > 0 {
            if let Some(p) = self.providers.iter_mut().find(|p| p.id == provider) {
                p.model_count = self
                    .models
                    .iter()
                    .filter(|m| m.provider == provider)
                    .count();
            }
        }
    }
}

impl Default for ModelCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Builtin data
// ---------------------------------------------------------------------------

fn builtin_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            id: "anthropic".into(),
            display_name: "Anthropic".into(),
            api_key_env: "ANTHROPIC_API_KEY".into(),
            base_url: ANTHROPIC_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        // Kimi Code — Moonshot's Anthropic-compatible endpoint. Listed second
        // because it is the other first-class coding provider alongside Anthropic.
        ProviderInfo {
            id: "kimi".into(),
            display_name: "Kimi (Moonshot)".into(),
            api_key_env: "KIMI_API_KEY".into(),
            base_url: KIMI_CODE_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        ProviderInfo {
            id: "deepseek".into(),
            display_name: "DeepSeek".into(),
            api_key_env: "DEEPSEEK_API_KEY".into(),
            base_url: DEEPSEEK_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        ProviderInfo {
            id: "minimax".into(),
            display_name: "MiniMax".into(),
            api_key_env: "MINIMAX_API_KEY".into(),
            base_url: MINIMAX_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        ProviderInfo {
            id: "zhipu".into(),
            display_name: "Zhipu AI (GLM)".into(),
            api_key_env: "ZHIPU_API_KEY".into(),
            base_url: ZHIPU_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        ProviderInfo {
            id: "openrouter".into(),
            display_name: "OpenRouter".into(),
            api_key_env: "OPENROUTER_API_KEY".into(),
            base_url: OPENROUTER_BASE_URL.into(),
            key_required: true,
            auth_status: AuthStatus::Missing,
            model_count: 0,
        },
        ProviderInfo {
            id: "ollama".into(),
            display_name: "Ollama".into(),
            api_key_env: "OLLAMA_API_KEY".into(),
            base_url: OLLAMA_BASE_URL.into(),
            key_required: false,
            auth_status: AuthStatus::NotRequired,
            model_count: 0,
        },
    ]
}

fn builtin_aliases() -> HashMap<String, String> {
    let pairs = [
        // Anthropic
        ("sonnet", "claude-sonnet-4-20250514"),
        ("claude-sonnet", "claude-sonnet-4-20250514"),
        ("haiku", "claude-haiku-4-5-20251001"),
        ("claude-haiku", "claude-haiku-4-5-20251001"),
        ("opus", "claude-opus-4-20250514"),
        ("claude-opus", "claude-opus-4-20250514"),
        // Kimi (all names resolve to the single canonical model Kimi Code exposes).
        ("kimi", "kimi-for-coding"),
        ("k2", "kimi-for-coding"),
        ("k2-thinking", "kimi-for-coding"),
        ("kimi-k2-thinking", "kimi-for-coding"),
        ("kimi-code", "kimi-for-coding"),
        // DeepSeek
        ("deepseek", "deepseek-chat"),
        ("deepseek-v3", "deepseek-chat"),
        ("deepseek-r1", "deepseek-reasoner"),
        // Zhipu / GLM
        ("glm", "glm-4-plus"),
        ("glm-4", "glm-4-plus"),
        ("glm-4.6", "glm-4-plus"),
        // MiniMax
        ("minimax", "MiniMax-M2.7"),
    ];
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), v.to_string()))
        .collect()
}

fn builtin_models() -> Vec<ModelCatalogEntry> {
    vec![
        // ══════════════════════════════════════════════════════════════
        // Anthropic (3)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "claude-opus-4-20250514".into(),
            display_name: "Claude Opus 4".into(),
            provider: "anthropic".into(),
            tier: ModelTier::Frontier,
            context_window: 200_000,
            max_output_tokens: 32_000,
            input_cost_per_m: 15.0,
            output_cost_per_m: 75.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec!["opus".into(), "claude-opus".into()],
        },
        ModelCatalogEntry {
            id: "claude-sonnet-4-20250514".into(),
            display_name: "Claude Sonnet 4".into(),
            provider: "anthropic".into(),
            tier: ModelTier::Smart,
            context_window: 200_000,
            max_output_tokens: 64_000,
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec!["sonnet".into(), "claude-sonnet".into()],
        },
        ModelCatalogEntry {
            id: "claude-haiku-4-5-20251001".into(),
            display_name: "Claude Haiku 4.5".into(),
            provider: "anthropic".into(),
            tier: ModelTier::Fast,
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.25,
            output_cost_per_m: 1.25,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec!["haiku".into(), "claude-haiku".into()],
        },
        // ══════════════════════════════════════════════════════════════
        // Kimi Code (1 unified model — see comment in drivers/mod.rs)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "kimi-for-coding".into(),
            display_name: "Kimi K2 (Moonshot)".into(),
            provider: "kimi".into(),
            tier: ModelTier::Frontier,
            context_window: 262_144,
            max_output_tokens: 32_768,
            input_cost_per_m: 0.60,
            output_cost_per_m: 2.50,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec![
                "kimi".into(),
                "k2".into(),
                "k2-thinking".into(),
                "kimi-k2-thinking".into(),
                "kimi-code".into(),
            ],
        },
        // ══════════════════════════════════════════════════════════════
        // DeepSeek (2)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "deepseek-chat".into(),
            display_name: "DeepSeek V3".into(),
            provider: "deepseek".into(),
            tier: ModelTier::Smart,
            context_window: 64_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.27,
            output_cost_per_m: 1.10,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec!["deepseek".into(), "deepseek-v3".into()],
        },
        ModelCatalogEntry {
            id: "deepseek-reasoner".into(),
            display_name: "DeepSeek R1".into(),
            provider: "deepseek".into(),
            tier: ModelTier::Frontier,
            context_window: 64_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.55,
            output_cost_per_m: 2.19,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec!["deepseek-r1".into()],
        },
        // ══════════════════════════════════════════════════════════════
        // MiniMax (2)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "MiniMax-M2.7".into(),
            display_name: "MiniMax M2.7".into(),
            provider: "minimax".into(),
            tier: ModelTier::Frontier,
            context_window: 1_000_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.50,
            output_cost_per_m: 2.00,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec!["minimax".into()],
        },
        ModelCatalogEntry {
            id: "MiniMax-M1".into(),
            display_name: "MiniMax M1".into(),
            provider: "minimax".into(),
            tier: ModelTier::Smart,
            context_window: 1_000_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.30,
            output_cost_per_m: 1.10,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec![],
        },
        // ══════════════════════════════════════════════════════════════
        // Zhipu AI / GLM (2)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "glm-4-plus".into(),
            display_name: "GLM-4 Plus".into(),
            provider: "zhipu".into(),
            tier: ModelTier::Frontier,
            context_window: 128_000,
            max_output_tokens: 4_096,
            input_cost_per_m: 1.50,
            output_cost_per_m: 5.00,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec!["glm".into(), "glm-4".into(), "glm-4.6".into()],
        },
        ModelCatalogEntry {
            id: "glm-4-flash".into(),
            display_name: "GLM-4 Flash".into(),
            provider: "zhipu".into(),
            tier: ModelTier::Fast,
            context_window: 128_000,
            max_output_tokens: 4_096,
            input_cost_per_m: 0.10,
            output_cost_per_m: 0.10,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec![],
        },
        // ══════════════════════════════════════════════════════════════
        // OpenRouter (universal gateway — a few popular entry points)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "anthropic/claude-sonnet-4".into(),
            display_name: "Claude Sonnet 4 via OpenRouter".into(),
            provider: "openrouter".into(),
            tier: ModelTier::Smart,
            context_window: 200_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 3.0,
            output_cost_per_m: 15.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec![],
        },
        ModelCatalogEntry {
            id: "openai/gpt-4o".into(),
            display_name: "GPT-4o via OpenRouter".into(),
            provider: "openrouter".into(),
            tier: ModelTier::Smart,
            context_window: 128_000,
            max_output_tokens: 16_384,
            input_cost_per_m: 2.5,
            output_cost_per_m: 10.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec![],
        },
        ModelCatalogEntry {
            id: "google/gemini-2.5-pro".into(),
            display_name: "Gemini 2.5 Pro via OpenRouter".into(),
            provider: "openrouter".into(),
            tier: ModelTier::Frontier,
            context_window: 2_000_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 1.25,
            output_cost_per_m: 5.0,
            supports_tools: true,
            supports_vision: true,
            supports_streaming: true,
            aliases: vec![],
        },
        ModelCatalogEntry {
            id: "openrouter/auto".into(),
            display_name: "OpenRouter Auto (best-effort)".into(),
            provider: "openrouter".into(),
            tier: ModelTier::Smart,
            context_window: 128_000,
            max_output_tokens: 8_192,
            input_cost_per_m: 1.0,
            output_cost_per_m: 3.0,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec![],
        },
        // ══════════════════════════════════════════════════════════════
        // Ollama (local — a few popular defaults, auto-discovered too)
        // ══════════════════════════════════════════════════════════════
        ModelCatalogEntry {
            id: "llama3.2".into(),
            display_name: "Llama 3.2 (Ollama)".into(),
            provider: "ollama".into(),
            tier: ModelTier::Local,
            context_window: 131_072,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.0,
            output_cost_per_m: 0.0,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec!["llama".into()],
        },
        ModelCatalogEntry {
            id: "qwen2.5-coder".into(),
            display_name: "Qwen 2.5 Coder (Ollama)".into(),
            provider: "ollama".into(),
            tier: ModelTier::Local,
            context_window: 32_768,
            max_output_tokens: 8_192,
            input_cost_per_m: 0.0,
            output_cost_per_m: 0.0,
            supports_tools: true,
            supports_vision: false,
            supports_streaming: true,
            aliases: vec![],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_catalog_has_expected_providers() {
        let catalog = ModelCatalog::new();
        assert_eq!(catalog.list_providers().len(), 7);
        for id in [
            "anthropic",
            "kimi",
            "deepseek",
            "minimax",
            "zhipu",
            "openrouter",
            "ollama",
        ] {
            assert!(catalog.get_provider(id).is_some(), "missing provider: {id}");
        }
    }

    #[test]
    fn test_deleted_providers_gone() {
        // Regression: confirm v0.7.0 trimmed the list correctly.
        let catalog = ModelCatalog::new();
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
            "bedrock",
            "vllm",
            "lmstudio",
        ] {
            assert!(
                catalog.get_provider(removed).is_none(),
                "deleted provider still registered: {removed}"
            );
        }
    }

    #[test]
    fn test_kimi_single_model() {
        let catalog = ModelCatalog::new();
        let kimi = catalog.models_by_provider("kimi");
        assert_eq!(kimi.len(), 1);
        assert_eq!(kimi[0].id, "kimi-for-coding");
        assert!(kimi[0].supports_vision);
        assert!(kimi[0].supports_tools);
    }

    #[test]
    fn test_kimi_aliases_resolve() {
        let catalog = ModelCatalog::new();
        for alias in [
            "kimi",
            "k2",
            "k2-thinking",
            "kimi-k2-thinking",
            "kimi-code",
            "kimi-for-coding",
        ] {
            let m = catalog
                .find_model(alias)
                .unwrap_or_else(|| panic!("alias `{alias}` must resolve"));
            assert_eq!(m.id, "kimi-for-coding");
        }
    }

    #[test]
    fn test_anthropic_aliases_resolve() {
        let catalog = ModelCatalog::new();
        for (alias, expected) in [
            ("sonnet", "claude-sonnet-4-20250514"),
            ("haiku", "claude-haiku-4-5-20251001"),
            ("opus", "claude-opus-4-20250514"),
        ] {
            let m = catalog.find_model(alias).unwrap();
            assert_eq!(m.id, expected);
        }
    }

    #[test]
    fn test_glm_alias_resolves() {
        let catalog = ModelCatalog::new();
        let m = catalog.find_model("glm").unwrap();
        assert_eq!(m.id, "glm-4-plus");
        assert_eq!(m.provider, "zhipu");
    }

    #[test]
    fn test_deepseek_aliases() {
        let catalog = ModelCatalog::new();
        assert_eq!(catalog.find_model("deepseek").unwrap().id, "deepseek-chat");
        assert_eq!(
            catalog.find_model("deepseek-r1").unwrap().id,
            "deepseek-reasoner"
        );
    }

    #[test]
    fn test_pricing_lookup() {
        let catalog = ModelCatalog::new();
        let (i, o) = catalog.pricing("claude-sonnet-4-20250514").unwrap();
        assert!((i - 3.0).abs() < 0.01);
        assert!((o - 15.0).abs() < 0.01);
        // Kimi pricing
        let (ki, ko) = catalog.pricing("kimi-for-coding").unwrap();
        assert!((ki - 0.60).abs() < 0.01);
        assert!((ko - 2.50).abs() < 0.01);
    }

    #[test]
    fn test_ollama_is_local_tier() {
        let catalog = ModelCatalog::new();
        let local = catalog.models_by_tier(ModelTier::Local);
        assert!(!local.is_empty());
        assert!(local.iter().all(|m| m.provider == "ollama"));
    }

    #[test]
    fn test_ollama_no_key_required() {
        let catalog = ModelCatalog::new();
        let p = catalog.get_provider("ollama").unwrap();
        assert!(!p.key_required);
    }

    #[test]
    fn test_merge_discovered_models_adds_new() {
        let mut catalog = ModelCatalog::new();
        let before = catalog.models_by_provider("ollama").len();
        catalog.merge_discovered_models(
            "ollama",
            &["codestral:latest".to_string(), "deepseek:7b".to_string()],
        );
        let after = catalog.models_by_provider("ollama").len();
        assert_eq!(after, before + 2);
        let m = catalog.find_model("deepseek:7b").unwrap();
        assert_eq!(m.tier, ModelTier::Local);
        assert!((m.input_cost_per_m).abs() < f64::EPSILON);
    }

    #[test]
    fn test_merge_discovered_models_skips_existing() {
        let mut catalog = ModelCatalog::new();
        let before = catalog.list_models().len();
        catalog.merge_discovered_models("ollama", &["llama3.2".to_string()]);
        assert_eq!(catalog.list_models().len(), before);
    }

    #[test]
    fn test_default_creates_valid_catalog() {
        let catalog = ModelCatalog::default();
        assert!(!catalog.list_models().is_empty());
        assert!(!catalog.list_providers().is_empty());
    }

    #[test]
    fn test_find_model_case_insensitive() {
        let catalog = ModelCatalog::new();
        assert!(catalog.find_model("SONNET").is_some());
        assert!(catalog.find_model("Kimi").is_some());
    }

    #[test]
    fn test_openrouter_has_gateway_models() {
        let catalog = ModelCatalog::new();
        let or = catalog.models_by_provider("openrouter");
        assert!(or.len() >= 3);
        assert!(or.iter().any(|m| m.id == "openrouter/auto"));
    }
}
