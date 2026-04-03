//! MCP (Model Context Protocol) server for RustyHand.
//!
//! Exposes running agents as MCP tools over JSON-RPC 2.0 stdio.
//! Each agent becomes a callable tool named `rusty_hand_agent_{name}`.
//! Additionally exposes static tools covering the full RustyHand API:
//! system health/status, agent CRUD, budget, workflows, channels,
//! cron jobs, templates, and built-in tools.
//!
//! Protocol: Content-Length framing over stdin/stdout.
//! Connects to running daemon via HTTP, falls back to in-process kernel.

use rusty_hand_kernel::RustyHandKernel;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

/// Backend for MCP: either a running daemon or an in-process kernel.
enum McpBackend {
    Daemon {
        base_url: String,
        client: reqwest::blocking::Client,
    },
    InProcess {
        kernel: Box<RustyHandKernel>,
        rt: tokio::runtime::Runtime,
    },
}

// ── Static tool definitions ────────────────────────────────────────────────

/// Return JSON definitions for all static MCP tools.
fn static_tool_definitions() -> Vec<Value> {
    vec![
        // ── System (5) ─────────────────────────────────────
        json!({
            "name": "rustyhand_system_health",
            "description": "Check RustyHand daemon health and database connectivity.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_system_status",
            "description": "Get RustyHand system status: uptime, agent count, default provider/model.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_config_reload",
            "description": "Hot-reload RustyHand configuration from ~/.rustyhand/config.toml.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_config_get",
            "description": "Get the current runtime configuration (redacted secrets).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_config_set",
            "description": "Set a config field by dotted path (e.g. 'default_model.provider'). Applied immediately.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key": { "type": "string", "description": "Dotted config path, e.g. 'default_model.model'" },
                    "value": { "type": "string", "description": "New value as string" }
                },
                "required": ["key", "value"]
            }
        }),
        // ── Agent (8) ──────────────────────────────────────
        json!({
            "name": "rustyhand_agent_list",
            "description": "List all agents with IDs, names, states, and models.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_agent_get",
            "description": "Get full details of a specific agent by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" }
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "rustyhand_agent_spawn",
            "description": "Spawn a new agent from a TOML manifest string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "manifest_toml": { "type": "string", "description": "Agent manifest in TOML format" }
                },
                "required": ["manifest_toml"]
            }
        }),
        json!({
            "name": "rustyhand_agent_kill",
            "description": "Kill (stop and remove) a running agent by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent to kill" }
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "rustyhand_agent_message",
            "description": "Send a message to an agent and get its response (triggers LLM call).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the target agent" },
                    "message": { "type": "string", "description": "Message to send" }
                },
                "required": ["agent_id", "message"]
            }
        }),
        json!({
            "name": "rustyhand_agent_session",
            "description": "Get the conversation session (message history) for an agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" }
                },
                "required": ["agent_id"]
            }
        }),
        json!({
            "name": "rustyhand_agent_set_model",
            "description": "Change an agent's LLM model. Provider is auto-detected from the model catalog.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" },
                    "model": { "type": "string", "description": "Model ID or alias (e.g. 'MiniMax-M2.7', 'claude-sonnet-4-20250514')" }
                },
                "required": ["agent_id", "model"]
            }
        }),
        json!({
            "name": "rustyhand_agent_session_reset",
            "description": "Reset (clear) an agent's conversation session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" }
                },
                "required": ["agent_id"]
            }
        }),
        // ── Providers & Models (2) ─────────────────────────
        json!({
            "name": "rustyhand_provider_list",
            "description": "List all LLM providers with auth status (configured/missing), model counts, and API key env var names.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_model_list",
            "description": "List available models (only from configured providers). Shows tier, context window, cost, capabilities.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "provider": { "type": "string", "description": "Optional: filter by provider ID" }
                }
            }
        }),
        // ── Budget (2) ─────────────────────────────────────
        json!({
            "name": "rustyhand_budget_status",
            "description": "Get global budget status: hourly/daily/monthly spend vs limits.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_budget_agents",
            "description": "Get per-agent cost ranking and budget usage.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        // ── Workflow (3) ───────────────────────────────────
        json!({
            "name": "rustyhand_workflow_list",
            "description": "List all registered workflow definitions.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_workflow_run",
            "description": "Execute a workflow by ID with the given input string.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow_id": { "type": "string", "description": "UUID of the workflow" },
                    "input": { "type": "string", "description": "Input string for the workflow" }
                },
                "required": ["workflow_id", "input"]
            }
        }),
        json!({
            "name": "rustyhand_workflow_runs",
            "description": "List past runs of a specific workflow.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "workflow_id": { "type": "string", "description": "UUID of the workflow" }
                },
                "required": ["workflow_id"]
            }
        }),
        // ── Cron (3) ───────────────────────────────────────
        json!({
            "name": "rustyhand_cron_list",
            "description": "List all scheduled cron jobs across all agents.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_cron_create",
            "description": "Create a new cron job for an agent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "UUID of the agent" },
                    "name": { "type": "string", "description": "Job name" },
                    "schedule": { "type": "string", "description": "Cron expression (e.g. '0 9 * * *')" },
                    "message": { "type": "string", "description": "Message to send on each trigger" }
                },
                "required": ["agent_id", "name", "schedule", "message"]
            }
        }),
        json!({
            "name": "rustyhand_cron_delete",
            "description": "Delete a cron job by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "job_id": { "type": "string", "description": "UUID of the cron job" }
                },
                "required": ["job_id"]
            }
        }),
        // ── Approvals (3) ──────────────────────────────────
        json!({
            "name": "rustyhand_approval_list",
            "description": "List all execution approval requests (pending, approved, rejected).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_approval_approve",
            "description": "Approve a pending execution approval request.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "approval_id": { "type": "string", "description": "UUID of the approval request" }
                },
                "required": ["approval_id"]
            }
        }),
        json!({
            "name": "rustyhand_approval_reject",
            "description": "Reject a pending execution approval request.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "approval_id": { "type": "string", "description": "UUID of the approval request" }
                },
                "required": ["approval_id"]
            }
        }),
        // ── Other (3) ──────────────────────────────────────
        json!({
            "name": "rustyhand_channel_list",
            "description": "List all messaging channel adapters and their configuration status.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_template_list",
            "description": "List available agent templates from ~/.rustyhand/agents/.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "rustyhand_tool_list",
            "description": "List all built-in tools available to agents.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}

// ── Tool dispatch ──────────────────────────────────────────────────────────

/// Dispatch a static tool call to the appropriate backend method.
fn dispatch_tool(backend: &McpBackend, tool_name: &str, args: &Value) -> Result<Value, String> {
    match tool_name {
        // System
        "rustyhand_system_health" => backend.api_health(),
        "rustyhand_system_status" => backend.api_status(),
        "rustyhand_config_reload" => backend.api_config_reload(),
        "rustyhand_config_get" => backend.api_config_get(),
        "rustyhand_config_set" => {
            let key = require_str(args, "key")?;
            let value = require_str(args, "value")?;
            backend.api_config_set(&key, &value)
        }
        // Agent
        "rustyhand_agent_list" => backend.api_agent_list(),
        "rustyhand_agent_get" => {
            let agent_id = require_str(args, "agent_id")?;
            backend.api_agent_get(&agent_id)
        }
        "rustyhand_agent_spawn" => {
            let manifest_toml = require_str(args, "manifest_toml")?;
            backend.api_agent_spawn(&manifest_toml)
        }
        "rustyhand_agent_kill" => {
            let agent_id = require_str(args, "agent_id")?;
            backend.api_agent_kill(&agent_id)
        }
        "rustyhand_agent_message" => {
            let agent_id = require_str(args, "agent_id")?;
            let message = require_str(args, "message")?;
            backend.api_agent_message(&agent_id, &message)
        }
        "rustyhand_agent_session" => {
            let agent_id = require_str(args, "agent_id")?;
            backend.api_agent_session(&agent_id)
        }
        "rustyhand_agent_set_model" => {
            let agent_id = require_str(args, "agent_id")?;
            let model = require_str(args, "model")?;
            backend.api_agent_set_model(&agent_id, &model)
        }
        "rustyhand_agent_session_reset" => {
            let agent_id = require_str(args, "agent_id")?;
            backend.api_agent_session_reset(&agent_id)
        }
        // Providers & Models
        "rustyhand_provider_list" => backend.api_provider_list(),
        "rustyhand_model_list" => {
            let provider = args["provider"].as_str().map(|s| s.to_string());
            backend.api_model_list(provider.as_deref())
        }
        // Budget
        "rustyhand_budget_status" => backend.api_budget_status(),
        "rustyhand_budget_agents" => backend.api_budget_agents(),
        // Workflow
        "rustyhand_workflow_list" => backend.api_workflow_list(),
        "rustyhand_workflow_run" => {
            let workflow_id = require_str(args, "workflow_id")?;
            let input = require_str(args, "input")?;
            backend.api_workflow_run(&workflow_id, &input)
        }
        "rustyhand_workflow_runs" => {
            let workflow_id = require_str(args, "workflow_id")?;
            backend.api_workflow_runs(&workflow_id)
        }
        // Cron
        "rustyhand_cron_list" => backend.api_cron_list(),
        "rustyhand_cron_create" => {
            let agent_id = require_str(args, "agent_id")?;
            let name = require_str(args, "name")?;
            let schedule = require_str(args, "schedule")?;
            let message = require_str(args, "message")?;
            backend.api_cron_create(&agent_id, &name, &schedule, &message)
        }
        "rustyhand_cron_delete" => {
            let job_id = require_str(args, "job_id")?;
            backend.api_cron_delete(&job_id)
        }
        // Approvals
        "rustyhand_approval_list" => backend.api_approval_list(),
        "rustyhand_approval_approve" => {
            let approval_id = require_str(args, "approval_id")?;
            backend.api_approval_approve(&approval_id)
        }
        "rustyhand_approval_reject" => {
            let approval_id = require_str(args, "approval_id")?;
            backend.api_approval_reject(&approval_id)
        }
        // Other
        "rustyhand_channel_list" => backend.api_channel_list(),
        "rustyhand_template_list" => backend.api_template_list(),
        "rustyhand_tool_list" => backend.api_tool_list(),
        _ => Err(format!("Unknown tool: {tool_name}")),
    }
}

/// Extract a required string argument from the JSON args object.
fn require_str(args: &Value, key: &str) -> Result<String, String> {
    args[key]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Missing required argument: '{key}'"))
}

// ── McpBackend implementation ──────────────────────────────────────────────

impl McpBackend {
    // ── Helpers ────────────────────────────────────────────────────────────

    /// HTTP GET returning parsed JSON (Daemon mode helper).
    fn daemon_get(
        client: &reqwest::blocking::Client,
        base_url: &str,
        path: &str,
    ) -> Result<Value, String> {
        let resp = client
            .get(format!("{base_url}{path}"))
            .send()
            .map_err(|e| format!("HTTP error: {e}"))?;
        resp.json::<Value>()
            .map_err(|e| format!("Parse error: {e}"))
    }

    /// HTTP POST returning parsed JSON (Daemon mode helper).
    fn daemon_post(
        client: &reqwest::blocking::Client,
        base_url: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, String> {
        let mut req = client.post(format!("{base_url}{path}"));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().map_err(|e| format!("HTTP error: {e}"))?;
        resp.json::<Value>()
            .map_err(|e| format!("Parse error: {e}"))
    }

    /// HTTP PUT returning parsed JSON (Daemon mode helper).
    fn daemon_put(
        client: &reqwest::blocking::Client,
        base_url: &str,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, String> {
        let mut req = client.put(format!("{base_url}{path}"));
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().map_err(|e| format!("HTTP error: {e}"))?;
        resp.json::<Value>()
            .map_err(|e| format!("Parse error: {e}"))
    }

    /// HTTP DELETE returning parsed JSON (Daemon mode helper).
    fn daemon_delete(
        client: &reqwest::blocking::Client,
        base_url: &str,
        path: &str,
    ) -> Result<Value, String> {
        let resp = client
            .delete(format!("{base_url}{path}"))
            .send()
            .map_err(|e| format!("HTTP error: {e}"))?;
        resp.json::<Value>()
            .map_err(|e| format!("Parse error: {e}"))
    }

    // ── Existing methods (backward compat) ─────────────────────────────────

    fn list_agents(&self) -> Vec<(String, String, String)> {
        // Returns (id, name, description) triples for dynamic per-agent tools
        match self {
            McpBackend::Daemon { base_url, client } => {
                let resp = client
                    .get(format!("{base_url}/api/agents"))
                    .send()
                    .ok()
                    .and_then(|r| r.json::<Value>().ok());
                match resp.and_then(|v| v.as_array().cloned()) {
                    Some(agents) => agents
                        .iter()
                        .map(|a| {
                            (
                                a["id"].as_str().unwrap_or("").to_string(),
                                a["name"].as_str().unwrap_or("").to_string(),
                                a["description"].as_str().unwrap_or("").to_string(),
                            )
                        })
                        .collect(),
                    None => Vec::new(),
                }
            }
            McpBackend::InProcess { kernel, .. } => kernel
                .registry
                .list()
                .iter()
                .map(|e| {
                    (
                        e.id.to_string(),
                        e.name.clone(),
                        e.manifest.description.clone(),
                    )
                })
                .collect(),
        }
    }

    fn send_message(&self, agent_id: &str, message: &str) -> Result<String, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                let resp = client
                    .post(format!("{base_url}/api/agents/{agent_id}/message"))
                    .json(&json!({"message": message}))
                    .send()
                    .map_err(|e| format!("HTTP error: {e}"))?;
                let body: Value = resp.json().map_err(|e| format!("Parse error: {e}"))?;
                if let Some(response) = body["response"].as_str() {
                    Ok(response.to_string())
                } else {
                    Err(body["error"]
                        .as_str()
                        .unwrap_or("Unknown error")
                        .to_string())
                }
            }
            McpBackend::InProcess { kernel, rt } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                let result = rt
                    .block_on(kernel.send_message(aid, message))
                    .map_err(|e| format!("{e}"))?;
                Ok(result.response)
            }
        }
    }

    /// Find agent ID by tool name (strip `rusty_hand_agent_` prefix, match by name).
    fn resolve_tool_agent(&self, tool_name: &str) -> Option<String> {
        let agent_name = tool_name
            .strip_prefix("rusty_hand_agent_")?
            .replace('_', "-");
        let agents = self.list_agents();
        // Try exact match first (with underscores replaced by hyphens)
        for (id, name, _) in &agents {
            if name.replace(' ', "-").to_lowercase() == agent_name.to_lowercase() {
                return Some(id.clone());
            }
        }
        // Try with underscores
        let agent_name_underscore = tool_name.strip_prefix("rusty_hand_agent_")?;
        for (id, name, _) in &agents {
            if name.replace('-', "_").to_lowercase() == agent_name_underscore.to_lowercase() {
                return Some(id.clone());
            }
        }
        None
    }

    // ── System ─────────────────────────────────────────────────────────────

    fn api_health(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/health")
            }
            McpBackend::InProcess { .. } => Ok(json!({"status": "ok"})),
        }
    }

    fn api_status(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/status")
            }
            McpBackend::InProcess { kernel, .. } => {
                let agents = kernel.registry.list();
                Ok(json!({
                    "status": "running",
                    "agent_count": agents.len(),
                    "default_provider": kernel.config.default_model.provider,
                    "default_model": kernel.config.default_model.model,
                }))
            }
        }
    }

    fn api_config_reload(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_post(client, base_url, "/api/config/reload", None)
            }
            McpBackend::InProcess { kernel, .. } => {
                let plan = kernel.reload_config()?;
                let status = if !plan.has_changes() {
                    "no_changes"
                } else if plan.restart_required {
                    "partial"
                } else {
                    "applied"
                };
                Ok(json!({
                    "status": status,
                    "restart_required": plan.restart_required,
                    "restart_reasons": plan.restart_reasons,
                    "hot_actions_applied": plan.hot_actions.len(),
                    "noop_changes": plan.noop_changes,
                }))
            }
        }
    }

    // ── Agent ──────────────────────────────────────────────────────────────

    fn api_agent_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/agents")
            }
            McpBackend::InProcess { kernel, .. } => {
                let agents = kernel.registry.list();
                serde_json::to_value(&agents).map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    fn api_agent_spawn(&self, manifest_toml: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                "/api/agents",
                Some(&json!({"manifest_toml": manifest_toml})),
            ),
            McpBackend::InProcess { kernel, .. } => {
                let manifest: rusty_hand_types::agent::AgentManifest =
                    toml::from_str(manifest_toml)
                        .map_err(|e| format!("Invalid TOML manifest: {e}"))?;
                let name = manifest.name.clone();
                let agent_id = kernel.spawn_agent(manifest).map_err(|e| format!("{e}"))?;
                Ok(json!({"agent_id": agent_id.to_string(), "name": name}))
            }
        }
    }

    fn api_agent_kill(&self, agent_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_delete(client, base_url, &format!("/api/agents/{agent_id}"))
            }
            McpBackend::InProcess { kernel, .. } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                kernel.kill_agent(aid).map_err(|e| format!("{e}"))?;
                Ok(json!({"status": "killed", "agent_id": agent_id}))
            }
        }
    }

    fn api_agent_message(&self, agent_id: &str, message: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                &format!("/api/agents/{agent_id}/message"),
                Some(&json!({"message": message})),
            ),
            McpBackend::InProcess { kernel, rt } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                let result = rt
                    .block_on(kernel.send_message(aid, message))
                    .map_err(|e| format!("{e}"))?;
                Ok(json!({
                    "response": result.response,
                    "input_tokens": result.total_usage.input_tokens,
                    "output_tokens": result.total_usage.output_tokens,
                    "iterations": result.iterations,
                }))
            }
        }
    }

    fn api_agent_session(&self, agent_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, &format!("/api/agents/{agent_id}/session"))
            }
            McpBackend::InProcess { kernel, .. } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                let entry = kernel.registry.get(aid).ok_or("Agent not found")?;
                let session = kernel
                    .memory
                    .get_session(entry.session_id)
                    .map_err(|e| format!("{e}"))?
                    .ok_or("Session not found")?;
                let messages: Vec<Value> = session
                    .messages
                    .iter()
                    .filter_map(|m| serde_json::to_value(m).ok())
                    .collect();
                Ok(json!({
                    "session_id": session.id.to_string(),
                    "agent_id": session.agent_id.to_string(),
                    "message_count": messages.len(),
                    "context_window_tokens": session.context_window_tokens,
                    "label": session.label,
                    "messages": messages,
                }))
            }
        }
    }

    // ── Budget ─────────────────────────────────────────────────────────────

    fn api_budget_status(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/budget")
            }
            McpBackend::InProcess { kernel, .. } => {
                let budget = kernel.budget_config();
                let status = kernel.metering.budget_status(&budget);
                serde_json::to_value(&status).map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    fn api_budget_agents(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/budget/agents")
            }
            McpBackend::InProcess { kernel, .. } => {
                let agents = kernel.registry.list();
                let ranking: Vec<Value> = agents
                    .iter()
                    .map(|a| {
                        json!({
                            "agent_id": a.id.to_string(),
                            "name": &a.name,
                        })
                    })
                    .collect();
                Ok(json!(ranking))
            }
        }
    }

    // ── Workflow ────────────────────────────────────────────────────────────

    fn api_workflow_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/workflows")
            }
            McpBackend::InProcess { kernel, rt } => {
                let workflows = rt.block_on(kernel.workflows.list_workflows());
                serde_json::to_value(&workflows).map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    fn api_workflow_run(&self, workflow_id: &str, input: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                &format!("/api/workflows/{workflow_id}/run"),
                Some(&json!({"input": input})),
            ),
            McpBackend::InProcess { kernel, rt } => {
                let wid = rusty_hand_kernel::workflow::WorkflowId(
                    workflow_id.parse().map_err(|_| "Invalid workflow UUID")?,
                );
                let (run_id, output) = rt
                    .block_on(kernel.run_workflow(wid, input.to_string()))
                    .map_err(|e| format!("{e}"))?;
                Ok(json!({
                    "run_id": run_id.to_string(),
                    "output": output,
                    "status": "completed",
                }))
            }
        }
    }

    fn api_workflow_runs(&self, workflow_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_get(
                client,
                base_url,
                &format!("/api/workflows/{workflow_id}/runs"),
            ),
            McpBackend::InProcess { kernel, rt } => {
                let runs = rt.block_on(kernel.workflows.list_runs(None));
                serde_json::to_value(&runs).map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    // ── Other ──────────────────────────────────────────────────────────────

    fn api_channel_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/channels")
            }
            McpBackend::InProcess { kernel, .. } => serde_json::to_value(&kernel.config.channels)
                .map_err(|e| format!("Serialize error: {e}")),
        }
    }

    fn api_cron_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/cron/jobs")
            }
            McpBackend::InProcess { kernel, .. } => {
                let jobs = kernel.cron_scheduler.list_all_jobs();
                let items: Vec<Value> = jobs
                    .iter()
                    .filter_map(|j| serde_json::to_value(j).ok())
                    .collect();
                let total = items.len();
                Ok(json!({"jobs": items, "total": total}))
            }
        }
    }

    fn api_template_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/templates")
            }
            McpBackend::InProcess { .. } => {
                let agents_dir = rusty_hand_kernel::config::rusty_hand_home().join("agents");
                let mut templates = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&agents_dir) {
                    for entry in entries.flatten() {
                        let manifest_path = entry.path().join("agent.toml");
                        if manifest_path.exists() {
                            if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
                                if let Ok(manifest) = toml::from_str::<
                                    rusty_hand_types::agent::AgentManifest,
                                >(&contents)
                                {
                                    templates.push(json!({
                                        "name": manifest.name,
                                        "description": manifest.description,
                                    }));
                                }
                            }
                        }
                    }
                }
                let total = templates.len();
                Ok(json!({"templates": templates, "total": total}))
            }
        }
    }

    fn api_tool_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/tools")
            }
            McpBackend::InProcess { .. } => {
                let defs = rusty_hand_runtime::tool_runner::builtin_tool_definitions();
                let tools: Vec<Value> = defs
                    .iter()
                    .filter_map(|d| serde_json::to_value(d).ok())
                    .collect();
                let total = tools.len();
                Ok(json!({"tools": tools, "total": total}))
            }
        }
    }

    // ── New tools ─────────────────────────────────────────────────────────

    fn api_agent_get(&self, agent_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, &format!("/api/agents/{agent_id}"))
            }
            McpBackend::InProcess { kernel, .. } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                let entry = kernel.registry.get(aid).ok_or("Agent not found")?;
                serde_json::to_value(&entry).map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    fn api_agent_set_model(&self, agent_id: &str, model: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_put(
                client,
                base_url,
                &format!("/api/agents/{agent_id}/model"),
                Some(&json!({"model": model})),
            ),
            McpBackend::InProcess { kernel, .. } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                kernel
                    .set_agent_model(aid, model)
                    .map_err(|e| format!("{e}"))?;
                Ok(json!({"status": "ok", "model": model}))
            }
        }
    }

    fn api_agent_session_reset(&self, agent_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                &format!("/api/agents/{agent_id}/session/reset"),
                None,
            ),
            McpBackend::InProcess { kernel, .. } => {
                let aid: rusty_hand_types::agent::AgentId =
                    agent_id.parse().map_err(|_| "Invalid agent ID")?;
                kernel.reset_session(aid).map_err(|e| format!("{e}"))?;
                Ok(json!({"status": "reset", "agent_id": agent_id}))
            }
        }
    }

    fn api_provider_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/providers")
            }
            McpBackend::InProcess { kernel, .. } => {
                let catalog = kernel
                    .model_catalog
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                let providers: Vec<Value> = catalog
                    .list_providers()
                    .iter()
                    .map(|p| {
                        json!({
                            "id": p.id,
                            "display_name": p.display_name,
                            "auth_status": p.auth_status,
                            "model_count": p.model_count,
                            "key_required": p.key_required,
                            "api_key_env": p.api_key_env,
                        })
                    })
                    .collect();
                let total = providers.len();
                Ok(json!({"providers": providers, "total": total}))
            }
        }
    }

    fn api_model_list(&self, provider: Option<&str>) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                let path = if let Some(p) = provider {
                    format!("/api/models?available=true&provider={p}")
                } else {
                    "/api/models?available=true".to_string()
                };
                Self::daemon_get(client, base_url, &path)
            }
            McpBackend::InProcess { kernel, .. } => {
                let catalog = kernel
                    .model_catalog
                    .read()
                    .unwrap_or_else(|e| e.into_inner());
                let models: Vec<Value> = catalog
                    .available_models()
                    .iter()
                    .filter(|m| provider.is_none_or(|p| m.provider == p))
                    .map(|m| {
                        json!({
                            "id": m.id,
                            "display_name": m.display_name,
                            "provider": m.provider,
                            "tier": m.tier,
                            "context_window": m.context_window,
                            "input_cost_per_m": m.input_cost_per_m,
                            "output_cost_per_m": m.output_cost_per_m,
                            "supports_tools": m.supports_tools,
                            "supports_vision": m.supports_vision,
                        })
                    })
                    .collect();
                let total = models.len();
                Ok(json!({"models": models, "total": total}))
            }
        }
    }

    fn api_config_get(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/config")
            }
            McpBackend::InProcess { kernel, .. } => {
                serde_json::to_value(&kernel.config)
                    .map_err(|e| format!("Serialize error: {e}"))
            }
        }
    }

    fn api_config_set(&self, key: &str, value: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                "/api/config/set",
                Some(&json!({"key": key, "value": value})),
            ),
            McpBackend::InProcess { .. } => {
                Err("config_set not supported in in-process mode".to_string())
            }
        }
    }

    fn api_cron_create(
        &self,
        agent_id: &str,
        name: &str,
        schedule: &str,
        message: &str,
    ) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                "/api/cron/jobs",
                Some(&json!({
                    "agent_id": agent_id,
                    "name": name,
                    "enabled": true,
                    "schedule": { "kind": "cron", "expr": schedule },
                    "action": {
                        "kind": "agent_turn",
                        "message": message,
                        "timeout_secs": 300
                    },
                    "delivery": { "kind": "none" }
                })),
            ),
            McpBackend::InProcess { .. } => {
                Err("cron_create not supported in in-process mode".to_string())
            }
        }
    }

    fn api_cron_delete(&self, job_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_delete(client, base_url, &format!("/api/cron/jobs/{job_id}"))
            }
            McpBackend::InProcess { .. } => {
                Err("cron_delete not supported in in-process mode".to_string())
            }
        }
    }

    fn api_approval_list(&self) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => {
                Self::daemon_get(client, base_url, "/api/approvals")
            }
            McpBackend::InProcess { kernel, .. } => {
                let pending = kernel.approval_manager.list_pending();
                let items: Vec<Value> = pending
                    .iter()
                    .filter_map(|a| serde_json::to_value(a).ok())
                    .collect();
                Ok(json!({"approvals": items, "total": items.len()}))
            }
        }
    }

    fn api_approval_approve(&self, approval_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                &format!("/api/approvals/{approval_id}/approve"),
                Some(&json!({})),
            ),
            McpBackend::InProcess { kernel, .. } => {
                let aid: uuid::Uuid =
                    approval_id.parse().map_err(|_| "Invalid approval ID")?;
                kernel
                    .approval_manager
                    .resolve(
                        aid,
                        rusty_hand_types::approval::ApprovalDecision::Approved,
                        None,
                    )
                    .map_err(|e| e.to_string())?;
                Ok(json!({"status": "approved", "id": approval_id}))
            }
        }
    }

    fn api_approval_reject(&self, approval_id: &str) -> Result<Value, String> {
        match self {
            McpBackend::Daemon { base_url, client } => Self::daemon_post(
                client,
                base_url,
                &format!("/api/approvals/{approval_id}/reject"),
                Some(&json!({})),
            ),
            McpBackend::InProcess { kernel, .. } => {
                let aid: uuid::Uuid =
                    approval_id.parse().map_err(|_| "Invalid approval ID")?;
                kernel
                    .approval_manager
                    .resolve(
                        aid,
                        rusty_hand_types::approval::ApprovalDecision::Denied,
                        Some("Rejected via MCP".to_string()),
                    )
                    .map_err(|e| e.to_string())?;
                Ok(json!({"status": "rejected", "id": approval_id}))
            }
        }
    }
}

