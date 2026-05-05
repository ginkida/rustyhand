//! Route handlers for the RustyHand API.

use crate::types::*;
use axum::body::Bytes;
use axum::extract::{Extension, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use axum::Json;
use dashmap::DashMap;
use rusty_hand_kernel::triggers::{TriggerId, TriggerPattern};
use rusty_hand_kernel::workflow::{
    ErrorMode, StepAgent, StepMode, Workflow, WorkflowId, WorkflowStep,
};
use rusty_hand_kernel::RustyHandKernel;
use rusty_hand_runtime::kernel_handle::KernelHandle;
use rusty_hand_runtime::tool_runner::builtin_tool_definitions;
use rusty_hand_types::agent::{normalize_agent_group, AgentId, AgentIdentity, AgentManifest};
use rusty_hand_types::error::RustyHandError;
use rusty_hand_types::memory::{ExportFormat, Memory};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};
use std::time::Instant;

/// Shared application state.
///
/// The kernel is wrapped in Arc so it can serve as both the main kernel
/// and the KernelHandle for inter-agent tool access.
pub struct AppState {
    pub kernel: Arc<RustyHandKernel>,
    pub started_at: Instant,
    /// Channel bridge manager — held behind a Mutex so it can be swapped on hot-reload.
    pub bridge_manager: tokio::sync::Mutex<Option<rusty_hand_channels::bridge::BridgeManager>>,
    /// Live channel config — updated on every hot-reload so list_channels() reflects reality.
    pub channels_config: tokio::sync::RwLock<rusty_hand_types::config::ChannelsConfig>,
    /// Notify handle to trigger graceful HTTP server shutdown from the API.
    pub shutdown_notify: Arc<tokio::sync::Notify>,
    /// Allowed origins for WebSocket upgrade validation (prevents cross-site WS hijacking).
    pub allowed_ws_origins: Vec<String>,
}

/// Parse an agent ID from a path parameter, returning a 400 error with debug logging on failure.
fn parse_agent_id(id: &str) -> Result<AgentId, (StatusCode, Json<serde_json::Value>)> {
    id.parse().map_err(|_| {
        tracing::debug!(raw_id = %id, "Rejected invalid agent ID");
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid agent ID"})),
        )
    })
}

/// Return a 404 for a missing agent, with debug logging.
fn agent_not_found(id: &str) -> (StatusCode, Json<serde_json::Value>) {
    tracing::debug!(agent_id = %id, "Agent not found");
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({"error": "Agent not found"})),
    )
}

/// Create a sanitized JSON error response safe for external clients.
///
/// Logs the full error detail server-side with a correlation ID,
/// then returns a generic message to the client. The correlation ID is
/// included in 500-level responses so operators can find the cause in logs.
/// Standard pagination query parameters for list endpoints.
#[derive(serde::Deserialize)]
pub struct PaginationQuery {
    /// Maximum number of items to return (default 100, max 500).
    pub limit: Option<usize>,
    /// Number of items to skip (default 0).
    pub offset: Option<usize>,
}

impl PaginationQuery {
    fn limit(&self) -> usize {
        self.limit.unwrap_or(100).min(500)
    }
    fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }
}

fn safe_error(
    status: StatusCode,
    context: &str,
    error: &dyn std::fmt::Display,
) -> (StatusCode, Json<serde_json::Value>) {
    let error_id = uuid::Uuid::new_v4();
    tracing::error!(
        error_id = %error_id,
        context = %context,
        error = %error,
        "Request failed"
    );
    let msg = match status {
        StatusCode::BAD_REQUEST => format!("{context}: invalid request"),
        StatusCode::NOT_FOUND => format!("{context}: not found"),
        _ => format!("{context} failed (ref: {error_id})"),
    };
    (status, Json(serde_json::json!({ "error": msg })))
}

#[derive(serde::Deserialize)]
pub struct BackupQuery {
    format: Option<String>,
}

/// POST /api/agents — Spawn a new agent.
pub async fn spawn_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SpawnRequest>,
) -> impl IntoResponse {
    // SECURITY: Reject oversized manifests to prevent parser memory exhaustion.
    const MAX_MANIFEST_SIZE: usize = 1024 * 1024; // 1MB
    if req.manifest_toml.len() > MAX_MANIFEST_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Manifest too large (max 1MB)"})),
        );
    }

    // SECURITY: Verify Ed25519 signature when a signed manifest is provided
    if let Some(ref signed_json) = req.signed_manifest {
        match state.kernel.verify_signed_manifest(signed_json) {
            Ok(verified_toml) => {
                // Ensure the signed manifest matches the provided manifest_toml
                if verified_toml.trim() != req.manifest_toml.trim() {
                    tracing::warn!("Signed manifest content does not match manifest_toml");
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(
                            serde_json::json!({"error": "Signed manifest content does not match manifest_toml"}),
                        ),
                    );
                }
            }
            Err(e) => {
                tracing::warn!("Manifest signature verification failed: {e}");
                state.kernel.audit_log.record(
                    "system",
                    rusty_hand_runtime::audit::AuditAction::AuthAttempt,
                    "manifest signature verification failed",
                    format!("error: {e}"),
                );
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error": "Manifest signature verification failed"})),
                );
            }
        }
    }

    let manifest: AgentManifest = match toml::from_str(&req.manifest_toml) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Invalid manifest TOML: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid manifest format"})),
            );
        }
    };

    let name = manifest.name.clone();
    match state.kernel.spawn_agent(manifest) {
        Ok(id) => (
            StatusCode::CREATED,
            Json(serde_json::json!(SpawnResponse {
                agent_id: id.to_string(),
                name,
            })),
        ),
        Err(rusty_hand_kernel::error::KernelError::RustyHand(RustyHandError::InvalidInput(
            message,
        ))) => {
            tracing::warn!("Spawn failed: {message}");
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": message})),
            )
        }
        Err(e) => {
            tracing::warn!("Spawn failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Agent spawn failed"})),
            )
        }
    }
}

/// GET /api/agents — List all agents.
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> impl IntoResponse {
    // Batch-load all session previews in ONE query (eliminates N+1)
    let previews = state
        .kernel
        .memory
        .get_session_previews_batch()
        .unwrap_or_default();

    let agents: Vec<serde_json::Value> = state
        .kernel
        .registry
        .list()
        .into_iter()
        .map(|e| {
            // Look up preview from batch-loaded map instead of per-agent query
            let agent_id_str = e.id.to_string();
            let (last_message_preview, last_activity) = previews
                .get(&agent_id_str)
                .map(|(preview, updated_at)| (preview.clone(), updated_at.clone()))
                .unwrap_or_else(|| (String::new(), e.created_at.to_rfc3339()));

            serde_json::json!({
                "id": e.id.to_string(),
                "name": e.name,
                "state": format!("{:?}", e.state).to_lowercase(),
                "mode": e.mode,
                "created_at": e.created_at.to_rfc3339(),
                "group": e.manifest.group,
                "description": e.manifest.description,
                "system_prompt": e.manifest.model.system_prompt,
                "model_provider": e.manifest.model.provider,
                "model_name": e.manifest.model.model,
                "profile": e.manifest.profile,
                "identity": {
                    "emoji": e.identity.emoji,
                    "avatar_url": e.identity.avatar_url,
                    "color": e.identity.color,
                    "archetype": e.identity.archetype,
                    "vibe": e.identity.vibe,
                    "greeting_style": e.identity.greeting_style,
                },
                "last_message_preview": last_message_preview,
                "last_activity": last_activity,
            })
        })
        .collect();

    // Apply pagination
    let total = agents.len();
    let offset = pagination.offset();
    let limit = pagination.limit();
    let paginated: Vec<_> = agents.into_iter().skip(offset).take(limit).collect();

    Json(serde_json::json!({
        "agents": paginated,
        "total": total,
        "offset": offset,
        "limit": limit,
    }))
}

/// GET /api/agents/export — Export all agent manifests as JSON for backup/migration.
pub async fn export_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents: Vec<serde_json::Value> = state
        .kernel
        .registry
        .list()
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "name": e.name,
                "state": format!("{:?}", e.state).to_lowercase(),
                "mode": e.mode,
                "group": e.manifest.group,
                "description": e.manifest.description,
                "model": {
                    "provider": e.manifest.model.provider,
                    "model": e.manifest.model.model,
                    "system_prompt": e.manifest.model.system_prompt,
                    "temperature": e.manifest.model.temperature,
                    "max_tokens": e.manifest.model.max_tokens,
                },
                "profile": e.manifest.profile,
                "skills": e.manifest.skills,
                "identity": {
                    "emoji": e.identity.emoji,
                    "color": e.identity.color,
                    "archetype": e.identity.archetype,
                    "vibe": e.identity.vibe,
                },
                "resources": {
                    "max_cost_per_hour_usd": e.manifest.resources.max_cost_per_hour_usd,
                    "max_cost_per_day_usd": e.manifest.resources.max_cost_per_day_usd,
                },
                "created_at": e.created_at.to_rfc3339(),
            })
        })
        .collect();

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("rustyhand-agents-{timestamp}.json");

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "application/json".to_string(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        Json(serde_json::json!({
            "version": "1.0",
            "exported_at": chrono::Utc::now().to_rfc3339(),
            "agent_count": agents.len(),
            "agents": agents,
        })),
    )
}

/// POST /api/agents/import — Bulk import agents from a JSON export.
pub async fn import_agents(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agents = match req.get("agents").and_then(|a| a.as_array()) {
        Some(a) => a,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'agents' array in import data"})),
            );
        }
    };

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();

    // Snapshot existing agent names once to avoid repeated registry scans.
    let existing_names: std::collections::HashSet<String> = state
        .kernel
        .registry
        .list()
        .into_iter()
        .map(|e| e.name)
        .collect();
    // Track names imported in this batch so duplicate entries in the same file are skipped.
    let mut seen_in_batch: std::collections::HashSet<String> = std::collections::HashSet::new();

    for agent_json in agents {
        let name = agent_json["name"].as_str().unwrap_or("imported-agent");
        let provider = agent_json["model"]["provider"]
            .as_str()
            .unwrap_or("anthropic");
        let model = agent_json["model"]["model"]
            .as_str()
            .unwrap_or("claude-sonnet-4-20250514");
        let system_prompt = agent_json["model"]["system_prompt"].as_str().unwrap_or("");
        let group = agent_json["group"].as_str().unwrap_or("");
        let description = agent_json["description"].as_str().unwrap_or("");

        // Deduplicate: skip if an agent with this name already exists or was just imported.
        if existing_names.contains(name) || !seen_in_batch.insert(name.to_string()) {
            skipped += 1;
            errors.push(format!("Skipped '{}' — name already exists", name));
            continue;
        }

        #[allow(clippy::field_reassign_with_default)]
        let manifest = {
            let mut m = rusty_hand_types::agent::AgentManifest::default();
            m.name = name.to_string();
            m.model.provider = provider.to_string();
            m.model.model = model.to_string();
            m.model.system_prompt = system_prompt.to_string();
            m.group = Some(group.to_string());
            m.description = description.to_string();
            m
        };

        match state.kernel.spawn_agent(manifest) {
            Ok(id) => {
                imported += 1;
                tracing::info!(agent_id = %id, name, "Imported agent");
            }
            Err(e) => {
                errors.push(format!("Failed to import '{}': {}", name, e));
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "imported": imported,
            "skipped": skipped,
            "errors": errors,
            "total_in_file": agents.len(),
        })),
    )
}

/// GET /api/agents/search — Server-side agent search with filters.
///
/// Query params: q (text search), model (provider filter), state (Running/Idle/Suspended),
/// group (group name filter).
pub async fn search_agents(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").map(|s| s.to_lowercase());
    let model_filter = params.get("model").map(|s| s.to_lowercase());
    let state_filter = params.get("state").map(|s| s.to_lowercase());
    let group_filter = params.get("group").cloned();

    let agents: Vec<serde_json::Value> = state
        .kernel
        .registry
        .list()
        .into_iter()
        .filter(|e| {
            // Text search across name, description, group, model
            if let Some(ref q) = query {
                let name_match = e.name.to_lowercase().contains(q);
                let desc_match = e.manifest.description.to_lowercase().contains(q);
                let group_match = e
                    .manifest
                    .group
                    .as_deref()
                    .map(|g| g.to_lowercase().contains(q))
                    .unwrap_or(false);
                let model_match = e.manifest.model.model.to_lowercase().contains(q)
                    || e.manifest.model.provider.to_lowercase().contains(q);
                if !(name_match || desc_match || group_match || model_match) {
                    return false;
                }
            }
            // Model/provider filter
            if let Some(ref m) = model_filter {
                if !e.manifest.model.provider.to_lowercase().contains(m)
                    && !e.manifest.model.model.to_lowercase().contains(m)
                {
                    return false;
                }
            }
            // State filter
            if let Some(ref s) = state_filter {
                let agent_state = format!("{:?}", e.state).to_lowercase();
                if !agent_state.contains(s) {
                    return false;
                }
            }
            // Group filter
            if let Some(ref g) = group_filter {
                let agent_group = e.manifest.group.as_deref().unwrap_or("");
                if agent_group != g {
                    return false;
                }
            }
            true
        })
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "name": e.name,
                "state": format!("{:?}", e.state).to_lowercase(),
                "group": e.manifest.group,
                "model_provider": e.manifest.model.provider,
                "model_name": e.manifest.model.model,
                "description": e.manifest.description,
            })
        })
        .collect();

    Json(serde_json::json!({
        "results": agents,
        "total": agents.len(),
    }))
}

/// GET /api/search?q=<query>&limit=20 — Full-text search across all sessions.
pub async fn search_sessions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params
        .get("q")
        .map(|s| s.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    if q.len() < 2 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Query must be at least 2 characters"})),
        );
    }
    let limit: usize = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20)
        .min(50);

    match state.kernel.memory.search_sessions(&q, limit) {
        Ok(results) => (
            StatusCode::OK,
            Json(serde_json::json!({"results": results, "query": q})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Search failed: {e}")})),
        ),
    }
}

/// Resolve uploaded file attachments into ContentBlock::Image blocks.
///
/// Reads each file from the upload directory, base64-encodes it, and
/// returns image content blocks ready to insert into a session message.
pub fn resolve_attachments(
    attachments: &[AttachmentRef],
) -> Vec<rusty_hand_types::message::ContentBlock> {
    use base64::Engine;

    let upload_dir = std::env::temp_dir().join("rusty_hand_uploads");
    let mut blocks = Vec::new();

    for att in attachments {
        // Look up metadata from the upload registry
        let meta = UPLOAD_REGISTRY.get(&att.file_id);
        let content_type = if let Some(ref m) = meta {
            m.content_type.clone()
        } else if !att.content_type.is_empty() {
            att.content_type.clone()
        } else {
            continue; // Skip unknown attachments
        };

        // Only process image types
        if !content_type.starts_with("image/") {
            continue;
        }

        // Validate file_id is a UUID to prevent path traversal
        if uuid::Uuid::parse_str(&att.file_id).is_err() {
            continue;
        }

        let file_path = upload_dir.join(&att.file_id);
        match std::fs::read(&file_path) {
            Ok(data) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&data);
                blocks.push(rusty_hand_types::message::ContentBlock::Image {
                    media_type: content_type,
                    data: b64,
                });
            }
            Err(e) => {
                tracing::warn!(file_id = %att.file_id, error = %e, "Failed to read upload for attachment");
            }
        }
    }

    blocks
}

/// Pre-insert image attachments into an agent's session so the LLM can see them.
///
/// This injects image content blocks into the session BEFORE the kernel
/// adds the text user message, so the LLM receives: [..., User(images), User(text)].
pub fn inject_attachments_into_session(
    kernel: &RustyHandKernel,
    agent_id: AgentId,
    image_blocks: Vec<rusty_hand_types::message::ContentBlock>,
) {
    use rusty_hand_types::message::{Message, MessageContent, Role};

    let entry = match kernel.registry.get(agent_id) {
        Some(e) => e,
        None => return,
    };

    let mut session = match kernel.memory.get_session(entry.session_id) {
        Ok(Some(s)) => s,
        _ => rusty_hand_memory::session::Session {
            id: entry.session_id,
            agent_id,
            messages: Vec::new(),
            context_window_tokens: 0,
            label: None,
        },
    };

    session.messages.push(Message {
        role: Role::User,
        content: MessageContent::Blocks(image_blocks),
    });

    if let Err(e) = kernel.memory.save_session(&session) {
        tracing::warn!(error = %e, "Failed to save session with image attachments");
    }
}

/// POST /api/agents/:id/message — Send a message to an agent.
pub async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<MessageRequest>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    // Reject empty/whitespace-only messages
    if req.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message cannot be empty"})),
        );
    }

    // SECURITY: Reject oversized messages to prevent OOM / LLM token abuse.
    const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64KB
    if req.message.len() > MAX_MESSAGE_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Message too large (max 64KB)"})),
        );
    }

    // Resolve file attachments into image content blocks
    if !req.attachments.is_empty() {
        let image_blocks = resolve_attachments(&req.attachments);
        if !image_blocks.is_empty() {
            inject_attachments_into_session(&state.kernel, agent_id, image_blocks);
        }
    }

    let kernel_handle: Arc<dyn KernelHandle> = state.kernel.clone() as Arc<dyn KernelHandle>;
    match state
        .kernel
        .send_message_with_handle(agent_id, &req.message, Some(kernel_handle))
        .await
    {
        Ok(result) => {
            // Guard: ensure we never return an empty response to the client
            let response = if result.response.trim().is_empty() {
                format!(
                    "[The agent completed processing but returned no text response. ({} in / {} out | {} iter)]",
                    result.total_usage.input_tokens,
                    result.total_usage.output_tokens,
                    result.iterations,
                )
            } else {
                result.response
            };
            (
                StatusCode::OK,
                Json(serde_json::json!(MessageResponse {
                    response,
                    input_tokens: result.total_usage.input_tokens,
                    output_tokens: result.total_usage.output_tokens,
                    iterations: result.iterations,
                })),
            )
        }
        Err(e) => {
            tracing::warn!("send_message failed for agent {id}: {e}");
            safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Message delivery", &e)
        }
    }
}

/// GET /api/agents/:id/session — Get agent session (conversation history).
pub async fn get_agent_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => return agent_not_found(&id),
    };

    match state.kernel.memory.get_session(entry.session_id) {
        Ok(Some(session)) => {
            // First pass: collect tool results keyed by tool_use_id so we can
            // attach them to the ToolUse entries which live in a different message.
            let mut tool_results: std::collections::HashMap<String, (String, bool)> =
                std::collections::HashMap::new();
            for m in &session.messages {
                if let rusty_hand_types::message::MessageContent::Blocks(blocks) = &m.content {
                    for b in blocks {
                        if let rusty_hand_types::message::ContentBlock::ToolResult {
                            tool_use_id,
                            content: result,
                            is_error,
                        } = b
                        {
                            let preview: String = result.chars().take(2000).collect();
                            tool_results.insert(tool_use_id.clone(), (preview, *is_error));
                        }
                    }
                }
            }

            // Second pass: build the output messages, pairing ToolUse with results.
            let messages: Vec<serde_json::Value> = session
                .messages
                .iter()
                .filter_map(|m| {
                    let mut tools: Vec<serde_json::Value> = Vec::new();
                    let content = match &m.content {
                        rusty_hand_types::message::MessageContent::Text(t) => t.clone(),
                        rusty_hand_types::message::MessageContent::Blocks(blocks) => {
                            let mut texts = Vec::new();
                            for b in blocks {
                                match b {
                                    rusty_hand_types::message::ContentBlock::Text { text } => {
                                        texts.push(text.clone());
                                    }
                                    rusty_hand_types::message::ContentBlock::Image { .. } => {
                                        texts.push("[Image]".to_string());
                                    }
                                    rusty_hand_types::message::ContentBlock::ToolUse {
                                        id,
                                        name,
                                        input,
                                    } => {
                                        let input_str =
                                            serde_json::to_string(input).unwrap_or_default();
                                        let mut tool = serde_json::json!({
                                            "name": name,
                                            "input": input_str,
                                            "running": false,
                                            "expanded": false,
                                        });
                                        // Attach result from the paired ToolResult message
                                        if let Some((result, is_error)) = tool_results.get(id) {
                                            tool["result"] =
                                                serde_json::Value::String(result.clone());
                                            tool["is_error"] = serde_json::Value::Bool(*is_error);
                                        }
                                        tools.push(tool);
                                    }
                                    rusty_hand_types::message::ContentBlock::ToolResult {
                                        ..
                                    } => {
                                        // Results are attached to ToolUse entries above
                                    }
                                    _ => {}
                                }
                            }
                            texts.join("\n")
                        }
                    };
                    // Skip messages that are purely tool results (User role with only ToolResult blocks)
                    if content.is_empty() && tools.is_empty() {
                        return None;
                    }
                    let mut msg = serde_json::json!({
                        "role": format!("{:?}", m.role),
                        "content": content,
                    });
                    if !tools.is_empty() {
                        msg["tools"] = serde_json::Value::Array(tools);
                    }
                    Some(msg)
                })
                .collect();
            // Compute context pressure so the UI can initialise correctly on load
            let context_pressure = if let Some(entry) = state.kernel.registry.get(agent_id) {
                use rusty_hand_runtime::compactor::{generate_context_report, ContextPressure};
                let tools = rusty_hand_runtime::tool_runner::builtin_tool_definitions();
                let cw = if session.context_window_tokens > 0 {
                    session.context_window_tokens as usize
                } else {
                    200_000
                };
                let report = generate_context_report(
                    &session.messages,
                    Some(&entry.manifest.model.system_prompt),
                    Some(&tools),
                    cw,
                );
                match report.pressure {
                    ContextPressure::Low => "low",
                    ContextPressure::Medium => "medium",
                    ContextPressure::High => "high",
                    ContextPressure::Critical => "critical",
                }
            } else {
                "low"
            };
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "session_id": session.id.0.to_string(),
                    "agent_id": session.agent_id.0.to_string(),
                    "message_count": session.messages.len(),
                    "context_window_tokens": session.context_window_tokens,
                    "context_pressure": context_pressure,
                    "label": session.label,
                    "messages": messages,
                })),
            )
        }
        Ok(None) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": entry.session_id.0.to_string(),
                "agent_id": agent_id.to_string(),
                "message_count": 0,
                "context_window_tokens": 0,
                "messages": [],
            })),
        ),
        Err(e) => {
            tracing::warn!("Session load failed for agent {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Session load failed"})),
            )
        }
    }
}

/// DELETE /api/agents/:id — Kill an agent.
pub async fn kill_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    match state.kernel.kill_agent(agent_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "killed", "agent_id": id})),
        ),
        Err(e) => {
            tracing::warn!("kill_agent failed for {id}: {e}");
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found or already terminated"})),
            )
        }
    }
}

/// POST /api/agents/:id/restart — Stop an agent and re-spawn with the same manifest.
///
/// Unlike DELETE (kill), this preserves session history — old sessions remain
/// in the DB and are still retrievable. The new agent starts fresh.
/// Returns both old and new agent IDs so callers can reconnect.
pub async fn restart_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    match state.kernel.restart_agent(agent_id) {
        Ok((old_id, new_id)) => {
            let name = state
                .kernel
                .registry
                .get(new_id)
                .map(|e| e.name)
                .unwrap_or_default();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "restarted",
                    "old_agent_id": old_id.to_string(),
                    "new_agent_id": new_id.to_string(),
                    "name": name,
                })),
            )
        }
        Err(rusty_hand_kernel::error::KernelError::RustyHand(
            rusty_hand_types::error::RustyHandError::AgentNotFound(_),
        )) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Agent '{id}' not found")})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Agent restart", &e),
    }
}

