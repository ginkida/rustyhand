//! Response-shape contract tests for the production HTTP router.
//!
//! These tests pin the JSON shapes that the dashboard and CLI read from each
//! endpoint. Almost every "fix: X reads wrong field" commit between v0.7.27
//! and v0.7.29 would have been caught by a test in this file — the symptom
//! pattern was that the API renamed/added/removed a field while the consumers
//! were silently reading `undefined`. Each assertion below is a contract that
//! some real consumer relies on.
//!
//! Boot strategy: full production router via `build_router()`, real kernel,
//! real axum on a random port, ollama as the LLM provider so no API key is
//! needed. No LLM calls are made — every endpoint exercised here is read-only
//! or returns control flow without ever touching the model.

use rusty_hand_api::server;
use rusty_hand_kernel::RustyHandKernel;
use rusty_hand_types::config::{DefaultModelConfig, KernelConfig};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::Arc;

struct TestServer {
    base_url: String,
    kernel: Arc<RustyHandKernel>,
    _tmp: tempfile::TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.kernel.shutdown();
    }
}

async fn start_test_server() -> Option<TestServer> {
    let tmp = tempfile::tempdir().ok()?;

    let mut config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        default_model: DefaultModelConfig {
            provider: "ollama".to_string(),
            model: "test-model".to_string(),
            api_key_env: "OLLAMA_API_KEY".to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    };
    // Disable network features that would try to bind to additional ports.
    config.network_enabled = false;
    config.pairing.enabled = false;

    let kernel = RustyHandKernel::boot_with_config(config).ok()?;
    let kernel = Arc::new(kernel);
    kernel.set_self_handle();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.ok()?;
    let addr = listener.local_addr().ok()?;
    let (router, _state) = server::build_router(kernel.clone(), addr).await;

    // SECURITY: the production server uses
    // `into_make_service_with_connect_info::<SocketAddr>()` to inject the
    // peer's IP into request extensions; the auth middleware needs that to
    // distinguish loopback from remote callers. Tests would otherwise be
    // rejected with 403 because the middleware can't see the peer IP.
    tokio::spawn(async move {
        let _ = axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await;
    });

    Some(TestServer {
        base_url: format!("http://{addr}"),
        kernel,
        _tmp: tmp,
    })
}

macro_rules! require_server {
    ($future:expr) => {
        match $future.await {
            Some(server) => server,
            None => {
                eprintln!("Skipping test: could not bring up test server");
                return;
            }
        }
    };
}

async fn get_json(base_url: &str, path: &str) -> Value {
    let url = format!("{base_url}{path}");
    let resp = reqwest::get(&url)
        .await
        .unwrap_or_else(|e| panic!("GET {url} failed: {e}"));
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .unwrap_or_else(|e| panic!("GET {url} body was not JSON: {e}"));
    assert!(status.is_success(), "GET {path} returned {status}: {body}");
    body
}

/// Helper: assert a JSON object has every named string field.
fn require_keys(v: &Value, keys: &[&str], context: &str) {
    let obj = v
        .as_object()
        .unwrap_or_else(|| panic!("{context}: expected object, got {v}"));
    for key in keys {
        assert!(
            obj.contains_key(*key),
            "{context}: missing field `{key}` in {v}"
        );
    }
}

const TEST_MANIFEST: &str = r#"
name = "shape-test-agent"
version = "0.1.0"
description = "Response shape test agent"
author = "test"
module = "builtin:chat"

[model]
provider = "ollama"
model = "test-model"
system_prompt = "You are a test agent."
temperature = 0.42
max_tokens = 1234

[capabilities]
"#;

async fn spawn_test_agent(server: &TestServer) -> String {
    let url = format!("{}/api/agents", server.base_url);
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .expect("spawn POST failed");
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .unwrap_or_else(|e| panic!("spawn body not JSON: {e}"));
    assert!(status.is_success(), "spawn returned {status}: {body}");
    body["agent_id"]
        .as_str()
        .expect("spawn response missing agent_id")
        .to_string()
}

