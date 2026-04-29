//! Channel bridge — connects channel adapters to the RustyHand kernel.
//!
//! Defines `ChannelBridgeHandle` (implemented by rusty-hand-api on the kernel) and
//! `BridgeManager` which owns running adapters and dispatches messages.

use crate::formatter;
use crate::router::AgentRouter;
use crate::types::{ChannelAdapter, ChannelContent, ChannelMessage, ChannelUser};
use async_trait::async_trait;
use dashmap::DashMap;
use futures::StreamExt;
use rusty_hand_types::agent::AgentId;
use rusty_hand_types::config::{ChannelOverrides, DmPolicy, GroupPolicy, OutputFormat};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::watch;
use tracing::{debug, error, info, warn};

/// Kernel operations needed by channel adapters.
///
/// Defined here to avoid circular deps (rusty-hand-channels can't depend on rusty-hand-kernel).
/// Implemented in rusty-hand-api on the actual kernel.
#[async_trait]
pub trait ChannelBridgeHandle: Send + Sync {
    /// Send a message to an agent and get the text response.
    async fn send_message(&self, agent_id: AgentId, message: &str) -> Result<String, String>;

    /// Send a message and receive streaming chunks via callback.
    /// Default implementation falls back to non-streaming send_message.
    async fn send_message_streaming(
        &self,
        agent_id: AgentId,
        message: &str,
        _on_chunk: Box<dyn Fn(&str) + Send + Sync>,
    ) -> Result<String, String> {
        self.send_message(agent_id, message).await
    }