/// GET /api/status — Kernel status.
pub async fn status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let uptime = state.started_at.elapsed().as_secs();
    let entries = state.kernel.registry.list();
    let agents: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "name": e.name,
                "state": format!("{:?}", e.state).to_lowercase(),
                "model_provider": e.manifest.model.provider,
                "model_name": e.manifest.model.model,
                "group": e.manifest.group,
            })
        })
        .collect();
    let running_count = entries
        .iter()
        .filter(|e| e.state == rusty_hand_types::agent::AgentState::Running)
        .count();

    Json(serde_json::json!({
        "status": "running",
        "version": env!("CARGO_PKG_VERSION"),
        "agent_count": agents.len(),
        "running_count": running_count,
        "default_provider": state.kernel.config.default_model.provider,
        "default_model": state.kernel.config.default_model.model,
        "uptime_seconds": uptime,
        "data_dir": state.kernel.config.data_dir.display().to_string(),
        "agents": agents,
    }))
}

/// GET /api/auth/me — Current authenticated principal.
pub async fn auth_me(
    Extension(user): Extension<crate::middleware::AuthenticatedUser>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "authenticated": true,
        "user_id": user.user_id.map(|id| id.to_string()),
        "name": user.name,
        "role": user.role.to_string(),
        "source": user.source.to_string(),
    }))
}

/// GET /api/auth/users — List configured RBAC users (hashes never exposed).
pub async fn list_auth_users(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut users = state
        .kernel
        .auth
        .list_users()
        .into_iter()
        .map(|user| {
            serde_json::json!({
                "user_id": user.id.to_string(),
                "name": user.name,
                "role": user.role.to_string(),
            })
        })
        .collect::<Vec<_>>();
    users.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));

    Json(serde_json::json!({ "users": users }))
}

/// POST /api/shutdown — Graceful shutdown.
pub async fn shutdown(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<crate::middleware::AuthenticatedUser>>,
) -> impl IntoResponse {
    tracing::info!("Shutdown requested via API");
    let actor = actor
        .map(|Extension(user)| user.name)
        .unwrap_or_else(|| "system".to_string());
    // SECURITY: Record shutdown in audit trail
    state.kernel.audit_log.record(
        actor,
        rusty_hand_runtime::audit::AuditAction::ConfigChange,
        "shutdown requested via API",
        "ok",
    );
    // Drain MCP connections first (async), then fall through to sync shutdown.
    state.kernel.close_mcp_connections().await;
    state.kernel.shutdown();
    // Signal the HTTP server to initiate graceful shutdown so the process exits.
    state.shutdown_notify.notify_one();
    Json(serde_json::json!({"status": "shutting_down"}))
}

// ---------------------------------------------------------------------------
// Workflow routes
// ---------------------------------------------------------------------------

/// POST /api/workflows — Register a new workflow.
pub async fn create_workflow(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = req["name"].as_str().unwrap_or("unnamed").to_string();
    let description = req["description"].as_str().unwrap_or("").to_string();

    let steps_json = match req["steps"].as_array() {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'steps' array"})),
            );
        }
    };

    let mut steps = Vec::new();
    for s in steps_json {
        let step_name = s["name"].as_str().unwrap_or("step").to_string();
        let agent = if let Some(id) = s["agent_id"].as_str() {
            StepAgent::ById { id: id.to_string() }
        } else if let Some(name) = s["agent_name"].as_str() {
            StepAgent::ByName {
                name: name.to_string(),
            }
        } else {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({"error": format!("Step '{}' needs 'agent_id' or 'agent_name'", step_name)}),
                ),
            );
        };

        let mode = match s["mode"].as_str().unwrap_or("sequential") {
            "fan_out" => StepMode::FanOut,
            "collect" => StepMode::Collect,
            "conditional" => StepMode::Conditional {
                condition: s["condition"].as_str().unwrap_or("").to_string(),
            },
            "loop" => StepMode::Loop {
                max_iterations: s["max_iterations"].as_u64().unwrap_or(5) as u32,
                until: s["until"].as_str().unwrap_or("").to_string(),
            },
            _ => StepMode::Sequential,
        };

        let error_mode = match s["error_mode"].as_str().unwrap_or("fail") {
            "skip" => ErrorMode::Skip,
            "retry" => ErrorMode::Retry {
                max_retries: s["max_retries"].as_u64().unwrap_or(3) as u32,
            },
            _ => ErrorMode::Fail,
        };

        steps.push(WorkflowStep {
            name: step_name,
            agent,
            prompt_template: s["prompt"].as_str().unwrap_or("{{input}}").to_string(),
            mode,
            timeout_secs: s["timeout_secs"].as_u64().unwrap_or(120),
            error_mode,
            output_var: s["output_var"].as_str().map(String::from),
        });
    }

    let workflow = Workflow {
        id: WorkflowId::new(),
        name,
        description,
        steps,
        created_at: chrono::Utc::now(),
    };

    let id = state.kernel.register_workflow(workflow).await;
    (
        StatusCode::CREATED,
        Json(serde_json::json!({"workflow_id": id.to_string()})),
    )
}

/// GET /api/workflows — List all workflows.
pub async fn list_workflows(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let workflows = state.kernel.workflows.list_workflows().await;
    let list: Vec<serde_json::Value> = workflows
        .iter()
        .map(|w| {
            serde_json::json!({
                "id": w.id.to_string(),
                "name": w.name,
                "description": w.description,
                "steps": w.steps.len(),
                "created_at": w.created_at.to_rfc3339(),
            })
        })
        .collect();
    Json(list).into_response()
}

/// POST /api/workflows/:id/run — Execute a workflow.
pub async fn run_workflow(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let workflow_id = WorkflowId(match id.parse() {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid workflow ID"})),
            );
        }
    });

    let input = req["input"].as_str().unwrap_or("").to_string();

    match state.kernel.run_workflow(workflow_id, input).await {
        Ok((run_id, output)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "run_id": run_id.to_string(),
                "output": output,
                "status": "completed",
            })),
        ),
        Err(e) => {
            tracing::warn!(workflow_id = %id, error = %e, "Workflow run failed");
            let error_msg = format!("Workflow execution failed: {e}");
            let truncated = rusty_hand_types::text::truncate_bytes(&error_msg, 500);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": truncated})),
            )
        }
    }
}

/// GET /api/workflows/:id/runs — List runs for a workflow.
pub async fn list_workflow_runs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let workflow_id = match id.parse() {
        Ok(uuid) => WorkflowId(uuid),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid workflow ID"})),
            )
                .into_response();
        }
    };

    let runs = state.kernel.workflows.list_runs(None).await;
    let list: Vec<serde_json::Value> = runs
        .iter()
        .filter(|r| r.workflow_id == workflow_id)
        .map(|r| {
            serde_json::json!({
                "id": r.id.to_string(),
                "workflow_id": r.workflow_id.to_string(),
                "workflow_name": r.workflow_name,
                "state": serde_json::to_value(&r.state).unwrap_or_default(),
                "steps_completed": r.step_results.len(),
                "started_at": r.started_at.to_rfc3339(),
                "completed_at": r.completed_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();
    Json(list).into_response()
}

// ---------------------------------------------------------------------------
// Trigger routes
// ---------------------------------------------------------------------------

/// POST /api/triggers — Register a new event trigger.
pub async fn create_trigger(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id_str = match req["agent_id"].as_str() {
        Some(id) => id,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'agent_id'"})),
            );
        }
    };

    let agent_id = match parse_agent_id(agent_id_str) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let pattern: TriggerPattern = match req.get("pattern") {
        Some(p) => match serde_json::from_value(p.clone()) {
            Ok(pat) => pat,
            Err(e) => {
                tracing::warn!("Invalid trigger pattern: {e}");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid trigger pattern"})),
                );
            }
        },
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'pattern'"})),
            );
        }
    };

    let prompt_template = req["prompt_template"]
        .as_str()
        .unwrap_or("Event: {{event}}")
        .to_string();
    let max_fires = req["max_fires"].as_u64().unwrap_or(0);

    match state
        .kernel
        .register_trigger(agent_id, pattern, prompt_template, max_fires)
    {
        Ok(trigger_id) => (
            StatusCode::CREATED,
            Json(serde_json::json!({
                "trigger_id": trigger_id.to_string(),
                "agent_id": agent_id.to_string(),
            })),
        ),
        Err(e) => {
            tracing::warn!("Trigger registration failed: {e}");
            (
                StatusCode::NOT_FOUND,
                Json(
                    serde_json::json!({"error": "Trigger registration failed (agent not found?)"}),
                ),
            )
        }
    }
}

/// GET /api/triggers — List all triggers (optionally filter by ?agent_id=...).
pub async fn list_triggers(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let agent_filter = params
        .get("agent_id")
        .and_then(|id| id.parse::<AgentId>().ok());

    let triggers = state.kernel.list_triggers(agent_filter);
    let list: Vec<serde_json::Value> = triggers
        .iter()
        .map(|t| {
            serde_json::json!({
                "id": t.id.to_string(),
                "agent_id": t.agent_id.to_string(),
                "pattern": serde_json::to_value(&t.pattern).unwrap_or_default(),
                "prompt_template": t.prompt_template,
                "enabled": t.enabled,
                "fire_count": t.fire_count,
                "max_fires": t.max_fires,
                "created_at": t.created_at.to_rfc3339(),
            })
        })
        .collect();
    Json(list)
}

/// DELETE /api/triggers/:id — Remove a trigger.
pub async fn delete_trigger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let trigger_id = TriggerId(match id.parse() {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid trigger ID"})),
            );
        }
    });

    if state.kernel.remove_trigger(trigger_id) {
        (
            StatusCode::OK,
            Json(serde_json::json!({"status": "removed", "trigger_id": id})),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Trigger not found"})),
        )
    }
}

// ---------------------------------------------------------------------------
// Profile + Mode endpoints
// ---------------------------------------------------------------------------

/// GET /api/profiles — List all tool profiles and their tool lists.
pub async fn list_profiles() -> impl IntoResponse {
    use rusty_hand_types::agent::ToolProfile;

    let profiles = [
        ("minimal", ToolProfile::Minimal),
        ("coding", ToolProfile::Coding),
        ("research", ToolProfile::Research),
        ("messaging", ToolProfile::Messaging),
        ("automation", ToolProfile::Automation),
        ("full", ToolProfile::Full),
    ];

    let result: Vec<serde_json::Value> = profiles
        .iter()
        .map(|(name, profile)| {
            serde_json::json!({
                "name": name,
                "tools": profile.tools(),
            })
        })
        .collect();

    Json(result)
}

/// PUT /api/agents/:id/mode — Change an agent's operational mode.
pub async fn set_agent_mode(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<SetModeRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    match state.kernel.registry.set_mode(agent_id, body.mode) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "updated",
                "agent_id": id,
                "mode": body.mode,
            })),
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Version endpoint
// ---------------------------------------------------------------------------

/// GET /api/version — Build & version info.
pub async fn version() -> impl IntoResponse {
    Json(serde_json::json!({
        "name": "rustyhand",
        "version": env!("CARGO_PKG_VERSION"),
        "build_date": option_env!("BUILD_DATE").unwrap_or("dev"),
        "git_sha": option_env!("GIT_SHA").unwrap_or("unknown"),
        "rust_version": option_env!("RUSTC_VERSION").unwrap_or("unknown"),
        "platform": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
    }))
}

// ---------------------------------------------------------------------------
// Single agent detail + SSE streaming
// ---------------------------------------------------------------------------

/// GET /api/agents/:id — Get a single agent's detailed info.
pub async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => return agent_not_found(&id),
    };

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": entry.id.to_string(),
            "name": entry.name,
            "state": format!("{:?}", entry.state).to_lowercase(),
            "mode": entry.mode,
            "profile": entry.manifest.profile,
            "created_at": entry.created_at.to_rfc3339(),
            "session_id": entry.session_id.0.to_string(),
            "group": entry.manifest.group,
            "model": {
                "provider": entry.manifest.model.provider,
                "model": entry.manifest.model.model,
                "temperature": entry.manifest.model.temperature,
                "max_tokens": entry.manifest.model.max_tokens,
                "thinking_enabled": entry.manifest.model.thinking.is_some(),
                "response_format": match entry.manifest.model.response_format {
                    rusty_hand_types::agent::ResponseFormat::Json => "json",
                    _ => "text",
                },
            },
            "capabilities": {
                "tools": entry.manifest.capabilities.tools,
                "network": entry.manifest.capabilities.network,
            },
            "description": entry.manifest.description,
            "tags": entry.manifest.tags,
            "identity": {
                "emoji": entry.identity.emoji,
                "avatar_url": entry.identity.avatar_url,
                "color": entry.identity.color,
                "archetype": entry.identity.archetype,
                "vibe": entry.identity.vibe,
                "greeting_style": entry.identity.greeting_style,
            },
            "skills": entry.manifest.skills,
            "skills_mode": if entry.manifest.skills.is_empty() { "all" } else { "allowlist" },
            "mcp_servers": entry.manifest.mcp_servers,
            "mcp_servers_mode": if entry.manifest.mcp_servers.is_empty() { "all" } else { "allowlist" },
        })),
    )
}

/// POST /api/agents/:id/message/stream — SSE streaming response.
pub async fn send_message_stream(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<MessageRequest>,
) -> axum::response::Response {
    use axum::response::sse::{Event, Sse};
    use futures::stream;
    use rusty_hand_runtime::llm_driver::StreamEvent;

    // Reject empty/whitespace-only messages
    if req.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Message cannot be empty"})),
        )
            .into_response();
    }

    // SECURITY: Reject oversized messages to prevent OOM / LLM token abuse.
    const MAX_MESSAGE_SIZE: usize = 64 * 1024; // 64KB
    if req.message.len() > MAX_MESSAGE_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "Message too large (max 64KB)"})),
        )
            .into_response();
    }

    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
                .into_response();
        }
    };

    if state.kernel.registry.get(agent_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        )
            .into_response();
    }

    let kernel_handle: Arc<dyn KernelHandle> = state.kernel.clone() as Arc<dyn KernelHandle>;
    let (rx, _handle) =
        match state
            .kernel
            .send_message_streaming(agent_id, &req.message, Some(kernel_handle))
        {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!("Streaming message failed for agent {id}: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Streaming message failed"})),
                )
                    .into_response();
            }
        };

    // Max stream duration to prevent stuck connections (10 minutes)
    let stream_deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(600);

    let sse_stream = stream::unfold(rx, move |mut rx| async move {
        let remaining = stream_deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            tracing::debug!("SSE stream reached 10-minute timeout");
            return None;
        }
        match tokio::time::timeout(remaining, rx.recv()).await {
            Ok(Some(event)) => {
                let sse_event: Result<Event, std::convert::Infallible> = Ok(match event {
                    StreamEvent::TextDelta { text } => Event::default()
                        .event("chunk")
                        .json_data(serde_json::json!({"content": text, "done": false}))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::ToolUseStart { name, .. } => Event::default()
                        .event("tool_use")
                        .json_data(serde_json::json!({"tool": name}))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::ToolUseEnd { name, input, .. } => Event::default()
                        .event("tool_result")
                        .json_data(serde_json::json!({"tool": name, "input": input}))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::ContentComplete { usage, .. } => Event::default()
                        .event("done")
                        .json_data(serde_json::json!({
                            "done": true,
                            "usage": {
                                "input_tokens": usage.input_tokens,
                                "output_tokens": usage.output_tokens,
                            }
                        }))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::PhaseChange { phase, detail } => Event::default()
                        .event("phase")
                        .json_data(serde_json::json!({
                            "phase": phase,
                            "detail": detail,
                        }))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::ThinkingDelta { text } => Event::default()
                        .event("thinking_delta")
                        .json_data(serde_json::json!({"content": text}))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    StreamEvent::ToolInputDelta { text } => Event::default()
                        .event("tool_input_delta")
                        .json_data(serde_json::json!({"content": text}))
                        .unwrap_or_else(|_| Event::default().data("error")),
                    _ => Event::default().comment("skip"),
                });
                Some((sse_event, rx))
            }
            Ok(None) => None,
            Err(_) => {
                // Stream timeout — send a final event and close
                let timeout_event: Result<Event, std::convert::Infallible> = Ok(Event::default()
                    .event("error")
                    .json_data(serde_json::json!({"error": "Stream timeout (10 min limit)"}))
                    .unwrap_or_else(|_| Event::default().data("timeout")));
                Some((timeout_event, rx))
            }
        }
    });

    Sse::new(sse_stream).into_response()
}

// ---------------------------------------------------------------------------
// Channel status endpoints — data-driven registry for all 40 adapters
// ---------------------------------------------------------------------------

/// Field type for the channel configuration form.
#[derive(Clone, Copy)]
enum FieldType {
    Secret,
    Text,
    Number,
    List,
}

impl FieldType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Secret => "secret",
            Self::Text => "text",
            Self::Number => "number",
            Self::List => "list",
        }
    }
}

/// A single configurable field for a channel adapter.
#[derive(Clone)]
struct ChannelField {
    key: &'static str,
    label: &'static str,
    field_type: FieldType,
    env_var: Option<&'static str>,
    required: bool,
    placeholder: &'static str,
    /// If true, this field is hidden under "Show Advanced" in the UI.
    advanced: bool,
}

/// Metadata for one channel adapter.
struct ChannelMeta {
    name: &'static str,
    display_name: &'static str,
    icon: &'static str,
    description: &'static str,
    difficulty: &'static str,
    setup_time: &'static str,
    /// One-line quick setup hint shown in the simple form view.
    quick_setup: &'static str,
    fields: &'static [ChannelField],
    setup_steps: &'static [&'static str],
    config_template: &'static str,
}

const CHANNEL_REGISTRY: &[ChannelMeta] = &[
    ChannelMeta {
        name: "telegram", display_name: "Telegram", icon: "TG",
        description: "Telegram Bot API — long-polling adapter",
        difficulty: "Easy", setup_time: "~2 min",
        quick_setup: "Paste your bot token from @BotFather",
        fields: &[
            ChannelField { key: "bot_token_env", label: "Bot Token", field_type: FieldType::Secret, env_var: Some("TELEGRAM_BOT_TOKEN"), required: true, placeholder: "123456:ABC-DEF...", advanced: false },
            ChannelField { key: "allowed_users", label: "Allowed User IDs", field_type: FieldType::List, env_var: None, required: false, placeholder: "12345, 67890", advanced: true },
            ChannelField { key: "default_agent", label: "Default Agent", field_type: FieldType::Text, env_var: None, required: false, placeholder: "assistant", advanced: true },
            ChannelField { key: "poll_interval_secs", label: "Poll Interval (sec)", field_type: FieldType::Number, env_var: None, required: false, placeholder: "1", advanced: true },
        ],
        setup_steps: &["Open @BotFather on Telegram", "Send /newbot and follow the prompts", "Paste the token below"],
        config_template: "[channels.telegram]\nbot_token_env = \"TELEGRAM_BOT_TOKEN\"",
    },
    ChannelMeta {
        name: "discord", display_name: "Discord", icon: "DC",
        description: "Discord Gateway bot adapter",
        difficulty: "Easy", setup_time: "~3 min",
        quick_setup: "Paste your bot token from the Discord Developer Portal",
        fields: &[
            ChannelField { key: "bot_token_env", label: "Bot Token", field_type: FieldType::Secret, env_var: Some("DISCORD_BOT_TOKEN"), required: true, placeholder: "MTIz...", advanced: false },
            ChannelField { key: "allowed_guilds", label: "Allowed Guild IDs", field_type: FieldType::List, env_var: None, required: false, placeholder: "123456789, 987654321", advanced: true },
            ChannelField { key: "default_agent", label: "Default Agent", field_type: FieldType::Text, env_var: None, required: false, placeholder: "assistant", advanced: true },
            ChannelField { key: "intents", label: "Intents Bitmask", field_type: FieldType::Number, env_var: None, required: false, placeholder: "33280", advanced: true },
        ],
        setup_steps: &["Go to discord.com/developers/applications", "Create a bot and copy the token", "Paste it below"],
        config_template: "[channels.discord]\nbot_token_env = \"DISCORD_BOT_TOKEN\"",
    },
    ChannelMeta {
        name: "slack", display_name: "Slack", icon: "SL",
        description: "Slack Socket Mode + Events API",
        difficulty: "Medium", setup_time: "~5 min",
        quick_setup: "Paste your App Token and Bot Token from api.slack.com",
        fields: &[
            ChannelField { key: "app_token_env", label: "App Token (xapp-)", field_type: FieldType::Secret, env_var: Some("SLACK_APP_TOKEN"), required: true, placeholder: "xapp-1-...", advanced: false },
            ChannelField { key: "bot_token_env", label: "Bot Token (xoxb-)", field_type: FieldType::Secret, env_var: Some("SLACK_BOT_TOKEN"), required: true, placeholder: "xoxb-...", advanced: false },
            ChannelField { key: "allowed_channels", label: "Allowed Channel IDs", field_type: FieldType::List, env_var: None, required: false, placeholder: "C01234, C56789", advanced: true },
            ChannelField { key: "default_agent", label: "Default Agent", field_type: FieldType::Text, env_var: None, required: false, placeholder: "assistant", advanced: true },
        ],
        setup_steps: &["Create app at api.slack.com/apps", "Enable Socket Mode and copy App Token", "Copy Bot Token from OAuth & Permissions"],
        config_template: "[channels.slack]\napp_token_env = \"SLACK_APP_TOKEN\"\nbot_token_env = \"SLACK_BOT_TOKEN\"",
    },
];

/// Check if a channel is configured (has a `[channels.xxx]` section in config).
fn is_channel_configured(config: &rusty_hand_types::config::ChannelsConfig, name: &str) -> bool {
    match name {
        "telegram" => config.telegram.is_some(),
        "discord" => config.discord.is_some(),
        "slack" => config.slack.is_some(),
        _ => false,
    }
}

/// Build a JSON field descriptor, checking env var presence but never exposing secrets.
fn build_field_json(f: &ChannelField) -> serde_json::Value {
    let has_value = f
        .env_var
        .map(|ev| std::env::var(ev).map(|v| !v.is_empty()).unwrap_or(false))
        .unwrap_or(false);
    serde_json::json!({
        "key": f.key,
        "label": f.label,
        "type": f.field_type.as_str(),
        "env_var": f.env_var,
        "required": f.required,
        "has_value": has_value,
        "placeholder": f.placeholder,
        "advanced": f.advanced,
    })
}

/// Find a channel definition by name.
fn find_channel_meta(name: &str) -> Option<&'static ChannelMeta> {
    CHANNEL_REGISTRY.iter().find(|c| c.name == name)
}

/// GET /api/channels — List the supported channel adapters with status and field metadata.
pub async fn list_channels(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Read the live channels config (updated on every hot-reload) instead of the
    // stale boot-time kernel.config, so newly configured channels show correctly.
    let live_channels = state.channels_config.read().await;
    // Snapshot the bridge's started-adapter set. An adapter is in this
    // set iff its `start()` method returned Ok — for Telegram that
    // means `getMe` succeeded, for Discord/Slack that the gateway/socket
    // handshake completed. Lets us split `configured + has_token` into
    // `auth_status: "ok"` vs `auth_status: "auth_failed"`.
    let started: std::collections::HashSet<String> = {
        let guard = state.bridge_manager.lock().await;
        guard
            .as_ref()
            .map(|m| m.started_channels())
            .unwrap_or_default()
    };
    let mut channels = Vec::new();
    let mut configured_count = 0u32;

    for meta in CHANNEL_REGISTRY {
        let configured = is_channel_configured(&live_channels, meta.name);
        if configured {
            configured_count += 1;
        }

        // Check if all required secret env vars are set
        let has_token = meta
            .fields
            .iter()
            .filter(|f| f.required && f.env_var.is_some())
            .all(|f| {
                f.env_var
                    .map(|ev| std::env::var(ev).map(|v| !v.is_empty()).unwrap_or(false))
                    .unwrap_or(true)
            });

        // Auth state, derived from configured + has_token + started:
        // - "not_configured": no `[channels.<name>]` section in config
        // - "missing_token":  configured but the required env var is unset
        // - "ok":             configured + token + bridge accepted the token
        // - "auth_failed":    configured + token but bridge couldn't start
        //                     (Telegram getMe 401, Slack handshake failed, etc.)
        let auth_status = if !configured {
            "not_configured"
        } else if !has_token {
            "missing_token"
        } else if started.contains(meta.name) {
            "ok"
        } else {
            "auth_failed"
        };

        let fields: Vec<serde_json::Value> = meta.fields.iter().map(build_field_json).collect();

        channels.push(serde_json::json!({
            "name": meta.name,
            "display_name": meta.display_name,
            "icon": meta.icon,
            "description": meta.description,
            "difficulty": meta.difficulty,
            "setup_time": meta.setup_time,
            "quick_setup": meta.quick_setup,
            "configured": configured,
            "has_token": has_token,
            "auth_status": auth_status,
            "fields": fields,
            "setup_steps": meta.setup_steps,
            "config_template": meta.config_template,
        }));
    }

    Json(serde_json::json!({
        "channels": channels,
        "total": channels.len(),
        "configured_count": configured_count,
    }))
}

