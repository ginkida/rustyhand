//! Event-driven agent triggers — agents auto-activate when events match patterns.
//!
//! Agents register triggers that describe which events should wake them.
//! When a matching event arrives on the EventBus, the trigger system
//! sends the event content as a message to the subscribing agent.
//!
//! Persistence: when constructed via [`TriggerEngine::with_persistence`],
//! triggers are loaded from a JSON file at startup and re-saved (atomic
//! `.tmp` + rename) after every mutation. Webhook-style triggers therefore
//! survive daemon restart instead of being silently dropped.

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use rusty_hand_types::agent::AgentId;
use rusty_hand_types::event::{Event, EventPayload, LifecycleEvent, SystemEvent};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Unique identifier for a trigger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TriggerId(pub Uuid);

impl TriggerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TriggerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TriggerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// What kind of events a trigger matches on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerPattern {
    /// Match any lifecycle event (agent spawned, started, terminated, etc.).
    Lifecycle,
    /// Match when a specific agent is spawned.
    AgentSpawned { name_pattern: String },
    /// Match when any agent is terminated.
    AgentTerminated,
    /// Match any system event.
    System,
    /// Match a specific system event by keyword.
    SystemKeyword { keyword: String },
    /// Match any memory update event.
    MemoryUpdate,
    /// Match memory updates for a specific key pattern.
    MemoryKeyPattern { key_pattern: String },
    /// Match all events (wildcard).
    All,
    /// Match custom events by content substring.
    ContentMatch { substring: String },
}

/// A registered trigger definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trigger {
    /// Unique trigger ID.
    pub id: TriggerId,
    /// Which agent owns this trigger.
    pub agent_id: AgentId,
    /// The event pattern to match.
    pub pattern: TriggerPattern,
    /// Prompt template to send when triggered. Use `{{event}}` for event description.
    pub prompt_template: String,
    /// Whether this trigger is currently active.
    pub enabled: bool,
    /// When this trigger was created.
    pub created_at: DateTime<Utc>,
    /// How many times this trigger has fired.
    pub fire_count: u64,
    /// Maximum number of times this trigger can fire (0 = unlimited).
    pub max_fires: u64,
}

/// The trigger engine manages event-to-agent routing.
pub struct TriggerEngine {
    /// All registered triggers.
    triggers: DashMap<TriggerId, Trigger>,
    /// Index: agent_id → list of trigger IDs belonging to that agent.
    agent_triggers: DashMap<AgentId, Vec<TriggerId>>,
    /// Optional persistence file. When set, every mutation (register, remove,
    /// set_enabled, fire-count increment) saves the full trigger list to
    /// `<path>` via atomic write.
    persist_path: Option<PathBuf>,
}

impl TriggerEngine {
    /// Create a new in-memory trigger engine (no disk persistence).
    pub fn new() -> Self {
        Self {
            triggers: DashMap::new(),
            agent_triggers: DashMap::new(),
            persist_path: None,
        }
    }

