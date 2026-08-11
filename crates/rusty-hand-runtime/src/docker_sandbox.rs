//! Docker container sandbox — OS-level isolation for agent code execution.
//!
//! Provides secure command execution inside Docker containers with strict
//! resource limits, network isolation, and capability dropping.

use rusty_hand_types::config::DockerSandboxConfig;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info, warn};

/// A running sandbox container.
#[derive(Debug, Clone)]
pub struct SandboxContainer {
    pub container_id: String,
    pub agent_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Result of executing a command in the sandbox.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// SECURITY: Sanitize container name — alphanumeric + dash only.
fn sanitize_container_name(name: &str) -> Result<String, String> {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    if sanitized.is_empty() {
        return Err("Container name cannot be empty".into());
    }
    if sanitized.len() > 63 {
        return Err("Container name too long (max 63 chars)".into());
    }
    Ok(sanitized)
}

/// SECURITY: Validate Docker image name — only allow safe characters.
fn validate_image_name(image: &str) -> Result<(), String> {
    if image.is_empty() {
        return Err("Docker image name cannot be empty".into());
    }
    // Allow: alphanumeric, dots, colons, slashes, dashes, underscores
    if !image
        .chars()
        .all(|c| c.is_alphanumeric() || ".:/-_".contains(c))
    {
        return Err(format!("Invalid Docker image name: {image}"));
    }
    Ok(())
}

/// SECURITY: Sanitize command — reject dangerous shell metacharacters.
///
/// The command will be passed to `sh -c` inside a sandboxed Docker container.
/// We block patterns that could enable injection of additional commands beyond
/// what the agent intended. Pipes (`|`) are allowed because they are a normal
/// part of shell usage within the sandbox.
fn validate_command(command: &str) -> Result<(), String> {
    if command.is_empty() {
        return Err("Command cannot be empty".into());
    }

    // Reject command substitution patterns
    let injection_patterns = ["`", "$(", "${"];
    for pattern in &injection_patterns {
        if command.contains(pattern) {
            return Err(format!(
                "Command contains disallowed pattern '{}' — potential injection",
                pattern
            ));
        }
    }

    // Reject command chaining operators
    let chaining_patterns = ["&&", "||", ";"];
    for pattern in &chaining_patterns {
        if command.contains(pattern) {
            return Err(format!(
                "Command contains disallowed chaining operator '{}' — use separate exec calls",
                pattern
            ));
        }
    }

    // Reject output redirection (use stdout/stderr capture instead)
    // Check >> before > so the error message is precise
    let redirect_patterns = [">>", ">", "<"];
    for pattern in &redirect_patterns {
        if command.contains(pattern) {
            return Err(format!(
                "Command contains disallowed redirect '{}' — use stdout/stderr capture instead",
                pattern
            ));
        }
    }

    // Reject embedded newlines (could bypass single-line validation)
    if command.contains('\n') || command.contains('\r') {
        return Err("Command contains newline — potential injection".into());
    }

    Ok(())
}

/// Check if Docker is available on this system.
pub async fn is_docker_available() -> bool {
    match tokio::process::Command::new("docker")
        .arg("version")
        .arg("--format")
        .arg("{{.Server.Version}}")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
    {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

/// Create and start a sandbox container for an agent.
pub async fn create_sandbox(
    config: &DockerSandboxConfig,
    agent_id: &str,
    workspace: &Path,
) -> Result<SandboxContainer, String> {
    validate_image_name(&config.image)?;
    let container_name = sanitize_container_name(&format!(
        "{}-{}",
        config.container_prefix,
        &agent_id[..agent_id.len().min(8)]
    ))?;

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("run").arg("-d").arg("--name").arg(&container_name);

    // Resource limits
    cmd.arg("--memory").arg(&config.memory_limit);
    cmd.arg("--cpus").arg(config.cpu_limit.to_string());
    cmd.arg("--pids-limit").arg(config.pids_limit.to_string());

    // Security: drop ALL capabilities, prevent privilege escalation
    cmd.arg("--cap-drop").arg("ALL");
    cmd.arg("--security-opt").arg("no-new-privileges");

    // Add back specific capabilities if configured
    for cap in &config.cap_add {
        // Validate: only allow known capability names (alphanumeric + underscore)
        if cap.chars().all(|c| c.is_alphanumeric() || c == '_') {
            cmd.arg("--cap-add").arg(cap);
        } else {
            warn!("Skipping invalid capability: {cap}");
        }
    }

    // Read-only root filesystem
    if config.read_only_root {
        cmd.arg("--read-only");
    }

    // Network isolation
    cmd.arg("--network").arg(&config.network);

    // tmpfs mounts
    for tmpfs_mount in &config.tmpfs {
        cmd.arg("--tmpfs").arg(tmpfs_mount);
    }

    // Mount workspace read-only
    let ws_str = workspace.display().to_string();
    cmd.arg("-v").arg(format!("{ws_str}:{}:ro", config.workdir));

    // Working directory
    cmd.arg("-w").arg(&config.workdir);

    // Image + command to keep container alive
    cmd.arg(&config.image).arg("sleep").arg("infinity");

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    debug!(container = %container_name, image = %config.image, "Creating Docker sandbox");

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run docker: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Docker create failed: {}", stderr.trim()));
    }

    let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    Ok(SandboxContainer {
        container_id,
        agent_id: agent_id.to_string(),
        created_at: chrono::Utc::now(),
    })
}

/// `(stdout_kept, stdout_total, stderr_kept, stderr_total, status)` from one
/// capped `docker exec` capture.
type CapturedExec = (Vec<u8>, usize, Vec<u8>, usize, std::process::ExitStatus);

/// Read `reader` to EOF, keeping at most `cap` bytes in `buf` and DISCARDING the
/// rest. Returns the total number of bytes the stream produced.
///
/// Draining past the cap is load-bearing, not tidiness: an OS pipe holds ~64 KiB,
/// so a reader that simply stopped at the cap would leave the child blocked
/// writing into a full pipe forever. `child.wait()` would then never return and
/// EVERY over-cap command would burn the full timeout and report a timeout error
/// instead of its (truncated) output — strictly worse than the unbounded
/// `cmd.output()` this replaced. Reading-and-discarding bounds host memory while
/// still letting the child finish and yield a real exit code.
async fn read_capped<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
    buf: &mut Vec<u8>,
    cap: usize,
) -> usize {
    use tokio::io::AsyncReadExt;
    let mut chunk = [0u8; 8192];
    let mut total = 0usize;
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                total += n;
                if buf.len() < cap {
                    let take = n.min(cap - buf.len());
                    buf.extend_from_slice(&chunk[..take]);
                }
            }
        }
    }
    total
}