/// POST /api/channels/{name}/configure — Save channel secrets + config fields.
pub async fn configure_channel(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let meta = match find_channel_meta(&name) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Unknown channel"})),
            )
        }
    };

    let fields = match body.get("fields").and_then(|v| v.as_object()) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'fields' object"})),
            )
        }
    };

    let home = rusty_hand_kernel::config::rusty_hand_home();
    let secrets_path = home.join("secrets.env");
    let config_path = home.join("config.toml");
    let mut config_fields: HashMap<String, String> = HashMap::new();

    for field_def in meta.fields {
        let value = fields
            .get(field_def.key)
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if value.is_empty() {
            continue;
        }

        if let Some(env_var) = field_def.env_var {
            // Secret field — write to secrets.env and set in process
            if let Err(e) = write_secret_env(&secrets_path, env_var, value) {
                return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Channel secret save", &e);
            }
            std::env::set_var(env_var, value);
        } else {
            // Config field — collect for TOML write
            config_fields.insert(field_def.key.to_string(), value.to_string());
        }
    }

    // Write config.toml section
    if let Err(e) = upsert_channel_config(&config_path, &name, &config_fields) {
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Channel config save", &e);
    }

    // Hot-reload: activate the channel immediately
    match crate::channel_bridge::reload_channels_from_disk(&state).await {
        Ok(started) => {
            let activated = started.iter().any(|s| s.eq_ignore_ascii_case(&name));
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "configured",
                    "channel": name,
                    "activated": activated,
                    "started_channels": started,
                    "note": if activated {
                        format!("{} activated successfully.", name)
                    } else {
                        "Channel configured but could not start (check credentials).".to_string()
                    }
                })),
            )
        }
        Err(e) => {
            tracing::warn!(error = %e, "Channel hot-reload failed after configure");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "configured_but_not_activated",
                    "channel": name,
                    "activated": false,
                    "error": format!("Channel configured but hot-reload failed: {e}. Restart daemon to activate.")
                })),
            )
        }
    }
}

/// DELETE /api/channels/{name}/configure — Remove channel secrets + config section.
pub async fn remove_channel(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let meta = match find_channel_meta(&name) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Unknown channel"})),
            )
        }
    };

    let home = rusty_hand_kernel::config::rusty_hand_home();
    let secrets_path = home.join("secrets.env");
    let config_path = home.join("config.toml");

    // Remove all secret env vars for this channel
    for field_def in meta.fields {
        if let Some(env_var) = field_def.env_var {
            if let Err(e) = remove_secret_env(&secrets_path, env_var) {
                tracing::warn!(env_var, error = %e, "Failed to remove secret from secrets.env");
            }
            std::env::remove_var(env_var);
        }
    }

    // Remove config section
    if let Err(e) = remove_channel_config(&config_path, &name) {
        return safe_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Channel config removal",
            &e,
        );
    }

    // Hot-reload: deactivate the channel immediately
    match crate::channel_bridge::reload_channels_from_disk(&state).await {
        Ok(started) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "removed",
                "channel": name,
                "remaining_channels": started,
                "note": format!("{} deactivated.", name)
            })),
        ),
        Err(e) => {
            tracing::warn!(error = %e, "Channel hot-reload failed after remove");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "status": "removed_but_still_active",
                    "channel": name,
                    "error": format!("Channel removed from config but hot-reload failed: {e}. Restart daemon to fully deactivate.")
                })),
            )
        }
    }
}

/// POST /api/channels/{name}/test — Basic connectivity check for a channel.
pub async fn test_channel(Path(name): Path<String>) -> impl IntoResponse {
    let meta = match find_channel_meta(&name) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"status": "error", "message": "Unknown channel"})),
            )
        }
    };

    // Check all required env vars are set
    let mut missing = Vec::new();
    for field_def in meta.fields {
        if field_def.required {
            if let Some(env_var) = field_def.env_var {
                if std::env::var(env_var).map(|v| v.is_empty()).unwrap_or(true) {
                    missing.push(env_var);
                }
            }
        }
    }

    if !missing.is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "error",
                "message": format!("Missing required env vars: {}", missing.join(", "))
            })),
        );
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "message": format!("All required credentials for {} are set.", meta.display_name)
        })),
    )
}

/// POST /api/channels/reload — Manually trigger a channel hot-reload from disk config.
pub async fn reload_channels(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match crate::channel_bridge::reload_channels_from_disk(&state).await {
        Ok(started) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "ok",
                "started": started,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "status": "error",
                "error": e,
            })),
        ),
    }
}

// ---------------------------------------------------------------------------
// Template endpoints
// ---------------------------------------------------------------------------

/// GET /api/templates — List available agent templates.
pub async fn list_templates() -> impl IntoResponse {
    let mut templates = Vec::new();
    let mut seen = std::collections::HashSet::new();

    // Scan home_dir/agents/ first, then `$RUSTY_HAND_AGENTS_DIR` (the
    // image-bundled location). User-customized templates win on name
    // collision because the home dir is searched first.
    for agents_dir in rusty_hand_kernel::config::agents_search_dirs() {
        let entries = match std::fs::read_dir(&agents_dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("agent.toml");
            if !manifest_path.exists() {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            if !seen.insert(name.clone()) {
                continue;
            }

            let description = std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|content| toml::from_str::<AgentManifest>(&content).ok())
                .map(|m| m.description)
                .unwrap_or_default();

            templates.push(serde_json::json!({
                "name": name,
                "description": description,
            }));
        }
    }

    Json(serde_json::json!({
        "templates": templates,
        "total": templates.len(),
    }))
}

/// GET /api/templates/:name — Get template details.
pub async fn get_template(Path(name): Path<String>) -> impl IntoResponse {
    let manifest_path = match rusty_hand_kernel::config::resolve_agent_manifest(&name) {
        Some(p) => p,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Template not found"})),
            );
        }
    };

    match std::fs::read_to_string(&manifest_path) {
        Ok(content) => match toml::from_str::<AgentManifest>(&content) {
            Ok(manifest) => (
                StatusCode::OK,
                Json(serde_json::json!({
                    "name": name,
                    "manifest": {
                        "name": manifest.name,
                        "description": manifest.description,
                        "module": manifest.module,
                        "tags": manifest.tags,
                        "model": {
                            "provider": manifest.model.provider,
                            "model": manifest.model.model,
                        },
                        "capabilities": {
                            "tools": manifest.capabilities.tools,
                            "network": manifest.capabilities.network,
                        },
                    },
                    "manifest_toml": content,
                })),
            ),
            Err(e) => {
                tracing::warn!("Invalid template manifest for '{name}': {e}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Invalid template manifest"})),
                )
            }
        },
        Err(e) => {
            tracing::warn!("Failed to read template '{name}': {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Failed to read template"})),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// Memory endpoints
// ---------------------------------------------------------------------------

/// GET /api/memory/export — Export a full memory backup.
pub async fn export_memory_backup(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BackupQuery>,
) -> impl IntoResponse {
    let format = match parse_backup_format(query.format.as_deref()) {
        Ok(format) => format,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error })),
            )
                .into_response();
        }
    };

    match state.kernel.memory.export(format).await {
        Ok(data) => {
            let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
            let filename = format!("rustyhand-backup-{timestamp}.{}", backup_extension(format));
            (
                StatusCode::OK,
                [
                    (
                        header::CONTENT_TYPE,
                        backup_content_type(format).to_string(),
                    ),
                    (
                        header::CONTENT_DISPOSITION,
                        format!("attachment; filename=\"{filename}\""),
                    ),
                ],
                data,
            )
                .into_response()
        }
        Err(error) => {
            tracing::warn!("Memory export failed: {error}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Backup export failed" })),
            )
                .into_response()
        }
    }
}

/// POST /api/memory/import — Restore a full memory backup.
pub async fn import_memory_backup(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BackupQuery>,
    body: Bytes,
) -> impl IntoResponse {
    let format = match parse_backup_format(query.format.as_deref()) {
        Ok(format) => format,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error })),
            );
        }
    };

    match state.kernel.memory.import(body.as_ref(), format).await {
        Ok(report) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "imported",
                "entities_imported": report.entities_imported,
                "relations_imported": report.relations_imported,
                "memories_imported": report.memories_imported,
                "errors": report.errors,
            })),
        ),
        Err(error) => {
            tracing::warn!("Memory import failed: {error}");
            safe_error(StatusCode::BAD_REQUEST, "Backup import", &error)
        }
    }
}

/// GET /api/memory/agents/:id/kv — List KV pairs for an agent.
pub async fn get_agent_kv(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    match state.kernel.memory.list_kv(agent_id) {
        Ok(pairs) => {
            let kv: Vec<serde_json::Value> = pairs
                .into_iter()
                .map(|(k, v)| serde_json::json!({"key": k, "value": v}))
                .collect();
            (StatusCode::OK, Json(serde_json::json!({"kv_pairs": kv})))
        }
        Err(e) => {
            tracing::warn!("Memory list_kv failed for agent {id}: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Memory operation failed"})),
            )
        }
    }
}

/// GET /api/memory/agents/:id/kv/:key — Get a specific KV value.
pub async fn get_agent_kv_key(
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(String, String)>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    match state.kernel.memory.structured_get(agent_id, &key) {
        Ok(Some(val)) => (
            StatusCode::OK,
            Json(serde_json::json!({"key": key, "value": val})),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Key not found"})),
        ),
        Err(e) => {
            tracing::warn!("Memory get failed for agent {id}, key '{key}': {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Memory operation failed"})),
            )
        }
    }
}

/// PUT /api/memory/agents/:id/kv/:key — Set a KV value.
pub async fn set_agent_kv_key(
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    let value = body.get("value").cloned().unwrap_or(body);

    match state.kernel.memory.structured_set(agent_id, &key, value) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "stored", "key": key})),
        ),
        Err(e) => {
            tracing::warn!("Memory set failed for agent {id}, key '{key}': {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Memory operation failed"})),
            )
        }
    }
}

/// DELETE /api/memory/agents/:id/kv/:key — Delete a KV value.
pub async fn delete_agent_kv_key(
    State(state): State<Arc<AppState>>,
    Path((id, key)): Path<(String, String)>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    match state.kernel.memory.structured_delete(agent_id, &key) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "deleted", "key": key})),
        ),
        Err(e) => {
            tracing::warn!("Memory delete failed for agent {id}, key '{key}': {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Memory operation failed"})),
            )
        }
    }
}

fn parse_backup_format(raw: Option<&str>) -> Result<ExportFormat, &'static str> {
    match raw.unwrap_or("json").to_ascii_lowercase().as_str() {
        "json" => Ok(ExportFormat::Json),
        "messagepack" | "msgpack" | "mpk" => Ok(ExportFormat::MessagePack),
        _ => Err("Unsupported backup format. Use 'json' or 'messagepack'."),
    }
}

fn backup_content_type(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Json => "application/json",
        ExportFormat::MessagePack => "application/x-msgpack",
    }
}

fn backup_extension(format: ExportFormat) -> &'static str {
    match format {
        ExportFormat::Json => "json",
        ExportFormat::MessagePack => "msgpack",
    }
}

/// GET /api/health — Minimal liveness probe (public, no auth required).
/// Returns only status and version to prevent information leakage.
/// Use GET /api/health/detail for full diagnostics (requires auth).
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Check database connectivity
    let shared_id = rusty_hand_types::agent::AgentId(uuid::Uuid::from_bytes([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ]));
    let db_ok = state
        .kernel
        .memory
        .structured_get(shared_id, "__health_check__")
        .is_ok();

    let status = if db_ok { "ok" } else { "degraded" };

    Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

/// GET /api/health/detail — Full health diagnostics (requires auth).
pub async fn health_detail(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let health = state.kernel.supervisor.health();

    let shared_id = rusty_hand_types::agent::AgentId(uuid::Uuid::from_bytes([
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    ]));
    let db_ok = state
        .kernel
        .memory
        .structured_get(shared_id, "__health_check__")
        .is_ok();

    let config_warnings = state.kernel.config.validate();
    let status = if db_ok { "ok" } else { "degraded" };

    Json(serde_json::json!({
        "status": status,
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.started_at.elapsed().as_secs(),
        "panic_count": health.panic_count,
        "restart_count": health.restart_count,
        "agent_count": state.kernel.registry.count(),
        "database": if db_ok { "connected" } else { "error" },
        "config_warnings": config_warnings,
    }))
}

/// GET /api/onboarding — Public onboarding status (no secrets exposed).
///
/// Returns whether an API key is configured and the agent count,
/// so the dashboard can decide whether to show the setup wizard.
pub async fn onboarding_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let api_key_set =
        !state.kernel.config.api_key.is_empty() && state.kernel.config.api_key != "not set";
    let agent_count = state.kernel.registry.count();

    Json(serde_json::json!({
        "api_key_set": api_key_set,
        "agent_count": agent_count,
    }))
}

// ---------------------------------------------------------------------------
// Prometheus metrics endpoint
// ---------------------------------------------------------------------------

/// GET /api/metrics — Prometheus text-format metrics.
///
/// Returns counters and gauges for monitoring RustyHand in production:
/// - `rusty_hand_agents_active` — number of active agents
/// - `rusty_hand_uptime_seconds` — seconds since daemon started
/// - `rusty_hand_tokens_total` — total tokens consumed (per agent)
/// - `rusty_hand_tool_calls_total` — total tool calls (per agent)
/// - `rusty_hand_panics_total` — supervisor panic count
/// - `rusty_hand_restarts_total` — supervisor restart count
pub async fn prometheus_metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut out = String::with_capacity(2048);

    // Uptime
    let uptime = state.started_at.elapsed().as_secs();
    out.push_str("# HELP rusty_hand_uptime_seconds Time since daemon started.\n");
    out.push_str("# TYPE rusty_hand_uptime_seconds gauge\n");
    out.push_str(&format!("rusty_hand_uptime_seconds {uptime}\n\n"));

    // Active agents
    let agents = state.kernel.registry.list();
    let active = agents
        .iter()
        .filter(|a| matches!(a.state, rusty_hand_types::agent::AgentState::Running))
        .count();
    out.push_str("# HELP rusty_hand_agents_active Number of active agents.\n");
    out.push_str("# TYPE rusty_hand_agents_active gauge\n");
    out.push_str(&format!("rusty_hand_agents_active {active}\n"));
    out.push_str("# HELP rusty_hand_agents_total Total number of registered agents.\n");
    out.push_str("# TYPE rusty_hand_agents_total gauge\n");
    out.push_str(&format!("rusty_hand_agents_total {}\n\n", agents.len()));

    // Per-agent token and tool usage
    out.push_str("# HELP rusty_hand_tokens_total Total tokens consumed (rolling hourly window).\n");
    out.push_str("# TYPE rusty_hand_tokens_total gauge\n");
    out.push_str("# HELP rusty_hand_tool_calls_total Total tool calls (rolling hourly window).\n");
    out.push_str("# TYPE rusty_hand_tool_calls_total gauge\n");
    for agent in &agents {
        let name = &agent.name;
        let provider = &agent.manifest.model.provider;
        let model = &agent.manifest.model.model;
        if let Some((tokens, tools)) = state.kernel.scheduler.get_usage(agent.id) {
            out.push_str(&format!(
                "rusty_hand_tokens_total{{agent=\"{name}\",provider=\"{provider}\",model=\"{model}\"}} {tokens}\n"
            ));
            out.push_str(&format!(
                "rusty_hand_tool_calls_total{{agent=\"{name}\"}} {tools}\n"
            ));
        }
    }
    out.push('\n');

    // Supervisor health
    let health = state.kernel.supervisor.health();
    out.push_str("# HELP rusty_hand_panics_total Total supervisor panics since start.\n");
    out.push_str("# TYPE rusty_hand_panics_total counter\n");
    out.push_str(&format!("rusty_hand_panics_total {}\n", health.panic_count));
    out.push_str("# HELP rusty_hand_restarts_total Total supervisor restarts since start.\n");
    out.push_str("# TYPE rusty_hand_restarts_total counter\n");
    out.push_str(&format!(
        "rusty_hand_restarts_total {}\n\n",
        health.restart_count
    ));

    // Cost tracking
    if let Ok(today) = state.kernel.memory.usage().query_today_cost() {
        out.push_str("# HELP rusty_hand_cost_today_usd Total LLM cost today in USD.\n");
        out.push_str("# TYPE rusty_hand_cost_today_usd gauge\n");
        out.push_str(&format!("rusty_hand_cost_today_usd {today:.6}\n"));
    }
    if let Ok(hourly) = state.kernel.memory.usage().query_global_hourly() {
        out.push_str("# HELP rusty_hand_cost_hourly_usd LLM cost in the last hour in USD.\n");
        out.push_str("# TYPE rusty_hand_cost_hourly_usd gauge\n");
        out.push_str(&format!("rusty_hand_cost_hourly_usd {hourly:.6}\n"));
    }
    if let Ok(monthly) = state.kernel.memory.usage().query_global_monthly() {
        out.push_str("# HELP rusty_hand_cost_monthly_usd LLM cost this month in USD.\n");
        out.push_str("# TYPE rusty_hand_cost_monthly_usd gauge\n");
        out.push_str(&format!("rusty_hand_cost_monthly_usd {monthly:.6}\n\n"));
    }

    // LLM cache stats
    out.push_str(
        "# HELP rusty_hand_llm_cache_entries Number of entries in the LLM response cache.\n",
    );
    out.push_str("# TYPE rusty_hand_llm_cache_entries gauge\n");
    out.push_str("rusty_hand_llm_cache_entries 0\n\n");

    // MCP connections
    let mcp_count = state.kernel.mcp_tools.lock().map(|t| t.len()).unwrap_or(0);
    out.push_str("# HELP rusty_hand_mcp_tools_total Number of MCP tools available.\n");
    out.push_str("# TYPE rusty_hand_mcp_tools_total gauge\n");
    out.push_str(&format!("rusty_hand_mcp_tools_total {mcp_count}\n\n"));

    // Version info
    out.push_str("# HELP rusty_hand_info RustyHand version and build info.\n");
    out.push_str("# TYPE rusty_hand_info gauge\n");
    out.push_str(&format!(
        "rusty_hand_info{{version=\"{}\"}} 1\n",
        env!("CARGO_PKG_VERSION")
    ));

    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        out,
    )
}

// ---------------------------------------------------------------------------
// Skills endpoints
// ---------------------------------------------------------------------------

/// GET /api/skills — List installed skills.
pub async fn list_skills(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let skills_dir = state.kernel.config.home_dir.join("skills");
    let mut registry = rusty_hand_skills::registry::SkillRegistry::new(skills_dir);
    // v0.7.1 regression fix: the dashboard listed 0 skills because we only
    // called load_all() which reads ~/.rustyhand/skills/ from disk — the
    // 60 bundled (compile-time embedded) skills were missing entirely.
    // Match the kernel's own boot sequence: bundled first, then on-disk.
    registry.load_bundled();
    if let Err(e) = registry.load_all() {
        tracing::warn!(error = %e, "Failed to load skills registry");
    }

    let skills: Vec<serde_json::Value> = registry
        .list()
        .iter()
        .map(|s| {
            let source = match &s.manifest.source {
                Some(rusty_hand_skills::SkillSource::ClawHub { slug, version }) => {
                    serde_json::json!({"type": "clawhub", "slug": slug, "version": version})
                }
                Some(rusty_hand_skills::SkillSource::OpenClaw) => {
                    serde_json::json!({"type": "openclaw"})
                }
                Some(rusty_hand_skills::SkillSource::Bundled) => {
                    serde_json::json!({"type": "bundled"})
                }
                Some(rusty_hand_skills::SkillSource::Native) | None => {
                    serde_json::json!({"type": "local"})
                }
            };
            serde_json::json!({
                "name": s.manifest.skill.name,
                "description": s.manifest.skill.description,
                "version": s.manifest.skill.version,
                "author": s.manifest.skill.author,
                "runtime": format!("{:?}", s.manifest.runtime.runtime_type),
                "tools_count": s.manifest.tools.provided.len(),
                "tags": s.manifest.skill.tags,
                "enabled": s.enabled,
                "source": source,
                "has_prompt_context": s.manifest.prompt_context.is_some(),
            })
        })
        .collect();

    Json(serde_json::json!({ "skills": skills, "total": skills.len() }))
}

/// POST /api/skills/install — Install a skill from FangHub (GitHub).
pub async fn install_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SkillInstallRequest>,
) -> impl IntoResponse {
    let skills_dir = state.kernel.config.home_dir.join("skills");
    let config = rusty_hand_skills::marketplace::MarketplaceConfig::default();
    let client = rusty_hand_skills::marketplace::MarketplaceClient::new(config);

    match client.install(&req.name, &skills_dir).await {
        Ok(version) => {
            // Hot-reload so agents see the new skill immediately
            state.kernel.reload_skills();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "installed",
                    "name": req.name,
                    "version": version,
                })),
            )
        }
        Err(e) => {
            tracing::warn!("Skill install failed: {e}");
            safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Skill install", &e)
        }
    }
}

/// POST /api/skills/install-custom — Install a skill from inline code.
///
/// Thin wrapper around the `skill_install` builtin tool. Writes the skill to
/// `~/.rustyhand/skills/<name>/` with proper stdin/stdout boilerplate, then
/// triggers a hot-reload so it's available immediately.
///
/// Body: `{ "name", "language" ("python"|"javascript"), "description", "content", "overwrite"? }`
///
/// Auth: same api_key middleware as the rest of /api/*. No separate allowlist
/// check needed here (unlike the /mcp endpoint) — if you have the api_key you
/// already have full control of the kernel.
///
/// Distinct from `install_skill` which fetches a published skill from the
/// FangHub marketplace by name.
pub async fn install_custom_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Reuse the tool function directly so behaviour is identical to what
    // capability-builder produces via MCP: same validation, same wrapper,
    // same marker file. Single source of truth.
    let skills_dir = state.kernel.config.home_dir.join("skills");
    let mut registry = rusty_hand_skills::registry::SkillRegistry::new(skills_dir);
    // We pass a freshly-loaded registry so skills_dir() and is_frozen() work.
    if let Err(e) = registry.load_all() {
        tracing::warn!(error = %e, "Failed to load skills registry for install");
    }

    // Build a synthetic tool invocation and dispatch through the public
    // execute_tool entrypoint. allowed_tools=["skill_install"] so the
    // capability gate is satisfied without granting anything else.
    let kernel_handle: Arc<dyn rusty_hand_runtime::kernel_handle::KernelHandle> =
        state.kernel.clone() as Arc<dyn rusty_hand_runtime::kernel_handle::KernelHandle>;
    let allowed = vec!["skill_install".to_string()];
    let result = rusty_hand_runtime::tool_runner::execute_tool(
        "dashboard-install",
        "skill_install",
        &req,
        Some(&kernel_handle),
        Some(&allowed),
        None,
        Some(&registry),
        Some(&state.kernel.mcp_connections),
        Some(&state.kernel.web_ctx),
        Some(&state.kernel.browser_ctx),
        None,
        None,
        Some(&state.kernel.media_engine),
        None,
        None,
        None,
        Some(&*state.kernel.process_manager),
    )
    .await;

    if result.is_error {
        return safe_error(
            StatusCode::BAD_REQUEST,
            "Skill install",
            &result.content.as_str(),
        );
    }

    // Refresh the live skill registry so new skill is immediately callable.
    state.kernel.reload_skills();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "installed",
            "message": result.content,
        })),
    )
}

