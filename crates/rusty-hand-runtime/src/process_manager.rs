//! Interactive process manager — persistent process sessions.
//!
//! Allows agents to start long-running processes (REPLs, servers, watchers),
//! write to their stdin, read from stdout/stderr, and kill them.

use dashmap::DashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Unique process identifier.
pub type ProcessId = String;

/// A managed persistent process.
struct ManagedProcess {
    /// stdin writer.
    stdin: Option<tokio::process::ChildStdin>,
    /// Accumulated stdout output.
    stdout_buf: Arc<Mutex<Vec<String>>>,
    /// Accumulated stderr output.
    stderr_buf: Arc<Mutex<Vec<String>>>,
    /// The child process handle.
    child: tokio::process::Child,
    /// Agent that owns this process.
    agent_id: String,
    /// Command that was started.
    command: String,
    /// When the process was started.
    started_at: std::time::Instant,
}

/// Process info for listing.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    /// Process ID.
    pub id: ProcessId,
    /// Agent that owns this process.
    pub agent_id: String,
    /// Command that was started.
    pub command: String,
    /// Whether the process is still running.
    pub alive: bool,
    /// Uptime in seconds.
    pub uptime_secs: u64,
}

/// Manager for persistent agent processes.
pub struct ProcessManager {
    processes: DashMap<ProcessId, ManagedProcess>,
    max_per_agent: usize,
    next_id: std::sync::atomic::AtomicU64,
}

impl ProcessManager {
    /// Create a new process manager.
    pub fn new(max_per_agent: usize) -> Self {
        Self {
            processes: DashMap::new(),
            max_per_agent,
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Start a persistent process. Returns the process ID.
    ///
    /// `allowed_env_vars` controls which environment variables the child process
    /// inherits. The process environment is always sandboxed to prevent API key leakage.
    pub async fn start(
        &self,
        agent_id: &str,
        command: &str,
        args: &[String],
        allowed_env_vars: &[String],
    ) -> Result<ProcessId, String> {
        // Check per-agent limit
        let agent_count = self
            .processes
            .iter()
            .filter(|entry| entry.value().agent_id == agent_id)
            .count();

        if agent_count >= self.max_per_agent {
            return Err(format!(
                "Agent '{}' already has {} processes (max: {})",
                agent_id, agent_count, self.max_per_agent
            ));
        }

        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        // SECURITY: Sandbox environment to prevent API key leakage
        crate::subprocess_sandbox::sandbox_command(&mut cmd, allowed_env_vars);

        // On Unix, make the child its own process-group leader so that
        // `kill_process_tree` (which signals the negative PID) reaches the
        // whole descendant tree, not just the immediate child. tokio's
        // `Command` exposes `process_group` directly, so no extra trait
        // import is needed.
        #[cfg(unix)]
        {
            cmd.process_group(0);
        }

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to start process '{}': {}", command, e))?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_buf = Arc::new(Mutex::new(Vec::<String>::new()));
        let stderr_buf = Arc::new(Mutex::new(Vec::<String>::new()));

        // Spawn background readers for stdout/stderr
        // Per-line cap prevents OOM from subprocess producing mega-long lines.
        const MAX_LINE_BYTES: usize = 64 * 1024; // 64KB per line
        const MAX_LINES: usize = 1000;
        const DRAIN_COUNT: usize = 100;

        // Naive `&line[..MAX_LINE_BYTES]` panics if the byte at
        // MAX_LINE_BYTES lands mid-character (any non-ASCII output —
        // Cyrillic, CJK, emoji — can do this). The panic kills the
        // reader task silently and the rest of the subprocess output
        // is lost. Use the shared UTF-8-safe truncator so we always
        // cut at a char boundary.
        if let Some(out) = stdout {
            let buf = stdout_buf.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(out);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let truncated = if line.len() > MAX_LINE_BYTES {
                        let head = rusty_hand_types::text::truncate_bytes(&line, MAX_LINE_BYTES);
                        format!("{head}... [truncated, {} bytes]", line.len())
                    } else {
                        line
                    };
                    let mut b = buf.lock().await;
                    if b.len() >= MAX_LINES {
                        b.drain(..DRAIN_COUNT);
                    }
                    b.push(truncated);
                }
            });
        }

