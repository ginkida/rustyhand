//! Execution approval manager — gates dangerous operations behind human approval.

use chrono::Utc;
use dashmap::DashMap;
use rusty_hand_types::approval::{
    ApprovalDecision, ApprovalPolicy, ApprovalRequest, ApprovalResponse, RiskLevel,
};
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Max pending requests per agent.
const MAX_PENDING_PER_AGENT: usize = 5;

/// Callback type for push notifications when new approvals are created.
pub type ApprovalNotifyCallback = Arc<dyn Fn(&ApprovalRequest) + Send + Sync>;

/// Manages approval requests with oneshot channels for blocking resolution.
pub struct ApprovalManager {
    pending: DashMap<Uuid, PendingRequest>,
    policy: std::sync::RwLock<ApprovalPolicy>,
    /// Optional callback fired when a new approval request is created.
    /// Used by the channel bridge to push notifications to Telegram/etc.
    on_new_request: std::sync::RwLock<Option<ApprovalNotifyCallback>>,
}

struct PendingRequest {
    request: ApprovalRequest,
    sender: tokio::sync::oneshot::Sender<ApprovalDecision>,
}

impl ApprovalManager {
    pub fn new(policy: ApprovalPolicy) -> Self {
        Self {
            pending: DashMap::new(),
            policy: std::sync::RwLock::new(policy),
            on_new_request: std::sync::RwLock::new(None),
        }
    }

    /// Register a callback that fires when a new approval request is created.
    ///
    /// Used by the channel bridge to push inline-keyboard notifications
    /// to Telegram/Discord/etc. without polling.
    pub fn set_notification_callback(&self, cb: ApprovalNotifyCallback) {
        *self
            .on_new_request
            .write()
            .unwrap_or_else(|e| e.into_inner()) = Some(cb);
    }

    /// Check if a tool requires approval based on current policy.
    pub fn requires_approval(&self, tool_name: &str) -> bool {
        let policy = self.policy.read().unwrap_or_else(|e| e.into_inner());
        policy.require_approval.iter().any(|t| t == tool_name)
    }

    /// Submit an approval request. Returns a future that resolves when approved/denied/timed out.
    pub async fn request_approval(&self, req: ApprovalRequest) -> ApprovalDecision {
        // Trust mode: when `policy.auto_approve_autonomous = true` the
        // operator has explicitly opted into auto-approving every request
        // (single-operator setups, trusted local agents, CI/CD pipelines).
        // We log every auto-approval at WARN level so it's visible in
        // operations logs — silent auto-approval would be a security
        // footgun. Pre-v0.7.20 this field was dead code: the policy
        // surface accepted it but no call site consulted it.
        {
            let policy = self.policy.read().unwrap_or_else(|e| e.into_inner());
            if policy.auto_approve_autonomous {
                warn!(
                    request_id = %req.id,
                    agent_id = %req.agent_id,
                    tool = %req.tool_name,
                    risk = ?req.risk_level,
                    summary = %req.action_summary,
                    "AUTO-APPROVING request (auto_approve_autonomous=true) \u{2014} \
                     review trust mode in your config if this was unintended"
                );
                return ApprovalDecision::Approved;
            }
        }

        // Check per-agent pending limit
        let agent_pending = self
            .pending
            .iter()
            .filter(|r| r.value().request.agent_id == req.agent_id)
            .count();
        if agent_pending >= MAX_PENDING_PER_AGENT {
            warn!(agent_id = %req.agent_id, "Approval request rejected: too many pending");
            return ApprovalDecision::Denied;
        }

        let timeout = std::time::Duration::from_secs(req.timeout_secs);
        let id = req.id;

        let (tx, rx) = tokio::sync::oneshot::channel();
        self.pending.insert(
            id,
            PendingRequest {
                request: req,
                sender: tx,
            },
        );

        info!(request_id = %id, "Approval request submitted, waiting for resolution");

        // Fire notification callback BEFORE blocking on the oneshot.
        // This lets the bridge push a Telegram message with approve/reject buttons.
        // NOTE: callback must be non-blocking (it runs on the agent's tokio task).
        {
            let guard = self
                .on_new_request
                .read()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(cb) = guard.as_ref() {
                if let Some(entry) = self.pending.get(&id) {
                    cb(&entry.value().request);
                }
            }
        }

        match tokio::time::timeout(timeout, rx).await {
            Ok(Ok(decision)) => {
                debug!(request_id = %id, ?decision, "Approval resolved");
                decision
            }
            _ => {
                self.pending.remove(&id);
                warn!(request_id = %id, "Approval request timed out");
                ApprovalDecision::TimedOut
            }
        }
    }