/// POST /api/skills/uninstall — Uninstall a skill.
pub async fn uninstall_skill(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SkillUninstallRequest>,
) -> impl IntoResponse {
    let skills_dir = state.kernel.config.home_dir.join("skills");
    let mut registry = rusty_hand_skills::registry::SkillRegistry::new(skills_dir);
    // Load bundled too so `remove()` can surface a clear "cannot remove
    // bundled skill" error when the user tries to uninstall a compile-time
    // embedded skill they see in the dashboard list.
    registry.load_bundled();
    if let Err(e) = registry.load_all() {
        tracing::warn!(error = %e, "Failed to load skills registry");
    }

    match registry.remove(&req.name) {
        Ok(()) => {
            // Hot-reload so agents stop seeing the removed skill
            state.kernel.reload_skills();
            (
                StatusCode::OK,
                Json(serde_json::json!({"status": "uninstalled", "name": req.name})),
            )
        }
        Err(e) => safe_error(StatusCode::NOT_FOUND, "Skill uninstall", &e),
    }
}

/// GET /api/marketplace/search — Search the FangHub marketplace.
pub async fn marketplace_search(
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").cloned().unwrap_or_default();
    if query.is_empty() {
        return Json(serde_json::json!({"results": [], "total": 0}));
    }

    let config = rusty_hand_skills::marketplace::MarketplaceConfig::default();
    let client = rusty_hand_skills::marketplace::MarketplaceClient::new(config);

    match client.search(&query).await {
        Ok(results) => {
            let items: Vec<serde_json::Value> = results
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "name": r.name,
                        "description": r.description,
                        "stars": r.stars,
                        "url": r.url,
                    })
                })
                .collect();
            Json(serde_json::json!({"results": items, "total": items.len()}))
        }
        Err(e) => {
            tracing::warn!("Marketplace search failed: {e}");
            Json(
                serde_json::json!({"results": [], "total": 0, "error": "Marketplace search failed"}),
            )
        }
    }
}

// ---------------------------------------------------------------------------
// ClawHub (OpenClaw ecosystem) endpoints
// ---------------------------------------------------------------------------

/// GET /api/clawhub/search — Search ClawHub skills using vector/semantic search.
///
/// Query parameters:
/// - `q` — search query (required)
/// - `limit` — max results (default: 20, max: 50)
pub async fn clawhub_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let query = params.get("q").cloned().unwrap_or_default();
    if query.is_empty() {
        return (
            StatusCode::OK,
            Json(serde_json::json!({"items": [], "next_cursor": null})),
        );
    }

    let limit: u32 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let cache_dir = state.kernel.config.home_dir.join(".cache").join("clawhub");
    let client = rusty_hand_skills::clawhub::ClawHubClient::new(cache_dir);

    match client.search(&query, limit).await {
        Ok(results) => {
            let items: Vec<serde_json::Value> = results
                .results
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "slug": e.slug,
                        "name": e.display_name,
                        "description": e.summary,
                        "version": e.version,
                        "score": e.score,
                        "updated_at": e.updated_at,
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "items": items,
                    "next_cursor": null,
                })),
            )
        }
        Err(e) => {
            tracing::warn!("ClawHub search failed: {e}");
            (
                StatusCode::OK,
                Json(
                    serde_json::json!({"items": [], "next_cursor": null, "error": "ClawHub search failed"}),
                ),
            )
        }
    }
}

/// GET /api/clawhub/browse — Browse ClawHub skills by sort order.
///
/// Query parameters:
/// - `sort` — sort order: "trending", "downloads", "stars", "updated", "rating" (default: "trending")
/// - `limit` — max results (default: 20, max: 50)
/// - `cursor` — pagination cursor from previous response
pub async fn clawhub_browse(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let sort = match params.get("sort").map(|s| s.as_str()) {
        Some("downloads") => rusty_hand_skills::clawhub::ClawHubSort::Downloads,
        Some("stars") => rusty_hand_skills::clawhub::ClawHubSort::Stars,
        Some("updated") => rusty_hand_skills::clawhub::ClawHubSort::Updated,
        Some("rating") => rusty_hand_skills::clawhub::ClawHubSort::Rating,
        _ => rusty_hand_skills::clawhub::ClawHubSort::Trending,
    };

    let limit: u32 = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20);

    let cursor = params.get("cursor").map(|s| s.as_str());

    let cache_dir = state.kernel.config.home_dir.join(".cache").join("clawhub");
    let client = rusty_hand_skills::clawhub::ClawHubClient::new(cache_dir);

    match client.browse(sort, limit, cursor).await {
        Ok(results) => {
            let items: Vec<serde_json::Value> = results
                .items
                .iter()
                .map(clawhub_browse_entry_to_json)
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "items": items,
                    "next_cursor": results.next_cursor,
                })),
            )
        }
        Err(e) => {
            tracing::warn!("ClawHub browse failed: {e}");
            (
                StatusCode::OK,
                Json(
                    serde_json::json!({"items": [], "next_cursor": null, "error": "ClawHub browse failed"}),
                ),
            )
        }
    }
}

/// GET /api/clawhub/skill/{slug} — Get detailed info about a ClawHub skill.
pub async fn clawhub_skill_detail(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> impl IntoResponse {
    let cache_dir = state.kernel.config.home_dir.join(".cache").join("clawhub");
    let client = rusty_hand_skills::clawhub::ClawHubClient::new(cache_dir);

    let skills_dir = state.kernel.config.home_dir.join("skills");
    let is_installed = client.is_installed(&slug, &skills_dir);

    match client.get_skill(&slug).await {
        Ok(detail) => {
            let version = detail
                .latest_version
                .as_ref()
                .map(|v| v.version.as_str())
                .unwrap_or("");
            let author = detail
                .owner
                .as_ref()
                .map(|o| o.handle.as_str())
                .unwrap_or("");
            let author_name = detail
                .owner
                .as_ref()
                .map(|o| o.display_name.as_str())
                .unwrap_or("");
            let author_image = detail
                .owner
                .as_ref()
                .map(|o| o.image.as_str())
                .unwrap_or("");

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "slug": detail.skill.slug,
                    "name": detail.skill.display_name,
                    "description": detail.skill.summary,
                    "version": version,
                    "downloads": detail.skill.stats.downloads,
                    "stars": detail.skill.stats.stars,
                    "author": author,
                    "author_name": author_name,
                    "author_image": author_image,
                    "tags": detail.skill.tags,
                    "updated_at": detail.skill.updated_at,
                    "created_at": detail.skill.created_at,
                    "installed": is_installed,
                })),
            )
        }
        Err(e) => safe_error(StatusCode::NOT_FOUND, "Skill lookup", &e),
    }
}

/// POST /api/clawhub/install — Install a skill from ClawHub.
///
/// Runs the full security pipeline: SHA256 verification, format detection,
/// manifest security scan, prompt injection scan, and binary dependency check.
pub async fn clawhub_install(
    State(state): State<Arc<AppState>>,
    Json(req): Json<crate::types::ClawHubInstallRequest>,
) -> impl IntoResponse {
    let skills_dir = state.kernel.config.home_dir.join("skills");
    let cache_dir = state.kernel.config.home_dir.join(".cache").join("clawhub");
    let client = rusty_hand_skills::clawhub::ClawHubClient::new(cache_dir);

    // Check if already installed
    if client.is_installed(&req.slug, &skills_dir) {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({
                "error": format!("Skill '{}' is already installed", req.slug),
                "status": "already_installed",
            })),
        );
    }

    match client.install(&req.slug, &skills_dir).await {
        Ok(result) => {
            let warnings: Vec<serde_json::Value> = result
                .warnings
                .iter()
                .map(|w| {
                    serde_json::json!({
                        "severity": format!("{:?}", w.severity),
                        "message": w.message,
                    })
                })
                .collect();

            let translations: Vec<serde_json::Value> = result
                .tool_translations
                .iter()
                .map(|(from, to)| serde_json::json!({"from": from, "to": to}))
                .collect();

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": "installed",
                    "name": result.skill_name,
                    "version": result.version,
                    "slug": result.slug,
                    "is_prompt_only": result.is_prompt_only,
                    "warnings": warnings,
                    "tool_translations": translations,
                })),
            )
        }
        Err(e) => {
            let status = if e.to_string().contains("SecurityBlocked") {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            tracing::warn!("ClawHub install failed: {e}");
            safe_error(status, "ClawHub install", &e)
        }
    }
}

/// Convert a browse entry (nested stats/tags) to a flat JSON object for the frontend.
fn clawhub_browse_entry_to_json(
    entry: &rusty_hand_skills::clawhub::ClawHubBrowseEntry,
) -> serde_json::Value {
    let version = rusty_hand_skills::clawhub::ClawHubClient::entry_version(entry);
    serde_json::json!({
        "slug": entry.slug,
        "name": entry.display_name,
        "description": entry.summary,
        "version": version,
        "downloads": entry.stats.downloads,
        "stars": entry.stats.stars,
        "updated_at": entry.updated_at,
    })
}

// ---------------------------------------------------------------------------
// MCP server endpoints
// ---------------------------------------------------------------------------

/// GET /api/mcp/servers — List configured MCP servers and their tools.
pub async fn list_mcp_servers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Get configured servers from config
    let config_servers: Vec<serde_json::Value> = state
        .kernel
        .config
        .mcp_servers
        .iter()
        .map(|s| {
            let transport = match &s.transport {
                rusty_hand_types::config::McpTransportEntry::Stdio { command, args } => {
                    serde_json::json!({
                        "type": "stdio",
                        "command": command,
                        "args": args,
                    })
                }
                rusty_hand_types::config::McpTransportEntry::Sse { url } => {
                    serde_json::json!({
                        "type": "sse",
                        "url": url,
                    })
                }
            };
            serde_json::json!({
                "name": s.name,
                "transport": transport,
                "timeout_secs": s.timeout_secs,
                "env": s.env,
            })
        })
        .collect();

    // Get connected servers and their tools from the live MCP connections
    let connections = state.kernel.mcp_connections.lock().await;
    let connected: Vec<serde_json::Value> = connections
        .iter()
        .map(|conn| {
            let tools: Vec<serde_json::Value> = conn
                .tools()
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                    })
                })
                .collect();
            serde_json::json!({
                "name": conn.name(),
                "tools_count": tools.len(),
                "tools": tools,
                "connected": true,
            })
        })
        .collect();

    Json(serde_json::json!({
        "configured": config_servers,
        "connected": connected,
        "total_configured": config_servers.len(),
        "total_connected": connected.len(),
    }))
}

// ---------------------------------------------------------------------------
// Audit endpoints
// ---------------------------------------------------------------------------

/// GET /api/audit/recent — Get recent audit log entries.
pub async fn audit_recent(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let n: usize = params
        .get("n")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .min(1000); // Cap at 1000

    let entries = state.kernel.audit_log.recent(n);
    let tip = state.kernel.audit_log.tip_hash();

    let items: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            let agent_name = uuid::Uuid::parse_str(&e.agent_id)
                .ok()
                .and_then(|uuid| state.kernel.registry.get(AgentId(uuid)))
                .map(|entry| entry.name);
            serde_json::json!({
                "seq": e.seq,
                "timestamp": e.timestamp,
                "agent_id": e.agent_id,
                "agent_name": agent_name,
                "action": format!("{:?}", e.action),
                "detail": e.detail,
                "outcome": e.outcome,
                "hash": e.hash,
            })
        })
        .collect();

    Json(serde_json::json!({
        "entries": items,
        "total": state.kernel.audit_log.len(),
        "tip_hash": tip,
    }))
}

/// GET /api/audit/verify — Verify the audit chain integrity.
pub async fn audit_verify(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let entry_count = state.kernel.audit_log.len();
    match state.kernel.audit_log.verify_integrity() {
        Ok(()) => {
            if entry_count == 0 {
                // SECURITY: Warn that an empty audit log has no forensic value
                Json(serde_json::json!({
                    "valid": true,
                    "entries": 0,
                    "warning": "Audit log is empty — no events have been recorded yet",
                    "tip_hash": state.kernel.audit_log.tip_hash(),
                }))
            } else {
                Json(serde_json::json!({
                    "valid": true,
                    "entries": entry_count,
                    "tip_hash": state.kernel.audit_log.tip_hash(),
                }))
            }
        }
        Err(msg) => Json(serde_json::json!({
            "valid": false,
            "error": msg,
            "entries": entry_count,
        })),
    }
}

/// GET /api/logs/stream — SSE endpoint for real-time audit log streaming.
///
/// Streams new audit entries as Server-Sent Events. Accepts optional query
/// parameters for filtering:
///   - `level`  — filter by classified level (info, warn, error)
///   - `filter` — text substring filter across action/detail/agent_id
///   - `token`  — auth token (for EventSource clients that cannot set headers)
///
/// A heartbeat ping is sent every 15 seconds to keep the connection alive.
/// The endpoint polls the audit log every second and sends only new entries
/// (tracked by sequence number). On first connect, existing entries are sent
/// as a backfill so the client has immediate context.
pub async fn logs_stream(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> axum::response::Response {
    use axum::response::sse::{Event, KeepAlive, Sse};

    let level_filter = params.get("level").cloned().unwrap_or_default();
    let text_filter = params
        .get("filter")
        .cloned()
        .unwrap_or_default()
        .to_lowercase();
    let agent_id_filter = params.get("agent_id").cloned().unwrap_or_default();

    let (tx, rx) = tokio::sync::mpsc::channel::<
        Result<axum::response::sse::Event, std::convert::Infallible>,
    >(256);

    tokio::spawn(async move {
        let mut last_seq: u64 = 0;
        let mut first_poll = true;

        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let entries = state.kernel.audit_log.recent(200);

            for entry in &entries {
                // On first poll, send all existing entries as backfill.
                // After that, only send entries newer than last_seq.
                if !first_poll && entry.seq <= last_seq {
                    continue;
                }

                let action_str = format!("{:?}", entry.action);

                // Apply agent_id filter (exact prefix match covers both UUID and name)
                if !agent_id_filter.is_empty() && !entry.agent_id.starts_with(&agent_id_filter) {
                    continue;
                }

                // Apply level filter
                if !level_filter.is_empty() {
                    let classified = classify_audit_level(&action_str);
                    if classified != level_filter {
                        continue;
                    }
                }

                // Apply text filter
                if !text_filter.is_empty() {
                    let haystack = format!("{} {} {}", action_str, entry.detail, entry.agent_id)
                        .to_lowercase();
                    if !haystack.contains(&text_filter) {
                        continue;
                    }
                }

                let json = serde_json::json!({
                    "seq": entry.seq,
                    "timestamp": entry.timestamp,
                    "agent_id": entry.agent_id,
                    "action": action_str,
                    "detail": entry.detail,
                    "outcome": entry.outcome,
                    "hash": entry.hash,
                });
                let data = serde_json::to_string(&json).unwrap_or_default();
                if tx.send(Ok(Event::default().data(data))).await.is_err() {
                    return; // Client disconnected
                }
            }

            // Update tracking state
            if let Some(last) = entries.last() {
                last_seq = last.seq;
            }
            first_poll = false;
        }
    });

    let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
    Sse::new(rx_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(std::time::Duration::from_secs(15))
                .text("ping"),
        )
        .into_response()
}

/// Classify an audit action string into a level (info, warn, error).
fn classify_audit_level(action: &str) -> &'static str {
    let a = action.to_lowercase();
    if a.contains("error") || a.contains("fail") || a.contains("crash") || a.contains("denied") {
        "error"
    } else if a.contains("warn") || a.contains("block") || a.contains("kill") {
        "warn"
    } else {
        "info"
    }
}

// ---------------------------------------------------------------------------
// Peer endpoints
// ---------------------------------------------------------------------------

/// GET /api/peers — List known RHP peers.
pub async fn list_peers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if let Some(peer_registry) = state.kernel.peer_registry() {
        let peers: Vec<serde_json::Value> = peer_registry
            .all_peers()
            .iter()
            .map(|p| {
                serde_json::json!({
                    "node_id": p.node_id,
                    "node_name": p.node_name,
                    "address": p.address.to_string(),
                    "state": format!("{:?}", p.state),
                    "agents": p.agents.iter().map(|a| serde_json::json!({
                        "id": a.id,
                        "name": a.name,
                    })).collect::<Vec<_>>(),
                    "connected_at": p.connected_at.to_rfc3339(),
                    "protocol_version": p.protocol_version,
                })
            })
            .collect();
        Json(serde_json::json!({"peers": peers, "total": peers.len()}))
    } else {
        Json(serde_json::json!({"peers": [], "total": 0}))
    }
}

/// GET /api/network/status — RHP network status summary.
pub async fn network_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let enabled = state.kernel.config.network_enabled
        && !state.kernel.config.network.shared_secret.is_empty();

    let (node_id, listen_address, connected_peers, total_peers) =
        if let Some(peer_node) = state.kernel.peer_node() {
            let registry = peer_node.registry();
            (
                peer_node.node_id().to_string(),
                peer_node.local_addr().to_string(),
                registry.connected_count(),
                registry.total_count(),
            )
        } else {
            (String::new(), String::new(), 0, 0)
        };

    Json(serde_json::json!({
        "enabled": enabled,
        "node_id": node_id,
        "listen_address": listen_address,
        "connected_peers": connected_peers,
        "total_peers": total_peers,
    }))
}

// ---------------------------------------------------------------------------
// Tools endpoint
// ---------------------------------------------------------------------------

/// GET /api/tools — List all built-in tool definitions.
pub async fn list_tools() -> impl IntoResponse {
    let tools: Vec<serde_json::Value> = builtin_tool_definitions()
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema,
            })
        })
        .collect();

    Json(serde_json::json!({"tools": tools, "total": tools.len()}))
}

// ---------------------------------------------------------------------------
// Knowledge graph endpoint
// ---------------------------------------------------------------------------

/// GET /api/knowledge — Get knowledge graph (all entities and relations).
pub async fn knowledge_graph(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    use rusty_hand_types::memory::GraphPattern;

    // Query all relations (empty pattern = match all, limited to 100 by query_graph)
    let pattern = GraphPattern {
        source: None,
        relation: None,
        target: None,
        max_depth: 10,
    };

    match state.kernel.memory.query_graph(pattern).await {
        Ok(matches) => {
            let mut nodes: Vec<serde_json::Value> = Vec::new();
            let mut edges: Vec<serde_json::Value> = Vec::new();
            let mut seen_nodes = std::collections::HashSet::new();

            for m in &matches {
                // Source entity
                if seen_nodes.insert(m.source.id.clone()) {
                    nodes.push(serde_json::json!({
                        "id": m.source.id,
                        "type": m.source.entity_type,
                        "name": m.source.name,
                        "properties": m.source.properties,
                    }));
                }
                // Target entity
                if seen_nodes.insert(m.target.id.clone()) {
                    nodes.push(serde_json::json!({
                        "id": m.target.id,
                        "type": m.target.entity_type,
                        "name": m.target.name,
                        "properties": m.target.properties,
                    }));
                }
                // Relation (uses source/target from the entity match)
                edges.push(serde_json::json!({
                    "source": m.source.id,
                    "target": m.target.id,
                    "type": serde_json::to_string(&m.relation.relation).unwrap_or_default(),
                    "confidence": m.relation.confidence,
                    "properties": m.relation.properties,
                }));
            }

            Json(serde_json::json!({
                "nodes": nodes,
                "edges": edges,
                "total_nodes": nodes.len(),
                "total_edges": edges.len(),
            }))
        }
        Err(e) => {
            tracing::warn!("Knowledge graph query failed: {e}");
            Json(serde_json::json!({
                "nodes": [],
                "edges": [],
                "error": "Knowledge graph query failed",
            }))
        }
    }
}

// ---------------------------------------------------------------------------
// Config endpoint
// ---------------------------------------------------------------------------

/// GET /api/config — Get kernel configuration (secrets redacted).
pub async fn get_config(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Return a redacted view of the kernel config
    let config = &state.kernel.config;
    Json(serde_json::json!({
        "home_dir": config.home_dir.to_string_lossy(),
        "data_dir": config.data_dir.to_string_lossy(),
        "api_key": if config.api_key.is_empty() { "not set" } else { "***" },
        "default_model": {
            "provider": config.default_model.provider,
            "model": config.default_model.model,
            "api_key_env": config.default_model.api_key_env,
        },
        "memory": {
            "decay_rate": config.memory.decay_rate,
        },
    }))
}

// ---------------------------------------------------------------------------
// Usage endpoint
// ---------------------------------------------------------------------------

/// GET /api/usage — Get per-agent usage statistics.
pub async fn usage_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents: Vec<serde_json::Value> = state
        .kernel
        .registry
        .list()
        .iter()
        .map(|e| {
            let (tokens, tool_calls) = state.kernel.scheduler.get_usage(e.id).unwrap_or((0, 0));
            serde_json::json!({
                "agent_id": e.id.to_string(),
                "name": e.name,
                "total_tokens": tokens,
                "tool_calls": tool_calls,
            })
        })
        .collect();

    Json(serde_json::json!({"agents": agents}))
}

// ---------------------------------------------------------------------------
// Usage summary endpoints
// ---------------------------------------------------------------------------

/// GET /api/usage/summary — Get overall usage summary from UsageStore.
pub async fn usage_summary(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.kernel.memory.usage().query_summary(None) {
        Ok(s) => Json(serde_json::json!({
            "total_input_tokens": s.total_input_tokens,
            "total_output_tokens": s.total_output_tokens,
            "total_cost_usd": s.total_cost_usd,
            "call_count": s.call_count,
            "total_tool_calls": s.total_tool_calls,
        })),
        Err(_) => Json(serde_json::json!({
            "total_input_tokens": 0,
            "total_output_tokens": 0,
            "total_cost_usd": 0.0,
            "call_count": 0,
            "total_tool_calls": 0,
        })),
    }
}

/// GET /api/usage/by-model — Get usage grouped by model.
pub async fn usage_by_model(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.kernel.memory.usage().query_by_model() {
        Ok(models) => {
            let list: Vec<serde_json::Value> = models
                .iter()
                .map(|m| {
                    serde_json::json!({
                        "model": m.model,
                        "total_cost_usd": m.total_cost_usd,
                        "total_input_tokens": m.total_input_tokens,
                        "total_output_tokens": m.total_output_tokens,
                        "call_count": m.call_count,
                    })
                })
                .collect();
            Json(serde_json::json!({"models": list}))
        }
        Err(_) => Json(serde_json::json!({"models": []})),
    }
}

/// GET /api/usage/daily — Get daily usage breakdown for the last 7 days.
pub async fn usage_daily(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let days = state.kernel.memory.usage().query_daily_breakdown(7);
    let today_cost = state.kernel.memory.usage().query_today_cost();
    let first_event = state.kernel.memory.usage().query_first_event_date();

    let days_list = match days {
        Ok(d) => d
            .iter()
            .map(|day| {
                serde_json::json!({
                    "date": day.date,
                    "cost_usd": day.cost_usd,
                    "tokens": day.tokens,
                    "calls": day.calls,
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    Json(serde_json::json!({
        "days": days_list,
        "today_cost_usd": today_cost.unwrap_or(0.0),
        "first_event_date": first_event.unwrap_or(None),
    }))
}

// ---------------------------------------------------------------------------
// Budget endpoints
// ---------------------------------------------------------------------------

/// GET /api/budget — Current budget status (limits, spend, % used).
pub async fn budget_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let budget = state.kernel.budget_config();
    let status = state.kernel.metering.budget_status(&budget);
    Json(serde_json::to_value(&status).unwrap_or_default())
}

/// PUT /api/budget — Update global budget limits (in-memory only, not persisted to config.toml).
pub async fn update_budget(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let budget = state.kernel.update_budget_config(
        body["max_hourly_usd"].as_f64(),
        body["max_daily_usd"].as_f64(),
        body["max_monthly_usd"].as_f64(),
        body["alert_threshold"].as_f64(),
    );

    let status = state.kernel.metering.budget_status(&budget);
    Json(serde_json::to_value(&status).unwrap_or_default())
}

/// GET /api/budget/agents/{id} — Per-agent budget/quota status.
pub async fn agent_budget_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            )
        }
    };

    let quota = &entry.manifest.resources;
    let usage_store = rusty_hand_memory::usage::UsageStore::new(state.kernel.memory.usage_conn());
    let hourly = usage_store.query_hourly(agent_id).unwrap_or(0.0);
    let daily = usage_store.query_daily(agent_id).unwrap_or(0.0);
    let monthly = usage_store.query_monthly(agent_id).unwrap_or(0.0);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "agent_id": agent_id.to_string(),
            "agent_name": entry.name,
            "hourly": {
                "spend": hourly,
                "limit": quota.max_cost_per_hour_usd,
                "pct": if quota.max_cost_per_hour_usd > 0.0 { hourly / quota.max_cost_per_hour_usd } else { 0.0 },
            },
            "daily": {
                "spend": daily,
                "limit": quota.max_cost_per_day_usd,
                "pct": if quota.max_cost_per_day_usd > 0.0 { daily / quota.max_cost_per_day_usd } else { 0.0 },
            },
            "monthly": {
                "spend": monthly,
                "limit": quota.max_cost_per_month_usd,
                "pct": if quota.max_cost_per_month_usd > 0.0 { monthly / quota.max_cost_per_month_usd } else { 0.0 },
            },
        })),
    )
}