/// Execute a command inside an existing sandbox container.
pub async fn exec_in_sandbox(
    container: &SandboxContainer,
    command: &str,
    timeout: Duration,
) -> Result<ExecResult, String> {
    validate_command(command)?;

    let mut cmd = tokio::process::Command::new("docker");
    cmd.arg("exec")
        .arg(&container.container_id)
        .arg("sh")
        .arg("-c")
        .arg(command);

    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    debug!(container = %container.container_id, "Executing in Docker sandbox");

    // SECURITY: `cmd.output()` buffers the container's ENTIRE stdout/stderr into
    // host memory with no cap (the cgroup --memory limit covers the in-container
    // process, not the host-side drain), so a high-output command (`yes`,
    // `cat /dev/zero`) could OOM the daemon. Stream both pipes with a per-stream
    // host cap instead, draining (and discarding) the remainder so the child can
    // still exit normally. On timeout the child is killed via kill_on_drop.
    //
    // NOTE: kill_on_drop reaps the `docker exec` CLIENT process. The process
    // inside the container is not signalled by that alone — callers rely on
    // destroy_sandbox (which `docker rm -f`s the container) for that, and
    // tool_docker_exec always does so after this returns.
    const MAX_CAPTURE_BYTES: usize = 5 * 1024 * 1024;
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Docker exec spawn failed: {e}"))?;
    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "Docker exec: no stdout pipe".to_string())?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "Docker exec: no stderr pipe".to_string())?;

    let collect = async {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let (out_total, err_total) = tokio::join!(
            read_capped(&mut stdout_pipe, &mut out, MAX_CAPTURE_BYTES),
            read_capped(&mut stderr_pipe, &mut err, MAX_CAPTURE_BYTES),
        );
        let status = child
            .wait()
            .await
            .map_err(|e| format!("Docker exec wait failed: {e}"))?;
        Ok::<CapturedExec, String>((out, out_total, err, err_total, status))
    };

    let (stdout_bytes, stdout_total, stderr_bytes, stderr_total, status) =
        tokio::time::timeout(timeout, collect)
            .await
            .map_err(|_| format!("Docker exec timed out after {}s", timeout.as_secs()))??;

    let stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let stderr = String::from_utf8_lossy(&stderr_bytes).to_string();
    let exit_code = status.code().unwrap_or(-1);

    // Truncate large outputs. Container stdout/stderr is frequently
    // non-ASCII — use a char-boundary-safe truncation so byte `max_output`
    // never splits a multi-byte char (raw `&s[..max_output]` panics).
    //
    // Report the stream's TRUE total (from read_capped), not the retained
    // length: past MAX_CAPTURE_BYTES the tail was discarded, so `stdout.len()`
    // would understate a 100 MiB stream as exactly 5 MiB.
    let max_output = 50_000;
    let stdout = if stdout.len() > max_output {
        format!(
            "{}... [truncated, {} total bytes]",
            rusty_hand_types::text::truncate_bytes(&stdout, max_output),
            stdout_total
        )
    } else {
        stdout
    };
    let stderr = if stderr.len() > max_output {
        format!(
            "{}... [truncated, {} total bytes]",
            rusty_hand_types::text::truncate_bytes(&stderr, max_output),
            stderr_total
        )
    } else {
        stderr
    };

    Ok(ExecResult {
        stdout,
        stderr,
        exit_code,
    })
}

