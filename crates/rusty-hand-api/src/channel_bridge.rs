//! Channel bridge wiring — connects the RustyHand kernel to channel adapters.
//!
//! Implements `ChannelBridgeHandle` on `RustyHandKernel` and provides the
//! `start_channel_bridge()` entry point called by the daemon.

use async_trait::async_trait;
use rusty_hand_channels::bridge::{BridgeManager, ChannelBridgeHandle};
use rusty_hand_channels::discord::DiscordAdapter;
use rusty_hand_channels::router::AgentRouter;
use rusty_hand_channels::slack::SlackAdapter;
use rusty_hand_channels::telegram::TelegramAdapter;
use rusty_hand_channels::types::ChannelAdapter;
use rusty_hand_kernel::RustyHandKernel;
use rusty_hand_types::agent::AgentId;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

/// Convert a `StreamEvent` to a text string for streaming adapters.
///
/// Returns `Some(text)` for events that should be displayed (text deltas,
/// tool status markers), or `None` for events the adapter should skip.
fn stream_event_to_text(event: &rusty_hand_runtime::llm_driver::StreamEvent) -> Option<String> {
    use rusty_hand_runtime::llm_driver::StreamEvent;
    match event {
        StreamEvent::TextDelta { text } => Some(text.clone()),
        StreamEvent::ToolUseStart { name, .. } => Some(format!("\n⚙️ {name}...\n")),
        StreamEvent::ToolExecutionResult { name, is_error, .. } => {
            let icon = if *is_error { "❌" } else { "✅" };
            Some(format!("\n{icon} {name}\n"))
        }
        _ => None,
    }
}

/// Wraps `RustyHandKernel` to implement `ChannelBridgeHandle`.
pub struct KernelBridgeAdapter {
    kernel: Arc<RustyHandKernel>,
    started_at: Instant,
}

#[async_trait]
impl ChannelBridgeHandle for KernelBridgeAdapter {
    async fn send_message(&self, agent_id: AgentId, message: &str) -> Result<String, String> {
        let result = self
            .kernel
            .send_message(agent_id, message)
            .await
            .map_err(|e| format!("{e}"))?;
        Ok(result.response)
    }

