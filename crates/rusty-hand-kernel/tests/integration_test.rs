//! Integration test: boot kernel -> spawn agent -> send message via Kimi Code API.
//!
//! Run with: KIMI_API_KEY=sk-kimi-... cargo test -p rusty-hand-kernel --test integration_test -- --nocapture
//!
//! Kimi was chosen as the default for this test because it's one of the two
//! first-class coding providers in v0.7.0 and has a generous free tier in dev mode.

use rusty_hand_kernel::RustyHandKernel;
use rusty_hand_types::agent::AgentManifest;
use rusty_hand_types::config::{DefaultModelConfig, KernelConfig};

fn test_config() -> KernelConfig {
    let tmp = std::env::temp_dir().join("rusty-hand-integration-test");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();

    KernelConfig {
        home_dir: tmp.clone(),
        data_dir: tmp.join("data"),
        default_model: DefaultModelConfig {
            provider: "kimi".to_string(),
            model: "kimi-for-coding".to_string(),
            api_key_env: "KIMI_API_KEY".to_string(),
            base_url: None,
        },
        ..KernelConfig::default()
    }
}

#[tokio::test]
async fn test_full_pipeline_with_kimi() {
    if std::env::var("KIMI_API_KEY").is_err() {
        eprintln!("KIMI_API_KEY not set, skipping integration test");
        return;
    }

    // Boot kernel
    let config = test_config();
    let kernel = RustyHandKernel::boot_with_config(config).expect("Kernel should boot");

    // Spawn agent
    let manifest: AgentManifest = toml::from_str(
        r#"
name = "test-agent"
version = "0.1.0"
description = "Integration test agent"
author = "test"
module = "builtin:chat"

[model]
provider = "kimi"
model = "kimi-for-coding"
system_prompt = "You are a test agent. Reply concisely in one sentence."

[capabilities]
tools = ["file_read"]
memory_read = ["*"]
memory_write = ["self.*"]
"#,
    )
    .unwrap();

    let agent_id = kernel.spawn_agent(manifest).expect("Agent should spawn");

    // Send message
    let result = kernel
        .send_message(agent_id, "Say hello in exactly 5 words.")
        .await
        .expect("Message should get a response");

    println!("\n=== AGENT RESPONSE ===");
    println!("{}", result.response);
    println!(
        "=== USAGE: {} tokens in, {} tokens out, {} iterations ===",
        result.total_usage.input_tokens, result.total_usage.output_tokens, result.iterations
    );

    assert!(!result.response.is_empty(), "Response should not be empty");
    assert!(
        result.total_usage.input_tokens > 0,
        "Should have used tokens"
    );

    // Kill agent
    kernel.kill_agent(agent_id).expect("Agent should be killed");
    kernel.shutdown();
}

#[tokio::test]
async fn test_multiple_agents_different_models() {
    if std::env::var("KIMI_API_KEY").is_err() {
        eprintln!("KIMI_API_KEY not set, skipping integration test");
        return;
    }

    let config = test_config();
    let kernel = RustyHandKernel::boot_with_config(config).expect("Kernel should boot");

    // Spawn agent 1: Kimi Code (frontier-tier coding model)
    let manifest1: AgentManifest = toml::from_str(
        r#"
name = "agent-kimi"
version = "0.1.0"
description = "Kimi agent"
author = "test"
module = "builtin:chat"

[model]
provider = "kimi"
model = "kimi-for-coding"
system_prompt = "You are Agent A. Always start your reply with 'A:'."

[capabilities]
memory_read = ["*"]
memory_write = ["self.*"]
"#,
    )
    .unwrap();

    // Spawn agent 2: Kimi via the k2-thinking alias (server-side alias, same
    // backend). Exercises catalog alias resolution end-to-end.
    let manifest2: AgentManifest = toml::from_str(
        r#"
name = "agent-k2"
version = "0.1.0"
description = "Kimi via alias"
author = "test"
module = "builtin:chat"

[model]
provider = "kimi"
model = "kimi-k2-thinking"
system_prompt = "You are Agent B. Always start your reply with 'B:'."

[capabilities]
memory_read = ["*"]
memory_write = ["self.*"]
"#,
    )
    .unwrap();

    let id1 = kernel.spawn_agent(manifest1).expect("Agent 1 should spawn");
    let id2 = kernel.spawn_agent(manifest2).expect("Agent 2 should spawn");

    let r1 = kernel
        .send_message(id1, "Say hello briefly.")
        .await
        .expect("A should reply");
    let r2 = kernel
        .send_message(id2, "Say hello briefly.")
        .await
        .expect("B should reply");

    println!("Agent A said: {}", r1.response);
    println!("Agent B said: {}", r2.response);

    assert!(!r1.response.is_empty());
    assert!(!r2.response.is_empty());

    kernel.kill_agent(id1).expect("Agent 1 should be killed");
    kernel.kill_agent(id2).expect("Agent 2 should be killed");
    kernel.shutdown();
}