// ---------------------------------------------------------------------------
// Endpoint shape contracts — one #[tokio::test] per endpoint, named so a
// failure tells you exactly which contract broke.
// ---------------------------------------------------------------------------

/// `GET /api/profiles` returns a bare JSON array (NOT wrapped in `{profiles: [...]}`).
/// The agents.js spawn-profile picker depends on this.
#[tokio::test]
async fn profiles_endpoint_returns_bare_array() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/profiles").await;
    let arr = body
        .as_array()
        .expect("/api/profiles must be a bare array (not an object envelope)");
    assert!(
        !arr.is_empty(),
        "/api/profiles should list at least one profile"
    );
    let first = &arr[0];
    require_keys(first, &["name", "tools"], "/api/profiles[0]");
}

/// `GET /api/agents` returns the paginated envelope and each agent exposes
/// the *flat* model fields the dashboard config form reads.
#[tokio::test]
async fn agents_envelope_contains_flat_model_fields() {
    let server = require_server!(start_test_server());
    spawn_test_agent(&server).await;

    let body = get_json(&server.base_url, "/api/agents").await;
    require_keys(
        &body,
        &["agents", "total", "offset", "limit"],
        "/api/agents",
    );

    let agents = body["agents"].as_array().expect("agents is array");
    assert!(!agents.is_empty(), "expected the spawned agent to appear");
    let agent = &agents[0];
    require_keys(
        agent,
        &[
            "id",
            "name",
            "state",
            "model_provider",
            "model_name",
            "model_temperature",
            "model_max_tokens",
            "model_thinking_enabled",
            "identity",
        ],
        "/api/agents[0]",
    );
    let temp = agent["model_temperature"]
        .as_f64()
        .expect("model_temperature must be a number");
    // The manifest stores temperature as f32, so the JSON round-trip widens
    // it back to f64 with float-precision noise — assert with a tolerance.
    assert!(
        (temp - 0.42).abs() < 1e-4,
        "model_temperature should be ~0.42, got {temp}"
    );
    assert_eq!(agent["model_max_tokens"].as_u64(), Some(1234));
    assert_eq!(agent["model_thinking_enabled"].as_bool(), Some(false));
}

/// `GET /api/triggers` returns a bare array (not wrapped).
#[tokio::test]
async fn triggers_endpoint_returns_bare_array() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/triggers").await;
    body.as_array().expect("/api/triggers must be a bare array");
}

/// `GET /api/workflows` returns a bare array (not wrapped).
#[tokio::test]
async fn workflows_endpoint_returns_bare_array() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/workflows").await;
    body.as_array()
        .expect("/api/workflows must be a bare array");
}

/// `GET /api/cron/jobs` returns `{jobs, total}`. The dashboard automation
/// page reads `data.jobs` and the CLI reads `body.jobs`.
#[tokio::test]
async fn cron_jobs_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/cron/jobs").await;
    require_keys(&body, &["jobs", "total"], "/api/cron/jobs");
    body["jobs"].as_array().expect("jobs is array");
}

/// `GET /api/approvals` envelope; entries always include `status: "pending"`
/// because `list_pending()` only ever returns pending items but the
/// `ApprovalRequest` struct itself has no `status` field. Without this
/// injection, the dashboard's `pendingCount` filter is always 0.
#[tokio::test]
async fn approvals_endpoint_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/approvals").await;
    require_keys(&body, &["approvals", "total"], "/api/approvals");
    body["approvals"].as_array().expect("approvals is array");
    // We can't easily seed a pending approval from an integration test
    // without an interactive driver, so we only guarantee the envelope.
    // The status-injection contract is exercised by the unit test in
    // routes::list_approvals and by a manual smoke test.
}

