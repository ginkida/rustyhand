//! Contract tests for the React control-panel dashboard served at `GET /`.
//!
//! The dashboard is assembled in `webchat.rs` from `static/index_head.html`,
//! `static/css/panel.css`, the precompiled JSX modules under
//! `static/js/panel/`, and the React UMD bundles vendored from npm. Each
//! piece is `include_str!`'d at compile time so the binary is fully
//! self-contained — no CDN, no runtime download.
//!
//! These tests pin the assembled HTML so a misplaced rename or a missing
//! script tag fails CI instead of shipping a broken dashboard.
//!
//! Detection-only tests — they hit the static handler, not the kernel, so
//! they need no test server boot.
//!
//! Background: v0.7.45 replaced the Alpine.js dashboard with a precompiled
//! React 18 panel. The risk during the rewrite was that one of the script
//! tags or the React UMD bundle could silently go missing and the page
//! would render as a blank `<div id="root">`. These assertions make that
//! failure mode loud.
use axum::body::to_bytes;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use rusty_hand_api::webchat;

async fn fetch_dashboard() -> (StatusCode, String) {
    let resp = webchat::webchat_page().await.into_response();
    let status = resp.status();
    let body = to_bytes(resp.into_body(), 4 * 1024 * 1024)
        .await
        .expect("read body");
    (status, String::from_utf8(body.to_vec()).expect("utf8 body"))
}

#[tokio::test]
async fn dashboard_serves_with_200_and_html_content_type() {
    let (status, body) = fetch_dashboard().await;
    assert_eq!(status, StatusCode::OK, "GET / should be 200");
    assert!(
        body.starts_with("<!DOCTYPE html>"),
        "dashboard must start with HTML doctype, got: {}",
        &body[..body.len().min(120)]
    );
    assert!(
        body.contains("<title>RustyHand · Control Panel</title>"),
        "dashboard title must reflect the new control panel name"
    );
}

#[tokio::test]
async fn dashboard_contains_react_root_mount_point() {
    let (_, body) = fetch_dashboard().await;
    assert!(
        body.contains("<div id=\"root\"></div>"),
        "React panel mounts into #root — the div must be in the body"
    );
    assert!(
        body.contains("ReactDOM.createRoot"),
        "app.js mounts via ReactDOM.createRoot; if this is missing the bundle won't render"
    );
}

#[tokio::test]
async fn dashboard_bundles_react_and_reactdom_inline() {
    let (_, body) = fetch_dashboard().await;
    // The React UMD bundles are recognizable by these well-known globals
    // and string literals. They're vendored from npm at static/vendor/.
    assert!(
        body.contains("react.production.min.js")
            || body.contains("__SECRET_INTERNALS_DO_NOT_USE_OR_YOU_WILL_BE_FIRED"),
        "React production runtime must be inlined into the page"
    );
    assert!(
        body.contains("react-dom.production.min.js") || body.contains("createRoot"),
        "ReactDOM production runtime must be inlined into the page"
    );
}

