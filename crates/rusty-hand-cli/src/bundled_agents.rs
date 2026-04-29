//! Compile-time embedded agent templates.
//!
//! All 30 bundled agent templates are embedded into the binary via `include_str!`.
//! This ensures `rustyhand agent new` works immediately after install — no filesystem
//! discovery needed.

/// Returns all bundled agent templates as `(name, toml_content)` pairs.
pub fn bundled_agents() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "analyst",
            include_str!("../../../agents/analyst/agent.toml"),
        ),
        (
            "architect",
            include_str!("../../../agents/architect/agent.toml"),
        ),
        (
            "assistant",
            include_str!("../../../agents/assistant/agent.toml"),
        ),
        (
            "capability-builder",
            include_str!("../../../agents/capability-builder/agent.toml"),
        ),
        ("coder", include_str!("../../../agents/coder/agent.toml")),
        (
            "coordinator",
            include_str!("../../../agents/coordinator/agent.toml"),
        ),
        (
            "code-reviewer",
            include_str!("../../../agents/code-reviewer/agent.toml"),
        ),
        (
            "customer-support",
            include_str!("../../../agents/customer-support/agent.toml"),
        ),
        (
            "data-scientist",
            include_str!("../../../agents/data-scientist/agent.toml"),
        ),
        (
            "debugger",
            include_str!("../../../agents/debugger/agent.toml"),
        ),
        (
            "devops-lead",
            include_str!("../../../agents/devops-lead/agent.toml"),
        ),
        (
            "diagnostic",
            include_str!("../../../agents/diagnostic/agent.toml"),
        ),
        (
            "doc-writer",
            include_str!("../../../agents/doc-writer/agent.toml"),
        ),
        (
            "email-assistant",
            include_str!("../../../agents/email-assistant/agent.toml"),
        ),
        (
            "health-tracker",
            include_str!("../../../agents/health-tracker/agent.toml"),
        ),
        (
            "hello-world",
            include_str!("../../../agents/hello-world/agent.toml"),
        ),
        (
            "home-automation",
            include_str!("../../../agents/home-automation/agent.toml"),
        ),
        (
            "legal-assistant",
            include_str!("../../../agents/legal-assistant/agent.toml"),
        ),
        (
            "meeting-assistant",
            include_str!("../../../agents/meeting-assistant/agent.toml"),
        ),
        ("ops", include_str!("../../../agents/ops/agent.toml")),
        (
            "orchestrator",
            include_str!("../../../agents/orchestrator/agent.toml"),
        ),
        (
            "personal-finance",
            include_str!("../../../agents/personal-finance/agent.toml"),
        ),
        (
            "planner",
            include_str!("../../../agents/planner/agent.toml"),
        ),
        (
            "recruiter",
            include_str!("../../../agents/recruiter/agent.toml"),
        ),
        (
            "researcher",
            include_str!("../../../agents/researcher/agent.toml"),
        ),
        (
            "sales-assistant",
            include_str!("../../../agents/sales-assistant/agent.toml"),
        ),
        (
            "security-auditor",
            include_str!("../../../agents/security-auditor/agent.toml"),
        ),
        (
            "social-media",
            include_str!("../../../agents/social-media/agent.toml"),
        ),
        (
            "test-engineer",
            include_str!("../../../agents/test-engineer/agent.toml"),
        ),
        (
            "translator",
            include_str!("../../../agents/translator/agent.toml"),
        ),
        (
            "travel-planner",
            include_str!("../../../agents/travel-planner/agent.toml"),
        ),
        ("tutor", include_str!("../../../agents/tutor/agent.toml")),
        ("writer", include_str!("../../../agents/writer/agent.toml")),
    ]
}