    /// Resolve a pending request (called by API/UI).
    pub fn resolve(
        &self,
        request_id: Uuid,
        decision: ApprovalDecision,
        decided_by: Option<String>,
    ) -> Result<ApprovalResponse, String> {
        match self.pending.remove(&request_id) {
            Some((_, pending)) => {
                let response = ApprovalResponse {
                    request_id,
                    decision,
                    decided_at: Utc::now(),
                    decided_by,
                };
                // Send decision to waiting agent (ignore error if receiver dropped)
                let _ = pending.sender.send(decision);
                info!(request_id = %request_id, ?decision, "Approval request resolved");
                Ok(response)
            }
            None => Err(format!("No pending approval request with id {request_id}")),
        }
    }

    /// List all pending requests (for API/dashboard display).
    pub fn list_pending(&self) -> Vec<ApprovalRequest> {
        self.pending
            .iter()
            .map(|r| r.value().request.clone())
            .collect()
    }

    /// Number of pending requests.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Update the approval policy (for hot-reload).
    pub fn update_policy(&self, policy: ApprovalPolicy) {
        *self.policy.write().unwrap_or_else(|e| e.into_inner()) = policy;
    }

    /// Get a copy of the current policy.
    pub fn policy(&self) -> ApprovalPolicy {
        self.policy
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Classify the risk level of a tool invocation.
    pub fn classify_risk(tool_name: &str) -> RiskLevel {
        match tool_name {
            "shell_exec" => RiskLevel::Critical,
            "file_write" | "file_delete" => RiskLevel::High,
            "web_fetch" | "browser_navigate" => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_hand_types::approval::ApprovalPolicy;
    use std::sync::Arc;

    fn default_manager() -> ApprovalManager {
        ApprovalManager::new(ApprovalPolicy::default())
    }

    /// Manager with the historical interactive-prompt policy (trust mode
    /// off). Tests that exercise the queue / oneshot / pending / timeout
    /// paths need this — under the v0.7.21 default, every request is
    /// auto-approved instantly so those code paths are unreachable.
    fn interactive_manager() -> ApprovalManager {
        ApprovalManager::new(ApprovalPolicy {
            require_approval: vec!["shell_exec".to_string()],
            timeout_secs: 60,
            auto_approve_autonomous: false,
        })
    }

    fn make_request(agent_id: &str, tool_name: &str, timeout_secs: u64) -> ApprovalRequest {
        ApprovalRequest {
            id: Uuid::new_v4(),
            agent_id: agent_id.to_string(),
            tool_name: tool_name.to_string(),
            description: "test operation".to_string(),
            action_summary: "test action".to_string(),
            risk_level: RiskLevel::High,
            requested_at: Utc::now(),
            timeout_secs,
        }
    }

    // -----------------------------------------------------------------------
    // requires_approval
    // -----------------------------------------------------------------------

    #[test]
    fn test_requires_approval_default() {
        let mgr = default_manager();
        assert!(mgr.requires_approval("shell_exec"));
        assert!(!mgr.requires_approval("file_read"));
    }

    #[test]
    fn test_requires_approval_custom_policy() {
        let policy = ApprovalPolicy {
            require_approval: vec!["file_write".to_string(), "file_delete".to_string()],
            timeout_secs: 30,
            auto_approve_autonomous: false,
        };
        let mgr = ApprovalManager::new(policy);
        assert!(mgr.requires_approval("file_write"));
        assert!(mgr.requires_approval("file_delete"));
        assert!(!mgr.requires_approval("shell_exec"));
        assert!(!mgr.requires_approval("file_read"));
    }

    // -----------------------------------------------------------------------
    // classify_risk
    // -----------------------------------------------------------------------

    #[test]
    fn test_classify_risk() {
        assert_eq!(
            ApprovalManager::classify_risk("shell_exec"),
            RiskLevel::Critical
        );
        assert_eq!(
            ApprovalManager::classify_risk("file_write"),
            RiskLevel::High
        );
        assert_eq!(
            ApprovalManager::classify_risk("file_delete"),
            RiskLevel::High
        );
        assert_eq!(
            ApprovalManager::classify_risk("web_fetch"),
            RiskLevel::Medium
        );
        assert_eq!(
            ApprovalManager::classify_risk("browser_navigate"),
            RiskLevel::Medium
        );
        assert_eq!(ApprovalManager::classify_risk("file_read"), RiskLevel::Low);
        assert_eq!(
            ApprovalManager::classify_risk("unknown_tool"),
            RiskLevel::Low
        );
    }

    // -----------------------------------------------------------------------
    // resolve nonexistent
    // -----------------------------------------------------------------------

    #[test]
    fn test_resolve_nonexistent() {
        let mgr = default_manager();
        let result = mgr.resolve(Uuid::new_v4(), ApprovalDecision::Approved, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No pending approval request"));
    }

    // -----------------------------------------------------------------------
    // list_pending empty
    // -----------------------------------------------------------------------

    #[test]
    fn test_list_pending_empty() {
        let mgr = default_manager();
        assert!(mgr.list_pending().is_empty());
    }

    // -----------------------------------------------------------------------
    // update_policy
    // -----------------------------------------------------------------------

    #[test]
    fn test_update_policy() {
        let mgr = default_manager();
        assert!(mgr.requires_approval("shell_exec"));
        assert!(!mgr.requires_approval("file_write"));

        let new_policy = ApprovalPolicy {
            require_approval: vec!["file_write".to_string()],
            timeout_secs: 120,
            auto_approve_autonomous: true,
        };
        mgr.update_policy(new_policy);

        assert!(!mgr.requires_approval("shell_exec"));
        assert!(mgr.requires_approval("file_write"));

        let policy = mgr.policy();
        assert_eq!(policy.timeout_secs, 120);
        assert!(policy.auto_approve_autonomous);
    }

    // -----------------------------------------------------------------------
    // pending_count
    // -----------------------------------------------------------------------

    #[test]
    fn test_pending_count() {
        let mgr = default_manager();
        assert_eq!(mgr.pending_count(), 0);
    }

    // -----------------------------------------------------------------------
    // request_approval — timeout
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_request_approval_timeout() {
        let mgr = Arc::new(interactive_manager());
        let req = make_request("agent-1", "shell_exec", 10);
        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::TimedOut);
        // After timeout, pending map should be cleaned up
        assert_eq!(mgr.pending_count(), 0);
    }

    // -----------------------------------------------------------------------
    // request_approval — approve
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_request_approval_approve() {
        let mgr = Arc::new(interactive_manager());
        let req = make_request("agent-1", "shell_exec", 60);
        let request_id = req.id;

        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            // Small delay to let the request register
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let result = mgr2.resolve(
                request_id,
                ApprovalDecision::Approved,
                Some("admin".to_string()),
            );
            assert!(result.is_ok());
            let resp = result.unwrap();
            assert_eq!(resp.decision, ApprovalDecision::Approved);
            assert_eq!(resp.decided_by, Some("admin".to_string()));
        });

        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::Approved);
    }