/// `GET /api/audit/recent` returns `{entries, total, tip_hash}`. Each entry
/// carries the fields the CLI `security audit` and dashboard activity tab read.
/// Also asserts `?agent_id=` actually filters (not just returns 200) — the
/// dashboard activity tab depends on this for per-agent timelines.
#[tokio::test]
async fn audit_recent_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    // Spawn two agents so we can verify the agent_id filter discriminates.
    let agent_a = spawn_test_agent(&server).await;

    // Spawn a second agent with a slightly different manifest name.
    let manifest_b = TEST_MANIFEST.replace("shape-test-agent", "shape-test-agent-b");
    let resp = reqwest::Client::new()
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": manifest_b}))
        .send()
        .await
        .expect("second spawn");
    let agent_b = resp.json::<Value>().await.unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();

    let body = get_json(&server.base_url, "/api/audit/recent?n=50").await;
    require_keys(
        &body,
        &["entries", "total", "tip_hash"],
        "/api/audit/recent",
    );
    let entries = body["entries"].as_array().expect("entries is array");
    let first = entries
        .first()
        .expect("audit log should have spawn entries from two agents");
    require_keys(
        first,
        &[
            "seq",
            "timestamp",
            "agent_id",
            "agent_name",
            "action",
            "detail",
            "outcome",
            "hash",
        ],
        "/api/audit/recent.entries[0]",
    );
    first["timestamp"]
        .as_str()
        .expect("timestamp must be a string (CLI calls .as_str on it)");

    // Verify the agent_id filter actually filters: querying for agent_a must
    // exclude agent_b's entries. Pre-test, the activity tab silently mixed
    // both agents' events because agent_id mismatches were tolerated.
    let filtered = get_json(
        &server.base_url,
        &format!("/api/audit/recent?n=50&agent_id={agent_a}"),
    )
    .await;
    let filtered_entries = filtered["entries"]
        .as_array()
        .expect("filtered entries is array");
    assert!(
        !filtered_entries.is_empty(),
        "agent_a should appear in its own filtered audit log"
    );
    for e in filtered_entries {
        let id = e["agent_id"].as_str().unwrap_or("");
        assert!(
            id.starts_with(&agent_a),
            "filtered entry has agent_id={id}, expected prefix {agent_a}"
        );
        assert_ne!(
            id, agent_b,
            "agent_b entries must not appear when filtering for agent_a"
        );
    }
}

/// `GET /api/audit/verify` returns `{valid, entries, tip_hash}` for a healthy
/// chain. The dashboard "Verify Now" button reads `chainResult.entries`.
#[tokio::test]
async fn audit_verify_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/audit/verify").await;
    require_keys(
        &body,
        &["valid", "entries", "tip_hash"],
        "/api/audit/verify",
    );
    assert_eq!(body["valid"].as_bool(), Some(true));
    // Brand-new daemon: no entries yet. The empty-log case must surface a
    // `warning` field so consumers (CLI security verify, settings dashboard)
    // can show "no entries — nothing to verify" instead of the misleading
    // "chain valid" success message. Pre-v0.7.30 the dashboard ignored the
    // warning; pre-v0.7.32 the CLI did too.
    assert_eq!(body["entries"].as_u64(), Some(0));
    assert!(
        body["warning"].is_string(),
        "empty audit log should surface a `warning` field so consumers don't \
         render a misleading success state, got: {body}"
    );
}

/// `GET /api/skills` returns `{skills, total}`. Each skill has `name`,
/// `runtime`, and `enabled`. The skills.js page filters on `s.enabled !== false`.
#[tokio::test]
async fn skills_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/skills").await;
    require_keys(&body, &["skills", "total"], "/api/skills");
    let skills = body["skills"].as_array().expect("skills is array");
    if let Some(first) = skills.first() {
        require_keys(first, &["name", "runtime", "enabled"], "/api/skills[0]");
    }
}

