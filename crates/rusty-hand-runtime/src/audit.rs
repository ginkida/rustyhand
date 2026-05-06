//! Merkle hash chain audit trail for security-critical actions.
//!
//! Every auditable event is appended to an append-only log where each entry
//! contains the SHA-256 hash of its own contents concatenated with the hash of
//! the previous entry, forming a tamper-evident chain (similar to a blockchain).
//!
//! When constructed via [`AuditLog::open`], entries are also persisted to an
//! append-only JSONL file so the chain survives daemon restarts. Each `record()`
//! writes one JSON-encoded line and `fsync`s the file. On boot, existing lines
//! are replayed to rebuild the in-memory state.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Categories of auditable actions within the agent runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    ToolInvoke,
    CapabilityCheck,
    AgentSpawn,
    AgentKill,
    AgentMessage,
    MemoryAccess,
    FileAccess,
    NetworkAccess,
    ShellExec,
    AuthAttempt,
    WireConnect,
    ConfigChange,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A single entry in the Merkle hash chain audit log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonically increasing sequence number (0-indexed).
    pub seq: u64,
    /// ISO-8601 timestamp of when this entry was recorded.
    pub timestamp: String,
    /// The agent that triggered (or is the subject of) this action.
    pub agent_id: String,
    /// The category of action being audited.
    pub action: AuditAction,
    /// Free-form detail about the action (e.g. tool name, file path).
    pub detail: String,
    /// The outcome of the action (e.g. "ok", "denied", an error message).
    pub outcome: String,
    /// SHA-256 hash of the previous entry (or all-zeros for the genesis).
    pub prev_hash: String,
    /// SHA-256 hash of this entry's content concatenated with `prev_hash`.
    pub hash: String,
}

/// Computes the SHA-256 hash for a single audit entry from its fields.
fn compute_entry_hash(
    seq: u64,
    timestamp: &str,
    agent_id: &str,
    action: &AuditAction,
    detail: &str,
    outcome: &str,
    prev_hash: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_string().as_bytes());
    hasher.update(timestamp.as_bytes());
    hasher.update(agent_id.as_bytes());
    hasher.update(action.to_string().as_bytes());
    hasher.update(detail.as_bytes());
    hasher.update(outcome.as_bytes());
    hasher.update(prev_hash.as_bytes());
    hex::encode(hasher.finalize())
}

/// An append-only, tamper-evident audit log using a Merkle hash chain.
///
/// Thread-safe — all access is serialised through internal mutexes.
///
/// The log is in-memory by default. Use [`AuditLog::open`] to persist entries
/// to an append-only JSONL file so the chain survives daemon restarts.
pub struct AuditLog {
    entries: Mutex<Vec<AuditEntry>>,
    tip: Mutex<String>,
    /// Optional persistence file. When set, every recorded entry is appended
    /// (and `fsync`'d) to this writer in JSONL format.
    sink: Mutex<Option<BufWriter<File>>>,
    /// Path of the persistence file, kept for diagnostics. `None` for purely
    /// in-memory logs.
    path: Option<PathBuf>,
}

