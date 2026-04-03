//! Browser automation via `agent-browser` CLI.
//!
//! Replaces the old Python/Playwright bridge with the native Rust `agent-browser`
//! daemon. Each agent gets an isolated session (`--session {agent_id}`), and the
//! daemon manages browser lifecycle automatically.
//!
//! # Security
//! - SSRF check runs in Rust *before* sending navigate commands
//! - Subprocess launched with cleared environment (only essential vars)
//! - All page content wrapped with `wrap_external_content()` markers
//! - Session limits: max concurrent via DashMap tracking

use dashmap::DashMap;
use rusty_hand_types::config::BrowserConfig;
use tracing::{debug, info};

// ── Manager ─────────────────────────────────────────────────────────────────

/// Manages browser sessions for all agents via `agent-browser` CLI.
pub struct BrowserManager {
    /// Track which agents have active sessions (for cleanup & limit enforcement).
    active_sessions: DashMap<String, ()>,
    config: BrowserConfig,
}

impl BrowserManager {
    /// Create a new BrowserManager with the given configuration.
    pub fn new(config: BrowserConfig) -> Self {
        Self {
            active_sessions: DashMap::new(),
            config,
        }
    }

    /// Check if `agent-browser` is available on PATH (or at configured path).
    pub fn is_available(&self) -> bool {
        std::process::Command::new(&self.config.executable)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Run an `agent-browser` command for a specific agent session.
    /// Returns parsed JSON output on success.
    pub async fn run_command(
        &self,
        agent_id: &str,
        args: &[&str],
    ) -> Result<serde_json::Value, String> {
        // Enforce session limit on first command for a new agent
        if !self.active_sessions.contains_key(agent_id) {
            if self.active_sessions.len() >= self.config.max_sessions {
                return Err(format!(
                    "Maximum browser sessions reached ({}). Close an existing session first.",
                    self.config.max_sessions
                ));
            }
            self.active_sessions.insert(agent_id.to_string(), ());
        }

        let mut cmd = tokio::process::Command::new(&self.config.executable);
        cmd.arg("--session").arg(agent_id);
        cmd.arg("--json");

        if !self.config.headless {
            cmd.arg("--headed");
        }

        cmd.args(args);

        // SECURITY: Isolate environment — clear everything, pass through only essentials
        cmd.env_clear();
        let passthrough_vars = Self::safe_env_vars();
        for var in &passthrough_vars {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }
        // Pass agent-browser-specific env vars through
        for var in &[
            "AGENT_BROWSER_EXECUTABLE_PATH",
            "AGENT_BROWSER_DEFAULT_TIMEOUT",
        ] {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }

        debug!(agent_id, args = ?args, "Running agent-browser command");

        let output = cmd.output().await.map_err(|e| {
            format!("Failed to run agent-browser: {e}. Install it with: npm install -g agent-browser")
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() && stdout.trim().is_empty() {
            return Err(format!(
                "agent-browser exited with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        // agent-browser --json always returns JSON on stdout
        let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
            format!(
                "Failed to parse agent-browser output: {e}\nstdout: {stdout}\nstderr: {stderr}"
            )
        })?;

        // Check for error in the JSON response
        if parsed["success"].as_bool() == Some(false) {
            let err = parsed["error"]
                .as_str()
                .unwrap_or("Unknown agent-browser error");
            return Err(err.to_string());
        }

        Ok(parsed)
    }

    /// Close an agent's browser session.
    pub async fn close_session(&self, agent_id: &str) {
        if self.active_sessions.remove(agent_id).is_some() {
            let result = self.run_command_raw(agent_id, &["close"]).await;
            match result {
                Ok(_) => info!(agent_id, "Browser session closed"),
                Err(e) => debug!(agent_id, error = %e, "Browser session close (may already be closed)"),
            }
        }
    }

    /// Clean up an agent's browser session (called after agent loop ends).
    pub async fn cleanup_agent(&self, agent_id: &str) {
        self.close_session(agent_id).await;
    }

    /// Run a command without session limit checking (for close/cleanup).
    async fn run_command_raw(
        &self,
        agent_id: &str,
        args: &[&str],
    ) -> Result<(), String> {
        let mut cmd = tokio::process::Command::new(&self.config.executable);
        cmd.arg("--session").arg(agent_id);
        cmd.args(args);
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        cmd.env_clear();
        for var in &Self::safe_env_vars() {
            if let Ok(v) = std::env::var(var) {
                cmd.env(var, v);
            }
        }

        let status = cmd.status().await.map_err(|e| e.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("agent-browser exited with {status}"))
        }
    }

    /// Environment variables safe to pass through to agent-browser subprocess.
    fn safe_env_vars() -> Vec<&'static str> {
        let mut vars = vec!["PATH", "HOME", "LANG", "TERM"];
        #[cfg(windows)]
        {
            vars.extend_from_slice(&[
                "SYSTEMROOT",
                "TEMP",
                "TMP",
                "USERPROFILE",
                "APPDATA",
                "LOCALAPPDATA",
            ]);
        }
        #[cfg(not(windows))]
        {
            vars.extend_from_slice(&["TMPDIR", "XDG_CACHE_HOME", "XDG_DATA_HOME"]);
        }
        vars
    }
}

// ── Tool handler functions ──────────────────────────────────────────────────

/// browser_navigate — Navigate to a URL. SSRF-checked in Rust before delegating.
pub async fn tool_browser_navigate(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let url = input["url"].as_str().ok_or("Missing 'url' parameter")?;

    // SECURITY: SSRF check in Rust before sending to browser
    crate::web_fetch::check_ssrf(url)?;

    let resp = mgr.run_command(agent_id, &["open", url]).await?;

    // After navigation, get a snapshot for the agent
    let snapshot = mgr
        .run_command(agent_id, &["snapshot", "-i", "-c"])
        .await
        .ok();

    let title = resp["data"]["title"]
        .as_str()
        .or_else(|| resp["title"].as_str())
        .unwrap_or("(no title)");
    let page_url = resp["data"]["url"]
        .as_str()
        .or_else(|| resp["url"].as_str())
        .unwrap_or(url);

    let snapshot_text = snapshot
        .and_then(|s| {
            s["data"]["snapshot"]
                .as_str()
                .or_else(|| s["snapshot"].as_str())
                .map(|t| t.to_string())
        })
        .unwrap_or_default();

    // Wrap with external content markers
    let wrapped = crate::web_content::wrap_external_content(page_url, &snapshot_text);

    Ok(format!(
        "Navigated to: {page_url}\nTitle: {title}\n\n{wrapped}\n\nUse @ref selectors (e.g., @e1, @e2) from the snapshot above to interact with elements."
    ))
}

/// browser_snapshot — Get accessibility tree with element refs. Best for AI interaction.
pub async fn tool_browser_snapshot(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let mut args = vec!["snapshot"];

    // Default to interactive + compact for AI use
    let interactive = input["interactive"].as_bool().unwrap_or(true);
    let compact = input["compact"].as_bool().unwrap_or(true);
    let selector = input["selector"].as_str();
    let depth_str;

    if interactive {
        args.push("-i");
    }
    if compact {
        args.push("-c");
    }
    if let Some(sel) = selector {
        args.push("-s");
        args.push(sel);
    }
    if let Some(depth) = input["depth"].as_u64() {
        depth_str = depth.to_string();
        args.push("-d");
        args.push(&depth_str);
    }

    let resp = mgr.run_command(agent_id, &args).await?;

    let snapshot = resp["data"]["snapshot"]
        .as_str()
        .or_else(|| resp["snapshot"].as_str())
        .or_else(|| resp["data"].as_str())
        .unwrap_or("(empty snapshot)");

    Ok(format!(
        "Accessibility snapshot (use @e1, @e2, etc. refs to interact):\n\n{snapshot}"
    ))
}

/// browser_click — Click an element by ref (@e1) or CSS selector.
pub async fn tool_browser_click(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let selector = input["selector"]
        .as_str()
        .ok_or("Missing 'selector' parameter")?;

    let resp = mgr.run_command(agent_id, &["click", selector]).await?;

    let _ = resp; // click returns minimal data
    Ok(format!("Clicked: {selector}"))
}

/// browser_type — Fill text into an input field.
pub async fn tool_browser_type(
    input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    let selector = input["selector"]
        .as_str()
        .ok_or("Missing 'selector' parameter")?;
    let text = input["text"].as_str().ok_or("Missing 'text' parameter")?;

    // agent-browser uses "fill" for clear+type
    let resp = mgr
        .run_command(agent_id, &["fill", selector, text])
        .await?;

    let _ = resp;
    Ok(format!("Typed into {selector}: {text}"))
}

/// browser_screenshot — Take a screenshot of the current page.
pub async fn tool_browser_screenshot(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    // Save to our uploads dir so it's accessible via /api/uploads/
    let upload_dir = std::env::temp_dir().join("rusty_hand_uploads");
    let _ = std::fs::create_dir_all(&upload_dir);
    let file_id = uuid::Uuid::new_v4().to_string();
    let file_path = upload_dir.join(format!("{file_id}.png"));
    let file_path_str = file_path.to_string_lossy().to_string();

    let resp = mgr
        .run_command(agent_id, &["screenshot", &file_path_str])
        .await?;

    // Verify the file was created
    let image_urls = if file_path.exists() {
        vec![format!("/api/uploads/{file_id}.png")]
    } else {
        // Try to get the path from response
        let saved_path = resp["data"]["path"]
            .as_str()
            .or_else(|| resp["path"].as_str());
        if let Some(p) = saved_path {
            // Copy from agent-browser's temp to our uploads
            let dest = upload_dir.join(format!("{file_id}.png"));
            if std::fs::copy(p, &dest).is_ok() {
                vec![format!("/api/uploads/{file_id}.png")]
            } else {
                vec![]
            }
        } else {
            vec![]
        }
    };

    let result = serde_json::json!({
        "screenshot": true,
        "image_urls": image_urls,
    });

    Ok(result.to_string())
}

/// browser_read_page — Read the current page content.
pub async fn tool_browser_read_page(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    // Get page title and URL
    let title_resp = mgr.run_command(agent_id, &["get", "title"]).await.ok();
    let url_resp = mgr.run_command(agent_id, &["get", "url"]).await.ok();

    let title = title_resp
        .as_ref()
        .and_then(|r| {
            r["data"]["title"]
                .as_str()
                .or_else(|| r["data"].as_str())
                .or_else(|| r["title"].as_str())
        })
        .unwrap_or("(no title)");
    let url = url_resp
        .as_ref()
        .and_then(|r| {
            r["data"]["url"]
                .as_str()
                .or_else(|| r["data"].as_str())
                .or_else(|| r["url"].as_str())
        })
        .unwrap_or("");

    // Get the interactive snapshot (better for AI than raw HTML)
    let snapshot_resp = mgr
        .run_command(agent_id, &["snapshot", "-i", "-c"])
        .await?;

    let content = snapshot_resp["data"]["snapshot"]
        .as_str()
        .or_else(|| snapshot_resp["snapshot"].as_str())
        .or_else(|| snapshot_resp["data"].as_str())
        .unwrap_or("(empty page)");

    let wrapped = crate::web_content::wrap_external_content(url, content);

    Ok(format!(
        "Page: {title}\nURL: {url}\n\n{wrapped}\n\nUse @ref selectors (e.g., @e1, @e2) to interact with elements."
    ))
}

/// browser_close — Close the browser session for this agent.
pub async fn tool_browser_close(
    _input: &serde_json::Value,
    mgr: &BrowserManager,
    agent_id: &str,
) -> Result<String, String> {
    mgr.close_session(agent_id).await;
    Ok("Browser session closed.".to_string())
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_config_defaults() {
        let config = BrowserConfig::default();
        assert!(config.headless);
        assert_eq!(config.viewport_width, 1280);
        assert_eq!(config.viewport_height, 720);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 300);
        assert_eq!(config.max_sessions, 5);
        assert_eq!(config.executable, "agent-browser");
    }

    #[test]
    fn test_browser_manager_new() {
        let config = BrowserConfig::default();
        let mgr = BrowserManager::new(config);
        assert!(mgr.active_sessions.is_empty());
    }

    #[test]
    fn test_safe_env_vars_include_path() {
        let vars = BrowserManager::safe_env_vars();
        assert!(vars.contains(&"PATH"));
        assert!(vars.contains(&"HOME"));
    }
}
