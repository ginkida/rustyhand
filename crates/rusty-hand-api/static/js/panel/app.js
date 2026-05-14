const { useState, useEffect } = React;
const TWEAKS_DEFAULTS = (
  /*EDITMODE-BEGIN*/
  {
    "theme": "dark",
    "accent": "rust",
    "density": "normal",
    "monoHeadings": true,
    "showDemoBanner": true
  }
);
const ACCENTS = {
  rust: { rust: "oklch(0.665 0.165 50)", rust2: "oklch(0.72 0.15 55)", dim: "oklch(0.42 0.10 50)" },
  copper: { rust: "oklch(0.65 0.14 35)", rust2: "oklch(0.71 0.13 38)", dim: "oklch(0.40 0.09 35)" },
  amber: { rust: "oklch(0.78 0.155 85)", rust2: "oklch(0.82 0.14 85)", dim: "oklch(0.46 0.10 85)" },
  forest: { rust: "oklch(0.66 0.15 155)", rust2: "oklch(0.72 0.14 155)", dim: "oklch(0.40 0.10 155)" },
  electric: { rust: "oklch(0.70 0.20 290)", rust2: "oklch(0.76 0.18 290)", dim: "oklch(0.42 0.13 290)" }
};
const NAV = [
  { kind: "section", label: "Work" },
  { id: "overview", label: "Overview", icon: /* @__PURE__ */ React.createElement(I.overview, null), count: null },
  { id: "agents", label: "Agents", icon: /* @__PURE__ */ React.createElement(I.agents, null), count: 10 },
  { id: "chat", label: "Chat", icon: /* @__PURE__ */ React.createElement(I.chat, null), count: null },
  { id: "approvals", label: "Approvals", icon: /* @__PURE__ */ React.createElement(I.approvals, null), count: 3, badge: "warn" },
  { kind: "section", label: "Build" },
  { id: "workflows", label: "Workflows", icon: /* @__PURE__ */ React.createElement(I.workflows, null), count: 5 },
  { id: "automation", label: "Automation", icon: /* @__PURE__ */ React.createElement(I.automation, null), count: 9 },
  { id: "channels", label: "Channels", icon: /* @__PURE__ */ React.createElement(I.channels, null), count: 4 },
  { id: "skills", label: "Skills", icon: /* @__PURE__ */ React.createElement(I.skills, null), count: 60 },
  { kind: "section", label: "Inspect" },
  { id: "analytics", label: "Analytics", icon: /* @__PURE__ */ React.createElement(I.analytics, null), count: null },
  { id: "knowledge", label: "Knowledge", icon: /* @__PURE__ */ React.createElement(I.knowledge, null), count: null },
  { id: "memory", label: "Memory", icon: /* @__PURE__ */ React.createElement(I.audit, null), count: null },
  { id: "audit", label: "Audit log", icon: /* @__PURE__ */ React.createElement(I.audit, null), count: null },
  { kind: "section", label: "System" },
  { id: "mcp", label: "MCP servers", icon: /* @__PURE__ */ React.createElement(I.cpu, null), count: null },
  { id: "network", label: "Network", icon: /* @__PURE__ */ React.createElement(I.link, null), count: null },
  { id: "bindings", label: "Bindings", icon: /* @__PURE__ */ React.createElement(I.link, null), count: null },
  { id: "settings", label: "Settings", icon: /* @__PURE__ */ React.createElement(I.settings, null), count: null }
];
function Sidebar({ page, go }) {
  const [health] = usePolling("/api/health/detail", 2e4);
  const [onb] = usePolling("/api/onboarding", 3e4);
  const [approvalsResp] = usePolling("/api/approvals", 1e4);
  const approvalsCount = approvalsResp && Array.isArray(approvalsResp.approvals) ? approvalsResp.approvals.length : null;
  const uptime = health && health.uptime_seconds != null ? formatUptimeShort(health.uptime_seconds) : null;
  return /* @__PURE__ */ React.createElement("nav", { className: "sidebar" }, /* @__PURE__ */ React.createElement("div", { className: "sb-brand" }, /* @__PURE__ */ React.createElement("div", { className: "sb-mark" }, "RH"), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "sb-title" }, "Rusty Hand"), /* @__PURE__ */ React.createElement("div", { className: "sb-sub" }, "v0.7.51 \xB7 schema v8"))), /* @__PURE__ */ React.createElement("div", { className: "sb-nav", style: { flex: 1, overflow: "auto", padding: "6px 6px" } }, NAV.map((it, i) => {
    if (it.kind === "section") return /* @__PURE__ */ React.createElement("div", { key: i, className: "sb-section-label", style: { marginTop: i === 0 ? 4 : 10 } }, it.label);
    const active = page === it.id;
    const liveCount = it.id === "approvals" && approvalsCount != null ? approvalsCount : it.count;
    const liveBadge = it.id === "approvals" && approvalsCount > 0 ? "warn" : it.badge;
    return /* @__PURE__ */ React.createElement(
      "a",
      {
        key: it.id,
        href: `#/${it.id}`,
        className: "sb-item " + (active ? "active" : ""),
        onClick: (e) => {
          if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
          e.preventDefault();
          go(it.id);
        }
      },
      /* @__PURE__ */ React.createElement("span", { className: "sb-icon" }, it.icon),
      /* @__PURE__ */ React.createElement("span", null, it.label),
      liveCount != null && /* @__PURE__ */ React.createElement("span", { className: "sb-count", style: liveBadge === "warn" ? { color: "var(--amber)", borderColor: "oklch(0.78 0.14 88 / .35)" } : {} }, liveCount)
    );
  })), /* @__PURE__ */ React.createElement("div", { className: "sb-status" }, /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "dot " + (health ? "live" : "warn") }), /* @__PURE__ */ React.createElement("span", null, health ? "kernel live" : "checking\u2026"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, uptime || "\u2014")), onb && onb.demo_mode && /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "dot demo" }), /* @__PURE__ */ React.createElement("span", null, "demo mode"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, onb.provider || "mock")), /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "badge live", style: { padding: "1px 5px" } }, "v"), /* @__PURE__ */ React.createElement("span", null, health && health.version ? health.version : "\u2014"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, health && health.agent_count != null ? `${health.agent_count} agents` : ""))));
}
const CRUMBS = {
  overview: ["RustyHand", "Overview"],
  agents: ["RustyHand", "Agents"],
  chat: ["RustyHand", "Chat"],
  workflows: ["RustyHand", "Workflows"],
  automation: ["RustyHand", "Automation"],
  channels: ["RustyHand", "Channels"],
  analytics: ["RustyHand", "Analytics"],
  knowledge: ["RustyHand", "Knowledge"],
  memory: ["RustyHand", "Memory"],
  skills: ["RustyHand", "Skills"],
  approvals: ["RustyHand", "Approvals"],
  audit: ["RustyHand", "Audit log"],
  mcp: ["RustyHand", "MCP servers"],
  network: ["RustyHand", "Network"],
  bindings: ["RustyHand", "Bindings"],
  settings: ["RustyHand", "Settings"]
};
function Topbar({ page, onOpenPalette, onOpenHelp }) {
  const crumbs = CRUMBS[page] || ["RustyHand"];
  return /* @__PURE__ */ React.createElement("div", { className: "topbar" }, /* @__PURE__ */ React.createElement("div", { className: "crumbs" }, crumbs.map((c, i) => /* @__PURE__ */ React.createElement("span", { key: i }, i > 0 && /* @__PURE__ */ React.createElement("span", { className: "crumb-sep" }, " / "), /* @__PURE__ */ React.createElement("span", { className: i === crumbs.length - 1 ? "crumb-now" : "" }, c)))), /* @__PURE__ */ React.createElement("button", { className: "cmd", onClick: onOpenPalette }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("span", null, "Jump to agent, page\u2026"), /* @__PURE__ */ React.createElement("span", { className: "kbd kbd-row" }, /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u2318"), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "K"))), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", title: "Keyboard shortcuts (?)", onClick: onOpenHelp }, /* @__PURE__ */ React.createElement("span", { style: { fontFamily: "var(--ff-mono)", fontSize: 12, fontWeight: 600 } }, "?")), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", title: "Operator" }, /* @__PURE__ */ React.createElement("div", { className: "avatar", style: { width: 24, height: 24, fontSize: 10, background: "linear-gradient(135deg,oklch(0.6 0.13 22),oklch(0.42 0.1 35))" } }, "OP")));
}
const __PALETTE_RECENT_KEY = "rh.panel.paletteRecent";
function loadRecentPicks() {
  try {
    const raw = localStorage.getItem(__PALETTE_RECENT_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch (e) {
    return [];
  }
}
function pushRecentPick(row) {
  try {
    const cur = loadRecentPicks();
    const key = `${row.kind}:${row.id}`;
    const filtered = cur.filter((r) => `${r.kind}:${r.id}` !== key);
    const next = [{ kind: row.kind, id: row.id, label: row.label, sub: row.sub }, ...filtered].slice(0, 12);
    localStorage.setItem(__PALETTE_RECENT_KEY, JSON.stringify(next));
  } catch (e) {
  }
}
function CommandPalette({ open, onClose, go, openAgent }) {
  const [q, setQ] = useState("");
  const [highlight, setHighlight] = useState(0);
  const [agentsResp] = useApi(open ? "/api/agents?limit=200" : null);
  const [wfResp] = useApi(open ? "/api/workflows" : null);
  const [sessionsResp] = useApi(open ? "/api/sessions?limit=200" : null);
  const [auditResp] = useApi(open ? "/api/audit/recent?n=200" : null);
  const inputRef = useRef(null);
  useEffect(() => {
    if (open) {
      setQ("");
      setHighlight(0);
      setTimeout(() => inputRef.current && inputRef.current.focus(), 30);
    }
  }, [open]);
  const allRows = React.useMemo(() => {
    if (!open) return [];
    const ql = q.toLowerCase().trim();
    const agents = agentsResp && agentsResp.agents || [];
    const workflows = Array.isArray(wfResp) ? wfResp : wfResp && wfResp.workflows || [];
    const sessions = sessionsResp && sessionsResp.sessions || [];
    const audit = auditResp && auditResp.entries || [];
    const matches = (s) => !ql || (s || "").toLowerCase().includes(ql);
    const pageRows = NAV.filter((n) => !n.kind && matches(n.label)).map((n) => ({ kind: "page", id: n.id, label: n.label, sub: "page" }));
    const agentRows = agents.filter((a) => matches(a.name) || matches(a.model_name) || matches(a.id)).slice(0, 12).map((a) => ({ kind: "agent", id: a.id, label: a.name, sub: `${a.model_name || ""} \xB7 ${String(a.id).slice(0, 8)}` }));
    const wfRows = workflows.filter((w) => matches(w.name) || matches(w.description) || matches(w.id)).slice(0, 10).map((w) => ({ kind: "workflow", id: w.id, label: w.name || w.id, sub: w.description ? String(w.description).slice(0, 60) : "workflow" }));
    const sessionRows = sessions.filter((s) => matches(s.label) || matches(s.agent_name) || matches(s.session_id)).slice(0, 10).map((s) => ({ kind: "session", id: s.session_id, label: s.label || String(s.session_id).slice(0, 8), sub: `${s.agent_name || ""} \xB7 ${s.message_count || 0} msg` }));
    const auditRows = ql && ql.length >= 3 ? audit.filter((e) => (e.hash || "").toLowerCase().startsWith(ql) || matches(e.action) || matches(e.agent_name)).slice(0, 10).map((e) => ({ kind: "audit", id: e.hash || String(e.seq), label: e.action || "(action)", sub: `${e.agent_name || "kernel"} \xB7 ${String(e.hash || "").slice(0, 8)}` })) : [];
    if (!ql) {
      const recent = loadRecentPicks().filter((r) => r.kind !== "audit");
      return recent.concat(pageRows.filter((p) => !recent.some((r) => r.kind === "page" && r.id === p.id)));
    }
    return pageRows.concat(agentRows, wfRows, sessionRows, auditRows);
  }, [open, q, agentsResp, wfResp, sessionsResp, auditResp]);
  React.useEffect(() => {
    setHighlight(0);
  }, [q]);
  const pick = (row) => {
    if (!row) return;
    pushRecentPick(row);
    if (row.kind === "page") go(row.id);
    else if (row.kind === "agent") {
      const a = (agentsResp && agentsResp.agents || []).find((x) => x.id === row.id);
      if (a) openAgent(normalizeAgent(a));
    } else if (row.kind === "workflow") go("workflows");
    else if (row.kind === "session") go("memory");
    else if (row.kind === "audit") go("audit");
    onClose();
  };
  if (!open) return null;
  const rows = allRows;
  const onKeyDown = (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      pick(rows[highlight] || rows[0]);
    } else if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(rows.length - 1, h + 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(0, h - 1));
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal palette", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "palette-input" }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement(
    "input",
    {
      ref: inputRef,
      placeholder: "Search pages, agents, workflows, sessions, audit hash\u2026",
      value: q,
      onChange: (e) => setQ(e.target.value),
      onKeyDown
    }
  ), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "esc")), /* @__PURE__ */ React.createElement("div", { className: "palette-body" }, rows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "12px", fontSize: 11.5 } }, "No matches."), !q && rows.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { padding: "6px 12px", fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase" } }, loadRecentPicks().length > 0 ? "Recent + pages" : "Pages"), rows.map((row, i) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: `${row.kind}-${row.id}`,
      className: "palette-row" + (i === highlight ? " active" : ""),
      onMouseEnter: () => setHighlight(i),
      onClick: () => pick(row)
    },
    /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 9.5, letterSpacing: ".15em", textTransform: "uppercase", color: "var(--fg-4)", width: 60 } }, row.kind),
    /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12.5 } }, row.label),
    /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11, marginLeft: "auto", maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, row.sub)
  )))));
}
function AuthGate({ children }) {
  const [authState, setAuthState] = useState(null);
  const probe = React.useCallback(async () => {
    try {
      const me = await rhFetch("/api/auth/me");
      setAuthState(me || true);
    } catch (e) {
      if (e.status === 401 || e.status === 403) setAuthState(false);
      else {
        setAuthState(true);
      }
    }
  }, []);
  useEffect(() => {
    probe();
  }, [probe]);
  const onLogin = async (key) => {
    setApiKey(key);
    setAuthState(null);
    await probe();
  };
  if (authState === null) {
    return /* @__PURE__ */ React.createElement("div", { className: "auth-splash" }, /* @__PURE__ */ React.createElement("div", { className: "mono dim" }, "Connecting to kernel\u2026"));
  }
  if (authState === false) {
    return /* @__PURE__ */ React.createElement(LoginScreen, { onLogin });
  }
  return children;
}
function LoginScreen({ onLogin }) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const submit = async (e) => {
    if (e) e.preventDefault();
    if (!key.trim()) {
      setErr("API key required");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      setApiKey(key);
      const me = await rhFetch("/api/auth/me");
      if (!me || !me.authenticated) throw new Error("not authenticated");
      onLogin(key);
    } catch (e2) {
      setApiKey("");
      setErr(`Invalid key (${e2.status || e2.message || e2})`);
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "auth-splash" }, /* @__PURE__ */ React.createElement("form", { className: "auth-card", onSubmit: submit }, /* @__PURE__ */ React.createElement("div", { className: "auth-mark" }, "RH"), /* @__PURE__ */ React.createElement("div", { className: "auth-title" }, "RustyHand \xB7 Control Panel"), /* @__PURE__ */ React.createElement("div", { className: "auth-sub" }, "Enter your API key to continue. Configure it in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " under ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "api_key"), " or via per-user RBAC."), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "password", autoFocus: true, placeholder: "rh_\u2026", value: key, onChange: (e) => setKey(e.target.value) }), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), /* @__PURE__ */ React.createElement("button", { type: "submit", className: "btn primary", disabled: busy || !key.trim(), style: { width: "100%" } }, busy ? "Verifying\u2026" : "Sign in"), /* @__PURE__ */ React.createElement("div", { className: "dim mono auth-foot" }, "Running on localhost? Auth is auto-granted \u2014 this screen shouldn't appear.")));
}
const __ONBOARDED_KEY = "rh.panel.onboarded";
function shouldShowOnboarding(onb, agentsResp) {
  if (!onb || !agentsResp) return false;
  try {
    if (localStorage.getItem(__ONBOARDED_KEY)) return false;
  } catch (e) {
  }
  if (!onb.demo_mode) return false;
  const total = (agentsResp && agentsResp.total) != null ? agentsResp.total : (agentsResp.agents || []).length;
  return total <= 1;
}
function OnboardingWizard({ onClose }) {
  const [step, setStep] = useState(0);
  const [providersResp] = useApi("/api/providers");
  const providers = providersResp && providersResp.providers || [];
  const defaultProvider = providers.find((p) => p.key_required !== false && (p.auth_status || "").toLowerCase() !== "ok");
  const [providerName, setProviderName] = useState(defaultProvider ? defaultProvider.id : "anthropic");
  React.useEffect(() => {
    if (!providerName && providers.length) setProviderName(providers[0].id);
  }, [providers.length]);
  const [apiKey, setApiKey2] = useState("");
  const [savingKey, setSavingKey] = useState(false);
  const [keyErr, setKeyErr] = useState(null);
  const [keySaved, setKeySaved] = useState(false);
  const [agentName, setAgentName] = useState("my-agent");
  const [spawning, setSpawning] = useState(false);
  const [spawnErr, setSpawnErr] = useState(null);
  const dismiss = () => {
    try {
      localStorage.setItem(__ONBOARDED_KEY, "1");
    } catch (e) {
    }
    onClose();
  };
  const saveKey = async () => {
    if (!apiKey.trim()) {
      setKeyErr("Key required");
      return;
    }
    setSavingKey(true);
    setKeyErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(providerName)}/key`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: apiKey })
      });
      setKeySaved(true);
      toastOk(`${providerName} key saved`);
      setStep(2);
    } catch (e) {
      setKeyErr(String(e.message || e));
    } finally {
      setSavingKey(false);
    }
  };
  const skipKey = () => {
    setStep(2);
  };
  const spawn = async () => {
    if (!agentName.trim()) {
      setSpawnErr("Name required");
      return;
    }
    setSpawning(true);
    setSpawnErr(null);
    try {
      const provider = providers.find((p) => p.id === providerName) || providers[0] || { id: "anthropic" };
      const defaultModel = providerName === "anthropic" ? "claude-sonnet-4" : providerName === "openai" ? "gpt-4o-mini" : providerName === "deepseek" ? "deepseek-chat" : "claude-sonnet-4";
      const manifest_toml = `name = "${agentName.trim()}"
version = "0.1.0"
description = "Spawned from RustyHand onboarding wizard"
author = "operator"
module = "builtin:chat"

[model]
provider = "${provider.id}"
model = "${defaultModel}"
system_prompt = "You are a helpful agent."
temperature = 0.4
max_tokens = 2048

[capabilities]
tools = ["research"]
`;
      await rhFetch("/api/agents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ manifest_toml })
      });
      toastOk(`Spawned ${agentName.trim()}`);
      dismiss();
    } catch (e) {
      setSpawnErr(String(e.message || e));
    } finally {
      setSpawning(false);
    }
  };
  const skipSpawn = () => dismiss();
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal wide onboarding", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("div", { className: "auth-mark" }, "RH"), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono", style: { fontSize: 14 } }, "Welcome to RustyHand"), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, marginTop: 2 } }, "Step ", step + 1, " of 3"))), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: dismiss, title: "Skip and don't show again" }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, step === 0 && /* @__PURE__ */ React.createElement("div", { className: "col gap-12" }, /* @__PURE__ */ React.createElement("div", { style: { fontSize: 13, lineHeight: 1.55 } }, "RustyHand is a self-hostable agent operating system. This panel runs against the kernel listening at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, window.location.host), ". In the next two steps we'll set up a real LLM provider and spawn your first agent \u2014 about 30 seconds."), /* @__PURE__ */ React.createElement("div", { className: "card", style: { padding: 12 } }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "What you get"), /* @__PURE__ */ React.createElement("ul", { className: "md-ul", style: { margin: 0, paddingLeft: 18 } }, /* @__PURE__ */ React.createElement("li", null, "Multi-agent orchestration with a single kernel \u2014 workflows, triggers, cron jobs, channels (Telegram/Discord/Slack)."), /* @__PURE__ */ React.createElement("li", null, "Persistent memory with knowledge graph + vector embeddings."), /* @__PURE__ */ React.createElement("li", null, "16-layer security: capability gates, WASM sandbox, Merkle-chained audit, Ed25519-signed manifests."), /* @__PURE__ */ React.createElement("li", null, "26 supported LLM providers, auto-detected at boot."))), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12 } }, "You're currently in ", /* @__PURE__ */ React.createElement("b", { style: { color: "var(--rust)" } }, "Demo mode"), " \u2014 the mock driver echoes input back so the UI is interactive without any LLM cost. Add a real API key to use Claude / GPT / etc.")), step === 1 && /* @__PURE__ */ React.createElement("div", { className: "col gap-12" }, /* @__PURE__ */ React.createElement("div", { style: { fontSize: 13 } }, "Pick an LLM provider and paste the API key. The key is stored encrypted in", /* @__PURE__ */ React.createElement("span", { className: "mono" }, " ~/.rustyhand/config.toml"), " and zeroized after each use."), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Provider"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: providerName, onChange: (e) => setProviderName(e.target.value) }, providers.filter((p) => p.key_required !== false).map((p) => /* @__PURE__ */ React.createElement("option", { key: p.id, value: p.id }, p.display_name || p.id, (p.auth_status || "").toLowerCase() === "ok" ? "  \u2014 configured" : "")))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "API key"), /* @__PURE__ */ React.createElement(
    "input",
    {
      className: "modal-field",
      type: "password",
      placeholder: "sk-\u2026",
      value: apiKey,
      onChange: (e) => setApiKey2(e.target.value),
      onKeyDown: (e) => {
        if (e.key === "Enter") saveKey();
      },
      autoFocus: true
    }
  )), keyErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, keyErr)), keySaved && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.74 0.135 150 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "SAVED"), /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11.5 } }, providerName, " key stored. Continuing\u2026")), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5 } }, "Don't have a key? ", /* @__PURE__ */ React.createElement("a", { href: "https://console.anthropic.com/settings/keys", target: "_blank", rel: "noreferrer" }, "Get one from Anthropic"), " \xB7 skip this step to stay in demo mode.")), step === 2 && /* @__PURE__ */ React.createElement("div", { className: "col gap-12" }, /* @__PURE__ */ React.createElement("div", { style: { fontSize: 13 } }, "Spawn an agent that uses the provider you just configured. You can rename it, change the system prompt, swap models \u2014 everything is editable in the agent's drawer after spawn."), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Agent name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: agentName, onChange: (e) => setAgentName(e.target.value), autoFocus: true })), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 160 } }, `name = "${agentName || "my-agent"}"
provider = "${providerName}"
tools = ["research"]   # web_search, web_fetch, etc.
temperature = 0.4
max_tokens = 2048`), spawnErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, spawnErr)))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: dismiss, style: { marginRight: "auto" } }, "Skip setup"), step > 0 && /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setStep(step - 1) }, "Back"), step === 0 && /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setStep(1) }, "Get started"), step === 1 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "btn", onClick: skipKey }, "Stay in demo mode"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: saveKey, disabled: savingKey || !apiKey.trim() }, savingKey ? "Saving\u2026" : "Save key")), step === 2 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "btn", onClick: skipSpawn }, "Skip"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: spawn, disabled: spawning || !agentName.trim() }, spawning ? "Spawning\u2026" : "Spawn agent")))));
}
function HelpOverlay({ open, onClose }) {
  useEscapeKey(open ? onClose : null);
  if (!open) return null;
  const groups = [
    {
      label: "Navigation",
      rows: [
        ["\u2318 K  /  Ctrl K", "Open command palette"],
        ["1 \u2014 9", "Jump to the Nth sidebar entry"],
        ["g a", "Sidebar links \u2014 middle-click for new tab"],
        ["Esc", "Close any overlay (palette, drawer, modal)"]
      ]
    },
    {
      label: "On any page",
      rows: [
        ["/", "Focus the page's search field"],
        ["n", "Open the primary \u201CNew \u2026\u201D modal (Spawn, New job, etc.)"],
        ["r", "Refresh the current page"],
        ["?", "Toggle this help overlay"]
      ]
    },
    {
      label: "Inside the chat",
      rows: [
        ["Enter", "Send the message"],
        ["Click \u2699 trace", "Expand to see tool input + result"]
      ]
    },
    {
      label: "Workflows page",
      rows: [
        ["Click a run row", "Inspect step-by-step output + tokens"],
        ["Drag \u2630 on a step", "Reorder steps in the visual builder"]
      ]
    },
    {
      label: "Agents page",
      rows: [
        ["Checkbox in row", "Add to bulk selection"],
        ["Bulk bar", "Kill / Restart all selected agents"],
        ["Group / Flat", "Toggle grouping by team"]
      ]
    }
  ];
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Keyboard shortcuts"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "grid-12", style: { rowGap: 16 } }, groups.map((g) => /* @__PURE__ */ React.createElement("div", { key: g.label, className: "col-6" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, g.label), /* @__PURE__ */ React.createElement("div", { className: "col gap-4" }, g.rows.map(([k, desc]) => /* @__PURE__ */ React.createElement("div", { key: k, className: "row", style: { padding: "4px 0", gap: 12 } }, /* @__PURE__ */ React.createElement("span", { className: "kbd-row", style: { minWidth: 130 } }, k.split(/\s+\/\s+|\s+/).map(
    (part, i, arr) => /^[A-Za-z]$/.test(part) || part === "Esc" || part === "Enter" || part === "\u2318" || part === "Ctrl" || part === "K" || /^\d+$/.test(part) || part === "\u2014" || part === "/" || part === "?" || part === "n" || part === "r" ? /* @__PURE__ */ React.createElement("span", { key: i, className: "kbd" }, part) : /* @__PURE__ */ React.createElement("span", { key: i, style: { margin: "0 4px" } }, part)
  )), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 12 } }, desc)))))))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Close"))));
}
class PageErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { err: null, key: props.pageId };
  }
  static getDerivedStateFromProps(props, state) {
    return props.pageId !== state.key ? { err: null, key: props.pageId } : null;
  }
  static getDerivedStateFromError(err) {
    return { err };
  }
  componentDidCatch(err, info) {
    console.error(`[panel] page "${this.props.pageId}" crashed`, err, info);
  }
  render() {
    if (this.state.err) {
      const e = this.state.err;
      return /* @__PURE__ */ React.createElement("div", { style: { padding: "32px 8px" } }, /* @__PURE__ */ React.createElement("div", { className: "card", style: { maxWidth: 640, margin: "0 auto" } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mb-12" }, /* @__PURE__ */ React.createElement("div", { className: "auth-mark" }, "!"), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14 } }, "This page crashed"), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12 } }, "The rest of the panel is still working. Try a different page or reset this one."))), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 200, fontSize: 11 } }, String(e && (e.stack || e.message || e))), /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mt-12" }, /* @__PURE__ */ React.createElement("button", { className: "btn", onClick: () => this.setState({ err: null }) }, "Reset page"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => window.location.reload() }, "Reload"))));
    }
    return this.props.children;
  }
}
function App() {
  const route = useHashRoute();
  const page = route.page;
  const setPage = React.useCallback((p) => route.navigate(p, {}), [route]);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [t, setTweak] = useTweaks(TWEAKS_DEFAULTS);
  const [onbResp] = useApi("/api/onboarding");
  const [agentsListResp] = useApi("/api/agents?limit=2");
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  React.useEffect(() => {
    if (shouldShowOnboarding(onbResp, agentsListResp)) setOnboardingOpen(true);
  }, [onbResp, agentsListResp]);
  const drawerId = page === "agents" && route.params.id ? route.params.id : null;
  const [drawerResp] = useApi(drawerId ? `/api/agents/${drawerId}` : null);
  const drawerAgent = drawerId && drawerResp ? drawerResp.id ? normalizeAgent(drawerResp) : null : null;
  const openAgent = React.useCallback((agent) => {
    if (agent && agent.id) route.navigate("agents", { id: agent.id });
  }, [route]);
  const closeDrawer = React.useCallback(() => route.navigate("agents", {}), [route]);
  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-theme", t.theme);
    root.setAttribute("data-density", t.density === "normal" ? "" : t.density);
    const a = ACCENTS[t.accent] || ACCENTS.rust;
    root.style.setProperty("--rust", a.rust);
    root.style.setProperty("--rust-2", a.rust2);
    root.style.setProperty("--rust-dim", a.dim);
  }, [t.theme, t.accent, t.density]);
  const [helpOpen, setHelpOpen] = useState(false);
  useEffect(() => {
    const onKey = (e) => {
      var _a, _b;
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (e.key === "Escape") {
        setPaletteOpen(false);
        setHelpOpen(false);
        return;
      }
      if ((_b = (_a = e.target).matches) == null ? void 0 : _b.call(_a, "input, textarea, select, [contenteditable]")) return;
      if (e.key === "?") {
        e.preventDefault();
        setHelpOpen((v) => !v);
        return;
      }
      if (e.key === "/") {
        const el = document.querySelector(".content .search-field input");
        if (el) {
          e.preventDefault();
          el.focus();
        }
        return;
      }
      if (e.key === "n" || e.key === "N") {
        const ev = new CustomEvent("rh:hotkey:new", { detail: { page } });
        window.dispatchEvent(ev);
        return;
      }
      if (e.key === "r" || e.key === "R") {
        const ev = new CustomEvent("rh:hotkey:refresh", { detail: { page } });
        window.dispatchEvent(ev);
        return;
      }
      const items = NAV.filter((n) => !n.kind);
      const idx = parseInt(e.key, 10);
      if (idx >= 1 && idx <= items.length) {
        setPage(items[idx - 1].id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPage, page]);
  let pageEl;
  if (page === "overview") pageEl = /* @__PURE__ */ React.createElement(OverviewPage, { go: setPage });
  else if (page === "agents") pageEl = /* @__PURE__ */ React.createElement(AgentsPage, { openAgent });
  else if (page === "chat") pageEl = /* @__PURE__ */ React.createElement(ChatPage, null);
  else if (page === "workflows") pageEl = /* @__PURE__ */ React.createElement(WorkflowsPage, null);
  else if (page === "automation") pageEl = /* @__PURE__ */ React.createElement(AutomationPage, null);
  else if (page === "channels") pageEl = /* @__PURE__ */ React.createElement(ChannelsPage, null);
  else if (page === "analytics") pageEl = /* @__PURE__ */ React.createElement(AnalyticsPage, null);
  else if (page === "knowledge") pageEl = /* @__PURE__ */ React.createElement(KnowledgePage, null);
  else if (page === "memory") pageEl = /* @__PURE__ */ React.createElement(MemoryPage, null);
  else if (page === "mcp") pageEl = /* @__PURE__ */ React.createElement(McpPage, null);
  else if (page === "network") pageEl = /* @__PURE__ */ React.createElement(NetworkPage, null);
  else if (page === "bindings") pageEl = /* @__PURE__ */ React.createElement(BindingsPage, null);
  else if (page === "skills") pageEl = /* @__PURE__ */ React.createElement(SkillsPage, null);
  else if (page === "approvals") pageEl = /* @__PURE__ */ React.createElement(ApprovalsPage, null);
  else if (page === "audit") pageEl = /* @__PURE__ */ React.createElement(AuditPage, null);
  else if (page === "settings") pageEl = /* @__PURE__ */ React.createElement(SettingsPage, null);
  else pageEl = /* @__PURE__ */ React.createElement(OverviewPage, { go: setPage });
  return /* @__PURE__ */ React.createElement("div", { className: "app", "data-screen-label": `Page \xB7 ${page}` }, /* @__PURE__ */ React.createElement(Sidebar, { page, go: setPage }), /* @__PURE__ */ React.createElement("div", { className: "main" }, /* @__PURE__ */ React.createElement(Topbar, { page, onOpenPalette: () => setPaletteOpen(true), onOpenHelp: () => setHelpOpen(true) }), /* @__PURE__ */ React.createElement("div", { className: "content", style: { position: "relative" } }, /* @__PURE__ */ React.createElement(PageErrorBoundary, { pageId: page }, pageEl), /* @__PURE__ */ React.createElement(AgentDrawer, { agent: drawerAgent, onClose: closeDrawer }))), /* @__PURE__ */ React.createElement(CommandPalette, { open: paletteOpen, onClose: () => setPaletteOpen(false), go: setPage, openAgent }), /* @__PURE__ */ React.createElement(HelpOverlay, { open: helpOpen, onClose: () => setHelpOpen(false) }), onboardingOpen && /* @__PURE__ */ React.createElement(OnboardingWizard, { onClose: () => setOnboardingOpen(false) }), /* @__PURE__ */ React.createElement(TweaksPanel, null, /* @__PURE__ */ React.createElement(TweakSection, { label: "Theme" }), /* @__PURE__ */ React.createElement(TweakRadio, { label: "Mode", value: t.theme, options: ["dark", "light"], onChange: (v) => setTweak("theme", v) }), /* @__PURE__ */ React.createElement(TweakSection, { label: "Accent" }), /* @__PURE__ */ React.createElement(
    TweakSelect,
    {
      label: "Color",
      value: t.accent,
      options: ["rust", "copper", "amber", "forest", "electric"],
      onChange: (v) => setTweak("accent", v)
    }
  ), /* @__PURE__ */ React.createElement(TweakSection, { label: "Density" }), /* @__PURE__ */ React.createElement(
    TweakRadio,
    {
      label: "Rows",
      value: t.density,
      options: ["compact", "normal", "comfy"],
      onChange: (v) => setTweak("density", v)
    }
  ), /* @__PURE__ */ React.createElement(TweakSection, { label: "UI" }), /* @__PURE__ */ React.createElement(TweakToggle, { label: "Demo banner", value: t.showDemoBanner, onChange: (v) => setTweak("showDemoBanner", v) })));
}
ReactDOM.createRoot(document.getElementById("root")).render(
  /* @__PURE__ */ React.createElement(ErrorBoundary, null, /* @__PURE__ */ React.createElement(AuthGate, null, /* @__PURE__ */ React.createElement(App, null)), /* @__PURE__ */ React.createElement(ToastHost, null), /* @__PURE__ */ React.createElement(ConfirmHost, null))
);