// ── MCP server main loop ───────────────────────────────────────────────────

/// Run the MCP server over stdio.
pub fn run_mcp_server(config: Option<std::path::PathBuf>) {
    let backend = create_backend(config);

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut writer = stdout.lock();

    loop {
        match read_message(&mut reader) {
            Ok(Some(msg)) => {
                let response = handle_message(&backend, &msg);
                if let Some(resp) = response {
                    write_message(&mut writer, &resp);
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }
}

fn create_backend(config: Option<std::path::PathBuf>) -> McpBackend {
    // Try daemon first
    if let Some(base_url) = super::find_daemon() {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");
        return McpBackend::Daemon { base_url, client };
    }

    // Fall back to in-process kernel
    let kernel = match RustyHandKernel::boot(config.as_deref()) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("Failed to boot kernel for MCP: {e}");
            std::process::exit(1);
        }
    };
    let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
    McpBackend::InProcess {
        kernel: Box::new(kernel),
        rt,
    }
}

// ── Content-Length framed I/O ──────────────────────────────────────────────

/// Read a Content-Length framed JSON-RPC message from the reader.
fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Value>> {
    // Read headers until empty line
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        let bytes_read = reader.read_line(&mut header)?;
        if bytes_read == 0 {
            return Ok(None); // EOF
        }

        let trimmed = header.trim();
        if trimmed.is_empty() {
            break; // End of headers
        }

        if let Some(len_str) = trimmed.strip_prefix("Content-Length: ") {
            content_length = len_str.parse().unwrap_or(0);
        }
    }

    if content_length == 0 {
        return Ok(None);
    }

    // SECURITY: Reject oversized messages to prevent OOM.
    const MAX_MCP_MESSAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB
    if content_length > MAX_MCP_MESSAGE_SIZE {
        // Drain the oversized body to avoid stream desync
        let mut discard = [0u8; 4096];
        let mut remaining = content_length;
        while remaining > 0 {
            let to_read = remaining.min(4096);
            if reader.read_exact(&mut discard[..to_read]).is_err() {
                break;
            }
            remaining -= to_read;
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("MCP message too large: {content_length} bytes (max {MAX_MCP_MESSAGE_SIZE})"),
        ));
    }

    // Read the body
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body)?;

    match serde_json::from_slice(&body) {
        Ok(v) => Ok(Some(v)),
        Err(_) => Ok(None),
    }
}

