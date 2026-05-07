//! Real HTTP integration tests for the RustyHand API.
//!
//! These tests boot a real kernel, start a real axum HTTP server on a random
//! port, and hit actual endpoints with reqwest.  No mocking.
//!
//! Tests that require an LLM API call are gated behind KIMI_API_KEY.
//!
//! Run: cargo test -p rusty-hand-api --test api_integration_test -- --nocapture

use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use axum::Router;
use rand::rngs::OsRng;
use rusty_hand_api::middleware;
use rusty_hand_api::routes::{self, AppState};
use rusty_hand_api::ws;
use rusty_hand_kernel::workflow::WorkflowId;
use rusty_hand_kernel::RustyHandKernel;
use rusty_hand_types::agent::AgentId;
use rusty_hand_types::config::{DefaultModelConfig, KernelConfig, UserConfig};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

// ---------------------------------------------------------------------------
// Test infrastructure
// ---------------------------------------------------------------------------

struct TestServer {
    base_url: String,
    state: Arc<AppState>,
    _tmp: tempfile::TempDir,
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.state.kernel.shutdown();
    }
}

/// Start a test server using ollama as default provider (no API key needed).
/// This lets the kernel boot without any real LLM credentials.
/// Tests that need actual LLM calls should use `start_test_server_with_llm()`.
async fn start_test_server() -> Option<TestServer> {
    start_test_server_with_provider("ollama", "test-model", "OLLAMA_API_KEY").await
}

/// Start a test server with Kimi as the LLM provider (requires KIMI_API_KEY).
async fn start_test_server_with_llm() -> Option<TestServer> {
    start_test_server_with_provider("kimi", "kimi-for-coding", "KIMI_API_KEY").await
}

async fn start_test_server_with_provider(
    provider: &str,
    model: &str,
    api_key_env: &str,
) -> Option<TestServer> {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");

    let config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        default_model: DefaultModelConfig {
            provider: provider.to_string(),
            model: model.to_string(),
            api_key_env: api_key_env.to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    };

    let kernel = RustyHandKernel::boot_with_config(config).expect("Kernel should boot");
    let kernel = Arc::new(kernel);
    kernel.set_self_handle();

    let state = Arc::new(AppState {
        kernel,
        started_at: Instant::now(),
        bridge_manager: tokio::sync::Mutex::new(None),
        channels_config: tokio::sync::RwLock::new(Default::default()),
        shutdown_notify: Arc::new(tokio::sync::Notify::new()),
        allowed_ws_origins: vec![],
    });

    let app = Router::new()
        .route("/api/health", axum::routing::get(routes::health))
        .route("/api/status", axum::routing::get(routes::status))
        .route(
            "/api/agents",
            axum::routing::get(routes::list_agents).post(routes::spawn_agent),
        )
        .route(
            "/api/agents/{id}/message",
            axum::routing::post(routes::send_message),
        )
        .route(
            "/api/agents/{id}/session",
            axum::routing::get(routes::get_agent_session),
        )
        .route("/api/agents/{id}/ws", axum::routing::get(ws::agent_ws))
        .route(
            "/api/agents/{id}",
            axum::routing::get(routes::get_agent)
                .patch(routes::patch_agent_config)
                .delete(routes::kill_agent),
        )
        .route(
            "/api/agents/{id}/restart",
            axum::routing::post(routes::restart_agent),
        )
        .route(
            "/api/sessions/{id}",
            axum::routing::get(routes::get_session).delete(routes::delete_session),
        )
        .route(
            "/api/sessions/{id}/export.md",
            axum::routing::get(routes::export_session_markdown),
        )
        .route(
            "/api/sessions/{id}/label",
            axum::routing::put(routes::set_session_label),
        )
        .route(
            "/api/triggers",
            axum::routing::get(routes::list_triggers).post(routes::create_trigger),
        )
        .route(
            "/api/triggers/{id}",
            axum::routing::delete(routes::delete_trigger),
        )
        .route(
            "/api/workflows",
            axum::routing::get(routes::list_workflows).post(routes::create_workflow),
        )
        .route(
            "/api/workflows/{id}/run",
            axum::routing::post(routes::run_workflow),
        )
        .route(
            "/api/workflows/{id}/runs",
            axum::routing::get(routes::list_workflow_runs),
        )
        .route(
            "/api/cron/jobs",
            axum::routing::get(routes::list_cron_jobs).post(routes::create_cron_job),
        )
        .route(
            "/api/cron/jobs/{id}",
            axum::routing::put(routes::update_cron_job).delete(routes::delete_cron_job),
        )
        .route(
            "/api/cron/jobs/{id}/enable",
            axum::routing::put(routes::toggle_cron_job),
        )
        .route(
            "/api/cron/jobs/{id}/run",
            axum::routing::post(routes::run_cron_job),
        )
        .route(
            "/api/cron/jobs/{id}/status",
            axum::routing::get(routes::cron_job_status),
        )
        .route("/api/shutdown", axum::routing::post(routes::shutdown))
        .layer(axum::middleware::from_fn(middleware::request_logging))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Skipping integration test: local TCP bind denied: {e}");
            return None;
        }
        Err(e) => panic!("Failed to bind test server: {e}"),
    };
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Some(TestServer {
        base_url: format!("http://{}", addr),
        state,
        _tmp: tmp,
    })
}

macro_rules! require_server {
    ($future:expr) => {
        match $future.await {
            Some(server) => server,
            None => return,
        }
    };
}

