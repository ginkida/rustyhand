// RustyHand control panel — app shell + router + sidebar + topbar + tweaks.

const { useState, useEffect } = React;

const TWEAKS_DEFAULTS = /*EDITMODE-BEGIN*/{
  "theme": "dark",
  "accent": "rust",
  "density": "normal",
  "monoHeadings": true,
  "showDemoBanner": true
}/*EDITMODE-END*/;

const ACCENTS = {
  rust:    { rust: "oklch(0.665 0.165 50)",  rust2: "oklch(0.72 0.15 55)",  dim: "oklch(0.42 0.10 50)"  },
  copper:  { rust: "oklch(0.65 0.14 35)",    rust2: "oklch(0.71 0.13 38)",  dim: "oklch(0.40 0.09 35)"  },
  amber:   { rust: "oklch(0.78 0.155 85)",   rust2: "oklch(0.82 0.14 85)",  dim: "oklch(0.46 0.10 85)"  },
  forest:  { rust: "oklch(0.66 0.15 155)",   rust2: "oklch(0.72 0.14 155)", dim: "oklch(0.40 0.10 155)" },
  electric:{ rust: "oklch(0.70 0.20 290)",   rust2: "oklch(0.76 0.18 290)", dim: "oklch(0.42 0.13 290)" },
};

const NAV = [
  { kind: "section", label: "Work" },
  { id: "overview",   label: "Overview",   icon: <I.overview/>,   count: null },
  { id: "agents",     label: "Agents",     icon: <I.agents/>,     count: 10 },
  { id: "chat",       label: "Chat",       icon: <I.chat/>,       count: null },
  { id: "approvals",  label: "Approvals",  icon: <I.approvals/>,  count: 3, badge: "warn" },
  { kind: "section", label: "Build" },
  { id: "workflows",  label: "Workflows",  icon: <I.workflows/>,  count: 5 },
  { id: "automation", label: "Automation", icon: <I.automation/>, count: 9 },
  { id: "channels",   label: "Channels",   icon: <I.channels/>,   count: 4 },
  { id: "skills",     label: "Skills",     icon: <I.skills/>,     count: 60 },
  { kind: "section", label: "Inspect" },
  { id: "analytics",  label: "Analytics",  icon: <I.analytics/>,  count: null },
  { id: "knowledge",  label: "Knowledge",  icon: <I.knowledge/>,  count: null },
  { id: "memory",     label: "Memory",     icon: <I.audit/>,      count: null },
  { id: "audit",      label: "Audit log",  icon: <I.audit/>,      count: null },
  { id: "settings",   label: "Settings",   icon: <I.settings/>,   count: null },
];

function Sidebar({ page, go }) {
  return (
    <nav className="sidebar">
      <div className="sb-brand">
        <div className="sb-mark">RH</div>
        <div>
          <div className="sb-title">Rusty Hand</div>
          <div className="sb-sub">v0.7.44 · schema v8</div>
        </div>
      </div>

      <div className="sb-nav" style={{flex:1, overflow:"auto", padding:"6px 6px"}}>
        {NAV.map((it, i) => {
          if (it.kind === "section") return <div key={i} className="sb-section-label" style={{marginTop: i === 0 ? 4 : 10}}>{it.label}</div>;
          const active = page === it.id;
          return (
            <div key={it.id} className={"sb-item " + (active ? "active" : "")} onClick={() => go(it.id)}>
              <span className="sb-icon">{it.icon}</span>
              <span>{it.label}</span>
              {it.count != null && <span className="sb-count" style={it.badge === "warn" ? {color:"var(--amber)", borderColor:"oklch(0.78 0.14 88 / .35)"} : {}}>{it.count}</span>}
            </div>
          );
        })}
      </div>

      <div className="sb-status">
        <div className="sb-status-row">
          <span className="dot live"/>
          <span>kernel live</span>
          <span style={{marginLeft:"auto"}}>3d 4h</span>
        </div>
        <div className="sb-status-row">
          <span className="dot demo"/>
          <span>demo mode</span>
          <span style={{marginLeft:"auto"}}>mock</span>
        </div>
        <div className="sb-status-row">
          <span className="badge live" style={{padding:"1px 5px"}}>WS</span>
          <span>127.0.0.1:4200</span>
          <span style={{marginLeft:"auto"}}>42ms</span>
        </div>
      </div>
    </nav>
  );
}

const CRUMBS = {
  overview:   ["RustyHand", "Overview"],
  agents:     ["RustyHand", "Agents"],
  chat:       ["RustyHand", "Chat"],
  workflows:  ["RustyHand", "Workflows"],
  automation: ["RustyHand", "Automation"],
  channels:   ["RustyHand", "Channels"],
  analytics:  ["RustyHand", "Analytics"],
  knowledge:  ["RustyHand", "Knowledge"],
  memory:     ["RustyHand", "Memory"],
  skills:     ["RustyHand", "Skills"],
  approvals:  ["RustyHand", "Approvals"],
  audit:      ["RustyHand", "Audit log"],
  settings:   ["RustyHand", "Settings"],
};