/// Write a Content-Length framed JSON-RPC response to the writer.
fn write_message(writer: &mut impl Write, msg: &Value) {
    let body = serde_json::to_string(msg).unwrap_or_default();
    let _ = write!(writer, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = writer.flush();
}

// ── JSON-RPC message handling ──────────────────────────────────────────────

/// Handle a JSON-RPC message and return an optional response.
fn handle_message(backend: &McpBackend, msg: &Value) -> Option<Value> {
    let method = msg["method"].as_str().unwrap_or("");
    let id = msg.get("id").cloned();

    match method {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "rustyhand",
                    "version": "0.1.0"
                }
            });
            Some(jsonrpc_response(id?, result))
        }

        "notifications/initialized" => None, // Notification, no response

        "tools/list" => {
            // Static tools (21)
            let mut tools = static_tool_definitions();

            // Dynamic per-agent tools (backward compatible)
            let agents = backend.list_agents();
            for (_, name, description) in &agents {
                let tool_name = format!("rusty_hand_agent_{}", name.replace('-', "_"));
                let desc = if description.is_empty() {
                    format!("Send a message to RustyHand agent '{name}'")
                } else {
                    description.clone()
                };
                tools.push(json!({
                    "name": tool_name,
                    "description": desc,
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "message": {
                                "type": "string",
                                "description": "Message to send to the agent"
                            }
                        },
                        "required": ["message"]
                    }
                }));
            }

            Some(jsonrpc_response(id?, json!({ "tools": tools })))
        }

        "tools/call" => {
            let params = &msg["params"];
            let tool_name = params["name"].as_str().unwrap_or("");
            let args = &params["arguments"];

            // Try dynamic per-agent tool first (rusty_hand_agent_*)
            if tool_name.starts_with("rusty_hand_agent_") {
                let message = args["message"].as_str().unwrap_or("").to_string();
                if message.is_empty() {
                    return Some(jsonrpc_error(id?, -32602, "Missing 'message' argument"));
                }
                let agent_id = match backend.resolve_tool_agent(tool_name) {
                    Some(id) => id,
                    None => {
                        return Some(jsonrpc_error(
                            id?,
                            -32602,
                            &format!("Unknown agent tool: {tool_name}"),
                        ));
                    }
                };
                return match backend.send_message(&agent_id, &message) {
                    Ok(response) => Some(jsonrpc_response(
                        id?,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": response
                            }]
                        }),
                    )),
                    Err(e) => Some(jsonrpc_response(
                        id?,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {e}")
                            }],
                            "isError": true
                        }),
                    )),
                };
            }

            // Static tools
            match dispatch_tool(backend, tool_name, args) {
                Ok(result) => {
                    let text = serde_json::to_string_pretty(&result).unwrap_or_default();
                    Some(jsonrpc_response(
                        id?,
                        json!({
                            "content": [{
                                "type": "text",
                                "text": text
                            }]
                        }),
                    ))
                }
                Err(e) => Some(jsonrpc_response(
                    id?,
                    json!({
                        "content": [{
                            "type": "text",
                            "text": format!("Error: {e}")
                        }],
                        "isError": true
                    }),
                )),
            }
        }

        _ => {
            // Unknown method
            id.map(|id| jsonrpc_error(id, -32601, &format!("Method not found: {method}")))
        }
    }
}