        if let Some(err) = stderr {
            let buf = stderr_buf.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(err);
                let mut lines = reader.lines();
                while let Ok(Some(line)) = lines.next_line().await {
                    let truncated = if line.len() > MAX_LINE_BYTES {
                        let head = rusty_hand_types::text::truncate_bytes(&line, MAX_LINE_BYTES);
                        format!("{head}... [truncated, {} bytes]", line.len())
                    } else {
                        line
                    };
                    let mut b = buf.lock().await;
                    if b.len() >= MAX_LINES {
                        b.drain(..DRAIN_COUNT);
                    }
                    b.push(truncated);
                }
            });
        }

        let id = format!(
            "proc_{}",
            self.next_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        );

        let cmd_display = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };

        debug!(process_id = %id, command = %cmd_display, agent = %agent_id, "Started persistent process");

        self.processes.insert(
            id.clone(),
            ManagedProcess {
                stdin,
                stdout_buf,
                stderr_buf,
                child,
                agent_id: agent_id.to_string(),
                command: cmd_display,
                started_at: std::time::Instant::now(),
            },
        );

        Ok(id)
    }

    /// Write data to a process's stdin.
    pub async fn write(&self, process_id: &str, data: &str) -> Result<(), String> {
        let mut entry = self
            .processes
            .get_mut(process_id)
            .ok_or_else(|| format!("Process '{}' not found", process_id))?;

        let proc = entry.value_mut();
        if let Some(stdin) = &mut proc.stdin {
            stdin
                .write_all(data.as_bytes())
                .await
                .map_err(|e| format!("Write failed: {}", e))?;
            stdin
                .flush()
                .await
                .map_err(|e| format!("Flush failed: {}", e))?;
            Ok(())
        } else {
            Err("Process stdin is closed".to_string())
        }
    }

    /// Read accumulated stdout/stderr (non-blocking drain).
    pub async fn read(&self, process_id: &str) -> Result<(Vec<String>, Vec<String>), String> {
        let entry = self
            .processes
            .get(process_id)
            .ok_or_else(|| format!("Process '{}' not found", process_id))?;

        let mut stdout = entry.stdout_buf.lock().await;
        let mut stderr = entry.stderr_buf.lock().await;

        let out_lines: Vec<String> = stdout.drain(..).collect();
        let err_lines: Vec<String> = stderr.drain(..).collect();

        Ok((out_lines, err_lines))
    }

    /// Kill a process.
    pub async fn kill(&self, process_id: &str) -> Result<(), String> {
        let (_, mut proc) = self
            .processes
            .remove(process_id)
            .ok_or_else(|| format!("Process '{}' not found", process_id))?;

        if let Some(pid) = proc.child.id() {
            debug!(process_id, pid, "Killing persistent process");
            let _ = crate::subprocess_sandbox::kill_process_tree(pid, 3000).await;
        }
        let _ = proc.child.kill().await;
        Ok(())
    }

    /// List all processes for an agent.
    pub fn list(&self, agent_id: &str) -> Vec<ProcessInfo> {
        self.processes
            .iter()
            .filter(|entry| entry.value().agent_id == agent_id)
            .map(|entry| {
                let alive = entry.value().child.id().is_some();
                ProcessInfo {
                    id: entry.key().clone(),
                    agent_id: entry.value().agent_id.clone(),
                    command: entry.value().command.clone(),
                    alive,
                    uptime_secs: entry.value().started_at.elapsed().as_secs(),
                }
            })
            .collect()
    }

    /// Cleanup: kill processes older than timeout.
    pub async fn cleanup(&self, max_age_secs: u64) {
        let to_remove: Vec<ProcessId> = self
            .processes
            .iter()
            .filter(|entry| entry.value().started_at.elapsed().as_secs() > max_age_secs)
            .map(|entry| entry.key().clone())
            .collect();

        for id in to_remove {
            warn!(process_id = %id, "Cleaning up stale process");
            let _ = self.kill(&id).await;
        }
    }

    /// Total process count.
    pub fn count(&self) -> usize {
        self.processes.len()
    }
}

impl Default for ProcessManager {
    fn default() -> Self {
        Self::new(5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_and_list() {
        let pm = ProcessManager::new(5);

        let cmd = if cfg!(windows) { "cmd" } else { "cat" };
        let args: Vec<String> = if cfg!(windows) {
            vec!["/C".to_string(), "echo".to_string(), "hello".to_string()]
        } else {
            vec![]
        };

        let id = pm.start("agent1", cmd, &args, &[]).await.unwrap();
        assert!(id.starts_with("proc_"));

        let list = pm.list("agent1");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].agent_id, "agent1");

        // Cleanup
        let _ = pm.kill(&id).await;
    }

    #[tokio::test]
    async fn test_per_agent_limit() {
        let pm = ProcessManager::new(1);

        // Use a short-lived command to avoid platform-specific process-tree kill behavior.
        // The manager counts tracked entries, so a completed process still occupies a slot.
        let cmd = if cfg!(windows) { "cmd" } else { "true" };
        let args: Vec<String> = if cfg!(windows) {
            vec!["/C".to_string(), "exit".to_string(), "0".to_string()]
        } else {
            vec![]
        };

        let _id1 = pm.start("agent1", cmd, &args, &[]).await.unwrap();
        let result = pm.start("agent1", cmd, &args, &[]).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("max: 1"));
    }

    #[tokio::test]
    async fn test_kill_nonexistent() {
        let pm = ProcessManager::new(5);
        let result = pm.kill("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_read_nonexistent() {
        let pm = ProcessManager::new(5);
        let result = pm.read("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_default_process_manager() {
        let pm = ProcessManager::default();
        assert_eq!(pm.max_per_agent, 5);
        assert_eq!(pm.count(), 0);
    }

    /// Regression: the per-line truncator used `&line[..MAX_LINE_BYTES]`,
    /// which panics when MAX_LINE_BYTES falls mid-character. Any
    /// subprocess emitting a >64KB line of non-ASCII output (Cyrillic,
    /// CJK, emoji) crashes the reader task and the rest of its stdout/
    /// stderr is silently dropped. We can't easily drive the real
    /// reader task in a unit test, but we can verify the shared
    /// truncator we now delegate to handles char-boundary cases
    /// correctly — which is the whole reason for the fix.
    #[test]
    fn truncate_bytes_handles_multibyte_at_boundary() {
        // Build a string where byte `n-1` is mid-character: lots of
        // ASCII followed by a 2-byte Cyrillic char straddling the cut.
        let mut s = "x".repeat(63);
        s.push('Я'); // 2-byte UTF-8 (0xD0 0xAF) at bytes 63..65
        s.push_str(&"y".repeat(10));
        // Cut at byte 64 — naive slice would split `Я`.
        let cut = rusty_hand_types::text::truncate_bytes(&s, 64);
        // Truncator must back up to the start of `Я` (byte 63).
        assert_eq!(cut.len(), 63);
        assert!(cut.ends_with("x"));
        // The full input is still well-formed UTF-8 after concat.
        assert!(s.is_char_boundary(63));
    }
}