/// GET /api/budget/agents — Per-agent cost ranking (top spenders).
pub async fn agent_budget_ranking(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let usage_store = rusty_hand_memory::usage::UsageStore::new(state.kernel.memory.usage_conn());
    let agents: Vec<serde_json::Value> = state
        .kernel
        .registry
        .list()
        .iter()
        .filter_map(|entry| {
            let daily = usage_store.query_daily(entry.id).unwrap_or(0.0);
            if daily > 0.0 {
                Some(serde_json::json!({
                    "agent_id": entry.id.to_string(),
                    "name": entry.name,
                    "daily_cost_usd": daily,
                    "hourly_limit": entry.manifest.resources.max_cost_per_hour_usd,
                    "daily_limit": entry.manifest.resources.max_cost_per_day_usd,
                    "monthly_limit": entry.manifest.resources.max_cost_per_month_usd,
                }))
            } else {
                None
            }
        })
        .collect();

    Json(serde_json::json!({"agents": agents, "total": agents.len()}))
}

// ---------------------------------------------------------------------------
// Session listing endpoints
// ---------------------------------------------------------------------------

/// GET /api/sessions — List all sessions with metadata.
pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(pagination): Query<PaginationQuery>,
) -> impl IntoResponse {
    match state.kernel.memory.list_sessions() {
        Ok(sessions) => {
            let enriched: Vec<serde_json::Value> = sessions
                .into_iter()
                .map(|session| {
                    let agent_id = session["agent_id"].as_str().unwrap_or("").to_string();
                    let agent_name = uuid::Uuid::parse_str(&agent_id)
                        .ok()
                        .and_then(|uuid| state.kernel.registry.get(AgentId(uuid)))
                        .map(|entry| entry.name);
                    serde_json::json!({
                        "session_id": session["session_id"].as_str().unwrap_or(""),
                        "agent_id": agent_id,
                        "agent_name": agent_name,
                        "message_count": session["message_count"].as_u64().unwrap_or(0),
                        "created_at": session["created_at"].as_str().unwrap_or(""),
                        "label": session.get("label").cloned().unwrap_or(serde_json::Value::Null),
                    })
                })
                .collect();
            let total = enriched.len();
            let offset = pagination.offset();
            let limit = pagination.limit();
            let paginated: Vec<_> = enriched.into_iter().skip(offset).take(limit).collect();
            Json(
                serde_json::json!({"sessions": paginated, "total": total, "offset": offset, "limit": limit}),
            )
        }
        Err(_) => Json(serde_json::json!({"sessions": [], "total": 0})),
    }
}

/// GET /api/sessions/:id — Retrieve a single session with all messages as JSON.
pub async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session_id = match id.parse::<uuid::Uuid>() {
        Ok(u) => rusty_hand_types::agent::SessionId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid session ID"})),
            )
                .into_response();
        }
    };

    match state.kernel.memory.get_session(session_id) {
        Ok(Some(session)) => {
            let messages: Vec<serde_json::Value> = session
                .messages
                .iter()
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": session.id.0.to_string(),
                    "agent_id": session.agent_id.to_string(),
                    "label": session.label,
                    "message_count": session.messages.len(),
                    "context_window_tokens": session.context_window_tokens,
                    "messages": messages,
                })),
            )
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Session not found"})),
        )
            .into_response(),
        Err(e) => {
            safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session fetch", &e).into_response()
        }
    }
}

/// GET /api/sessions/:id/export.md — Export a session as Markdown.
pub async fn export_session_markdown(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    use axum::response::IntoResponse as _;

    let session_id = match id.parse::<uuid::Uuid>() {
        Ok(u) => rusty_hand_types::agent::SessionId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                axum::response::Html("Invalid session ID"),
            )
                .into_response();
        }
    };

    let session = match state.kernel.memory.get_session(session_id) {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                axum::response::Html("Session not found"),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::response::Html(format!("Error: {e}")),
            )
                .into_response();
        }
    };

    let mut md = String::new();
    let label = session.label.as_deref().unwrap_or("Untitled Session");
    md.push_str(&format!("# {label}\n\n"));
    md.push_str(&format!("_Session ID: {}_\n\n", session.id.0));
    md.push_str("---\n\n");

    for msg in &session.messages {
        use rusty_hand_types::message::{ContentBlock, MessageContent, Role};
        let role_label = match msg.role {
            Role::System => "**System**",
            Role::User => "**You**",
            Role::Assistant => "**Assistant**",
        };
        md.push_str(&format!("### {role_label}\n\n"));
        match &msg.content {
            MessageContent::Text(t) => {
                md.push_str(t);
                md.push_str("\n\n");
            }
            MessageContent::Blocks(blocks) => {
                for block in blocks {
                    match block {
                        ContentBlock::Text { text } => {
                            md.push_str(text);
                            md.push_str("\n\n");
                        }
                        ContentBlock::Thinking { thinking } => {
                            md.push_str("<details><summary>Thinking</summary>\n\n");
                            md.push_str(thinking);
                            md.push_str("\n\n</details>\n\n");
                        }
                        ContentBlock::ToolUse { name, input, .. } => {
                            md.push_str(&format!(
                                "```tool-call\n{name}({})\n```\n\n",
                                serde_json::to_string_pretty(input).unwrap_or_default()
                            ));
                        }
                        ContentBlock::ToolResult {
                            content, is_error, ..
                        } => {
                            let label = if *is_error { "error" } else { "result" };
                            md.push_str(&format!("```tool-{label}\n{content}\n```\n\n"));
                        }
                        ContentBlock::Image { media_type, .. } => {
                            md.push_str(&format!("_[Image: {media_type}]_\n\n"));
                        }
                        _ => {}
                    }
                }
            }
        }
        md.push_str("---\n\n");
    }

    let filename = format!(
        "{}.md",
        label
            .to_lowercase()
            .replace(|c: char| !c.is_alphanumeric(), "-")
            .trim_matches('-')
    );
    let disposition = format!("attachment; filename=\"{filename}\"");
    (
        StatusCode::OK,
        [
            ("Content-Type", "text/markdown; charset=utf-8"),
            ("Content-Disposition", disposition.as_str()),
        ],
        md,
    )
        .into_response()
}

/// DELETE /api/sessions/:id — Delete a session.
pub async fn delete_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let session_id = match id.parse::<uuid::Uuid>() {
        Ok(u) => rusty_hand_types::agent::SessionId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid session ID"})),
            );
        }
    };

    match state.kernel.memory.delete_session(session_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "deleted", "session_id": id})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session deletion", &e),
    }
}

/// PUT /api/sessions/:id/label — Set a session label.
pub async fn set_session_label(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let session_id = match id.parse::<uuid::Uuid>() {
        Ok(u) => rusty_hand_types::agent::SessionId(u),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid session ID"})),
            );
        }
    };

    let label = req.get("label").and_then(|v| v.as_str());

    // Validate label if present
    if let Some(lbl) = label {
        if let Err(e) = rusty_hand_types::agent::SessionLabel::new(lbl) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": e.to_string()})),
            );
        }
    }

    match state.kernel.memory.set_session_label(session_id, label) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "updated",
                "session_id": id,
                "label": label,
            })),
        ),
        Err(e) => safe_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Session label update",
            &e,
        ),
    }
}

/// GET /api/sessions/by-label/:label — Find session by label (scoped to agent).
pub async fn find_session_by_label(
    State(state): State<Arc<AppState>>,
    Path((agent_id_str, label)): Path<(String, String)>,
) -> impl IntoResponse {
    let agent_id = match agent_id_str.parse::<uuid::Uuid>() {
        Ok(u) => rusty_hand_types::agent::AgentId(u),
        Err(_) => {
            // Try name lookup
            match state.kernel.registry.find_by_name(&agent_id_str) {
                Some(entry) => entry.id,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "Agent not found"})),
                    );
                }
            }
        }
    };

    match state.kernel.memory.find_session_by_label(agent_id, &label) {
        Ok(Some(session)) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "session_id": session.id.0.to_string(),
                "agent_id": session.agent_id.0.to_string(),
                "label": session.label,
                "message_count": session.messages.len(),
            })),
        ),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No session found with that label"})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session lookup", &e),
    }
}

// ---------------------------------------------------------------------------
// Trigger update endpoint
// ---------------------------------------------------------------------------

/// PUT /api/triggers/:id — Update a trigger (enable/disable toggle).
pub async fn update_trigger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let trigger_id = TriggerId(match id.parse() {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid trigger ID"})),
            );
        }
    });

    if let Some(enabled) = req.get("enabled").and_then(|v| v.as_bool()) {
        if state.kernel.set_trigger_enabled(trigger_id, enabled) {
            (
                StatusCode::OK,
                Json(
                    serde_json::json!({"status": "updated", "trigger_id": id, "enabled": enabled}),
                ),
            )
        } else {
            (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Trigger not found"})),
            )
        }
    } else {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Missing 'enabled' field"})),
        )
    }
}

// ---------------------------------------------------------------------------
// Agent update endpoint
// ---------------------------------------------------------------------------

/// PUT /api/agents/:id — Update an agent (currently: re-set manifest fields).
pub async fn update_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AgentUpdateRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    if state.kernel.registry.get(agent_id).is_none() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        );
    }

    // Parse the new manifest
    let _manifest: AgentManifest = match toml::from_str(&req.manifest_toml) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("Invalid manifest: {e}")})),
            );
        }
    };

    // Note: Full manifest update requires kill + respawn. For now, acknowledge receipt.
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "acknowledged",
            "agent_id": id,
            "note": "Full manifest update requires agent restart. Use DELETE + POST to apply.",
        })),
    )
}

// ---------------------------------------------------------------------------
// Migration endpoint
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Security dashboard endpoint
// ---------------------------------------------------------------------------

/// GET /api/security — Security feature status for the dashboard.
pub async fn security_status(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let auth_mode = if state.kernel.config.api_key.is_empty() {
        "localhost_only"
    } else {
        "bearer_token"
    };

    let audit_count = state.kernel.audit_log.len();

    Json(serde_json::json!({
        "core_protections": {
            "path_traversal": true,
            "ssrf_protection": true,
            "capability_system": true,
            "privilege_escalation_prevention": true,
            "subprocess_isolation": true,
            "security_headers": true,
            "wire_hmac_auth": true,
            "request_id_tracking": true
        },
        "configurable": {
            "rate_limiter": {
                "enabled": true,
                "tokens_per_minute": 500,
                "algorithm": "GCRA"
            },
            "websocket_limits": {
                "max_per_ip": 5,
                "idle_timeout_secs": 1800,
                "max_message_size": 65536,
                "max_messages_per_minute": 10
            },
            "wasm_sandbox": {
                "fuel_metering": true,
                "epoch_interruption": true,
                "default_timeout_secs": 30,
                "default_fuel_limit": 1_000_000u64
            },
            "auth": {
                "mode": auth_mode,
                "api_key_set": !state.kernel.config.api_key.is_empty()
            }
        },
        "monitoring": {
            "audit_trail": {
                "enabled": true,
                "algorithm": "SHA-256 Merkle Chain",
                "entry_count": audit_count
            },
            "taint_tracking": {
                "enabled": true,
                "tracked_labels": [
                    "ExternalNetwork",
                    "UserInput",
                    "PII",
                    "Secret",
                    "UntrustedAgent"
                ]
            },
            "manifest_signing": {
                "algorithm": "Ed25519",
                "available": true
            }
        },
        "secret_zeroization": true,
        "total_features": 15
    }))
}

// ── Model Catalog Endpoints ─────────────────────────────────────────

/// GET /api/models — List all models in the catalog.
///
/// Query parameters:
/// - `provider` — filter by provider (e.g. `?provider=anthropic`)
/// - `tier` — filter by tier (e.g. `?tier=smart`)
/// - `available` — only show models from configured providers (`?available=true`)
pub async fn list_models(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let catalog = state
        .kernel
        .model_catalog
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let provider_filter = params.get("provider").map(|s| s.to_lowercase());
    let tier_filter = params.get("tier").map(|s| s.to_lowercase());
    let available_only = params
        .get("available")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(false);

    let models: Vec<serde_json::Value> = catalog
        .list_models()
        .iter()
        .filter(|m| {
            if let Some(ref p) = provider_filter {
                if m.provider.to_lowercase() != *p {
                    return false;
                }
            }
            if let Some(ref t) = tier_filter {
                if m.tier.to_string() != *t {
                    return false;
                }
            }
            if available_only {
                let provider = catalog.get_provider(&m.provider);
                if let Some(p) = provider {
                    if p.auth_status == rusty_hand_types::model_catalog::AuthStatus::Missing {
                        return false;
                    }
                }
            }
            true
        })
        .map(|m| {
            let available = catalog
                .get_provider(&m.provider)
                .map(|p| p.auth_status != rusty_hand_types::model_catalog::AuthStatus::Missing)
                .unwrap_or(false);
            serde_json::json!({
                "id": m.id,
                "display_name": m.display_name,
                "provider": m.provider,
                "tier": m.tier,
                "context_window": m.context_window,
                "max_output_tokens": m.max_output_tokens,
                "input_cost_per_m": m.input_cost_per_m,
                "output_cost_per_m": m.output_cost_per_m,
                "supports_tools": m.supports_tools,
                "supports_vision": m.supports_vision,
                "supports_streaming": m.supports_streaming,
                "available": available,
            })
        })
        .collect();

    let total = catalog.list_models().len();
    let available_count = catalog.available_models().len();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "models": models,
            "total": total,
            "available": available_count,
        })),
    )
}

/// GET /api/models/aliases — List all alias-to-model mappings.
pub async fn list_aliases(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let aliases = state
        .kernel
        .model_catalog
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .list_aliases()
        .clone();
    let entries: Vec<serde_json::Value> = aliases
        .iter()
        .map(|(alias, model_id)| {
            serde_json::json!({
                "alias": alias,
                "model_id": model_id,
            })
        })
        .collect();

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "aliases": entries,
            "total": entries.len(),
        })),
    )
}

/// GET /api/models/{id} — Get a single model by ID or alias.
pub async fn get_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let catalog = state
        .kernel
        .model_catalog
        .read()
        .unwrap_or_else(|e| e.into_inner());
    match catalog.find_model(&id) {
        Some(m) => {
            let available = catalog
                .get_provider(&m.provider)
                .map(|p| p.auth_status != rusty_hand_types::model_catalog::AuthStatus::Missing)
                .unwrap_or(false);
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "id": m.id,
                    "display_name": m.display_name,
                    "provider": m.provider,
                    "tier": m.tier,
                    "context_window": m.context_window,
                    "max_output_tokens": m.max_output_tokens,
                    "input_cost_per_m": m.input_cost_per_m,
                    "output_cost_per_m": m.output_cost_per_m,
                    "supports_tools": m.supports_tools,
                    "supports_vision": m.supports_vision,
                    "supports_streaming": m.supports_streaming,
                    "aliases": m.aliases,
                    "available": available,
                })),
            )
        }
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Model '{}' not found", id)})),
        ),
    }
}

/// GET /api/providers — List all providers with auth status.
///
/// For local providers (ollama, vllm, lmstudio), also probes reachability and
/// discovers available models via their health endpoints.
pub async fn list_providers(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let provider_list: Vec<rusty_hand_types::model_catalog::ProviderInfo> = {
        let catalog = state
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        catalog.list_providers().to_vec()
    };

    let mut providers: Vec<serde_json::Value> = Vec::with_capacity(provider_list.len());

    for p in &provider_list {
        let mut entry = serde_json::json!({
            "id": p.id,
            "display_name": p.display_name,
            "auth_status": p.auth_status,
            "model_count": p.model_count,
            "key_required": p.key_required,
            "api_key_env": p.api_key_env,
        });

        // For local providers, add reachability info via health probe
        if !p.key_required {
            entry["is_local"] = serde_json::json!(true);
            let probe =
                rusty_hand_runtime::provider_health::probe_provider(&p.id, &p.base_url).await;
            entry["reachable"] = serde_json::json!(probe.reachable);
            entry["latency_ms"] = serde_json::json!(probe.latency_ms);
            if !probe.discovered_models.is_empty() {
                entry["discovered_models"] = serde_json::json!(probe.discovered_models);
            }
            if let Some(err) = &probe.error {
                entry["error"] = serde_json::json!(err);
            }
        }

        providers.push(entry);
    }

    let total = providers.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "providers": providers,
            "total": total,
        })),
    )
}

// ── A2A (Agent-to-Agent) Protocol Endpoints ─────────────────────────

/// GET /.well-known/agent.json — A2A Agent Card for the default agent.
pub async fn a2a_agent_card(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents = state.kernel.registry.list();
    let base_url = format!("http://{}", state.kernel.config.api_listen);

    if let Some(first) = agents.first() {
        let card = rusty_hand_runtime::a2a::build_agent_card(&first.manifest, &base_url);
        (
            StatusCode::OK,
            Json(serde_json::to_value(&card).unwrap_or_default()),
        )
    } else {
        let card = serde_json::json!({
            "name": "rustyhand",
            "description": "RustyHand Agent OS — no agents spawned yet",
            "url": format!("{base_url}/a2a"),
            "version": "0.1.0",
            "capabilities": { "streaming": true },
            "skills": [],
            "defaultInputModes": ["text"],
            "defaultOutputModes": ["text"],
        });
        (StatusCode::OK, Json(card))
    }
}

/// GET /a2a/agents — List all A2A agent cards.
pub async fn a2a_list_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents = state.kernel.registry.list();
    let base_url = format!("http://{}", state.kernel.config.api_listen);

    let cards: Vec<serde_json::Value> = agents
        .iter()
        .map(|entry| {
            let card = rusty_hand_runtime::a2a::build_agent_card(&entry.manifest, &base_url);
            serde_json::to_value(&card).unwrap_or_default()
        })
        .collect();

    let total = cards.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "agents": cards,
            "total": total,
        })),
    )
}

/// POST /a2a/tasks/send — Submit a task to an agent via A2A.
pub async fn a2a_send_task(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Extract message text from A2A format
    let message_text = request["params"]["message"]["parts"]
        .as_array()
        .and_then(|parts| {
            parts.iter().find_map(|p| {
                if p["type"].as_str() == Some("text") {
                    p["text"].as_str().map(String::from)
                } else {
                    None
                }
            })
        })
        .unwrap_or_else(|| "No message provided".to_string());

    // Find target agent (use first available or specified)
    let agents = state.kernel.registry.list();
    if agents.is_empty() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "No agents available"})),
        );
    }

    let agent = &agents[0];
    let task_id = uuid::Uuid::new_v4().to_string();
    let session_id = request["params"]["sessionId"].as_str().map(String::from);

    // Create the task in the store as Working
    let task = rusty_hand_runtime::a2a::A2aTask {
        id: task_id.clone(),
        session_id: session_id.clone(),
        status: rusty_hand_runtime::a2a::A2aTaskStatus::Working,
        messages: vec![rusty_hand_runtime::a2a::A2aMessage {
            role: "user".to_string(),
            parts: vec![rusty_hand_runtime::a2a::A2aPart::Text {
                text: message_text.clone(),
            }],
        }],
        artifacts: vec![],
    };
    state.kernel.a2a_task_store.insert(task);

    // Send message to agent
    match state.kernel.send_message(agent.id, &message_text).await {
        Ok(result) => {
            let response_msg = rusty_hand_runtime::a2a::A2aMessage {
                role: "agent".to_string(),
                parts: vec![rusty_hand_runtime::a2a::A2aPart::Text {
                    text: result.response,
                }],
            };
            state
                .kernel
                .a2a_task_store
                .complete(&task_id, response_msg, vec![]);
            match state.kernel.a2a_task_store.get(&task_id) {
                Some(completed_task) => (
                    StatusCode::OK,
                    Json(serde_json::to_value(&completed_task).unwrap_or_default()),
                ),
                None => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Task disappeared after completion"})),
                ),
            }
        }
        Err(e) => {
            let error_msg = rusty_hand_runtime::a2a::A2aMessage {
                role: "agent".to_string(),
                parts: vec![rusty_hand_runtime::a2a::A2aPart::Text {
                    text: "Agent task failed".to_string(),
                }],
            };
            state.kernel.a2a_task_store.fail(&task_id, error_msg);
            match state.kernel.a2a_task_store.get(&task_id) {
                Some(failed_task) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::to_value(&failed_task).unwrap_or_default()),
                ),
                None => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Agent task", &e),
            }
        }
    }
}

/// GET /a2a/tasks/{id} — Get task status from the task store.
pub async fn a2a_get_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    match state.kernel.a2a_task_store.get(&task_id) {
        Some(task) => (
            StatusCode::OK,
            Json(serde_json::to_value(&task).unwrap_or_default()),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Task '{}' not found", task_id)})),
        ),
    }
}

/// POST /a2a/tasks/{id}/cancel — Cancel a tracked task.
pub async fn a2a_cancel_task(
    State(state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    if state.kernel.a2a_task_store.cancel(&task_id) {
        match state.kernel.a2a_task_store.get(&task_id) {
            Some(task) => (
                StatusCode::OK,
                Json(serde_json::to_value(&task).unwrap_or_default()),
            ),
            None => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Task disappeared after cancellation"})),
            ),
        }
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Task '{}' not found", task_id)})),
        )
    }
}

// ── A2A Management Endpoints (outbound) ─────────────────────────────────

/// GET /api/a2a/agents — List discovered external A2A agents.
pub async fn a2a_list_external_agents(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let agents = state
        .kernel
        .a2a_external_agents
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let items: Vec<serde_json::Value> = agents
        .iter()
        .map(|(url, card)| {
            serde_json::json!({
                "name": card.name,
                "url": url,
                "description": card.description,
                "skills": card.skills,
                "version": card.version,
            })
        })
        .collect();
    Json(serde_json::json!({"agents": items, "total": items.len()}))
}