fn jsonrpc_response(id: Value, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result
    })
}

fn jsonrpc_error(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_backend() -> McpBackend {
        McpBackend::Daemon {
            base_url: "http://localhost:9999".to_string(),
            client: reqwest::blocking::Client::new(),
        }
    }

    #[test]
    fn test_handle_initialize() {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });
        let backend = test_backend();
        let resp = handle_message(&backend, &msg).unwrap();
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(resp["result"]["serverInfo"]["name"], "rustyhand");
    }

    #[test]
    fn test_handle_notifications_initialized() {
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        let backend = test_backend();
        let resp = handle_message(&backend, &msg);
        assert!(resp.is_none()); // No response for notifications
    }

    #[test]
    fn test_handle_unknown_method() {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "unknown/method"
        });
        let backend = test_backend();
        let resp = handle_message(&backend, &msg).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
    }

    #[test]
    fn test_jsonrpc_response() {
        let resp = jsonrpc_response(json!(1), json!({"status": "ok"}));
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["status"], "ok");
    }

    #[test]
    fn test_jsonrpc_error() {
        let resp = jsonrpc_error(json!(2), -32601, "Not found");
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 2);
        assert_eq!(resp["error"]["code"], -32601);
        assert_eq!(resp["error"]["message"], "Not found");
    }

    #[test]
    fn test_read_message() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let mut reader = io::BufReader::new(input.as_bytes());
        let msg = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(msg["method"], "initialize");
        assert_eq!(msg["id"], 1);
    }

    // ── New tests ──────────────────────────────────────────────────────────

    #[test]
    fn test_tools_list_includes_static_tools() {
        let defs = static_tool_definitions();
        assert_eq!(
            defs.len(),
            29,
            "Expected exactly 29 static tool definitions"
        );

        let expected = [
            "rustyhand_system_health",
            "rustyhand_system_status",
            "rustyhand_config_reload",
            "rustyhand_config_get",
            "rustyhand_config_set",
            "rustyhand_agent_list",
            "rustyhand_agent_get",
            "rustyhand_agent_spawn",
            "rustyhand_agent_kill",
            "rustyhand_agent_message",
            "rustyhand_agent_session",
            "rustyhand_agent_set_model",
            "rustyhand_agent_session_reset",
            "rustyhand_provider_list",
            "rustyhand_model_list",
            "rustyhand_budget_status",
            "rustyhand_budget_agents",
            "rustyhand_workflow_list",
            "rustyhand_workflow_run",
            "rustyhand_workflow_runs",
            "rustyhand_cron_list",
            "rustyhand_cron_create",
            "rustyhand_cron_delete",
            "rustyhand_approval_list",
            "rustyhand_approval_approve",
            "rustyhand_approval_reject",
            "rustyhand_channel_list",
            "rustyhand_template_list",
            "rustyhand_tool_list",
        ];
        let names: Vec<&str> = defs.iter().map(|d| d["name"].as_str().unwrap()).collect();
        for name in &expected {
            assert!(names.contains(name), "Missing static tool: {name}");
        }
    }

    #[test]
    fn test_tool_names_are_unique() {
        let defs = static_tool_definitions();
        let mut seen = std::collections::HashSet::new();
        for def in &defs {
            let name = def["name"].as_str().unwrap();
            assert!(seen.insert(name), "Duplicate tool name: {name}");
        }
    }

    #[test]
    fn test_dispatch_unknown_tool() {
        let backend = test_backend();
        let result = dispatch_tool(&backend, "nonexistent_tool", &json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unknown tool"));
    }

    #[test]
    fn test_dispatch_missing_required_arg() {
        let backend = test_backend();
        // agent_kill requires "agent_id"
        let result = dispatch_tool(&backend, "rustyhand_agent_kill", &json!({}));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("agent_id"));
    }

    #[test]
    fn test_require_str() {
        let args = json!({"name": "test", "count": 42});
        assert_eq!(require_str(&args, "name").unwrap(), "test");
        assert!(require_str(&args, "missing").is_err());
        // Non-string value
        assert!(require_str(&args, "count").is_err());
    }

    #[test]
    fn test_static_tools_have_input_schema() {
        let defs = static_tool_definitions();
        for def in &defs {
            let name = def["name"].as_str().unwrap();
            assert!(
                def.get("inputSchema").is_some(),
                "Tool {name} missing inputSchema"
            );
            assert_eq!(
                def["inputSchema"]["type"], "object",
                "Tool {name} inputSchema type must be 'object'"
            );
        }
    }

    #[test]
    fn test_tools_list_response_includes_static() {
        let backend = test_backend();
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/list",
            "params": {}
        });
        let resp = handle_message(&backend, &msg).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        // Should have at least 29 static tools (dynamic agents may add more)
        assert!(
            tools.len() >= 29,
            "Expected >= 29 tools, got {}",
            tools.len()
        );
        // Verify a few specific static tools are present
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(names.contains(&"rustyhand_system_health"));
        assert!(names.contains(&"rustyhand_agent_list"));
        assert!(names.contains(&"rustyhand_tool_list"));
    }
}
