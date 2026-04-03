//! Cron job scheduler engine for the RustyHand kernel.
//!
//! Manages scheduled jobs (recurring and one-shot) across all agents.
//! This is separate from `scheduler.rs` which handles agent resource tracking.
//!
//! The scheduler stores jobs in a `DashMap` for concurrent access, persists
//! them to a JSON file on disk, and exposes methods for the kernel tick loop
//! to query due jobs and record outcomes.

use chrono::{Duration, Utc};
use dashmap::DashMap;
use rusty_hand_types::agent::AgentId;
use rusty_hand_types::error::{RustyHandError, RustyHandResult};
use rusty_hand_types::scheduler::{CronJob, CronJobId, CronSchedule};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{debug, info, warn};

/// Maximum consecutive errors before a job is auto-disabled.
const MAX_CONSECUTIVE_ERRORS: u32 = 5;

// ---------------------------------------------------------------------------
// JobMeta — extra runtime state not stored in CronJob itself
// ---------------------------------------------------------------------------

/// Runtime metadata for a cron job that extends the base `CronJob` type.
///
/// The `CronJob` struct in `rusty-hand-types` is intentionally lean (no
/// `one_shot`, `last_status`, or error tracking). The scheduler tracks
/// these operational details separately.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMeta {
    /// The underlying job definition.
    pub job: CronJob,
    /// Whether this job should be removed after a single successful execution.
    pub one_shot: bool,
    /// Human-readable status of the last execution (e.g. `"ok"` or `"error: ..."`).
    pub last_status: Option<String>,
    /// Number of consecutive failed executions.
    pub consecutive_errors: u32,
}

