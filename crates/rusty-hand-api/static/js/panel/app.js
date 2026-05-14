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
  return /* @__PURE__ */ React.createElement("nav", { className: "sidebar" }, /* @__PURE__ */ React.createElement("div", { className: "sb-brand" }, /* @__PURE__ */ React.createElement("div", { className: "sb-mark" }, "RH"), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "sb-title" }, "Rusty Hand"), /* @__PURE__ */ React.createElement("div", { className: "sb-sub" }, "v0.7.46 \xB7 schema v8"))), /* @__PURE__ */ React.createElement("div", { className: "sb-nav", style: { flex: 1, overflow: "auto", padding: "6px 6px" } }, NAV.map((it, i) => {
    if (it.kind === "section") return /* @__PURE__ */ React.createElement("div", { key: i, className: "sb-section-label", style: { marginTop: i === 0 ? 4 : 10 } }, it.label);
    const active = page === it.id;
    return /* @__PURE__ */ React.createElement("div", { key: it.id, className: "sb-item " + (active ? "active" : ""), onClick: () => go(it.id) }, /* @__PURE__ */ React.createElement("span", { className: "sb-icon" }, it.icon), /* @__PURE__ */ React.createElement("span", null, it.label), it.count != null && /* @__PURE__ */ React.createElement("span", { className: "sb-count", style: it.badge === "warn" ? { color: "var(--amber)", borderColor: "oklch(0.78 0.14 88 / .35)" } : {} }, it.count));
  })), /* @__PURE__ */ React.createElement("div", { className: "sb-status" }, /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", null, "kernel live"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, "3d 4h")), /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "dot demo" }), /* @__PURE__ */ React.createElement("span", null, "demo mode"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, "mock")), /* @__PURE__ */ React.createElement("div", { className: "sb-status-row" }, /* @__PURE__ */ React.createElement("span", { className: "badge live", style: { padding: "1px 5px" } }, "WS"), /* @__PURE__ */ React.createElement("span", null, "127.0.0.1:4200"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, "42ms"))));
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
function Topbar({ page, onOpenPalette }) {
  const crumbs = CRUMBS[page] || ["RustyHand"];
  return /* @__PURE__ */ React.createElement("div", { className: "topbar" }, /* @__PURE__ */ React.createElement("div", { className: "crumbs" }, crumbs.map((c, i) => /* @__PURE__ */ React.createElement("span", { key: i }, i > 0 && /* @__PURE__ */ React.createElement("span", { className: "crumb-sep" }, " / "), /* @__PURE__ */ React.createElement("span", { className: i === crumbs.length - 1 ? "crumb-now" : "" }, c)))), /* @__PURE__ */ React.createElement("button", { className: "cmd", onClick: onOpenPalette }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("span", null, "Jump to agent, page\u2026"), /* @__PURE__ */ React.createElement("span", { className: "kbd kbd-row" }, /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u2318"), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "K"))), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", title: "Notifications" }, /* @__PURE__ */ React.createElement(I.zap, null)), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", title: "Operator" }, /* @__PURE__ */ React.createElement("div", { className: "avatar", style: { width: 24, height: 24, fontSize: 10, background: "linear-gradient(135deg,oklch(0.6 0.13 22),oklch(0.42 0.1 35))" } }, "OP")));
}
function CommandPalette({ open, onClose, go, openAgent }) {
  const [q, setQ] = useState("");
  const [agentsResp] = useApi(open ? "/api/agents?limit=200" : null);
  const inputRef = useRef(null);
  useEffect(() => {
    if (open) {
      setQ("");
      setTimeout(() => inputRef.current && inputRef.current.focus(), 30);
    }
  }, [open]);
  if (!open) return null;
  const agents = agentsResp && agentsResp.agents || [];
  const ql = q.toLowerCase();
  const pageRows = NAV.filter((n) => !n.kind && (!q || n.label.toLowerCase().includes(ql))).map((n) => ({ kind: "page", id: n.id, label: n.label, sub: "page" }));
  const agentRows = agents.filter((a) => !q || a.name.toLowerCase().includes(ql) || (a.model_name || "").toLowerCase().includes(ql) || (a.id || "").toLowerCase().includes(ql)).slice(0, 20).map((a) => ({ kind: "agent", id: a.id, label: a.name, sub: `${a.model_name || ""} \xB7 ${String(a.id).slice(0, 8)}` }));
  const rows = pageRows.concat(agentRows);
  const pick = (row) => {
    if (row.kind === "page") {
      go(row.id);
      onClose();
      return;
    }
    if (row.kind === "agent") {
      const a = agents.find((x) => x.id === row.id);
      if (a) openAgent(normalizeAgent(a));
      onClose();
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal palette", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "palette-input" }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement(
    "input",
    {
      ref: inputRef,
      placeholder: "Type to filter pages and agents\u2026",
      value: q,
      onChange: (e) => setQ(e.target.value),
      onKeyDown: (e) => {
        if (e.key === "Enter" && rows[0]) pick(rows[0]);
        if (e.key === "Escape") onClose();
      }
    }
  ), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "esc")), /* @__PURE__ */ React.createElement("div", { className: "palette-body" }, rows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "12px", fontSize: 11.5 } }, "No matches."), rows.map((row, i) => /* @__PURE__ */ React.createElement("button", { key: `${row.kind}-${row.id}`, className: "palette-row", onClick: () => pick(row) }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 9.5, letterSpacing: ".15em", textTransform: "uppercase", color: "var(--fg-4)", width: 48 } }, row.kind), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12.5 } }, row.label), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11, marginLeft: "auto" } }, row.sub))))));
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
function App() {
  const [page, setPage] = useState("overview");
  const [drawerAgent, setDrawerAgent] = useState(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [t, setTweak] = useTweaks(TWEAKS_DEFAULTS);
  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-theme", t.theme);
    root.setAttribute("data-density", t.density === "normal" ? "" : t.density);
    const a = ACCENTS[t.accent] || ACCENTS.rust;
    root.style.setProperty("--rust", a.rust);
    root.style.setProperty("--rust-2", a.rust2);
    root.style.setProperty("--rust-dim", a.dim);
  }, [t.theme, t.accent, t.density]);
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
        return;
      }
      if ((_b = (_a = e.target).matches) == null ? void 0 : _b.call(_a, "input, textarea")) return;
      const items = NAV.filter((n) => !n.kind);
      const idx = parseInt(e.key, 10);
      if (idx >= 1 && idx <= items.length) {
        setPage(items[idx - 1].id);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
  let pageEl;
  if (page === "overview") pageEl = /* @__PURE__ */ React.createElement(OverviewPage, { go: setPage });
  else if (page === "agents") pageEl = /* @__PURE__ */ React.createElement(AgentsPage, { openAgent: setDrawerAgent });
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
  return /* @__PURE__ */ React.createElement("div", { className: "app", "data-screen-label": `Page \xB7 ${page}` }, /* @__PURE__ */ React.createElement(Sidebar, { page, go: setPage }), /* @__PURE__ */ React.createElement("div", { className: "main" }, /* @__PURE__ */ React.createElement(Topbar, { page, onOpenPalette: () => setPaletteOpen(true) }), /* @__PURE__ */ React.createElement("div", { className: "content", style: { position: "relative" } }, pageEl, /* @__PURE__ */ React.createElement(AgentDrawer, { agent: drawerAgent, onClose: () => setDrawerAgent(null) }))), /* @__PURE__ */ React.createElement(CommandPalette, { open: paletteOpen, onClose: () => setPaletteOpen(false), go: setPage, openAgent: setDrawerAgent }), /* @__PURE__ */ React.createElement(TweaksPanel, null, /* @__PURE__ */ React.createElement(TweakSection, { label: "Theme" }), /* @__PURE__ */ React.createElement(TweakRadio, { label: "Mode", value: t.theme, options: ["dark", "light"], onChange: (v) => setTweak("theme", v) }), /* @__PURE__ */ React.createElement(TweakSection, { label: "Accent" }), /* @__PURE__ */ React.createElement(
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
  /* @__PURE__ */ React.createElement(ErrorBoundary, null, /* @__PURE__ */ React.createElement(AuthGate, null, /* @__PURE__ */ React.createElement(App, null)), /* @__PURE__ */ React.createElement(ToastHost, null))
);