/// Stop and remove a sandbox container, retrying on transient Docker errors.
pub async fn destroy_sandbox(container: &SandboxContainer) -> Result<(), String> {
    debug!(container = %container.container_id, "Destroying Docker sandbox");

    // Retry up to 3 times with short backoff — Docker daemon can be briefly
    // busy under load and a single failure would leak the container.
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(250 * attempt as u64)).await;
        }
        match tokio::process::Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(&container.container_id)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
        {
            Ok(output) if output.status.success() => return Ok(()),
            Ok(output) => {
                last_err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            }
            Err(e) => {
                last_err = format!("Failed to run docker rm: {e}");
            }
        }
    }
    warn!(container = %container.container_id, "Docker rm failed after 3 attempts: {last_err}");
    Err(format!("Docker rm failed: {last_err}"))
}

/// Decide whether one `docker ps` line describes a container this process may
/// force-remove. Returns the container id when it is safe to reap.
///
/// Expects the `{{.ID}} {{.State}} {{.CreatedAt}}` format, where `CreatedAt`
/// looks like `2026-06-30 14:23:01 +0000 UTC`.
///
/// The rules encode who might still own the container:
///   * A container that is **not running** (exited/created/dead) cannot belong
///     to a live sibling instance's in-flight exec, so it is reaped regardless
///     of age. This is the crash-recovery case the reaper exists for — gating it
///     behind an age threshold would leave a just-crashed run's container in
///     place, and since sandbox names are derived from the agent id, that stale
///     container makes every later exec for that agent fail on a name conflict.
///   * A **running** container may be a sibling's, so it is only reaped once it
///     is older than `min_age`.
///   * An **unparseable** timestamp is never reaped — we don't force-remove a
///     container whose age we can't establish.
///
/// Split out as a pure function so the parsing and the policy are unit-testable
/// without a Docker daemon.
fn reap_decision(
    line: &str,
    now: chrono::DateTime<chrono::Utc>,
    min_age: chrono::Duration,
) -> Option<String> {
    let mut parts = line.split_whitespace();
    let id = match parts.next() {
        Some(id) if !id.is_empty() => id.to_string(),
        _ => return None,
    };
    let state = parts.next()?.to_ascii_lowercase();
    // "paused" and "restarting" are live states — treat them like "running".
    if !matches!(state.as_str(), "running" | "paused" | "restarting") {
        return Some(id);
    }

    let rest: Vec<&str> = parts.collect();
    if rest.len() < 3 {
        return None;
    }
    let ts = format!("{} {} {}", rest[0], rest[1], rest[2]);
    let created = chrono::DateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S %z").ok()?;
    if now.signed_duration_since(created.with_timezone(&chrono::Utc)) >= min_age {
        Some(id)
    } else {
        None
    }
}

