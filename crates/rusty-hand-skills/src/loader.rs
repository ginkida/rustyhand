//! Skill loader — loads and executes skills from various runtimes.

use crate::{SkillError, SkillManifest, SkillRuntime, SkillToolResult};
use std::path::Path;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error};

/// Maximum bytes captured from a skill's stdout/stderr (each, independently).
/// A malicious or buggy skill could otherwise stream unbounded output and OOM
/// the host. Once the cap is exceeded we stop reading and kill the child.
const MAX_SKILL_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

/// Execute a skill tool by spawning the appropriate runtime.
pub async fn execute_skill_tool(
    manifest: &SkillManifest,
    skill_dir: &Path,
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<SkillToolResult, SkillError> {
    // Verify the tool exists in the manifest
    let _tool_def = manifest
        .tools
        .provided
        .iter()
        .find(|t| t.name == tool_name)
        .ok_or_else(|| SkillError::NotFound(format!("Tool {tool_name} not in skill manifest")))?;

    match manifest.runtime.runtime_type {
        SkillRuntime::Python => {
            execute_python(skill_dir, &manifest.runtime.entry, tool_name, input).await
        }
        SkillRuntime::Node => {
            execute_node(skill_dir, &manifest.runtime.entry, tool_name, input).await
        }
        // Unreachable in practice: the registry refuses WASM skills at load
        // time (see registry::load_skill), so a WASM skill never registers a
        // callable tool. Kept as a defensive fallback with the same guidance.
        SkillRuntime::Wasm => Err(SkillError::RuntimeNotAvailable(
            "WASM skills are not supported in this build — reimplement as a Python or Node skill"
                .to_string(),
        )),
        SkillRuntime::Builtin => Err(SkillError::RuntimeNotAvailable(
            "Builtin skills are handled by the kernel directly".to_string(),
        )),
        SkillRuntime::PromptOnly => {
            // Prompt-only skills inject context into the system prompt.
            // When a tool call arrives here, guide the LLM to use built-in tools.
            Ok(SkillToolResult {
                output: serde_json::json!({
                    "note": "Prompt-context skill — instructions are in your system prompt. Use built-in tools directly."
                }),
                is_error: false,
            })
        }
    }
}

/// Execute a Python skill script.
async fn execute_python(
    skill_dir: &Path,
    entry: &str,
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<SkillToolResult, SkillError> {
    let script_path = skill_dir.join(entry);
    if !script_path.exists() {
        return Err(SkillError::ExecutionFailed(format!(
            "Python script not found: {}",
            script_path.display()
        )));
    }

    // Build the JSON payload to send via stdin
    let payload = serde_json::json!({
        "tool": tool_name,
        "input": input,
    });

    let python = find_python().ok_or_else(|| {
        SkillError::RuntimeNotAvailable(
            "Python not found. Install Python 3.8+ to run Python skills.".to_string(),
        )
    })?;

    debug!(
        "Executing Python skill: {} {}",
        python,
        script_path.display()
    );

    let mut cmd = tokio::process::Command::new(&python);
    cmd.arg(&script_path)
        .current_dir(skill_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Reap the child if this future is dropped/cancelled (timeout) instead
        // of leaving it orphaned.
        .kill_on_drop(true);

    // SECURITY: Isolate environment to prevent secret leakage.
    // Skills are third-party code — they must not inherit API keys,
    // tokens, or credentials from the host environment.
    cmd.env_clear();
    // Preserve PATH for binary resolution and platform essentials
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    #[cfg(windows)]
    {
        if let Ok(sp) = std::env::var("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", sp);
        }
        if let Ok(tmp) = std::env::var("TEMP") {
            cmd.env("TEMP", tmp);
        }
    }
    // Python needs PYTHONIOENCODING for UTF-8 output
    cmd.env("PYTHONIOENCODING", "utf-8");

    let child = cmd
        .spawn()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to spawn Python: {e}")))?;

    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| SkillError::ExecutionFailed(format!("JSON serialize: {e}")))?;

    // Write stdin concurrently with draining bounded stdout/stderr, then wait.
    let (stdout_bytes, stderr_bytes, status) = drive_skill_process(child, payload_bytes).await?;

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        error!("Python skill failed: {stderr}");
        return Ok(SkillToolResult {
            output: serde_json::json!({ "error": stderr.to_string() }),
            is_error: true,
        });
    }

    // Parse stdout as JSON
    let stdout = String::from_utf8_lossy(&stdout_bytes);
    match serde_json::from_str::<serde_json::Value>(&stdout) {
        Ok(value) => Ok(SkillToolResult {
            output: value,
            is_error: false,
        }),
        Err(_) => Ok(SkillToolResult {
            output: serde_json::json!({ "result": stdout.trim() }),
            is_error: false,
        }),
    }
}