/// Manifest that uses ollama (no API key required, won't make real LLM calls).
const TEST_MANIFEST: &str = r#"
name = "test-agent"
version = "0.1.0"
description = "Integration test agent"
author = "test"
module = "builtin:chat"

[model]
provider = "ollama"
model = "test-model"
system_prompt = "You are a test agent. Reply concisely."

[capabilities]
tools = ["file_read"]
memory_read = ["*"]
memory_write = ["self.*"]
"#;

/// Manifest that uses Groq for real LLM tests.
const LLM_MANIFEST: &str = r#"
name = "test-agent"
version = "0.1.0"
description = "Integration test agent"
author = "test"
module = "builtin:chat"

[model]
provider = "kimi"
model = "kimi-for-coding"
system_prompt = "You are a test agent. Reply concisely."

[capabilities]
tools = ["file_read"]
memory_read = ["*"]
memory_write = ["self.*"]
"#;

/// Manifest for the offline mock driver — no API key needed, deterministic
/// echo response. Useful for testing the spawn → message → session flow
/// without burning real LLM credits.
const MOCK_MANIFEST: &str = r#"
name = "mock-test-agent"
version = "0.1.0"
description = "Mock-driver integration test agent"
author = "test"
module = "builtin:chat"

[model]
provider = "mock"
model = "mock-model"
system_prompt = "You are a deterministic mock."

[capabilities]
"#;

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health_endpoint() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/health", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);

    // Middleware injects x-request-id
    assert!(resp.headers().contains_key("x-request-id"));

    let body: serde_json::Value = resp.json().await.unwrap();
    // Public health endpoint returns minimal info (redacted for security)
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    // Detailed fields should NOT appear in public health endpoint
    assert!(body["database"].is_null());
    assert!(body["agent_count"].is_null());
}

#[tokio::test]
async fn test_status_endpoint() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/status", server.base_url))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "running");
    assert_eq!(body["agent_count"], 0);
    assert_eq!(body["running_count"], 0);
    assert!(body["uptime_seconds"].is_number());
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["default_provider"], "ollama");
    assert!(body["agents"].as_array().is_some());
    assert!(body["data_dir"].is_string());
}

#[tokio::test]
async fn test_spawn_list_kill_agent() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // --- Spawn ---
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "test-agent");
    let agent_id = body["agent_id"].as_str().unwrap().to_string();
    assert!(!agent_id.is_empty());

    // --- List (1 agent) ---
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["name"], "test-agent");
    assert_eq!(agents[0]["id"], agent_id);
    assert_eq!(agents[0]["model_provider"], "ollama");

    // --- Kill ---
    let resp = client
        .delete(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "killed");

    // --- List (empty) ---
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_agent_group_roundtrip_and_persistence() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let manifest = r#"
name = "grouped-agent"
version = "0.1.0"
description = "Integration test agent with group"
group = "Operations"
author = "test"
module = "builtin:chat"

[model]
provider = "ollama"
model = "test-model"
system_prompt = "You are a test agent. Reply concisely."