#[tokio::test]
async fn dashboard_includes_all_panel_modules() {
    let (_, body) = fetch_dashboard().await;
    // Every panel module exposes a recognizable global on `window`. If any
    // include_str path is wrong or a file is missing these strings vanish.
    let markers = [
        ("RH_DATA", "data.js must expose window.RH_DATA"),
        ("rhFetch", "api.js must expose rhFetch"),
        ("useApi", "api.js must expose useApi"),
        ("usePolling", "api.js must expose usePolling"),
        ("normalizeAgent", "api.js must expose normalizeAgent"),
        ("ChannelIcon", "icons.js must expose ChannelIcon"),
        ("useTweaks", "tweaks-panel.js must expose useTweaks"),
        ("OverviewPage", "pages.js must expose OverviewPage"),
        ("AgentsPage", "pages.js must expose AgentsPage"),
        ("ChatPage", "pages.js must expose ChatPage"),
        ("WorkflowsPage", "pages.js must expose WorkflowsPage"),
        ("AutomationPage", "pages.js must expose AutomationPage"),
        ("ChannelsPage", "pages.js must expose ChannelsPage"),
        ("AnalyticsPage", "pages.js must expose AnalyticsPage"),
        ("KnowledgePage", "pages.js must expose KnowledgePage"),
        ("SkillsPage", "pages.js must expose SkillsPage"),
        ("ApprovalsPage", "pages.js must expose ApprovalsPage"),
        ("AuditPage", "pages.js must expose AuditPage"),
        ("SettingsPage", "pages.js must expose SettingsPage"),
        ("MemoryPage", "pages.js must expose MemoryPage"),
        ("McpPage", "pages.js must expose McpPage"),
        ("NetworkPage", "pages.js must expose NetworkPage"),
        ("BindingsPage", "pages.js must expose BindingsPage"),
        ("HealthPage", "pages.js must expose HealthPage"),
        ("ToastHost", "api.js must expose ToastHost"),
        ("ErrorBoundary", "api.js must expose ErrorBoundary"),
        ("usePagination", "api.js must expose usePagination"),
        ("useEscapeKey", "api.js must expose useEscapeKey"),
        ("Pagination", "pages.js must expose Pagination component"),
        ("useHashRoute", "api.js must expose useHashRoute"),
        ("useAsyncAction", "api.js must expose useAsyncAction"),
        (
            "PageErrorBoundary",
            "app.js must expose per-page ErrorBoundary",
        ),
        ("ConfirmHost", "api.js must expose ConfirmHost"),
        ("confirmDialog", "api.js must expose confirmDialog"),
        ("Skel", "api.js must expose Skel"),
        ("SkelRow", "api.js must expose SkelRow"),
        ("loadRecentPicks", "palette must expose recent-picks helper"),
        ("ToolTraceCard", "chat must expose ToolTraceCard"),
        ("coalesceToolTraces", "chat must coalesce repeated tools"),
        (
            "WorkflowRunInspector",
            "workflows must expose run inspector",
        ),
        ("RunStepCard", "workflows must expose per-step card"),
        ("AgentRow", "agents must expose AgentRow"),
        ("HelpOverlay", "shortcuts help overlay must be present"),
        ("rh:hotkey:new", "hotkey dispatch wired"),
        ("Tip", "tooltip primitive must be exposed"),
        // Iter 35-37
        ("config-diff", "agent config diff card must render"),
        ("WorkflowImportModal", "workflow YAML import modal"),
        (
            "/api/workflows/import-yaml",
            "workflow YAML import endpoint",
        ),
        ("AgentActivityCharts", "agent drawer charts"),
        ("ApprovalContextModal", "approvals context modal"),
        ("audit-match", "audit search highlight"),
        // Iter 39-41
        ("OnboardingWizard", "first-launch onboarding wizard"),
        ("shouldShowOnboarding", "onboarding gating logic"),
        ("SkillDetailModal", "skill detail modal"),
        ("ChannelTestCard", "structured channel-test renderer"),
        ("Authorized users", "RBAC users section in Settings"),
        ("swatch", "identity color picker swatches"),
        ("rh.panel.agentsCompact", "Agents compact density toggle"),
        // Iter 43-45
        ("useEventSource", "SSE hook must be exposed"),
        ("/api/logs/stream", "live activity feed wired to SSE"),
        ("CHAT_SLASH_COMMANDS", "slash-command catalog"),
        ("ChatInput", "ChatInput component (slash popup host)"),
        ("forkAgent", "agent fork action wired"),
        ("AgentKvEditor", "per-agent KV editor"),
        ("exportWorkflowYaml", "workflow YAML export helper"),
        ("KnowledgeAddNodeModal", "knowledge add-node modal"),
        ("/api/knowledge/entities", "knowledge add-entity endpoint"),
        // Iter 47-49
        ("KnowledgeAddRelationModal", "knowledge add-relation modal"),
        (
            "/api/knowledge/relations",
            "knowledge add-relation endpoint",
        ),
        ("audit-pulse", "audit deep-link pulse highlight"),
        ("chat-resize", "chat column resize handles"),
        ("rh.panel.chatLeft", "chat width persistence"),
        ("LogLevelCard", "settings log verbosity card"),
        ("Preflight", "workflow run preflight section"),
    ];
    for (needle, msg) in markers {
        assert!(body.contains(needle), "{msg} (missing `{needle}`)");
    }
}