/// Reap orphaned sandbox containers left over from a previous rustyhand
/// run (e.g. after a crash). Lists containers whose name matches the
/// configured prefix and removes the ones [`reap_decision`] deems unowned.
/// Non-fatal.
pub async fn reap_orphan_sandboxes(prefix: &str) {
    // Anchor the filter: Docker's `name=` filter is an unanchored regex, so a
    // bare prefix also matches any container that merely CONTAINS it.
    let filter = format!("name=^{prefix}");
    let output = match tokio::process::Command::new("docker")
        .arg("ps")
        .arg("-a")
        .arg("--filter")
        .arg(&filter)
        .arg("--format")
        .arg("{{.ID}} {{.State}} {{.CreatedAt}}")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => {
            debug!("Orphan reaper skipped: docker ps failed: {e}");
            return;
        }
    };

    if !output.status.success() {
        return;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let now = chrono::Utc::now();
    let min_age = chrono::Duration::hours(1);
    let mut reaped = 0usize;
    for line in stdout.lines() {
        let Some(id) = reap_decision(line, now, min_age) else {
            continue;
        };
        let rm = tokio::process::Command::new("docker")
            .arg("rm")
            .arg("-f")
            .arg(&id)
            .output()
            .await;
        if matches!(rm, Ok(o) if o.status.success()) {
            reaped += 1;
        }
    }
    if reaped > 0 {
        info!("Reaped {reaped} orphaned sandbox container(s)");
    }
}

// ---------------------------------------------------------------------------
// Container Pool (Gap 5) — reuse containers across sessions
// ---------------------------------------------------------------------------

use dashmap::DashMap;
use std::sync::Arc;

/// Pool entry for a reusable container.
#[derive(Debug, Clone)]
struct PoolEntry {
    container: SandboxContainer,
    config_hash: u64,
    last_used: std::time::Instant,
    created: std::time::Instant,
}

/// Container pool for reusing Docker containers.
pub struct ContainerPool {
    entries: Arc<DashMap<String, PoolEntry>>,
}

impl ContainerPool {
    /// Create a new container pool.
    pub fn new() -> Self {
        Self {
            entries: Arc::new(DashMap::new()),
        }
    }

    /// Acquire a container from the pool matching the config hash, or None.
    pub fn acquire(&self, config_hash: u64, cool_secs: u64) -> Option<SandboxContainer> {
        let mut found_key = None;
        for entry in self.entries.iter() {
            if entry.config_hash == config_hash && entry.last_used.elapsed().as_secs() >= cool_secs
            {
                found_key = Some(entry.key().clone());
                break;
            }
        }
        if let Some(key) = found_key {
            self.entries.remove(&key).map(|(_, e)| e.container)
        } else {
            None
        }
    }

    /// Release a container back to the pool.
    pub fn release(&self, container: SandboxContainer, config_hash: u64) {
        self.entries.insert(
            container.container_id.clone(),
            PoolEntry {
                container,
                config_hash,
                last_used: std::time::Instant::now(),
                created: std::time::Instant::now(),
            },
        );
    }

    /// Cleanup containers older than max_age or idle longer than idle_timeout.
    pub async fn cleanup(&self, idle_timeout_secs: u64, max_age_secs: u64) {
        let to_remove: Vec<(String, SandboxContainer)> = self
            .entries
            .iter()
            .filter(|e| {
                e.last_used.elapsed().as_secs() > idle_timeout_secs
                    || e.created.elapsed().as_secs() > max_age_secs
            })
            .map(|e| (e.key().clone(), e.container.clone()))
            .collect();

        for (key, container) in to_remove {
            debug!(container_id = %container.container_id, "Cleaning up stale pool container");
            let _ = destroy_sandbox(&container).await;
            self.entries.remove(&key);
        }
    }