/// Execute a Node.js skill script.
async fn execute_node(
    skill_dir: &Path,
    entry: &str,
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<SkillToolResult, SkillError> {
    let script_path = skill_dir.join(entry);
    if !script_path.exists() {
        return Err(SkillError::ExecutionFailed(format!(
            "Node.js script not found: {}",
            script_path.display()
        )));
    }

    let node = find_node().ok_or_else(|| {
        SkillError::RuntimeNotAvailable(
            "Node.js not found. Install Node.js 18+ to run Node skills.".to_string(),
        )
    })?;

    let payload = serde_json::json!({
        "tool": tool_name,
        "input": input,
    });

    debug!(
        "Executing Node.js skill: {} {}",
        node,
        script_path.display()
    );

    let mut cmd = tokio::process::Command::new(&node);
    cmd.arg(&script_path)
        .current_dir(skill_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Reap the child if this future is dropped/cancelled (timeout) instead
        // of leaving it orphaned.
        .kill_on_drop(true);

    // SECURITY: Isolate environment (same as Python — prevent secret leakage)
    cmd.env_clear();
    if let Ok(path) = std::env::var("PATH") {
        cmd.env("PATH", path);
    }
    if let Ok(home) = std::env::var("HOME") {
        cmd.env("HOME", home);
    }
    #[cfg(windows)]
    {
        if let Ok(sp) = std::env::var("SYSTEMROOT") {
            cmd.env("SYSTEMROOT", sp);
        }
        if let Ok(tmp) = std::env::var("TEMP") {
            cmd.env("TEMP", tmp);
        }
    }
    // Node needs NODE_PATH sometimes
    cmd.env("NODE_NO_WARNINGS", "1");

    let child = cmd
        .spawn()
        .map_err(|e| SkillError::ExecutionFailed(format!("Failed to spawn Node.js: {e}")))?;

    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|e| SkillError::ExecutionFailed(format!("JSON serialize: {e}")))?;

    // Write stdin concurrently with draining bounded stdout/stderr, then wait.
    let (stdout_bytes, stderr_bytes, status) = drive_skill_process(child, payload_bytes).await?;

    if !status.success() {
        let stderr = String::from_utf8_lossy(&stderr_bytes);
        return Ok(SkillToolResult {
            output: serde_json::json!({ "error": stderr.to_string() }),
            is_error: true,
        });
    }

    let stdout = String::from_utf8_lossy(&stdout_bytes);
    match serde_json::from_str::<serde_json::Value>(&stdout) {
        Ok(value) => Ok(SkillToolResult {
            output: value,
            is_error: false,
        }),
        Err(_) => Ok(SkillToolResult {
            output: serde_json::json!({ "result": stdout.trim() }),
            is_error: false,
        }),
    }
}

/// Read from `reader` into `buf` until EOF or `MAX_SKILL_OUTPUT_BYTES`,
/// whichever comes first. Returns `Ok(true)` if the cap was hit (output was
/// truncated), `Ok(false)` otherwise.
async fn read_capped<R>(reader: Option<R>, buf: &mut Vec<u8>) -> std::io::Result<bool>
where
    R: tokio::io::AsyncRead + Unpin,
{
    use tokio::io::AsyncReadExt;
    let Some(mut reader) = reader else {
        return Ok(false);
    };
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader.read(&mut chunk).await?;
        if n == 0 {
            return Ok(false);
        }
        let remaining = MAX_SKILL_OUTPUT_BYTES.saturating_sub(buf.len());
        if n > remaining {
            buf.extend_from_slice(&chunk[..remaining]);
            return Ok(true);
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// Drive a spawned skill process to completion.
///
/// Writes the JSON `payload_bytes` to the child's stdin on a separate task so
/// it runs *concurrently* with draining stdout/stderr — large payloads
/// (>~64 KiB) would otherwise deadlock against a skill that writes stdout while
/// still reading stdin. stdout/stderr are each capped at
/// `MAX_SKILL_OUTPUT_BYTES`; if either exceeds the cap, the child is killed and
/// its captured output is marked as truncated.
///
/// Returns `(stdout, stderr, exit_status)`.
async fn drive_skill_process(
    mut child: tokio::process::Child,
    payload_bytes: Vec<u8>,
) -> Result<(Vec<u8>, Vec<u8>, std::process::ExitStatus), SkillError> {
    // Spawn the stdin writer so it cannot deadlock against output reads.
    let stdin_task = child.stdin.take().map(|mut stdin| {
        tokio::spawn(async move {
            // Best-effort: a skill may legitimately exit before consuming all
            // input. Dropping `stdin` afterwards closes the pipe (EOF).
            let _ = stdin.write_all(&payload_bytes).await;
        })
    });

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    let (out_res, err_res) = tokio::join!(
        read_capped(stdout, &mut stdout_buf),
        read_capped(stderr, &mut stderr_buf),
    );
    let out_truncated =
        out_res.map_err(|e| SkillError::ExecutionFailed(format!("Read stdout: {e}")))?;
    let err_truncated =
        err_res.map_err(|e| SkillError::ExecutionFailed(format!("Read stderr: {e}")))?;

    // Ensure the stdin writer has finished (it may already have).
    if let Some(task) = stdin_task {
        let _ = task.await;
    }

    // A skill that floods output past the cap is misbehaving — kill it so
    // `wait()` returns promptly, and surface the truncation in stderr. The kill
    // forces a non-success exit status, so callers report it via the error path.
    if out_truncated || err_truncated {
        let _ = child.kill().await;
        stderr_buf.extend_from_slice(
            format!("\n[rustyhand: skill output truncated at {MAX_SKILL_OUTPUT_BYTES} bytes]")
                .as_bytes(),
        );
    }

    let status = child
        .wait()
        .await
        .map_err(|e| SkillError::ExecutionFailed(format!("Wait for child: {e}")))?;

    Ok((stdout_buf, stderr_buf, status))
}

/// Find Python 3 binary.
fn find_python() -> Option<String> {
    for name in &["python3", "python"] {
        if std::process::Command::new(name)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok()
        {
            return Some(name.to_string());
        }
    }
    None
}

/// Find Node.js binary.
fn find_node() -> Option<String> {
    if std::process::Command::new("node")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
    {
        return Some("node".to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_python() {
        // Just ensure it doesn't panic — result depends on environment
        let _ = find_python();
    }

    #[test]
    fn test_find_node() {
        let _ = find_node();
    }

    #[tokio::test]
    async fn test_prompt_only_execution() {
        use crate::{
            SkillManifest, SkillMeta, SkillRequirements, SkillRuntimeConfig, SkillToolDef,
            SkillTools,
        };
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let manifest = SkillManifest {
            skill: SkillMeta {
                name: "test-prompt".to_string(),
                version: "0.1.0".to_string(),
                description: "A prompt-only test".to_string(),
                author: String::new(),
                license: String::new(),
                tags: vec![],
            },
            runtime: SkillRuntimeConfig {
                runtime_type: SkillRuntime::PromptOnly,
                entry: String::new(),
            },
            tools: SkillTools {
                provided: vec![SkillToolDef {
                    name: "test_tool".to_string(),
                    description: "Test".to_string(),
                    input_schema: serde_json::json!({"type": "object"}),
                }],
            },
            requirements: SkillRequirements::default(),
            prompt_context: Some("You are a helpful assistant.".to_string()),
            source: None,
        };

        let result = execute_skill_tool(&manifest, dir.path(), "test_tool", &serde_json::json!({}))
            .await
            .unwrap();
        assert!(!result.is_error);
        let note = result.output["note"].as_str().unwrap();
        assert!(note.contains("system prompt"));
    }
}