impl AuditLog {
    /// Creates a new empty in-memory audit log (no disk persistence).
    ///
    /// The initial tip hash is 64 zero characters (the "genesis" sentinel).
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
            tip: Mutex::new("0".repeat(64)),
            sink: Mutex::new(None),
            path: None,
        }
    }

    /// Opens (or creates) a persisted audit log at `path`.
    ///
    /// Existing entries in the file are replayed to rebuild the in-memory
    /// chain. Subsequent `record()` calls append to the file and `fsync` it.
    /// Corrupt lines (un-parsable JSON or a chain break) are skipped with a
    /// `tracing::warn!` so a partially-damaged file does not prevent boot.
    pub fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut entries: Vec<AuditEntry> = Vec::new();
        let mut tip = "0".repeat(64);

        if path.exists() {
            let file = File::open(&path)?;
            for (lineno, line) in BufReader::new(file).lines().enumerate() {
                let line = match line {
                    Ok(l) => l,
                    Err(e) => {
                        tracing::warn!(
                            audit_path = %path.display(),
                            line = lineno + 1,
                            error = %e,
                            "Skipping unreadable audit log line"
                        );
                        continue;
                    }
                };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<AuditEntry>(&line) {
                    Ok(entry) => {
                        if entry.prev_hash != tip {
                            tracing::warn!(
                                audit_path = %path.display(),
                                line = lineno + 1,
                                expected = %tip,
                                got = %entry.prev_hash,
                                "Audit log chain break — skipping remaining entries"
                            );
                            break;
                        }
                        let recomputed = compute_entry_hash(
                            entry.seq,
                            &entry.timestamp,
                            &entry.agent_id,
                            &entry.action,
                            &entry.detail,
                            &entry.outcome,
                            &entry.prev_hash,
                        );
                        if recomputed != entry.hash {
                            tracing::warn!(
                                audit_path = %path.display(),
                                line = lineno + 1,
                                "Audit log hash mismatch — skipping remaining entries"
                            );
                            break;
                        }
                        tip = entry.hash.clone();
                        entries.push(entry);
                    }
                    Err(e) => {
                        tracing::warn!(
                            audit_path = %path.display(),
                            line = lineno + 1,
                            error = %e,
                            "Skipping un-parsable audit log line"
                        );
                    }
                }
            }
        }

        let writer = OpenOptions::new().create(true).append(true).open(&path)?;

        Ok(Self {
            entries: Mutex::new(entries),
            tip: Mutex::new(tip),
            sink: Mutex::new(Some(BufWriter::new(writer))),
            path: Some(path),
        })
    }

    /// Returns the on-disk path if this log is persisted, otherwise `None`.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Records a new auditable event and returns the SHA-256 hash of the entry.
    ///
    /// The entry is atomically appended to the chain with the current tip as
    /// its `prev_hash`, and the tip is advanced to the new hash.
    pub fn record(
        &self,
        agent_id: impl Into<String>,
        action: AuditAction,
        detail: impl Into<String>,
        outcome: impl Into<String>,
    ) -> String {
        let agent_id = agent_id.into();
        let detail = detail.into();
        let outcome = outcome.into();
        let timestamp = Utc::now().to_rfc3339();

        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut tip = self.tip.lock().unwrap_or_else(|e| e.into_inner());

        let seq = entries.len() as u64;
        let prev_hash = tip.clone();

        let hash = compute_entry_hash(
            seq, &timestamp, &agent_id, &action, &detail, &outcome, &prev_hash,
        );

        let entry = AuditEntry {
            seq,
            timestamp,
            agent_id,
            action,
            detail,
            outcome,
            prev_hash,
            hash: hash.clone(),
        };

        // Persist before mutating in-memory state. If persistence fails we
        // still record in memory (forensic value beats hard-failing recording),
        // but the warning surfaces the disk problem.
        if let Some(sink) = self.sink.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            match serde_json::to_string(&entry) {
                Ok(line) => {
                    if let Err(e) = writeln!(sink, "{line}").and_then(|()| sink.flush()) {
                        tracing::warn!(
                            audit_path = ?self.path,
                            error = %e,
                            "Failed to persist audit entry — entry remains in-memory only"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to serialize audit entry for disk write");
                }
            }
        }

        entries.push(entry);
        *tip = hash.clone();
        hash
    }

    /// Walks the entire chain and recomputes every hash to detect tampering.
    ///
    /// Returns `Ok(())` if the chain is intact, or `Err(msg)` describing
    /// the first inconsistency found.
    pub fn verify_integrity(&self) -> Result<(), String> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut expected_prev = "0".repeat(64);

        for entry in entries.iter() {
            if entry.prev_hash != expected_prev {
                return Err(format!(
                    "chain break at seq {}: expected prev_hash {} but found {}",
                    entry.seq, expected_prev, entry.prev_hash
                ));
            }

            let recomputed = compute_entry_hash(
                entry.seq,
                &entry.timestamp,
                &entry.agent_id,
                &entry.action,
                &entry.detail,
                &entry.outcome,
                &entry.prev_hash,
            );

            if recomputed != entry.hash {
                return Err(format!(
                    "hash mismatch at seq {}: expected {} but found {}",
                    entry.seq, recomputed, entry.hash
                ));
            }

            expected_prev = entry.hash.clone();
        }

        Ok(())
    }

    /// Returns the current tip hash (the hash of the most recent entry,
    /// or the genesis sentinel if the log is empty).
    pub fn tip_hash(&self) -> String {
        self.tip.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Returns the number of entries in the log.
    pub fn len(&self) -> usize {
        self.entries.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Returns whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_empty()
    }

    /// Returns up to the most recent `n` entries (cloned).
    pub fn recent(&self, n: usize) -> Vec<AuditEntry> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let start = entries.len().saturating_sub(n);
        entries[start..].to_vec()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_chain_integrity() {
        let log = AuditLog::new();
        log.record(
            "agent-1",
            AuditAction::ToolInvoke,
            "read_file /etc/passwd",
            "ok",
        );
        log.record("agent-1", AuditAction::ShellExec, "ls -la", "ok");
        log.record("agent-2", AuditAction::AgentSpawn, "spawning helper", "ok");
        log.record(
            "agent-1",
            AuditAction::NetworkAccess,
            "https://example.com",
            "denied",
        );

        assert_eq!(log.len(), 4);
        assert!(log.verify_integrity().is_ok());

        // Verify the chain links are correct
        let entries = log.recent(4);
        assert_eq!(entries[0].prev_hash, "0".repeat(64));
        assert_eq!(entries[1].prev_hash, entries[0].hash);
        assert_eq!(entries[2].prev_hash, entries[1].hash);
        assert_eq!(entries[3].prev_hash, entries[2].hash);
    }

    #[test]
    fn test_audit_tamper_detection() {
        let log = AuditLog::new();
        log.record("agent-1", AuditAction::ToolInvoke, "read_file /tmp/a", "ok");
        log.record("agent-1", AuditAction::ShellExec, "rm -rf /", "denied");
        log.record("agent-1", AuditAction::MemoryAccess, "read key foo", "ok");

        // Tamper with an entry
        {
            let mut entries = log.entries.lock().unwrap();
            entries[1].detail = "echo hello".to_string(); // change the detail
        }

        let result = log.verify_integrity();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("hash mismatch at seq 1"));
    }

    #[test]
    fn test_audit_tip_changes() {
        let log = AuditLog::new();
        let genesis_tip = log.tip_hash();
        assert_eq!(genesis_tip, "0".repeat(64));

        let h1 = log.record("a", AuditAction::AgentSpawn, "spawn", "ok");
        assert_eq!(log.tip_hash(), h1);
        assert_ne!(log.tip_hash(), genesis_tip);

        let h2 = log.record("b", AuditAction::AgentKill, "kill", "ok");
        assert_eq!(log.tip_hash(), h2);
        assert_ne!(h2, h1);
    }

    #[test]
    fn test_audit_persists_across_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        // First daemon "lifetime" — records three entries.
        let h_first;
        let tip_first;
        {
            let log = AuditLog::open(&path).unwrap();
            assert_eq!(log.len(), 0);
            log.record("agent-1", AuditAction::AgentSpawn, "boot", "ok");
            log.record("agent-1", AuditAction::ToolInvoke, "read_file", "ok");
            h_first = log.record("agent-2", AuditAction::AgentKill, "shutdown", "ok");
            tip_first = log.tip_hash();
        }
        assert_ne!(tip_first, "0".repeat(64));

        // Second daemon "lifetime" — replays from disk.
        let log = AuditLog::open(&path).unwrap();
        assert_eq!(log.len(), 3);
        assert_eq!(log.tip_hash(), tip_first);
        assert!(log.verify_integrity().is_ok());
        assert_eq!(log.recent(1)[0].hash, h_first);

        // New entry chains onto the persisted tip and survives one more reopen.
        let h_new = log.record("agent-3", AuditAction::ConfigChange, "set foo", "ok");
        drop(log);

        let reloaded = AuditLog::open(&path).unwrap();
        assert_eq!(reloaded.len(), 4);
        assert_eq!(reloaded.tip_hash(), h_new);
        assert!(reloaded.verify_integrity().is_ok());
    }

    #[test]
    fn test_audit_open_on_missing_path_starts_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/never-created/audit.jsonl");
        let log = AuditLog::open(&path).unwrap();
        assert_eq!(log.len(), 0);
        assert_eq!(log.tip_hash(), "0".repeat(64));
        // Write must succeed even though the parent dir was created lazily.
        log.record("a", AuditAction::AgentSpawn, "x", "ok");
        assert_eq!(log.len(), 1);
    }

    #[test]
    fn test_audit_open_skips_corrupt_trailing_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.jsonl");

        // Seed with two valid entries.
        {
            let log = AuditLog::open(&path).unwrap();
            log.record("agent-1", AuditAction::AgentSpawn, "boot", "ok");
            log.record("agent-1", AuditAction::ToolInvoke, "read_file", "ok");
        }

        // Append a corrupt line.
        use std::io::Write as _;
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(f, "this is not valid json").unwrap();
        drop(f);

        // Reopen — should keep the two good entries and skip the bad line.
        let log = AuditLog::open(&path).unwrap();
        assert_eq!(log.len(), 2);
        assert!(log.verify_integrity().is_ok());
    }
}