    /// Send a message and pipe streaming text chunks through a channel.
    ///
    /// Unlike `send_message_streaming` (callback-based), this uses an owned
    /// `Sender<String>` which avoids async_trait lifetime issues with `Fn(&str)`.
    /// Default: falls back to `send_message()` and sends the full response as one chunk.
    async fn send_message_to_stream(
        &self,
        agent_id: AgentId,
        message: &str,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, String> {
        let response = self.send_message(agent_id, message).await?;
        let _ = tx.send(response.clone()).await;
        Ok(response)
    }

    /// Find an agent by name, returning its ID.
    async fn find_agent_by_name(&self, name: &str) -> Result<Option<AgentId>, String>;

    /// List running agents as (id, name) pairs.
    async fn list_agents(&self) -> Result<Vec<(AgentId, String)>, String>;

    /// Spawn an agent by manifest name, returning its ID.
    async fn spawn_agent_by_name(&self, manifest_name: &str) -> Result<AgentId, String>;

    /// Return uptime info string (e.g., "2h 15m, 5 agents").
    async fn uptime_info(&self) -> String {
        let agents = self.list_agents().await.unwrap_or_default();
        format!("{} agent(s) running", agents.len())
    }

    /// List available models as formatted text for channel display.
    async fn list_models_text(&self) -> String {
        "Model listing not available.".to_string()
    }

    /// List providers and their auth status as formatted text for channel display.
    async fn list_providers_text(&self) -> String {
        "Provider listing not available.".to_string()
    }

    /// Reset an agent's session (clear messages, fresh session ID).
    async fn reset_session(&self, _agent_id: AgentId) -> Result<String, String> {
        Err("Not implemented".to_string())
    }

    /// Trigger LLM-based session compaction for an agent.
    async fn compact_session(&self, _agent_id: AgentId) -> Result<String, String> {
        Err("Not implemented".to_string())
    }

    /// Set an agent's model.
    async fn set_model(&self, _agent_id: AgentId, _model: &str) -> Result<String, String> {
        Err("Not implemented".to_string())
    }

    /// Stop an agent's current LLM run.
    async fn stop_run(&self, _agent_id: AgentId) -> Result<String, String> {
        Err("Not implemented".to_string())
    }

    /// Get session token usage and estimated cost.
    async fn session_usage(&self, _agent_id: AgentId) -> Result<String, String> {
        Err("Not implemented".to_string())
    }

    /// Toggle extended thinking mode for an agent.
    async fn set_thinking(&self, _agent_id: AgentId, _on: bool) -> Result<String, String> {
        Ok("Extended thinking preference saved.".to_string())
    }

    /// List installed skills as formatted text for channel display.
    async fn list_skills_text(&self) -> String {
        "Skill listing not available.".to_string()
    }

    /// Authorize a channel user for an action.
    ///
    /// Returns Ok(()) if the user is allowed, Err(reason) if denied.
    /// Default implementation: allow all (RBAC disabled).
    async fn authorize_channel_user(
        &self,
        _channel_type: &str,
        _platform_id: &str,
        _action: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    /// Get per-channel overrides for a given channel type.
    ///
    /// Returns `None` if the channel is not configured or has no overrides.
    async fn channel_overrides(&self, _channel_type: &str) -> Option<ChannelOverrides> {
        None
    }

    /// Record a delivery result for tracking (optional — default no-op).
    async fn record_delivery(
        &self,
        _agent_id: AgentId,
        _channel: &str,
        _recipient: &str,
        _success: bool,
        _error: Option<&str>,
    ) {
        // Default: no tracking
    }

    /// Check if auto-reply is enabled and the message should trigger one.
    /// Returns Some(reply_text) if auto-reply fires, None otherwise.
    async fn check_auto_reply(&self, _agent_id: AgentId, _message: &str) -> Option<String> {
        None
    }

    // ── Automation: workflows, triggers, schedules, approvals ──

    /// List all registered workflows as formatted text.
    async fn list_workflows_text(&self) -> String {
        "Workflows not available.".to_string()
    }

    /// Run a workflow by name with the given input text.
    async fn run_workflow_text(&self, _name: &str, _input: &str) -> String {
        "Workflows not available.".to_string()
    }

    /// List all registered triggers as formatted text.
    async fn list_triggers_text(&self) -> String {
        "Triggers not available.".to_string()
    }

    /// Create a trigger for an agent with the given pattern and prompt.
    async fn create_trigger_text(
        &self,
        _agent_name: &str,
        _pattern: &str,
        _prompt: &str,
    ) -> String {
        "Triggers not available.".to_string()
    }

    /// Delete a trigger by UUID prefix.
    async fn delete_trigger_text(&self, _id_prefix: &str) -> String {
        "Triggers not available.".to_string()
    }

    /// List all cron jobs as formatted text.
    async fn list_schedules_text(&self) -> String {
        "Schedules not available.".to_string()
    }

    /// Manage a cron job: add, del, or run.
    async fn manage_schedule_text(&self, _action: &str, _args: &[String]) -> String {
        "Schedules not available.".to_string()
    }

    /// Describe an image (for photo understanding).
    ///
    /// Used by the bridge to auto-describe photos from Telegram before
    /// forwarding the description to the agent.
    async fn describe_image(&self, _file_path: &str) -> Result<String, String> {
        Err("Image description not available.".to_string())
    }

    /// Transcribe an audio file to text (speech-to-text).
    ///
    /// Used by the bridge to auto-transcribe Telegram voice messages before
    /// forwarding the text to the agent.
    async fn transcribe_audio(&self, _file_path: &str) -> Result<String, String> {
        Err("Audio transcription not available.".to_string())
    }

    /// List pending approval requests as formatted text.
    async fn list_approvals_text(&self) -> String {
        "No approvals pending.".to_string()
    }

    /// List pending approvals as structured data for interactive UIs.
    ///
    /// Returns `(display_text, id_prefix, tool_name, risk_emoji)` tuples.
    async fn list_approval_details(&self) -> Vec<(String, String, String, String)> {
        vec![]
    }

    /// Approve or reject a pending approval by UUID prefix.
    async fn resolve_approval_text(&self, _id_prefix: &str, _approve: bool) -> String {
        "Approvals not available.".to_string()
    }

    // ── Budget, Network, A2A ──

    /// Show global budget status (limits, spend, % used).
    async fn budget_text(&self) -> String {
        "Budget information not available.".to_string()
    }

    /// Show RHP peer network status.
    async fn peers_text(&self) -> String {
        "Peer network not available.".to_string()
    }

    /// List discovered external A2A agents.
    async fn a2a_agents_text(&self) -> String {
        "A2A agents not available.".to_string()
    }
}

/// Per-channel rate limiter tracking message timestamps per user.
///
/// Key: `"{channel_type}:{platform_id}"`, Value: timestamps of recent messages.
#[derive(Debug, Clone, Default)]
pub struct ChannelRateLimiter {
    /// Recent message timestamps per user key.
    buckets: Arc<DashMap<String, Vec<Instant>>>,
}

impl ChannelRateLimiter {
    /// Check if a user is rate-limited. Returns `Ok(())` if allowed, `Err(msg)` if blocked.
    ///
    /// `max_per_minute`: 0 means unlimited.
    pub fn check(
        &self,
        channel_type: &str,
        platform_id: &str,
        max_per_minute: u32,
    ) -> Result<(), String> {
        if max_per_minute == 0 {
            return Ok(());
        }

        let key = format!("{channel_type}:{platform_id}");
        let now = Instant::now();
        let window = std::time::Duration::from_secs(60);

        let mut entry = self.buckets.entry(key).or_default();
        // Evict timestamps older than 1 minute
        entry.retain(|&ts| now.duration_since(ts) < window);

        if entry.len() >= max_per_minute as usize {
            return Err(format!(
                "Rate limit exceeded ({max_per_minute} messages/minute). Please wait."
            ));
        }

        entry.push(now);
        Ok(())
    }
}

/// Lightweight approval notification for push to channels.
#[derive(Debug, Clone)]
pub struct ApprovalNotification {
    pub agent_id: String,
    pub id_prefix: String,
    pub summary: String,
    pub risk_emoji: String,
    pub timeout_secs: u64,
}

/// Owns all running channel adapters and dispatches messages to agents.
pub struct BridgeManager {
    handle: Arc<dyn ChannelBridgeHandle>,
    router: Arc<AgentRouter>,
    rate_limiter: ChannelRateLimiter,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl BridgeManager {
    pub fn new(handle: Arc<dyn ChannelBridgeHandle>, router: Arc<AgentRouter>) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            handle,
            router,
            rate_limiter: ChannelRateLimiter::default(),
            shutdown_tx,
            shutdown_rx,
            tasks: Vec::new(),
        }
    }

    /// Start an adapter: subscribe to its message stream and spawn a dispatch task.
    pub async fn start_adapter(
        &mut self,
        adapter: Arc<dyn ChannelAdapter>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stream = adapter.start().await?;
        let handle = self.handle.clone();
        let router = self.router.clone();
        let rate_limiter = self.rate_limiter.clone();
        let adapter_clone = adapter.clone();
        let mut shutdown = self.shutdown_rx.clone();

        let task = tokio::spawn(async move {
            let mut stream = std::pin::pin!(stream);
            loop {
                tokio::select! {
                    msg = stream.next() => {
                        match msg {
                            Some(message) => {
                                dispatch_message(
                                    &message,
                                    &handle,
                                    &router,
                                    &adapter_clone,
                                    &rate_limiter,
                                ).await;
                            }
                            None => {
                                info!("Channel adapter {} stream ended", adapter_clone.name());
                                break;
                            }
                        }
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() {
                            info!("Shutting down channel adapter {}", adapter_clone.name());
                            break;
                        }
                    }
                }
            }
        });

        self.tasks.push(task);
        Ok(())
    }

    /// Start a background task that receives approval notifications and
    /// sends inline-keyboard messages to the appropriate Telegram chat.
    ///
    /// The `rx` channel receives `ApprovalRequest` from the kernel's
    /// `ApprovalManager` notification callback. The router's `last_sender`
    /// map is used to find which chat to notify.
    pub fn start_approval_notifier(
        &mut self,
        mut rx: tokio::sync::mpsc::Receiver<ApprovalNotification>,
        adapters: Vec<Arc<dyn ChannelAdapter>>,
    ) {
        let router = self.router.clone();
        let mut shutdown = self.shutdown_rx.clone();

        let task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    notification = rx.recv() => {
                        let Some(notif) = notification else { break };
                        // Find the chat that last talked to this agent
                        let Some((_channel, platform_id)) = router.last_sender_for_agent(&notif.agent_id) else {
                            debug!("Approval notification: no known chat for agent {}", notif.agent_id);
                            continue;
                        };

                        let user = crate::types::ChannelUser {
                            platform_id: platform_id.clone(),
                            display_name: String::new(),
                            rusty_hand_user: None,
                        };
                        let buttons = vec![vec![
                            crate::types::InlineButton {
                                text: "✅ Approve".to_string(),
                                callback_data: format!("approve:{}", notif.id_prefix),
                            },
                            crate::types::InlineButton {
                                text: "❌ Reject".to_string(),
                                callback_data: format!("reject:{}", notif.id_prefix),
                            },
                        ]];
                        let text = format!(
                            "{} Agent **{}** wants to execute:\n`{}`\n⏱️ {}s timeout",
                            notif.risk_emoji, notif.agent_id, notif.summary, notif.timeout_secs,
                        );

                        // Try each adapter until one can send
                        let mut sent = false;
                        for adapter in &adapters {
                            if adapter.send_with_buttons(&user, &text, &buttons).await.is_ok() {
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            warn!(
                                agent = %notif.agent_id,
                                "Failed to push approval notification — no adapter could deliver"
                            );
                        }
                    }
                    _ = shutdown.changed() => {
                        if *shutdown.borrow() { break; }
                    }
                }
            }
        });

        self.tasks.push(task);
    }

    /// Stop all adapters and wait for dispatch tasks to finish.
    ///
    /// Each task is awaited with a 10s timeout. If a task is stuck in a
    /// blocking network call (e.g. Telegram long-poll), it is aborted so
    /// shutdown cannot hang indefinitely.
    pub async fn stop(&mut self) {
        let _ = self.shutdown_tx.send(true);
        for task in self.tasks.drain(..) {
            let abort_handle = task.abort_handle();
            match tokio::time::timeout(std::time::Duration::from_secs(10), task).await {
                Ok(_) => {}
                Err(_) => {
                    warn!("Channel adapter task did not stop within 10s — aborting");
                    abort_handle.abort();
                }
            }
        }
    }
}