    /// Number of containers in the pool.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for ContainerPool {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Bind Mount Validation (Gap 5) — prevent mounting sensitive host paths
// ---------------------------------------------------------------------------

/// Default blocked mount paths (always blocked regardless of config).
const BLOCKED_MOUNT_PATHS: &[&str] = &[
    "/etc",
    "/proc",
    "/sys",
    "/dev",
    "/var/run/docker.sock",
    "/root",
    "/boot",
];

/// Validate a bind mount path for security.
///
/// Blocks:
/// - Sensitive system paths (/etc, /proc, /sys, Docker socket)
/// - Non-absolute paths
/// - Symlink escape attempts
/// - Paths in the configured blocked_mounts list
pub fn validate_bind_mount(path: &str, blocked: &[String]) -> Result<(), String> {
    let p = std::path::Path::new(path);

    // Must be absolute (Docker bind mounts use Unix paths, so check for '/' prefix
    // in addition to platform-native is_absolute check)
    if !p.is_absolute() && !path.starts_with('/') {
        return Err(format!("Bind mount path must be absolute: {path}"));
    }

    // Check for path traversal
    for component in p.components() {
        if let std::path::Component::ParentDir = component {
            return Err(format!("Bind mount path contains '..': {path}"));
        }
    }

    // Check default blocked paths
    for blocked_path in BLOCKED_MOUNT_PATHS {
        if path.starts_with(blocked_path) {
            return Err(format!(
                "Bind mount to '{blocked_path}' is blocked for security"
            ));
        }
    }

    // Check user-configured blocked paths
    for bp in blocked {
        if path.starts_with(bp.as_str()) {
            return Err(format!("Bind mount to '{bp}' is blocked by configuration"));
        }
    }

    // Check for symlink escape (best-effort — canonicalize if path exists)
    if p.exists() {
        match p.canonicalize() {
            Ok(canonical) => {
                let canonical_str = canonical.to_string_lossy();
                for blocked_path in BLOCKED_MOUNT_PATHS {
                    if canonical_str.starts_with(blocked_path) {
                        return Err(format!(
                            "Bind mount resolves to blocked path via symlink: {} → {}",
                            path, canonical_str
                        ));
                    }
                }
            }
            Err(_) => {
                // Can't canonicalize — path doesn't exist yet, allow it
            }
        }
    }

    Ok(())
}

/// Hash a Docker sandbox config for pool matching.
pub fn config_hash(config: &DockerSandboxConfig) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    config.image.hash(&mut hasher);
    config.network.hash(&mut hasher);
    config.memory_limit.hash(&mut hasher);
    config.workdir.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── read_capped ─────────────────────────────────────────────────────────

    /// The whole point of draining past the cap: the reader must reach EOF even
    /// when the stream is far larger than the cap, so the child can exit and
    /// `wait()` can return. A reader that stopped at the cap would leave the
    /// writer blocked on a full pipe and every over-cap command would hit the
    /// caller's timeout instead of returning its (truncated) output.
    #[tokio::test]
    async fn read_capped_drains_past_the_cap_and_reports_the_true_total() {
        let data = vec![b'x'; 200_000];
        let mut reader = std::io::Cursor::new(data.clone());
        let mut buf = Vec::new();
        let total = read_capped(&mut reader, &mut buf, 1_000).await;

        assert_eq!(buf.len(), 1_000, "must retain exactly the cap");
        assert_eq!(total, 200_000, "must report the full stream length");
    }

    #[tokio::test]
    async fn read_capped_keeps_everything_under_the_cap() {
        let mut reader = std::io::Cursor::new(b"hello".to_vec());
        let mut buf = Vec::new();
        let total = read_capped(&mut reader, &mut buf, 1_000).await;
        assert_eq!(buf, b"hello");
        assert_eq!(total, 5);
    }

    /// End-to-end guard on the deadlock this replaced: a real child that writes
    /// far more than the cap must still be reaped, and quickly.
    #[tokio::test]
    async fn over_cap_child_still_exits() {
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg("head -c 2000000 /dev/zero")
            .stdout(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .expect("spawn");
        let mut pipe = child.stdout.take().expect("stdout");
        let mut buf = Vec::new();

        let total = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            read_capped(&mut pipe, &mut buf, 4096),
        )
        .await
        .expect("read_capped must finish, not block on a full pipe");

        assert_eq!(buf.len(), 4096);
        assert_eq!(total, 2_000_000);
        let status = tokio::time::timeout(std::time::Duration::from_secs(10), child.wait())
            .await
            .expect("child must be reapable once its pipe is drained")
            .expect("wait");
        assert!(status.success());
    }

    // ── reap_decision ───────────────────────────────────────────────────────

    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339("2026-06-30T15:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc)
    }

    /// The crash-recovery case the reaper exists for. A container left behind by
    /// a crashed run is `exited`; gating it behind an age threshold (as the
    /// first version of this fix did) meant a restart within the hour never
    /// cleaned it up — and since sandbox names derive from the agent id, the
    /// stale container then broke every later exec with a name conflict.
    #[test]
    fn reaps_non_running_containers_regardless_of_age() {
        let line = "abc123 exited 2026-06-30 14:59:30 +0000 UTC";
        assert_eq!(
            reap_decision(line, now(), chrono::Duration::hours(1)),
            Some("abc123".to_string())
        );
    }