    // -----------------------------------------------------------------------
    // request_approval — deny
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_request_approval_deny() {
        let mgr = Arc::new(interactive_manager());
        let req = make_request("agent-1", "shell_exec", 60);
        let request_id = req.id;

        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let result = mgr2.resolve(request_id, ApprovalDecision::Denied, None);
            assert!(result.is_ok());
        });

        let decision = mgr.request_approval(req).await;
        assert_eq!(decision, ApprovalDecision::Denied);
    }

    // -----------------------------------------------------------------------
    // max pending per agent
    // -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_max_pending_per_agent() {
        let mgr = Arc::new(interactive_manager());

        // Fill up 5 pending requests for agent-1 (they will all be waiting)
        let mut ids = Vec::new();
        for _ in 0..MAX_PENDING_PER_AGENT {
            let req = make_request("agent-1", "shell_exec", 300);
            ids.push(req.id);
            let mgr_clone = Arc::clone(&mgr);
            tokio::spawn(async move {
                mgr_clone.request_approval(req).await;
            });
        }

        // Give spawned tasks time to register
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert_eq!(mgr.pending_count(), MAX_PENDING_PER_AGENT);

        // 6th request for the same agent should be immediately denied
        let req6 = make_request("agent-1", "shell_exec", 300);
        let decision = mgr.request_approval(req6).await;
        assert_eq!(decision, ApprovalDecision::Denied);

        // A different agent should still be able to submit
        let req_other = make_request("agent-2", "shell_exec", 300);
        let other_id = req_other.id;
        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            mgr2.request_approval(req_other).await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert_eq!(mgr.pending_count(), MAX_PENDING_PER_AGENT + 1);

        // Cleanup: resolve all pending to avoid hanging tasks
        for id in &ids {
            let _ = mgr.resolve(*id, ApprovalDecision::Denied, None);
        }
        let _ = mgr.resolve(other_id, ApprovalDecision::Denied, None);
    }

    // -----------------------------------------------------------------------
    // policy defaults
    // -----------------------------------------------------------------------

    #[test]
    fn test_policy_defaults() {
        let mgr = default_manager();
        let policy = mgr.policy();
        assert_eq!(policy.require_approval, vec!["shell_exec".to_string()]);
        assert_eq!(policy.timeout_secs, 300);
        // v0.7.21: trust mode is the default; see ApprovalPolicy doc.
        assert!(policy.auto_approve_autonomous);
    }

    /// Regression: pre-v0.7.20 `auto_approve_autonomous` was a defined
    /// field on `ApprovalPolicy` but no call site consulted it — setting
    /// it to `true` in config did nothing. This test pins the new
    /// behavior: when trust mode is enabled, every request is approved
    /// instantly without prompting. The Telegram/Discord notification
    /// callback is bypassed, the per-agent pending limit is bypassed,
    /// and no oneshot wait happens.
    #[tokio::test]
    async fn auto_approve_autonomous_skips_prompt() {
        let trust_policy = ApprovalPolicy {
            require_approval: vec!["shell_exec".to_string()],
            timeout_secs: 300,
            auto_approve_autonomous: true,
        };
        let mgr = ApprovalManager::new(trust_policy);

        // Notification callback must NOT fire in trust mode — the whole
        // point is to skip the human-in-the-loop hop.
        let cb_fired = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cb_flag = cb_fired.clone();
        mgr.set_notification_callback(Arc::new(move |_| {
            cb_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        }));

        let req = make_request("agent-trust", "shell_exec", 300);
        let decision = mgr.request_approval(req).await;

        assert_eq!(
            decision,
            ApprovalDecision::Approved,
            "trust mode must auto-approve"
        );
        assert!(
            !cb_fired.load(std::sync::atomic::Ordering::SeqCst),
            "trust mode must not fire the notification callback (no Telegram prompt)"
        );
        assert_eq!(
            mgr.pending_count(),
            0,
            "trust mode must not leave a pending entry behind"
        );
    }

    /// Trust mode must respect a hot-update: toggling between
    /// interactive and trust modes must take effect immediately.
    #[tokio::test]
    async fn auto_approve_autonomous_respects_hot_update() {
        let mgr = Arc::new(interactive_manager());

        // First request: interactive policy — no auto-approve. Resolve manually.
        let req1 = make_request("agent-1", "shell_exec", 60);
        let id1 = req1.id;
        let mgr2 = Arc::clone(&mgr);
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            let _ = mgr2.resolve(id1, ApprovalDecision::Approved, None);
        });
        assert_eq!(mgr.request_approval(req1).await, ApprovalDecision::Approved);

        // Hot-toggle to trust mode.
        mgr.update_policy(ApprovalPolicy {
            require_approval: vec!["shell_exec".to_string()],
            timeout_secs: 300,
            auto_approve_autonomous: true,
        });

        // Second request: must be auto-approved instantly without anyone
        // calling resolve() — proves the toggle took effect.
        let req2 = make_request("agent-1", "shell_exec", 300);
        assert_eq!(mgr.request_approval(req2).await, ApprovalDecision::Approved);
        assert_eq!(mgr.pending_count(), 0);
    }
}
