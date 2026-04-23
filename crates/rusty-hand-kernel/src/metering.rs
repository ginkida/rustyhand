//! Metering engine — tracks LLM cost and enforces spending quotas.

use rusty_hand_memory::usage::{ModelUsage, UsageRecord, UsageStore, UsageSummary};
use rusty_hand_types::agent::{AgentId, ResourceQuota};
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use std::sync::Arc;

/// The metering engine tracks usage cost and enforces quota limits.
pub struct MeteringEngine {
    /// Persistent usage store (SQLite-backed).
    store: Arc<UsageStore>,
}

impl MeteringEngine {
    /// Create a new metering engine with the given usage store.
    pub fn new(store: Arc<UsageStore>) -> Self {
        Self { store }
    }

    /// Record a usage event (persists to SQLite).
    pub fn record(&self, record: &UsageRecord) -> RustyHandResult<()> {
        self.store.record(record)
    }

    /// Check if an agent is within its spending quotas (hourly, daily, monthly).
    /// Returns Ok(()) if under all quotas, or QuotaExceeded error if over any.
    pub fn check_quota(&self, agent_id: AgentId, quota: &ResourceQuota) -> RustyHandResult<()> {
        // Hourly check
        if quota.max_cost_per_hour_usd > 0.0 {
            let hourly_cost = self.store.query_hourly(agent_id)?;
            if hourly_cost >= quota.max_cost_per_hour_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Agent {} exceeded hourly cost quota: ${:.4} / ${:.4}",
                    agent_id, hourly_cost, quota.max_cost_per_hour_usd
                )));
            }
        }

        // Daily check
        if quota.max_cost_per_day_usd > 0.0 {
            let daily_cost = self.store.query_daily(agent_id)?;
            if daily_cost >= quota.max_cost_per_day_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Agent {} exceeded daily cost quota: ${:.4} / ${:.4}",
                    agent_id, daily_cost, quota.max_cost_per_day_usd
                )));
            }
        }

        // Monthly check
        if quota.max_cost_per_month_usd > 0.0 {
            let monthly_cost = self.store.query_monthly(agent_id)?;
            if monthly_cost >= quota.max_cost_per_month_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Agent {} exceeded monthly cost quota: ${:.4} / ${:.4}",
                    agent_id, monthly_cost, quota.max_cost_per_month_usd
                )));
            }
        }

        Ok(())
    }

    /// Check global budget limits (across all agents).
    pub fn check_global_budget(
        &self,
        budget: &rusty_hand_types::config::BudgetConfig,
    ) -> RustyHandResult<()> {
        if budget.max_hourly_usd > 0.0 {
            let cost = self.store.query_global_hourly()?;
            if cost >= budget.max_hourly_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Global hourly budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_hourly_usd
                )));
            }
        }

        if budget.max_daily_usd > 0.0 {
            let cost = self.store.query_today_cost()?;
            if cost >= budget.max_daily_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Global daily budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_daily_usd
                )));
            }
        }

        if budget.max_monthly_usd > 0.0 {
            let cost = self.store.query_global_monthly()?;
            if cost >= budget.max_monthly_usd {
                return Err(RustyHandError::QuotaExceeded(format!(
                    "Global monthly budget exceeded: ${:.4} / ${:.4}",
                    cost, budget.max_monthly_usd
                )));
            }
        }

        Ok(())
    }

    /// Get budget status — current spend vs limits for all time windows.
    pub fn budget_status(&self, budget: &rusty_hand_types::config::BudgetConfig) -> BudgetStatus {
        let hourly = self.store.query_global_hourly().unwrap_or(0.0);
        let daily = self.store.query_today_cost().unwrap_or(0.0);
        let monthly = self.store.query_global_monthly().unwrap_or(0.0);

        BudgetStatus {
            hourly_spend: hourly,
            hourly_limit: budget.max_hourly_usd,
            hourly_pct: if budget.max_hourly_usd > 0.0 {
                hourly / budget.max_hourly_usd
            } else {
                0.0
            },
            daily_spend: daily,
            daily_limit: budget.max_daily_usd,
            daily_pct: if budget.max_daily_usd > 0.0 {
                daily / budget.max_daily_usd
            } else {
                0.0
            },
            monthly_spend: monthly,
            monthly_limit: budget.max_monthly_usd,
            monthly_pct: if budget.max_monthly_usd > 0.0 {
                monthly / budget.max_monthly_usd
            } else {
                0.0
            },
            alert_threshold: budget.alert_threshold,
        }
    }

    /// Get a usage summary, optionally filtered by agent.
    pub fn get_summary(&self, agent_id: Option<AgentId>) -> RustyHandResult<UsageSummary> {
        self.store.query_summary(agent_id)
    }

    /// Get usage grouped by model.
    pub fn get_by_model(&self) -> RustyHandResult<Vec<ModelUsage>> {
        self.store.query_by_model()
    }

    /// Estimate the cost of an LLM call based on model and token counts.
    ///
    /// Pricing table (approximate, per million tokens) for the 7 v0.7.0 providers:
    ///
    /// | Model Family          | Input $/M | Output $/M |
    /// |-----------------------|-----------|------------|
    /// | claude-haiku          |     0.25  |      1.25  |
    /// | claude-sonnet         |     3.00  |     15.00  |
    /// | claude-opus           |    15.00  |     75.00  |
    /// | kimi-* (Kimi Code)    |     0.60  |      2.50  |
    /// | deepseek-chat         |     0.27  |      1.10  |
    /// | deepseek-reasoner/r1  |     0.55  |      2.19  |
    /// | MiniMax-M2            |     0.50  |      2.00  |
    /// | MiniMax-M1            |     0.30  |      1.10  |
    /// | glm-4-flash           |     0.10  |      0.10  |
    /// | glm-*                 |     1.50  |      5.00  |
    /// | openrouter passthru   | varies (upstream rate)  |
    /// | ollama/llama/qwen-*   |     0.00  |      0.00  |
    /// | Default (unknown)     |     1.00  |      3.00  |
    pub fn estimate_cost(model: &str, input_tokens: u64, output_tokens: u64) -> f64 {
        let model_lower = model.to_lowercase();
        let (input_per_m, output_per_m) = estimate_cost_rates(&model_lower);

        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_per_m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_per_m;
        input_cost + output_cost
    }

    /// Estimate cost using the model catalog as the pricing source.
    ///
    /// Falls back to the default rate ($1/$3 per million) if the model is not
    /// found in the catalog.
    pub fn estimate_cost_with_catalog(
        catalog: &rusty_hand_runtime::model_catalog::ModelCatalog,
        model: &str,
        input_tokens: u64,
        output_tokens: u64,
    ) -> f64 {
        let (input_per_m, output_per_m) = catalog.pricing(model).unwrap_or((1.0, 3.0));
        let input_cost = (input_tokens as f64 / 1_000_000.0) * input_per_m;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * output_per_m;
        input_cost + output_cost
    }

    /// Clean up old usage records.
    pub fn cleanup(&self, days: u32) -> RustyHandResult<usize> {
        self.store.cleanup_old(days)
    }
}