/// Bundled agent names tried (in order) as a last-ditch fallback when
/// no router rule matches an inbound message and no `default_agent`
/// is set on the channel.
const FALLBACK_AGENT_NAMES: &[&str] = &["assistant", "coordinator", "coder"];

/// Pick an agent for an inbound message that the router couldn't route.
///
/// The fresh-install path: a Telegram bot token is set, no
/// `default_agent` was configured (or the configured one couldn't
/// spawn), no bindings match. Without this fallback the user sees
/// "No agent assigned" forever. With it, we look at running agents
/// first (any will do, sorted by name for determinism), then try to
/// spawn one of the standard bundled meta-agents.
///
/// Returns either the resolved agent id or a user-facing error message
/// with a hint about how to fix the misconfiguration.
async fn resolve_fallback_agent(handle: &dyn ChannelBridgeHandle) -> Result<AgentId, String> {
    if let Ok(mut agents) = handle.list_agents().await {
        agents.sort_by(|a, b| a.1.cmp(&b.1));
        if let Some((id, _)) = agents.into_iter().next() {
            return Ok(id);
        }
    }
    for name in FALLBACK_AGENT_NAMES {
        match handle.spawn_agent_by_name(name).await {
            Ok(id) => return Ok(id),
            Err(e) => {
                debug!("Fallback spawn '{name}' failed: {e}");
            }
        }
    }
    Err(
        "No agents are running and no bundled fallback could be spawned. \
         Set `default_agent` under your channel section in config.toml \
         (e.g. `[channels.telegram] default_agent = \"assistant\"`) and \
         restart the daemon."
            .to_string(),
    )
}

/// Resolve channel type to its config string key.
fn channel_type_str(channel: &crate::types::ChannelType) -> &str {
    match channel {
        crate::types::ChannelType::Telegram => "telegram",
        crate::types::ChannelType::Discord => "discord",
        crate::types::ChannelType::Slack => "slack",
        crate::types::ChannelType::WebChat => "webchat",
        crate::types::ChannelType::CLI => "cli",
        crate::types::ChannelType::Custom(s) => s.as_str(),
    }
}

/// Send a response, applying output formatting and optional threading.
async fn send_response(
    adapter: &dyn ChannelAdapter,
    user: &ChannelUser,
    text: String,
    thread_id: Option<&str>,
    output_format: OutputFormat,
) {
    send_response_reply(adapter, user, text, thread_id, output_format, None).await;
}

async fn send_response_reply(
    adapter: &dyn ChannelAdapter,
    user: &ChannelUser,
    text: String,
    thread_id: Option<&str>,
    output_format: OutputFormat,
    reply_to: Option<&str>,
) {
    let formatted = formatter::format_for_channel(&text, output_format);
    let content = ChannelContent::Text(formatted);

    let result = if let Some(tid) = thread_id {
        adapter.send_in_thread(user, content, tid).await
    } else if let Some(reply_id) = reply_to {
        adapter.send_reply(user, content, reply_id).await
    } else {
        adapter.send(user, content).await
    };

    if let Err(e) = result {
        error!("Failed to send response: {e}");
    }
}