/// `GET /api/tools` returns `{tools, total}`. Each tool has `name` and
/// `description`. The settings.js tools tab and CLI both read these.
#[tokio::test]
async fn tools_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/tools").await;
    require_keys(&body, &["tools", "total"], "/api/tools");
    let tools = body["tools"].as_array().expect("tools is array");
    let first = tools.first().expect("tools list is never empty");
    require_keys(first, &["name", "description"], "/api/tools[0]");
}

/// `GET /api/providers` returns `{providers, total}`. Each entry has
/// `id`, `display_name`, and `auth_status`.
#[tokio::test]
async fn providers_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/providers").await;
    require_keys(&body, &["providers", "total"], "/api/providers");
    let providers = body["providers"].as_array().expect("providers is array");
    if let Some(first) = providers.first() {
        require_keys(
            first,
            &["id", "display_name", "auth_status"],
            "/api/providers[0]",
        );
    }
}

/// `GET /api/channels` returns `{channels, total, configured_count}`. Each
/// channel exposes `name`, `display_name`, `auth_status` (used by
/// channels.js statusBadge and CLI `channel list`).
#[tokio::test]
async fn channels_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/channels").await;
    require_keys(
        &body,
        &["channels", "total", "configured_count"],
        "/api/channels",
    );
    let channels = body["channels"].as_array().expect("channels is array");
    let first = channels.first().expect("channel registry is never empty");
    require_keys(
        first,
        &[
            "name",
            "display_name",
            "auth_status",
            "configured",
            "fields",
        ],
        "/api/channels[0]",
    );
}

/// `GET /api/agents/{id}/sessions` returns `{sessions: [...]}`. The CLI
/// `cmd_agent_sessions` reads `s["session_id"]`, `s["message_count"]`,
/// `s["updated_at"]`, `s["label"]` from each session.
#[tokio::test]
async fn agent_sessions_envelope() {
    let server = require_server!(start_test_server());
    let agent_id = spawn_test_agent(&server).await;
    let body = get_json(
        &server.base_url,
        &format!("/api/agents/{agent_id}/sessions"),
    )
    .await;
    require_keys(&body, &["sessions"], "/api/agents/:id/sessions");
    body["sessions"].as_array().expect("sessions is array");
}

/// `GET /api/status` returns the kernel status the CLI `system info` reads.
/// Specifically `status`, `agent_count`, `default_provider`, `default_model`,
/// `uptime_seconds`, `data_dir` are all referenced by cmd_system_info.
#[tokio::test]
async fn status_endpoint_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/status").await;
    require_keys(
        &body,
        &[
            "status",
            "version",
            "agent_count",
            "running_count",
            "default_provider",
            "default_model",
            "uptime_seconds",
            "data_dir",
            "agents",
        ],
        "/api/status",
    );
    assert_eq!(body["status"].as_str(), Some("running"));
    body["agents"].as_array().expect("agents is array");
}

/// `GET /api/peers` returns `{peers, total}`. Each peer (when present) has
/// `node_id`, `node_name`, `address`, `state`, `agents`.
#[tokio::test]
async fn peers_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/peers").await;
    require_keys(&body, &["peers", "total"], "/api/peers");
    let peers = body["peers"].as_array().expect("peers is array");
    if let Some(first) = peers.first() {
        require_keys(
            first,
            &[
                "node_id",
                "node_name",
                "address",
                "state",
                "agents",
                "protocol_version",
            ],
            "/api/peers[0]",
        );
    }
}

/// `GET /api/network/status` exposes the RHP network summary.
#[tokio::test]
async fn network_status_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/network/status").await;
    require_keys(
        &body,
        &[
            "enabled",
            "node_id",
            "listen_address",
            "connected_peers",
            "total_peers",
        ],
        "/api/network/status",
    );
}