#[tokio::test]
async fn dashboard_inlines_panel_stylesheet() {
    let (_, body) = fetch_dashboard().await;
    // The design's industrial palette uses oklch() variables on :root —
    // if the CSS include path breaks, these vanish.
    assert!(
        body.contains("--rust:"),
        "panel.css must inline the --rust accent variable"
    );
    assert!(
        body.contains("oklch("),
        "panel.css must inline the oklch() color tokens"
    );
    assert!(
        body.contains(".sidebar") && body.contains(".topbar"),
        "panel.css must define the sidebar + topbar layout classes"
    );
}

#[tokio::test]
async fn dashboard_no_alpine_residue() {
    // Regression: the v0.7.44 dashboard was Alpine.js. The rewrite must not
    // leave dangling x-data/x-cloak/x-show directives or alpine.min.js.
    let (_, body) = fetch_dashboard().await;
    assert!(
        !body.contains("alpine.min.js"),
        "Alpine.js bundle must be gone after the React rewrite"
    );
    assert!(
        !body.contains(" x-data="),
        "Alpine x-data directives must be gone after the React rewrite"
    );
    assert!(
        !body.contains(" x-cloak"),
        "Alpine x-cloak directives must be gone after the React rewrite"
    );
}

#[tokio::test]
async fn dashboard_wires_every_kernel_endpoint_it_uses() {
    // The React panel consumes the kernel's REST API. If a page is
    // accidentally reverted to mock data, its corresponding fetch path
    // disappears from the bundle — caught here. Each endpoint below has
    // at least one page that fetches it; adding a page should add a
    // marker.
    let (_, body) = fetch_dashboard().await;
    let endpoints = [
        // Overview
        "/api/health/detail",
        "/api/agents",
        "/api/onboarding",
        "/api/audit/recent",
        "/api/usage/daily",
        "/api/approvals",
        "/api/providers",
        // Agents drawer
        "/api/budget/agents",
        // Chat
        "/api/tools",
        // Workflows
        "/api/workflows",
        // Automation
        "/api/cron/jobs",
        "/api/triggers",
        // Channels / Skills
        "/api/channels",
        "/api/skills",
        // Analytics extras
        "/api/usage/by-model",
        "/api/usage",
        // Knowledge / Settings / Audit
        "/api/knowledge",
        "/api/knowledge/query",
        "/api/config",
        "/api/audit/verify",
        // System pages (iter 20)
        "/api/mcp/servers",
        "/api/network/status",
        "/api/peers",
        "/api/bindings",
    ];
    for path in endpoints {
        assert!(
            body.contains(path),
            "panel must wire endpoint `{path}` — if it was removed, either restore the consumer or update this test"
        );
    }
}

#[tokio::test]
async fn dashboard_bundle_size_is_sane() {
    // The bundle inlines React + ReactDOM (~140KB) + ~140KB of panel JS
    // + ~25KB of design CSS. Under 100KB means we lost a script; over
    // 1MB means somebody accidentally inlined Babel-standalone or a
    // large vendor library, regressing page load.
    let (_, body) = fetch_dashboard().await;
    let kb = body.len() / 1024;
    assert!(
        kb > 100,
        "dashboard is suspiciously small ({kb} KB) — did an include_str! path break?"
    );
    assert!(
        kb < 1024,
        "dashboard is suspiciously large ({kb} KB) — did somebody inline a megabyte of vendor JS?"
    );
}