    /// Create a trigger engine that loads from `path` on construction and
    /// saves there on every mutation. If the file does not exist, the engine
    /// starts empty. If the file exists but is unreadable or malformed, a
    /// warning is logged and the engine starts empty (so a corrupt file
    /// can't block kernel boot).
    pub fn with_persistence(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        let engine = Self {
            triggers: DashMap::new(),
            agent_triggers: DashMap::new(),
            persist_path: Some(path.clone()),
        };

        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(data) => match serde_json::from_str::<Vec<Trigger>>(&data) {
                    Ok(loaded) => {
                        let count = loaded.len();
                        for t in loaded {
                            let id = t.id;
                            let agent_id = t.agent_id;
                            engine.triggers.insert(id, t);
                            engine.agent_triggers.entry(agent_id).or_default().push(id);
                        }
                        info!(count, path = %path.display(), "Loaded triggers from disk");
                    }
                    Err(e) => {
                        warn!(
                            path = %path.display(),
                            error = %e,
                            "Failed to parse triggers file — starting empty"
                        );
                    }
                },
                Err(e) => {
                    warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to read triggers file — starting empty"
                    );
                }
            }
        }

        engine
    }

    /// Persist the current trigger set to disk. No-op if no persist_path is
    /// set. Logs (but does not return) any I/O errors so callers don't have
    /// to plumb them through.
    fn persist(&self) {
        let path = match &self.persist_path {
            Some(p) => p,
            None => return,
        };
        let triggers: Vec<Trigger> = self.triggers.iter().map(|e| e.value().clone()).collect();
        let data = match serde_json::to_string_pretty(&triggers) {
            Ok(d) => d,
            Err(e) => {
                warn!(error = %e, "Failed to serialize triggers");
                return;
            }
        };
        let tmp = path.with_extension("json.tmp");
        if let Err(e) = std::fs::write(&tmp, data.as_bytes()) {
            warn!(path = %tmp.display(), error = %e, "Failed to write triggers temp file");
            return;
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            warn!(path = %path.display(), error = %e, "Failed to rename triggers file");
        }
    }

    /// Register a new trigger.
    pub fn register(
        &self,
        agent_id: AgentId,
        pattern: TriggerPattern,
        prompt_template: String,
        max_fires: u64,
    ) -> TriggerId {
        let trigger = Trigger {
            id: TriggerId::new(),
            agent_id,
            pattern,
            prompt_template,
            enabled: true,
            created_at: Utc::now(),
            fire_count: 0,
            max_fires,
        };
        let id = trigger.id;
        self.triggers.insert(id, trigger);
        self.agent_triggers.entry(agent_id).or_default().push(id);
        self.persist();

        info!(trigger_id = %id, agent_id = %agent_id, "Trigger registered");
        id
    }

    /// Remove a trigger.
    pub fn remove(&self, trigger_id: TriggerId) -> bool {
        if let Some((_, trigger)) = self.triggers.remove(&trigger_id) {
            if let Some(mut list) = self.agent_triggers.get_mut(&trigger.agent_id) {
                list.retain(|id| *id != trigger_id);
            }
            self.persist();
            true
        } else {
            false
        }
    }

    /// Remove all triggers for an agent.
    pub fn remove_agent_triggers(&self, agent_id: AgentId) {
        if let Some((_, trigger_ids)) = self.agent_triggers.remove(&agent_id) {
            for id in trigger_ids {
                self.triggers.remove(&id);
            }
            self.persist();
        }
    }

    /// Enable or disable a trigger. Returns true if the trigger was found.
    pub fn set_enabled(&self, trigger_id: TriggerId, enabled: bool) -> bool {
        if let Some(mut t) = self.triggers.get_mut(&trigger_id) {
            t.enabled = enabled;
            drop(t);
            self.persist();
            true
        } else {
            false
        }
    }

    /// List all triggers for an agent.
    pub fn list_agent_triggers(&self, agent_id: AgentId) -> Vec<Trigger> {
        self.agent_triggers
            .get(&agent_id)
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| self.triggers.get(id).map(|t| t.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// List all registered triggers.
    pub fn list_all(&self) -> Vec<Trigger> {
        self.triggers.iter().map(|e| e.value().clone()).collect()
    }

    /// Evaluate an event against all triggers. Returns a list of
    /// (agent_id, message_to_send) pairs for matching triggers.
    pub fn evaluate(&self, event: &Event) -> Vec<(AgentId, String)> {
        let event_description = describe_event(event);
        let mut matches = Vec::new();
        let mut mutated = false;

        for mut entry in self.triggers.iter_mut() {
            let trigger = entry.value_mut();

            if !trigger.enabled {
                continue;
            }

            // Check max fires
            if trigger.max_fires > 0 && trigger.fire_count >= trigger.max_fires {
                trigger.enabled = false;
                mutated = true;
                continue;
            }

            if matches_pattern(&trigger.pattern, event, &event_description) {
                let message = trigger
                    .prompt_template
                    .replace("{{event}}", &event_description);
                matches.push((trigger.agent_id, message));
                trigger.fire_count += 1;
                mutated = true;

                debug!(
                    trigger_id = %trigger.id,
                    agent_id = %trigger.agent_id,
                    fire_count = trigger.fire_count,
                    "Trigger fired"
                );
            }
        }

        // Persist fire-count and auto-disable updates so they survive a
        // restart. A trigger that's reached max_fires must NOT re-fire on
        // boot, and `fire_count` is user-visible (CLI / dashboard show it).
        if mutated {
            self.persist();
        }

        matches
    }

    /// Get a trigger by ID.
    pub fn get(&self, trigger_id: TriggerId) -> Option<Trigger> {
        self.triggers.get(&trigger_id).map(|t| t.clone())
    }
}

impl Default for TriggerEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if an event matches a trigger pattern.
///
/// Empty substring/keyword/pattern fields treat the trigger as
/// "match nothing" — the previous `str.contains("")` semantics
/// vacuously returned true, so an operator who accidentally
/// registered `MemoryKeyPattern { key_pattern: "" }` (e.g. by
/// typing the bare prefix `"memory:"` in the proactive-condition
/// parser, or by submitting an unfilled UI form) silently created
/// a "fires on every memory update" trigger. Use `"*"` as the
/// explicit wildcard.
fn matches_pattern(pattern: &TriggerPattern, event: &Event, description: &str) -> bool {
    match pattern {
        TriggerPattern::All => true,
        TriggerPattern::Lifecycle => {
            matches!(event.payload, EventPayload::Lifecycle(_))
        }
        TriggerPattern::AgentSpawned { name_pattern } => {
            if let EventPayload::Lifecycle(LifecycleEvent::Spawned { name, .. }) = &event.payload {
                if name_pattern == "*" {
                    true
                } else if name_pattern.is_empty() {
                    false
                } else {
                    name.contains(name_pattern.as_str())
                }
            } else {
                false
            }
        }
        TriggerPattern::AgentTerminated => matches!(
            event.payload,
            EventPayload::Lifecycle(LifecycleEvent::Terminated { .. })
                | EventPayload::Lifecycle(LifecycleEvent::Crashed { .. })
        ),
        TriggerPattern::System => {
            matches!(event.payload, EventPayload::System(_))
        }
        TriggerPattern::SystemKeyword { keyword } => {
            if let EventPayload::System(se) = &event.payload {
                if keyword.is_empty() {
                    return false;
                }
                let se_str = format!("{:?}", se).to_lowercase();
                se_str.contains(&keyword.to_lowercase())
            } else {
                false
            }
        }
        TriggerPattern::MemoryUpdate => {
            matches!(event.payload, EventPayload::MemoryUpdate(_))
        }
        TriggerPattern::MemoryKeyPattern { key_pattern } => {
            if let EventPayload::MemoryUpdate(delta) = &event.payload {
                if key_pattern == "*" {
                    true
                } else if key_pattern.is_empty() {
                    false
                } else {
                    delta.key.contains(key_pattern.as_str())
                }
            } else {
                false
            }
        }
        TriggerPattern::ContentMatch { substring } => {
            if substring.is_empty() {
                return false;
            }
            description
                .to_lowercase()
                .contains(&substring.to_lowercase())
        }
    }
}

/// Create a human-readable description of an event for use in prompts.
fn describe_event(event: &Event) -> String {
    match &event.payload {
        EventPayload::Message(msg) => {
            format!("Message from {:?}: {}", msg.role, msg.content)
        }
        EventPayload::ToolResult(tr) => {
            // Tool outputs can contain arbitrary text — file_read of a
            // Cyrillic doc, web_search results in any language, browser
            // page text. Naive `&s[..200]` panics when byte 200 lands
            // inside a multi-byte UTF-8 char. describe_event is called
            // from the trigger evaluation hot path; one oversized
            // non-ASCII tool result would crash the kernel's event
            // dispatch task.
            let preview = rusty_hand_types::text::truncate_bytes(&tr.content, 200);
            format!(
                "Tool '{}' {} ({}ms): {}",
                tr.tool_id,
                if tr.success { "succeeded" } else { "failed" },
                tr.execution_time_ms,
                preview
            )
        }
        EventPayload::MemoryUpdate(delta) => {
            format!(
                "Memory {:?} on key '{}' for agent {}",
                delta.operation, delta.key, delta.agent_id
            )
        }
        EventPayload::Lifecycle(le) => match le {
            LifecycleEvent::Spawned { agent_id, name } => {
                format!("Agent '{name}' (id: {agent_id}) was spawned")
            }
            LifecycleEvent::Started { agent_id } => {
                format!("Agent {agent_id} started")
            }
            LifecycleEvent::Suspended { agent_id } => {
                format!("Agent {agent_id} suspended")
            }
            LifecycleEvent::Resumed { agent_id } => {
                format!("Agent {agent_id} resumed")
            }
            LifecycleEvent::Terminated { agent_id, reason } => {
                format!("Agent {agent_id} terminated: {reason}")
            }
            LifecycleEvent::Crashed { agent_id, error } => {
                format!("Agent {agent_id} crashed: {error}")
            }
        },
        EventPayload::Network(ne) => {
            format!("Network event: {:?}", ne)
        }
        EventPayload::System(se) => match se {
            SystemEvent::KernelStarted => "Kernel started".to_string(),
            SystemEvent::KernelStopping => "Kernel stopping".to_string(),
            SystemEvent::QuotaWarning {
                agent_id,
                resource,
                usage_percent,
            } => format!("Quota warning: agent {agent_id}, {resource} at {usage_percent:.1}%"),
            SystemEvent::HealthCheck { status } => {
                format!("Health check: {status}")
            }
            SystemEvent::QuotaEnforced {
                agent_id,
                spent,
                limit,
            } => {
                format!("Quota enforced: agent {agent_id}, spent ${spent:.4} / ${limit:.4}")
            }
            SystemEvent::ModelRouted {
                agent_id,
                complexity,
                model,
            } => {
                format!("Model routed: agent {agent_id}, complexity={complexity}, model={model}")
            }
            SystemEvent::UserAction {
                user_id,
                action,
                result,
            } => {
                format!("User action: {user_id} {action} -> {result}")
            }
            SystemEvent::HealthCheckFailed {
                agent_id,
                unresponsive_secs,
            } => {
                format!(
                    "Health check failed: agent {agent_id}, unresponsive for {unresponsive_secs}s"
                )
            }
        },
        EventPayload::Custom(data) => {
            format!("Custom event ({} bytes)", data.len())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::event::*;

    #[test]
    fn test_register_trigger() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();
        let id = engine.register(
            agent_id,
            TriggerPattern::All,
            "Event occurred: {{event}}".to_string(),
            0,
        );
        assert!(engine.get(id).is_some());
    }

    #[test]
    fn test_evaluate_lifecycle() {
        let engine = TriggerEngine::new();
        let watcher = AgentId::new();
        engine.register(
            watcher,
            TriggerPattern::Lifecycle,
            "Lifecycle: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: AgentId::new(),
                name: "new-agent".to_string(),
            }),
        );

        let matches = engine.evaluate(&event);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].0, watcher);
        assert!(matches[0].1.contains("new-agent"));
    }

    #[test]
    fn test_evaluate_agent_spawned_pattern() {
        let engine = TriggerEngine::new();
        let watcher = AgentId::new();
        engine.register(
            watcher,
            TriggerPattern::AgentSpawned {
                name_pattern: "coder".to_string(),
            },
            "Coder spawned: {{event}}".to_string(),
            0,
        );

        // This should match
        let event = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: AgentId::new(),
                name: "coder".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);

        // This should NOT match
        let event2 = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: AgentId::new(),
                name: "researcher".to_string(),
            }),
        );
        assert_eq!(engine.evaluate(&event2).len(), 0);
    }

    #[test]
    fn test_max_fires() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();
        engine.register(
            agent_id,
            TriggerPattern::All,
            "Event: {{event}}".to_string(),
            2, // max 2 fires
        );

        let event = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::System(SystemEvent::HealthCheck {
                status: "ok".to_string(),
            }),
        );

        // First two should match
        assert_eq!(engine.evaluate(&event).len(), 1);
        assert_eq!(engine.evaluate(&event).len(), 1);
        // Third should not
        assert_eq!(engine.evaluate(&event).len(), 0);
    }

    #[test]
    fn test_remove_trigger() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();
        let id = engine.register(agent_id, TriggerPattern::All, "msg".to_string(), 0);
        assert!(engine.remove(id));
        assert!(engine.get(id).is_none());
    }

    #[test]
    fn test_remove_agent_triggers() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();
        engine.register(agent_id, TriggerPattern::All, "a".to_string(), 0);
        engine.register(agent_id, TriggerPattern::System, "b".to_string(), 0);
        assert_eq!(engine.list_agent_triggers(agent_id).len(), 2);

        engine.remove_agent_triggers(agent_id);
        assert_eq!(engine.list_agent_triggers(agent_id).len(), 0);
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triggers.json");

        let agent_a = AgentId::new();
        let agent_b = AgentId::new();

        // First lifetime: register two triggers, fire one of them.
        let id_a;
        let id_b;
        let fire_count_a;
        {
            let engine = TriggerEngine::with_persistence(&path);
            id_a = engine.register(
                agent_a,
                TriggerPattern::All,
                "all: {{event}}".to_string(),
                0,
            );
            id_b = engine.register(
                agent_b,
                TriggerPattern::Lifecycle,
                "lifecycle: {{event}}".to_string(),
                0,
            );

            let event = Event::new(
                AgentId::new(),
                EventTarget::Broadcast,
                EventPayload::System(SystemEvent::HealthCheck {
                    status: "ok".to_string(),
                }),
            );
            engine.evaluate(&event); // matches All only — fires id_a
            engine.evaluate(&event); // fires id_a again
            fire_count_a = engine.get(id_a).unwrap().fire_count;
            assert_eq!(fire_count_a, 2);
        }

        // File must exist after persistence.
        assert!(path.exists(), "triggers file should exist after register");

        // Second lifetime: a fresh engine reads them back, including fire_count.
        let engine = TriggerEngine::with_persistence(&path);
        assert_eq!(engine.list_all().len(), 2);
        let restored_a = engine.get(id_a).expect("trigger A should reload");
        let restored_b = engine.get(id_b).expect("trigger B should reload");
        assert_eq!(restored_a.fire_count, fire_count_a);
        assert_eq!(restored_b.fire_count, 0);
        assert_eq!(restored_a.agent_id, agent_a);
        assert_eq!(restored_b.agent_id, agent_b);

        // The agent_triggers index must also be rebuilt.
        let by_a = engine.list_agent_triggers(agent_a);
        assert_eq!(by_a.len(), 1);
        assert_eq!(by_a[0].id, id_a);
    }

    #[test]
    fn test_persistence_remove_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triggers.json");
        let agent = AgentId::new();

        let engine = TriggerEngine::with_persistence(&path);
        let id = engine.register(agent, TriggerPattern::All, "x".to_string(), 0);
        assert!(engine.remove(id));
        drop(engine);

        let reloaded = TriggerEngine::with_persistence(&path);
        assert!(reloaded.get(id).is_none());
        assert!(reloaded.list_all().is_empty());
    }

    #[test]
    fn test_persistence_max_fires_disabled_state_persists() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triggers.json");
        let agent = AgentId::new();
        let id;
        {
            let engine = TriggerEngine::with_persistence(&path);
            id = engine.register(agent, TriggerPattern::All, "x".to_string(), 1);
            let event = Event::new(
                AgentId::new(),
                EventTarget::Broadcast,
                EventPayload::System(SystemEvent::HealthCheck {
                    status: "ok".to_string(),
                }),
            );
            engine.evaluate(&event); // fires once
            engine.evaluate(&event); // hits max_fires, sets enabled=false
            assert!(!engine.get(id).unwrap().enabled);
        }

        // After restart the trigger must remain disabled — otherwise a
        // restart would silently let max_fires=1 triggers fire a second time.
        let reloaded = TriggerEngine::with_persistence(&path);
        let restored = reloaded.get(id).expect("trigger should reload");
        assert!(!restored.enabled);
        assert_eq!(restored.fire_count, 1);
    }

    #[test]
    fn test_persistence_corrupt_file_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("triggers.json");
        std::fs::write(&path, "this is not valid JSON").unwrap();

        // Corrupt file should not panic or refuse to construct — engine
        // starts empty so the kernel can still boot.
        let engine = TriggerEngine::with_persistence(&path);
        assert!(engine.list_all().is_empty());

        // After registering a fresh trigger, the corrupt file is overwritten
        // with valid JSON.
        let agent = AgentId::new();
        engine.register(agent, TriggerPattern::All, "x".to_string(), 0);
        let data = std::fs::read_to_string(&path).unwrap();
        let parsed: Vec<Trigger> = serde_json::from_str(&data).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_content_match() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();
        engine.register(
            agent_id,
            TriggerPattern::ContentMatch {
                substring: "quota".to_string(),
            },
            "Alert: {{event}}".to_string(),
            0,
        );

        let event = Event::new(
            AgentId::new(),
            EventTarget::System,
            EventPayload::System(SystemEvent::QuotaWarning {
                agent_id: AgentId::new(),
                resource: "tokens".to_string(),
                usage_percent: 85.0,
            }),
        );
        assert_eq!(engine.evaluate(&event).len(), 1);
    }

    /// Regression: `str.contains("")` returns true vacuously, so the
    /// previous implementation silently turned an empty `name_pattern`
    /// / `keyword` / `key_pattern` / `substring` into a wildcard.
    /// An operator who typed the bare prefix `"memory:"` in the
    /// proactive-condition parser (background.rs:235) or who left a
    /// trigger-form field empty in the dashboard registered a trigger
    /// that fired on every event of the matching family. Now empty
    /// fields explicitly match nothing — operators get an obviously-
    /// non-firing trigger they can spot, instead of a noisy fire-on-
    /// every-event trigger that quietly spams the audit log. Use the
    /// explicit `"*"` for wildcards.
    #[test]
    fn empty_field_patterns_match_nothing() {
        let engine = TriggerEngine::new();
        let agent_id = AgentId::new();

        // Empty AgentSpawned name_pattern — previously matched every spawn.
        engine.register(
            agent_id,
            TriggerPattern::AgentSpawned {
                name_pattern: String::new(),
            },
            "{{event}}".to_string(),
            0,
        );
        let spawn = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: AgentId::new(),
                name: "any-agent".to_string(),
            }),
        );
        assert_eq!(
            engine.evaluate(&spawn).len(),
            0,
            "empty name_pattern must match nothing (use \"*\" for wildcard)"
        );

        // Empty ContentMatch — previously matched every event.
        let agent2 = AgentId::new();
        engine.register(
            agent2,
            TriggerPattern::ContentMatch {
                substring: String::new(),
            },
            "{{event}}".to_string(),
            0,
        );
        let any = Event::new(
            AgentId::new(),
            EventTarget::System,
            EventPayload::System(SystemEvent::QuotaWarning {
                agent_id: AgentId::new(),
                resource: "tokens".to_string(),
                usage_percent: 85.0,
            }),
        );
        // ContentMatch + AgentSpawned with empty fields → still no extra fires.
        assert_eq!(
            engine.evaluate(&any).len(),
            0,
            "empty ContentMatch substring must match nothing"
        );

        // Explicit "*" wildcard still works.
        let agent3 = AgentId::new();
        engine.register(
            agent3,
            TriggerPattern::AgentSpawned {
                name_pattern: "*".to_string(),
            },
            "{{event}}".to_string(),
            0,
        );
        let spawn_again = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::Lifecycle(LifecycleEvent::Spawned {
                agent_id: AgentId::new(),
                name: "anything".to_string(),
            }),
        );
        assert_eq!(
            engine.evaluate(&spawn_again).len(),
            1,
            "explicit \"*\" wildcard must still match every spawn"
        );
    }

    /// Regression: `describe_event` used `&tr.content[..tr.content.len().min(200)]`
    /// to truncate ToolResult content for the activation prompt — naive
    /// byte slicing that panics when byte 200 lands inside a multi-byte
    /// UTF-8 char. Tool outputs commonly contain non-ASCII (file_read of
    /// a Cyrillic doc, web_search results in any language, browser page
    /// text). One oversized non-ASCII tool result crashes the kernel's
    /// event-dispatch task that publishes ToolResult events through the
    /// trigger engine.
    #[test]
    fn describe_event_handles_multibyte_tool_output_at_truncation_boundary() {
        // 1-byte ASCII prefix shifts the 2-byte Я sequence onto odd
        // byte offsets so byte 200 lands inside a Я (its continuation
        // byte), not on a boundary.
        let blob = format!("a{}", "Я".repeat(120));
        assert!(blob.len() > 200);
        assert!(
            !blob.is_char_boundary(200),
            "test setup must place byte 200 mid-char"
        );

        let event = Event::new(
            AgentId::new(),
            EventTarget::Broadcast,
            EventPayload::ToolResult(ToolOutput {
                tool_id: "file_read".to_string(),
                tool_use_id: "tu-1".to_string(),
                content: blob,
                success: true,
                execution_time_ms: 12,
            }),
        );

        // Pre-fix: panic inside `&tr.content[..200]`.
        let desc = describe_event(&event);
        assert!(desc.contains("file_read"));
        assert!(desc.contains("succeeded"));
        assert!(
            std::str::from_utf8(desc.as_bytes()).is_ok(),
            "description must be valid UTF-8 even when truncated"
        );
    }
}
