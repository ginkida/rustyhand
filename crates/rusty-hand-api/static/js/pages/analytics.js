/**
 * Analytics page — cost tracking, usage charts, and agent performance.
 *
 * Data sources:
 *   GET /api/usage/summary   — total tokens, cost, calls
 *   GET /api/usage/by-model  — breakdown by model
 *   GET /api/usage/daily     — 7-day daily breakdown
 *   GET /api/budget          — budget limits and current spend
 *   GET /api/budget/agents   — per-agent cost ranking
 */

function analyticsPage() {
    return {
        summary: null,
        byModel: [],
        daily: [],
        budget: null,
        agentCosts: [],
        loading: true,
        error: null,

        async init() {
            await this.refresh();
        },

        async refresh() {
            this.loading = true;
            this.error = null;
            try {
                const [summaryRes, modelRes, dailyRes, budgetRes, agentRes] = await Promise.all([
                    api.get('/api/usage/summary'),
                    api.get('/api/usage/by-model'),
                    api.get('/api/usage/daily'),
                    api.get('/api/budget'),
                    api.get('/api/budget/agents'),
                ]);
                this.summary = summaryRes;
                this.byModel = modelRes.models || modelRes || [];
                this.daily = dailyRes.daily || dailyRes || [];
                this.budget = budgetRes;
                this.agentCosts = agentRes.agents || agentRes || [];
            } catch (e) {
                this.error = e.message || 'Failed to load analytics';
            }
            this.loading = false;
        },

        // Format USD cost
        usd(value) {
            if (value == null) return '$0.00';
            return '$' + Number(value).toFixed(4);
        },

        // Format large numbers
        fmt(n) {
            if (n == null) return '0';
            if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + 'M';
            if (n >= 1_000) return (n / 1_000).toFixed(1) + 'K';
            return String(n);
        },

        // Budget usage percentage
        budgetPct(spent, limit) {
            if (!limit || limit <= 0) return null;
            return Math.min(100, Math.round((spent / limit) * 100));
        },

        // Simple ASCII bar for tables
        bar(value, max) {
            if (!max || max <= 0) return '';
            const pct = Math.min(100, Math.round((value / max) * 100));
            const filled = Math.round(pct / 5);
            return '\u2588'.repeat(filled) + '\u2591'.repeat(20 - filled) + ' ' + pct + '%';
        },

        // Max cost in agent ranking (for bar scaling)
        maxAgentCost() {
            if (!this.agentCosts.length) return 1;
            return Math.max(...this.agentCosts.map(a => a.total_cost || a.cost || 0), 0.0001);
        },

        // Max daily cost (for bar scaling)
        maxDailyCost() {
            if (!this.daily.length) return 1;
            return Math.max(...this.daily.map(d => d.cost || 0), 0.0001);
        },
    };
}