/// POST /api/a2a/discover — Discover a new external A2A agent by URL.
pub async fn a2a_discover_external(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let url = match body["url"].as_str() {
        Some(u) => u.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'url' field"})),
            )
        }
    };

    // SECURITY: Block SSRF — prevent requests to internal/private networks
    if let Err(e) = rusty_hand_runtime::check_ssrf(&url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        );
    }

    let client = rusty_hand_runtime::a2a::A2aClient::new();
    match client.discover(&url).await {
        Ok(card) => {
            let card_json = serde_json::to_value(&card).unwrap_or_default();
            // Store in kernel's external agents list
            {
                let mut agents = state
                    .kernel
                    .a2a_external_agents
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                // Update or add
                if let Some(existing) = agents.iter_mut().find(|(u, _)| u == &url) {
                    existing.1 = card;
                } else {
                    agents.push((url.clone(), card));
                }
            }
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "url": url,
                    "agent": card_json,
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// POST /api/a2a/send — Send a task to an external A2A agent.
pub async fn a2a_send_external(
    State(_state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let url = match body["url"].as_str() {
        Some(u) => u.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'url' field"})),
            )
        }
    };

    // SECURITY: Block SSRF — prevent requests to internal/private networks
    if let Err(e) = rusty_hand_runtime::check_ssrf(&url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        );
    }

    let message = match body["message"].as_str() {
        Some(m) => m.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'message' field"})),
            )
        }
    };
    let session_id = body["session_id"].as_str();

    let client = rusty_hand_runtime::a2a::A2aClient::new();
    match client.send_task(&url, &message, session_id).await {
        Ok(task) => (
            StatusCode::OK,
            Json(serde_json::to_value(&task).unwrap_or_default()),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// GET /api/a2a/tasks/{id}/status — Get task status from an external A2A agent.
pub async fn a2a_external_task_status(
    State(_state): State<Arc<AppState>>,
    Path(task_id): Path<String>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let url = match params.get("url") {
        Some(u) => u.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'url' query parameter"})),
            )
        }
    };

    // SECURITY: Block SSRF — prevent requests to internal/private networks
    if let Err(e) = rusty_hand_runtime::check_ssrf(&url) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        );
    }

    let client = rusty_hand_runtime::a2a::A2aClient::new();
    match client.get_task(&url, &task_id).await {
        Ok(task) => (
            StatusCode::OK,
            Json(serde_json::to_value(&task).unwrap_or_default()),
        ),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ── MCP HTTP Endpoint ───────────────────────────────────────────────────

/// POST /mcp — Handle MCP JSON-RPC requests over HTTP.
///
/// Exposes the same MCP protocol normally served via stdio, allowing
/// external MCP clients to connect over HTTP instead.
pub async fn mcp_http(
    State(state): State<Arc<AppState>>,
    Json(request): Json<serde_json::Value>,
) -> impl IntoResponse {
    // SECURITY: MCP server disabled by config → reject all requests.
    if !state.kernel.config.mcp_server.enabled {
        return Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": request.get("id").cloned(),
            "error": {"code": -32000, "message": "MCP server is disabled by config (mcp_server.enabled = false)"}
        }));
    }

    // Gather all available tools (builtin + skills + external MCP),
    // then filter to the allowlist so tools/list only shows callable tools.
    let mcp_cfg = &state.kernel.config.mcp_server;
    let mut tools = builtin_tool_definitions();
    {
        let registry = state
            .kernel
            .skill_registry
            .read()
            .unwrap_or_else(|e| e.into_inner());
        for skill_tool in registry.all_tool_definitions() {
            tools.push(rusty_hand_types::tool::ToolDefinition {
                name: skill_tool.name.clone(),
                description: skill_tool.description.clone(),
                input_schema: skill_tool.input_schema.clone(),
            });
        }
    }
    if let Ok(mcp_tools) = state.kernel.mcp_tools.lock() {
        tools.extend(mcp_tools.iter().cloned());
    }
    // Apply the MCP allowlist BEFORE exposing tools/list or executing calls.
    // Without this, any authenticated MCP client could call shell_exec,
    // skill_install, file_write etc. — effectively unscoped RCE.
    tools.retain(|t| mcp_cfg.is_tool_allowed(&t.name));

    // Check if this is a tools/call that needs real execution
    let method = request["method"].as_str().unwrap_or("");
    if method == "tools/call" {
        let tool_name = request["params"]["name"].as_str().unwrap_or("");
        let arguments = request["params"]
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // SECURITY: second line of defence — even if somehow the tool slipped
        // into the list, double-check the allowlist before execution. Also
        // distinguishes "unknown tool" (likely typo) from "tool denied" (auth).
        if !mcp_cfg.is_tool_allowed(tool_name) {
            tracing::warn!(
                tool_name,
                "MCP tool call denied by allowlist — add to mcp_server.extra_allowed_tools to permit"
            );
            return Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned(),
                "error": {
                    "code": -32000,
                    "message": format!(
                        "Tool '{tool_name}' not allowed via MCP. Add to [mcp_server] extra_allowed_tools in config.toml to enable."
                    )
                }
            }));
        }

        // Verify the tool exists (after auth check so we don't leak existence
        // of privileged tools to unauthorised callers — but in our case the
        // allowlist covers that).
        if !tools.iter().any(|t| t.name == tool_name) {
            return Json(serde_json::json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned(),
                "error": {"code": -32602, "message": format!("Unknown tool: {tool_name}")}
            }));
        }

        // Snapshot skill registry before async call (RwLockReadGuard is !Send)
        let skill_snapshot = state
            .kernel
            .skill_registry
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .snapshot();

        // Execute the tool via the kernel's tool runner
        let kernel_handle: Arc<dyn rusty_hand_runtime::kernel_handle::KernelHandle> =
            state.kernel.clone() as Arc<dyn rusty_hand_runtime::kernel_handle::KernelHandle>;
        let result = rusty_hand_runtime::tool_runner::execute_tool(
            "mcp-http",
            tool_name,
            &arguments,
            Some(&kernel_handle),
            None,
            None,
            Some(&skill_snapshot),
            Some(&state.kernel.mcp_connections),
            Some(&state.kernel.web_ctx),
            Some(&state.kernel.browser_ctx),
            None,
            None,
            Some(&state.kernel.media_engine),
            None, // exec_policy
            if state.kernel.config.tts.enabled {
                Some(&state.kernel.tts_engine)
            } else {
                None
            },
            if state.kernel.config.docker.enabled {
                Some(&state.kernel.config.docker)
            } else {
                None
            },
            Some(&*state.kernel.process_manager),
        )
        .await;

        return Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": request.get("id").cloned(),
            "result": {
                "content": [{"type": "text", "text": result.content}],
                "isError": result.is_error,
            }
        }));
    }

    // For non-tools/call methods (initialize, tools/list, etc.), delegate to the handler
    let response = rusty_hand_runtime::mcp_server::handle_mcp_request(&request, &tools).await;
    Json(response)
}

// ── Multi-Session Endpoints ─────────────────────────────────────────────

/// GET /api/agents/{id}/sessions — List all sessions for an agent.
pub async fn list_agent_sessions(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    match state.kernel.list_agent_sessions(agent_id) {
        Ok(sessions) => (
            StatusCode::OK,
            Json(serde_json::json!({"sessions": sessions})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session listing", &e),
    }
}

/// POST /api/agents/{id}/sessions — Create a new session for an agent.
pub async fn create_agent_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let label = req.get("label").and_then(|v| v.as_str());
    match state.kernel.create_agent_session(agent_id, label) {
        Ok(session) => (StatusCode::OK, Json(session)),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session creation", &e),
    }
}

/// POST /api/agents/{id}/sessions/{session_id}/switch — Switch to an existing session.
pub async fn switch_agent_session(
    State(state): State<Arc<AppState>>,
    Path((id, session_id_str)): Path<(String, String)>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let session_id = match session_id_str.parse::<uuid::Uuid>() {
        Ok(uuid) => rusty_hand_types::agent::SessionId(uuid),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid session ID"})),
            )
        }
    };
    match state.kernel.switch_agent_session(agent_id, session_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Session switched"})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session switch", &e),
    }
}

// ── Extended Chat Command API Endpoints ─────────────────────────────────

/// POST /api/agents/{id}/session/reset — Reset an agent's session.
pub async fn reset_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    match state.kernel.reset_session(agent_id) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Session reset"})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session reset", &e),
    }
}

/// POST /api/agents/{id}/session/compact — Trigger LLM session compaction.
pub async fn compact_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    match state.kernel.compact_agent_session(agent_id).await {
        Ok(msg) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": msg})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Session compaction", &e),
    }
}

/// POST /api/agents/{id}/stop — Cancel an agent's current LLM run.
/// GET /api/agents/:id/metrics — Per-agent usage and performance metrics.
pub async fn get_agent_metrics(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    use rusty_hand_runtime::kernel_handle::KernelHandle;
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    if state.kernel.registry.get(agent_id).is_none() {
        return agent_not_found(&id).into_response();
    }
    match state.kernel.agent_metrics(&agent_id.to_string()) {
        Ok(metrics) => (StatusCode::OK, Json(metrics)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Metrics query failed: {e}")})),
        )
            .into_response(),
    }
}

/// GET /api/agents/:id/memories — list episodic memories for an agent.
/// Query params: q (search), limit (default 50), offset (default 0).
pub async fn list_agent_memories(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> axum::response::Response {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e.into_response(),
    };
    if state.kernel.registry.get(agent_id).is_none() {
        return agent_not_found(&id).into_response();
    }

    let q = params
        .get("q")
        .map(|s| s.as_str())
        .unwrap_or("")
        .to_string();
    let limit: usize = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .min(200);
    let offset: usize = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let filter = rusty_hand_types::memory::MemoryFilter {
        agent_id: Some(agent_id),
        ..Default::default()
    };

    match state
        .kernel
        .memory
        .recall_with_embedding(&q, limit + offset, Some(filter), None)
    {
        Ok(fragments) => {
            let total = fragments.len();
            let items: Vec<_> = fragments
                .into_iter()
                .skip(offset)
                .take(limit)
                .map(|f| {
                    serde_json::json!({
                        "id": f.id.0.to_string(),
                        "content": f.content,
                        "source": format!("{:?}", f.source),
                        "scope": f.scope,
                        "confidence": f.confidence,
                        "access_count": f.access_count,
                        "created_at": f.created_at.to_rfc3339(),
                        "accessed_at": f.accessed_at.to_rfc3339(),
                    })
                })
                .collect();
            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "memories": items,
                    "total": total,
                    "offset": offset,
                    "limit": limit,
                })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to load memories: {e}")})),
        )
            .into_response(),
    }
}

pub async fn stop_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    match state.kernel.stop_agent_run(agent_id) {
        Ok(true) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "Run cancelled"})),
        ),
        Ok(false) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "message": "No active run"})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Stop agent", &e),
    }
}

/// PUT /api/agents/{id}/model — Switch an agent's model.
pub async fn set_model(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let model = match body["model"].as_str() {
        Some(m) if !m.is_empty() => m,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'model' field"})),
            )
        }
    };
    match state.kernel.set_agent_model(agent_id, model) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "model": model})),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Model switch", &e),
    }
}

// ── Per-Agent Skill & MCP Endpoints ────────────────────────────────────

/// GET /api/agents/{id}/skills — Get an agent's skill assignment info.
pub async fn get_agent_skills(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            )
        }
    };
    let available = state
        .kernel
        .skill_registry
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .skill_names();
    let mode = if entry.manifest.skills.is_empty() {
        "all"
    } else {
        "allowlist"
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "assigned": entry.manifest.skills,
            "available": available,
            "mode": mode,
        })),
    )
}

/// PUT /api/agents/{id}/skills — Update an agent's skill allowlist.
pub async fn set_agent_skills(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let skills: Vec<String> = body["skills"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match state.kernel.set_agent_skills(agent_id, skills.clone()) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "skills": skills})),
        ),
        Err(e) => safe_error(StatusCode::BAD_REQUEST, "Skill assignment", &e),
    }
}

/// GET /api/agents/{id}/mcp_servers — Get an agent's MCP server assignment info.
pub async fn get_agent_mcp_servers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            )
        }
    };
    // Collect known MCP server names from connected tools
    let mut available: Vec<String> = Vec::new();
    if let Ok(mcp_tools) = state.kernel.mcp_tools.lock() {
        let mut seen = std::collections::HashSet::new();
        for tool in mcp_tools.iter() {
            if let Some(server) = rusty_hand_runtime::mcp::extract_mcp_server(&tool.name) {
                if seen.insert(server.to_string()) {
                    available.push(server.to_string());
                }
            }
        }
    }
    let mode = if entry.manifest.mcp_servers.is_empty() {
        "all"
    } else {
        "allowlist"
    };
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "assigned": entry.manifest.mcp_servers,
            "available": available,
            "mode": mode,
        })),
    )
}

/// PUT /api/agents/{id}/mcp_servers — Update an agent's MCP server allowlist.
pub async fn set_agent_mcp_servers(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            )
        }
    };
    let servers: Vec<String> = body["mcp_servers"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    match state
        .kernel
        .set_agent_mcp_servers(agent_id, servers.clone())
    {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "mcp_servers": servers})),
        ),
        Err(e) => safe_error(StatusCode::BAD_REQUEST, "MCP server assignment", &e),
    }
}

// ── Provider Key Management Endpoints ──────────────────────────────────

/// POST /api/providers/{name}/key — Save an API key for a provider.
///
/// SECURITY: Writes to `~/.rustyhand/secrets.env`, sets env var in process,
/// and refreshes auth detection. Key is zeroized after use.
pub async fn set_provider_key(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Validate provider name against known list
    {
        let catalog = state
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if catalog.get_provider(&name).is_none() {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": format!("Unknown provider '{}'", name)})),
            );
        }
    }

    let key = match body["key"].as_str() {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing or empty 'key' field"})),
            );
        }
    };

    let env_var = {
        let catalog = state
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        catalog
            .get_provider(&name)
            .map(|p| p.api_key_env.clone())
            .unwrap_or_default()
    };

    if env_var.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider does not require an API key"})),
        );
    }

    // Write to secrets.env file
    let secrets_path = state.kernel.config.home_dir.join("secrets.env");
    if let Err(e) = write_secret_env(&secrets_path, &env_var, &key) {
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Provider key save", &e);
    }

    // Set env var in current process so detect_auth picks it up
    std::env::set_var(&env_var, &key);

    // Refresh auth detection
    state
        .kernel
        .model_catalog
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .detect_auth();

    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "saved", "provider": name})),
    )
}

/// DELETE /api/providers/{name}/key — Remove an API key for a provider.
pub async fn delete_provider_key(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let env_var = {
        let catalog = state
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        match catalog.get_provider(&name) {
            Some(p) => p.api_key_env.clone(),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("Unknown provider '{}'", name)})),
                );
            }
        }
    };

    if env_var.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider does not require an API key"})),
        );
    }

    // Remove from secrets.env
    let secrets_path = state.kernel.config.home_dir.join("secrets.env");
    if let Err(e) = remove_secret_env(&secrets_path, &env_var) {
        return safe_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Provider key removal",
            &e,
        );
    }

    // Remove from process environment
    std::env::remove_var(&env_var);

    // Refresh auth detection
    state
        .kernel
        .model_catalog
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .detect_auth();

    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "removed", "provider": name})),
    )
}

/// POST /api/providers/{name}/test — Test a provider's connectivity.
pub async fn test_provider(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let (env_var, base_url, key_required) = {
        let catalog = state
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        match catalog.get_provider(&name) {
            Some(p) => (p.api_key_env.clone(), p.base_url.clone(), p.key_required),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": format!("Unknown provider '{}'", name)})),
                );
            }
        }
    };

    let api_key = std::env::var(&env_var).ok();
    // Only require API key for providers that need one (skip local providers like ollama/vllm/lmstudio)
    if key_required && api_key.is_none() && !env_var.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Provider API key not configured"})),
        );
    }

    // Attempt a lightweight connectivity test
    let start = std::time::Instant::now();
    let driver_config = rusty_hand_runtime::llm_driver::DriverConfig {
        provider: name.clone(),
        api_key,
        base_url: if base_url.is_empty() {
            None
        } else {
            Some(base_url)
        },
    };

    match rusty_hand_runtime::drivers::create_driver(&driver_config) {
        Ok(driver) => {
            // Send a minimal completion request to test connectivity
            let test_req = rusty_hand_runtime::llm_driver::CompletionRequest {
                model: String::new(), // Driver will use default
                messages: vec![rusty_hand_types::message::Message::user("Hi")],
                tools: vec![],
                max_tokens: 1,
                temperature: 0.0,
                system: None,
                thinking: None,
                response_format: Default::default(),
            };
            match driver.complete(test_req).await {
                Ok(_) => {
                    let latency_ms = start.elapsed().as_millis();
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "ok",
                            "provider": name,
                            "latency_ms": latency_ms,
                        })),
                    )
                }
                Err(e) => safe_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Provider connection test",
                    &e,
                ),
            }
        }
        Err(e) => safe_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Provider driver creation",
            &e,
        ),
    }
}

/// POST /api/skills/create — Create a local prompt-only skill.
pub async fn create_skill(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let name = match body["name"].as_str() {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing or empty 'name' field"})),
            );
        }
    };

    // Validate name (alphanumeric + hyphens only)
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "Skill name must contain only letters, numbers, hyphens, and underscores"}),
            ),
        );
    }

    let description = body["description"].as_str().unwrap_or("").to_string();
    let runtime = body["runtime"].as_str().unwrap_or("prompt_only");
    let prompt_context = body["prompt_context"].as_str().unwrap_or("").to_string();

    // Only allow prompt_only skills from the web UI for safety
    if runtime != "prompt_only" {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "Only prompt_only skills can be created from the web UI"}),
            ),
        );
    }

    // Write skill.toml to ~/.rustyhand/skills/{name}/
    let skill_dir = state.kernel.config.home_dir.join("skills").join(&name);
    if skill_dir.exists() {
        return (
            StatusCode::CONFLICT,
            Json(serde_json::json!({"error": format!("Skill '{}' already exists", name)})),
        );
    }

    if let Err(e) = std::fs::create_dir_all(&skill_dir) {
        return safe_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Skill directory creation",
            &e,
        );
    }

    let toml_content = format!(
        "[skill]\nname = \"{}\"\ndescription = \"{}\"\nruntime = \"prompt_only\"\n\n[prompt]\ncontext = \"\"\"\n{}\n\"\"\"\n",
        name,
        description.replace('"', "\\\""),
        prompt_context
    );

    let toml_path = skill_dir.join("skill.toml");
    if let Err(e) = std::fs::write(&toml_path, &toml_content) {
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Skill creation", &e);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "created",
            "name": name,
            "note": "Restart the daemon to load the new skill, or it will be available on next boot."
        })),
    )
}

// ── Helper functions for secrets.env management ────────────────────────

/// Write or update a key in the secrets.env file.
/// File format: one `KEY=value` per line. Existing keys are overwritten.
fn write_secret_env(path: &std::path::Path, key: &str, value: &str) -> Result<(), std::io::Error> {
    let mut lines: Vec<String> = if path.exists() {
        std::fs::read_to_string(path)?
            .lines()
            .map(|l| l.to_string())
            .collect()
    } else {
        Vec::new()
    };

    // Remove existing line for this key
    lines.retain(|l| !l.starts_with(&format!("{key}=")));

    // Add new line
    lines.push(format!("{key}={value}"));

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(path, lines.join("\n") + "\n")?;

    // SECURITY: Restrict file permissions on Unix — secrets must not be world-readable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            tracing::error!(path = %path.display(), error = %e, "SECURITY: Failed to set 0600 permissions on secrets file");
        }
    }

    Ok(())
}

/// Remove a key from the secrets.env file.
fn remove_secret_env(path: &std::path::Path, key: &str) -> Result<(), std::io::Error> {
    if !path.exists() {
        return Ok(());
    }

    let lines: Vec<String> = std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.starts_with(&format!("{key}=")))
        .map(|l| l.to_string())
        .collect();

    std::fs::write(path, lines.join("\n") + "\n")?;

    Ok(())
}

// ── Config.toml channel management helpers ──────────────────────────

/// Upsert a `[channels.<name>]` section in config.toml with the given non-secret fields.
fn upsert_channel_config(
    config_path: &std::path::Path,
    channel_name: &str,
    fields: &HashMap<String, String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = if config_path.exists() {
        std::fs::read_to_string(config_path)?
    } else {
        String::new()
    };

    let mut doc: toml::Value = if content.trim().is_empty() {
        toml::Value::Table(toml::map::Map::new())
    } else {
        toml::from_str(&content)?
    };

    let root = doc.as_table_mut().ok_or("Config is not a TOML table")?;

    // Ensure [channels] table exists
    if !root.contains_key("channels") {
        root.insert(
            "channels".to_string(),
            toml::Value::Table(toml::map::Map::new()),
        );
    }
    let channels_table = root
        .get_mut("channels")
        .and_then(|v| v.as_table_mut())
        .ok_or("channels is not a table")?;

    // Build channel sub-table
    let mut ch_table = toml::map::Map::new();
    for (k, v) in fields {
        ch_table.insert(k.clone(), toml::Value::String(v.clone()));
    }
    channels_table.insert(channel_name.to_string(), toml::Value::Table(ch_table));

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::write(config_path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}

/// Remove a `[channels.<name>]` section from config.toml.
fn remove_channel_config(
    config_path: &std::path::Path,
    channel_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if !config_path.exists() {
        return Ok(());
    }

    let content = std::fs::read_to_string(config_path)?;
    if content.trim().is_empty() {
        return Ok(());
    }

    let mut doc: toml::Value = toml::from_str(&content)?;

    if let Some(channels) = doc
        .as_table_mut()
        .and_then(|r| r.get_mut("channels"))
        .and_then(|c| c.as_table_mut())
    {
        channels.remove(channel_name);
    }

    std::fs::write(config_path, toml::to_string_pretty(&doc)?)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Integration management endpoints
// ---------------------------------------------------------------------------

/// GET /api/integrations — List installed integrations with status.
pub async fn list_integrations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state
        .kernel
        .extension_registry
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let health = &state.kernel.extension_health;

    let mut entries = Vec::new();
    for info in registry.list_all_info() {
        let h = health.get_health(&info.template.id);
        let status = match &info.installed {
            Some(inst) if !inst.enabled => "disabled",
            Some(_) => match h.as_ref().map(|h| &h.status) {
                Some(rusty_hand_extensions::IntegrationStatus::Ready) => "ready",
                Some(rusty_hand_extensions::IntegrationStatus::Error(_)) => "error",
                _ => "installed",
            },
            None => continue, // Only show installed
        };
        entries.push(serde_json::json!({
            "id": info.template.id,
            "name": info.template.name,
            "icon": info.template.icon,
            "category": info.template.category.to_string(),
            "status": status,
            "tool_count": h.as_ref().map(|h| h.tool_count).unwrap_or(0),
            "installed_at": info.installed.as_ref().map(|i| i.installed_at.to_rfc3339()),
        }));
    }

    Json(serde_json::json!({
        "installed": entries,
        "count": entries.len(),
    }))
}

/// GET /api/integrations/available — List all available templates.
pub async fn list_available_integrations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let registry = state
        .kernel
        .extension_registry
        .read()
        .unwrap_or_else(|e| e.into_inner());
    let templates: Vec<serde_json::Value> = registry
        .list_templates()
        .iter()
        .map(|t| {
            let installed = registry.is_installed(&t.id);
            serde_json::json!({
                "id": t.id,
                "name": t.name,
                "description": t.description,
                "icon": t.icon,
                "category": t.category.to_string(),
                "installed": installed,
                "tags": t.tags,
                "required_env": t.required_env.iter().map(|e| serde_json::json!({
                    "name": e.name,
                    "label": e.label,
                    "help": e.help,
                    "is_secret": e.is_secret,
                    "get_url": e.get_url,
                })).collect::<Vec<_>>(),
                "has_oauth": t.oauth.is_some(),
                "setup_instructions": t.setup_instructions,
            })
        })
        .collect();

    Json(serde_json::json!({
        "integrations": templates,
        "count": templates.len(),
    }))
}

/// POST /api/integrations/add — Install an integration.
pub async fn add_integration(
    State(state): State<Arc<AppState>>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let id = match req.get("id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Missing 'id' field"})),
            );
        }
    };

    // Scope the write lock so it's dropped before any .await
    let install_err = {
        let mut registry = state
            .kernel
            .extension_registry
            .write()
            .unwrap_or_else(|e| e.into_inner());

        if registry.is_installed(&id) {
            Some((
                StatusCode::CONFLICT,
                format!("Integration '{}' already installed", id),
            ))
        } else if registry.get_template(&id).is_none() {
            Some((
                StatusCode::NOT_FOUND,
                format!("Unknown integration: '{}'", id),
            ))
        } else {
            let entry = rusty_hand_extensions::InstalledIntegration {
                id: id.clone(),
                installed_at: chrono::Utc::now(),
                enabled: true,
                oauth_provider: None,
                config: std::collections::HashMap::new(),
            };
            match registry.install(entry) {
                Ok(_) => None,
                Err(e) => Some((StatusCode::INTERNAL_SERVER_ERROR, format!("{e}"))),
            }
        }
    }; // write lock dropped here

    if let Some((status, error)) = install_err {
        return safe_error(status, "Integration install", &error);
    }

    state.kernel.extension_health.register(&id);

    // Hot-connect the new MCP server
    let connected = state.kernel.reload_extension_mcps().await.unwrap_or(0);

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "id": id,
            "status": "installed",
            "connected": connected > 0,
            "message": format!("Integration '{}' installed", id),
        })),
    )
}