/// Dispatch a single incoming message — handles bot commands or routes to an agent.
///
/// Applies per-channel policies (DM/group filtering, rate limiting, formatting, threading).
async fn dispatch_message(
    message: &ChannelMessage,
    handle: &Arc<dyn ChannelBridgeHandle>,
    router: &Arc<AgentRouter>,
    adapter_arc: &Arc<dyn ChannelAdapter>,
    rate_limiter: &ChannelRateLimiter,
) {
    let adapter: &dyn ChannelAdapter = adapter_arc.as_ref();
    let ct_str = channel_type_str(&message.channel);

    // Fetch per-channel overrides (if configured)
    let overrides = handle.channel_overrides(ct_str).await;
    let output_format = overrides
        .as_ref()
        .and_then(|o| o.output_format)
        .unwrap_or(OutputFormat::Markdown);
    let threading_enabled = overrides.as_ref().map(|o| o.threading).unwrap_or(false);
    let thread_id = if threading_enabled {
        message.thread_id.as_deref()
    } else {
        None
    };

    // --- DM/Group policy check ---
    if let Some(ref ov) = overrides {
        if message.is_group {
            match ov.group_policy {
                GroupPolicy::Ignore => {
                    debug!("Ignoring group message on {ct_str} (group_policy=ignore)");
                    return;
                }
                GroupPolicy::CommandsOnly => {
                    // Only allow slash commands and ChannelContent::Command
                    let is_command = matches!(&message.content, ChannelContent::Command { .. })
                        || matches!(&message.content, ChannelContent::Text(t) if t.starts_with('/'));
                    if !is_command {
                        debug!("Ignoring non-command group message on {ct_str} (group_policy=commands_only)");
                        return;
                    }
                }
                GroupPolicy::MentionOnly => {
                    // Pass through — adapters should only forward mentioned messages.
                    // This is a hint for adapters, not enforced here.
                }
                GroupPolicy::All => {}
            }
        } else {
            // DM
            match ov.dm_policy {
                DmPolicy::Ignore => {
                    debug!("Ignoring DM on {ct_str} (dm_policy=ignore)");
                    return;
                }
                DmPolicy::AllowedOnly => {
                    // Rely on RBAC authorize_channel_user below
                }
                DmPolicy::Respond => {}
            }
        }
    }

    // --- Rate limiting ---
    if let Some(ref ov) = overrides {
        if ov.rate_limit_per_user > 0 {
            if let Err(msg) =
                rate_limiter.check(ct_str, &message.sender.platform_id, ov.rate_limit_per_user)
            {
                send_response(adapter, &message.sender, msg, thread_id, output_format).await;
                return;
            }
        }
    }

    // --- Inline keyboard callback handling (approval buttons, etc.) ---
    if let ChannelContent::CallbackQuery {
        ref data,
        ref callback_id,
    } = message.content
    {
        let response_text = if let Some(id_prefix) = data.strip_prefix("approve:") {
            let result = handle.resolve_approval_text(id_prefix, true).await;
            let _ = adapter
                .answer_callback(callback_id, Some("✅ Approved"))
                .await;
            result
        } else if let Some(id_prefix) = data.strip_prefix("reject:") {
            let result = handle.resolve_approval_text(id_prefix, false).await;
            let _ = adapter
                .answer_callback(callback_id, Some("❌ Rejected"))
                .await;
            result
        } else {
            let _ = adapter.answer_callback(callback_id, None).await;
            // Forward unknown callbacks to the agent as text
            format!("[callback: {data}]")
        };
        send_response(
            adapter,
            &message.sender,
            response_text,
            thread_id,
            output_format,
        )
        .await;
        return;
    }

    let text = match &message.content {
        ChannelContent::Text(t) => t.clone(),
        ChannelContent::Command { name, args } => {
            let result = handle_command(name, args, handle, router, &message.sender).await;
            send_response(adapter, &message.sender, result, thread_id, output_format).await;
            return;
        }
        // Media: download file from Telegram, then forward to agent
        ChannelContent::Image { .. }
        | ChannelContent::Voice { .. }
        | ChannelContent::File { .. } => {
            let file_id = message
                .metadata
                .get("file_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let media_type = message
                .metadata
                .get("media_type")
                .and_then(|v| v.as_str())
                .unwrap_or("file");

            if file_id.is_empty() {
                send_response(
                    adapter,
                    &message.sender,
                    "Could not process media: missing file_id.".to_string(),
                    thread_id,
                    output_format,
                )
                .await;
                return;
            }

            // Download the file from Telegram
            let ext = match media_type {
                "photo" => "jpg",
                "voice" => "ogg",
                "audio" => "mp3",
                _ => "bin",
            };
            match adapter
                .download_file(file_id, ext)
                .await
                .map_err(|e| e.to_string())
            {
                Ok(local_path) => {
                    let caption_text = message
                        .metadata
                        .get("caption")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    // For voice/audio: auto-transcribe and send text to agent
                    if media_type == "voice" || media_type == "audio" {
                        match handle.transcribe_audio(&local_path).await {
                            Ok(transcription) => {
                                if caption_text.is_empty() {
                                    format!("[Voice message transcription]: {transcription}")
                                } else {
                                    format!(
                                        "[Voice message transcription]: {transcription}\n\nCaption: {caption_text}"
                                    )
                                }
                            }
                            Err(e) => {
                                warn!("Voice transcription failed: {e}");
                                let _ = tokio::fs::remove_file(&local_path).await;
                                "[Voice message received, but transcription failed. Please try again.]".to_string()
                            }
                        }
                    } else if media_type == "photo" {
                        // Auto-describe the photo for the agent
                        let description = match handle.describe_image(&local_path).await {
                            Ok(desc) => desc,
                            Err(_) => "Image received (description unavailable)".to_string(),
                        };
                        if caption_text.is_empty() {
                            format!("[Photo: {description}]")
                        } else {
                            format!("[Photo with caption \"{caption_text}\": {description}]")
                        }
                    } else {
                        let filename = match &message.content {
                            ChannelContent::File { filename, .. } => filename.clone(),
                            _ => "file".to_string(),
                        };
                        format!("[File received: {filename}. Saved to {local_path}]")
                    }
                }
                Err(e) => {
                    warn!("Media download failed for file_id={file_id}: {e}");
                    send_response(
                        adapter,
                        &message.sender,
                        "Could not download media file. Please try again.".to_string(),
                        thread_id,
                        output_format,
                    )
                    .await;
                    return;
                }
            }
        }
        ChannelContent::Location { lat, lon } => {
            format!("[Location shared: {lat}, {lon}](https://maps.google.com/?q={lat},{lon})")
        }
        _ => {
            send_response(
                adapter,
                &message.sender,
                "Unsupported message type.".to_string(),
                thread_id,
                output_format,
            )
            .await;
            return;
        }
    };

    // Special handling: /approvals with inline keyboard buttons
    if text == "/approvals" {
        let details = handle.list_approval_details().await;
        if details.is_empty() {
            send_response(
                adapter,
                &message.sender,
                "No approvals pending.".to_string(),
                thread_id,
                output_format,
            )
            .await;
        } else {
            for (display, id_prefix, _tool, risk_emoji) in &details {
                let buttons = vec![vec![
                    crate::types::InlineButton {
                        text: "✅ Approve".to_string(),
                        callback_data: format!("approve:{id_prefix}"),
                    },
                    crate::types::InlineButton {
                        text: "❌ Reject".to_string(),
                        callback_data: format!("reject:{id_prefix}"),
                    },
                ]];
                let text = format!("{risk_emoji} {display}");
                let _ = adapter
                    .send_with_buttons(&message.sender, &text, &buttons)
                    .await;
            }
        }
        return;
    }

    // Check if it's a slash command embedded in text (e.g. "/agents")
    if text.starts_with('/') {
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd = &parts[0][1..]; // strip leading '/'
        let args: Vec<String> = if parts.len() > 1 {
            parts[1].split_whitespace().map(String::from).collect()
        } else {
            vec![]
        };

        if matches!(
            cmd,
            "start"
                | "help"
                | "agents"
                | "agent"
                | "status"
                | "models"
                | "providers"
                | "new"
                | "compact"
                | "model"
                | "stop"
                | "usage"
                | "think"
                | "skills"
                | "workflows"
                | "workflow"
                | "triggers"
                | "trigger"
                | "schedules"
                | "schedule"
                | "approvals"
                | "approve"
                | "reject"
                | "budget"
                | "peers"
                | "a2a"
        ) {
            let result = handle_command(cmd, &args, handle, router, &message.sender).await;
            send_response(adapter, &message.sender, result, thread_id, output_format).await;
            return;
        }
        // Other slash commands pass through to the agent
    }

    // Check broadcast routing first
    if router.has_broadcast(&message.sender.platform_id) {
        let targets = router.resolve_broadcast(&message.sender.platform_id);
        if !targets.is_empty() {
            // RBAC check applies to broadcast too
            if let Err(denied) = handle
                .authorize_channel_user(ct_str, &message.sender.platform_id, "chat")
                .await
            {
                send_response(
                    adapter,
                    &message.sender,
                    format!("Access denied: {denied}"),
                    thread_id,
                    output_format,
                )
                .await;
                return;
            }
            let _ = adapter.send_typing(&message.sender).await;

            let strategy = router.broadcast_strategy();
            let mut responses = Vec::new();

            match strategy {
                rusty_hand_types::config::BroadcastStrategy::Parallel => {
                    let mut handles_vec = Vec::new();
                    for (name, maybe_id) in &targets {
                        if let Some(aid) = maybe_id {
                            let h = handle.clone();
                            let t = text.clone();
                            let aid = *aid;
                            let name = name.clone();
                            handles_vec.push(tokio::spawn(async move {
                                let result = h.send_message(aid, &t).await;
                                (name, aid, result)
                            }));
                        }
                    }
                    for jh in handles_vec {
                        if let Ok((name, _aid, result)) = jh.await {
                            match result {
                                Ok(r) => responses.push(format!("[{name}]: {r}")),
                                Err(e) => responses.push(format!("[{name}]: Error: {e}")),
                            }
                        }
                    }
                }
                rusty_hand_types::config::BroadcastStrategy::Sequential => {
                    for (name, maybe_id) in &targets {
                        if let Some(aid) = maybe_id {
                            match handle.send_message(*aid, &text).await {
                                Ok(r) => responses.push(format!("[{name}]: {r}")),
                                Err(e) => responses.push(format!("[{name}]: Error: {e}")),
                            }
                        }
                    }
                }
            }

            let combined = responses.join("\n\n");
            send_response(adapter, &message.sender, combined, thread_id, output_format).await;
            return;
        }
    }

    // Route to agent (standard path)
    let agent_id = router.resolve(
        &message.channel,
        &message.sender.platform_id,
        message.sender.rusty_hand_user.as_deref(),
    );

    let agent_id = match agent_id {
        Some(id) => id,
        None => match resolve_fallback_agent(handle.as_ref()).await {
            Ok(id) => {
                info!(
                    channel = ct_str,
                    "No default agent configured — auto-routed to fallback (set [channels.<x>].default_agent in config.toml to make this explicit)"
                );
                id
            }
            Err(msg) => {
                send_response(adapter, &message.sender, msg, thread_id, output_format).await;
                return;
            }
        },
    };

    // Track the last sender for this agent (used for approval push notifications)
    router.track_sender(agent_id, ct_str, &message.sender.platform_id);

    // RBAC: authorize the user before forwarding to agent
    if let Err(denied) = handle
        .authorize_channel_user(ct_str, &message.sender.platform_id, "chat")
        .await
    {
        send_response(
            adapter,
            &message.sender,
            format!("Access denied: {denied}"),
            thread_id,
            output_format,
        )
        .await;
        return;
    }

    // Auto-reply check — if enabled, the engine decides whether to process this message.
    // If auto-reply is enabled but suppressed for this message, skip agent call entirely.
    if let Some(reply) = handle.check_auto_reply(agent_id, &text).await {
        send_response(adapter, &message.sender, reply, thread_id, output_format).await;
        handle
            .record_delivery(agent_id, ct_str, &message.sender.platform_id, true, None)
            .await;
        return;
    }

    // Send typing indicator (best-effort)
    let _ = adapter.send_typing(&message.sender).await;

    // Send to agent and relay response.
    // For adapters with streaming support, pipe chunks in real-time so
    // the user sees progressive tool-use updates (e.g. "⚙️ web_search...").
    let result = if adapter.supports_streaming() {
        let (chunk_tx, chunk_rx) = tokio::sync::mpsc::channel::<String>(64);

        // Start adapter streaming consumer in parallel — it will
        // progressively edit the Telegram message as chunks arrive.
        let user_for_stream = message.sender.clone();
        let adapter_for_stream = Arc::clone(adapter_arc);
        let adapter_name = adapter.name().to_string();
        let stream_task = tokio::spawn(async move {
            adapter_for_stream
                .send_streaming(&user_for_stream, chunk_rx)
                .await
                .map_err(|e| e.to_string())
        });

        // Drive the agent loop; chunks are piped to chunk_tx in real-time
        // by KernelBridgeAdapter.send_message_to_stream(), which reads
        // StreamEvent from the kernel and converts tool markers to text.
        let response = handle
            .send_message_to_stream(agent_id, &text, chunk_tx)
            .await;

        // Wait for adapter to finish editing the last message. Promote a
        // streaming-side adapter failure into the response error so we do
        // NOT later record the message as successfully delivered.
        match stream_task.await {
            Ok(Ok(())) => response,
            Ok(Err(e)) => {
                warn!("Streaming send to {adapter_name} failed: {e}");
                match response {
                    Ok(_) => Err(format!("Streaming send to {adapter_name} failed: {e}")),
                    Err(original) => Err(original),
                }
            }
            Err(join_err) => {
                warn!("Streaming task for {adapter_name} panicked: {join_err}");
                match response {
                    Ok(_) => Err(format!(
                        "Streaming task for {adapter_name} panicked: {join_err}"
                    )),
                    Err(original) => Err(original),
                }
            }
        }
    } else {
        handle.send_message(agent_id, &text).await
    };

    match result {
        Ok(response) => {
            if !adapter.supports_streaming() {
                send_response_reply(
                    adapter,
                    &message.sender,
                    response,
                    thread_id,
                    output_format,
                    Some(&message.platform_message_id),
                )
                .await;
            }
            handle
                .record_delivery(agent_id, ct_str, &message.sender.platform_id, true, None)
                .await;
        }
        Err(e) => {
            warn!("Agent error for {agent_id}: {e}");
            let err_msg = format!("Agent error: {e}");
            send_response(
                adapter,
                &message.sender,
                err_msg.clone(),
                thread_id,
                output_format,
            )
            .await;
            handle
                .record_delivery(
                    agent_id,
                    ct_str,
                    &message.sender.platform_id,
                    false,
                    Some(&err_msg),
                )
                .await;
        }
    }
}

/// Handle a bot command (returns the response text).
async fn handle_command(
    name: &str,
    args: &[String],
    handle: &Arc<dyn ChannelBridgeHandle>,
    router: &Arc<AgentRouter>,
    sender: &ChannelUser,
) -> String {
    match name {
        "start" => {
            let agents = handle.list_agents().await.unwrap_or_default();
            let mut msg =
                "Welcome to RustyHand! I connect you to AI agents.\n\nAvailable agents:\n"
                    .to_string();
            if agents.is_empty() {
                msg.push_str("  (none running)\n");
            } else {
                for (_, name) in &agents {
                    msg.push_str(&format!("  - {name}\n"));
                }
            }
            msg.push_str("\nCommands:\n/agents - list agents\n/agent <name> - select an agent\n/help - show this help");
            msg
        }
        "help" => "RustyHand Bot Commands:\n\
             \n\
             Session:\n\
             /agents - list running agents\n\
             /agent <name> - select which agent to talk to\n\
             /new - reset session (clear messages)\n\
             /compact - trigger LLM session compaction\n\
             /model [name] - show or switch agent model\n\
             /stop - cancel current agent run\n\
             /usage - show session token usage and cost\n\
             /think [on|off] - toggle extended thinking\n\
             \n\
             Info:\n\
             /models - list available AI models\n\
             /providers - show configured providers\n\
             /skills - list installed skills\n\
             /status - show system status\n\
             \n\
             Automation:\n\
             /workflows - list workflows\n\
             /workflow run <name> [input] - run a workflow\n\
             /triggers - list event triggers\n\
             /trigger add <agent> <pattern> <prompt> - create trigger\n\
             /trigger del <id> - remove trigger\n\
             /schedules - list cron jobs\n\
             /schedule add <agent> <cron-5-fields> <message> - create job\n\
             /schedule del <id> - remove job\n\
             /schedule run <id> - run job now\n\
             /approvals - list pending approvals\n\
             /approve <id> - approve a request\n\
             /reject <id> - reject a request\n\
             \n\
             Monitoring:\n\
             /budget - show spending limits and current costs\n\
             /peers - show RHP peer network status\n\
             /a2a - list discovered external A2A agents\n\
             \n\
             /start - show welcome message\n\
             /help - show this help"
            .to_string(),
        "status" => handle.uptime_info().await,
        "agents" => {
            let agents = handle.list_agents().await.unwrap_or_default();
            if agents.is_empty() {
                "No agents running.".to_string()
            } else {
                let mut msg = "Running agents:\n".to_string();
                for (_, name) in &agents {
                    msg.push_str(&format!("  - {name}\n"));
                }
                msg
            }
        }
        "agent" => {
            if args.is_empty() {
                return "Usage: /agent <name>".to_string();
            }
            let agent_name = &args[0];
            match handle.find_agent_by_name(agent_name).await {
                Ok(Some(agent_id)) => {
                    router.set_user_default(sender.platform_id.clone(), agent_id);
                    format!("Now talking to agent: {agent_name}")
                }
                Ok(None) => {
                    // Try to spawn it
                    match handle.spawn_agent_by_name(agent_name).await {
                        Ok(agent_id) => {
                            router.set_user_default(sender.platform_id.clone(), agent_id);
                            format!("Spawned and connected to agent: {agent_name}")
                        }
                        Err(e) => {
                            format!("Agent '{agent_name}' not found and could not spawn: {e}")
                        }
                    }
                }
                Err(e) => format!("Error finding agent: {e}"),
            }
        }
        "new" => {
            // Need to resolve the user's current agent
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => handle
                    .reset_session(aid)
                    .await
                    .unwrap_or_else(|e| format!("Error: {e}")),
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "compact" => {
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => handle
                    .compact_session(aid)
                    .await
                    .unwrap_or_else(|e| format!("Error: {e}")),
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "model" => {
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => {
                    if args.is_empty() {
                        // Show current model
                        handle
                            .set_model(aid, "")
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    } else {
                        handle
                            .set_model(aid, &args[0])
                            .await
                            .unwrap_or_else(|e| format!("Error: {e}"))
                    }
                }
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "stop" => {
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => handle
                    .stop_run(aid)
                    .await
                    .unwrap_or_else(|e| format!("Error: {e}")),
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "usage" => {
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => handle
                    .session_usage(aid)
                    .await
                    .unwrap_or_else(|e| format!("Error: {e}")),
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "think" => {
            let agent_id = router.resolve(
                &crate::types::ChannelType::CLI,
                &sender.platform_id,
                sender.rusty_hand_user.as_deref(),
            );
            match agent_id {
                Some(aid) => {
                    let on = args.first().map(|a| a == "on").unwrap_or(true);
                    handle
                        .set_thinking(aid, on)
                        .await
                        .unwrap_or_else(|e| format!("Error: {e}"))
                }
                None => "No agent selected. Use /agent <name> first.".to_string(),
            }
        }
        "models" => handle.list_models_text().await,
        "providers" => handle.list_providers_text().await,
        "skills" => handle.list_skills_text().await,

        // ── Automation: workflows, triggers, schedules, approvals ──
        "workflows" => handle.list_workflows_text().await,
        "workflow" => {
            if args.len() >= 2 && args[0] == "run" {
                let wf_name = &args[1];
                let input = if args.len() > 2 {
                    args[2..].join(" ")
                } else {
                    String::new()
                };
                handle.run_workflow_text(wf_name, &input).await
            } else {
                "Usage: /workflow run <name> [input]".to_string()
            }
        }
        "triggers" => handle.list_triggers_text().await,
        "trigger" => {
            if args.len() >= 4 && args[0] == "add" {
                let agent_name = &args[1];
                let pattern = &args[2];
                let prompt = args[3..].join(" ");
                handle
                    .create_trigger_text(agent_name, pattern, &prompt)
                    .await
            } else if args.len() >= 2 && args[0] == "del" {
                handle.delete_trigger_text(&args[1]).await
            } else {
                "Usage:\n  /trigger add <agent> <pattern> <prompt>\n  /trigger del <id-prefix>"
                    .to_string()
            }
        }
        "schedules" => handle.list_schedules_text().await,
        "schedule" => {
            if args.is_empty() {
                return "Usage:\n  /schedule add <agent> <cron-5-fields> <message>\n  /schedule del <id-prefix>\n  /schedule run <id-prefix>".to_string();
            }
            let action = args[0].as_str();
            match action {
                "add" | "del" | "run" => {
                    handle.manage_schedule_text(action, &args[1..]).await
                }
                _ => "Usage:\n  /schedule add <agent> <cron-5-fields> <message>\n  /schedule del <id-prefix>\n  /schedule run <id-prefix>".to_string(),
            }
        }
        "approvals" => handle.list_approvals_text().await,
        "approve" => {
            if args.is_empty() {
                "Usage: /approve <id-prefix>".to_string()
            } else {
                handle.resolve_approval_text(&args[0], true).await
            }
        }
        "reject" => {
            if args.is_empty() {
                "Usage: /reject <id-prefix>".to_string()
            } else {
                handle.resolve_approval_text(&args[0], false).await
            }
        }

        // ── Budget, Network, A2A ──
        "budget" => handle.budget_text().await,
        "peers" => handle.peers_text().await,
        "a2a" => handle.a2a_agents_text().await,

        _ => format!("Unknown command: /{name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ChannelType;
    use std::sync::Mutex;

    /// Mock kernel handle for testing.
    struct MockHandle {
        agents: Mutex<Vec<(AgentId, String)>>,
    }

    #[async_trait]
    impl ChannelBridgeHandle for MockHandle {
        async fn send_message(&self, _agent_id: AgentId, message: &str) -> Result<String, String> {
            Ok(format!("Echo: {message}"))
        }
        async fn find_agent_by_name(&self, name: &str) -> Result<Option<AgentId>, String> {
            let agents = self.agents.lock().unwrap();
            Ok(agents.iter().find(|(_, n)| n == name).map(|(id, _)| *id))
        }
        async fn list_agents(&self) -> Result<Vec<(AgentId, String)>, String> {
            Ok(self.agents.lock().unwrap().clone())
        }
        async fn spawn_agent_by_name(&self, _manifest_name: &str) -> Result<AgentId, String> {
            Err("spawn not implemented in mock".to_string())
        }
    }

    /// Regression: when no router rule matches, the dispatcher must
    /// fall through to a running agent or a bundled fallback rather
    /// than dead-ending with "No agent assigned". The user complaint
    /// that prompted the v0.7.7 → v0.7.10 rescue chain.
    #[tokio::test]
    async fn fallback_routes_to_running_agent_alphabetically() {
        let coder = AgentId::new();
        let assistant = AgentId::new();
        let zeta = AgentId::new();
        // Insert in deliberately non-alphabetical order to make sure
        // the helper sorts.
        let mock: Arc<dyn ChannelBridgeHandle> = Arc::new(MockHandle {
            agents: Mutex::new(vec![
                (zeta, "zeta".to_string()),
                (coder, "coder".to_string()),
                (assistant, "assistant".to_string()),
            ]),
        });
        let resolved = resolve_fallback_agent(mock.as_ref()).await.unwrap();
        assert_eq!(resolved, assistant, "fallback picks first agent by name");
    }

    #[tokio::test]
    async fn fallback_errors_clearly_when_nothing_running_or_spawnable() {
        let mock: Arc<dyn ChannelBridgeHandle> = Arc::new(MockHandle {
            agents: Mutex::new(vec![]),
        });
        let err = resolve_fallback_agent(mock.as_ref()).await.unwrap_err();
        assert!(
            err.contains("default_agent") && err.contains("config.toml"),
            "fallback error must point the user at the config knob, got: {err}"
        );
    }

    #[test]
    fn test_command_parsing() {
        // Verify slash commands are parsed correctly from text
        let text = "/agent hello-world";
        assert!(text.starts_with('/'));
        let parts: Vec<&str> = text.splitn(2, ' ').collect();
        let cmd = &parts[0][1..];
        assert_eq!(cmd, "agent");
        let args: Vec<String> = if parts.len() > 1 {
            parts[1].split_whitespace().map(String::from).collect()
        } else {
            vec![]
        };
        assert_eq!(args, vec!["hello-world"]);
    }

    #[tokio::test]
    async fn test_dispatch_routes_to_correct_agent() {
        let agent_id = AgentId::new();
        let mock = Arc::new(MockHandle {
            agents: Mutex::new(vec![(agent_id, "test-agent".to_string())]),
        });

        let handle: Arc<dyn ChannelBridgeHandle> = mock;

        // Verify find_agent_by_name works
        let found = handle.find_agent_by_name("test-agent").await.unwrap();
        assert_eq!(found, Some(agent_id));

        let not_found = handle.find_agent_by_name("nonexistent").await.unwrap();
        assert_eq!(not_found, None);

        // Verify send_message echoes
        let response = handle.send_message(agent_id, "hello").await.unwrap();
        assert_eq!(response, "Echo: hello");
    }

    #[tokio::test]
    async fn test_handle_command_agents() {
        let agent_id = AgentId::new();
        let handle: Arc<dyn ChannelBridgeHandle> = Arc::new(MockHandle {
            agents: Mutex::new(vec![(agent_id, "coder".to_string())]),
        });
        let router = Arc::new(AgentRouter::new());
        let sender = ChannelUser {
            platform_id: "user1".to_string(),
            display_name: "Test".to_string(),
            rusty_hand_user: None,
        };

        let result = handle_command("agents", &[], &handle, &router, &sender).await;
        assert!(result.contains("coder"));

        let result = handle_command("help", &[], &handle, &router, &sender).await;
        assert!(result.contains("/agents"));
    }

    #[tokio::test]
    async fn test_handle_command_agent_select() {
        let agent_id = AgentId::new();
        let handle: Arc<dyn ChannelBridgeHandle> = Arc::new(MockHandle {
            agents: Mutex::new(vec![(agent_id, "coder".to_string())]),
        });
        let router = Arc::new(AgentRouter::new());
        let sender = ChannelUser {
            platform_id: "user1".to_string(),
            display_name: "Test".to_string(),
            rusty_hand_user: None,
        };

        // Select existing agent
        let result =
            handle_command("agent", &["coder".to_string()], &handle, &router, &sender).await;
        assert!(result.contains("Now talking to agent: coder"));

        // Verify router was updated
        let resolved = router.resolve(&ChannelType::Telegram, "user1", None);
        assert_eq!(resolved, Some(agent_id));
    }

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = ChannelRateLimiter::default();
        assert!(limiter.check("telegram", "user1", 5).is_ok());
        assert!(limiter.check("telegram", "user1", 5).is_ok());
        assert!(limiter.check("telegram", "user1", 5).is_ok());
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = ChannelRateLimiter::default();
        for _ in 0..3 {
            limiter.check("telegram", "user1", 3).unwrap();
        }
        // 4th should be blocked
        let result = limiter.check("telegram", "user1", 3);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Rate limit exceeded"));
    }

    #[test]
    fn test_rate_limiter_zero_means_unlimited() {
        let limiter = ChannelRateLimiter::default();
        for _ in 0..100 {
            assert!(limiter.check("telegram", "user1", 0).is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_separate_users() {
        let limiter = ChannelRateLimiter::default();
        for _ in 0..3 {
            limiter.check("telegram", "user1", 3).unwrap();
        }
        // user1 is blocked
        assert!(limiter.check("telegram", "user1", 3).is_err());
        // user2 should still be ok
        assert!(limiter.check("telegram", "user2", 3).is_ok());
    }

    #[test]
    fn test_dm_policy_filtering() {
        // Test that DmPolicy::Ignore would be checked
        assert_eq!(DmPolicy::default(), DmPolicy::Respond);
        assert_eq!(GroupPolicy::default(), GroupPolicy::MentionOnly);
    }

    #[test]
    fn test_channel_type_str() {
        assert_eq!(channel_type_str(&ChannelType::Telegram), "telegram");
        assert_eq!(channel_type_str(&ChannelType::Discord), "discord");
        assert_eq!(channel_type_str(&ChannelType::Slack), "slack");
        assert_eq!(channel_type_str(&ChannelType::WebChat), "webchat");
        assert_eq!(channel_type_str(&ChannelType::CLI), "cli");
        assert_eq!(
            channel_type_str(&ChannelType::Custom("matrix".to_string())),
            "matrix"
        );
    }
}