    async fn send_message_to_stream(
        &self,
        agent_id: AgentId,
        message: &str,
        tx: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String, String> {
        let kernel = Arc::clone(&self.kernel);
        let msg_owned = message.to_string();
        let (mut rx, join_handle) = kernel
            .send_message_streaming(agent_id, &msg_owned, Some(kernel.clone()))
            .map_err(|e| format!("{e}"))?;

        // Pipe StreamEvent → tx channel as text chunks in real-time.
        // TextDelta passes through; tool events emit status markers
        // (⚙️/✅/❌) that the Telegram adapter shows as progressive edits.
        //
        // Don't break on ContentComplete — the agent loop may execute tools
        // and call the LLM again (multiple iterations). The channel is closed
        // by the kernel when the entire agent loop finishes.
        while let Some(event) = rx.recv().await {
            if let Some(text) = stream_event_to_text(&event) {
                let _ = tx.send(text).await;
            }
        }

        let result = join_handle
            .await
            .map_err(|e| format!("Task join: {e}"))?
            .map_err(|e| format!("{e}"))?;
        Ok(result.response)
    }

    async fn find_agent_by_name(&self, name: &str) -> Result<Option<AgentId>, String> {
        Ok(self.kernel.registry.find_by_name(name).map(|e| e.id))
    }

    async fn list_agents(&self) -> Result<Vec<(AgentId, String)>, String> {
        Ok(self
            .kernel
            .registry
            .list()
            .iter()
            .map(|e| (e.id, e.name.clone()))
            .collect())
    }

    async fn spawn_agent_by_name(&self, manifest_name: &str) -> Result<AgentId, String> {
        let manifest_path = resolve_manifest_path(&self.kernel.config.home_dir, manifest_name)
            .ok_or_else(|| {
                format!(
                    "Manifest '{manifest_name}' not found in {}/agents/ or $RUSTY_HAND_AGENTS_DIR",
                    self.kernel.config.home_dir.display()
                )
            })?;

        let contents = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read manifest: {e}"))?;

        let manifest: rusty_hand_types::agent::AgentManifest =
            toml::from_str(&contents).map_err(|e| format!("Invalid manifest TOML: {e}"))?;

        let agent_id = self
            .kernel
            .spawn_agent(manifest)
            .map_err(|e| format!("Failed to spawn agent: {e}"))?;

        Ok(agent_id)
    }

    async fn uptime_info(&self) -> String {
        let uptime = self.started_at.elapsed();
        let agents = self.list_agents().await.unwrap_or_default();
        let secs = uptime.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        if hours > 0 {
            format!(
                "RustyHand status: {}h {}m uptime, {} agent(s)",
                hours,
                mins,
                agents.len()
            )
        } else {
            format!(
                "RustyHand status: {}m uptime, {} agent(s)",
                mins,
                agents.len()
            )
        }
    }

    async fn list_models_text(&self) -> String {
        let catalog = self
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let available = catalog.available_models();
        if available.is_empty() {
            return "No models available. Configure API keys to enable providers.".to_string();
        }
        let mut msg = format!("Available models ({}):\n", available.len());
        // Group by provider
        let mut by_provider: std::collections::HashMap<
            &str,
            Vec<&rusty_hand_types::model_catalog::ModelCatalogEntry>,
        > = std::collections::HashMap::new();
        for m in &available {
            by_provider.entry(m.provider.as_str()).or_default().push(m);
        }
        let mut providers: Vec<&&str> = by_provider.keys().collect();
        providers.sort();
        for provider in providers {
            let provider_name = catalog
                .get_provider(provider)
                .map(|p| p.display_name.as_str())
                .unwrap_or(provider);
            msg.push_str(&format!("\n{}:\n", provider_name));
            for m in &by_provider[provider] {
                let cost = if m.input_cost_per_m > 0.0 {
                    format!(
                        " (${:.2}/${:.2} per M)",
                        m.input_cost_per_m, m.output_cost_per_m
                    )
                } else {
                    " (free/local)".to_string()
                };
                msg.push_str(&format!("  {} — {}{}\n", m.id, m.display_name, cost));
            }
        }
        msg
    }

    async fn list_providers_text(&self) -> String {
        let catalog = self
            .kernel
            .model_catalog
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let mut msg = "Providers:\n".to_string();
        for p in catalog.list_providers() {
            let status = match p.auth_status {
                rusty_hand_types::model_catalog::AuthStatus::Configured => "configured",
                rusty_hand_types::model_catalog::AuthStatus::Missing => "not configured",
                rusty_hand_types::model_catalog::AuthStatus::NotRequired => "local (no key needed)",
            };
            msg.push_str(&format!(
                "  {} — {} [{}, {} model(s)]\n",
                p.id, p.display_name, status, p.model_count
            ));
        }
        msg
    }

    async fn list_skills_text(&self) -> String {
        let skills = self
            .kernel
            .skill_registry
            .read()
            .unwrap_or_else(|e| e.into_inner());
        let skills = skills.list();
        if skills.is_empty() {
            return "No skills installed. Place skills in ~/.rustyhand/skills/ or install from the marketplace.".to_string();
        }
        let mut msg = format!("Installed skills ({}):\n", skills.len());
        for skill in &skills {
            let runtime = format!("{:?}", skill.manifest.runtime.runtime_type);
            let tools_count = skill.manifest.tools.provided.len();
            let enabled = if skill.enabled { "" } else { " [disabled]" };
            msg.push_str(&format!(
                "  {} — {} ({}, {} tool(s)){}\n",
                skill.manifest.skill.name,
                skill.manifest.skill.description,
                runtime,
                tools_count,
                enabled,
            ));
        }
        msg
    }

    // ── Automation: workflows, triggers, schedules, approvals ──

    async fn list_workflows_text(&self) -> String {
        let workflows = self.kernel.workflows.list_workflows().await;
        if workflows.is_empty() {
            return "No workflows defined.".to_string();
        }
        let mut msg = format!("Workflows ({}):\n", workflows.len());
        for wf in &workflows {
            let steps = wf.steps.len();
            let desc = if wf.description.is_empty() {
                String::new()
            } else {
                format!(" — {}", wf.description)
            };
            msg.push_str(&format!("  {} ({} step(s)){}\n", wf.name, steps, desc));
        }
        msg
    }

    async fn run_workflow_text(&self, name: &str, input: &str) -> String {
        let workflows = self.kernel.workflows.list_workflows().await;
        let wf = match workflows.iter().find(|w| w.name.eq_ignore_ascii_case(name)) {
            Some(w) => w.clone(),
            None => return format!("Workflow '{name}' not found. Use /workflows to list."),
        };

        let run_id = match self
            .kernel
            .workflows
            .create_run(wf.id, input.to_string())
            .await
        {
            Some(id) => id,
            None => return "Failed to create workflow run.".to_string(),
        };

        let kernel = self.kernel.clone();
        let registry_ref = &self.kernel.registry;
        let result = self
            .kernel
            .workflows
            .execute_run(
                run_id,
                |step_agent| match step_agent {
                    rusty_hand_kernel::workflow::StepAgent::ById { id } => {
                        let aid: AgentId = id.parse().ok()?;
                        let entry = registry_ref.get(aid)?;
                        Some((aid, entry.name.clone()))
                    }
                    rusty_hand_kernel::workflow::StepAgent::ByName { name } => {
                        let entry = registry_ref.find_by_name(name)?;
                        Some((entry.id, entry.name.clone()))
                    }
                },
                |agent_id, message| {
                    let k = kernel.clone();
                    async move {
                        let result = k
                            .send_message(agent_id, &message)
                            .await
                            .map_err(|e| format!("{e}"))?;
                        Ok((
                            result.response,
                            result.total_usage.input_tokens,
                            result.total_usage.output_tokens,
                        ))
                    }
                },
            )
            .await;

        match result {
            Ok(output) => format!("Workflow '{}' completed:\n{}", wf.name, output),
            Err(e) => format!("Workflow '{}' failed: {}", wf.name, e),
        }
    }

    async fn list_triggers_text(&self) -> String {
        let triggers = self.kernel.triggers.list_all();
        if triggers.is_empty() {
            return "No triggers configured.".to_string();
        }
        let mut msg = format!("Triggers ({}):\n", triggers.len());
        for t in &triggers {
            let agent_name = self
                .kernel
                .registry
                .get(t.agent_id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| t.agent_id.to_string());
            let status = if t.enabled { "on" } else { "off" };
            let id_short = &t.id.0.to_string()[..8];
            msg.push_str(&format!(
                "  [{}] {} -> {} ({:?}) fires:{} [{}]\n",
                id_short,
                agent_name,
                t.prompt_template.chars().take(40).collect::<String>(),
                t.pattern,
                t.fire_count,
                status,
            ));
        }
        msg
    }

    async fn create_trigger_text(
        &self,
        agent_name: &str,
        pattern_str: &str,
        prompt: &str,
    ) -> String {
        let agent = match self.kernel.registry.find_by_name(agent_name) {
            Some(e) => e,
            None => return format!("Agent '{agent_name}' not found."),
        };

        let pattern = match parse_trigger_pattern(pattern_str) {
            Some(p) => p,
            None => {
                return format!(
                "Unknown pattern '{pattern_str}'. Valid: lifecycle, spawned:<name>, terminated, \
                 system, system:<keyword>, memory, memory:<key>, match:<text>, all"
            )
            }
        };

        let trigger_id = self
            .kernel
            .triggers
            .register(agent.id, pattern, prompt.to_string(), 0);
        let id_short = &trigger_id.0.to_string()[..8];
        format!("Trigger created [{id_short}] for agent '{agent_name}'.")
    }

    async fn delete_trigger_text(&self, id_prefix: &str) -> String {
        let triggers = self.kernel.triggers.list_all();
        let matched: Vec<_> = triggers
            .iter()
            .filter(|t| t.id.0.to_string().starts_with(id_prefix))
            .collect();
        match matched.len() {
            0 => format!("No trigger found matching '{id_prefix}'."),
            1 => {
                let t = matched[0];
                if self.kernel.triggers.remove(t.id) {
                    format!("Trigger [{}] removed.", &t.id.0.to_string()[..8])
                } else {
                    "Failed to remove trigger.".to_string()
                }
            }
            n => format!("{n} triggers match '{id_prefix}'. Be more specific."),
        }
    }

    async fn list_schedules_text(&self) -> String {
        let jobs = self.kernel.cron_scheduler.list_all_jobs();
        if jobs.is_empty() {
            return "No scheduled jobs.".to_string();
        }
        let mut msg = format!("Cron jobs ({}):\n", jobs.len());
        for job in &jobs {
            let agent_name = self
                .kernel
                .registry
                .get(job.agent_id)
                .map(|e| e.name.clone())
                .unwrap_or_else(|| job.agent_id.to_string());
            let status = if job.enabled { "on" } else { "off" };
            let id_short = &job.id.0.to_string()[..8];
            let sched = match &job.schedule {
                rusty_hand_types::scheduler::CronSchedule::Cron { expr, .. } => expr.clone(),
                rusty_hand_types::scheduler::CronSchedule::Every { every_secs } => {
                    format!("every {every_secs}s")
                }
                rusty_hand_types::scheduler::CronSchedule::At { at } => {
                    format!("at {}", at.format("%Y-%m-%d %H:%M"))
                }
            };
            let last = job
                .last_run
                .map(|t| t.format("%m-%d %H:%M").to_string())
                .unwrap_or_else(|| "never".to_string());
            msg.push_str(&format!(
                "  [{}] {} — {} ({}) last:{} [{}]\n",
                id_short, job.name, sched, agent_name, last, status,
            ));
        }
        msg
    }

    async fn manage_schedule_text(&self, action: &str, args: &[String]) -> String {
        match action {
            "add" => {
                // Expected: <agent> <f1> <f2> <f3> <f4> <f5> <message...>
                // 5 cron fields: min hour dom month dow
                if args.len() < 7 {
                    return "Usage: /schedule add <agent> <min> <hour> <dom> <month> <dow> <message>".to_string();
                }
                let agent_name = &args[0];
                let agent = match self.kernel.registry.find_by_name(agent_name) {
                    Some(e) => e,
                    None => return format!("Agent '{agent_name}' not found."),
                };
                let cron_expr = args[1..6].join(" ");
                let message = args[6..].join(" ");

                let job = rusty_hand_types::scheduler::CronJob {
                    id: rusty_hand_types::scheduler::CronJobId::new(),
                    agent_id: agent.id,
                    name: format!("chat-{}", &agent.name),
                    enabled: true,
                    schedule: rusty_hand_types::scheduler::CronSchedule::Cron {
                        expr: cron_expr.clone(),
                        tz: None,
                    },
                    action: rusty_hand_types::scheduler::CronAction::AgentTurn {
                        message: message.clone(),
                        model_override: None,
                        timeout_secs: None,
                    },
                    delivery: rusty_hand_types::scheduler::CronDelivery::None,
                    created_at: chrono::Utc::now(),
                    last_run: None,
                    next_run: None,
                };

                match self.kernel.cron_scheduler.add_job(job, false) {
                    Ok(id) => {
                        let id_short = &id.0.to_string()[..8];
                        format!("Job [{id_short}] created: '{cron_expr}' -> {agent_name}: \"{message}\"")
                    }
                    Err(e) => format!("Failed to create job: {e}"),
                }
            }
            "del" => {
                if args.is_empty() {
                    return "Usage: /schedule del <id-prefix>".to_string();
                }
                let prefix = &args[0];
                let jobs = self.kernel.cron_scheduler.list_all_jobs();
                let matched: Vec<_> = jobs
                    .iter()
                    .filter(|j| j.id.0.to_string().starts_with(prefix.as_str()))
                    .collect();
                match matched.len() {
                    0 => format!("No job found matching '{prefix}'."),
                    1 => {
                        let j = matched[0];
                        match self.kernel.cron_scheduler.remove_job(j.id) {
                            Ok(_) => {
                                format!("Job [{}] '{}' removed.", &j.id.0.to_string()[..8], j.name)
                            }
                            Err(e) => format!("Failed to remove job: {e}"),
                        }
                    }
                    n => format!("{n} jobs match '{prefix}'. Be more specific."),
                }
            }
            "run" => {
                if args.is_empty() {
                    return "Usage: /schedule run <id-prefix>".to_string();
                }
                let prefix = &args[0];
                let jobs = self.kernel.cron_scheduler.list_all_jobs();
                let matched: Vec<_> = jobs
                    .iter()
                    .filter(|j| j.id.0.to_string().starts_with(prefix.as_str()))
                    .collect();
                match matched.len() {
                    0 => format!("No job found matching '{prefix}'."),
                    1 => {
                        let j = matched[0];
                        let message = match &j.action {
                            rusty_hand_types::scheduler::CronAction::AgentTurn {
                                message, ..
                            } => message.clone(),
                            rusty_hand_types::scheduler::CronAction::SystemEvent { text } => {
                                text.clone()
                            }
                            rusty_hand_types::scheduler::CronAction::WorkflowRun {
                                input, ..
                            } => input.clone(),
                        };
                        match self.kernel.send_message(j.agent_id, &message).await {
                            Ok(result) => {
                                let id_short = &j.id.0.to_string()[..8];
                                format!("Job [{id_short}] ran:\n{}", result.response)
                            }
                            Err(e) => format!("Failed to run job: {e}"),
                        }
                    }
                    n => format!("{n} jobs match '{prefix}'. Be more specific."),
                }
            }
            _ => "Unknown schedule action. Use: add, del, run".to_string(),
        }
    }

    async fn describe_image(&self, file_path: &str) -> Result<String, String> {
        use rusty_hand_types::media::{MediaAttachment, MediaSource, MediaType};

        let size = tokio::fs::metadata(file_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let ext = std::path::Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        let mime = match ext {
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "bmp" => "image/bmp",
            _ => "image/jpeg",
        };
        let attachment = MediaAttachment {
            media_type: MediaType::Image,
            mime_type: mime.to_string(),
            source: MediaSource::FilePath {
                path: file_path.to_string(),
            },
            size_bytes: size,
        };

        let result = self
            .kernel
            .media_engine
            .describe_image(&attachment)
            .await
            .map_err(|e| format!("Image description failed: {e}"))?;

        Ok(result.description)
    }

    async fn transcribe_audio(&self, file_path: &str) -> Result<String, String> {
        use rusty_hand_types::media::{MediaAttachment, MediaSource, MediaType};

        let path = std::path::Path::new(file_path);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("ogg");
        let mime = match ext {
            "ogg" => "audio/ogg",
            "mp3" | "mpeg" => "audio/mpeg",
            "wav" => "audio/wav",
            "m4a" => "audio/m4a",
            "flac" => "audio/flac",
            "webm" => "audio/webm",
            _ => "audio/ogg",
        };
        let size = tokio::fs::metadata(file_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        let attachment = MediaAttachment {
            media_type: MediaType::Audio,
            mime_type: mime.to_string(),
            source: MediaSource::FilePath {
                path: file_path.to_string(),
            },
            size_bytes: size,
        };

        let result = self
            .kernel
            .media_engine
            .transcribe_audio(&attachment)
            .await
            .map_err(|e| format!("Transcription failed: {e}"))?;

        Ok(result.description)
    }

    async fn list_approval_details(&self) -> Vec<(String, String, String, String)> {
        self.kernel
            .approval_manager
            .list_pending()
            .iter()
            .map(|req| {
                let id_short = req.id.to_string()[..8].to_string();
                let display = if req.action_summary.is_empty() {
                    format!(
                        "Agent **{}** wants to execute **{}**",
                        req.agent_id, req.tool_name
                    )
                } else {
                    format!("Agent **{}**: `{}`", req.agent_id, req.action_summary)
                };
                let emoji = req.risk_level.emoji().to_string();
                (display, id_short, req.tool_name.clone(), emoji)
            })
            .collect()
    }

    async fn list_approvals_text(&self) -> String {
        let pending = self.kernel.approval_manager.list_pending();
        if pending.is_empty() {
            return "No pending approvals.".to_string();
        }
        let mut msg = format!("Pending approvals ({}):\n", pending.len());
        for req in &pending {
            let id_short = &req.id.to_string()[..8];
            let age_secs = (chrono::Utc::now() - req.requested_at).num_seconds();
            let age = if age_secs >= 60 {
                format!("{}m", age_secs / 60)
            } else {
                format!("{age_secs}s")
            };
            msg.push_str(&format!(
                "  [{}] {} — {} ({:?}) age:{}\n",
                id_short, req.agent_id, req.tool_name, req.risk_level, age,
            ));
            if !req.action_summary.is_empty() {
                msg.push_str(&format!("    {}\n", req.action_summary));
            }
        }
        msg.push_str("\nUse /approve <id> or /reject <id>");
        msg
    }

    async fn resolve_approval_text(&self, id_prefix: &str, approve: bool) -> String {
        let pending = self.kernel.approval_manager.list_pending();
        let matched: Vec<_> = pending
            .iter()
            .filter(|r| r.id.to_string().starts_with(id_prefix))
            .collect();
        match matched.len() {
            0 => format!("No pending approval matching '{id_prefix}'."),
            1 => {
                let req = matched[0];
                let decision = if approve {
                    rusty_hand_types::approval::ApprovalDecision::Approved
                } else {
                    rusty_hand_types::approval::ApprovalDecision::Denied
                };
                match self.kernel.approval_manager.resolve(
                    req.id,
                    decision,
                    Some("channel".to_string()),
                ) {
                    Ok(_) => {
                        let verb = if approve { "Approved" } else { "Rejected" };
                        format!(
                            "{} [{}] {} — {}",
                            verb,
                            &req.id.to_string()[..8],
                            req.tool_name,
                            req.agent_id
                        )
                    }
                    Err(e) => format!("Failed to resolve approval: {e}"),
                }
            }
            n => format!("{n} approvals match '{id_prefix}'. Be more specific."),
        }
    }

    async fn reset_session(&self, agent_id: AgentId) -> Result<String, String> {
        self.kernel
            .reset_session(agent_id)
            .map_err(|e| format!("{e}"))?;
        Ok("Session reset. Chat history cleared.".to_string())
    }

    async fn compact_session(&self, agent_id: AgentId) -> Result<String, String> {
        self.kernel
            .compact_agent_session(agent_id)
            .await
            .map_err(|e| format!("{e}"))
    }

    async fn set_model(&self, agent_id: AgentId, model: &str) -> Result<String, String> {
        if model.is_empty() {
            // Show current model
            let entry = self
                .kernel
                .registry
                .get(agent_id)
                .ok_or_else(|| "Agent not found".to_string())?;
            return Ok(format!(
                "Current model: {} (provider: {})",
                entry.manifest.model.model, entry.manifest.model.provider
            ));
        }
        self.kernel
            .set_agent_model(agent_id, model)
            .map_err(|e| format!("{e}"))?;
        Ok(format!("Model switched to: {model}"))
    }

    async fn stop_run(&self, agent_id: AgentId) -> Result<String, String> {
        let cancelled = self
            .kernel
            .stop_agent_run(agent_id)
            .map_err(|e| format!("{e}"))?;
        if cancelled {
            Ok("Run cancelled.".to_string())
        } else {
            Ok("No active run to cancel.".to_string())
        }
    }

    async fn session_usage(&self, agent_id: AgentId) -> Result<String, String> {
        let (input, output, cost) = self
            .kernel
            .session_usage_cost(agent_id)
            .map_err(|e| format!("{e}"))?;
        let total = input + output;
        let mut msg = format!("Session usage:\n  Input: ~{input} tokens\n  Output: ~{output} tokens\n  Total: ~{total} tokens");
        if cost > 0.0 {
            msg.push_str(&format!("\n  Estimated cost: ${cost:.4}"));
        }
        Ok(msg)
    }

    async fn set_thinking(&self, _agent_id: AgentId, on: bool) -> Result<String, String> {
        // Future-ready: stores preference but doesn't affect model behavior yet
        let state = if on { "enabled" } else { "disabled" };
        Ok(format!(
            "Extended thinking {state}. (This will take effect when supported by the model.)"
        ))
    }

    async fn channel_overrides(
        &self,
        channel_type: &str,
    ) -> Option<rusty_hand_types::config::ChannelOverrides> {
        let channels = &self.kernel.config.channels;
        match channel_type {
            "telegram" => channels.telegram.as_ref().map(|c| c.overrides.clone()),
            "discord" => channels.discord.as_ref().map(|c| c.overrides.clone()),
            "slack" => channels.slack.as_ref().map(|c| c.overrides.clone()),
            _ => None,
        }
    }

    async fn authorize_channel_user(
        &self,
        channel_type: &str,
        platform_id: &str,
        action: &str,
    ) -> Result<(), String> {
        if !self.kernel.auth.is_enabled() {
            return Ok(()); // RBAC not configured — allow all
        }

        let user_id = self
            .kernel
            .auth
            .identify(channel_type, platform_id)
            .ok_or_else(|| "Unrecognized user. Contact an admin to get access.".to_string())?;

        let auth_action = match action {
            "chat" => rusty_hand_kernel::auth::Action::ChatWithAgent,
            "spawn" => rusty_hand_kernel::auth::Action::SpawnAgent,
            "kill" => rusty_hand_kernel::auth::Action::KillAgent,
            "install_skill" => rusty_hand_kernel::auth::Action::InstallSkill,
            _ => rusty_hand_kernel::auth::Action::ChatWithAgent,
        };

        self.kernel
            .auth
            .authorize(user_id, &auth_action)
            .map_err(|e| e.to_string())
    }

    async fn record_delivery(
        &self,
        agent_id: AgentId,
        channel: &str,
        recipient: &str,
        success: bool,
        error: Option<&str>,
    ) {
        let receipt = if success {
            rusty_hand_kernel::DeliveryTracker::sent_receipt(channel, recipient)
        } else {
            rusty_hand_kernel::DeliveryTracker::failed_receipt(
                channel,
                recipient,
                error.unwrap_or("Unknown error"),
            )
        };
        self.kernel.delivery_tracker.record(agent_id, receipt);

        // Persist last channel for cron CronDelivery::LastChannel
        if success {
            let kv_val = serde_json::json!({"channel": channel, "recipient": recipient});
            let _ = self
                .kernel
                .memory
                .structured_set(agent_id, "delivery.last_channel", kv_val);
        }
    }

    async fn check_auto_reply(&self, agent_id: AgentId, message: &str) -> Option<String> {
        // Check if auto-reply should fire for this message
        let channel_type = "bridge"; // Generic; the bridge layer handles specifics
        self.kernel
            .auto_reply_engine
            .should_reply(message, channel_type, agent_id)?;
        // Fire auto-reply synchronously (bridge already runs in background task)
        match self.kernel.send_message(agent_id, message).await {
            Ok(result) => Some(result.response),
            Err(e) => {
                tracing::warn!(error = %e, "Auto-reply failed");
                None
            }
        }
    }

    // ── Budget, Network, A2A ──

    async fn budget_text(&self) -> String {
        let budget = self.kernel.budget_config();
        let status = self.kernel.metering.budget_status(&budget);

        let fmt_limit = |v: f64| -> String {
            if v > 0.0 {
                format!("${v:.2}")
            } else {
                "unlimited".to_string()
            }
        };
        let fmt_pct = |pct: f64, limit: f64| -> String {
            if limit > 0.0 {
                format!(" ({:.1}%)", pct * 100.0)
            } else {
                String::new()
            }
        };

        format!(
            "Budget Status:\n\
             \n\
             Hourly:  ${:.4} / {}{}\n\
             Daily:   ${:.4} / {}{}\n\
             Monthly: ${:.4} / {}{}\n\
             \n\
             Alert threshold: {}%",
            status.hourly_spend,
            fmt_limit(status.hourly_limit),
            fmt_pct(status.hourly_pct, status.hourly_limit),
            status.daily_spend,
            fmt_limit(status.daily_limit),
            fmt_pct(status.daily_pct, status.daily_limit),
            status.monthly_spend,
            fmt_limit(status.monthly_limit),
            fmt_pct(status.monthly_pct, status.monthly_limit),
            (status.alert_threshold * 100.0) as u32,
        )
    }

    async fn peers_text(&self) -> String {
        if !self.kernel.config.network_enabled {
            return "RHP peer network is disabled. Set network_enabled = true in config.toml."
                .to_string();
        }
        match self.kernel.peer_registry() {
            Some(registry) => {
                let peers = registry.all_peers();
                if peers.is_empty() {
                    "RHP network enabled but no peers connected.".to_string()
                } else {
                    let mut msg = format!("RHP Peers ({} connected):\n", peers.len());
                    for p in &peers {
                        msg.push_str(&format!(
                            "  {} — {} ({:?})\n",
                            p.node_id, p.address, p.state
                        ));
                    }
                    msg
                }
            }
            None => "RHP peer node not started.".to_string(),
        }
    }

    async fn a2a_agents_text(&self) -> String {
        let agents = self
            .kernel
            .a2a_external_agents
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if agents.is_empty() {
            return "No external A2A agents discovered.\nUse the dashboard or API to discover agents.".to_string();
        }
        let mut msg = format!("External A2A Agents ({}):\n", agents.len());
        for (url, card) in agents.iter() {
            msg.push_str(&format!("  {} — {}\n", card.name, url));
            let desc = &card.description;
            if !desc.is_empty() {
                let short = if desc.len() > 60 {
                    rusty_hand_types::text::truncate_bytes(desc, 60)
                } else {
                    desc.as_str()
                };
                msg.push_str(&format!("    {short}\n"));
            }
        }
        msg
    }
}

/// Parse a trigger pattern string from chat into a `TriggerPattern`.
fn parse_trigger_pattern(s: &str) -> Option<rusty_hand_kernel::triggers::TriggerPattern> {
    use rusty_hand_kernel::triggers::TriggerPattern;
    if let Some(rest) = s.strip_prefix("spawned:") {
        return Some(TriggerPattern::AgentSpawned {
            name_pattern: rest.to_string(),
        });
    }
    if let Some(rest) = s.strip_prefix("system:") {
        return Some(TriggerPattern::SystemKeyword {
            keyword: rest.to_string(),
        });
    }
    if let Some(rest) = s.strip_prefix("memory:") {
        return Some(TriggerPattern::MemoryKeyPattern {
            key_pattern: rest.to_string(),
        });
    }
    if let Some(rest) = s.strip_prefix("match:") {
        return Some(TriggerPattern::ContentMatch {
            substring: rest.to_string(),
        });
    }
    match s {
        "lifecycle" => Some(TriggerPattern::Lifecycle),
        "terminated" => Some(TriggerPattern::AgentTerminated),
        "system" => Some(TriggerPattern::System),
        "memory" => Some(TriggerPattern::MemoryUpdate),
        "all" => Some(TriggerPattern::All),
        _ => None,
    }
}

/// Resolve a default agent by name — find running or spawn from manifest.
async fn resolve_default_agent(
    handle: &KernelBridgeAdapter,
    name: &str,
    router: &mut AgentRouter,
    adapter_name: &str,
) {
    match handle.find_agent_by_name(name).await {
        Ok(Some(agent_id)) => {
            router.set_default(agent_id);
            info!("{adapter_name} default agent: {name} ({agent_id})");
        }
        _ => match handle.spawn_agent_by_name(name).await {
            Ok(agent_id) => {
                router.set_default(agent_id);
                info!("{adapter_name}: spawned default agent {name} ({agent_id})");
            }
            Err(e) => {
                warn!("{adapter_name}: could not find or spawn default agent '{name}': {e}");
            }
        },
    }
}

/// Locate an agent manifest file by name, checking the user's home directory
/// first and falling back to `$RUSTY_HAND_AGENTS_DIR` (set by the Docker
/// image to `/opt/rustyhand/agents`).
///
/// Without the env-var fallback, the Docker daemon couldn't auto-spawn the
/// `default_agent` configured for a channel: bundled manifests live in
/// `/opt/rustyhand/agents/`, but `kernel.config.home_dir` is `/data` and
/// `/data/agents/` is empty on a fresh volume — so every Telegram message
/// returned "No agent assigned" until the user manually copied a manifest
/// or ran `rustyhand init` somewhere.
fn resolve_manifest_path(home_dir: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let local = home_dir.join("agents").join(name).join("agent.toml");
    if local.exists() {
        return Some(local);
    }
    if let Ok(env_dir) = std::env::var("RUSTY_HAND_AGENTS_DIR") {
        if !env_dir.is_empty() {
            let p = std::path::PathBuf::from(env_dir)
                .join(name)
                .join("agent.toml");
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}

/// Read a token from an env var, returning None with a warning if missing/empty.
fn read_token(env_var: &str, adapter_name: &str) -> Option<String> {
    match std::env::var(env_var) {
        Ok(t) if !t.is_empty() => Some(t),
        Ok(_) => {
            warn!("{adapter_name} bot token env var '{env_var}' is empty, skipping");
            None
        }
        Err(_) => {
            warn!("{adapter_name} bot token env var '{env_var}' not set, skipping");
            None
        }
    }
}

/// Start the channel bridge for all configured channels based on kernel config.
///
/// Returns `Some(BridgeManager)` if any channels were configured and started,
/// or `None` if no channels are configured.
pub async fn start_channel_bridge(kernel: Arc<RustyHandKernel>) -> Option<BridgeManager> {
    let channels = kernel.config.channels.clone();
    let (bridge, _names) = start_channel_bridge_with_config(kernel, &channels).await;
    bridge
}

/// Start channels from an explicit `ChannelsConfig` (used by hot-reload).
///
/// Returns `(Option<BridgeManager>, Vec<started_channel_names>)`.
pub async fn start_channel_bridge_with_config(
    kernel: Arc<RustyHandKernel>,
    config: &rusty_hand_types::config::ChannelsConfig,
) -> (Option<BridgeManager>, Vec<String>) {
    let has_any = config.telegram.is_some() || config.discord.is_some() || config.slack.is_some();

    if !has_any {
        return (None, Vec::new());
    }

    let handle = KernelBridgeAdapter {
        kernel: kernel.clone(),
        started_at: Instant::now(),
    };

    // Collect all adapters to start
    let mut adapters: Vec<(Arc<dyn ChannelAdapter>, Option<String>)> = Vec::new();

    // Telegram
    if let Some(ref tg_config) = config.telegram {
        if let Some(token) = read_token(&tg_config.bot_token_env, "Telegram") {
            let poll_interval = Duration::from_secs(tg_config.poll_interval_secs);
            let adapter = Arc::new(TelegramAdapter::new(
                token,
                tg_config.allowed_users.clone(),
                poll_interval,
            ));
            adapters.push((adapter, tg_config.default_agent.clone()));
        }
    }

    // Discord
    if let Some(ref dc_config) = config.discord {
        if let Some(token) = read_token(&dc_config.bot_token_env, "Discord") {
            let adapter = Arc::new(DiscordAdapter::new(
                token,
                dc_config.allowed_guilds.clone(),
                dc_config.intents,
            ));
            adapters.push((adapter, dc_config.default_agent.clone()));
        }
    }

    // Slack
    if let Some(ref sl_config) = config.slack {
        if let Some(app_token) = read_token(&sl_config.app_token_env, "Slack (app)") {
            if let Some(bot_token) = read_token(&sl_config.bot_token_env, "Slack (bot)") {
                let adapter = Arc::new(SlackAdapter::new(
                    app_token,
                    bot_token,
                    sl_config.allowed_channels.clone(),
                ));
                adapters.push((adapter, sl_config.default_agent.clone()));
            }
        }
    }

    if adapters.is_empty() {
        return (None, Vec::new());
    }

    // Resolve default agent from first adapter that has one configured
    let mut router = AgentRouter::new();
    for (_, default_agent) in &adapters {
        if let Some(ref name) = default_agent {
            resolve_default_agent(&handle, name, &mut router, "Channel bridge").await;
            break; // Only need one default
        }
    }

    // Load bindings and broadcast config from kernel
    let bindings = kernel.list_bindings();
    if !bindings.is_empty() {
        // Register all known agents in the router's name cache for binding resolution
        for entry in kernel.registry.list() {
            router.register_agent(entry.name.clone(), entry.id);
        }
        router.load_bindings(&bindings);
        info!(count = bindings.len(), "Loaded agent bindings into router");
    }
    router.load_broadcast(kernel.broadcast.clone());

    let bridge_handle: Arc<dyn ChannelBridgeHandle> = Arc::new(KernelBridgeAdapter {
        kernel: kernel.clone(),
        started_at: Instant::now(),
    });
    let router = Arc::new(router);
    let mut manager = BridgeManager::new(bridge_handle, router.clone());

    let mut started_names = Vec::new();
    let mut started_adapters: Vec<Arc<dyn ChannelAdapter>> = Vec::new();
    for (adapter, _) in adapters {
        let name = adapter.name().to_string();
        match manager.start_adapter(adapter.clone()).await {
            Ok(()) => {
                info!("{name} channel bridge started");
                started_names.push(name);
                started_adapters.push(adapter);
            }
            Err(e) => {
                error!("Failed to start {name} bridge: {e}");
            }
        }
    }

    if started_names.is_empty() {
        (None, Vec::new())
    } else {
        // Wire approval push notifications: when the kernel creates a new
        // approval request, the callback sends it through the channel to
        // the bridge's background task, which pushes inline keyboards to
        // the user's Telegram/Discord/etc. chat.
        let (approval_tx, approval_rx) =
            tokio::sync::mpsc::channel::<rusty_hand_channels::bridge::ApprovalNotification>(32);

        // Collect started adapters for the notifier
        // (re-start is not possible, so we clone the adapter list from the bridge)
        // For simplicity, we only support push notifications on adapters that
        // support inline keyboards. Currently: Telegram.
        // The notifier tries each adapter in turn.
        let started_adapters_clone = started_adapters.clone();
        manager.start_approval_notifier(approval_rx, started_adapters);

        // Register the callback on the kernel's approval manager
        kernel.approval_manager.set_notification_callback(Arc::new(
            move |req: &rusty_hand_types::approval::ApprovalRequest| {
                let notif = rusty_hand_channels::bridge::ApprovalNotification {
                    agent_id: req.agent_id.clone(),
                    id_prefix: req.id.to_string()[..8].to_string(),
                    summary: req.action_summary.clone(),
                    risk_emoji: req.risk_level.emoji().to_string(),
                    timeout_secs: req.timeout_secs,
                };
                let _ = approval_tx.try_send(notif);
            },
        ));

        // Wire autonomous response push: when an agent running in
        // continuous/periodic mode generates a response, push it to the
        // last known Telegram/Discord chat for that agent.
        {
            let adapters_for_push = started_adapters_clone.clone();
            let router_for_push = router.clone();
            kernel.set_autonomous_response_callback(Arc::new(
                move |agent_id: AgentId, response: &str| {
                    let Some((_, platform_id)) =
                        router_for_push.last_sender_for_agent(&agent_id.0.to_string())
                    else {
                        return;
                    };
                    let user = rusty_hand_channels::types::ChannelUser {
                        platform_id,
                        display_name: String::new(),
                        rusty_hand_user: None,
                    };
                    let content =
                        rusty_hand_channels::types::ChannelContent::Text(response.to_string());
                    let adapters = adapters_for_push.clone();
                    let agent_id_str = agent_id.0.to_string();
                    // Fire-and-forget: spawn a task to send the response
                    tokio::spawn(async move {
                        let mut sent = false;
                        for adapter in &adapters {
                            if adapter.send(&user, content.clone()).await.is_ok() {
                                sent = true;
                                break;
                            }
                        }
                        if !sent {
                            tracing::warn!(
                                agent = %agent_id_str,
                                "Failed to push autonomous response — no adapter could deliver"
                            );
                        }
                    });
                },
            ));
        }

        (Some(manager), started_names)
    }
}

/// Reload channels from disk config — stops old bridge, starts new one.
///
/// Reads `config.toml` fresh, rebuilds the channel bridge, and stores it
/// in `AppState.bridge_manager`. Returns the list of started channel names.
pub async fn reload_channels_from_disk(
    state: &crate::routes::AppState,
) -> Result<Vec<String>, String> {
    // Stop existing bridge
    {
        let mut guard = state.bridge_manager.lock().await;
        if let Some(ref mut bridge) = *guard {
            bridge.stop().await;
        }
        *guard = None;
    }

    // Re-read config from disk
    let config_path = state.kernel.config.home_dir.join("config.toml");
    let fresh_config = rusty_hand_kernel::config::load_config(Some(&config_path));

    // Update the live channels config so list_channels() reflects reality
    *state.channels_config.write().await = fresh_config.channels.clone();

    // Start new bridge with fresh channel config
    let (new_bridge, started) =
        start_channel_bridge_with_config(state.kernel.clone(), &fresh_config.channels).await;

    // Store the new bridge
    *state.bridge_manager.lock().await = new_bridge;

    info!(
        started = started.len(),
        channels = ?started,
        "Channel hot-reload complete"
    );

    Ok(started)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_skips_when_no_config() {
        let config = rusty_hand_types::config::KernelConfig::default();
        assert!(config.channels.telegram.is_none());
        assert!(config.channels.discord.is_none());
        assert!(config.channels.slack.is_none());
    }

    /// Regression: the channel bridge must find bundled agent manifests
    /// via `RUSTY_HAND_AGENTS_DIR` even when `home_dir/agents/` is empty.
    /// Before the fix this returned `None` for the Docker case
    /// (home_dir = /data, manifests at /opt/rustyhand/agents/), which
    /// made every Telegram message respond "No agent assigned".
    ///
    /// The three cases are folded into one test because they all mutate
    /// the same process-wide env var; the parallel cargo test runner would
    /// otherwise race them.
    #[test]
    fn resolve_manifest_path_covers_home_env_and_missing() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let bundle = tmp.path().join("bundle");
        std::fs::create_dir_all(home.join("agents")).unwrap();
        std::fs::create_dir_all(bundle.join("assistant")).unwrap();
        std::fs::write(bundle.join("assistant/agent.toml"), "name = \"bundle\"").unwrap();

        // 1. With the env var set and home_dir empty, the bundled manifest
        //    must be found — this is the Docker case.
        std::env::set_var("RUSTY_HAND_AGENTS_DIR", &bundle);
        let env_only = resolve_manifest_path(&home, "assistant");
        assert_eq!(
            env_only.as_deref(),
            Some(bundle.join("assistant/agent.toml").as_path()),
            "RUSTY_HAND_AGENTS_DIR fallback should resolve missing manifests"
        );

        // 2. When the user has customized home_dir/agents/<name>/agent.toml,
        //    that copy must win over the bundle dir.
        std::fs::create_dir_all(home.join("agents/assistant")).unwrap();
        std::fs::write(home.join("agents/assistant/agent.toml"), "name = \"home\"").unwrap();
        let home_wins = resolve_manifest_path(&home, "assistant");
        assert_eq!(
            home_wins.as_deref(),
            Some(home.join("agents/assistant/agent.toml").as_path()),
            "user-customized home_dir manifest must win over the bundle dir"
        );

        // 3. Unknown manifest with the env var still set → None.
        assert!(resolve_manifest_path(&home, "ghost").is_none());

        // 4. With the env var unset and home_dir empty for that name → None.
        std::env::remove_var("RUSTY_HAND_AGENTS_DIR");
        assert!(resolve_manifest_path(&home, "ghost").is_none());
    }
}