/// Install bundled agent templates to `~/.rustyhand/agents/`.
/// Skips any template that already exists on disk (user customization preserved).
pub fn install_bundled_agents(agents_dir: &std::path::Path) {
    for (name, content) in bundled_agents() {
        let dest_dir = agents_dir.join(name);
        let dest_file = dest_dir.join("agent.toml");
        if dest_file.exists() {
            continue; // Preserve user customization
        }
        if std::fs::create_dir_all(&dest_dir).is_ok() {
            let _ = std::fs::write(&dest_file, content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::agent::AgentManifest;

    /// Regression: every bundled manifest must use a provider that's still
    /// in the v0.7.x catalog. v0.7.0 dropped `groq`, `gemini`, `openai`,
    /// `xai`, `cohere`, etc. — but 27/40 bundled manifests still pointed
    /// at `groq` or `gemini` until v0.7.9. With the entrypoint defaulting
    /// `default_agent = "assistant"` (v0.7.7+), a fresh Docker container
    /// would try to spawn `assistant`, hit "Unknown provider 'groq'" at
    /// `create_driver`, fail to set the channel router's default, and
    /// keep replying "No agent assigned" forever. Pin the supported set
    /// here so the same regression can't sneak back via a future
    /// manifest tweak.
    ///
    /// Also pins `api_key_env` (when set) to the conventional name for
    /// the declared provider — v0.7.9 missed this and shipped manifests
    /// with `provider = "anthropic"` but
    /// `api_key_env = "GEMINI_API_KEY"`, which made the dedicated driver
    /// instantiate without a real key and silently return empty
    /// responses on every request. Also pins `[[fallback_models]]`
    /// entries the same way.
    #[test]
    fn every_bundled_manifest_uses_a_supported_provider() {
        const SUPPORTED: &[&str] = &[
            "anthropic",
            "kimi",
            "deepseek",
            "minimax",
            "zhipu",
            "openrouter",
            "ollama",
        ];
        // The conventional env var that goes with each provider. If a
        // manifest sets api_key_env explicitly, it must match — otherwise
        // the driver gets created with a key that doesn't exist and the
        // call silently fails.
        fn conventional_env(provider: &str) -> &'static str {
            match provider {
                "anthropic" => "ANTHROPIC_API_KEY",
                "kimi" => "KIMI_API_KEY",
                "deepseek" => "DEEPSEEK_API_KEY",
                "minimax" => "MINIMAX_API_KEY",
                "zhipu" => "ZHIPU_API_KEY",
                "openrouter" => "OPENROUTER_API_KEY",
                "ollama" => "",
                _ => "",
            }
        }

        let mut bad_provider: Vec<(String, String)> = Vec::new();
        let mut bad_key: Vec<(String, String, String, String)> = Vec::new();

        for (name, toml_src) in bundled_agents() {
            let m: AgentManifest = toml::from_str(toml_src)
                .unwrap_or_else(|e| panic!("{name} template should parse: {e}"));

            // Primary model.
            if !SUPPORTED.contains(&m.model.provider.as_str()) {
                bad_provider.push((name.to_string(), m.model.provider.clone()));
                continue;
            }
            if let Some(ref key_env) = m.model.api_key_env {
                let expected = conventional_env(&m.model.provider);
                if !expected.is_empty() && key_env != expected {
                    bad_key.push((
                        name.to_string(),
                        "[model]".to_string(),
                        m.model.provider.clone(),
                        key_env.clone(),
                    ));
                }
            }

            // Fallback models — same constraint.
            for (i, fb) in m.fallback_models.iter().enumerate() {
                if !SUPPORTED.contains(&fb.provider.as_str()) {
                    bad_provider.push((format!("{name}.fallback[{i}]"), fb.provider.clone()));
                    continue;
                }
                if let Some(ref key_env) = fb.api_key_env {
                    let expected = conventional_env(&fb.provider);
                    if !expected.is_empty() && key_env != expected {
                        bad_key.push((
                            name.to_string(),
                            format!("[[fallback_models]] #{i}"),
                            fb.provider.clone(),
                            key_env.clone(),
                        ));
                    }
                }
            }
        }

        assert!(
            bad_provider.is_empty(),
            "Bundled manifests reference removed providers: {bad_provider:?}. \
             v0.7.0 dropped groq/gemini/openai/etc.; switch to anthropic \
             (the kernel default) or another supported provider: {SUPPORTED:?}"
        );
        assert!(
            bad_key.is_empty(),
            "Bundled manifests have api_key_env that doesn't match the \
             declared provider: {bad_key:?}. The dedicated driver would \
             read a non-existent env var and silently fail with empty \
             responses (the v0.7.9 → v0.7.10 regression that broke every \
             Telegram reply)."
        );
    }

    #[test]
    fn meta_agents_parse_and_use_anthropic() {
        for name in &["coordinator", "capability-builder", "diagnostic"] {
            let toml_src = bundled_agents()
                .into_iter()
                .find(|(n, _)| n == name)
                .unwrap_or_else(|| panic!("{name} template should exist"))
                .1;
            let m: AgentManifest = toml::from_str(toml_src)
                .unwrap_or_else(|e| panic!("{name} template should parse: {e}"));
            assert_eq!(m.name, *name, "{name}: name mismatch");
            assert_eq!(
                m.model.provider, "anthropic",
                "{name}: meta-agents default to Anthropic Sonnet for routing quality"
            );
            assert!(
                m.tags.iter().any(|t| t == "meta"),
                "{name}: should be tagged 'meta' for discovery"
            );
        }
    }

    #[test]
    fn coordinator_can_delegate_to_other_agents() {
        let coord = bundled_agents()
            .into_iter()
            .find(|(n, _)| n == &"coordinator")
            .unwrap()
            .1;
        let m: AgentManifest = toml::from_str(coord).unwrap();
        // Coordinator's whole job is routing — must have agent_send + agent_list.
        assert!(m.capabilities.tools.iter().any(|t| t == "agent_send"));
        assert!(m.capabilities.tools.iter().any(|t| t == "agent_list"));
        // Wildcard message permission so it can talk to any specialist.
        assert!(m.capabilities.agent_message.iter().any(|a| a == "*"));
    }

    #[test]
    fn capability_builder_can_write_skills() {
        let cb = bundled_agents()
            .into_iter()
            .find(|(n, _)| n == &"capability-builder")
            .unwrap()
            .1;
        let m: AgentManifest = toml::from_str(cb).unwrap();
        assert!(m.capabilities.tools.iter().any(|t| t == "file_write"));
        assert!(m.capabilities.tools.iter().any(|t| t == "shell_exec"));
        assert!(m.capabilities.tools.iter().any(|t| t == "web_search"));
        // The whole point of this agent: it can install skills end-to-end.
        assert!(
            m.capabilities.tools.iter().any(|t| t == "skill_install"),
            "capability-builder must have skill_install for autonomous skill creation"
        );
    }

    #[test]
    fn diagnostic_reads_audit_but_does_not_modify() {
        let diag = bundled_agents()
            .into_iter()
            .find(|(n, _)| n == &"diagnostic")
            .unwrap()
            .1;
        let m: AgentManifest = toml::from_str(diag).unwrap();
        // Diagnostic uses self_history/self_metrics (real builtin tools)
        // and web_fetch to localhost for the kernel audit API.
        assert!(m.capabilities.tools.iter().any(|t| t == "self_history"));
        assert!(m.capabilities.tools.iter().any(|t| t == "web_fetch"));
        // Read-only: no shell_exec, no agent_spawn.
        assert!(!m.capabilities.tools.iter().any(|t| t == "shell_exec"));
        assert!(!m.capabilities.agent_spawn);
    }

    #[test]
    fn test_assistant_template_has_scheduler_tools() {
        let assistant = bundled_agents()
            .into_iter()
            .find(|(name, _)| *name == "assistant")
            .expect("assistant template should exist")
            .1;

        let manifest: AgentManifest =
            toml::from_str(assistant).expect("assistant template should parse");

        assert!(manifest
            .capabilities
            .tools
            .iter()
            .any(|t| t == "cron_create"));
        assert!(manifest.capabilities.tools.iter().any(|t| t == "cron_list"));
        assert!(manifest
            .capabilities
            .tools
            .iter()
            .any(|t| t == "cron_cancel"));
    }
}