#[tokio::test]
async fn dashboard_wires_production_features() {
    // Iter 13-16 closed the gap on production-ready features: agent edit
    // tabs, ClawHub, login screen, memory browser, workflow visual
    // builder. Each marker below pins one of those features so they
    // can't silently regress.
    let (_, body) = fetch_dashboard().await;
    let markers = [
        // Iter 13: agent edit tabs
        (
            "AgentConfigForm",
            "Agent config inline editor (drawer Config tab)",
        ),
        (
            "AgentIdentityForm",
            "Agent identity inline editor (drawer Identity tab)",
        ),
        ("/api/agents/${agent.id}/config", "PATCH agent config"),
        ("/api/agents/${agent.id}/identity", "PATCH agent identity"),
        ("/api/agents/${agent.id}/model", "PUT agent model"),
        // Iter 14: ClawHub + knowledge filter
        ("ClawHubModal", "ClawHub browse modal"),
        ("/api/clawhub/browse", "ClawHub browse"),
        ("/api/clawhub/search", "ClawHub search"),
        ("/api/clawhub/install", "ClawHub install"),
        // Iter 15: workflow visual builder
        ("WorkflowStepCard", "Visual step builder card"),
        ("Add step", "Visual builder add-step affordance"),
        // Iter 16: login + memory
        ("LoginScreen", "Login screen"),
        ("AuthGate", "Authentication gate component"),
        ("MemoryPage", "Memory browser page"),
        ("setApiKey", "API key persistence helper"),
        ("/api/auth/me", "Auth probe endpoint"),
        (
            "/api/sessions/${encodeURIComponent(id)}",
            "Session detail / delete",
        ),
        (
            "/api/sessions/${encodeURIComponent(id)}/label",
            "Session label set",
        ),
        ("/api/memory/export", "Memory backup"),
        ("/api/memory/import", "Memory restore"),
        // Iter 18: true cypher endpoint + workflow DnD
        ("/api/knowledge/query", "Cypher query endpoint"),
        ("parseMiniCypher", "Cypher mini-DSL parser"),
        ("dnd-handle", "Workflow DnD handle"),
        ("onDragStart", "Workflow DnD wiring"),
    ];
    for (needle, what) in markers {
        assert!(
            body.contains(needle),
            "missing `{needle}` ({what}) — production-ready feature was reverted"
        );
    }
}

