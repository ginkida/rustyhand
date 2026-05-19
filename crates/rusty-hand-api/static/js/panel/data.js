// Mock telemetry/data for RustyHand control panel.

window.RH_DATA = (() => {
  const agents = [
    { id: "rusty",          name: "rusty",            emoji: "🦀", group: "system",     state: "idle",    model: "mock-model",              provider: "mock",      messages: 3,    cost: 0.00,  last: "Welcome! Try the demo pipeline.",   updated: "now",       hue: 22  },
    { id: "recruiter-01",   name: "recruiter-01",     emoji: "🎯", group: "sales",      state: "running", model: "claude-sonnet-4-6", provider: "anthropic", messages: 412,  cost: 4.21,  last: "Found 12 candidates matching senior-rust filter…", updated: "2m",  hue: 200 },
    { id: "social-media",   name: "social-media",     emoji: "📣", group: "marketing",  state: "running", model: "kimi-for-coding",        provider: "kimi",      messages: 1284, cost: 0.84,  last: "Drafted 4 posts for Wednesday queue.",          updated: "11m", hue: 295 },
    { id: "security-audit", name: "security-auditor", emoji: "🔒", group: "ops",        state: "waiting", model: "claude-opus-4-7", provider: "anthropic", messages: 88,   cost: 12.10, last: "Awaiting approval to rotate prod API key.",     updated: "4h",  hue: 12  },
    { id: "log-analyzer",   name: "log-analyzer",     emoji: "📈", group: "ops",        state: "running", model: "deepseek-v4-flash",      provider: "deepseek",  messages: 24010,cost: 0.31,  last: "Indexed 1.2M lines from prod-api-3.",            updated: "30s", hue: 150 },
    { id: "weekly-digest",  name: "weekly-digest",    emoji: "📰", group: "ops",        state: "scheduled",model: "claude-sonnet-4-6", provider: "anthropic", messages: 8,    cost: 0.42,  last: "Next run: Mon 09:00 UTC.",                       updated: "1d",  hue: 88  },
    { id: "tutor-rust",     name: "tutor-rust",       emoji: "🎓", group: "personal",   state: "idle",    model: "claude-haiku-4-5-20251001", provider: "anthropic", messages: 96,   cost: 0.05,  last: "Quizzed me on Pin<&mut T>. Got 7/10.",          updated: "3h",  hue: 50  },
    { id: "home-auto",      name: "home-automation",  emoji: "🏠", group: "personal",   state: "running", model: "llama3.2",              provider: "ollama",    messages: 5621, cost: 0.00,  last: "Lights dimmed at sunset.",                       updated: "1m",  hue: 250 },
    { id: "diagnostic",     name: "diagnostic",       emoji: "🩻", group: "system",     state: "error",   model: "claude-sonnet-4-6", provider: "anthropic", messages: 3,    cost: 0.00,  last: "Circuit breaker OPEN — auth profile cooldown 412s", updated: "12m", hue: 12 },
    { id: "coordinator",    name: "coordinator",      emoji: "🧭", group: "system",     state: "idle",    model: "claude-sonnet-4-6", provider: "anthropic", messages: 14,   cost: 0.18,  last: "Delegated 3 tasks to capability-builder.",       updated: "20m", hue: 22  },
  ];

  const workflows = [
    { id: "demo-pipeline",     name: "demo-pipeline",     steps: 2,  runs_24h: 3,   p50_ms: 412,  ok: 100, kind: "sample" },
    { id: "leadgen-funnel",    name: "leadgen-funnel",    steps: 6,  runs_24h: 142, p50_ms: 1840, ok: 97,  kind: "pipeline" },
    { id: "log-triage",        name: "log-triage",        steps: 4,  runs_24h: 88,  p50_ms: 920,  ok: 100, kind: "pipeline" },
    { id: "weekly-digest-gen", name: "weekly-digest-gen", steps: 5,  runs_24h: 0,   p50_ms: 6100, ok: 100, kind: "cron" },
    { id: "rotate-secrets",    name: "rotate-secrets",    steps: 3,  runs_24h: 0,   p50_ms: 240,  ok: 100, kind: "manual" },
  ];

  const cron = [
    { id: "demo-daily-ping",    cron: "0 9 * * *",     action: "agent:rusty.message",      next: "in 7h 12m",  fires: 0,    enabled: false },
    { id: "weekly-digest",      cron: "0 9 * * 1",     action: "workflow:weekly-digest-gen", next: "Mon 09:00", fires: 12,   enabled: true  },
    { id: "log-rotation-watch", cron: "*/15 * * * *",  action: "agent:log-analyzer.event", next: "in 3m",      fires: 4022, enabled: true  },
    { id: "price-sweep",        cron: "0 */6 * * *",   action: "workflow:price-sweep",     next: "in 2h 38m",  fires: 88,   enabled: true  },
    { id: "weekend-quiet",      cron: "0 22 * * 5",    action: "system:agent_pause",       next: "Fri 22:00",  fires: 5,    enabled: true  },
  ];

  const triggers = [
    { id: "tg-recruiter",   kind: "telegram", target: "recruiter-01",   fired: 412, last: "2m",  status: "active" },
    { id: "webhook-leads",  kind: "webhook",  target: "leadgen-funnel", fired: 1280,last: "30s", status: "active" },
    { id: "discord-ops",    kind: "discord",  target: "log-analyzer",   fired: 88,  last: "11m", status: "active" },
    { id: "spawn-on-cve",   kind: "event",    target: "security-audit", fired: 4,   last: "1d",  status: "armed"  },
  ];

  const channels = [
    { id: "tg-primary",  kind: "telegram", handle: "@rusty_ops_bot",     bound: ["rusty","recruiter-01","social-media"], state: "live",      rate: "184 msg/h" },
    { id: "discord-ops", kind: "discord",  handle: "ops-channel",        bound: ["log-analyzer"],                         state: "live",      rate: "12 msg/h"  },
    { id: "slack-int",   kind: "slack",    handle: "#integrations",      bound: ["weekly-digest"],                        state: "idle",      rate: "0 msg/h"   },
    { id: "tg-personal", kind: "telegram", handle: "@home_helper_bot",   bound: ["home-auto"],                            state: "live",      rate: "6 msg/h"   },
  ];

  const approvals = [
    { id: "ap-281", agent: "security-audit", action: "rotate prod API key (anthropic-primary)", risk: "high",   age: "12m" },
    { id: "ap-280", agent: "recruiter-01",   action: "send outreach to 42 candidates",          risk: "medium", age: "1h"  },
    { id: "ap-279", agent: "social-media",   action: "post draft to X (rusty_devhub)",          risk: "low",    age: "4h"  },
  ];

  // Last 24 1-hour buckets — cost in USD
  const costSeries = [
    0.12, 0.08, 0.04, 0.03, 0.02, 0.05, 0.18, 0.41, 0.62, 0.74, 0.91, 1.20,
    1.10, 1.04, 0.88, 0.76, 0.62, 0.58, 0.71, 0.83, 0.94, 1.18, 1.42, 1.21,
  ];

  const modelSpend = [
    { model: "claude-sonnet-4-6", spend: 6.84, share: 0.46 },
    { model: "claude-opus-4-7",   spend: 3.91, share: 0.26 },
    { model: "kimi-for-coding",          spend: 1.42, share: 0.10 },
    { model: "deepseek-v4-flash",        spend: 0.31, share: 0.02 },
    { model: "claude-haiku-4-5-20251001", spend: 0.74, share: 0.05 },
    { model: "llama3.2",                 spend: 0.00, share: 0.11 },
  ];

  const audit = [
    { time: "12:41:08", hash: "9a3f2b7e", parent: "f1ac…", actor: "operator@local", action: "agent.spawn",            payload: "id=recruiter-01 model=claude-sonnet-4-6" },
    { time: "12:40:54", hash: "f1ac8d12", parent: "c84e…", actor: "system",         action: "trigger.fired",          payload: "id=webhook-leads count=1281" },
    { time: "12:40:21", hash: "c84e6611", parent: "5b09…", actor: "recruiter-01",   action: "tool.web_search",        payload: "query=\"senior rust engineer remote\"" },
    { time: "12:39:55", hash: "5b095fa1", parent: "21d3…", actor: "log-analyzer",   action: "memory.ingest",          payload: "doc=prod-api-3.log chunks=4019" },
    { time: "12:39:30", hash: "21d30aaa", parent: "0e44…", actor: "security-audit", action: "approval.requested",     payload: "ap-281 risk=high" },
    { time: "12:38:51", hash: "0e44c1b2", parent: "9c12…", actor: "operator@local", action: "workflow.run",           payload: "id=leadgen-funnel input.sz=2.1kb" },
    { time: "12:37:12", hash: "9c12f70d", parent: "bb47…", actor: "system",         action: "cron.tick",              payload: "next=log-rotation-watch in=15m" },
    { time: "12:36:00", hash: "bb47e018", parent: "—",     actor: "system",         action: "kernel.boot",            payload: "schema=v8 demo=on" },
  ];

  // tool-trace sample for chat replay
  const chatScript = [
    { role: "user",   text: "Find me 10 senior Rust engineers open to remote, prioritize ones publishing on async runtimes." },
    { role: "trace",  tool: "web_search", arg: "senior rust engineer remote async",    state: "running" },
    { role: "trace",  tool: "web_search", arg: "senior rust engineer remote async",    state: "done", elapsed: "2.1s" },
    { role: "trace",  tool: "kg_query",   arg: "kind=person tag=rust",                  state: "running" },
    { role: "trace",  tool: "kg_query",   arg: "kind=person tag=rust",                  state: "done", elapsed: "0.4s" },
    { role: "trace",  tool: "rank",       arg: "weights=[publishes,recency,async]",     state: "done", elapsed: "0.1s" },
    { role: "agent",  text: "Found 12 candidates. Top 10 (ranked by async-runtime authorship + recency):\n\n1. **A. Linder** — Tokio contributor, last commit 3d ago. Berlin → open to remote.\n2. **K. Mendez** — author of async-std comparison post (Apr 2026). LATAM.\n3. **R. Holloway** — wrote glommio benchmark series. Dublin.\n\n…and 7 more. Want me to draft an outreach for the top 3?" },
  ];

  return { agents, workflows, cron, triggers, channels, approvals, costSeries, modelSpend, audit, chatScript };
})();
