// RustyHand control panel — app shell + router + sidebar + topbar + tweaks.

const { useState, useEffect, useRef, useMemo } = React;

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
  { kind: "section", label: "System" },
  { id: "health",     label: "Health",     icon: <I.shield/>,     count: null },
  { id: "mcp",        label: "MCP servers",icon: <I.cpu/>,        count: null },
  { id: "network",    label: "Network",    icon: <I.link/>,       count: null },
  { id: "bindings",   label: "Bindings",   icon: <I.link/>,       count: null },
  { id: "settings",   label: "Settings",   icon: <I.settings/>,   count: null },
];

// localStorage-backed list of pinned sidebar item IDs (manual ordering).
// Right-click a sidebar item to toggle pinned/unpinned; pinned items
// render in a "Pinned" section at the top of the nav.
function usePinnedNav() {
  const KEY = "rh-panel-pinned-nav";
  const [pinned, setPinned] = React.useState(() => {
    try {
      const raw = localStorage.getItem(KEY);
      return Array.isArray(JSON.parse(raw)) ? JSON.parse(raw) : [];
    } catch (_) { return []; }
  });
  const toggle = (id) => {
    setPinned(prev => {
      const next = prev.includes(id) ? prev.filter(x => x !== id) : [...prev, id];
      try { localStorage.setItem(KEY, JSON.stringify(next)); } catch (_) {}
      return next;
    });
  };
  return [pinned, toggle];
}