impl JobMeta {
    /// Wrap a `CronJob` with default metadata.
    pub fn new(job: CronJob, one_shot: bool) -> Self {
        Self {
            job,
            one_shot,
            last_status: None,
            consecutive_errors: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// CronScheduler
// ---------------------------------------------------------------------------

/// Cron job scheduler — manages scheduled jobs for all agents.
///
/// Thread-safe via `DashMap`. The kernel should call [`due_jobs`] on a
/// regular interval (e.g. every 10-30 seconds) to discover jobs that need
/// to fire, then call [`record_success`] or [`record_failure`] after
/// execution completes.
pub struct CronScheduler {
    /// All tracked jobs, keyed by their unique ID.
    jobs: DashMap<CronJobId, JobMeta>,
    /// Path to the persistence file (`<home>/cron_jobs.json`).
    persist_path: PathBuf,
    /// Global cap on total jobs across all agents (atomic for hot-reload).
    max_total_jobs: AtomicUsize,
}

impl CronScheduler {
    /// Create a new scheduler.
    ///
    /// `home_dir` is the RustyHand data directory; jobs are persisted to
    /// `<home_dir>/cron_jobs.json`. `max_total_jobs` caps the total number
    /// of jobs across all agents.
    pub fn new(home_dir: &Path, max_total_jobs: usize) -> Self {
        Self {
            jobs: DashMap::new(),
            persist_path: home_dir.join("cron_jobs.json"),
            max_total_jobs: AtomicUsize::new(max_total_jobs),
        }
    }

    /// Update the max total jobs limit (for hot-reload).
    pub fn set_max_total_jobs(&self, new_max: usize) {
        self.max_total_jobs.store(new_max, Ordering::Relaxed);
    }

    // -- Persistence --------------------------------------------------------

    /// Load persisted jobs from disk.
    ///
    /// Returns the number of jobs loaded. If the persistence file does not
    /// exist, returns `Ok(0)` without error.
    pub fn load(&self) -> RustyHandResult<usize> {
        if !self.persist_path.exists() {
            return Ok(0);
        }
        let data = std::fs::read_to_string(&self.persist_path)
            .map_err(|e| RustyHandError::Internal(format!("Failed to read cron jobs: {e}")))?;
        let metas: Vec<JobMeta> = serde_json::from_str(&data)
            .map_err(|e| RustyHandError::Internal(format!("Failed to parse cron jobs: {e}")))?;
        let count = metas.len();
        for meta in metas {
            self.jobs.insert(meta.job.id, meta);
        }
        info!(count, "Loaded cron jobs from disk");
        Ok(count)
    }

    /// Persist all jobs to disk via atomic write (write to `.tmp`, then rename).
    pub fn persist(&self) -> RustyHandResult<()> {
        let metas: Vec<JobMeta> = self.jobs.iter().map(|r| r.value().clone()).collect();
        let data = serde_json::to_string_pretty(&metas)
            .map_err(|e| RustyHandError::Internal(format!("Failed to serialize cron jobs: {e}")))?;
        let tmp_path = self.persist_path.with_extension("json.tmp");
        std::fs::write(&tmp_path, data.as_bytes()).map_err(|e| {
            RustyHandError::Internal(format!("Failed to write cron jobs temp file: {e}"))
        })?;
        std::fs::rename(&tmp_path, &self.persist_path).map_err(|e| {
            RustyHandError::Internal(format!("Failed to rename cron jobs file: {e}"))
        })?;
        debug!(count = metas.len(), "Persisted cron jobs");
        Ok(())
    }

    // -- CRUD ---------------------------------------------------------------

    /// Add a new job. Validates fields, computes the initial `next_run`,
    /// and inserts it into the scheduler.
    ///
    /// `one_shot` controls whether the job is removed after a single
    /// successful execution.
    pub fn add_job(&self, mut job: CronJob, one_shot: bool) -> RustyHandResult<CronJobId> {
        // Global limit
        let max_jobs = self.max_total_jobs.load(Ordering::Relaxed);
        if self.jobs.len() >= max_jobs {
            return Err(RustyHandError::Internal(format!(
                "Global cron job limit reached ({})",
                max_jobs
            )));
        }

        // Per-agent count
        let agent_count = self
            .jobs
            .iter()
            .filter(|r| r.value().job.agent_id == job.agent_id)
            .count();

        // CronJob.validate returns Result<(), String>
        job.validate(agent_count)
            .map_err(RustyHandError::InvalidInput)?;

        // Compute initial next_run
        job.next_run = Some(compute_next_run(&job.schedule));

        let id = job.id;
        self.jobs.insert(id, JobMeta::new(job, one_shot));
        Ok(id)
    }

    /// Remove a job by ID. Returns the removed `CronJob`.
    pub fn remove_job(&self, id: CronJobId) -> RustyHandResult<CronJob> {
        self.jobs
            .remove(&id)
            .map(|(_, meta)| meta.job)
            .ok_or_else(|| RustyHandError::Internal(format!("Cron job {id} not found")))
    }

    /// Enable or disable a job. Re-enabling resets errors and recomputes
    /// `next_run`.
    pub fn set_enabled(&self, id: CronJobId, enabled: bool) -> RustyHandResult<()> {
        match self.jobs.get_mut(&id) {
            Some(mut meta) => {
                meta.job.enabled = enabled;
                if enabled {
                    meta.consecutive_errors = 0;
                    meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
                }
                Ok(())
            }
            None => Err(RustyHandError::Internal(format!("Cron job {id} not found"))),
        }
    }

    // -- Queries ------------------------------------------------------------

    /// Get a single job by ID.
    pub fn get_job(&self, id: CronJobId) -> Option<CronJob> {
        self.jobs.get(&id).map(|r| r.value().job.clone())
    }

    /// Get the full metadata for a job (includes `one_shot`, `last_status`,
    /// `consecutive_errors`).
    pub fn get_meta(&self, id: CronJobId) -> Option<JobMeta> {
        self.jobs.get(&id).map(|r| r.value().clone())
    }

    /// List all jobs for a specific agent.
    pub fn list_jobs(&self, agent_id: AgentId) -> Vec<CronJob> {
        self.jobs
            .iter()
            .filter(|r| r.value().job.agent_id == agent_id)
            .map(|r| r.value().job.clone())
            .collect()
    }

    /// List all jobs across all agents.
    pub fn list_all_jobs(&self) -> Vec<CronJob> {
        self.jobs.iter().map(|r| r.value().job.clone()).collect()
    }

    /// List full metadata for all jobs of a specific agent.
    pub fn list_metas(&self, agent_id: AgentId) -> Vec<JobMeta> {
        self.jobs
            .iter()
            .filter(|r| r.value().job.agent_id == agent_id)
            .map(|r| r.value().clone())
            .collect()
    }

    /// List full metadata for all jobs across all agents.
    pub fn list_all_metas(&self) -> Vec<JobMeta> {
        self.jobs.iter().map(|r| r.value().clone()).collect()
    }

    /// Update an existing job in-place, preserving its ID, `created_at`, and
    /// `last_run`. Recomputes `next_run` and resets `consecutive_errors`.
    pub fn update_job(
        &self,
        id: CronJobId,
        mut updated: CronJob,
        one_shot: bool,
    ) -> RustyHandResult<()> {
        match self.jobs.get_mut(&id) {
            Some(mut meta) => {
                // Preserve identity and history
                updated.id = id;
                updated.created_at = meta.job.created_at;
                updated.last_run = meta.job.last_run;
                updated.next_run = Some(compute_next_run(&updated.schedule));

                meta.job = updated;
                meta.one_shot = one_shot;
                meta.consecutive_errors = 0;
                Ok(())
            }
            None => Err(RustyHandError::Internal(format!("Cron job {id} not found"))),
        }
    }

    /// Total number of tracked jobs.
    pub fn total_jobs(&self) -> usize {
        self.jobs.len()
    }

    /// Return jobs whose `next_run` is at or before `now` and are enabled.
    ///
    /// **Atomically claims** each returned job by setting its `next_run` to
    /// `None`. This prevents the next tick from picking up the same job while
    /// it is still executing. The caller **must** call [`record_success`] or
    /// [`record_failure`] after execution to reschedule the job.
    pub fn claim_due_jobs(&self) -> Vec<CronJob> {
        let now = Utc::now();
        let mut due = Vec::new();
        for mut entry in self.jobs.iter_mut() {
            let meta = entry.value_mut();
            if meta.job.enabled && meta.job.next_run.map(|t| t <= now).unwrap_or(false) {
                // Claim: clear next_run so the next tick won't re-fire this job.
                meta.job.next_run = None;
                due.push(meta.job.clone());
            }
        }
        due
    }

    // -- Outcome recording --------------------------------------------------

    /// Record a successful execution for a job.
    ///
    /// Updates `last_run`, resets errors, and either removes the job (if
    /// one-shot) or advances `next_run`.
    pub fn record_success(&self, id: CronJobId) {
        // We need to check one_shot first, then potentially remove.
        let should_remove = {
            if let Some(mut meta) = self.jobs.get_mut(&id) {
                meta.job.last_run = Some(Utc::now());
                meta.last_status = Some("ok".to_string());
                meta.consecutive_errors = 0;
                if meta.one_shot {
                    true
                } else {
                    meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
                    false
                }
            } else {
                return;
            }
        };
        if should_remove {
            self.jobs.remove(&id);
        }
    }

    /// Record a failed execution for a job.
    ///
    /// If `transient` is `true` (e.g. LLM provider temporarily down, rate
    /// limited, timed out), the error counter is **not** incremented — the
    /// job is simply rescheduled for its next interval so it can retry when
    /// the provider recovers.
    ///
    /// If `transient` is `false` (permanent errors like auth or billing),
    /// the consecutive error counter is incremented and the job is
    /// auto-disabled after [`MAX_CONSECUTIVE_ERRORS`] consecutive permanent
    /// failures.
    pub fn record_failure(&self, id: CronJobId, error_msg: &str, transient: bool) {
        if let Some(mut meta) = self.jobs.get_mut(&id) {
            meta.job.last_run = Some(Utc::now());
            let truncated: String = error_msg.chars().take(256).collect();
            meta.last_status = Some(format!("error: {truncated}"));
            if transient {
                // Transient error (provider down, rate-limit, timeout) — reschedule
                // without counting toward the auto-disable threshold.
                debug!(
                    job_id = %id,
                    "Transient failure, rescheduling without incrementing error count"
                );
                meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
            } else {
                meta.consecutive_errors += 1;
                if meta.consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    warn!(
                        job_id = %id,
                        errors = meta.consecutive_errors,
                        "Auto-disabling cron job after repeated permanent failures"
                    );
                    meta.job.enabled = false;
                } else {
                    meta.job.next_run = Some(compute_next_run(&meta.job.schedule));
                }
            }
        }
    }

    /// Check whether an error message indicates a transient (retryable) failure.
    ///
    /// Transient failures include LLM provider rate-limits, overloads, timeouts,
    /// cooldowns, and network errors. The job should be rescheduled without
    /// counting toward the auto-disable threshold.
    pub fn is_transient_error(error_msg: &str) -> bool {
        let lower = error_msg.to_lowercase();
        // LLM provider transient errors (from call_with_retry / llm_errors classifier)
        lower.contains("rate limit")
            || lower.contains("overload")
            || lower.contains("cooldown")
            || lower.contains("timed out")
            || lower.contains("timeout")
            || lower.contains("econnreset")
            || lower.contains("etimedout")
            || lower.contains("econnrefused")
            || lower.contains("service unavailable")
            || lower.contains("502")
            || lower.contains("503")
            || lower.contains("504")
    }
}

// ---------------------------------------------------------------------------
// compute_next_run
// ---------------------------------------------------------------------------

/// Compute the next fire time for a schedule.
///
/// - `At { at }` — returns `at` directly.
/// - `Every { every_secs }` — returns `now + every_secs`.
/// - `Cron { expr, .. }` — parses the 5-field cron expression and returns the
///   next matching time after `now`.
pub fn compute_next_run(schedule: &CronSchedule) -> chrono::DateTime<Utc> {
    match schedule {
        CronSchedule::At { at } => *at,
        CronSchedule::Every { every_secs } => Utc::now() + Duration::seconds(*every_secs as i64),
        CronSchedule::Cron { expr, .. } => {
            // The `cron` crate expects 7 fields (sec min hour dom mon dow year)
            // but standard cron uses 5 (min hour dom mon dow). Prepend "0 " for
            // seconds and append " *" for year.
            let seven_field = format!("0 {expr} *");
            match cron::Schedule::from_str(&seven_field) {
                Ok(sched) => sched
                    .upcoming(Utc)
                    .next()
                    .unwrap_or_else(|| Utc::now() + Duration::hours(1)),
                Err(e) => {
                    warn!(expr, error = %e, "Invalid cron expression, falling back to 1h");
                    Utc::now() + Duration::hours(1)
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use rusty_hand_types::scheduler::{CronAction, CronDelivery};

    /// Build a minimal valid `CronJob` with an `Every` schedule.
    fn make_job(agent_id: AgentId) -> CronJob {
        CronJob {
            id: CronJobId::new(),
            agent_id,
            name: "test-job".into(),
            enabled: true,
            schedule: CronSchedule::Every { every_secs: 3600 },
            action: CronAction::SystemEvent {
                text: "ping".into(),
            },
            delivery: CronDelivery::None,
            created_at: Utc::now(),
            last_run: None,
            next_run: None,
        }
    }

    /// Create a scheduler backed by a temp directory.
    fn make_scheduler(max_total: usize) -> (CronScheduler, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let sched = CronScheduler::new(tmp.path(), max_total);
        (sched, tmp)
    }

    // -- test_add_job_and_list ----------------------------------------------

    #[test]
    fn test_add_job_and_list() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);

        let id = sched.add_job(job, false).unwrap();

        // Should appear in agent list
        let jobs = sched.list_jobs(agent);
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].id, id);
        assert_eq!(jobs[0].name, "test-job");

        // Should appear in global list
        let all = sched.list_all_jobs();
        assert_eq!(all.len(), 1);

        // get_job should return it
        let fetched = sched.get_job(id).unwrap();
        assert_eq!(fetched.agent_id, agent);

        // next_run should have been computed
        assert!(fetched.next_run.is_some());
        assert_eq!(sched.total_jobs(), 1);
    }

    // -- test_remove_job ----------------------------------------------------

    #[test]
    fn test_remove_job() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        let removed = sched.remove_job(id).unwrap();
        assert_eq!(removed.name, "test-job");
        assert_eq!(sched.total_jobs(), 0);

        // Removing again should fail
        assert!(sched.remove_job(id).is_err());
    }

    // -- test_add_job_global_limit ------------------------------------------

    #[test]
    fn test_add_job_global_limit() {
        let (sched, _tmp) = make_scheduler(2);
        let agent = AgentId::new();

        let j1 = make_job(agent);
        let j2 = make_job(agent);
        let j3 = make_job(agent);

        sched.add_job(j1, false).unwrap();
        sched.add_job(j2, false).unwrap();

        // Third should hit global limit
        let err = sched.add_job(j3, false).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("limit"),
            "Expected global limit error, got: {msg}"
        );
    }