    /// A running container may belong to a live sibling instance sharing the
    /// default prefix, so it is only reaped once it is old enough.
    #[test]
    fn spares_young_running_containers() {
        let line = "abc123 running 2026-06-30 14:59:30 +0000 UTC";
        assert_eq!(reap_decision(line, now(), chrono::Duration::hours(1)), None);
    }

    #[test]
    fn reaps_old_running_containers() {
        let line = "abc123 running 2026-06-30 13:00:00 +0000 UTC";
        assert_eq!(
            reap_decision(line, now(), chrono::Duration::hours(1)),
            Some("abc123".to_string())
        );
    }

    /// "paused" / "restarting" are live states, not orphans.
    #[test]
    fn treats_paused_and_restarting_as_live() {
        for state in ["paused", "restarting"] {
            let line = format!("abc123 {state} 2026-06-30 14:59:30 +0000 UTC");
            assert_eq!(
                reap_decision(&line, now(), chrono::Duration::hours(1)),
                None,
                "{state} must not be reaped while young"
            );
        }
    }

    /// Never force-remove a container whose age we cannot establish.
    #[test]
    fn skips_unparseable_and_malformed_lines() {
        for line in [
            "abc123 running not-a-timestamp",
            "abc123 running",
            "abc123",
            "",
            "   ",
        ] {
            assert_eq!(
                reap_decision(line, now(), chrono::Duration::hours(1)),
                None,
                "must not reap on line {line:?}"
            );
        }
    }

    /// Docker renders CreatedAt with a real offset on non-UTC hosts.
    #[test]
    fn parses_non_utc_offsets() {
        let line = "abc123 running 2026-06-30 18:00:00 +0500 MSK"; // == 13:00Z
        assert_eq!(
            reap_decision(line, now(), chrono::Duration::hours(1)),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_sanitize_container_name_valid() {
        let result = sanitize_container_name("rusty-hand-sandbox-abc123").unwrap();
        assert_eq!(result, "rusty-hand-sandbox-abc123");
    }

    #[test]
    fn test_sanitize_container_name_special_chars() {
        let result = sanitize_container_name("test;rm -rf /").unwrap();
        assert!(!result.contains(';'));
        assert!(!result.contains(' '));
    }

    #[test]
    fn test_sanitize_container_name_empty() {
        assert!(sanitize_container_name("").is_err());
    }

    #[test]
    fn test_sanitize_container_name_too_long() {
        let long = "a".repeat(100);
        assert!(sanitize_container_name(&long).is_err());
    }

    #[test]
    fn test_validate_image_name_valid() {
        assert!(validate_image_name("python:3.12-slim").is_ok());
        assert!(validate_image_name("ubuntu:22.04").is_ok());
        assert!(validate_image_name("registry.example.com/my-image:latest").is_ok());
    }

    #[test]
    fn test_validate_image_name_empty() {
        assert!(validate_image_name("").is_err());
    }

    #[test]
    fn test_validate_image_name_invalid() {
        assert!(validate_image_name("image;rm -rf /").is_err());
        assert!(validate_image_name("image`whoami`").is_err());
        assert!(validate_image_name("image$(id)").is_err());
    }

    #[test]
    fn test_validate_command_valid() {
        assert!(validate_command("python script.py").is_ok());
        assert!(validate_command("ls -la /workspace").is_ok());
        assert!(validate_command("echo hello | grep h").is_ok());
    }

    #[test]
    fn test_validate_command_empty() {
        assert!(validate_command("").is_err());
    }

    #[test]
    fn test_validate_command_backticks() {
        assert!(validate_command("echo `whoami`").is_err());
    }

    #[test]
    fn test_validate_command_dollar_paren() {
        assert!(validate_command("echo $(id)").is_err());
    }

    #[test]
    fn test_validate_command_dollar_brace() {
        assert!(validate_command("echo ${HOME}").is_err());
    }

    #[test]
    fn test_validate_command_chaining() {
        assert!(validate_command("echo a; echo b").is_err());
        assert!(validate_command("true && rm -rf /").is_err());
        assert!(validate_command("false || malicious").is_err());
    }

    #[test]
    fn test_validate_command_redirect() {
        assert!(validate_command("echo secret > /tmp/leak").is_err());
        assert!(validate_command("echo data >> /tmp/append").is_err());
        assert!(validate_command("cat < /etc/passwd").is_err());
    }

    #[test]
    fn test_validate_command_newlines() {
        assert!(validate_command("echo hello\nrm -rf /").is_err());
        assert!(validate_command("line1\r\nline2").is_err());
    }

    #[tokio::test]
    async fn test_docker_available() {
        // Just verify it doesn't panic — result depends on Docker installation
        let _ = is_docker_available().await;
    }

    #[test]
    fn test_config_defaults() {
        let config = DockerSandboxConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.image, "python:3.12-slim");
        assert_eq!(config.container_prefix, "rusty-hand-sandbox");
        assert_eq!(config.workdir, "/workspace");
        assert_eq!(config.network, "none");
        assert_eq!(config.memory_limit, "512m");
        assert_eq!(config.cpu_limit, 1.0);
        assert_eq!(config.timeout_secs, 60);
        assert!(config.read_only_root);
        assert!(config.cap_add.is_empty());
        assert_eq!(config.tmpfs, vec!["/tmp:size=64m"]);
        assert_eq!(config.pids_limit, 100);
    }

    #[test]
    fn test_exec_result_fields() {
        let result = ExecResult {
            stdout: "hello".to_string(),
            stderr: String::new(),
            exit_code: 0,
        };
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.stdout, "hello");
    }