function Sidebar({ page, go }) {
  // Live sidebar status: kernel uptime/version + pending approvals count.
  // Approvals subscribe to SSE for instant badge update + toast on new
  // requests; health/onboarding remain on a slow poll.
  const [health] = usePolling("/api/health/detail", 20000);
  const [onb] = usePolling("/api/onboarding", 30000);
  const [pinned, togglePin] = usePinnedNav();
  const [approvalsCount, setApprovalsCount] = React.useState(null);
  const lastIdsRef = React.useRef(null);
  useEventSource("/api/approvals/stream", (msg) => {
    if (!msg || !Array.isArray(msg.pending)) return;
    const ids = msg.pending.map(p => p.id).sort();
    if (lastIdsRef.current !== null) {
      const prev = new Set(lastIdsRef.current);
      const fresh = ids.filter(id => !prev.has(id));
      if (fresh.length > 0 && page !== "approvals") {
        const first = msg.pending.find(p => p.id === fresh[0]);
        const label = first ? (first.action_summary || first.tool_name || "Approval pending") : "Approval pending";
        toastWarn(`${fresh.length === 1 ? "" : `${fresh.length}× `}${label}`, { ttl: 6000 });
      }
    }
    lastIdsRef.current = ids;
    setApprovalsCount(ids.length);
  });
  const uptime = (health && health.uptime_seconds != null) ? formatUptimeShort(health.uptime_seconds) : null;
  return (
    <nav className="sidebar">
      <div className="sb-brand">
        <div className="sb-mark">RH</div>
        <div>
          <div className="sb-title">Rusty Hand</div>
          <div className="sb-sub">v0.7.68 · schema v8</div>
        </div>
      </div>

      <div className="sb-nav" style={{flex:1, overflow:"auto", padding:"6px 6px"}}>
        {pinned.length > 0 && (
          <>
            <div className="sb-section-label" style={{marginTop:4}}>Pinned</div>
            {pinned
              .map(id => NAV.find(n => n.id === id))
              .filter(Boolean)
              .map(it => {
                const active = page === it.id;
                const liveCount = it.id === "approvals" && approvalsCount != null ? approvalsCount : it.count;
                const liveBadge = it.id === "approvals" && approvalsCount > 0 ? "warn" : it.badge;
                return (
                  <a key={`pin-${it.id}`} href={`#/${it.id}`}
                     className={"sb-item " + (active ? "active" : "")}
                     onContextMenu={(e) => { e.preventDefault(); togglePin(it.id); }}
                     title="Right-click to unpin"
                     onClick={(e) => {
                       if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
                       e.preventDefault();
                       go(it.id);
                     }}>
                    <span className="sb-icon">{it.icon}</span>
                    <span>{it.label}</span>
                    {liveCount != null && <span className="sb-count" style={liveBadge === "warn" ? {color:"var(--amber)", borderColor:"oklch(0.78 0.14 88 / .35)"} : {}}>{liveCount}</span>}
                  </a>
                );
              })}
          </>
        )}
        {NAV.map((it, i) => {
          if (it.kind === "section") return <div key={i} className="sb-section-label" style={{marginTop: i === 0 && pinned.length === 0 ? 4 : 10}}>{it.label}</div>;
          const active = page === it.id;
          // Live override of the static approvals count from NAV.
          const liveCount = it.id === "approvals" && approvalsCount != null ? approvalsCount : it.count;
          const liveBadge = it.id === "approvals" && approvalsCount > 0 ? "warn" : it.badge;
          const isPinned = pinned.includes(it.id);
          // Render as <a> so middle-click + Cmd-click + "open in new tab"
          // work like a real link. The hashchange listener picks up the
          // navigation; we still preventDefault + call go(...) on plain
          // left-click for instant route-update without scroll-to-top.
          return (
            <a key={it.id} href={`#/${it.id}`}
               className={"sb-item " + (active ? "active" : "")}
               onContextMenu={(e) => { e.preventDefault(); togglePin(it.id); }}
               title={isPinned ? "Right-click to unpin" : "Right-click to pin to top"}
               onClick={(e) => {
                 if (e.metaKey || e.ctrlKey || e.shiftKey || e.button !== 0) return;
                 e.preventDefault();
                 go(it.id);
               }}>
              <span className="sb-icon">{it.icon}</span>
              <span>{it.label}</span>
              {isPinned && <span className="dim mono" style={{fontSize:10, marginLeft:"auto", opacity:.5}}>★</span>}
              {liveCount != null && <span className="sb-count" style={liveBadge === "warn" ? {color:"var(--amber)", borderColor:"oklch(0.78 0.14 88 / .35)"} : {}}>{liveCount}</span>}
            </a>
          );
        })}
      </div>

      <div className="sb-status">
        <div className="sb-status-row">
          <span className={"dot " + (health ? "live" : "warn")}/>
          <span>{health ? "kernel live" : "checking…"}</span>
          <span style={{marginLeft:"auto"}}>{uptime || "—"}</span>
        </div>
        {onb && onb.demo_mode && (
          <div className="sb-status-row">
            <span className="dot demo"/>
            <span>demo mode</span>
            <span style={{marginLeft:"auto"}}>{onb.provider || "mock"}</span>
          </div>
        )}
        <div className="sb-status-row">
          <span className="badge live" style={{padding:"1px 5px"}}>v</span>
          <span>{(health && health.version) ? health.version : "—"}</span>
          <span style={{marginLeft:"auto"}}>{health && health.agent_count != null ? `${health.agent_count} agents` : ""}</span>
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
  health:     ["RustyHand", "Health"],
  mcp:        ["RustyHand", "MCP servers"],
  network:    ["RustyHand", "Network"],
  bindings:   ["RustyHand", "Bindings"],
  settings:   ["RustyHand", "Settings"],
};

function Topbar({ page, onOpenPalette, onOpenHelp }) {
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
      <button className="icon-btn" title="Keyboard shortcuts (?)" onClick={onOpenHelp}>
        <span style={{fontFamily:"var(--ff-mono)", fontSize:12, fontWeight:600}}>?</span>
      </button>
      <button className="icon-btn" title="Operator"><div className="avatar" style={{width:24,height:24,fontSize:10,background:"linear-gradient(135deg,oklch(0.6 0.13 22),oklch(0.42 0.1 35))"}}>OP</div></button>
    </div>
  );
}