    // -- test_add_job_per_agent_limit ---------------------------------------

    #[test]
    fn test_add_job_per_agent_limit() {
        // MAX_JOBS_PER_AGENT = 50 in rusty-hand-types
        let (sched, _tmp) = make_scheduler(1000);
        let agent = AgentId::new();

        for i in 0..50 {
            let mut job = make_job(agent);
            job.name = format!("job-{i}");
            sched.add_job(job, false).unwrap();
        }

        // 51st should be rejected by validate()
        let mut overflow = make_job(agent);
        overflow.name = "overflow".into();
        let err = sched.add_job(overflow, false).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("50"),
            "Expected per-agent limit error, got: {msg}"
        );
    }

    // -- test_record_success_removes_one_shot --------------------------------

    #[test]
    fn test_record_success_removes_one_shot() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, true).unwrap(); // one_shot = true

        assert_eq!(sched.total_jobs(), 1);

        sched.record_success(id);

        // One-shot job should have been removed
        assert_eq!(sched.total_jobs(), 0);
        assert!(sched.get_job(id).is_none());
    }

    #[test]
    fn test_record_success_keeps_recurring() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap(); // one_shot = false

        sched.record_success(id);

        // Recurring job should still be there
        assert_eq!(sched.total_jobs(), 1);
        let meta = sched.get_meta(id).unwrap();
        assert_eq!(meta.last_status.as_deref(), Some("ok"));
        assert_eq!(meta.consecutive_errors, 0);
        assert!(meta.job.last_run.is_some());
    }

    // -- test_record_failure_auto_disable -----------------------------------

    #[test]
    fn test_record_failure_auto_disable() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        // Fail MAX_CONSECUTIVE_ERRORS - 1 times: should still be enabled
        for i in 0..(MAX_CONSECUTIVE_ERRORS - 1) {
            sched.record_failure(id, &format!("error {i}"), false);
            let meta = sched.get_meta(id).unwrap();
            assert!(
                meta.job.enabled,
                "Job should still be enabled after {} failures",
                i + 1
            );
            assert_eq!(meta.consecutive_errors, i + 1);
        }

        // One more failure should auto-disable
        sched.record_failure(id, "final error", false);
        let meta = sched.get_meta(id).unwrap();
        assert!(
            !meta.job.enabled,
            "Job should be auto-disabled after {MAX_CONSECUTIVE_ERRORS} failures"
        );
        assert_eq!(meta.consecutive_errors, MAX_CONSECUTIVE_ERRORS);
        assert!(
            meta.last_status.as_ref().unwrap().starts_with("error:"),
            "last_status should record the error"
        );
    }

    #[test]
    fn test_transient_failures_never_auto_disable() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        // Record many transient failures — should never auto-disable
        for i in 0..20 {
            sched.record_failure(id, &format!("Rate limited attempt {i}"), true);
            let meta = sched.get_meta(id).unwrap();
            assert!(
                meta.job.enabled,
                "Transient failures must not disable the job"
            );
            assert_eq!(
                meta.consecutive_errors, 0,
                "Transient failures must not increment error count"
            );
            assert!(meta.job.next_run.is_some(), "Job should be rescheduled");
        }
    }

    #[test]
    fn test_is_transient_error_classification() {
        // Transient patterns
        assert!(CronScheduler::is_transient_error(
            "Rate limited after 3 retries"
        ));
        assert!(CronScheduler::is_transient_error(
            "Model overloaded after 3 retries"
        ));
        assert!(CronScheduler::is_transient_error(
            "Provider 'minimax' is in cooldown (overloaded). Retry in 60s."
        ));
        assert!(CronScheduler::is_transient_error("timed out after 120s"));
        assert!(CronScheduler::is_transient_error("connection ECONNRESET"));
        assert!(CronScheduler::is_transient_error(
            "ETIMEDOUT connecting to api.minimax.chat"
        ));
        assert!(CronScheduler::is_transient_error("503 Service Unavailable"));

        // Permanent patterns — should NOT be transient
        assert!(!CronScheduler::is_transient_error("Invalid API key"));
        assert!(!CronScheduler::is_transient_error("Insufficient credits"));
        assert!(!CronScheduler::is_transient_error(
            "Model not found: abab6.5s-chat"
        ));
        assert!(!CronScheduler::is_transient_error("malformed request body"));
    }

    // -- test_due_jobs_only_enabled -----------------------------------------

    #[test]
    fn test_due_jobs_only_enabled() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();

        // Job 1: enabled, next_run in the past
        let mut j1 = make_job(agent);
        j1.name = "enabled-due".into();
        let id1 = sched.add_job(j1, false).unwrap();

        // Job 2: disabled
        let mut j2 = make_job(agent);
        j2.name = "disabled-job".into();
        let id2 = sched.add_job(j2, false).unwrap();
        sched.set_enabled(id2, false).unwrap();

        // Force job 1's next_run to the past
        if let Some(mut meta) = sched.jobs.get_mut(&id1) {
            meta.job.next_run = Some(Utc::now() - Duration::seconds(10));
        }

        // Force job 2's next_run to the past too (but it's disabled)
        if let Some(mut meta) = sched.jobs.get_mut(&id2) {
            meta.job.next_run = Some(Utc::now() - Duration::seconds(10));
        }

        let due = sched.claim_due_jobs();
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].name, "enabled-due");
    }

    #[test]
    fn test_due_jobs_future_not_included() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();

        let job = make_job(agent);
        sched.add_job(job, false).unwrap();

        // The job was just added with next_run = now + 3600s, so it should
        // not be due yet.
        let due = sched.claim_due_jobs();
        assert!(due.is_empty());
    }

    // -- test_set_enabled ---------------------------------------------------

    #[test]
    fn test_set_enabled() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();

        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        // Disable
        sched.set_enabled(id, false).unwrap();
        let meta = sched.get_meta(id).unwrap();
        assert!(!meta.job.enabled);

        // Re-enable resets error count
        sched.record_failure(id, "ignored because disabled", false);
        // Actually the job is disabled so record_failure still updates it.
        // Let's first re-enable to test reset.
        sched.set_enabled(id, true).unwrap();
        let meta = sched.get_meta(id).unwrap();
        assert!(meta.job.enabled);
        assert_eq!(meta.consecutive_errors, 0);
        assert!(meta.job.next_run.is_some());

        // Non-existent ID should fail
        let fake_id = CronJobId::new();
        assert!(sched.set_enabled(fake_id, true).is_err());
    }

    // -- test_persist_and_load ----------------------------------------------

    #[test]
    fn test_persist_and_load() {
        let tmp = tempfile::tempdir().unwrap();
        let agent = AgentId::new();

        // Create scheduler, add jobs, persist
        {
            let sched = CronScheduler::new(tmp.path(), 100);
            let mut j1 = make_job(agent);
            j1.name = "persist-a".into();
            let mut j2 = make_job(agent);
            j2.name = "persist-b".into();

            sched.add_job(j1, false).unwrap();
            sched.add_job(j2, true).unwrap(); // one_shot

            sched.persist().unwrap();
        }

        // Create a new scheduler and load from disk
        {
            let sched = CronScheduler::new(tmp.path(), 100);
            let count = sched.load().unwrap();
            assert_eq!(count, 2);
            assert_eq!(sched.total_jobs(), 2);

            let jobs = sched.list_jobs(agent);
            assert_eq!(jobs.len(), 2);

            let names: Vec<&str> = jobs.iter().map(|j| j.name.as_str()).collect();
            assert!(names.contains(&"persist-a"));
            assert!(names.contains(&"persist-b"));

            // Verify one_shot flag was preserved
            let b_id = jobs.iter().find(|j| j.name == "persist-b").unwrap().id;
            let meta = sched.get_meta(b_id).unwrap();
            assert!(meta.one_shot);
        }
    }

    #[test]
    fn test_load_no_file_returns_zero() {
        let tmp = tempfile::tempdir().unwrap();
        let sched = CronScheduler::new(tmp.path(), 100);
        assert_eq!(sched.load().unwrap(), 0);
    }

    // -- compute_next_run ---------------------------------------------------

    #[test]
    fn test_compute_next_run_at() {
        let target = Utc::now() + Duration::hours(2);
        let schedule = CronSchedule::At { at: target };
        let next = compute_next_run(&schedule);
        assert_eq!(next, target);
    }

    #[test]
    fn test_compute_next_run_every() {
        let before = Utc::now();
        let schedule = CronSchedule::Every { every_secs: 300 };
        let next = compute_next_run(&schedule);
        let after = Utc::now();

        // Should be roughly now + 300s
        assert!(next >= before + Duration::seconds(300));
        assert!(next <= after + Duration::seconds(300));
    }

    #[test]
    fn test_compute_next_run_cron_real() {
        let now = Utc::now();
        let schedule = CronSchedule::Cron {
            expr: "0 9 * * *".into(), // daily at 09:00
            tz: None,
        };
        let next = compute_next_run(&schedule);

        // Must be in the future and within 24h (daily schedule)
        assert!(next > now, "next_run should be in the future");
        assert!(
            next <= now + Duration::hours(24),
            "daily cron should fire within 24h"
        );
        // Must be at minute 0, hour 9
        assert_eq!(next.format("%H:%M").to_string(), "09:00");
    }

    #[test]
    fn test_compute_next_run_cron_invalid_fallback() {
        let now = Utc::now();
        let schedule = CronSchedule::Cron {
            expr: "not a cron".into(),
            tz: None,
        };
        let next = compute_next_run(&schedule);

        // Invalid expression falls back to ~1h from now
        assert!(next > now);
        assert!(next <= now + Duration::hours(1) + Duration::seconds(5));
    }

    // -- error message truncation in record_failure -------------------------

    #[test]
    fn test_record_failure_truncates_long_error() {
        let (sched, _tmp) = make_scheduler(100);
        let agent = AgentId::new();
        let job = make_job(agent);
        let id = sched.add_job(job, false).unwrap();

        let long_error = "x".repeat(1000);
        sched.record_failure(id, &long_error, false);

        let meta = sched.get_meta(id).unwrap();
        let status = meta.last_status.unwrap();
        // "error: " is 7 chars + 256 chars of truncated message = 263 max
        assert!(
            status.len() <= 263,
            "Status should be truncated, got {} chars",
            status.len()
        );
    }
}