    // ── Container Pool tests ──────────────────────────────────────────

    #[test]
    fn test_container_pool_empty() {
        let pool = ContainerPool::new();
        assert!(pool.is_empty());
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_container_pool_release_acquire() {
        let pool = ContainerPool::new();
        let container = SandboxContainer {
            container_id: "test123".to_string(),
            agent_id: "agent1".to_string(),
            created_at: chrono::Utc::now(),
        };
        pool.release(container, 12345);
        assert_eq!(pool.len(), 1);

        // Acquire with same hash — should succeed (cool_secs=0 for test)
        let acquired = pool.acquire(12345, 0);
        assert!(acquired.is_some());
        assert_eq!(acquired.unwrap().container_id, "test123");
        assert!(pool.is_empty());
    }

    #[test]
    fn test_container_pool_hash_mismatch() {
        let pool = ContainerPool::new();
        let container = SandboxContainer {
            container_id: "test123".to_string(),
            agent_id: "agent1".to_string(),
            created_at: chrono::Utc::now(),
        };
        pool.release(container, 12345);

        // Acquire with different hash — should fail
        let acquired = pool.acquire(99999, 0);
        assert!(acquired.is_none());
    }

    // ── Bind Mount Validation tests ──────────────────────────────────

    #[test]
    fn test_validate_bind_mount_valid() {
        assert!(validate_bind_mount("/home/user/workspace", &[]).is_ok());
        assert!(validate_bind_mount("/tmp/sandbox", &[]).is_ok());
    }

    #[test]
    fn test_validate_bind_mount_non_absolute() {
        assert!(validate_bind_mount("relative/path", &[]).is_err());
    }

    #[test]
    fn test_validate_bind_mount_blocked_paths() {
        assert!(validate_bind_mount("/etc/passwd", &[]).is_err());
        assert!(validate_bind_mount("/proc/self", &[]).is_err());
        assert!(validate_bind_mount("/sys/kernel", &[]).is_err());
        assert!(validate_bind_mount("/var/run/docker.sock", &[]).is_err());
    }

    #[test]
    fn test_validate_bind_mount_traversal() {
        assert!(validate_bind_mount("/home/user/../etc/passwd", &[]).is_err());
    }

    #[test]
    fn test_validate_bind_mount_custom_blocked() {
        let blocked = vec!["/data/secrets".to_string()];
        assert!(validate_bind_mount("/data/secrets/vault", &blocked).is_err());
        assert!(validate_bind_mount("/data/public", &blocked).is_ok());
    }

    #[test]
    fn test_config_hash_deterministic() {
        let c1 = DockerSandboxConfig::default();
        let c2 = DockerSandboxConfig::default();
        assert_eq!(config_hash(&c1), config_hash(&c2));
    }

    #[test]
    fn test_config_hash_different_images() {
        let c1 = DockerSandboxConfig::default();
        let c2 = DockerSandboxConfig {
            image: "node:20-slim".to_string(),
            ..Default::default()
        };
        assert_ne!(config_hash(&c1), config_hash(&c2));
    }
}
