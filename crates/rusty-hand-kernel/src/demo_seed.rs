//! Demo Mode resource seeding.
//!
//! On first boot in demo mode (provider = `mock`, no agents in registry,
//! marker file absent), seeds four sample resources so every major
//! dashboard page has content to interact with on first visit:
//!
//! - **`rusty`** welcome agent (chat-ready)
//! - **`demo-pipeline`** workflow (2-step sample, click to run)
//! - sample agent-spawn trigger (narrow pattern, won't fire on meta-agents)
//! - **`demo-daily-ping`** cron job (registered but disabled by default)
//!
//! After successful seeding, writes a marker file at
//! `~/.rustyhand/.rustyhand_demo_seeded` so the seeder doesn't re-create
//! samples the user has deliberately deleted. Re-seeding requires both
//! deleting the marker AND having an empty agent registry.
//!
//! All four resources are persistent (per the engines that own them) and
//! survive daemon restart. None of them depend on a live LLM — every
//! response goes through the mock driver.

use crate::kernel::RustyHandKernel;
use rusty_hand_types::scheduler::{CronAction, CronDelivery, CronJob, CronJobId, CronSchedule};
use tracing::{info, warn};

/// Seed demo resources iff:
///   1. provider = `mock` (i.e. demo mode active), AND
///   2. registry has no agents, AND
///   3. `.rustyhand_demo_seeded` marker file does not exist.
///
/// Failures of individual sub-steps are logged and skipped — the welcome
/// agent is the must-have; the workflow/trigger/cron are bonuses. Agent
/// spawn failure aborts the whole seed (no marker is written, so the next
/// boot re-attempts).
pub(crate) fn seed_if_demo_mode(kernel: &RustyHandKernel) {
    if kernel.config.default_model.provider != "mock" {
        return;
    }
    if !kernel.registry.list().is_empty() {
        return;
    }
    let marker = kernel.config.home_dir.join(".rustyhand_demo_seeded");
    if marker.exists() {
        return;
    }

    let agent_id = match kernel.spawn_agent(welcome_agent_manifest()) {
        Ok(id) => id,
        Err(e) => {
            warn!(error = %e, "Failed to spawn demo welcome agent");
            return;
        }
    };

    // Workflow: bonus, non-fatal on failure.
    kernel.workflows.register_blocking(sample_workflow_def());

    // Trigger: bonus, non-fatal on failure (registration is infallible).
    kernel.triggers.register(
        agent_id,
        crate::triggers::TriggerPattern::AgentSpawned {
            name_pattern: "demo-".to_string(),
        },
        "A new demo agent was spawned: {{event}}".to_string(),
        0, // unlimited fires
    );

    // Cron job: bonus, disabled by default so the user isn't surprised
    // by an unexpected fire while exploring.
    let sample_cron = sample_cron_job(agent_id);
    if let Err(e) = kernel.cron_scheduler.add_job(sample_cron, false) {
        warn!(error = %e, "Failed to register sample cron job in demo mode");
    } else if let Err(e) = kernel.cron_scheduler.persist() {
        warn!(error = %e, "Failed to persist sample cron job");
    }

    // Marker last — only after the welcome agent (the must-have) succeeded.
    // Track whether the marker write succeeded so we can adjust the final
    // info-level message: a failed marker write means the next boot will
    // re-seed (creating duplicate resources), so we shouldn't tell the
    // operator "they will not respawn."
    let marker_written = match std::fs::write(&marker, b"") {
        Ok(()) => true,
        Err(e) => {
            warn!(
                error = %e,
                "Could not write demo-seeded marker — the next boot will re-seed and \
                 create duplicate resources unless you fix the file permissions"
            );
            false
        }
    };

    if marker_written {
        info!(
            agent_id = %agent_id,
            "Demo Mode: seeded `rusty` agent + `demo-pipeline` workflow + sample trigger + \
             disabled `demo-daily-ping` cron job (delete any of them if unwanted; they will \
             not respawn on next boot)"
        );
    } else {
        info!(
            agent_id = %agent_id,
            "Demo Mode: seeded resources (warning: marker file could not be written, so \
             the next boot WILL re-seed and produce duplicates)"
        );
    }
}

fn welcome_agent_manifest() -> rusty_hand_types::agent::AgentManifest {
    rusty_hand_types::agent::AgentManifest {
        name: "rusty".to_string(),
        version: "0.1.0".to_string(),
        description: "Welcome agent — RustyHand demo mode".to_string(),
        author: "rusty-hand".to_string(),
        module: "builtin:chat".to_string(),
        tags: vec!["demo".to_string(), "welcome".to_string()],
        model: rusty_hand_types::agent::ModelConfig {
            provider: "mock".to_string(),
            model: "mock-model".to_string(),
            max_tokens: 1024,
            temperature: 0.5,
            system_prompt: WELCOME_SYSTEM_PROMPT.to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

const WELCOME_SYSTEM_PROMPT: &str = "You are Rusty, the welcome agent for RustyHand's demo \
    mode. The user is running RustyHand without an LLM API key, so every reply will \
    appear with a `[mock]` prefix — that's expected. The dashboard, audit log, workflow \
    engine, and trigger engine are all real and persistent; only the LLM responses are \
    stubbed. Be brief, be friendly, and point users to set ANTHROPIC_API_KEY (or any \
    other supported provider) when they want real responses.";

fn sample_workflow_def() -> crate::workflow::Workflow {
    use crate::workflow::{ErrorMode, StepAgent, StepMode, Workflow, WorkflowId, WorkflowStep};

    let step = |name: &str, prompt: &str| WorkflowStep {
        name: name.to_string(),
        agent: StepAgent::ByName {
            name: "rusty".to_string(),
        },
        prompt_template: prompt.to_string(),
        mode: StepMode::Sequential,
        timeout_secs: 30,
        error_mode: ErrorMode::Fail,
        output_var: None,
    };

    Workflow {
        id: WorkflowId::new(),
        name: "demo-pipeline".to_string(),
        description: "Sample 2-step pipeline showing how output from one agent feeds into \
                      the next. Try running it from the Workflows page."
            .to_string(),
        steps: vec![
            step("summarize", "Summarize this in one sentence: {{input}}"),
            step("translate", "Translate this summary to French: {{input}}"),
        ],
        created_at: chrono::Utc::now(),
    }
}

fn sample_cron_job(agent_id: rusty_hand_types::agent::AgentId) -> CronJob {
    CronJob {
        id: CronJobId::new(),
        agent_id,
        name: "demo-daily-ping".to_string(),
        enabled: false,
        schedule: CronSchedule::Cron {
            expr: "0 0 * * *".to_string(),
            tz: None,
        },
        action: CronAction::AgentTurn {
            message: "Good morning! Anything to report?".to_string(),
            model_override: None,
            timeout_secs: Some(30),
        },
        delivery: CronDelivery::None,
        created_at: chrono::Utc::now(),
        last_run: None,
        next_run: None,
    }
}