/// `GET /api/knowledge` returns `{nodes, edges, total_nodes, total_edges}`.
/// The knowledge.js page reads `data.nodes` and `data.edges`.
#[tokio::test]
async fn knowledge_graph_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/knowledge").await;
    // The endpoint may return either the success or error envelope; both
    // must always carry nodes/edges arrays so the dashboard never crashes.
    require_keys(&body, &["nodes", "edges"], "/api/knowledge");
    body["nodes"].as_array().expect("nodes is array");
    body["edges"].as_array().expect("edges is array");
}

/// `GET /api/models` returns `{models, total, available_count}`.
#[tokio::test]
async fn models_envelope_and_entry_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/models").await;
    require_keys(&body, &["models"], "/api/models");
    let models = body["models"].as_array().expect("models is array");
    if let Some(first) = models.first() {
        require_keys(first, &["id", "provider", "display_name"], "/api/models[0]");
    }
}

/// `GET /api/sessions` returns the paginated envelope with `agent_name`
/// joined onto each session entry. The CLI `cmd_sessions` reads
/// `s["session_id"]`, `s["agent_name"]`, `s["message_count"]`, `s["updated_at"]`.
#[tokio::test]
async fn sessions_endpoint_envelope() {
    let server = require_server!(start_test_server());
    spawn_test_agent(&server).await;
    let body = get_json(&server.base_url, "/api/sessions").await;
    require_keys(&body, &["sessions", "total"], "/api/sessions");
    let sessions = body["sessions"].as_array().expect("sessions is array");
    if let Some(first) = sessions.first() {
        require_keys(
            first,
            &[
                "session_id",
                "agent_id",
                "agent_name",
                "message_count",
                "created_at",
                "updated_at",
                "label",
            ],
            "/api/sessions[0]",
        );
    }
}

/// `GET /api/agents/{id}/metrics` returns counters the dashboard activity
/// tab reads. `total_calls` (NOT `call_count`) was a recent rename source.
#[tokio::test]
async fn agent_metrics_shape() {
    let server = require_server!(start_test_server());
    let agent_id = spawn_test_agent(&server).await;
    let body = get_json(&server.base_url, &format!("/api/agents/{agent_id}/metrics")).await;
    require_keys(&body, &["total_calls"], "/api/agents/:id/metrics");
}

/// `GET /api/budget` returns the flat BudgetStatus the analytics page reads
/// as `this.budget`. Pre-shape-test, an accidental rename of `hourly_spend`
/// to `hourly.spend` would have silently zeroed the budget bars in the UI.
#[tokio::test]
async fn budget_status_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/budget").await;
    require_keys(
        &body,
        &[
            "hourly_spend",
            "hourly_limit",
            "hourly_pct",
            "daily_spend",
            "daily_limit",
            "daily_pct",
            "monthly_spend",
            "monthly_limit",
            "monthly_pct",
            "alert_threshold",
        ],
        "/api/budget",
    );
}

/// `GET /api/budget/agents` returns `{agents, total}` — the analytics page
/// reads `agentRes.agents` (with a fallback to bare-array). Each entry has
/// `agent_id`, `name`, `daily_cost_usd`, and the per-agent limits.
#[tokio::test]
async fn budget_agents_ranking_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/budget/agents").await;
    require_keys(&body, &["agents", "total"], "/api/budget/agents");
    body["agents"].as_array().expect("agents is array");
}

/// `GET /api/usage/summary` returns the flat counters the analytics page
/// reads via `this.summary`. Field renames here would silently zero the
/// tokens / cost / calls headline on the dashboard.
#[tokio::test]
async fn usage_summary_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/usage/summary").await;
    require_keys(
        &body,
        &[
            "total_input_tokens",
            "total_output_tokens",
            "total_cost_usd",
            "call_count",
            "total_tool_calls",
        ],
        "/api/usage/summary",
    );
}