function Topbar({ page, onOpenPalette }) {
  const crumbs = CRUMBS[page] || ["RustyHand"];
  return (
    <div className="topbar">
      <div className="crumbs">
        {crumbs.map((c, i) => (
          <span key={i}>
            {i > 0 && <span className="crumb-sep"> / </span>}
            <span className={i === crumbs.length - 1 ? "crumb-now" : ""}>{c}</span>
          </span>
        ))}
      </div>
      <button className="cmd" onClick={onOpenPalette}>
        <I.search/>
        <span>Jump to agent, page…</span>
        <span className="kbd kbd-row"><span className="kbd">⌘</span><span className="kbd">K</span></span>
      </button>
      <button className="icon-btn" title="Notifications"><I.zap/></button>
      <button className="icon-btn" title="Operator"><div className="avatar" style={{width:24,height:24,fontSize:10,background:"linear-gradient(135deg,oklch(0.6 0.13 22),oklch(0.42 0.1 35))"}}>OP</div></button>
    </div>
  );
}

// Cmd+K command palette: lists pages + every loaded agent. Filters by
// substring on label / id / model. Returning agent picks open the drawer
// (handled by App via onPick).
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
  const agents = (agentsResp && agentsResp.agents) || [];
  const ql = q.toLowerCase();
  const pageRows = NAV.filter(n => !n.kind && (!q || n.label.toLowerCase().includes(ql)))
    .map(n => ({ kind: "page", id: n.id, label: n.label, sub: "page" }));
  const agentRows = agents
    .filter(a => !q || a.name.toLowerCase().includes(ql) || (a.model_name || "").toLowerCase().includes(ql) || (a.id || "").toLowerCase().includes(ql))
    .slice(0, 20)
    .map(a => ({ kind: "agent", id: a.id, label: a.name, sub: `${a.model_name || ""} · ${String(a.id).slice(0, 8)}` }));
  const rows = pageRows.concat(agentRows);

  const pick = (row) => {
    if (row.kind === "page") { go(row.id); onClose(); return; }
    if (row.kind === "agent") {
      // Normalize to the shape the drawer expects.
      const a = agents.find(x => x.id === row.id);
      if (a) openAgent(normalizeAgent(a));
      onClose();
    }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal palette" onClick={e => e.stopPropagation()}>
        <div className="palette-input">
          <I.search/>
          <input ref={inputRef} placeholder="Type to filter pages and agents…"
                 value={q} onChange={e => setQ(e.target.value)}
                 onKeyDown={e => { if (e.key === "Enter" && rows[0]) pick(rows[0]); if (e.key === "Escape") onClose(); }}/>
          <span className="kbd">esc</span>
        </div>
        <div className="palette-body">
          {rows.length === 0 && <div className="muted mono" style={{padding:"12px", fontSize:11.5}}>No matches.</div>}
          {rows.map((row, i) => (
            <button key={`${row.kind}-${row.id}`} className="palette-row" onClick={() => pick(row)}>
              <span className="mono" style={{fontSize:9.5, letterSpacing:".15em", textTransform:"uppercase", color:"var(--fg-4)", width:48}}>{row.kind}</span>
              <span className="mono" style={{fontSize:12.5}}>{row.label}</span>
              <span className="dim mono" style={{fontSize:11, marginLeft:"auto"}}>{row.sub}</span>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}

// Authentication gate: render LoginScreen when /api/auth/me returns 401.
// Localhost callers always succeed (the auth middleware grants the `owner`
// role automatically), so this is a no-op for the common dev case. Remote
// callers see the form until they paste a working API key.
function AuthGate({ children }) {
  // null = checking, false = needs login, object = authenticated
  const [authState, setAuthState] = useState(null);

  const probe = React.useCallback(async () => {
    try {
      const me = await rhFetch("/api/auth/me");
      setAuthState(me || true);
    } catch (e) {
      if (e.status === 401 || e.status === 403) setAuthState(false);
      else {
        // Network/server error — keep the user in the app rather than
        // showing a login screen for a transient outage. The polling
        // hooks will surface the error per-card.
        setAuthState(true);
      }
    }
  }, []);

  useEffect(() => { probe(); }, [probe]);

  const onLogin = async (key) => {
    setApiKey(key);
    setAuthState(null);
    await probe();
  };

  if (authState === null) {
    return <div className="auth-splash"><div className="mono dim">Connecting to kernel…</div></div>;
  }
  if (authState === false) {
    return <LoginScreen onLogin={onLogin}/>;
  }
  return children;
}

function LoginScreen({ onLogin }) {
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const submit = async (e) => {
    if (e) e.preventDefault();
    if (!key.trim()) { setErr("API key required"); return; }
    setBusy(true); setErr(null);
    try {
      // Verify the key works before persisting + accepting.
      setApiKey(key);
      const me = await rhFetch("/api/auth/me");
      if (!me || !me.authenticated) throw new Error("not authenticated");
      onLogin(key);
    } catch (e) {
      setApiKey("");
      setErr(`Invalid key (${e.status || e.message || e})`);
    } finally { setBusy(false); }
  };
  return (
    <div className="auth-splash">
      <form className="auth-card" onSubmit={submit}>
        <div className="auth-mark">RH</div>
        <div className="auth-title">RustyHand · Control Panel</div>
        <div className="auth-sub">Enter your API key to continue. Configure it in <span className="mono">~/.rustyhand/config.toml</span> under <span className="mono">api_key</span> or via per-user RBAC.</div>
        <input className="modal-field" type="password" autoFocus placeholder="rh_…" value={key} onChange={e => setKey(e.target.value)}/>
        {err && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
          <span className="dot err"/><span className="banner-title">ERROR</span>
          <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
        </div>}
        <button type="submit" className="btn primary" disabled={busy || !key.trim()} style={{width:"100%"}}>
          {busy ? "Verifying…" : "Sign in"}
        </button>
        <div className="dim mono auth-foot">
          Running on localhost? Auth is auto-granted — this screen shouldn't appear.
        </div>
      </form>
    </div>
  );
}

function App() {
  const [page, setPage] = useState("overview");
  const [drawerAgent, setDrawerAgent] = useState(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [t, setTweak] = useTweaks(TWEAKS_DEFAULTS);

  // Apply tweaks to :root
  useEffect(() => {
    const root = document.documentElement;
    root.setAttribute("data-theme", t.theme);
    root.setAttribute("data-density", t.density === "normal" ? "" : t.density);
    const a = ACCENTS[t.accent] || ACCENTS.rust;
    root.style.setProperty("--rust", a.rust);
    root.style.setProperty("--rust-2", a.rust2);
    root.style.setProperty("--rust-dim", a.dim);
  }, [t.theme, t.accent, t.density]);

  // Keyboard nav: 1-9 for top items + ⌘K / Ctrl-K for palette.
  useEffect(() => {
    const onKey = (e) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if (e.key === "Escape") { setPaletteOpen(false); return; }
      if (e.target.matches?.("input, textarea")) return;
      const items = NAV.filter(n => !n.kind);
      const idx = parseInt(e.key, 10);
      if (idx >= 1 && idx <= items.length) { setPage(items[idx-1].id); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  let pageEl;
  if (page === "overview") pageEl = <OverviewPage go={setPage}/>;
  else if (page === "agents") pageEl = <AgentsPage openAgent={setDrawerAgent}/>;
  else if (page === "chat") pageEl = <ChatPage/>;
  else if (page === "workflows") pageEl = <WorkflowsPage/>;
  else if (page === "automation") pageEl = <AutomationPage/>;
  else if (page === "channels") pageEl = <ChannelsPage/>;
  else if (page === "analytics") pageEl = <AnalyticsPage/>;
  else if (page === "knowledge") pageEl = <KnowledgePage/>;
  else if (page === "memory") pageEl = <MemoryPage/>;
  else if (page === "skills") pageEl = <SkillsPage/>;
  else if (page === "approvals") pageEl = <ApprovalsPage/>;
  else if (page === "audit") pageEl = <AuditPage/>;
  else if (page === "settings") pageEl = <SettingsPage/>;
  else pageEl = <OverviewPage go={setPage}/>;

  return (
    <div className="app" data-screen-label={`Page · ${page}`}>
      <Sidebar page={page} go={setPage}/>
      <div className="main">
        <Topbar page={page} onOpenPalette={() => setPaletteOpen(true)}/>
        <div className="content" style={{position:"relative"}}>
          {pageEl}
          <AgentDrawer agent={drawerAgent} onClose={() => setDrawerAgent(null)}/>
        </div>
      </div>

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} go={setPage} openAgent={setDrawerAgent}/>

      <TweaksPanel>
        <TweakSection label="Theme"/>
        <TweakRadio label="Mode" value={t.theme} options={["dark","light"]} onChange={v => setTweak("theme", v)}/>
        <TweakSection label="Accent"/>
        <TweakSelect label="Color" value={t.accent}
          options={["rust","copper","amber","forest","electric"]}
          onChange={v => setTweak("accent", v)}/>
        <TweakSection label="Density"/>
        <TweakRadio label="Rows" value={t.density}
          options={["compact","normal","comfy"]}
          onChange={v => setTweak("density", v)}/>
        <TweakSection label="UI"/>
        <TweakToggle label="Demo banner" value={t.showDemoBanner} onChange={v => setTweak("showDemoBanner", v)}/>
      </TweaksPanel>
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")).render(
  <AuthGate><App/></AuthGate>
);