/// Budget status snapshot — current spend vs limits for all time windows.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetStatus {
    pub hourly_spend: f64,
    pub hourly_limit: f64,
    pub hourly_pct: f64,
    pub daily_spend: f64,
    pub daily_limit: f64,
    pub daily_pct: f64,
    pub monthly_spend: f64,
    pub monthly_limit: f64,
    pub monthly_pct: f64,
    pub alert_threshold: f64,
}

/// Returns (input_per_million, output_per_million) pricing for a model.
///
/// Covers only the seven v0.7.0 providers (anthropic, kimi, deepseek,
/// minimax, zhipu/glm, openrouter gateway, ollama). Ollama is local
/// and free. For an unknown model string we fall back to a conservative
/// default rather than crash.
///
/// Order matters: more specific patterns must come before generic ones.
fn estimate_cost_rates(model: &str) -> (f64, f64) {
    // ── Anthropic ──────────────────────────────────────────────
    if model.contains("haiku") {
        return (0.25, 1.25);
    }
    if model.contains("opus") {
        return (15.0, 75.0);
    }
    if model.contains("sonnet") {
        return (3.0, 15.0);
    }

    // ── Kimi Code (Anthropic-compat) — one backend, many aliases ─
    if model.contains("kimi-for-coding")
        || model.contains("kimi-k2-thinking")
        || model.contains("kimi-code")
        || model.contains("k2-thinking")
        || model == "kimi"
        || model == "k2"
    {
        return (0.60, 2.50);
    }

    // ── DeepSeek ───────────────────────────────────────────────
    if model.contains("deepseek-reasoner") || model.contains("deepseek-r1") {
        return (0.55, 2.19);
    }
    if model.contains("deepseek") {
        return (0.27, 1.10);
    }

    // ── MiniMax ────────────────────────────────────────────────
    if model.contains("minimax-m2") || model.to_lowercase().contains("minimax-m2") {
        return (0.50, 2.00);
    }
    if model.contains("minimax-m1") || model.to_lowercase().contains("minimax-m1") {
        return (0.30, 1.10);
    }
    if model.to_lowercase().contains("minimax") {
        return (1.00, 3.00);
    }

    // ── Zhipu / GLM ────────────────────────────────────────────
    if model.contains("glm-4-flash") {
        return (0.10, 0.10);
    }
    if model.contains("glm") {
        return (1.50, 5.00);
    }

    // ── OpenRouter gateway passthroughs (prefix routing) ──────
    // OpenRouter charges upstream + 5% overhead; we approximate with
    // upstream's published rates and round up slightly.
    if model.starts_with("openai/") || model.starts_with("anthropic/") {
        return (3.0, 15.0);
    }
    if model.starts_with("google/") {
        return (1.25, 5.0);
    }
    if model == "openrouter/auto" {
        return (1.0, 3.0);
    }

    // ── Ollama (local, free) ───────────────────────────────────
    if model.contains("llama") || model.contains("qwen2.5-coder") {
        return (0.0, 0.0);
    }

    // ── Default (conservative) for unknown custom OpenAI-compat ─
    (1.0, 3.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_memory::MemorySubstrate;

    fn setup() -> MeteringEngine {
        let substrate = MemorySubstrate::open_in_memory(0.1).unwrap();
        let store = Arc::new(UsageStore::new(substrate.usage_conn()));
        MeteringEngine::new(store)
    }

    #[test]
    fn test_record_and_check_quota_under() {
        let engine = setup();
        let agent_id = AgentId::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 1.0,
            ..Default::default()
        };

        engine
            .record(&UsageRecord {
                agent_id,
                model: "claude-haiku".to_string(),
                input_tokens: 100,
                output_tokens: 50,
                cost_usd: 0.001,
                tool_calls: 0,
            })
            .unwrap();

        assert!(engine.check_quota(agent_id, &quota).is_ok());
    }

    #[test]
    fn test_check_quota_exceeded() {
        let engine = setup();
        let agent_id = AgentId::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 0.01,
            ..Default::default()
        };

        engine
            .record(&UsageRecord {
                agent_id,
                model: "claude-sonnet".to_string(),
                input_tokens: 10000,
                output_tokens: 5000,
                cost_usd: 0.05,
                tool_calls: 0,
            })
            .unwrap();

        let result = engine.check_quota(agent_id, &quota);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("exceeded hourly cost quota"));
    }

    #[test]
    fn test_check_quota_zero_limit_skipped() {
        let engine = setup();
        let agent_id = AgentId::new();
        let quota = ResourceQuota {
            max_cost_per_hour_usd: 0.0,
            ..Default::default()
        };

        // Even with high usage, a zero limit means no enforcement
        engine
            .record(&UsageRecord {
                agent_id,
                model: "claude-opus".to_string(),
                input_tokens: 100000,
                output_tokens: 50000,
                cost_usd: 100.0,
                tool_calls: 0,
            })
            .unwrap();

        assert!(engine.check_quota(agent_id, &quota).is_ok());
    }

    #[test]
    fn test_estimate_cost_haiku() {
        let cost = MeteringEngine::estimate_cost("claude-haiku-4-5-20251001", 1_000_000, 1_000_000);
        assert!((cost - 1.50).abs() < 0.01); // $0.25 + $1.25
    }

    #[test]
    fn test_estimate_cost_kimi_unified() {
        // Kimi Code has one backend model; all aliases must bill identically.
        let expected = 3.10; // $0.60 + $2.50 per million in + out
        for model in [
            "kimi-for-coding",
            "kimi-k2-thinking",
            "kimi-code",
            "k2-thinking",
            "kimi",
            "k2",
        ] {
            let cost = MeteringEngine::estimate_cost(model, 1_000_000, 1_000_000);
            assert!(
                (cost - expected).abs() < 0.01,
                "model `{model}` expected {expected}, got {cost}"
            );
        }
    }

    #[test]
    fn test_estimate_cost_kimi_does_not_fall_through_to_legacy() {
        // Regression guard: the Kimi match arm must precede the Moonshot fallback.
        let cost = MeteringEngine::estimate_cost("kimi-for-coding", 1_000_000, 1_000_000);
        assert!(cost > 3.0, "expected Kimi-tier pricing, got {cost}");
    }

    #[test]
    fn test_estimate_cost_sonnet() {
        let cost = MeteringEngine::estimate_cost("claude-sonnet-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01); // $3.00 + $15.00
    }

    #[test]
    fn test_estimate_cost_opus() {
        let cost = MeteringEngine::estimate_cost("claude-opus-4-20250514", 1_000_000, 1_000_000);
        assert!((cost - 90.0).abs() < 0.01); // $15.00 + $75.00
    }

    #[test]
    fn test_estimate_cost_deepseek_chat() {
        let cost = MeteringEngine::estimate_cost("deepseek-chat", 1_000_000, 1_000_000);
        assert!((cost - 1.37).abs() < 0.01); // $0.27 + $1.10
    }

    #[test]
    fn test_estimate_cost_deepseek_reasoner() {
        let cost = MeteringEngine::estimate_cost("deepseek-reasoner", 1_000_000, 1_000_000);
        assert!((cost - 2.74).abs() < 0.01); // $0.55 + $2.19
    }

    #[test]
    fn test_estimate_cost_minimax_m2() {
        let cost = MeteringEngine::estimate_cost("MiniMax-M2.7", 1_000_000, 1_000_000);
        assert!((cost - 2.50).abs() < 0.01); // $0.50 + $2.00
    }

    #[test]
    fn test_estimate_cost_minimax_m1() {
        let cost = MeteringEngine::estimate_cost("MiniMax-M1", 1_000_000, 1_000_000);
        assert!((cost - 1.40).abs() < 0.01); // $0.30 + $1.10
    }

    #[test]
    fn test_estimate_cost_glm() {
        let cost = MeteringEngine::estimate_cost("glm-4-plus", 1_000_000, 1_000_000);
        assert!((cost - 6.50).abs() < 0.01); // $1.50 + $5.00
    }

    #[test]
    fn test_estimate_cost_glm_flash() {
        let cost = MeteringEngine::estimate_cost("glm-4-flash", 1_000_000, 1_000_000);
        assert!((cost - 0.20).abs() < 0.01); // $0.10 + $0.10
    }

    #[test]
    fn test_estimate_cost_openrouter_anthropic_passthru() {
        let cost = MeteringEngine::estimate_cost("anthropic/claude-sonnet-4", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01); // OpenRouter rounds to upstream sonnet rates
    }

    #[test]
    fn test_estimate_cost_openrouter_auto() {
        let cost = MeteringEngine::estimate_cost("openrouter/auto", 1_000_000, 1_000_000);
        assert!((cost - 4.0).abs() < 0.01); // Conservative passthru estimate
    }

    #[test]
    fn test_estimate_cost_ollama_is_free() {
        let cost = MeteringEngine::estimate_cost("llama3.2", 1_000_000, 1_000_000);
        assert!(cost.abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_unknown_falls_back() {
        let cost = MeteringEngine::estimate_cost("my-custom-model", 1_000_000, 1_000_000);
        assert!((cost - 4.0).abs() < 0.01); // $1.00 + $3.00
    }

    #[test]
    fn test_estimate_cost_with_catalog() {
        let catalog = rusty_hand_runtime::model_catalog::ModelCatalog::new();
        // Sonnet: $3/M input, $15/M output
        let cost = MeteringEngine::estimate_cost_with_catalog(
            &catalog,
            "claude-sonnet-4-20250514",
            1_000_000,
            1_000_000,
        );
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_with_catalog_alias() {
        let catalog = rusty_hand_runtime::model_catalog::ModelCatalog::new();
        // "sonnet" alias should resolve to same pricing
        let cost =
            MeteringEngine::estimate_cost_with_catalog(&catalog, "sonnet", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_cost_with_catalog_unknown_uses_default() {
        let catalog = rusty_hand_runtime::model_catalog::ModelCatalog::new();
        // Unknown model falls back to $1/$3
        let cost = MeteringEngine::estimate_cost_with_catalog(
            &catalog,
            "totally-unknown-model",
            1_000_000,
            1_000_000,
        );
        assert!((cost - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_get_summary() {
        let engine = setup();
        let agent_id = AgentId::new();

        engine
            .record(&UsageRecord {
                agent_id,
                model: "haiku".to_string(),
                input_tokens: 500,
                output_tokens: 200,
                cost_usd: 0.005,
                tool_calls: 3,
            })
            .unwrap();

        let summary = engine.get_summary(Some(agent_id)).unwrap();
        assert_eq!(summary.call_count, 1);
        assert_eq!(summary.total_input_tokens, 500);
    }
}