/// `GET /api/usage/by-model` returns `{models: [{model, total_cost_usd,
/// total_input_tokens, total_output_tokens, call_count}]}`. The analytics
/// page reads `modelRes.models` with a bare-array fallback.
#[tokio::test]
async fn usage_by_model_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/usage/by-model").await;
    require_keys(&body, &["models"], "/api/usage/by-model");
    body["models"].as_array().expect("models is array");
}

/// `GET /api/usage/daily` returns `{days, today_cost_usd, first_event_date}`.
/// The analytics page reads `dailyRes.days` (with `daily` and bare-array
/// fallbacks) and `dailyRes.today_cost_usd` for the headline.
#[tokio::test]
async fn usage_daily_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/usage/daily").await;
    require_keys(
        &body,
        &["days", "today_cost_usd", "first_event_date"],
        "/api/usage/daily",
    );
    body["days"].as_array().expect("days is array");
}

/// `GET /api/usage` returns `{agents: [...]}` — used by older clients
/// (analytics page primarily uses /usage/summary now, but CLI may still
/// read this).
#[tokio::test]
async fn usage_stats_envelope() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/usage").await;
    require_keys(&body, &["agents"], "/api/usage");
    body["agents"].as_array().expect("agents is array");
}

/// `GET /api/health/detail` returns the diagnostics the CLI security
/// status command reads. Pre-shape-test, a renamed `agent_count` here would
/// silently make `rustyhand security status` show "Active agents: -".
#[tokio::test]
async fn health_detail_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/health/detail").await;
    require_keys(
        &body,
        &[
            "status",
            "version",
            "uptime_seconds",
            "agent_count",
            "database",
            "panic_count",
            "restart_count",
        ],
        "/api/health/detail",
    );
}

/// `GET /api/auth/me` returns the current principal — fields the dashboard
/// uses to decide which UI affordances to show.
#[tokio::test]
async fn auth_me_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/auth/me").await;
    require_keys(&body, &["authenticated", "role", "source"], "/api/auth/me");
    // No API key configured → implicit-owner localhost session.
    assert_eq!(body["authenticated"].as_bool(), Some(true));
    assert_eq!(body["source"].as_str(), Some("localhost"));
}

/// `GET /api/onboarding` returns `{api_key_set, agent_count, provider,
/// model, demo_mode}`. The dashboard reads `demo_mode` to render its
/// "DEMO MODE" banner — that field appearing as `null` or being dropped
/// would silently disable the banner.
#[tokio::test]
async fn onboarding_status_shape() {
    let server = require_server!(start_test_server());
    let body = get_json(&server.base_url, "/api/onboarding").await;
    require_keys(
        &body,
        &[
            "api_key_set",
            "agent_count",
            "provider",
            "model",
            "demo_mode",
        ],
        "/api/onboarding",
    );
    // Tests use no API key → must be reported as not-set.
    assert_eq!(body["api_key_set"].as_bool(), Some(false));
    // The test fixture pins provider=ollama, so demo_mode is false even
    // though no API key is set. The dashboard demo-mode banner only
    // appears when the active provider is `mock`.
    assert_eq!(body["demo_mode"].as_bool(), Some(false));
    assert_eq!(body["provider"].as_str(), Some("ollama"));
}

/// `GET /api/budget/agents/{id}` returns flat fields the dashboard reads.
/// The struct serialization is the source of truth — this test pins the
/// fields the dashboard activity tab references.
#[tokio::test]
async fn agent_budget_envelope() {
    let server = require_server!(start_test_server());
    let agent_id = spawn_test_agent(&server).await;
    let body = get_json(&server.base_url, &format!("/api/budget/agents/{agent_id}")).await;
    require_keys(
        &body,
        &["agent_id", "agent_name", "hourly", "daily", "monthly"],
        "/api/budget/agents/:id",
    );
    require_keys(
        &body["hourly"],
        &["spend", "limit", "pct"],
        "/api/budget/agents/:id.hourly",
    );
}