// Cmd+K command palette: lists pages + every loaded agent. Filters by
// substring on label / id / model. Returning agent picks open the drawer
// (handled by App via onPick).
// Command palette (⌘K). Searches across pages, agents, workflows,
// sessions, and audit-hash prefixes. Recent picks are remembered in
// localStorage and bubble to the top when the query is empty, so the
// palette doubles as a "jump back to where you were" affordance.
const __PALETTE_RECENT_KEY = "rh.panel.paletteRecent";
function loadRecentPicks() {
  try {
    const raw = localStorage.getItem(__PALETTE_RECENT_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch (e) { return []; }
}
function pushRecentPick(row) {
  try {
    const cur = loadRecentPicks();
    const key = `${row.kind}:${row.id}`;
    const filtered = cur.filter((r) => `${r.kind}:${r.id}` !== key);
    const next = [{ kind: row.kind, id: row.id, label: row.label, sub: row.sub }, ...filtered].slice(0, 12);
    localStorage.setItem(__PALETTE_RECENT_KEY, JSON.stringify(next));
  } catch (e) {}
}

function CommandPalette({ open, onClose, go, openAgent }) {
  const [q, setQ] = useState("");
  const [highlight, setHighlight] = useState(0);
  // All datasets fetched lazily on first open; usePolling stops when path is null.
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

  // Build row list. When q is empty we show recents + pages; once the
  // user types, every dataset participates.
  const allRows = React.useMemo(() => {
    if (!open) return [];
    const ql = q.toLowerCase().trim();
    const agents = (agentsResp && agentsResp.agents) || [];
    const workflows = Array.isArray(wfResp) ? wfResp : (wfResp && wfResp.workflows) || [];
    const sessions = (sessionsResp && sessionsResp.sessions) || [];
    const audit = (auditResp && auditResp.entries) || [];

    const matches = (s) => !ql || (s || "").toLowerCase().includes(ql);

    // Quick-action rows: jump to a page and immediately open its create
    // modal via the page's rh:hotkey:new listener. They appear when the
    // query matches "new", "add", or the action's own keywords.
    const QUICK_ACTIONS = [
      { id: "new-agent",    label: "New agent",    page: "agents",     keys: "agent spawn create" },
      { id: "new-workflow", label: "New workflow", page: "workflows",  keys: "workflow pipeline" },
      { id: "new-cron",     label: "New cron job", page: "automation", keys: "cron schedule job" },
      { id: "new-trigger",  label: "New trigger",  page: "automation", keys: "trigger event hook" },
    ];
    const quickActionMatches = (qa) => !ql
      || qa.label.toLowerCase().includes(ql)
      || qa.keys.toLowerCase().includes(ql)
      || "new add create".includes(ql);
    const quickRows = QUICK_ACTIONS
      .filter(quickActionMatches)
      .map((qa) => ({ kind: "action", id: qa.id, label: qa.label, sub: `→ ${qa.page} · ⌘N`, page: qa.page }));

    const pageRows = NAV.filter((n) => !n.kind && matches(n.label))
      .map((n) => ({ kind: "page", id: n.id, label: n.label, sub: "page" }));
    const agentRows = agents
      .filter((a) => matches(a.name) || matches(a.model_name) || matches(a.id))
      .slice(0, 12)
      .map((a) => ({ kind: "agent", id: a.id, label: a.name, sub: `${a.model_name || ""} · ${String(a.id).slice(0, 8)}` }));
    const wfRows = workflows
      .filter((w) => matches(w.name) || matches(w.description) || matches(w.id))
      .slice(0, 10)
      .map((w) => ({ kind: "workflow", id: w.id, label: w.name || w.id, sub: w.description ? String(w.description).slice(0, 60) : "workflow" }));
    const sessionRows = sessions
      .filter((s) => matches(s.label) || matches(s.agent_name) || matches(s.session_id))
      .slice(0, 10)
      .map((s) => ({ kind: "session", id: s.session_id, label: s.label || String(s.session_id).slice(0, 8), sub: `${s.agent_name || ""} · ${s.message_count || 0} msg` }));
    const auditRows = ql && ql.length >= 3
      ? audit
          .filter((e) => (e.hash || "").toLowerCase().startsWith(ql) || matches(e.action) || matches(e.agent_name))
          .slice(0, 10)
          .map((e) => ({ kind: "audit", id: e.hash || String(e.seq), label: e.action || "(action)", sub: `${e.agent_name || "kernel"} · ${String(e.hash || "").slice(0, 8)}` }))
      : [];

    if (!ql) {
      // No query → recents (de-duped against pages), then quick actions,
      // then pages. Quick actions get prime real estate because creating
      // things is the most common reason to open the palette.
      const recent = loadRecentPicks().filter((r) => r.kind !== "audit"); // audit hashes drift
      return recent
        .concat(quickRows)
        .concat(pageRows.filter((p) => !recent.some((r) => r.kind === "page" && r.id === p.id)));
    }
    return quickRows.concat(pageRows, agentRows, wfRows, sessionRows, auditRows);
  }, [open, q, agentsResp, wfResp, sessionsResp, auditResp]);

  React.useEffect(() => { setHighlight(0); }, [q]);

  const pick = (row) => {
    if (!row) return;
    pushRecentPick(row);
    if (row.kind === "page") go(row.id);
    else if (row.kind === "action") {
      // Navigate then dispatch the same hot-key event that 'n' would fire,
      // so the destination page opens its create modal automatically.
      go(row.page);
      setTimeout(() => {
        window.dispatchEvent(new CustomEvent("rh:hotkey:new", { detail: { page: row.page } }));
      }, 60);
    }
    else if (row.kind === "agent") {
      const a = ((agentsResp && agentsResp.agents) || []).find((x) => x.id === row.id);
      if (a) openAgent(normalizeAgent(a));
    } else if (row.kind === "workflow") go("workflows");
    else if (row.kind === "session") go("memory");
    else if (row.kind === "audit") go("audit");
    onClose();
  };

  if (!open) return null;
  const rows = allRows;

  const onKeyDown = (e) => {
    if (e.key === "Enter") { e.preventDefault(); pick(rows[highlight] || rows[0]); }
    else if (e.key === "Escape") onClose();
    else if (e.key === "ArrowDown") { e.preventDefault(); setHighlight((h) => Math.min(rows.length - 1, h + 1)); }
    else if (e.key === "ArrowUp") { e.preventDefault(); setHighlight((h) => Math.max(0, h - 1)); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal palette" onClick={(e) => e.stopPropagation()}>
        <div className="palette-input">
          <I.search/>
          <input ref={inputRef}
                 placeholder="Search pages, agents, workflows, sessions, audit hash…"
                 value={q}
                 onChange={(e) => setQ(e.target.value)}
                 onKeyDown={onKeyDown}/>
          <span className="kbd">esc</span>
        </div>
        <div className="palette-body">
          {rows.length === 0 && <div className="muted mono" style={{padding:"12px", fontSize:11.5}}>No matches.</div>}
          {!q && rows.length > 0 && (
            <div className="dim mono" style={{padding:"6px 12px", fontSize:10, letterSpacing:".12em", textTransform:"uppercase"}}>
              {loadRecentPicks().length > 0 ? "Recent + pages" : "Pages"}
            </div>
          )}
          {rows.map((row, i) => (
            <button key={`${row.kind}-${row.id}`}
                    className={"palette-row" + (i === highlight ? " active" : "")}
                    onMouseEnter={() => setHighlight(i)}
                    onClick={() => pick(row)}>
              <span className="mono" style={{fontSize:9.5, letterSpacing:".15em", textTransform:"uppercase", color:"var(--fg-4)", width:60}}>{row.kind}</span>
              <span className="mono" style={{fontSize:12.5}}>{row.label}</span>
              <span className="dim mono" style={{fontSize:11, marginLeft:"auto", maxWidth:280, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{row.sub}</span>
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

// Onboarding wizard: shows once on a fresh install (demo_mode=true,
// agent_count<=1 — meaning the kernel only has the welcome seed). After
// dismissal we store a `rh.panel.onboarded` flag so it never reappears.
//
// Three steps: welcome → add provider key → spawn first agent. Steps 2-3
// reuse `ProviderKeyModal` and `SpawnAgentModal` semantics via small
// inline forms so the wizard fits the modal's narrative flow (one
// "Next" button per step instead of separate modals).
const __ONBOARDED_KEY = "rh.panel.onboarded";
function shouldShowOnboarding(onb, agentsResp) {
  // Defer until both probes resolved so we don't flash the wizard.
  if (!onb || !agentsResp) return false;
  // Already dismissed by the user.
  try { if (localStorage.getItem(__ONBOARDED_KEY)) return false; } catch (e) { /* private mode */ }
  // Fresh install signal: kernel reports demo mode (no real API key) AND
  // the agent registry is the seeded default (1 demo agent at most).
  if (!onb.demo_mode) return false;
  const total = (agentsResp && agentsResp.total) != null ? agentsResp.total : (agentsResp.agents || []).length;
  return total <= 1;
}

function OnboardingWizard({ onClose }) {
  const [step, setStep] = useState(0);
  const [providersResp] = useApi("/api/providers");
  const providers = (providersResp && providersResp.providers) || [];
  // Default to the first provider that requires a key but doesn't have
  // one — that's the most likely setup gap. Falls back to anthropic.
  const defaultProvider = providers.find(p => p.key_required !== false && (p.auth_status || "").toLowerCase() !== "ok");
  const [providerName, setProviderName] = useState(defaultProvider ? defaultProvider.id : "anthropic");
  React.useEffect(() => {
    if (!providerName && providers.length) setProviderName(providers[0].id);
  }, [providers.length]);

  const [apiKey, setApiKey] = useState("");
  const [savingKey, setSavingKey] = useState(false);
  const [keyErr, setKeyErr] = useState(null);
  const [keySaved, setKeySaved] = useState(false);

  const [agentName, setAgentName] = useState("my-agent");
  const [spawning, setSpawning] = useState(false);
  const [spawnErr, setSpawnErr] = useState(null);

  const dismiss = () => {
    try { localStorage.setItem(__ONBOARDED_KEY, "1"); } catch (e) {}
    onClose();
  };

  const saveKey = async () => {
    if (!apiKey.trim()) { setKeyErr("Key required"); return; }
    setSavingKey(true); setKeyErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(providerName)}/key`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key: apiKey }),
      });
      setKeySaved(true);
      toastOk(`${providerName} key saved`);
      setStep(2);
    } catch (e) { setKeyErr(String(e.message || e)); }
    finally { setSavingKey(false); }
  };
  const skipKey = () => { setStep(2); };

  const spawn = async () => {
    if (!agentName.trim()) { setSpawnErr("Name required"); return; }
    setSpawning(true); setSpawnErr(null);
    try {
      const provider = providers.find(p => p.id === providerName) || providers[0] || { id: "anthropic" };
      const defaultModel = providerName === "anthropic" ? "claude-sonnet-4"
        : providerName === "openai" ? "gpt-4o-mini"
        : providerName === "deepseek" ? "deepseek-chat"
        : "claude-sonnet-4";
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
        body: JSON.stringify({ manifest_toml }),
      });
      toastOk(`Spawned ${agentName.trim()}`);
      dismiss();
    } catch (e) { setSpawnErr(String(e.message || e)); }
    finally { setSpawning(false); }
  };
  const skipSpawn = () => dismiss();

  return (
    <div className="modal-back" onClick={(e) => e.stopPropagation()}>
      <div className="modal wide onboarding" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div className="row gap-12">
            <div className="auth-mark">RH</div>
            <div>
              <b className="mono" style={{fontSize:14}}>Welcome to RustyHand</b>
              <div className="dim mono" style={{fontSize:11, marginTop:2}}>Step {step + 1} of 3</div>
            </div>
          </div>
          <button className="icon-btn" onClick={dismiss} title="Skip and don't show again"><I.close/></button>
        </div>
        <div className="modal-body">
          {step === 0 && (
            <div className="col gap-12">
              <div style={{fontSize:13, lineHeight:1.55}}>
                RustyHand is a self-hostable agent operating system. This panel runs against the kernel
                listening at <span className="mono">{window.location.host}</span>. In the next two steps
                we'll set up a real LLM provider and spawn your first agent — about 30 seconds.
              </div>
              <div className="card" style={{padding:12}}>
                <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>What you get</div>
                <ul className="md-ul" style={{margin:0, paddingLeft:18}}>
                  <li>Multi-agent orchestration with a single kernel — workflows, triggers, cron jobs, channels (Telegram/Discord/Slack).</li>
                  <li>Persistent memory with knowledge graph + vector embeddings.</li>
                  <li>16-layer security: capability gates, WASM sandbox, Merkle-chained audit, Ed25519-signed manifests.</li>
                  <li>26 supported LLM providers, auto-detected at boot.</li>
                </ul>
              </div>
              <div className="dim" style={{fontSize:12}}>
                You're currently in <b style={{color:"var(--rust)"}}>Demo mode</b> — the mock driver echoes input back so the
                UI is interactive without any LLM cost. Add a real API key to use Claude / GPT / etc.
              </div>
            </div>
          )}
          {step === 1 && (
            <div className="col gap-12">
              <div style={{fontSize:13}}>
                Pick an LLM provider and paste the API key. The key is stored encrypted in
                <span className="mono"> ~/.rustyhand/config.toml</span> and zeroized after each use.
              </div>
              <label className="t-row col">
                <span className="t-lbl">Provider</span>
                <select className="t-select" value={providerName} onChange={(e) => setProviderName(e.target.value)}>
                  {providers.filter(p => p.key_required !== false).map(p => (
                    <option key={p.id} value={p.id}>
                      {p.display_name || p.id}
                      {(p.auth_status || "").toLowerCase() === "ok" ? "  — configured" : ""}
                    </option>
                  ))}
                </select>
              </label>
              <label className="t-row col">
                <span className="t-lbl">API key</span>
                <input className="modal-field" type="password" placeholder="sk-…"
                       value={apiKey} onChange={(e) => setApiKey(e.target.value)}
                       onKeyDown={(e) => { if (e.key === "Enter") saveKey(); }}
                       autoFocus/>
              </label>
              {keyErr && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
                <span className="dot err"/><span className="banner-title">ERROR</span>
                <span className="banner-body mono" style={{fontSize:11}}>{keyErr}</span>
              </div>}
              {keySaved && <div className="banner" style={{borderColor:"oklch(0.74 0.135 150 / .35)"}}>
                <span className="dot live"/><span className="banner-title">SAVED</span>
                <span className="banner-body" style={{fontSize:11.5}}>{providerName} key stored. Continuing…</span>
              </div>}
              <div className="dim" style={{fontSize:11.5}}>
                Don't have a key? <a href="https://console.anthropic.com/settings/keys" target="_blank" rel="noreferrer">Get one from Anthropic</a> · skip this step to stay in demo mode.
              </div>
            </div>
          )}
          {step === 2 && (
            <div className="col gap-12">
              <div style={{fontSize:13}}>
                Spawn an agent that uses the provider you just configured. You can rename it, change the
                system prompt, swap models — everything is editable in the agent's drawer after spawn.
              </div>
              <label className="t-row col">
                <span className="t-lbl">Agent name</span>
                <input className="modal-field" value={agentName} onChange={(e) => setAgentName(e.target.value)} autoFocus/>
              </label>
              <pre className="codebox" style={{maxHeight:160}}>
{`name = "${agentName || "my-agent"}"
provider = "${providerName}"
tools = ["research"]   # web_search, web_fetch, etc.
temperature = 0.4
max_tokens = 2048`}
              </pre>
              {spawnErr && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
                <span className="dot err"/><span className="banner-title">ERROR</span>
                <span className="banner-body mono" style={{fontSize:11}}>{spawnErr}</span>
              </div>}
            </div>
          )}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={dismiss} style={{marginRight:"auto"}}>Skip setup</button>
          {step > 0 && <button className="btn ghost" onClick={() => setStep(step - 1)}>Back</button>}
          {step === 0 && <button className="btn primary" onClick={() => setStep(1)}>Get started</button>}
          {step === 1 && (
            <>
              <button className="btn" onClick={skipKey}>Stay in demo mode</button>
              <button className="btn primary" onClick={saveKey} disabled={savingKey || !apiKey.trim()}>
                {savingKey ? "Saving…" : "Save key"}
              </button>
            </>
          )}
          {step === 2 && (
            <>
              <button className="btn" onClick={skipSpawn}>Skip</button>
              <button className="btn primary" onClick={spawn} disabled={spawning || !agentName.trim()}>
                {spawning ? "Spawning…" : "Spawn agent"}
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}

// HelpOverlay — keyboard shortcut cheat sheet. Opened by `?`. Pure
// data-driven so adding a shortcut means adding a row, not editing
// a template string.
function HelpOverlay({ open, onClose }) {
  useEscapeKey(open ? onClose : null);
  if (!open) return null;
  const groups = [
    {
      label: "Navigation",
      rows: [
        ["⌘ K  /  Ctrl K", "Open command palette"],
        ["1 — 9", "Jump to the Nth sidebar entry"],
        ["g a", "Sidebar links — middle-click for new tab"],
        ["Esc", "Close any overlay (palette, drawer, modal)"],
      ],
    },
    {
      label: "On any page",
      rows: [
        ["/", "Focus the page's search field"],
        ["n", "Open the primary “New …” modal (Spawn, New job, etc.)"],
        ["r", "Refresh the current page"],
        ["?", "Toggle this help overlay"],
      ],
    },
    {
      label: "Inside the chat",
      rows: [
        ["Enter", "Send the message"],
        ["Click ⚙ trace", "Expand to see tool input + result"],
      ],
    },
    {
      label: "Workflows page",
      rows: [
        ["Click a run row", "Inspect step-by-step output + tokens"],
        ["Drag ☰ on a step", "Reorder steps in the visual builder"],
      ],
    },
    {
      label: "Agents page",
      rows: [
        ["Checkbox in row", "Add to bulk selection"],
        ["Bulk bar", "Kill / Restart all selected agents"],
        ["Group / Flat", "Toggle grouping by team"],
      ],
    },
  ];
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Keyboard shortcuts</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="grid-12" style={{rowGap:16}}>
            {groups.map((g) => (
              <div key={g.label} className="col-6">
                <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>{g.label}</div>
                <div className="col gap-4">
                  {g.rows.map(([k, desc]) => (
                    <div key={k} className="row" style={{padding:"4px 0", gap:12}}>
                      <span className="kbd-row" style={{minWidth:130}}>
                        {k.split(/\s+\/\s+|\s+/).map((part, i, arr) =>
                          /^[A-Za-z]$/.test(part) || part === "Esc" || part === "Enter" || part === "⌘" || part === "Ctrl" || part === "K" || /^\d+$/.test(part) || part === "—" || part === "/" || part === "?" || part === "n" || part === "r"
                            ? <span key={i} className="kbd">{part}</span>
                            : <span key={i} style={{margin:"0 4px"}}>{part}</span>
                        )}
                      </span>
                      <span className="dim" style={{fontSize:12}}>{desc}</span>
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

// Per-page ErrorBoundary: a render error in one page no longer kicks the
// whole shell to the recovery screen — other pages stay reachable via the
// sidebar. We key on `pageId` so navigating to a fresh page resets state
// even if the previous page crashed.
class PageErrorBoundary extends React.Component {
  constructor(props) { super(props); this.state = { err: null, key: props.pageId }; }
  static getDerivedStateFromProps(props, state) {
    return props.pageId !== state.key ? { err: null, key: props.pageId } : null;
  }
  static getDerivedStateFromError(err) { return { err }; }
  componentDidCatch(err, info) {
    // eslint-disable-next-line no-console
    console.error(`[panel] page "${this.props.pageId}" crashed`, err, info);
  }
  render() {
    if (this.state.err) {
      const e = this.state.err;
      return (
        <div style={{padding:"32px 8px"}}>
          <div className="card" style={{maxWidth:640, margin:"0 auto"}}>
            <div className="row gap-12 mb-12">
              <div className="auth-mark">!</div>
              <div>
                <div className="mono" style={{fontSize:14}}>This page crashed</div>
                <div className="dim" style={{fontSize:12}}>The rest of the panel is still working. Try a different page or reset this one.</div>
              </div>
            </div>
            <pre className="codebox" style={{maxHeight:200, fontSize:11}}>{String(e && (e.stack || e.message || e))}</pre>
            <div className="row gap-8 mt-12">
              <button className="btn" onClick={() => this.setState({ err: null })}>Reset page</button>
              <button className="btn primary" onClick={() => window.location.reload()}>Reload</button>
            </div>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

function App() {
  // Hash-based router replaces local `page` state so URLs are
  // bookmarkable and the back button works. `route.params.id` selects
  // an agent for the drawer when present (#/agents/{uuid}).
  const route = useHashRoute();
  const page = route.page;
  const setPage = React.useCallback((p) => route.navigate(p, {}), [route]);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [t, setTweak] = useTweaks(TWEAKS_DEFAULTS);

  // Onboarding probe — small, low-frequency reads that decide whether
  // to overlay the welcome wizard. After dismissal the wizard never
  // re-renders even if `demo_mode` flips again.
  const [onbResp] = useApi("/api/onboarding");
  const [agentsListResp] = useApi("/api/agents?limit=2");
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  React.useEffect(() => {
    if (shouldShowOnboarding(onbResp, agentsListResp)) setOnboardingOpen(true);
  }, [onbResp, agentsListResp]);

  // The drawer is route-driven. When `#/agents/{uuid}` is in the URL,
  // we fetch that single agent and hand it to `<AgentDrawer/>`. On
  // close we navigate back to `#/agents`.
  const drawerId = (page === "agents" && route.params.id) ? route.params.id : null;
  const [drawerResp] = useApi(drawerId ? `/api/agents/${drawerId}` : null);
  const drawerAgent = drawerId && drawerResp
    ? (drawerResp.id ? normalizeAgent(drawerResp) : null)
    : null;
  const openAgent = React.useCallback((agent) => {
    if (agent && agent.id) route.navigate("agents", { id: agent.id });
  }, [route]);
  const closeDrawer = React.useCallback(() => route.navigate("agents", {}), [route]);

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

  const [helpOpen, setHelpOpen] = useState(false);

  // Keyboard shortcuts:
  //   ⌘K / Ctrl-K : palette
  //   1..9        : jump to NAV item
  //   /           : focus page search (Agents, Memory, Knowledge, etc.)
  //   n           : open primary "New X" modal on current page
  //   r           : refresh current page
  //   ?           : help overlay
  //   esc         : close any open overlay
  useEffect(() => {
    const onKey = (e) => {
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
      // Inside form controls we don't want hotkeys stealing keystrokes.
      if (e.target.matches?.("input, textarea, select, [contenteditable]")) return;
      if (e.key === "?") { e.preventDefault(); setHelpOpen((v) => !v); return; }
      if (e.key === "/") {
        // Focus the first .search-field input on the active page.
        const el = document.querySelector(".content .search-field input");
        if (el) { e.preventDefault(); el.focus(); }
        return;
      }
      if (e.key === "n" || e.key === "N") {
        // Dispatch a custom event that pages listen for to open their
        // primary "New X" modal. Decouples App from per-page state.
        const ev = new CustomEvent("rh:hotkey:new", { detail: { page } });
        window.dispatchEvent(ev);
        return;
      }
      if (e.key === "r" || e.key === "R") {
        const ev = new CustomEvent("rh:hotkey:refresh", { detail: { page } });
        window.dispatchEvent(ev);
        return;
      }
      const items = NAV.filter(n => !n.kind);
      const idx = parseInt(e.key, 10);
      if (idx >= 1 && idx <= items.length) { setPage(items[idx-1].id); }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [setPage, page]);

  let pageEl;
  if (page === "overview") pageEl = <OverviewPage go={setPage}/>;
  else if (page === "agents") pageEl = <AgentsPage openAgent={openAgent}/>;
  else if (page === "chat") pageEl = <ChatPage/>;
  else if (page === "workflows") pageEl = <WorkflowsPage/>;
  else if (page === "automation") pageEl = <AutomationPage/>;
  else if (page === "channels") pageEl = <ChannelsPage/>;
  else if (page === "analytics") pageEl = <AnalyticsPage/>;
  else if (page === "knowledge") pageEl = <KnowledgePage/>;
  else if (page === "memory") pageEl = <MemoryPage/>;
  else if (page === "health") pageEl = <HealthPage/>;
  else if (page === "mcp") pageEl = <McpPage/>;
  else if (page === "network") pageEl = <NetworkPage/>;
  else if (page === "bindings") pageEl = <BindingsPage/>;
  else if (page === "skills") pageEl = <SkillsPage/>;
  else if (page === "approvals") pageEl = <ApprovalsPage/>;
  else if (page === "audit") pageEl = <AuditPage/>;
  else if (page === "settings") pageEl = <SettingsPage/>;
  else pageEl = <OverviewPage go={setPage}/>;

  return (
    <div className="app" data-screen-label={`Page · ${page}`}>
      <Sidebar page={page} go={setPage}/>
      <div className="main">
        <Topbar page={page} onOpenPalette={() => setPaletteOpen(true)} onOpenHelp={() => setHelpOpen(true)}/>
        <div className="content" style={{position:"relative"}}>
          <PageErrorBoundary pageId={page}>{pageEl}</PageErrorBoundary>
          <AgentDrawer agent={drawerAgent} onClose={closeDrawer}/>
        </div>
      </div>

      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} go={setPage} openAgent={openAgent}/>
      <HelpOverlay open={helpOpen} onClose={() => setHelpOpen(false)}/>
      {onboardingOpen && <OnboardingWizard onClose={() => setOnboardingOpen(false)}/>}

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
  <ErrorBoundary>
    <AuthGate><App/></AuthGate>
    <ToastHost/>
    <ConfirmHost/>
  </ErrorBoundary>
);