#[tokio::test]
async fn dashboard_wires_write_paths() {
    // Regression: in earlier iterations the dashboard had only read-side
    // polling and showed dead buttons for spawn/configure/install/etc.
    // These markers prove that each write-path string was *compiled into*
    // the bundle (i.e. the modal/action is present, not just an unwired
    // placeholder button). If somebody reverts a write-path component,
    // its corresponding string vanishes and this test fails loudly.
    let (_, body) = fetch_dashboard().await;
    let markers = [
        // Modals
        ("SpawnAgentModal", "Spawn agent flow"),
        ("ProviderKeyModal", "Provider key editor"),
        ("CronJobModal", "Cron job create form"),
        ("TriggerModal", "Trigger create form"),
        ("WorkflowCreateModal", "Workflow create form"),
        ("WorkflowRunModal", "Workflow run-with-input form"),
        ("ChannelConfigModal", "Channel configure form"),
        ("SkillInstallModal", "Custom skill install form"),
        ("CommandPalette", "Cmd+K palette"),
        // Helpers
        ("useAgentWs", "WebSocket chat hook"),
        ("renderMarkdown", "Markdown renderer in chat"),
        ("downloadBlob", "Blob download helper"),
        ("rowsToCsv", "CSV serializer"),
        // Write-paths
        (
            "/api/agents/${encodeURIComponent(id)}/restart",
            "Agent restart wiring",
        ),
        (
            "/api/providers/${encodeURIComponent(name)}/key",
            "Provider key set/delete",
        ),
        (
            "/api/providers/${encodeURIComponent(name)}/test",
            "Provider test endpoint",
        ),
        (
            "/api/cron/jobs/${encodeURIComponent(id)}/enable",
            "Cron toggle",
        ),
        (
            "/api/cron/jobs/${encodeURIComponent(id)}/run",
            "Cron run-now",
        ),
        (
            "/api/workflows/${encodeURIComponent(active.id)}/run",
            "Workflow run-now",
        ),
        (
            "/api/workflows/${encodeURIComponent(active.id)}/runs",
            "Workflow runs history",
        ),
        (
            "/api/channels/${encodeURIComponent(channel.name)}/configure",
            "Channel configure",
        ),
        (
            "/api/channels/${encodeURIComponent(name)}/test",
            "Channel test",
        ),
        ("/api/channels/reload", "Channel reload"),
        ("/api/skills/install-custom", "Custom skill install"),
        ("/api/skills/uninstall", "Skill uninstall"),
        // Iter 51-52: knowledge graph CRUD + approvals SSE + sidebar pinning
        (
            "/api/knowledge/entities/${encodeURIComponent(active.id)}",
            "Knowledge entity delete",
        ),
        (
            "/api/knowledge/relations/${encodeURIComponent(relId)}",
            "Knowledge relation delete",
        ),
        ("/api/approvals/stream", "Approvals SSE feed"),
        ("usePinnedNav", "Sidebar pinning hook"),
        ("bulkExport", "Memory bulk export"),
        ("bulkDelete", "Memory bulk delete"),
        // Iter 53-55: agent diff + workflow live run + narrow layout
        ("AgentDiffModal", "Agent vs agent manifest diff"),
        (
            "/api/agents/${encodeURIComponent(a.id)}/files/agent.toml",
            "Agent manifest fetch (for diff)",
        ),
        ("liveRun", "Workflow live-run polling toggle"),
        ("max-width: 1100px", "Sidebar icon-rail breakpoint"),
        ("max-width: 720px", "Mobile single-column breakpoint"),
        // Iter 57-59: approvals bulk ops + audit CSV + channel activity
        ("bulkDecide", "Approvals bulk approve/reject"),
        ("ChannelActivityCard", "Per-channel recent-activity drawer"),
        (
            "rustyhand-audit-${tag}",
            "Audit CSV export filename template",
        ),
        // Iter 61-63: bulk move-to-group + workflow delete + demo reseed + MCP tools
        ("bulkMoveToGroup", "Agents bulk move-to-group"),
        ("/api/onboarding/reset-demo", "Demo seed reset endpoint"),
        ("Re-seed on restart", "Settings demo reseed button"),
        // Iter 65-67: cron bulk toggle + audit level chips + config export
        ("bulkSetEnabled", "Cron bulk enable/disable"),
        ("auditLevelOf", "Audit level classifier"),
        ("rh.panel.auditLevel", "Audit level filter persistence"),
        ("/api/config/export", "Config TOML export endpoint"),
        // Iter 69-71: triggers bulk + bindings bulk + Health page
        ("bulkTrigSetEnabled", "Triggers bulk arm/disarm"),
        ("bulkRemove", "Bindings bulk remove"),
        ("HealthPage", "Health diagnostics page"),
        // Iter 73-75: clawhub peek + audit re-verify + palette quick-actions
        ("peekCache", "ClawHub peek detail cache"),
        ("forceReverify", "Audit force re-verify handler"),
        ("QUICK_ACTIONS", "Palette quick-action list"),
        ("rh:hotkey:new", "Palette quick-action dispatch"),
        // Iter 77-78: agent live metrics chart + workflow YAML inline editor
        ("AgentLiveSpark", "Per-agent live throughput sparkline"),
        ("WorkflowEditYamlModal", "Workflow YAML inline editor"),
        ("SAVE = DELETE + RECREATE", "Edit YAML modal warning banner"),
    ];
    for (needle, what) in markers {
        assert!(
            body.contains(needle),
            "missing `{needle}` ({what}) — the corresponding write-path was reverted"
        );
    }
}