/// DELETE /api/integrations/:id — Remove an integration.
pub async fn remove_integration(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    // Scope the write lock
    let uninstall_err = {
        let mut registry = state
            .kernel
            .extension_registry
            .write()
            .unwrap_or_else(|e| e.into_inner());
        registry.uninstall(&id).err()
    };

    if let Some(e) = uninstall_err {
        return safe_error(StatusCode::NOT_FOUND, "Integration removal", &e);
    }

    state.kernel.extension_health.unregister(&id);

    // Hot-disconnect the removed MCP server
    if let Err(e) = state.kernel.reload_extension_mcps().await {
        tracing::warn!(error = %e, "Failed to reload MCP servers after extension removal");
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "id": id,
            "status": "removed",
        })),
    )
}

/// POST /api/integrations/:id/reconnect — Reconnect an MCP server.
pub async fn reconnect_integration(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let is_installed = {
        let registry = state
            .kernel
            .extension_registry
            .read()
            .unwrap_or_else(|e| e.into_inner());
        registry.is_installed(&id)
    };

    if !is_installed {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": format!("Integration '{}' not installed", id)})),
        );
    }

    match state.kernel.reconnect_extension_mcp(&id).await {
        Ok(tool_count) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "id": id,
                "status": "connected",
                "tool_count": tool_count,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "id": id,
                "status": "error",
                "error": e,
            })),
        ),
    }
}

/// GET /api/integrations/health — Health status for all integrations.
pub async fn integrations_health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let health_entries = state.kernel.extension_health.all_health();
    let entries: Vec<serde_json::Value> = health_entries
        .iter()
        .map(|h| {
            serde_json::json!({
                "id": h.id,
                "status": h.status.to_string(),
                "tool_count": h.tool_count,
                "last_ok": h.last_ok.map(|t| t.to_rfc3339()),
                "last_error": h.last_error,
                "consecutive_failures": h.consecutive_failures,
                "reconnecting": h.reconnecting,
                "reconnect_attempts": h.reconnect_attempts,
                "connected_since": h.connected_since.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Json(serde_json::json!({
        "health": entries,
        "count": entries.len(),
    }))
}

/// POST /api/integrations/reload — Hot-reload integration configs and reconnect MCP.
pub async fn reload_integrations(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.kernel.reload_extension_mcps().await {
        Ok(connected) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "reloaded",
                "new_connections": connected,
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Scheduled Jobs (cron) endpoints
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Agent Identity endpoint
// ---------------------------------------------------------------------------

/// Request body for updating agent visual identity.
#[derive(serde::Deserialize)]
pub struct UpdateIdentityRequest {
    pub emoji: Option<String>,
    pub avatar_url: Option<String>,
    pub color: Option<String>,
    #[serde(default)]
    pub archetype: Option<String>,
    #[serde(default)]
    pub vibe: Option<String>,
    #[serde(default)]
    pub greeting_style: Option<String>,
}

/// PATCH /api/agents/{id}/identity — Update an agent's visual identity.
pub async fn update_agent_identity(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateIdentityRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    // Validate color format if provided
    if let Some(ref color) = req.color {
        if !color.is_empty() && !color.starts_with('#') {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Color must be a hex code starting with '#'"})),
            );
        }
    }

    // Validate avatar_url if provided
    if let Some(ref url) = req.avatar_url {
        if !url.is_empty()
            && !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("data:")
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Avatar URL must be http/https or data URI"})),
            );
        }
    }

    let identity = AgentIdentity {
        emoji: req.emoji,
        avatar_url: req.avatar_url,
        color: req.color,
        archetype: req.archetype,
        vibe: req.vibe,
        greeting_style: req.greeting_style,
    };

    match state.kernel.registry.update_identity(agent_id, identity) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"status": "ok", "agent_id": id})),
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Agent not found"})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Agent Config Hot-Update
// ---------------------------------------------------------------------------

/// Request body for patching agent config (name, description, prompt, identity, model).
#[derive(serde::Deserialize)]
pub struct PatchAgentConfigRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub group: Option<String>,
    pub system_prompt: Option<String>,
    pub emoji: Option<String>,
    pub avatar_url: Option<String>,
    pub color: Option<String>,
    pub archetype: Option<String>,
    pub vibe: Option<String>,
    pub greeting_style: Option<String>,
    /// Sampling temperature (0.0–2.0).
    pub temperature: Option<f32>,
    /// Max output tokens.
    pub max_tokens: Option<u32>,
    /// Enable or disable extended thinking.
    pub thinking_enabled: Option<bool>,
}

/// PATCH /api/agents/{id}/config — Hot-update agent name, description, system prompt, and identity.
pub async fn patch_agent_config(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PatchAgentConfigRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    // Validate color format if provided
    if let Some(ref color) = req.color {
        if !color.is_empty() && !color.starts_with('#') {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Color must be a hex code starting with '#'"})),
            );
        }
    }

    // Validate avatar_url if provided
    if let Some(ref url) = req.avatar_url {
        if !url.is_empty()
            && !url.starts_with("http://")
            && !url.starts_with("https://")
            && !url.starts_with("data:")
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Avatar URL must be http/https or data URI"})),
            );
        }
    }

    let mut did_update = false;

    // Update name
    if let Some(ref new_name) = req.name {
        if !new_name.is_empty() {
            if let Err(e) = state
                .kernel
                .registry
                .update_name(agent_id, new_name.clone())
            {
                return (
                    StatusCode::CONFLICT,
                    Json(serde_json::json!({"error": format!("{e}")})),
                );
            }
            did_update = true;
        }
    }

    // Update description
    if let Some(ref new_desc) = req.description {
        if state
            .kernel
            .registry
            .update_description(agent_id, new_desc.clone())
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Update group
    if req.group.is_some() {
        let normalized_group = match normalize_agent_group(req.group.clone()) {
            Ok(group) => group,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": format!("{e}")})),
                );
            }
        };
        if state
            .kernel
            .registry
            .update_group(agent_id, normalized_group)
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Update system prompt (hot-swap — takes effect on next message)
    if let Some(ref new_prompt) = req.system_prompt {
        if state
            .kernel
            .registry
            .update_system_prompt(agent_id, new_prompt.clone())
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Update identity fields (merge — only overwrite provided fields)
    let has_identity_field = req.emoji.is_some()
        || req.avatar_url.is_some()
        || req.color.is_some()
        || req.archetype.is_some()
        || req.vibe.is_some()
        || req.greeting_style.is_some();

    if has_identity_field {
        // Read current identity, merge with provided fields
        let current = state
            .kernel
            .registry
            .get(agent_id)
            .map(|e| e.identity)
            .unwrap_or_default();
        let merged = AgentIdentity {
            emoji: req.emoji.or(current.emoji),
            avatar_url: req.avatar_url.or(current.avatar_url),
            color: req.color.or(current.color),
            archetype: req.archetype.or(current.archetype),
            vibe: req.vibe.or(current.vibe),
            greeting_style: req.greeting_style.or(current.greeting_style),
        };
        if state
            .kernel
            .registry
            .update_identity(agent_id, merged)
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Temperature
    if let Some(temp) = req.temperature {
        if !(0.0..=2.0).contains(&temp) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "temperature must be 0.0–2.0"})),
            );
        }
        if state
            .kernel
            .registry
            .update_temperature(agent_id, temp)
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Max tokens
    if let Some(max_tok) = req.max_tokens {
        if max_tok == 0 {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "max_tokens must be > 0"})),
            );
        }
        if state
            .kernel
            .registry
            .update_max_tokens(agent_id, max_tok)
            .is_err()
        {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    // Thinking on/off
    if let Some(on) = req.thinking_enabled {
        if state.kernel.set_agent_thinking(agent_id, on).is_err() {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
        did_update = true;
    }

    if did_update {
        if let Some(entry) = state.kernel.registry.get(agent_id) {
            if let Err(e) = state.kernel.memory.save_agent(&entry) {
                tracing::warn!(agent_id = %agent_id, "Failed to persist patched agent config: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Agent updated but could not be persisted"})),
                );
            }
        }
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "ok", "agent_id": id})),
    )
}

// ---------------------------------------------------------------------------
// Agent Cloning
// ---------------------------------------------------------------------------

/// Request body for cloning an agent.
#[derive(serde::Deserialize)]
pub struct CloneAgentRequest {
    pub new_name: String,
}

/// POST /api/agents/{id}/clone — Clone an agent with its workspace files.
pub async fn clone_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CloneAgentRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    if req.new_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "new_name cannot be empty"})),
        );
    }

    let source = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
    };

    // Deep-clone manifest with new name
    let mut cloned_manifest = source.manifest.clone();
    cloned_manifest.name = req.new_name.clone();
    cloned_manifest.workspace = None; // Let kernel assign a new workspace

    // Spawn the cloned agent
    let new_id = match state.kernel.spawn_agent(cloned_manifest) {
        Ok(id) => id,
        Err(e) => {
            return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Agent clone", &e);
        }
    };

    // Copy workspace files from source to destination
    let new_entry = state.kernel.registry.get(new_id);
    if let (Some(ref src_ws), Some(ref new_entry)) = (source.manifest.workspace, new_entry) {
        if let Some(ref dst_ws) = new_entry.manifest.workspace {
            // Security: canonicalize both paths
            if let (Ok(src_can), Ok(dst_can)) = (src_ws.canonicalize(), dst_ws.canonicalize()) {
                for &fname in KNOWN_IDENTITY_FILES {
                    let src_file = src_can.join(fname);
                    let dst_file = dst_can.join(fname);
                    if src_file.exists() {
                        if let Err(e) = std::fs::copy(&src_file, &dst_file) {
                            tracing::warn!(src = %src_file.display(), dst = %dst_file.display(), error = %e, "Failed to copy identity file");
                        }
                    }
                }
            }
        }
    }

    // Copy identity from source
    let _ = state
        .kernel
        .registry
        .update_identity(new_id, source.identity.clone());

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "agent_id": new_id.to_string(),
            "name": req.new_name,
        })),
    )
}

// ---------------------------------------------------------------------------
// Workspace File Editor endpoints
// ---------------------------------------------------------------------------

/// Whitelisted workspace identity files that can be read/written via API.
const KNOWN_IDENTITY_FILES: &[&str] = &[
    "SOUL.md",
    "IDENTITY.md",
    "USER.md",
    "TOOLS.md",
    "MEMORY.md",
    "AGENTS.md",
    "BOOTSTRAP.md",
    "HEARTBEAT.md",
];

/// GET /api/agents/{id}/files — List workspace identity files.
pub async fn list_agent_files(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let agent_id = match parse_agent_id(&id) {
        Ok(id) => id,
        Err(e) => return e,
    };

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => return agent_not_found(&id),
    };

    let workspace = match entry.manifest.workspace {
        Some(ref ws) => ws.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent has no workspace"})),
            );
        }
    };

    let mut files = Vec::new();
    for &name in KNOWN_IDENTITY_FILES {
        let path = workspace.join(name);
        let (exists, size_bytes) = if path.exists() {
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            (true, size)
        } else {
            (false, 0u64)
        };
        files.push(serde_json::json!({
            "name": name,
            "exists": exists,
            "size_bytes": size_bytes,
        }));
    }

    (StatusCode::OK, Json(serde_json::json!({ "files": files })))
}

/// GET /api/agents/{id}/files/{filename} — Read a workspace identity file.
pub async fn get_agent_file(
    State(state): State<Arc<AppState>>,
    Path((id, filename)): Path<(String, String)>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    // Validate filename whitelist
    if !KNOWN_IDENTITY_FILES.contains(&filename.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "File not in whitelist"})),
        );
    }

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
    };

    let workspace = match entry.manifest.workspace {
        Some(ref ws) => ws.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent has no workspace"})),
            );
        }
    };

    // Security: canonicalize and verify stays inside workspace
    let file_path = workspace.join(&filename);
    let canonical = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "File not found"})),
            );
        }
    };
    let ws_canonical = match workspace.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace path error"})),
            );
        }
    };
    if !canonical.starts_with(&ws_canonical) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Path traversal denied"})),
        );
    }

    let content = match std::fs::read_to_string(&canonical) {
        Ok(c) => c,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "File not found"})),
            );
        }
    };

    let size_bytes = content.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "name": filename,
            "content": content,
            "size_bytes": size_bytes,
        })),
    )
}

/// Request body for writing a workspace identity file.
#[derive(serde::Deserialize)]
pub struct SetAgentFileRequest {
    pub content: String,
}

/// PUT /api/agents/{id}/files/{filename} — Write a workspace identity file.
pub async fn set_agent_file(
    State(state): State<Arc<AppState>>,
    Path((id, filename)): Path<(String, String)>,
    Json(req): Json<SetAgentFileRequest>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    // Validate filename whitelist
    if !KNOWN_IDENTITY_FILES.contains(&filename.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "File not in whitelist"})),
        );
    }

    // Max 32KB content
    const MAX_FILE_SIZE: usize = 32_768;
    if req.content.len() > MAX_FILE_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(serde_json::json!({"error": "File content too large (max 32KB)"})),
        );
    }

    let entry = match state.kernel.registry.get(agent_id) {
        Some(e) => e,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent not found"})),
            );
        }
    };

    let workspace = match entry.manifest.workspace {
        Some(ref ws) => ws.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Agent has no workspace"})),
            );
        }
    };

    // Security: verify workspace path and target stays inside it
    let ws_canonical = match workspace.canonicalize() {
        Ok(p) => p,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Workspace path error"})),
            );
        }
    };

    let file_path = workspace.join(&filename);
    // For new files, check the parent directory instead
    let check_path = if file_path.exists() {
        file_path
            .canonicalize()
            .unwrap_or_else(|_| file_path.clone())
    } else {
        // Parent must be inside workspace
        file_path
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .map(|p| p.join(&filename))
            .unwrap_or_else(|| file_path.clone())
    };
    if !check_path.starts_with(&ws_canonical) {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "Path traversal denied"})),
        );
    }

    // Atomic write: write to .tmp, then rename
    let tmp_path = workspace.join(format!(".{filename}.tmp"));
    if let Err(e) = std::fs::write(&tmp_path, &req.content) {
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "File write", &e);
    }
    if let Err(e) = std::fs::rename(&tmp_path, &file_path) {
        let _ = std::fs::remove_file(&tmp_path);
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "File write", &e);
    }

    let size_bytes = req.content.len();
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "name": filename,
            "size_bytes": size_bytes,
        })),
    )
}

// ---------------------------------------------------------------------------
// File Upload endpoints
// ---------------------------------------------------------------------------

/// Metadata stored alongside uploaded files.
struct UploadMeta {
    content_type: String,
}

/// In-memory upload metadata registry.
static UPLOAD_REGISTRY: LazyLock<DashMap<String, UploadMeta>> = LazyLock::new(DashMap::new);

/// Maximum upload size: 10 MB.
const MAX_UPLOAD_SIZE: usize = 10 * 1024 * 1024;

/// Allowed content type prefixes for upload.
const ALLOWED_CONTENT_TYPES: &[&str] = &["image/", "text/", "application/pdf", "audio/"];

fn is_allowed_content_type(ct: &str) -> bool {
    ALLOWED_CONTENT_TYPES
        .iter()
        .any(|prefix| ct.starts_with(prefix))
}

/// POST /api/agents/{id}/upload — Upload a file attachment.
///
/// Accepts raw body bytes. The client must set:
/// - `Content-Type` header (e.g., `image/png`, `text/plain`, `application/pdf`)
/// - `X-Filename` header (original filename)
pub async fn upload_file(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    headers: axum::http::HeaderMap,
    body: axum::body::Bytes,
) -> impl IntoResponse {
    // Validate agent ID format
    let _agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid agent ID"})),
            );
        }
    };

    // Extract content type
    let content_type = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    if !is_allowed_content_type(&content_type) {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                serde_json::json!({"error": "Unsupported content type. Allowed: image/*, text/*, audio/*, application/pdf"}),
            ),
        );
    }

    // Extract filename from header
    let filename = headers
        .get("X-Filename")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("upload")
        .to_string();

    // Validate size
    if body.len() > MAX_UPLOAD_SIZE {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(
                serde_json::json!({"error": format!("File too large (max {} MB)", MAX_UPLOAD_SIZE / (1024 * 1024))}),
            ),
        );
    }

    if body.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Empty file body"})),
        );
    }

    // Generate file ID and save
    let file_id = uuid::Uuid::new_v4().to_string();
    let upload_dir = std::env::temp_dir().join("rusty_hand_uploads");
    if let Err(e) = std::fs::create_dir_all(&upload_dir) {
        tracing::warn!("Failed to create upload dir: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to create upload directory"})),
        );
    }

    let file_path = upload_dir.join(&file_id);
    if let Err(e) = std::fs::write(&file_path, &body) {
        tracing::warn!("Failed to write upload: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": "Failed to save file"})),
        );
    }

    let size = body.len();
    UPLOAD_REGISTRY.insert(
        file_id.clone(),
        UploadMeta {
            content_type: content_type.clone(),
        },
    );

    // Auto-ingest text documents into agent memory (RAG pipeline)
    let ingested = if content_type.starts_with("text/") || content_type == "application/pdf" {
        let text_content = if content_type == "application/pdf" {
            // PDF: try to extract text (best-effort, use raw bytes as fallback)
            String::from_utf8_lossy(&body).to_string()
        } else {
            String::from_utf8_lossy(&body).to_string()
        };

        if !text_content.trim().is_empty() {
            let emb_driver = state.kernel.embedding_driver.as_deref();
            match rusty_hand_runtime::ingest::ingest_text(
                _agent_id,
                &text_content,
                &filename,
                &state.kernel.memory,
                emb_driver,
                None,
                None,
            )
            .await
            {
                Ok(result) => {
                    tracing::info!(
                        agent = %_agent_id,
                        chunks = result.chunks,
                        embedded = result.embedded,
                        source = %filename,
                        "Document ingested into agent memory"
                    );
                    Some(serde_json::json!({
                        "chunks": result.chunks,
                        "embedded": result.embedded,
                        "source": result.source,
                    }))
                }
                Err(e) => {
                    tracing::warn!("Document ingestion failed: {e}");
                    None
                }
            }
        } else {
            None
        }
    } else {
        None
    };

    // Auto-transcribe audio uploads using the media engine
    let transcription = if content_type.starts_with("audio/") {
        let attachment = rusty_hand_types::media::MediaAttachment {
            media_type: rusty_hand_types::media::MediaType::Audio,
            mime_type: content_type.clone(),
            source: rusty_hand_types::media::MediaSource::FilePath {
                path: file_path.to_string_lossy().to_string(),
            },
            size_bytes: size as u64,
        };
        match state
            .kernel
            .media_engine
            .transcribe_audio(&attachment)
            .await
        {
            Ok(result) => {
                tracing::info!(chars = result.description.len(), provider = %result.provider, "Audio transcribed");
                Some(result.description)
            }
            Err(e) => {
                tracing::warn!("Audio transcription failed: {e}");
                None
            }
        }
    } else {
        None
    };

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "file_id": file_id,
            "filename": filename,
            "content_type": content_type,
            "size": size,
            "transcription": transcription,
            "ingested": ingested,
        })),
    )
}

/// GET /api/uploads/{file_id} — Serve an uploaded file.
pub async fn serve_upload(Path(file_id): Path<String>) -> impl IntoResponse {
    // Validate file_id is a UUID to prevent path traversal
    if uuid::Uuid::parse_str(&file_id).is_err() {
        return (
            StatusCode::BAD_REQUEST,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json".to_string(),
            )],
            b"{\"error\":\"Invalid file ID\"}".to_vec(),
        );
    }

    let file_path = std::env::temp_dir()
        .join("rusty_hand_uploads")
        .join(&file_id);

    // Look up metadata from registry; fall back to disk probe for generated images
    // (image_generate saves files without registering in UPLOAD_REGISTRY).
    let content_type = match UPLOAD_REGISTRY.get(&file_id) {
        Some(m) => m.content_type.clone(),
        None => {
            // Infer content type from file magic bytes
            if !file_path.exists() {
                return (
                    StatusCode::NOT_FOUND,
                    [(
                        axum::http::header::CONTENT_TYPE,
                        "application/json".to_string(),
                    )],
                    b"{\"error\":\"File not found\"}".to_vec(),
                );
            }
            "image/png".to_string()
        }
    };

    match std::fs::read(&file_path) {
        Ok(data) => (
            StatusCode::OK,
            [(axum::http::header::CONTENT_TYPE, content_type)],
            data,
        ),
        Err(_) => (
            StatusCode::NOT_FOUND,
            [(
                axum::http::header::CONTENT_TYPE,
                "application/json".to_string(),
            )],
            b"{\"error\":\"File not found on disk\"}".to_vec(),
        ),
    }
}

// ---------------------------------------------------------------------------
// Execution Approval System — backed by kernel.approval_manager
// ---------------------------------------------------------------------------

/// GET /api/approvals — List pending approval requests.
pub async fn list_approvals(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let pending = state.kernel.approval_manager.list_pending();
    let total = pending.len();
    Json(serde_json::json!({"approvals": pending, "total": total}))
}

/// POST /api/approvals — Create a manual approval request (for external systems).
///
/// Note: Most approval requests are created automatically by the tool_runner
/// when an agent invokes a tool that requires approval. This endpoint exists
/// for external integrations that need to inject approval gates.
#[derive(serde::Deserialize)]
pub struct CreateApprovalRequest {
    pub agent_id: String,
    pub tool_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub action_summary: String,
}

pub async fn create_approval(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateApprovalRequest>,
) -> impl IntoResponse {
    use rusty_hand_types::approval::{ApprovalRequest, RiskLevel};

    let policy = state.kernel.approval_manager.policy();
    let id = uuid::Uuid::new_v4();
    let approval_req = ApprovalRequest {
        id,
        agent_id: req.agent_id,
        tool_name: req.tool_name.clone(),
        description: if req.description.is_empty() {
            format!("Manual approval request for {}", req.tool_name)
        } else {
            req.description
        },
        action_summary: if req.action_summary.is_empty() {
            req.tool_name.clone()
        } else {
            req.action_summary
        },
        risk_level: RiskLevel::High,
        requested_at: chrono::Utc::now(),
        timeout_secs: policy.timeout_secs,
    };

    // Spawn the request in the background (it will block until resolved or timed out)
    let kernel = Arc::clone(&state.kernel);
    let req_id = id;
    tokio::spawn(async move {
        let decision = kernel.approval_manager.request_approval(approval_req).await;
        tracing::info!(
            request_id = %req_id,
            ?decision,
            "Approval request resolved in background"
        );
    });

    (
        StatusCode::CREATED,
        Json(serde_json::json!({"id": id.to_string(), "status": "pending"})),
    )
}

/// POST /api/approvals/{id}/approve — Approve a pending request.
pub async fn approve_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid approval ID"})),
            );
        }
    };

    match state.kernel.approval_manager.resolve(
        uuid,
        rusty_hand_types::approval::ApprovalDecision::Approved,
        Some("api".to_string()),
    ) {
        Ok(resp) => (
            StatusCode::OK,
            Json(
                serde_json::json!({"id": id, "status": "approved", "decided_at": resp.decided_at.to_rfc3339()}),
            ),
        ),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))),
    }
}

/// POST /api/approvals/{id}/reject — Reject a pending request.
pub async fn reject_request(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let uuid = match uuid::Uuid::parse_str(&id) {
        Ok(u) => u,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "Invalid approval ID"})),
            );
        }
    };

    match state.kernel.approval_manager.resolve(
        uuid,
        rusty_hand_types::approval::ApprovalDecision::Denied,
        Some("api".to_string()),
    ) {
        Ok(resp) => (
            StatusCode::OK,
            Json(
                serde_json::json!({"id": id, "status": "rejected", "decided_at": resp.decided_at.to_rfc3339()}),
            ),
        ),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))),
    }
}