[capabilities]
tools = ["file_read"]
memory_read = ["*"]
memory_write = ["self.*"]
"#;

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": manifest}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents[0]["group"], "Operations");

    let resp = client
        .patch(format!("{}/api/agents/{}", server.base_url, agent_id))
        .json(&serde_json::json!({ "group": "Platform / API" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let detail: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(detail["group"], "Platform / API");

    let parsed_id: AgentId = agent_id.parse().unwrap();
    let persisted = server
        .state
        .kernel
        .memory
        .load_agent(parsed_id)
        .unwrap()
        .unwrap();
    assert_eq!(persisted.manifest.group.as_deref(), Some("Platform / API"));

    let resp = client
        .patch(format!("{}/api/agents/{}", server.base_url, agent_id))
        .json(&serde_json::json!({ "group": "" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let cleared = server
        .state
        .kernel
        .memory
        .load_agent(parsed_id)
        .unwrap()
        .unwrap();
    assert_eq!(cleared.manifest.group, None);
}

#[tokio::test]
async fn test_agent_session_empty() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap();

    // Session should be empty — no messages sent yet
    let resp = client
        .get(format!(
            "{}/api/agents/{}/session",
            server.base_url, agent_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["message_count"], 0);
    assert_eq!(body["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_cron_job_create_and_list() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    let resp = client
        .post(format!("{}/api/cron/jobs", server.base_url))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "name": "daily code analysis",
            "schedule": { "kind": "cron", "expr": "0 0 * * *" },
            "action": { "kind": "agent_turn", "message": "Analyze the repository for regressions" },
            "delivery": { "kind": "none" },
            "one_shot": false,
            "enabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);

    let resp = client
        .get(format!("{}/api/cron/jobs", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let jobs = body["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0]["name"], "daily code analysis");
    assert_eq!(jobs[0]["agent_id"], agent_id);
    assert_eq!(jobs[0]["schedule"]["kind"], "cron");
    assert_eq!(jobs[0]["schedule"]["expr"], "0 0 * * *");
    assert_eq!(jobs[0]["action"]["kind"], "agent_turn");
    assert!(jobs[0]["next_run"].is_string());

    let persisted = std::fs::read_to_string(server._tmp.path().join("cron_jobs.json")).unwrap();
    let persisted_jobs: serde_json::Value = serde_json::from_str(&persisted).unwrap();
    assert_eq!(persisted_jobs.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_cron_job_manual_run_system_event() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/cron/jobs", server.base_url))
        .json(&serde_json::json!({
            "agent_id": "00000000-0000-0000-0000-000000000001",
            "name": "wake-event",
            "schedule": { "kind": "cron", "expr": "0 0 * * *" },
            "action": { "kind": "system_event", "text": "wake up" },
            "delivery": { "kind": "none" },
            "one_shot": false,
            "enabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let create_result: serde_json::Value =
        serde_json::from_str(body["result"].as_str().unwrap()).unwrap();
    let job_id = create_result["job_id"].as_str().unwrap();

    let resp = client
        .post(format!("{}/api/cron/jobs/{job_id}/run", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "completed");
    assert_eq!(body["mode"], "system_event");
    assert_eq!(body["event_type"], "cron.wake-event");

    let resp = client
        .get(format!("{}/api/cron/jobs/{job_id}/status", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["last_run"].is_string());
}

#[tokio::test]
async fn test_send_message_with_llm() {
    if std::env::var("KIMI_API_KEY").is_err() {
        eprintln!("KIMI_API_KEY not set, skipping LLM integration test");
        return;
    }

    let server = require_server!(start_test_server_with_llm());
    let client = reqwest::Client::new();

    // Spawn
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": LLM_MANIFEST}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    // Send message through the real HTTP endpoint → kernel → Groq LLM
    let resp = client
        .post(format!(
            "{}/api/agents/{}/message",
            server.base_url, agent_id
        ))
        .json(&serde_json::json!({"message": "Say hello in exactly 3 words."}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let response_text = body["response"].as_str().unwrap();
    assert!(
        !response_text.is_empty(),
        "LLM response should not be empty"
    );
    assert!(body["input_tokens"].as_u64().unwrap() > 0);
    assert!(body["output_tokens"].as_u64().unwrap() > 0);

    // Session should now have messages
    let resp = client
        .get(format!(
            "{}/api/agents/{}/session",
            server.base_url, agent_id
        ))
        .send()
        .await
        .unwrap();
    let session: serde_json::Value = resp.json().await.unwrap();
    assert!(session["message_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_workflow_crud() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent for workflow
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_name = body["name"].as_str().unwrap().to_string();

    // Create workflow
    let resp = client
        .post(format!("{}/api/workflows", server.base_url))
        .json(&serde_json::json!({
            "name": "test-workflow",
            "description": "Integration test workflow",
            "steps": [
                {
                    "name": "step1",
                    "agent_name": agent_name,
                    "prompt": "Echo: {{input}}",
                    "mode": "sequential",
                    "timeout_secs": 30
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let workflow_id = body["workflow_id"].as_str().unwrap().to_string();
    assert!(!workflow_id.is_empty());

    // List workflows
    let resp = client
        .get(format!("{}/api/workflows", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let workflows: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(workflows.len(), 1);
    assert_eq!(workflows[0]["name"], "test-workflow");
    assert_eq!(workflows[0]["steps"], 1);
}

#[tokio::test]
async fn test_workflow_runs_endpoint_filters_by_workflow_id() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_name = body["name"].as_str().unwrap().to_string();

    let resp = client
        .post(format!("{}/api/workflows", server.base_url))
        .json(&serde_json::json!({
            "name": "workflow-a",
            "description": "Workflow A",
            "steps": [
                {
                    "name": "step-a",
                    "agent_name": agent_name,
                    "prompt": "Echo: {{input}}"
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let workflow_a = WorkflowId(body["workflow_id"].as_str().unwrap().parse().unwrap());

    let resp = client
        .post(format!("{}/api/workflows", server.base_url))
        .json(&serde_json::json!({
            "name": "workflow-b",
            "description": "Workflow B",
            "steps": [
                {
                    "name": "step-b",
                    "agent_name": agent_name,
                    "prompt": "Echo: {{input}}"
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let workflow_b = WorkflowId(body["workflow_id"].as_str().unwrap().parse().unwrap());

    server
        .state
        .kernel
        .workflows
        .create_run(workflow_a, "input-a-1".to_string())
        .await
        .expect("workflow A run should be created");
    server
        .state
        .kernel
        .workflows
        .create_run(workflow_a, "input-a-2".to_string())
        .await
        .expect("workflow A second run should be created");
    server
        .state
        .kernel
        .workflows
        .create_run(workflow_b, "input-b-1".to_string())
        .await
        .expect("workflow B run should be created");

    let resp = client
        .get(format!(
            "{}/api/workflows/{}/runs",
            server.base_url, workflow_a
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let runs: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(runs.len(), 2);
    assert!(runs
        .iter()
        .all(|run| run["workflow_id"] == workflow_a.to_string()));
    assert!(runs.iter().all(|run| run["workflow_name"] == "workflow-a"));

    let resp = client
        .get(format!("{}/api/workflows/not-a-uuid/runs", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_trigger_crud() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent for trigger
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    // Create trigger (Lifecycle pattern — simplest variant)
    let resp = client
        .post(format!("{}/api/triggers", server.base_url))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "pattern": "lifecycle",
            "prompt_template": "Handle: {{event}}",
            "max_fires": 5
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let trigger_id = body["trigger_id"].as_str().unwrap().to_string();
    assert_eq!(body["agent_id"], agent_id);

    // List triggers (unfiltered)
    let resp = client
        .get(format!("{}/api/triggers", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let triggers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(triggers.len(), 1);
    assert_eq!(triggers[0]["agent_id"], agent_id);
    assert_eq!(triggers[0]["enabled"], true);
    assert_eq!(triggers[0]["max_fires"], 5);

    // List triggers (filtered by agent_id)
    let resp = client
        .get(format!(
            "{}/api/triggers?agent_id={}",
            server.base_url, agent_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let triggers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(triggers.len(), 1);

    // Delete trigger
    let resp = client
        .delete(format!("{}/api/triggers/{}", server.base_url, trigger_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List triggers (should be empty)
    let resp = client
        .get(format!("{}/api/triggers", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let triggers: Vec<serde_json::Value> = resp.json().await.unwrap();
    assert_eq!(triggers.len(), 0);
}

#[tokio::test]
async fn test_invalid_agent_id_returns_400() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Send message to invalid ID
    let resp = client
        .post(format!("{}/api/agents/not-a-uuid/message", server.base_url))
        .json(&serde_json::json!({"message": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid"));

    // Kill invalid ID
    let resp = client
        .delete(format!("{}/api/agents/not-a-uuid", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Session for invalid ID
    let resp = client
        .get(format!("{}/api/agents/not-a-uuid/session", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_kill_nonexistent_agent_returns_404() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let fake_id = uuid::Uuid::new_v4();
    let resp = client
        .delete(format!("{}/api/agents/{}", server.base_url, fake_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_spawn_invalid_manifest_returns_400() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": "this is {{ not valid toml"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid manifest"));
}

#[tokio::test]
async fn test_request_id_header_is_uuid() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/health", server.base_url))
        .send()
        .await
        .unwrap();

    let request_id = resp
        .headers()
        .get("x-request-id")
        .expect("x-request-id header should be present");
    let id_str = request_id.to_str().unwrap();
    assert!(
        uuid::Uuid::parse_str(id_str).is_ok(),
        "x-request-id should be a valid UUID, got: {}",
        id_str
    );
}

#[tokio::test]
async fn test_multiple_agents_lifecycle() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn 3 agents
    let mut ids = Vec::new();
    for i in 0..3 {
        let manifest = format!(
            r#"
name = "agent-{i}"
version = "0.1.0"
description = "Multi-agent test {i}"
author = "test"
module = "builtin:chat"

[model]
provider = "ollama"
model = "test-model"
system_prompt = "Agent {i}."

[capabilities]
memory_read = ["*"]
memory_write = ["self.*"]
"#
        );

        let resp = client
            .post(format!("{}/api/agents", server.base_url))
            .json(&serde_json::json!({"manifest_toml": manifest}))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 201);
        let body: serde_json::Value = resp.json().await.unwrap();
        ids.push(body["agent_id"].as_str().unwrap().to_string());
    }

    // List should show 3
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 3);

    // Status should agree
    let resp = client
        .get(format!("{}/api/status", server.base_url))
        .send()
        .await
        .unwrap();
    let status: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(status["agent_count"], 3);

    // Kill one
    let resp = client
        .delete(format!("{}/api/agents/{}", server.base_url, ids[1]))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // List should show 2
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);

    // Kill the rest
    for id in [&ids[0], &ids[2]] {
        client
            .delete(format!("{}/api/agents/{}", server.base_url, id))
            .send()
            .await
            .unwrap();
    }

    // List should be empty
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 0);
}

// ---------------------------------------------------------------------------
// Auth integration tests
// ---------------------------------------------------------------------------

/// Start a test server with Bearer-token authentication enabled.
async fn start_test_server_with_auth(api_key: &str) -> Option<TestServer> {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");

    let config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        api_key: api_key.to_string(),
        default_model: DefaultModelConfig {
            provider: "ollama".to_string(),
            model: "test-model".to_string(),
            api_key_env: "OLLAMA_API_KEY".to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    };

    let kernel = RustyHandKernel::boot_with_config(config).expect("Kernel should boot");
    let kernel = Arc::new(kernel);
    kernel.set_self_handle();

    let state = Arc::new(AppState {
        kernel,
        started_at: Instant::now(),
        bridge_manager: tokio::sync::Mutex::new(None),
        channels_config: tokio::sync::RwLock::new(Default::default()),
        shutdown_notify: Arc::new(tokio::sync::Notify::new()),
        allowed_ws_origins: vec![],
    });

    let app = Router::new()
        .route("/api/health", axum::routing::get(routes::health))
        .route("/api/status", axum::routing::get(routes::status))
        .route("/api/auth/me", axum::routing::get(routes::auth_me))
        .route("/api/commands", axum::routing::get(routes::list_commands))
        .route(
            "/api/agents",
            axum::routing::get(routes::list_agents).post(routes::spawn_agent),
        )
        .route(
            "/api/agents/{id}/message",
            axum::routing::post(routes::send_message),
        )
        .route(
            "/api/agents/{id}/session",
            axum::routing::get(routes::get_agent_session),
        )
        .route("/api/agents/{id}/ws", axum::routing::get(ws::agent_ws))
        .route(
            "/api/agents/{id}",
            axum::routing::delete(routes::kill_agent),
        )
        .route(
            "/api/triggers",
            axum::routing::get(routes::list_triggers).post(routes::create_trigger),
        )
        .route(
            "/api/triggers/{id}",
            axum::routing::delete(routes::delete_trigger),
        )
        .route(
            "/api/workflows",
            axum::routing::get(routes::list_workflows).post(routes::create_workflow),
        )
        .route(
            "/api/workflows/{id}/run",
            axum::routing::post(routes::run_workflow),
        )
        .route(
            "/api/workflows/{id}/runs",
            axum::routing::get(routes::list_workflow_runs),
        )
        .route("/api/shutdown", axum::routing::post(routes::shutdown))
        .layer(axum::middleware::from_fn(middleware::request_logging))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::authorize_http,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Skipping integration test: local TCP bind denied: {e}");
            return None;
        }
        Err(e) => panic!("Failed to bind test server: {e}"),
    };
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Some(TestServer {
        base_url: format!("http://{}", addr),
        state,
        _tmp: tmp,
    })
}

async fn start_test_server_with_user_auth() -> Option<TestServer> {
    let tmp = tempfile::tempdir().expect("Failed to create temp dir");
    let owner_hash = Argon2::default()
        .hash_password(b"owner-token", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();
    let user_hash = Argon2::default()
        .hash_password(b"user-token", &SaltString::generate(&mut OsRng))
        .unwrap()
        .to_string();

    let config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        users: vec![
            UserConfig {
                name: "Owner".to_string(),
                role: "owner".to_string(),
                channel_bindings: HashMap::new(),
                api_key_hash: Some(owner_hash),
            },
            UserConfig {
                name: "Member".to_string(),
                role: "user".to_string(),
                channel_bindings: HashMap::new(),
                api_key_hash: Some(user_hash),
            },
        ],
        default_model: DefaultModelConfig {
            provider: "ollama".to_string(),
            model: "test-model".to_string(),
            api_key_env: "OLLAMA_API_KEY".to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    };

    let kernel = RustyHandKernel::boot_with_config(config).expect("Kernel should boot");
    let kernel = Arc::new(kernel);
    kernel.set_self_handle();

    let state = Arc::new(AppState {
        kernel,
        started_at: Instant::now(),
        bridge_manager: tokio::sync::Mutex::new(None),
        channels_config: tokio::sync::RwLock::new(Default::default()),
        shutdown_notify: Arc::new(tokio::sync::Notify::new()),
        allowed_ws_origins: vec![],
    });

    let app = Router::new()
        .route("/api/health", axum::routing::get(routes::health))
        .route("/api/status", axum::routing::get(routes::status))
        .route("/api/auth/me", axum::routing::get(routes::auth_me))
        .route("/api/commands", axum::routing::get(routes::list_commands))
        .route(
            "/api/agents",
            axum::routing::get(routes::list_agents).post(routes::spawn_agent),
        )
        .layer(axum::middleware::from_fn(middleware::request_logging))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::authorize_http,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::auth,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state.clone());

    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            eprintln!("Skipping integration test: local TCP bind denied: {e}");
            return None;
        }
        Err(e) => panic!("Failed to bind test server: {e}"),
    };
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    Some(TestServer {
        base_url: format!("http://{}", addr),
        state,
        _tmp: tmp,
    })
}

#[tokio::test]
async fn test_auth_health_is_public() {
    let server = require_server!(start_test_server_with_auth("secret-key-123"));
    let client = reqwest::Client::new();

    // /api/health should be accessible without auth
    let resp = client
        .get(format!("{}/api/health", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_auth_rejects_no_token() {
    let server = require_server!(start_test_server_with_auth("secret-key-123"));
    let client = reqwest::Client::new();

    // Protected endpoint without auth header → 401
    // Note: /api/status is public (dashboard needs it), so use a protected endpoint
    let resp = client
        .get(format!("{}/api/commands", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Missing"));
}

#[tokio::test]
async fn test_auth_rejects_wrong_token() {
    let server = require_server!(start_test_server_with_auth("secret-key-123"));
    let client = reqwest::Client::new();

    // Wrong bearer token → 401
    // Note: /api/status is public (dashboard needs it), so use a protected endpoint
    let resp = client
        .get(format!("{}/api/commands", server.base_url))
        .header("authorization", "Bearer wrong-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"].as_str().unwrap().contains("Invalid"));
}

#[tokio::test]
async fn test_auth_accepts_correct_token() {
    let server = require_server!(start_test_server_with_auth("secret-key-123"));
    let client = reqwest::Client::new();

    // Correct bearer token → 200
    let resp = client
        .get(format!("{}/api/status", server.base_url))
        .header("authorization", "Bearer secret-key-123")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "running");
}

#[tokio::test]
async fn test_auth_disabled_when_no_key() {
    // Empty API key = auth disabled
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Protected endpoint accessible without auth when no key is configured
    let resp = client
        .get(format!("{}/api/status", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_user_api_key_auth_exposes_identity() {
    let server = require_server!(start_test_server_with_user_auth());
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{}/api/auth/me", server.base_url))
        .header("authorization", "Bearer user-token")
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["name"], "Member");
    assert_eq!(body["role"], "user");
    assert_eq!(body["source"], "user_api_key");
}

#[tokio::test]
async fn test_user_api_key_rbac_denies_admin_endpoint() {
    let server = require_server!(start_test_server_with_user_auth());
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .header("authorization", "Bearer user-token")
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 403);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["error"]
        .as_str()
        .unwrap()
        .contains("admin role required"));

    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .header("authorization", "Bearer owner-token")
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 201);
}

// ---------------------------------------------------------------------------
// GET /api/sessions/:id
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_get_session_returns_session_json() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    // GET /api/agents/:id to obtain session_id
    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let agent_body: serde_json::Value = resp.json().await.unwrap();
    let session_id = agent_body["session_id"].as_str().unwrap().to_string();

    // GET /api/sessions/:session_id
    let resp = client
        .get(format!("{}/api/sessions/{}", server.base_url, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["id"], session_id);
    assert_eq!(body["agent_id"], agent_id);
    assert_eq!(body["message_count"], 0);
    assert!(body["messages"].as_array().is_some());

    // Invalid UUID → 400
    let resp = client
        .get(format!("{}/api/sessions/not-a-uuid", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Non-existent UUID → 404
    let resp = client
        .get(format!(
            "{}/api/sessions/00000000-0000-0000-0000-000000000000",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------------------------------------------------------------------------
// GET /api/sessions/:id/export.md
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_export_session_markdown() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    let agent_body: serde_json::Value = resp.json().await.unwrap();
    let session_id = agent_body["session_id"].as_str().unwrap().to_string();

    // Export as Markdown
    let resp = client
        .get(format!(
            "{}/api/sessions/{}/export.md",
            server.base_url, session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let ct = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.contains("text/markdown"),
        "Expected text/markdown, got {ct}"
    );

    let cd = resp
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        cd.contains("attachment"),
        "Expected attachment disposition, got {cd}"
    );
    assert!(cd.contains(".md"), "Filename should end in .md, got {cd}");

    let text = resp.text().await.unwrap();
    assert!(
        text.starts_with('#'),
        "Markdown should start with a heading"
    );
    assert!(text.contains("Session ID:"), "Should include session ID");

    // Non-existent → 404
    let resp = client
        .get(format!(
            "{}/api/sessions/00000000-0000-0000-0000-000000000000/export.md",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

// ---------------------------------------------------------------------------
// POST /api/agents/:id/restart
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_restart_agent_preserves_sessions() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let old_agent_id = body["agent_id"].as_str().unwrap().to_string();

    // Obtain session_id before restart
    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, old_agent_id))
        .send()
        .await
        .unwrap();
    let agent_body: serde_json::Value = resp.json().await.unwrap();
    let old_session_id = agent_body["session_id"].as_str().unwrap().to_string();

    // Restart
    let resp = client
        .post(format!(
            "{}/api/agents/{}/restart",
            server.base_url, old_agent_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "restarted");
    assert_eq!(body["old_agent_id"], old_agent_id);
    let new_agent_id = body["new_agent_id"].as_str().unwrap().to_string();
    assert_ne!(
        new_agent_id, old_agent_id,
        "New agent must have a different ID"
    );

    // Old session still accessible (sessions were NOT deleted)
    let resp = client
        .get(format!(
            "{}/api/sessions/{}",
            server.base_url, old_session_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "Old session must survive restart — sessions should be preserved"
    );
    let sess: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(sess["id"], old_session_id);

    // New agent is live
    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, new_agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let new_body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(new_body["name"], "test-agent");

    // Old agent ID is gone
    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, old_agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Restart nonexistent (already gone) → 404
    let resp = client
        .post(format!(
            "{}/api/agents/{}/restart",
            server.base_url, old_agent_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_session_label_set_and_clear() {
    let server = require_server!(start_test_server());
    let client = reqwest::Client::new();

    // Spawn agent to get a session
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": TEST_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();

    // Get the session ID from the agent
    let resp = client
        .get(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    let agent: serde_json::Value = resp.json().await.unwrap();
    let session_id = agent["session_id"].as_str().unwrap().to_string();

    // Initially label is null
    let resp = client
        .get(format!("{}/api/sessions/{}", server.base_url, session_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let sess: serde_json::Value = resp.json().await.unwrap();
    assert!(sess["label"].is_null(), "New session should have no label");

    // Set a label
    let resp = client
        .put(format!(
            "{}/api/sessions/{}/label",
            server.base_url, session_id
        ))
        .json(&serde_json::json!({"label": "my-label"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let upd: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(upd["label"], "my-label");

    // Confirm label persisted
    let resp = client
        .get(format!("{}/api/sessions/{}", server.base_url, session_id))
        .send()
        .await
        .unwrap();
    let sess: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(sess["label"], "my-label");

    // Clear the label (null)
    let resp = client
        .put(format!(
            "{}/api/sessions/{}/label",
            server.base_url, session_id
        ))
        .json(&serde_json::json!({"label": null}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Confirm cleared
    let resp = client
        .get(format!("{}/api/sessions/{}", server.base_url, session_id))
        .send()
        .await
        .unwrap();
    let sess: serde_json::Value = resp.json().await.unwrap();
    assert!(sess["label"].is_null(), "Label should be cleared");

    // Invalid session ID → 400
    let resp = client
        .put(format!("{}/api/sessions/not-a-uuid/label", server.base_url))
        .json(&serde_json::json!({"label": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Unknown session → 404
    let fake_id = uuid::Uuid::new_v4();
    let resp = client
        .put(format!(
            "{}/api/sessions/{}/label",
            server.base_url, fake_id
        ))
        .json(&serde_json::json!({"label": "x"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

/// Demo Mode boot: when the kernel boots with provider=mock and no user
/// agents exist, a welcome agent named `rusty` should be auto-spawned so
/// the dashboard is interactive on first load. The agent must accept a
/// message and reply through the mock driver.
#[tokio::test]
async fn test_demo_mode_auto_spawns_welcome_agent() {
    let server = require_server!(start_test_server_with_provider(
        "mock",
        "mock-model",
        "MOCK_API_KEY"
    ));
    let client = reqwest::Client::new();

    // The kernel should have auto-spawned exactly one welcome agent.
    let resp = client
        .get(format!("{}/api/agents", server.base_url))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    let agents = body["agents"].as_array().expect("agents envelope");
    let rusty = agents
        .iter()
        .find(|a| a["name"].as_str() == Some("rusty"))
        .expect("demo mode should auto-spawn `rusty`");
    let agent_id = rusty["id"].as_str().unwrap().to_string();

    // The agent must be tagged so the dashboard can highlight it as a demo.
    // (The test fixture doesn't read tags directly but registering them in
    // the manifest is part of the contract.)
    assert_eq!(rusty["model_provider"].as_str(), Some("mock"));

    // The agent is interactive end-to-end.
    let resp = client
        .post(format!(
            "{}/api/agents/{}/message",
            server.base_url, agent_id
        ))
        .json(&serde_json::json!({"message": "what can you do"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    let response = body["response"].as_str().unwrap_or("");
    assert!(
        response.contains("[mock]") && response.contains("what can you do"),
        "demo welcome agent should reply through the mock driver, got: {response}"
    );
}

/// End-to-end cron run that fires a workflow (the third CronAction variant).
/// system_event and agent_turn are covered by other tests; this fills the gap
/// for workflow_run, which has its own kernel branch and was not exercised
/// by any integration test before.
#[tokio::test]
async fn test_cron_manual_run_workflow_with_mock_driver() {
    let server = require_server!(start_test_server_with_provider(
        "mock",
        "mock-model",
        "MOCK_API_KEY"
    ));
    let client = reqwest::Client::new();

    // 1. Spawn a mock-driver agent for the workflow step.
    let manifest = MOCK_MANIFEST.replace("mock-test-agent", "cron-wf-mock-agent");
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": manifest}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"].as_str().unwrap().to_string();
    let agent_name = body["name"].as_str().unwrap().to_string();

    // 2. Register a single-step workflow.
    let resp = client
        .post(format!("{}/api/workflows", server.base_url))
        .json(&serde_json::json!({
            "name": "cron-wakeup-workflow",
            "description": "Workflow fired from a cron job",
            "steps": [
                {
                    "name": "wakeup",
                    "agent_name": agent_name,
                    "prompt": "Wakeup says: {{input}}",
                    "mode": "sequential",
                    "timeout_secs": 30
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let workflow_id = resp.json::<serde_json::Value>().await.unwrap()["workflow_id"]
        .as_str()
        .unwrap()
        .to_string();

    // 3. Create a cron job whose action is to run that workflow.
    let resp = client
        .post(format!("{}/api/cron/jobs", server.base_url))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "name": "morning-workflow",
            "schedule": { "kind": "cron", "expr": "0 0 1 1 *" },
            "action": {
                "kind": "workflow_run",
                "workflow_id": workflow_id,
                "input": "good morning",
                "timeout_secs": 30
            },
            "delivery": { "kind": "none" },
            "one_shot": false,
            "enabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "cron create returned {}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    let create_result: serde_json::Value =
        serde_json::from_str(body["result"].as_str().unwrap()).unwrap();
    let job_id = create_result["job_id"].as_str().unwrap().to_string();

    // 4. Manually fire the cron job.
    let resp = client
        .post(format!("{}/api/cron/jobs/{job_id}/run", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "cron run returned {}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "completed");
    assert_eq!(body["mode"], "workflow_run");
    assert_eq!(body["workflow_id"], workflow_id);

    // 5. The workflow output must reflect both the mock driver and the
    //    cron-supplied input — confirms the cron → workflow → driver chain
    //    propagated the input all the way through.
    let output = body["output"].as_str().unwrap_or("");
    assert!(
        output.contains("[mock]"),
        "cron workflow_run output should include the mock prefix, got: {output}"
    );
    assert!(
        output.contains("good morning"),
        "cron-supplied input should propagate to the workflow step, got: {output}"
    );
}

/// End-to-end manual cron run that actually invokes the agent loop.
///
/// The existing test_cron_job_manual_run_system_event covers the
/// system_event path which doesn't talk to an LLM. This test covers the
/// agent_turn path: a cron job that asks an agent (backed by the mock
/// driver) to respond. Catches regressions in the cron → kernel →
/// driver → cron-result wiring.
#[tokio::test]
async fn test_cron_manual_run_agent_turn_with_mock_driver() {
    let server = require_server!(start_test_server_with_provider(
        "mock",
        "mock-model",
        "MOCK_API_KEY"
    ));
    let client = reqwest::Client::new();

    // Spawn a mock-driver agent.
    let manifest = MOCK_MANIFEST.replace("mock-test-agent", "cron-mock-agent");
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": manifest}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let agent_id = resp.json::<serde_json::Value>().await.unwrap()["agent_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Create an agent_turn cron job. The schedule is far in the future
    // so the kernel's tick won't fire it before we manually invoke run.
    let resp = client
        .post(format!("{}/api/cron/jobs", server.base_url))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "name": "morning-ping",
            "schedule": { "kind": "cron", "expr": "0 0 1 1 *" }, // Jan 1 midnight
            "action": {
                "kind": "agent_turn",
                "message": "say hi",
                "timeout_secs": 10
            },
            "delivery": { "kind": "none" },
            "one_shot": false,
            "enabled": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201, "cron create returned {}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    let create_result: serde_json::Value =
        serde_json::from_str(body["result"].as_str().unwrap()).unwrap();
    let job_id = create_result["job_id"].as_str().unwrap().to_string();

    // Manually trigger the run.
    let resp = client
        .post(format!("{}/api/cron/jobs/{job_id}/run", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "cron run returned {}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "completed");
    assert_eq!(body["mode"], "agent_turn");

    // The cron run response carries the agent's reply (the mock echoes
    // the message). Verify the mock prefix is present so we know the
    // run actually went through the driver, not a dry-run path.
    let response = body["response"].as_str().unwrap_or("");
    assert!(
        response.contains("[mock]"),
        "cron agent_turn response should include the mock prefix, got: {response}"
    );
    assert!(
        response.contains("say hi"),
        "cron agent_turn response should echo the prompt, got: {response}"
    );
}

/// End-to-end workflow run using mock-driver agents.
///
/// This was previously not testable without an LLM API key. With the mock
/// driver in v0.7.30, we can exercise the full workflow → kernel → agent
/// → driver → step result pipeline in CI. Catches regressions in:
///   - Workflow registration via the API
///   - Step → agent name resolution
///   - Sequential step execution + output piping
///   - Workflow run completion + final output assembly
#[tokio::test]
async fn test_workflow_run_end_to_end_with_mock_driver() {
    let server = require_server!(start_test_server_with_provider(
        "mock",
        "mock-model",
        "MOCK_API_KEY"
    ));
    let client = reqwest::Client::new();

    // Spawn an agent backed by the mock driver. We need a unique manifest
    // name in case the test runs alongside other tests reusing MOCK_MANIFEST.
    let manifest = MOCK_MANIFEST.replace("mock-test-agent", "wf-mock-agent");
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": manifest}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_name = body["name"]
        .as_str()
        .expect("spawn returns the resolved agent name")
        .to_string();

    // Create a 2-step workflow. Step 1 receives `{{input}}`, step 2
    // receives step 1's output via `{{input}}`. With the mock driver
    // each step echoes its prompt as `[mock] <prompt>`, so the final
    // output is `[mock] Stage two: [mock] Stage one: hello`.
    let resp = client
        .post(format!("{}/api/workflows", server.base_url))
        .json(&serde_json::json!({
            "name": "echo-pipeline",
            "description": "Two-step echo through mock driver",
            "steps": [
                {
                    "name": "stage_one",
                    "agent_name": agent_name,
                    "prompt": "Stage one: {{input}}",
                    "mode": "sequential",
                    "timeout_secs": 30
                },
                {
                    "name": "stage_two",
                    "agent_name": agent_name,
                    "prompt": "Stage two: {{input}}",
                    "mode": "sequential",
                    "timeout_secs": 30
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "workflow create returned {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    let workflow_id = body["workflow_id"]
        .as_str()
        .expect("workflow_id in response")
        .to_string();

    // Run the workflow.
    let resp = client
        .post(format!(
            "{}/api/workflows/{}/run",
            server.base_url, workflow_id
        ))
        .json(&serde_json::json!({"input": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        200,
        "workflow run returned {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"].as_str(), Some("completed"));
    let output = body["output"]
        .as_str()
        .expect("workflow run returns final output");

    // The mock driver replies `[mock] <prompt>`. After two stages, the
    // final output must contain both prompts piped through.
    assert!(
        output.contains("[mock]"),
        "expected mock-prefixed output, got: {output}"
    );
    assert!(
        output.contains("Stage two:"),
        "final output should reflect stage_two's prompt template, got: {output}"
    );
    assert!(
        output.contains("Stage one:"),
        "final output should carry stage_one's content forward, got: {output}"
    );
    assert!(
        output.contains("hello"),
        "original input should propagate end-to-end, got: {output}"
    );

    // The runs endpoint should now have one completed run for this workflow.
    let resp = client
        .get(format!(
            "{}/api/workflows/{}/runs",
            server.base_url, workflow_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let runs: serde_json::Value = resp.json().await.unwrap();
    let arr = runs["runs"]
        .as_array()
        .or_else(|| runs.as_array())
        .expect("runs response is an array (bare or under .runs)");
    assert_eq!(arr.len(), 1, "expected exactly one run, got {}", arr.len());
}

/// End-to-end agent message round-trip using the deterministic mock driver.
/// This test exercises the full HTTP → kernel → driver → session pipeline
/// without an LLM API key, so it always runs in CI. Until v0.7.30 there was
/// no way to test agent.message end-to-end without burning real LLM credits.
#[tokio::test]
async fn test_mock_driver_full_message_roundtrip() {
    let server = require_server!(start_test_server_with_provider(
        "mock",
        "mock-model",
        "MOCK_API_KEY"
    ));
    let client = reqwest::Client::new();

    // Spawn an agent backed by the mock driver.
    let resp = client
        .post(format!("{}/api/agents", server.base_url))
        .json(&serde_json::json!({"manifest_toml": MOCK_MANIFEST}))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "spawn returned {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    let agent_id = body["agent_id"]
        .as_str()
        .expect("spawn response includes agent_id")
        .to_string();

    // Send a message — the mock driver should echo it back as `[mock] <text>`.
    let resp = client
        .post(format!(
            "{}/api/agents/{}/message",
            server.base_url, agent_id
        ))
        .json(&serde_json::json!({"message": "ping"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200, "message returned {}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    let response_text = body["response"]
        .as_str()
        .expect("message response includes `response`");
    assert_eq!(response_text, "[mock] ping");
    assert!(
        body["input_tokens"].as_u64().unwrap_or(0) > 0,
        "input_tokens should be tracked"
    );
    assert!(
        body["output_tokens"].as_u64().unwrap_or(0) > 0,
        "output_tokens should be tracked"
    );

    // Session should have grown.
    let resp = client
        .get(format!(
            "{}/api/agents/{}/session",
            server.base_url, agent_id
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let session: serde_json::Value = resp.json().await.unwrap();
    let count = session["message_count"].as_u64().unwrap_or(0);
    assert!(
        count >= 2,
        "session should have user + assistant messages, got {count}"
    );

    // A second turn must echo the latest user message, not the first.
    let resp = client
        .post(format!(
            "{}/api/agents/{}/message",
            server.base_url, agent_id
        ))
        .json(&serde_json::json!({"message": "pong"}))
        .send()
        .await
        .unwrap();
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["response"].as_str(), Some("[mock] pong"));

    // Kill cleanly.
    let resp = client
        .delete(format!("{}/api/agents/{}", server.base_url, agent_id))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}