// ---------------------------------------------------------------------------
// Config Reload endpoint
// ---------------------------------------------------------------------------

/// POST /api/config/reload — Reload configuration from disk and apply hot-reloadable changes.
///
/// Reads the config file, diffs against current config, validates the new config,
/// and applies hot-reloadable actions (approval policy, cron limits, etc.).
/// Returns the reload plan showing what changed and what was applied.
pub async fn config_reload(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<crate::middleware::AuthenticatedUser>>,
) -> impl IntoResponse {
    let actor = actor
        .map(|Extension(user)| user.name)
        .unwrap_or_else(|| "system".to_string());
    // SECURITY: Record config reload in audit trail
    state.kernel.audit_log.record(
        actor,
        rusty_hand_runtime::audit::AuditAction::ConfigChange,
        "config reload requested via API",
        "pending",
    );
    match state.kernel.reload_config() {
        Ok(plan) => {
            let status = if plan.restart_required {
                "partial"
            } else if plan.has_changes() {
                "applied"
            } else {
                "no_changes"
            };

            (
                StatusCode::OK,
                Json(serde_json::json!({
                    "status": status,
                    "restart_required": plan.restart_required,
                    "restart_reasons": plan.restart_reasons,
                    "hot_actions_applied": plan.hot_actions.iter().map(|a| format!("{a:?}")).collect::<Vec<_>>(),
                    "noop_changes": plan.noop_changes,
                })),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status": "error", "error": e})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Config Schema endpoint
// ---------------------------------------------------------------------------

/// GET /api/config/schema — Return a simplified JSON description of the config structure.
pub async fn config_schema() -> impl IntoResponse {
    Json(serde_json::json!({
        "sections": {
            "api": {
                "fields": {
                    "api_listen": "string",
                    "api_key": "string",
                    "log_level": "string"
                }
            },
            "default_model": {
                "fields": {
                    "provider": "string",
                    "model": "string",
                    "api_key_env": "string",
                    "base_url": "string"
                }
            },
            "memory": {
                "fields": {
                    "decay_rate": "number",
                    "vector_dims": "number"
                }
            },
            "web": {
                "fields": {
                    "provider": "string",
                    "timeout_secs": "number",
                    "max_results": "number"
                }
            },
            "browser": {
                "fields": {
                    "headless": "boolean",
                    "timeout_secs": "number",
                    "executable_path": "string"
                }
            },
            "network": {
                "fields": {
                    "enabled": "boolean",
                    "listen_addr": "string",
                    "shared_secret": "string"
                }
            },
            "extensions": {
                "fields": {
                    "auto_connect": "boolean",
                    "health_check_interval_secs": "number"
                }
            },
            "vault": {
                "fields": {
                    "path": "string"
                }
            },
            "a2a": {
                "fields": {
                    "enabled": "boolean",
                    "name": "string",
                    "description": "string",
                    "url": "string"
                }
            },
            "channels": {
                "fields": {
                    "telegram": "object",
                    "discord": "object",
                    "slack": "object"
                }
            },
            "proxy": {
                "fields": {
                    "url": "string",
                    "username": "string",
                    "password": "string",
                    "no_proxy": "array"
                },
                "description": "HTTP/HTTPS/SOCKS5 proxy for outbound requests (Bright Data, residential proxies). Password is write-only and never returned in GET /api/config."
            },
            "mcp_server": {
                "fields": {
                    "enabled": "boolean",
                    "extra_allowed_tools": "array",
                    "allow_all_tools": "boolean"
                },
                "description": "Controls which builtin tools remote MCP clients can invoke via POST /mcp. Defaults to safe read-only tools. Add privileged tools (shell_exec, skill_install, file_write) to extra_allowed_tools only if needed."
            }
        }
    }))
}

// ---------------------------------------------------------------------------
// Config Set endpoint
// ---------------------------------------------------------------------------

/// POST /api/config/set — Set a single config value and persist to config.toml.
///
/// Accepts JSON `{ "path": "section.key", "value": "..." }`.
/// Writes the value to the TOML config file and triggers a reload.
pub async fn config_set(
    State(state): State<Arc<AppState>>,
    actor: Option<Extension<crate::middleware::AuthenticatedUser>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let actor = actor
        .map(|Extension(user)| user.name)
        .unwrap_or_else(|| "system".to_string());
    let path = match body.get("path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"status": "error", "error": "missing 'path' field"})),
            );
        }
    };
    let value = match body.get("value") {
        Some(v) => v.clone(),
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"status": "error", "error": "missing 'value' field"})),
            );
        }
    };

    let config_path = state.kernel.config.home_dir.join("config.toml");

    // Read existing config as a TOML table, or start fresh
    let mut table: toml::value::Table = if config_path.exists() {
        match std::fs::read_to_string(&config_path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => toml::value::Table::new(),
        }
    } else {
        toml::value::Table::new()
    };

    // Convert JSON value to TOML value
    let toml_val = json_to_toml_value(&value);

    // Parse "section.key" path and set value
    let parts: Vec<&str> = path.split('.').collect();
    match parts.len() {
        1 => {
            table.insert(parts[0].to_string(), toml_val);
        }
        2 => {
            let section = table
                .entry(parts[0].to_string())
                .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
            if let toml::Value::Table(ref mut t) = section {
                t.insert(parts[1].to_string(), toml_val);
            }
        }
        3 => {
            let section = table
                .entry(parts[0].to_string())
                .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
            if let toml::Value::Table(ref mut t) = section {
                let sub = t
                    .entry(parts[1].to_string())
                    .or_insert_with(|| toml::Value::Table(toml::value::Table::new()));
                if let toml::Value::Table(ref mut t2) = sub {
                    t2.insert(parts[2].to_string(), toml_val);
                }
            }
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    serde_json::json!({"status": "error", "error": "path too deep (max 3 levels)"}),
                ),
            );
        }
    }

    // Write back
    let toml_string = match toml::to_string_pretty(&table) {
        Ok(s) => s,
        Err(e) => {
            return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Config save", &e);
        }
    };
    if let Err(e) = std::fs::write(&config_path, &toml_string) {
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Config save", &e);
    }

    // Trigger reload
    let reload_status = match state.kernel.reload_config() {
        Ok(plan) => {
            if plan.restart_required {
                "applied_partial"
            } else {
                "applied"
            }
        }
        Err(_) => "saved_reload_failed",
    };

    state.kernel.audit_log.record(
        actor,
        rusty_hand_runtime::audit::AuditAction::ConfigChange,
        format!("config set: {path}"),
        "completed",
    );

    (
        StatusCode::OK,
        Json(serde_json::json!({"status": reload_status, "path": path})),
    )
}

/// Convert a serde_json::Value to a toml::Value.
fn json_to_toml_value(value: &serde_json::Value) -> toml::Value {
    match value {
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        _ => toml::Value::String(value.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Delivery tracking endpoints
// ---------------------------------------------------------------------------

/// GET /api/agents/:id/deliveries — List recent delivery receipts for an agent.
pub async fn get_agent_deliveries(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let agent_id: AgentId = match id.parse() {
        Ok(id) => id,
        Err(_) => {
            // Try name lookup
            match state.kernel.registry.find_by_name(&id) {
                Some(entry) => entry.id,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "Agent not found"})),
                    );
                }
            }
        }
    };

    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(50)
        .min(500);

    let receipts = state.kernel.delivery_tracker.get_receipts(agent_id, limit);
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "agent_id": agent_id.to_string(),
            "count": receipts.len(),
            "receipts": receipts,
        })),
    )
}

// ---------------------------------------------------------------------------
// Cron job management endpoints
// ---------------------------------------------------------------------------

/// GET /api/cron/jobs — List all cron jobs, optionally filtered by agent_id.
pub async fn list_cron_jobs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let metas = if let Some(agent_id_str) = params.get("agent_id") {
        match uuid::Uuid::parse_str(agent_id_str) {
            Ok(uuid) => {
                let aid = AgentId(uuid);
                state.kernel.cron_scheduler.list_metas(aid)
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error": "Invalid agent_id"})),
                );
            }
        }
    } else {
        state.kernel.cron_scheduler.list_all_metas()
    };
    let total = metas.len();
    let jobs_json: Vec<serde_json::Value> = metas
        .into_iter()
        .map(|meta| {
            serde_json::json!({
                "id": meta.job.id.to_string(),
                "agent_id": meta.job.agent_id.to_string(),
                "name": meta.job.name,
                "enabled": meta.job.enabled,
                "schedule": meta.job.schedule,
                "action": meta.job.action,
                "delivery": meta.job.delivery,
                "created_at": meta.job.created_at,
                "last_run": meta.job.last_run,
                "next_run": meta.job.next_run,
                "one_shot": meta.one_shot,
                "last_status": meta.last_status,
                "consecutive_errors": meta.consecutive_errors,
            })
        })
        .collect();
    (
        StatusCode::OK,
        Json(serde_json::json!({"jobs": jobs_json, "total": total})),
    )
}

/// POST /api/cron/jobs — Create a new cron job.
pub async fn create_cron_job(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let agent_id = body["agent_id"].as_str().unwrap_or("");
    match state.kernel.cron_create(agent_id, body.clone()).await {
        Ok(result) => (
            StatusCode::CREATED,
            Json(serde_json::json!({"result": result})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// DELETE /api/cron/jobs/{id} — Delete a cron job.
pub async fn delete_cron_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            let job_id = rusty_hand_types::scheduler::CronJobId(uuid);
            match state.kernel.cron_scheduler.remove_job(job_id) {
                Ok(_) => {
                    if let Err(e) = state.kernel.cron_scheduler.persist() {
                        tracing::warn!(error = %e, "Failed to persist cron schedule");
                    }
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({"status": "deleted"})),
                    )
                }
                Err(e) => safe_error(StatusCode::NOT_FOUND, "Cron job deletion", &e),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid job ID"})),
        ),
    }
}

/// PUT /api/cron/jobs/{id} — Update an existing cron job (preserves ID).
pub async fn update_cron_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    match state.kernel.cron_update(&id, body.clone()).await {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!({"result": result}))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

/// PUT /api/cron/jobs/{id}/enable — Enable or disable a cron job.
pub async fn toggle_cron_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let enabled = body["enabled"].as_bool().unwrap_or(true);
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            let job_id = rusty_hand_types::scheduler::CronJobId(uuid);
            match state.kernel.cron_scheduler.set_enabled(job_id, enabled) {
                Ok(()) => {
                    if let Err(e) = state.kernel.cron_scheduler.persist() {
                        tracing::warn!(error = %e, "Failed to persist cron schedule");
                    }
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({"id": id, "enabled": enabled})),
                    )
                }
                Err(e) => safe_error(StatusCode::NOT_FOUND, "Cron job toggle", &e),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid job ID"})),
        ),
    }
}

/// POST /api/cron/jobs/{id}/run — Manually run a cron job now.
pub async fn run_cron_job(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            let job_id = rusty_hand_types::scheduler::CronJobId(uuid);
            let job = match state.kernel.cron_scheduler.get_meta(job_id) {
                Some(meta) => meta.job,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "Job not found"})),
                    );
                }
            };

            let result = state.kernel.execute_cron_job(&job).await;
            if let Err(e) = state.kernel.cron_scheduler.persist() {
                tracing::warn!(error = %e, "Failed to persist cron schedule");
            }

            match result {
                Ok(rusty_hand_kernel::kernel::CronRunOutcome::SystemEvent { event_type }) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "completed",
                        "job_id": id,
                        "mode": "system_event",
                        "event_type": event_type,
                    })),
                ),
                Ok(rusty_hand_kernel::kernel::CronRunOutcome::AgentTurn { agent_id, response }) => {
                    (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "status": "completed",
                            "job_id": id,
                            "agent_id": agent_id.to_string(),
                            "response": response,
                        })),
                    )
                }
                Ok(rusty_hand_kernel::kernel::CronRunOutcome::WorkflowRun {
                    workflow_id,
                    output,
                }) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "status": "completed",
                        "job_id": id,
                        "mode": "workflow_run",
                        "workflow_id": workflow_id,
                        "output": output,
                    })),
                ),
                Err(error) => (
                    if error.timed_out {
                        StatusCode::GATEWAY_TIMEOUT
                    } else {
                        StatusCode::INTERNAL_SERVER_ERROR
                    },
                    Json(serde_json::json!({
                        "status": "failed",
                        "job_id": id,
                        "error": error.message,
                    })),
                ),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid job ID"})),
        ),
    }
}

/// GET /api/cron/jobs/{id}/status — Get status of a specific cron job.
pub async fn cron_job_status(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match uuid::Uuid::parse_str(&id) {
        Ok(uuid) => {
            let job_id = rusty_hand_types::scheduler::CronJobId(uuid);
            match state.kernel.cron_scheduler.get_meta(job_id) {
                Some(meta) => (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "id": meta.job.id.to_string(),
                        "agent_id": meta.job.agent_id.to_string(),
                        "name": meta.job.name,
                        "enabled": meta.job.enabled,
                        "schedule": meta.job.schedule,
                        "action": meta.job.action,
                        "delivery": meta.job.delivery,
                        "created_at": meta.job.created_at,
                        "last_run": meta.job.last_run,
                        "next_run": meta.job.next_run,
                        "one_shot": meta.one_shot,
                        "last_status": meta.last_status,
                        "consecutive_errors": meta.consecutive_errors,
                    })),
                ),
                None => (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "Job not found"})),
                ),
            }
        }
        Err(_) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "Invalid job ID"})),
        ),
    }
}

// ---------------------------------------------------------------------------
// Webhook trigger endpoints
// ---------------------------------------------------------------------------

/// POST /hooks/wake — Inject a system event via webhook trigger.
///
/// Publishes a custom event through the kernel's event system, which can
/// trigger proactive agents that subscribe to the event type.
pub async fn webhook_wake(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<rusty_hand_types::webhook::WakePayload>,
) -> impl IntoResponse {
    // Check if webhook triggers are enabled
    let wh_config = match &state.kernel.config.webhook_triggers {
        Some(c) if c.enabled => c,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Webhook triggers not enabled"})),
            );
        }
    };

    // Validate bearer token (constant-time comparison)
    if !validate_webhook_token(&headers, &wh_config.token_env) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid or missing token"})),
        );
    }

    // Validate payload
    if let Err(e) = body.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        );
    }

    // Publish through the kernel's publish_event (KernelHandle trait), which
    // goes through the full event processing pipeline including trigger evaluation.
    let event_payload = serde_json::json!({
        "source": "webhook",
        "mode": body.mode,
        "text": body.text,
    });
    if let Err(e) =
        KernelHandle::publish_event(state.kernel.as_ref(), "webhook.wake", event_payload).await
    {
        tracing::warn!("Webhook wake event publish failed: {e}");
        return safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Webhook event", &e);
    }

    (
        StatusCode::OK,
        Json(serde_json::json!({"status": "accepted", "mode": body.mode})),
    )
}

/// POST /hooks/agent — Run an isolated agent turn via webhook.
///
/// Sends a message directly to the specified agent and returns the response.
/// This enables external systems (CI/CD, Slack, etc.) to trigger agent work.
pub async fn webhook_agent(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(body): Json<rusty_hand_types::webhook::AgentHookPayload>,
) -> impl IntoResponse {
    // Check if webhook triggers are enabled
    let wh_config = match &state.kernel.config.webhook_triggers {
        Some(c) if c.enabled => c,
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({"error": "Webhook triggers not enabled"})),
            );
        }
    };

    // Validate bearer token
    if !validate_webhook_token(&headers, &wh_config.token_env) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "Invalid or missing token"})),
        );
    }

    // Validate payload
    if let Err(e) = body.validate() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        );
    }

    // Resolve the agent by name or ID (if not specified, use the first running agent)
    let agent_id: AgentId = match &body.agent {
        Some(agent_ref) => match agent_ref.parse() {
            Ok(id) => id,
            Err(_) => {
                // Try name lookup
                match state.kernel.registry.find_by_name(agent_ref) {
                    Some(entry) => entry.id,
                    None => {
                        return (
                            StatusCode::NOT_FOUND,
                            Json(
                                serde_json::json!({"error": format!("Agent not found: {}", agent_ref)}),
                            ),
                        );
                    }
                }
            }
        },
        None => {
            // No agent specified — use the first available agent
            match state.kernel.registry.list().first() {
                Some(entry) => entry.id,
                None => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(serde_json::json!({"error": "No agents available"})),
                    );
                }
            }
        }
    };

    // Actually send the message to the agent and get the response
    match state.kernel.send_message(agent_id, &body.message).await {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "status": "completed",
                "agent_id": agent_id.to_string(),
                "response": result.response,
                "usage": {
                    "input_tokens": result.total_usage.input_tokens,
                    "output_tokens": result.total_usage.output_tokens,
                },
            })),
        ),
        Err(e) => safe_error(StatusCode::INTERNAL_SERVER_ERROR, "Agent execution", &e),
    }
}

// ─── Agent Bindings API ────────────────────────────────────────────────

/// GET /api/bindings — List all agent bindings.
pub async fn list_bindings(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let bindings = state.kernel.list_bindings();
    (
        StatusCode::OK,
        Json(serde_json::json!({ "bindings": bindings })),
    )
}

/// POST /api/bindings — Add a new agent binding.
pub async fn add_binding(
    State(state): State<Arc<AppState>>,
    Json(binding): Json<rusty_hand_types::config::AgentBinding>,
) -> impl IntoResponse {
    // Validate agent exists
    let agents = state.kernel.registry.list();
    let agent_exists = agents.iter().any(|e| e.name == binding.agent)
        || binding.agent.parse::<uuid::Uuid>().is_ok();
    if !agent_exists {
        tracing::warn!(agent = %binding.agent, "Binding references unknown agent");
    }

    state.kernel.add_binding(binding);
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "status": "created" })),
    )
}

/// DELETE /api/bindings/:index — Remove a binding by index.
pub async fn remove_binding(
    State(state): State<Arc<AppState>>,
    Path(index): Path<usize>,
) -> impl IntoResponse {
    match state.kernel.remove_binding(index) {
        Some(_) => (
            StatusCode::OK,
            Json(serde_json::json!({ "status": "removed" })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "Binding index out of range" })),
        ),
    }
}

// ─── Device Pairing endpoints ───────────────────────────────────────────

/// POST /api/pairing/request — Create a new pairing request (returns token + QR URI).
pub async fn pairing_request(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.kernel.config.pairing.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Pairing not enabled"})),
        )
            .into_response();
    }
    match state.kernel.pairing.create_pairing_request() {
        Ok(req) => {
            let qr_uri = format!("rustyhand://pair?token={}", req.token);
            Json(serde_json::json!({
                "token": req.token,
                "qr_uri": qr_uri,
                "expires_at": req.expires_at.to_rfc3339(),
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

/// POST /api/pairing/complete — Complete pairing with token + device info.
pub async fn pairing_complete(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !state.kernel.config.pairing.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Pairing not enabled"})),
        )
            .into_response();
    }
    let token = body.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let display_name = body
        .get("display_name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let platform = body
        .get("platform")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let push_token = body
        .get("push_token")
        .and_then(|v| v.as_str())
        .map(String::from);
    let device_info = rusty_hand_kernel::pairing::PairedDevice {
        device_id: uuid::Uuid::new_v4().to_string(),
        display_name: display_name.to_string(),
        platform: platform.to_string(),
        paired_at: chrono::Utc::now(),
        last_seen: chrono::Utc::now(),
        push_token,
    };
    match state.kernel.pairing.complete_pairing(token, device_info) {
        Ok(device) => Json(serde_json::json!({
            "device_id": device.device_id,
            "display_name": device.display_name,
            "platform": device.platform,
            "paired_at": device.paired_at.to_rfc3339(),
        }))
        .into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": e})),
        )
            .into_response(),
    }
}

/// GET /api/pairing/devices — List paired devices.
pub async fn pairing_devices(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if !state.kernel.config.pairing.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Pairing not enabled"})),
        )
            .into_response();
    }
    let devices: Vec<_> = state
        .kernel
        .pairing
        .list_devices()
        .into_iter()
        .map(|d| {
            serde_json::json!({
                "device_id": d.device_id,
                "display_name": d.display_name,
                "platform": d.platform,
                "paired_at": d.paired_at.to_rfc3339(),
                "last_seen": d.last_seen.to_rfc3339(),
            })
        })
        .collect();
    Json(serde_json::json!({"devices": devices})).into_response()
}

/// DELETE /api/pairing/devices/{id} — Remove a paired device.
pub async fn pairing_remove_device(
    State(state): State<Arc<AppState>>,
    Path(device_id): Path<String>,
) -> impl IntoResponse {
    if !state.kernel.config.pairing.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Pairing not enabled"})),
        )
            .into_response();
    }
    match state.kernel.pairing.remove_device(&device_id) {
        Ok(()) => Json(serde_json::json!({"ok": true})).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": e}))).into_response(),
    }
}

/// POST /api/pairing/notify — Push a notification to all paired devices.
pub async fn pairing_notify(
    State(state): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    if !state.kernel.config.pairing.enabled {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "Pairing not enabled"})),
        )
            .into_response();
    }
    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Rusty Hand");
    let message = body.get("message").and_then(|v| v.as_str()).unwrap_or("");
    if message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "message is required"})),
        )
            .into_response();
    }
    state.kernel.pairing.notify_devices(title, message).await;
    Json(serde_json::json!({"ok": true, "notified": state.kernel.pairing.list_devices().len()}))
        .into_response()
}

/// GET /api/commands — List available chat commands (for dynamic slash menu).
pub async fn list_commands(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut commands = vec![
        serde_json::json!({"cmd": "/help", "desc": "Show available commands"}),
        serde_json::json!({"cmd": "/new", "desc": "Reset session (clear history)"}),
        serde_json::json!({"cmd": "/compact", "desc": "Trigger LLM session compaction"}),
        serde_json::json!({"cmd": "/model", "desc": "Show or switch model (/model [name])"}),
        serde_json::json!({"cmd": "/stop", "desc": "Cancel current agent run"}),
        serde_json::json!({"cmd": "/usage", "desc": "Show session token usage & cost"}),
        serde_json::json!({"cmd": "/think", "desc": "Toggle extended thinking (/think [on|off|stream])"}),
        serde_json::json!({"cmd": "/context", "desc": "Show context window usage & pressure"}),
        serde_json::json!({"cmd": "/verbose", "desc": "Cycle tool detail level (/verbose [off|on|full])"}),
        serde_json::json!({"cmd": "/queue", "desc": "Check if agent is processing"}),
        serde_json::json!({"cmd": "/status", "desc": "Show system status"}),
        serde_json::json!({"cmd": "/clear", "desc": "Clear chat display"}),
        serde_json::json!({"cmd": "/exit", "desc": "Disconnect from agent"}),
    ];

    // Add skill-registered tool names as potential commands
    if let Ok(registry) = state.kernel.skill_registry.read() {
        for skill in registry.list() {
            let desc: String = skill.manifest.skill.description.chars().take(80).collect();
            commands.push(serde_json::json!({
                "cmd": format!("/{}", skill.manifest.skill.name),
                "desc": if desc.is_empty() { format!("Skill: {}", skill.manifest.skill.name) } else { desc },
                "source": "skill",
            }));
        }
    }

    Json(serde_json::json!({"commands": commands}))
}

/// SECURITY: Validate webhook bearer token using constant-time comparison.
fn validate_webhook_token(headers: &axum::http::HeaderMap, token_env: &str) -> bool {
    let expected = match std::env::var(token_env) {
        Ok(t) if t.len() >= 32 => t,
        _ => return false,
    };

    let provided = match headers.get("authorization") {
        Some(v) => match v.to_str() {
            Ok(s) if s.starts_with("Bearer ") => &s[7..],
            _ => return false,
        },
        None => return false,
    };

    use subtle::ConstantTimeEq;
    if provided.len() != expected.len() {
        return false;
    }
    provided.as_bytes().ct_eq(expected.as_bytes()).into()
}
