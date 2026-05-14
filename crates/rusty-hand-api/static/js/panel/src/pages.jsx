// RustyHand control panel — pages.
// Overview, Agents, Chat, Workflows, Automation, Channels, Analytics,
// Knowledge, Skills, Approvals, Audit, Settings.

const { useState, useEffect, useMemo, useRef } = React;
const D = window.RH_DATA;

/* ============================== OVERVIEW ============================== */
//
// Overview pulls live data from the kernel HTTP API. Each panel widget has
// its own hook so partial failures (e.g. /api/usage/daily errors but
// /api/agents succeeds) still render most of the page. Mock fixtures from
// `window.RH_DATA` act as fallback only when the corresponding fetch hasn't
// resolved yet — once data arrives, the design's mock vocabulary stays
// (state badges, sparklines) because `normalizeAgent` maps API state names
// to the design's vocabulary.
function OverviewPage({ go }) {
  const [agentsResp, , refreshAgents] = usePolling("/api/agents?limit=100", 15000);
  const [health] = usePolling("/api/health/detail", 10000);
  const [audit, , refreshAudit] = usePolling("/api/audit/recent?n=12", 8000);
  const [approvalsResp, , refreshApprovals] = usePolling("/api/approvals", 15000);
  const [onboarding] = useApi("/api/onboarding");
  const [usage] = usePolling("/api/usage/daily", 60000);
  const [providersResp] = useApi("/api/providers");

  const agents = (agentsResp && agentsResp.agents) ? agentsResp.agents.map(normalizeAgent) : D.agents;
  const totalAgents = (agentsResp && agentsResp.total) ?? agents.length;
  const live = agents.filter(a => a.state === "running").length;
  const errors = agents.filter(a => a.state === "error").length;
  const days = (usage && usage.days) || [];
  const cost24 = usage ? (usage.today_cost_usd || 0) : D.costSeries.reduce((s, v) => s + v, 0);
  const ticks24 = (usage && usage.ticks_today) || 0;
  const refresh = () => { refreshAgents(); refreshAudit(); refreshApprovals(); };

  const approvalRows = (approvalsResp && approvalsResp.approvals) || D.approvals;
  const version = (health && health.version) || "0.7.61";
  const uptime = (health && health.uptime_seconds) ? formatUptime(health.uptime_seconds) : null;

  return (
    <div>
      <Banner go={go} onboarding={onboarding}/>
      <div className="page-head">
        <div>
          <h1 className="page-title">Overview</h1>
          <p className="page-sub">
            System pulse · kernel <span className="mono">v{version}</span>
            {uptime && <> · uptime <span className="mono">{uptime}</span></>}
            {" "}· schema <span className="mono">v8</span>
          </p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/> Refresh</button>
          <button className="btn primary"><I.plus/> New agent</button>
        </div>
      </div>

      <div className="tiles">
        <Tile label="Agents running"   value={`${live}`}     foot={`of ${totalAgents} total`}     spark={sparkOf(agents.map((_,i) => live + (i % 3 - 1)), 12)}/>
        <Tile label="Cost · today"     value={`$${(cost24 || 0).toFixed(2)}`} foot={usage ? "from /api/usage/daily" : "loading…"} spark={days.slice(-12).map(d => d.cost_usd || 0)}/>
        <Tile label="Audit entries"    value={(audit && audit.total != null) ? audit.total.toLocaleString() : "—"} foot={(audit && audit.tip_hash) ? `tip ${String(audit.tip_hash).slice(0,8)}` : "loading…"} spark={[1,1,1,1,1,1,1,1,1,1,1,1]}/>
        <Tile label="Errors · agents"  value={`${errors}`}    foot={errors ? `${errors} crashed agent(s)` : "all green"} spark={[0,0,1,0,0,0,0,0,errors,0,0,0]} deltaCls="up" warn={errors > 0}/>
      </div>

      <div className="grid-12">
        <div className="col-8 col">
          <div className="card flush">
            <div className="card-head">
              <span>Live activity</span>
              <div className="ch-actions">
                <button className="btn sm ghost" onClick={refreshAudit}><I.refresh/></button>
                <button className="btn sm ghost"><I.download/> Export</button>
              </div>
            </div>
            <ActivityFeed entries={audit && audit.entries}/>
          </div>

          <div className="card flush">
            <div className="card-head">
              <span>Approvals · waiting</span>
              <div className="ch-actions"><button className="btn sm ghost" onClick={() => go("approvals")}>Open queue</button></div>
            </div>
            {approvalRows.length === 0
              ? <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>No approvals waiting.</div>
              : <ApprovalsTable rows={approvalRows.slice(0, 3)} compact onChange={refreshApprovals}/>}
          </div>
        </div>

        <div className="col-4 col">
          {onboarding && onboarding.demo_mode && (
            <div className="card">
              <div className="row between mb-12">
                <span className="muted mono" style={{fontSize:11, letterSpacing:'.12em', textTransform:'uppercase'}}>Demo seed</span>
                <span className="badge demo"><span className="dot demo"/>active</span>
              </div>
              <div className="col gap-8">
                <SeedRow icon={<I.agents/>} title="rusty"             sub="welcome agent · chat-ready"        onClick={() => go("chat")}/>
                <SeedRow icon={<I.workflows/>} title="demo-pipeline"  sub="2-step sample workflow · click to run" onClick={() => go("workflows")}/>
                <SeedRow icon={<I.event/>} title="sample trigger"     sub="agent-spawn on webhook"             onClick={() => go("automation")}/>
                <SeedRow icon={<I.cron/>} title="demo-daily-ping"     sub="cron 0 9 * * * · disabled"           onClick={() => go("automation")}/>
              </div>
            </div>
          )}

          <div className="card">
            <div className="row between mb-12">
              <span className="muted mono" style={{fontSize:11, letterSpacing:'.12em', textTransform:'uppercase'}}>Providers</span>
              <button className="btn sm ghost" onClick={() => go("settings")}>Manage</button>
            </div>
            <ProvidersList providers={providersResp && providersResp.providers}/>
          </div>

          <div className="card">
            <div className="row between mb-12">
              <span className="muted mono" style={{fontSize:11, letterSpacing:'.12em', textTransform:'uppercase'}}>Audit chain</span>
              <button className="btn sm ghost" onClick={() => go("audit")}>View</button>
            </div>
            <div className="col gap-4">
              <AuditSummary audit={audit}/>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// Format seconds → "3d 4h" / "12h 7m" / "42s" — matches the design.
function formatUptime(s) {
  if (s == null) return null;
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}

// Sparkline data shaping: tiles want length-N arrays. If the API didn't
// give us a series, synthesize a flat line so the spark area still
// renders (the design loses visual weight without it).
function sparkOf(arr, n) {
  if (!arr || arr.length === 0) return Array(n).fill(0);
  if (arr.length >= n) return arr.slice(-n);
  return Array(n - arr.length).fill(arr[0]).concat(arr);
}

const Banner = ({ go, onboarding }) => {
  // Hide the banner unless the API confirms demo mode is active. During
  // initial load we render nothing (vs. showing a stale banner from the
  // mock fixture).
  if (!onboarding || !onboarding.demo_mode) return null;
  return (
    <div className="banner">
      <span className="dot demo"/>
      <span className="banner-title">DEMO MODE</span>
      <span className="banner-body">
        Provider <span className="mono">{onboarding.provider || "mock"}</span>
        {onboarding.api_key_set === false && <> — no API key set</>}
        {" "}· {onboarding.agent_count || 0} agent(s) loaded.
      </span>
      <span className="banner-cta">
        <button className="btn sm ghost" onClick={() => go("settings")}>Add API key</button>
      </span>
    </div>
  );
};

const ProvidersList = ({ providers }) => {
  if (!providers) {
    return (
      <div className="col gap-6">
        {["anthropic","openai","deepseek","ollama","mock"].map(n => (
          <ProviderRow key={n} name={n} state="idle" detail="loading…"/>
        ))}
      </div>
    );
  }
  // The /api/providers response items look like:
  // { id, display_name, api_key_env, auth_status, key_required, model_count }
  // We map auth_status → row state vocabulary.
  return (
    <div className="col gap-6">
      {providers.slice(0, 6).map(p => {
        const id = p.id || p.name || "—";
        const auth = (p.auth_status || "").toLowerCase();
        let state = "idle";
        if (auth === "ok" || auth === "configured" || auth === "set") state = "connected";
        else if (auth === "local") state = "local";
        else if (auth === "fallback" || id === "mock") state = "fallback";
        else if (auth === "missing" && p.key_required === false) state = "local";
        const models = p.model_count != null ? `${p.model_count} model${p.model_count === 1 ? "" : "s"}` : "";
        const env = p.api_key_env && p.api_key_env !== "—" ? p.api_key_env : "";
        const detail = [models, env].filter(Boolean).join(" · ") || "—";
        return (
          <ProviderRow
            key={id}
            name={p.display_name || id}
            state={state}
            detail={detail}
          />
        );
      })}
    </div>
  );
};

const AuditSummary = ({ audit }) => {
  if (!audit) {
    return (
      <div className="kv">
        <dt>head</dt><dd className="dim">…</dd>
        <dt>length</dt><dd className="dim">…</dd>
        <dt>verified</dt><dd className="dim">checking…</dd>
        <dt>file</dt><dd className="dim">~/.rustyhand/data/audit.jsonl</dd>
      </div>
    );
  }
  const head = audit.tip_hash ? String(audit.tip_hash).slice(0, 8) : "—";
  return (
    <div className="kv">
      <dt>head</dt><dd className="mono">{head}</dd>
      <dt>length</dt><dd>{(audit.total != null ? audit.total : (audit.entries || []).length).toLocaleString()}</dd>
      <dt>verified</dt><dd style={{color:"var(--live)"}}>✓ live</dd>
      <dt>file</dt><dd className="dim">~/.rustyhand/data/audit.jsonl</dd>
    </div>
  );
};

const Tile = ({ label, value, foot, spark, delta, deltaCls = "up", warn }) => (
  <div className="tile">
    <div className="tile-label">{label}{warn && <I.warn/>}</div>
    <div className="tile-value">{value}</div>
    <div className="tile-foot">
      {delta && <span className={`delta ${deltaCls}`}>{delta}</span>} <span className="dim">· {foot}</span>
    </div>
    <div className="tile-spark"><Spark data={spark} width={88} height={28} color="var(--rust)"/></div>
  </div>
);

const SeedRow = ({ icon, title, sub, onClick }) => (
  <button onClick={onClick} className="row gap-8" style={{
    padding: "8px 10px", borderRadius: 7, border: "1px solid var(--border)",
    background: "var(--bg-2)", textAlign: "left", width: "100%",
  }}>
    <span style={{color:"var(--rust)", display:"grid", placeItems:"center", width:22}}>{icon}</span>
    <span className="col" style={{gap:1}}>
      <span style={{fontFamily:"var(--ff-mono)", fontSize:12}}>{title}</span>
      <span className="dim" style={{fontSize:11}}>{sub}</span>
    </span>
    <span style={{marginLeft:"auto", color:"var(--fg-3)"}}><I.arrowR/></span>
  </button>
);

const ProviderRow = ({ name, state, detail }) => {
  const map = { connected: "live", local: "sky", fallback: "demo" };
  return (
    <div className="row between" style={{ padding: "6px 0", borderBottom: "1px solid var(--border)" }}>
      <div className="col" style={{ gap: 2 }}>
        <span className="mono" style={{ fontSize: 12 }}>{name}</span>
        <span className="dim" style={{ fontSize: 11 }}>{detail}</span>
      </div>
      <span className={`badge ${map[state] || "idle"}`}>{state}</span>
    </div>
  );
};

// Pick a hue class for an audit action. Spawn/boot are "live" (green),
// approvals are "amber", trigger fires are "violet", everything else is
// "muted" (subdued). Stay close to the design's visual rhythm — a flat
// table of grey loses the at-a-glance read.
function actionColor(action) {
  const a = (action || "").toLowerCase();
  if (a.includes("spawn") || a.includes("boot") || a.includes("closed")) return "live";
  if (a.includes("trigger") || a.includes("workflow")) return "violet";
  if (a.includes("approval") || a.includes("denied") || a.includes("reject")) return "amber";
  if (a.includes("error") || a.includes("panic") || a.includes("crash")) return "muted";
  return "muted";
}

// Classify an audit entry into one of `error|warn|info` for the Audit
// page's filter chips. Mirrors `classify_audit_level` on the kernel side
// (used by /api/logs/stream's `level` query) so the chip choice on the
// client matches what the kernel would say if asked.
function auditLevelOf(entry) {
  const a = ((entry && (entry.action || "")) + " " + (entry && entry.outcome || "")).toLowerCase();
  if (a.includes("error") || a.includes("panic") || a.includes("crash") || a.includes("fail")) return "error";
  if (a.includes("approval") || a.includes("denied") || a.includes("reject") || a.includes("warn")) return "warn";
  return "info";
}

function formatTime(ts) {
  if (!ts) return "—";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return String(ts).slice(11, 19);
  return d.toLocaleTimeString("en-GB", { hour12: false });
}

const ActivityFeed = ({ entries }) => {
  const cls = c => ({ live: "var(--live)", violet: "var(--violet)", amber: "var(--amber)", muted: "var(--fg-3)" }[c]);
  // Live overlay: subscribe to /api/logs/stream and prepend new entries
  // to the polled `entries` prop. The polled snapshot still drives the
  // initial paint (so the feed isn't empty on first load while the SSE
  // backfill streams in). After that, SSE additions are the source of
  // truth — we de-dupe by hash || seq.
  const [live, setLive] = useState([]);
  const stream = useEventSource("/api/logs/stream", React.useCallback((msg) => {
    if (!msg || typeof msg !== "object") return;
    setLive((prev) => {
      const key = msg.hash || msg.seq;
      // Skip duplicates: the SSE backfill includes the latest 200
      // entries on first poll, which usually overlaps the polled list.
      if (prev.some(p => (p.hash || p.seq) === key)) return prev;
      // Cap at 200 to keep the DOM bounded; the user can navigate to
      // /audit for full history.
      return [msg, ...prev].slice(0, 200);
    });
  }, []));

  // Merge polled + live, de-duped, sorted descending by seq.
  const polled = entries || [];
  const merged = React.useMemo(() => {
    const seen = new Set();
    const out = [];
    for (const e of live.concat(polled)) {
      const k = e.hash || e.seq;
      if (k != null && seen.has(k)) continue;
      if (k != null) seen.add(k);
      out.push(e);
    }
    return out.sort((a, b) => (b.seq || 0) - (a.seq || 0)).slice(0, 200);
  }, [live, polled]);

  if (!entries && live.length === 0) {
    return (
      <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>
        Loading audit chain…
      </div>
    );
  }
  if (merged.length === 0) {
    return (
      <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>
        No audit entries yet — they appear as agents act.
      </div>
    );
  }
  return (
    <div style={{ maxHeight: 360, overflow: "auto" }}>
      <div className="row" style={{padding:"4px 14px", borderBottom:"1px solid var(--border)", gap:8, background:"var(--bg-2)"}}>
        <span className={"dot " + (stream.connected ? "live" : "warn")}/>
        <span className="dim mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
          {stream.connected ? "live · sse" : "stale · sse disconnected"}
        </span>
        <span className="dim mono" style={{fontSize:10.5, marginLeft:"auto"}}>{merged.length} events</span>
      </div>
      {merged.map((it, i) => {
        const color = actionColor(it.action);
        const detail = it.detail || it.outcome || "";
        return (
          <div key={it.hash || it.seq || i} className="row"
               style={{ padding: "9px 14px", borderBottom: "1px solid var(--border)", gap: 12 }}>
            <span className="mono dim" style={{ fontSize: 11, width: 70 }}>{formatTime(it.timestamp)}</span>
            <span className="mono" style={{ fontSize: 12, width: 140, color: cls(color), overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }}>
              {it.agent_name || it.agent_id || "kernel"}
            </span>
            <span className="mono" style={{ fontSize: 12, width: 170, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }}>{it.action}</span>
            <span className="mono dim" style={{ fontSize: 11.5, flex: 1, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap" }}>{detail}</span>
          </div>
        );
      })}
    </div>
  );
};

// rows from /api/approvals come in this shape: { id, agent_id, agent_name,
// action, risk, requested_at, status }. The design's mock used agent (string),
// action, risk, age — we adapt below so this component accepts both.
// `selectable` adds a checkbox column wired into the parent's selection
// Set; only the Approvals page uses it (the Overview embeds the table in
// read-mostly mode).
const ApprovalsTable = ({ rows, compact, onChange, onInspect, selectable, selected, onToggle, onToggleAll }) => {
  const decide = async (id, verdict) => {
    try {
      await rhFetch(`/api/approvals/${id}/${verdict}`, { method: "POST" });
      toastOk(`Approval ${verdict}d`);
      if (onChange) onChange();
    } catch (e) {
      toastErr(`${verdict} failed: ${e.message || e}`);
    }
  };
  const allChecked = selectable && rows.length > 0 && rows.every(r => selected && selected.has(r.id));
  return (
    <table className="tbl">
      <thead>
        <tr>
          {selectable && (
            <th style={{width:28}}>
              <input type="checkbox"
                     checked={allChecked}
                     ref={el => { if (el) el.indeterminate = !!(selected && selected.size > 0 && !allChecked); }}
                     onChange={() => onToggleAll && onToggleAll(rows)}
                     title={allChecked ? "Deselect all" : "Select all"}/>
            </th>
          )}
          <th>ID</th><th>Agent</th><th>Action</th><th>Risk</th><th>Age</th><th className="right">Decide</th>
        </tr>
      </thead>
      <tbody>
        {rows.map(r => {
          const agent = r.agent_name || r.agent || r.agent_id || "—";
          const age = r.age || relativeTime(r.requested_at || r.created_at);
          const risk = (r.risk || "low").toLowerCase();
          const isSel = selectable && selected && selected.has(r.id);
          return (
            <tr key={r.id}
                style={{cursor: onInspect ? "pointer" : "default", background: isSel ? "var(--surface-2)" : undefined}}
                onClick={() => onInspect && onInspect(r)}
                title={onInspect ? "Click to inspect full payload" : ""}>
              {selectable && (
                <td onClick={(e) => { e.stopPropagation(); onToggle && onToggle(r.id); }}>
                  <input type="checkbox" checked={!!isSel} readOnly tabIndex={-1}/>
                </td>
              )}
              <td className="mono">{r.id}</td>
              <td className="mono">{agent}</td>
              <td>{r.action}</td>
              <td><span className={`badge ${risk === "high" ? "error" : risk === "medium" ? "warn" : "idle"}`}>{risk}</span></td>
              <td className="mono muted">{age}</td>
              <td className="right" onClick={(e) => e.stopPropagation()}>
                <button className="btn sm primary" style={{height:24,padding:"2px 8px"}} onClick={() => decide(r.id, "approve")}>Approve</button>
                <button className="btn sm danger" style={{height:24,padding:"2px 8px",marginLeft:6}} onClick={() => decide(r.id, "reject")}>Reject</button>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
};

// Tiny pagination chip-row used by Agents / Memory / Audit. Driven by the
// `usePagination` hook from api.jsx.
function Pagination({ pg }) {
  if (!pg || pg.total === 0) return null;
  return (
    <div className="row gap-8" style={{padding:"10px 14px", borderTop:"1px solid var(--border)", justifyContent:"flex-end"}}>
      <span className="dim mono" style={{fontSize:11}}>
        {pg.offset + 1}–{Math.min(pg.offset + pg.pageSize, pg.total)} of {pg.total.toLocaleString()}
      </span>
      <button className="btn sm ghost" onClick={pg.prev} disabled={!pg.hasPrev}>← Prev</button>
      <span className="mono" style={{fontSize:11.5, minWidth:60, textAlign:"center"}}>{pg.page} / {pg.totalPages}</span>
      <button className="btn sm ghost" onClick={pg.next} disabled={!pg.hasNext}>Next →</button>
    </div>
  );
}

/* ============================== AGENTS ============================== */
function AgentsPage({ openAgent }) {
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const [showSpawn, setShowSpawn] = useState(false);
  const [rowMenu, setRowMenu] = useState(null);
  const [selected, setSelected] = useState(() => new Set());
  const [showDiff, setShowDiff] = useState(false);
  const [grouped, setGrouped] = useState(true);
  const [collapsedGroups, setCollapsedGroups] = useState(() => new Set());
  // Per-page density override. Persists to localStorage so an operator
  // who always wants compact rows doesn't reset the global tweak.
  const [compact, setCompactState] = useState(() => {
    try { return localStorage.getItem("rh.panel.agentsCompact") === "1"; } catch (e) { return false; }
  });
  const setCompact = React.useCallback((v) => {
    setCompactState(prev => {
      const next = typeof v === "function" ? v(prev) : v;
      try { localStorage.setItem("rh.panel.agentsCompact", next ? "1" : "0"); } catch (e) {}
      return next;
    });
  }, []);

  const [resp, fetchErr, refresh] = usePolling("/api/agents?limit=200", 15000);
  // `n` opens the spawn modal, `r` refreshes — listeners scoped to this
  // page via the rh:hotkey:* CustomEvents App dispatches.
  React.useEffect(() => {
    const onNew = (e) => { if (e.detail && e.detail.page === "agents") setShowSpawn(true); };
    const onRefresh = (e) => { if (e.detail && e.detail.page === "agents") refresh(); };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [refresh]);
  const agents = (resp && resp.agents) ? resp.agents.map(normalizeAgent) : D.agents;
  const filtered = agents.filter(a => {
    if (filter !== "all" && a.state !== filter && !(filter === "running" && a.state === "running")) return false;
    if (q && !a.name.toLowerCase().includes(q.toLowerCase()) && !a.model.toLowerCase().includes(q.toLowerCase()) && !(a.group || "").toLowerCase().includes(q.toLowerCase())) return false;
    return true;
  });

  // Drop selected ids that no longer exist (e.g. after kill) so the
  // bulk toolbar count stays honest.
  React.useEffect(() => {
    const live = new Set(agents.map(a => a.id));
    setSelected(prev => {
      const next = new Set([...prev].filter(id => live.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [agents.map(a => a.id).join(",")]);

  // Group filtered agents by their `group` field, sorted by name within
  // each group. Section order is stable across renders so the layout
  // doesn't shift between polls.
  const groupBuckets = React.useMemo(() => {
    if (!grouped) return null;
    const map = new Map();
    for (const a of filtered) {
      const g = a.group || "—";
      if (!map.has(g)) map.set(g, []);
      map.get(g).push(a);
    }
    const ordered = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
    for (const [, arr] of ordered) arr.sort((x, y) => x.name.localeCompare(y.name));
    return ordered;
  }, [filtered, grouped]);

  const toggleSelect = (id) => {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const toggleSelectAll = (visible) => {
    setSelected(prev => {
      if (visible.every(a => prev.has(a.id))) {
        const next = new Set(prev);
        for (const a of visible) next.delete(a.id);
        return next;
      }
      const next = new Set(prev);
      for (const a of visible) next.add(a.id);
      return next;
    });
  };
  const toggleGroup = (g) => {
    setCollapsedGroups(prev => {
      const next = new Set(prev);
      if (next.has(g)) next.delete(g); else next.add(g);
      return next;
    });
  };

  const killAgent = async (id) => {
    if (!(await confirmDialog({ title: "Kill agent", message: `Kill agent ${id}?`, danger: true, confirmLabel: "Kill" }))) return;
    try {
      await rhFetch(`/api/agents/${encodeURIComponent(id)}`, { method: "DELETE" });
      refresh();
    } catch (e) { toastErr(`kill failed: ${e.message || e}`); }
  };
  const restartAgent = async (id) => {
    try {
      await rhFetch(`/api/agents/${encodeURIComponent(id)}/restart`, { method: "POST" });
      refresh();
    } catch (e) { toastErr(`restart failed: ${e.message || e}`); }
  };
  const forkAgent = async (id, name) => {
    const proposed = `${name}-fork`;
    const ans = window.prompt(`Fork agent ${name} as:`, proposed);
    if (!ans || !ans.trim()) return;
    try {
      const r = await rhFetch(`/api/agents/${encodeURIComponent(id)}/clone`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ new_name: ans.trim() }),
      });
      toastOk(`Forked ${name} → ${ans.trim()}`);
      refresh();
    } catch (e) { toastErr(`fork failed: ${e.message || e}`); }
  };

  // Bulk actions: one network call per id, sequential to keep server
  // load predictable. We track partial successes so a single error
  // mid-batch doesn't lose progress visually.
  const bulkKill = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    if (!(await confirmDialog({ title: "Kill agents", message: `Kill ${ids.length} agent(s)? This cannot be undone.`, danger: true, confirmLabel: "Kill all" }))) return;
    let ok = 0, fail = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/agents/${encodeURIComponent(id)}`, { method: "DELETE" });
        ok++;
      } catch (e) {
        fail++;
        toastErr(`kill ${String(id).slice(0, 8)}: ${e.message || e}`);
      }
    }
    if (ok > 0) toastOk(`Killed ${ok} agent${ok === 1 ? "" : "s"}${fail ? ` (${fail} failed)` : ""}`);
    setSelected(new Set());
    refresh();
  };
  const bulkRestart = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    if (!(await confirmDialog({ title: "Restart agents", message: `Restart ${ids.length} agent(s)?`, confirmLabel: "Restart all" }))) return;
    let ok = 0, fail = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/agents/${encodeURIComponent(id)}/restart`, { method: "POST" });
        ok++;
      } catch (e) {
        fail++;
        toastErr(`restart ${String(id).slice(0, 8)}: ${e.message || e}`);
      }
    }
    if (ok > 0) toastOk(`Restarted ${ok} agent${ok === 1 ? "" : "s"}${fail ? ` (${fail} failed)` : ""}`);
    refresh();
  };
  // Bulk move-to-group: ask once for a target group name (with autocomplete
  // sourced from existing groups), then PATCH each agent's config with the
  // new group. Empty string clears the group, putting the agent under "—".
  const bulkMoveToGroup = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    const known = [...new Set(agents.map(a => a.group).filter(Boolean))].sort();
    const promptText = known.length > 0
      ? `Move ${ids.length} agent(s) to group:\n\nExisting groups: ${known.join(", ")}\n(empty = ungrouped)`
      : `Move ${ids.length} agent(s) to group:\n(empty = ungrouped)`;
    const ans = window.prompt(promptText, known[0] || "");
    if (ans == null) return; // user pressed Cancel
    const targetGroup = ans.trim();
    let ok = 0, fail = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/agents/${encodeURIComponent(id)}/config`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ group: targetGroup }),
        });
        ok++;
      } catch (e) {
        fail++;
        toastErr(`move ${String(id).slice(0, 8)}: ${e.message || e}`);
      }
    }
    if (ok > 0) {
      toastOk(`Moved ${ok} agent${ok === 1 ? "" : "s"} → ${targetGroup || "(ungrouped)"}${fail ? ` (${fail} failed)` : ""}`);
    }
    setSelected(new Set());
    refresh();
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Agents <span className="dim mono" style={{fontSize:14}}>· {agents.length}</span></h1>
          <p className="page-sub">Manifests live in <span className="mono">agents/&lt;name&gt;/agent.toml</span>. Hot-reloaded on save.</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
          <button className="btn ghost" onClick={() => setShowSpawn(true)}><I.copy/> Templates</button>
          <button className="btn primary" onClick={() => setShowSpawn(true)}><I.plus/> Spawn agent</button>
        </div>
      </div>

      {fetchErr && (
        <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
          <span className="dot err"/>
          <span className="banner-title">API ERROR</span>
          <span className="banner-body">Failed to load agents from kernel: <span className="mono">{fetchErr}</span></span>
        </div>
      )}

      <div className="filter-bar">
        <div className="seg">
          <button className={filter==="all"?"on":""} onClick={()=>setFilter("all")}>All · {agents.length}</button>
          <button className={filter==="running"?"on":""} onClick={()=>setFilter("running")}>Live · {agents.filter(a=>a.state==="running").length}</button>
          <button className={filter==="error"?"on":""} onClick={()=>setFilter("error")}>Errors · {agents.filter(a=>a.state==="error").length}</button>
          <button className={filter==="idle"?"on":""} onClick={()=>setFilter("idle")}>Idle · {agents.filter(a=>a.state==="idle").length}</button>
        </div>
        <div className="search-field">
          <I.search/>
          <input placeholder="Find agent, group, model…" value={q} onChange={e=>setQ(e.target.value)}/>
          <span className="kbd">⌘K</span>
        </div>
        <button className="btn ghost" onClick={() => setGrouped(g => !g)} title={grouped ? "Show flat list" : "Group by team"}>
          {grouped ? "Flat" : "Group"}
        </button>
        <button className="btn ghost" onClick={() => setCompact(c => !c)} title={compact ? "Comfortable rows" : "Dense rows"}>
          {compact ? "Compact" : "Cosy"}
        </button>
      </div>

      {selected.size > 0 && (
        <div className="bulk-bar">
          <span className="mono" style={{fontSize:12}}>{selected.size} selected</span>
          <button className="btn sm" onClick={bulkRestart}><I.refresh/> Restart</button>
          <button className="btn sm" onClick={bulkMoveToGroup} title="Move all selected agents to a single group">
            <I.link/> Move to group
          </button>
          {selected.size === 2 && (
            <button className="btn sm" onClick={() => setShowDiff(true)} title="Compare the two selected agents">
              <I.copy/> Diff
            </button>
          )}
          <button className="btn sm danger" onClick={bulkKill}><I.close/> Kill</button>
          <button className="btn sm ghost" onClick={() => setSelected(new Set())} style={{marginLeft:"auto"}}>Clear</button>
        </div>
      )}

      <div className="card flush" data-density={compact ? "compact" : ""}>
        <table className="tbl">
          <thead><tr>
            <th style={{width:30}}>
              <input
                type="checkbox"
                checked={filtered.length > 0 && filtered.every(a => selected.has(a.id))}
                ref={el => { if (el) el.indeterminate = selected.size > 0 && !filtered.every(a => selected.has(a.id)); }}
                onChange={() => toggleSelectAll(filtered)}
                title={filtered.every(a => selected.has(a.id)) ? "Deselect all" : "Select all"}/>
            </th>
            <th>Agent</th>{!grouped && <th>Group</th>}<th>Model</th><th>State</th>
            <th className="right">Msgs</th><th className="right">Cost · 24h</th><th>Last activity</th><th>Updated</th><th></th>
          </tr></thead>
          <tbody>
            {!resp && Array.from({length:5}).map((_,i) => (
              <SkelRow key={`s-${i}`} cols={[20, 160, 140, 90, 60, 60, 240, 50, 24]}/>
            ))}
            {!grouped && filtered.map(a => (
              <AgentRow key={a.id} agent={a} selected={selected.has(a.id)} onSelect={toggleSelect}
                        openAgent={openAgent} rowMenu={rowMenu} setRowMenu={setRowMenu}
                        restart={restartAgent} kill={killAgent} fork={forkAgent} showGroup={true}/>
            ))}
            {grouped && groupBuckets && groupBuckets.map(([g, arr]) => {
              const collapsed = collapsedGroups.has(g);
              return (
                <React.Fragment key={g}>
                  <tr className="group-row" onClick={() => toggleGroup(g)} style={{cursor:"pointer"}}>
                    <td colSpan={9} style={{padding:"6px 14px", background:"var(--bg-2)", borderBottom:"1px solid var(--border)"}}>
                      <span className="mono" style={{fontSize:11, letterSpacing:".12em", textTransform:"uppercase", color:"var(--fg-3)"}}>
                        {collapsed ? "▸" : "▾"} {g} <span className="dim" style={{marginLeft:6}}>{arr.length}</span>
                      </span>
                    </td>
                  </tr>
                  {!collapsed && arr.map(a => (
                    <AgentRow key={a.id} agent={a} selected={selected.has(a.id)} onSelect={toggleSelect}
                              openAgent={openAgent} rowMenu={rowMenu} setRowMenu={setRowMenu}
                              restart={restartAgent} kill={killAgent} fork={forkAgent} showGroup={false}/>
                  ))}
                </React.Fragment>
              );
            })}
          </tbody>
        </table>
      </div>

      {showSpawn && <SpawnAgentModal onClose={() => setShowSpawn(false)} onSpawned={() => { setShowSpawn(false); refresh(); }}/>}
      {showDiff && selected.size === 2 && (
        <AgentDiffModal
          agents={[...selected].map(id => agents.find(a => a.id === id)).filter(Boolean)}
          onClose={() => setShowDiff(false)}
        />
      )}
    </div>
  );
}

// Side-by-side diff of two agents' agent.toml manifests. Fetches both
// in parallel, then runs a tiny line-level diff (LCS-free, just marks
// lines that don't match between the two files at the same index). Good
// enough for spotting tool/skill/model drift between siblings.
function AgentDiffModal({ agents, onClose }) {
  const [a, b] = agents;
  const [contentA, setContentA] = useState(null);
  const [contentB, setContentB] = useState(null);
  const [err, setErr] = useState(null);
  useEscapeKey(onClose);
  React.useEffect(() => {
    let cancelled = false;
    const load = async () => {
      try {
        const [ra, rb] = await Promise.all([
          rhFetch(`/api/agents/${encodeURIComponent(a.id)}/files/agent.toml`),
          rhFetch(`/api/agents/${encodeURIComponent(b.id)}/files/agent.toml`),
        ]);
        if (cancelled) return;
        setContentA(typeof ra === "string" ? ra : (ra && ra.content) || "");
        setContentB(typeof rb === "string" ? rb : (rb && rb.content) || "");
      } catch (e) {
        if (!cancelled) setErr(String(e.message || e));
      }
    };
    load();
    return () => { cancelled = true; };
  }, [a.id, b.id]);
  const linesA = (contentA || "").split("\n");
  const linesB = (contentB || "").split("\n");
  const setB = new Set(linesB);
  const setA = new Set(linesA);
  // Stats: lines present in exactly one side.
  const onlyA = linesA.filter(l => l.trim() && !setB.has(l)).length;
  const onlyB = linesB.filter(l => l.trim() && !setA.has(l)).length;
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal lg" onClick={e => e.stopPropagation()} style={{maxWidth:1100, width:"95%"}}>
        <div className="modal-head">
          <div>
            <h3 className="modal-title">Diff: {a.name} ↔ {b.name}</h3>
            <p className="modal-sub mono" style={{fontSize:11}}>
              {contentA == null || contentB == null ? "loading…" : (
                <>
                  <span style={{color:"oklch(0.66 0.18 25)"}}>−{onlyA}</span> only in {a.name} ·{" "}
                  <span style={{color:"oklch(0.66 0.15 155)"}}>+{onlyB}</span> only in {b.name}
                </>
              )}
            </p>
          </div>
          <button className="btn ghost" onClick={onClose}><I.close/></button>
        </div>
        {err && (
          <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)", margin:"8px 16px"}}>
            <span className="dot err"/><span className="banner-title">LOAD FAILED</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>
        )}
        {!err && (contentA == null || contentB == null) && (
          <div className="muted mono" style={{padding:24, fontSize:12, textAlign:"center"}}>loading manifests…</div>
        )}
        {!err && contentA != null && contentB != null && (
          <div className="grid-12" style={{margin:"0 16px 16px"}}>
            <div className="col-6">
              <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
                {a.name} <span className="dim" style={{marginLeft:6}}>{linesA.length} lines</span>
              </div>
              <pre className="mono" style={{
                fontSize:11.5, lineHeight:1.5, maxHeight:520, overflow:"auto",
                background:"var(--bg-2)", padding:"10px 12px", borderRadius:6, margin:0,
              }}>
                {linesA.map((l, i) => {
                  const inB = setB.has(l);
                  return (
                    <div key={i} style={{
                      background: !inB && l.trim() ? "oklch(0.66 0.18 25 / .12)" : "transparent",
                      borderLeft: !inB && l.trim() ? "2px solid oklch(0.66 0.18 25 / .7)" : "2px solid transparent",
                      padding:"0 6px",
                      whiteSpace:"pre-wrap",
                      wordBreak:"break-word",
                    }}>{l || " "}</div>
                  );
                })}
              </pre>
            </div>
            <div className="col-6">
              <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
                {b.name} <span className="dim" style={{marginLeft:6}}>{linesB.length} lines</span>
              </div>
              <pre className="mono" style={{
                fontSize:11.5, lineHeight:1.5, maxHeight:520, overflow:"auto",
                background:"var(--bg-2)", padding:"10px 12px", borderRadius:6, margin:0,
              }}>
                {linesB.map((l, i) => {
                  const inA = setA.has(l);
                  return (
                    <div key={i} style={{
                      background: !inA && l.trim() ? "oklch(0.66 0.15 155 / .12)" : "transparent",
                      borderLeft: !inA && l.trim() ? "2px solid oklch(0.66 0.15 155 / .7)" : "2px solid transparent",
                      padding:"0 6px",
                      whiteSpace:"pre-wrap",
                      wordBreak:"break-word",
                    }}>{l || " "}</div>
                  );
                })}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// Single agent table row. Extracted so the grouped + flat list paths
// don't drift in cell ordering. `showGroup` toggles whether the Group
// column is rendered — hidden when the table is grouped (the section
// header carries the group name).
function AgentRow({ agent, selected, onSelect, openAgent, rowMenu, setRowMenu, restart, kill, fork, showGroup }) {
  const a = agent;
  return (
    <tr key={a.id} style={{cursor:"pointer", background: selected ? "var(--surface-2)" : undefined}}
        onClick={() => openAgent(a)}>
      <td onClick={(e) => { e.stopPropagation(); onSelect(a.id); }} style={{width:30}}>
        <input type="checkbox" checked={selected} readOnly tabIndex={-1}/>
      </td>
      <td>
        <div className="agent-row">
          <Avatar agent={a}/>
          <div>
            <div className="name">{a.name}</div>
            <div className="meta">{a.id}</div>
          </div>
        </div>
      </td>
      {showGroup && <td className="muted mono">{a.group}</td>}
      <td className="mono">{a.model}<div className="meta dim">{a.provider}</div></td>
      <td><StateBadge state={a.state}/></td>
      <td className="num mono">{a.messages.toLocaleString()}</td>
      <td className="num mono">${a.cost.toFixed(2)}</td>
      <td className="muted" style={{maxWidth:280,whiteSpace:"nowrap",overflow:"hidden",textOverflow:"ellipsis"}}>{a.last}</td>
      <td className="mono muted">{a.updated}</td>
      <td style={{position:"relative"}}>
        <button className="btn sm ghost" onClick={(e) => { e.stopPropagation(); setRowMenu(rowMenu === a.id ? null : a.id); }}>
          <I.more/>
        </button>
        {rowMenu === a.id && (
          <div className="row-menu" onClick={(e) => e.stopPropagation()}>
            <button onClick={() => { setRowMenu(null); restart(a.id); }}><I.refresh/> Restart</button>
            <button onClick={() => { setRowMenu(null); fork && fork(a.id, a.name); }}><I.copy/> Fork…</button>
            <button onClick={() => { setRowMenu(null); kill(a.id); }} style={{color:"var(--crimson)"}}><I.close/> Kill</button>
          </div>
        )}
      </td>
    </tr>
  );
}

function SpawnAgentModal({ onClose, onSpawned }) {
  useEscapeKey(onClose);
  const [templates] = useApi("/api/templates");
  const [profiles] = useApi("/api/profiles");
  const [mode, setMode] = useState("template"); // "template" | "custom"
  const [selectedTemplate, setSelectedTemplate] = useState(null);
  const [customName, setCustomName] = useState("");
  const [customProfile, setCustomProfile] = useState("research");
  const [customModel, setCustomModel] = useState("claude-sonnet-4");
  const [customProvider, setCustomProvider] = useState("anthropic");
  const [customManifest, setCustomManifest] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  const tmplList = Array.isArray(templates) ? templates : (templates && templates.templates) || [];
  const profileList = Array.isArray(profiles) ? profiles : (profiles && profiles.profiles) || [];

  // When the user picks a template, prefetch its manifest_toml so we can
  // POST exactly what the server hands out (avoids us re-generating).
  React.useEffect(() => {
    if (mode !== "template" || !selectedTemplate) return;
    let aborted = false;
    rhFetch(`/api/templates/${encodeURIComponent(selectedTemplate)}`)
      .then(d => { if (!aborted) setCustomManifest(d.manifest_toml || ""); })
      .catch(e => { if (!aborted) setErr(String(e.message || e)); });
    return () => { aborted = true; };
  }, [mode, selectedTemplate]);

  const generateManifest = () => {
    const name = customName.trim() || "new-agent";
    return `name = "${name}"
version = "0.1.0"
description = "Spawned from RustyHand control panel"
author = "operator"
module = "builtin:chat"

[model]
provider = "${customProvider}"
model = "${customModel}"
system_prompt = "You are a helpful agent."
temperature = 0.4
max_tokens = 2048

[capabilities]
tools = ["${customProfile}"]
`;
  };

  const spawn = async () => {
    setBusy(true);
    setErr(null);
    try {
      const manifest_toml = (mode === "template" && customManifest) || (mode === "custom" ? generateManifest() : customManifest);
      if (!manifest_toml.trim()) throw new Error("manifest is empty");
      await rhFetch("/api/agents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ manifest_toml }),
      });
      onSpawned();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-head">
          <b className="mono">Spawn agent</b>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="tabs" style={{marginBottom:14}}>
            <button className={mode === "template" ? "on" : ""} onClick={() => setMode("template")}>From template</button>
            <button className={mode === "custom" ? "on" : ""} onClick={() => setMode("custom")}>Custom</button>
          </div>
          {mode === "template" && (
            <div className="col gap-8">
              <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Templates ({tmplList.length})</span>
              <div className="col gap-4" style={{maxHeight:200, overflow:"auto"}}>
                {tmplList.length === 0 && <div className="dim mono" style={{fontSize:11}}>loading templates…</div>}
                {tmplList.map(t => (
                  <button key={t.name} onClick={() => setSelectedTemplate(t.name)}
                          className="row gap-8" style={{padding:"6px 10px", border:"1px solid var(--border)", borderRadius:6,
                                   background: selectedTemplate === t.name ? "var(--surface-2)" : "var(--bg-2)", textAlign:"left"}}>
                    <span className="mono" style={{fontSize:12}}>{t.name}</span>
                    {t.description && <span className="dim" style={{fontSize:11, marginLeft:"auto", maxWidth:280, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{t.description}</span>}
                  </button>
                ))}
              </div>
              {customManifest && (
                <>
                  <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Manifest preview</span>
                  <pre className="codebox" style={{maxHeight:160, overflow:"auto"}}>{customManifest}</pre>
                </>
              )}
            </div>
          )}
          {mode === "custom" && (
            <div className="col gap-8">
              <label className="t-row col">
                <span className="t-lbl">Name</span>
                <input className="modal-field" placeholder="my-agent" value={customName} onChange={e => setCustomName(e.target.value)}/>
              </label>
              <label className="t-row col">
                <span className="t-lbl">Provider</span>
                <input className="modal-field" value={customProvider} onChange={e => setCustomProvider(e.target.value)}/>
              </label>
              <label className="t-row col">
                <span className="t-lbl">Model</span>
                <input className="modal-field" value={customModel} onChange={e => setCustomModel(e.target.value)}/>
              </label>
              <label className="t-row col">
                <span className="t-lbl">Tool profile</span>
                <select className="t-select" value={customProfile} onChange={e => setCustomProfile(e.target.value)}>
                  {profileList.map(p => <option key={p.name || p} value={p.name || p}>{p.name || p}</option>)}
                </select>
              </label>
              <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Manifest preview</span>
              <pre className="codebox" style={{maxHeight:160, overflow:"auto"}}>{generateManifest()}</pre>
            </div>
          )}
          {err && <div className="banner" style={{marginTop:12, borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" disabled={busy || (mode === "template" && !selectedTemplate)} onClick={spawn}>
            {busy ? "Spawning…" : "Spawn"}
          </button>
        </div>
      </div>
    </div>
  );
}

function AgentDrawer({ agent, onClose }) {
  const [tab, setTab] = useState("info");
  useEffect(() => { setTab("info"); }, [agent && agent.id]);
  if (!agent) return null;
  const [detail, , refreshDetail] = useApi(agent ? `/api/agents/${agent.id}` : null);
  const [recent] = useApi(agent ? `/api/audit/recent?n=10&agent_id=${agent.id}` : null);
  const [budget] = useApi(agent ? `/api/budget/agents/${agent.id}` : null);

  const turns = (recent && recent.entries) || null;
  const cost24 = budget && budget.daily && budget.daily.spend != null ? Number(budget.daily.spend) : agent.cost;

  return (
    <>
      <div className={"drawer-back " + (agent ? "open" : "")} onClick={onClose}/>
      <aside className={"drawer " + (agent ? "open" : "")}>
        <div className="drawer-head">
          <Avatar agent={agent} size="lg"/>
          <div className="col" style={{gap:2}}>
            <div style={{fontFamily:"var(--ff-mono)",fontSize:15}}>{agent.name}</div>
            <div className="dim mono" style={{fontSize:11}}>{agent.id} · {agent.group}</div>
          </div>
          <div style={{marginLeft:"auto"}} className="row gap-6">
            <button className="icon-btn" onClick={onClose}><I.close/></button>
          </div>
        </div>
        <div className="tabs" style={{margin:"0 16px"}}>
          <button className={tab==="info"?"on":""} onClick={()=>setTab("info")}>Info</button>
          <button className={tab==="config"?"on":""} onClick={()=>setTab("config")}>Config</button>
          <button className={tab==="identity"?"on":""} onClick={()=>setTab("identity")}>Identity</button>
          <button className={tab==="activity"?"on":""} onClick={()=>setTab("activity")}>Activity</button>
        </div>
        <div className="drawer-body">
          {tab === "info" && (
            <>
              <div className="row gap-8 mb-12">
                <StateBadge state={agent.state}/>
                <span className="badge plain">{agent.model}</span>
                <span className="badge plain">{agent.provider}</span>
              </div>
              <div className="kv mb-16">
                <dt>messages</dt><dd>{(agent.messages || 0).toLocaleString()}</dd>
                <dt>cost 24h</dt><dd>${(cost24 || 0).toFixed(2)}</dd>
                <dt>last</dt><dd>{agent.updated} ago</dd>
                <dt>circuit</dt><dd style={{color: agent.state === "error" ? "var(--crimson)" : "var(--live)"}}>{agent.state === "error" ? "OPEN" : "CLOSED"}</dd>
              </div>
              <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Description</div>
              <div className="codebox mb-16" style={{whiteSpace:"pre-wrap"}}>
                {detail && detail.description ? detail.description : "—"}
              </div>
              <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>System prompt</div>
              <pre className="codebox mb-16" style={{maxHeight:200}}>
{detail && detail.model && detail.model.system_prompt
  ? detail.model.system_prompt
  : "(loading or no system prompt)"}
              </pre>
            </>
          )}
          {tab === "config" && detail && (
            <>
              <AgentConfigForm agent={agent} detail={detail} onSaved={refreshDetail}/>
              <div className="divider"/>
              <AgentKvEditor agent={agent}/>
            </>
          )}
          {tab === "config" && !detail && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {tab === "identity" && <AgentIdentityForm agent={agent} detail={detail} onSaved={refreshDetail}/>}
          {tab === "activity" && (
            <>
              <AgentActivityCharts agent={agent} budget={budget} turns={turns}/>
              <div className="muted mono mb-8 mt-16" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Recent turns</div>
              <div className="col gap-6">
                {!turns && <div className="dim mono" style={{fontSize:11.5, padding:"6px 8px"}}>loading audit…</div>}
                {turns && turns.length === 0 && <div className="dim mono" style={{fontSize:11.5, padding:"6px 8px"}}>no audit entries for this agent yet.</div>}
                {turns && turns.map((r, i) => (
                  <div key={r.hash || r.seq || i} className="row" style={{padding:"6px 8px",borderRadius:6,background:"var(--bg-2)"}}>
                    <span className="mono dim" style={{fontSize:11, width:70}}>{formatTime(r.timestamp)}</span>
                    <span className="mono" style={{fontSize:12, flex:1, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{r.action}</span>
                    <span className="mono dim" style={{fontSize:11, marginLeft:8, maxWidth:160, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{r.detail || r.outcome || ""}</span>
                  </div>
                ))}
              </div>
            </>
          )}
        </div>
      </aside>
    </>
  );
}

// Mini-charts for the agent drawer's Activity tab. We don't have a
// pre-computed time series for any single agent, so we synthesize from
// what's available: budget hourly/daily/monthly tiles + activity tally
// per audit-action bucket. Best with `useApi("/api/agents/{id}/metrics")`
// once that endpoint surfaces per-bucket history, but useful as-is for
// an at-a-glance read.
function AgentActivityCharts({ agent, budget, turns }) {
  // Poll metrics every 5s so the live sparkline below has data to plot.
  // usePolling stops on null path; the drawer unmount clears the
  // interval so the kernel doesn't keep getting pinged after close.
  const [metricsResp] = usePolling(agent ? `/api/agents/${agent.id}/metrics` : null, 5000);
  const metrics = metricsResp || {};
  // Per-agent ring buffer of metric samples for the live sparkline. We
  // track absolute total_tokens and message_count and render the deltas
  // so the chart shows what the agent did since the drawer opened, not
  // its lifetime total.
  const [history, setHistory] = React.useState([]);
  React.useEffect(() => {
    if (!metricsResp || metricsResp.total_tokens == null) return;
    setHistory(prev => {
      const next = [...prev, {
        t: Date.now(),
        tokens: Number(metricsResp.total_tokens || 0),
        msgs: Number(metricsResp.message_count || 0),
      }];
      return next.length > 60 ? next.slice(next.length - 60) : next;
    });
  }, [metricsResp && metricsResp.total_tokens, metricsResp && metricsResp.message_count]);
  // Reset when the drawer opens for a different agent.
  React.useEffect(() => { setHistory([]); }, [agent && agent.id]);
  // Convert absolute snapshots to per-tick deltas. Each sample becomes
  // (tokens_added_since_prev, msgs_added_since_prev).
  const deltas = React.useMemo(() => {
    if (history.length < 2) return [];
    const out = [];
    for (let i = 1; i < history.length; i++) {
      out.push({
        t: history[i].t,
        tokens: Math.max(0, history[i].tokens - history[i - 1].tokens),
        msgs: Math.max(0, history[i].msgs - history[i - 1].msgs),
      });
    }
    return out;
  }, [history]);
  const hourly = budget && budget.hourly ? budget.hourly : { spend: 0, limit: 0 };
  const daily = budget && budget.daily ? budget.daily : { spend: 0, limit: 0 };
  const monthly = budget && budget.monthly ? budget.monthly : { spend: 0, limit: 0 };

  // Tally audit entries by action — gives a sparkline-style preview of
  // what the agent has been doing. Useful for spotting hot-spots
  // (e.g. agent stuck in a tool-call loop will show one action dominating).
  const actionCounts = React.useMemo(() => {
    if (!Array.isArray(turns)) return [];
    const map = new Map();
    for (const t of turns) {
      const k = t.action || "(unknown)";
      map.set(k, (map.get(k) || 0) + 1);
    }
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [turns]);
  const maxCount = Math.max(1, ...actionCounts.map(([, n]) => n));

  // BudgetBar shows spend/limit fraction with a small color hint at risk.
  const BudgetBar = ({ label, info }) => {
    const spend = Number(info.spend || 0);
    const limit = Number(info.limit || 0);
    const pct = limit > 0 ? Math.min(100, Math.round((spend / limit) * 100)) : 0;
    const danger = pct >= 80;
    return (
      <div className="row gap-8 mb-4">
        <span className="dim mono" style={{fontSize:10.5, width:70, letterSpacing:".08em", textTransform:"uppercase"}}>{label}</span>
        <span className="bar" style={{flex:1, height:8, background:"var(--bg-2)", border:"1px solid var(--border)", borderRadius:3, position:"relative", overflow:"hidden"}}>
          <span style={{display:"block", height:"100%", width: `${pct}%`,
            background: danger
              ? "linear-gradient(90deg, var(--crimson), oklch(0.6 0.15 25))"
              : "linear-gradient(90deg, var(--rust), var(--rust-2))"}}/>
        </span>
        <span className="mono nums" style={{fontSize:11, width:120, textAlign:"right"}}>
          ${spend.toFixed(2)} / {limit > 0 ? `$${limit.toFixed(2)}` : "—"}
        </span>
      </div>
    );
  };

  return (
    <div>
      <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Budget usage</div>
      <div className="card" style={{padding:10, marginBottom:12}}>
        <BudgetBar label="hour" info={hourly}/>
        <BudgetBar label="day" info={daily}/>
        <BudgetBar label="month" info={monthly}/>
      </div>
      <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
        Activity histogram <span className="dim" style={{marginLeft:6, fontSize:10}}>(last {turns ? turns.length : 0} audit events)</span>
      </div>
      <div className="card" style={{padding:10}}>
        {actionCounts.length === 0 && <div className="dim" style={{fontSize:11.5}}>no audit events to plot.</div>}
        {actionCounts.map(([action, n]) => (
          <div key={action} className="row gap-8 mb-4">
            <span className="mono" style={{fontSize:11, width:120, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{action}</span>
            <span style={{flex:1, height:8, background:"var(--bg-2)", border:"1px solid var(--border)", borderRadius:3, overflow:"hidden"}}>
              <span style={{display:"block", height:"100%", width: `${(n / maxCount) * 100}%`, background:"linear-gradient(90deg, var(--violet), oklch(0.55 0.12 295))"}}/>
            </span>
            <span className="mono nums" style={{fontSize:11, width:30, textAlign:"right"}}>{n}</span>
          </div>
        ))}
      </div>
      {metrics && metrics.total_tokens != null && (
        <div className="kv mt-12" style={{fontSize:12}}>
          <dt>total tokens</dt><dd>{Number(metrics.total_tokens || 0).toLocaleString()}</dd>
          <dt>messages</dt><dd>{Number(metrics.message_count || 0).toLocaleString()}</dd>
          <dt>last activity</dt><dd className="mono dim">{metrics.last_activity ? relativeTime(metrics.last_activity) : "—"}</dd>
        </div>
      )}
      <div className="muted mono mt-12 mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
        Live throughput <span className="dim" style={{marginLeft:6, fontSize:10}}>(per 5s tick · since drawer opened)</span>
      </div>
      <div className="card" style={{padding:10}}>
        {deltas.length === 0 && (
          <div className="dim" style={{fontSize:11.5, padding:"6px 0"}}>collecting samples…</div>
        )}
        {deltas.length > 0 && (
          <AgentLiveSpark deltas={deltas}/>
        )}
      </div>
    </div>
  );
}

// Inline SVG sparkline of agent's recent throughput. Plots tokens/tick
// as a filled area + msgs/tick as a thin overlay line. Auto-scales to
// the visible range so a long quiet stretch doesn't flatten everything.
function AgentLiveSpark({ deltas }) {
  if (!deltas || deltas.length === 0) return null;
  const W = 320, H = 56, PAD = 4;
  const tokens = deltas.map(d => d.tokens);
  const msgs = deltas.map(d => d.msgs);
  const maxTokens = Math.max(1, ...tokens);
  const maxMsgs = Math.max(1, ...msgs);
  const xStep = (W - 2 * PAD) / Math.max(1, deltas.length - 1);
  const tokenPath = deltas.map((d, i) => {
    const x = PAD + i * xStep;
    const y = H - PAD - (d.tokens / maxTokens) * (H - 2 * PAD);
    return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const tokenArea = `${tokenPath} L${(W - PAD).toFixed(1)},${H - PAD} L${PAD},${H - PAD} Z`;
  const msgPath = deltas.map((d, i) => {
    const x = PAD + i * xStep;
    const y = H - PAD - (d.msgs / maxMsgs) * (H - 2 * PAD);
    return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const totalTokens = tokens.reduce((s, x) => s + x, 0);
  const totalMsgs = msgs.reduce((s, x) => s + x, 0);
  return (
    <div>
      <svg width="100%" height={H} viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none">
        <path d={tokenArea} fill="oklch(0.665 0.165 50 / .2)"/>
        <path d={tokenPath} fill="none" stroke="var(--rust)" strokeWidth="1.5"/>
        <path d={msgPath} fill="none" stroke="var(--violet)" strokeWidth="1" strokeDasharray="2 2"/>
      </svg>
      <div className="row gap-12 mt-4 dim mono" style={{fontSize:10.5}}>
        <span><span style={{color:"var(--rust)"}}>●</span> tokens · {totalTokens.toLocaleString()} ({maxTokens}/tick peak)</span>
        <span><span style={{color:"var(--violet)"}}>●</span> msgs · {totalMsgs} ({maxMsgs}/tick peak)</span>
        <span style={{marginLeft:"auto"}}>{deltas.length} samples</span>
      </div>
    </div>
  );
}

function AgentConfigForm({ agent, detail, onSaved }) {
  const current = detail || {};
  const model = current.model || {};
  const initial = React.useMemo(() => ({
    name: current.name || agent.name,
    group: current.group || agent.group || "",
    description: current.description || "",
    system_prompt: model.system_prompt || "",
    temperature: model.temperature != null ? String(model.temperature) : "0.4",
    max_tokens: model.max_tokens != null ? String(model.max_tokens) : "2048",
    thinking_enabled: !!model.thinking,
    model: model.model || agent.model,
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }), [agent && agent.id, detail]);
  const [name, setName] = useState(initial.name);
  const [group, setGroup] = useState(initial.group);
  const [description, setDescription] = useState(initial.description);
  const [systemPrompt, setSystemPrompt] = useState(initial.system_prompt);
  const [temperature, setTemperature] = useState(initial.temperature);
  const [maxTokens, setMaxTokens] = useState(initial.max_tokens);
  const [thinkingEnabled, setThinkingEnabled] = useState(initial.thinking_enabled);
  const [modelName, setModelName] = useState(initial.model);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const [ok, setOk] = useState(false);

  // Source available models for the dropdown. Filter to those whose
  // provider matches the agent (so picking "gpt-4o-mini" on an
  // anthropic-only agent doesn't appear as a viable option). If the
  // current model isn't in the list (e.g. custom alias), it's still
  // selectable as the first option so we don't lose data.
  const [modelsResp] = useApi("/api/models");
  const allModels = (modelsResp && modelsResp.models) || [];
  const provider = (model && model.provider) || agent.provider;
  const modelOptions = allModels
    .filter(m => !provider || m.provider === provider)
    .sort((a, b) => (a.display_name || a.id).localeCompare(b.display_name || b.id));

  // Compute the diff between current state and the initial detail. The
  // operator sees what's actually being saved before clicking — guards
  // against accidental changes (especially in a long system prompt).
  const diff = React.useMemo(() => {
    const rows = [];
    const push = (label, oldV, newV) => {
      if (String(oldV ?? "") !== String(newV ?? "")) rows.push({ label, oldV, newV });
    };
    push("name", initial.name, name);
    push("group", initial.group, group);
    push("description", initial.description, description);
    push("system_prompt", initial.system_prompt, systemPrompt);
    push("temperature", initial.temperature, temperature);
    push("max_tokens", initial.max_tokens, maxTokens);
    push("thinking_enabled", initial.thinking_enabled, thinkingEnabled);
    push("model", initial.model, modelName);
    return rows;
  }, [initial, name, group, description, systemPrompt, temperature, maxTokens, thinkingEnabled, modelName]);

  const save = async () => {
    if (diff.length === 0) { setOk(true); return; }
    setBusy(true); setErr(null); setOk(false);
    try {
      // Only PATCH fields that actually changed — minimal blast radius
      // if a downstream validator gets stricter about untouched fields.
      const patch = {};
      for (const r of diff) {
        if (r.label === "temperature") patch.temperature = Number(r.newV);
        else if (r.label === "max_tokens") patch.max_tokens = Number(r.newV);
        else if (r.label === "thinking_enabled") patch.thinking_enabled = !!r.newV;
        else if (r.label === "system_prompt") patch.system_prompt = r.newV;
        else if (r.label !== "model") patch[r.label] = r.newV;
      }
      if (Object.keys(patch).length > 0) {
        await rhFetch(`/api/agents/${agent.id}/config`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(patch),
        });
      }
      const modelChanged = diff.some(r => r.label === "model");
      if (modelChanged && modelName) {
        await rhFetch(`/api/agents/${agent.id}/model`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ model: modelName }),
        });
      }
      toastOk(`Saved ${diff.length} change${diff.length === 1 ? "" : "s"} to ${agent.name}`);
      setOk(true);
      onSaved();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="col gap-8">
      <label className="t-row col"><span className="t-lbl">Name</span>
        <input className="modal-field" value={name} onChange={e => setName(e.target.value)}/></label>
      <label className="t-row col"><span className="t-lbl">Group</span>
        <input className="modal-field" value={group} onChange={e => setGroup(e.target.value)}/></label>
      <label className="t-row col"><span className="t-lbl">Description</span>
        <textarea className="modal-field modal-textarea" style={{minHeight:60}} value={description} onChange={e => setDescription(e.target.value)}/></label>
      <label className="t-row col"><span className="t-lbl">System prompt</span>
        <textarea className="modal-field modal-textarea" style={{minHeight:120}} value={systemPrompt} onChange={e => setSystemPrompt(e.target.value)}/></label>
      <label className="t-row col">
        <span className="t-lbl">
          Model
          {modelOptions.length === 0 && modelsResp && <span className="dim" style={{marginLeft:6, fontSize:10}}>(catalog empty for provider {provider})</span>}
        </span>
        {modelOptions.length > 0 ? (
          <select className="t-select" value={modelName} onChange={e => setModelName(e.target.value)}>
            {!modelOptions.some(m => m.id === modelName) && modelName && (
              <option value={modelName}>{modelName} (custom)</option>
            )}
            {modelOptions.map(m => (
              <option key={m.id} value={m.id}>
                {m.display_name || m.id}{m.tier ? ` · ${m.tier}` : ""}{m.available === false ? " · not configured" : ""}
              </option>
            ))}
          </select>
        ) : (
          <input className="modal-field" value={modelName} onChange={e => setModelName(e.target.value)}/>
        )}
      </label>
      <div className="row gap-12">
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Temperature (0–2)</span>
          <input className="modal-field" type="number" step="0.05" min="0" max="2" value={temperature} onChange={e => setTemperature(e.target.value)}/></label>
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Max tokens</span>
          <input className="modal-field" type="number" min="1" value={maxTokens} onChange={e => setMaxTokens(e.target.value)}/></label>
      </div>
      <div className="t-row">
        <span className="t-lbl">Thinking enabled</span>
        <div className={"switch " + (thinkingEnabled ? "on" : "")} onClick={() => setThinkingEnabled(v => !v)}/>
      </div>
      {diff.length > 0 && (
        <div className="config-diff">
          <div className="config-diff-head">
            <span className="mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", color:"var(--fg-3)"}}>
              Pending changes · {diff.length}
            </span>
          </div>
          {diff.map(r => (
            <div key={r.label} className="config-diff-row">
              <span className="mono" style={{fontSize:11.5, color:"var(--fg-2)", width:140}}>{r.label}</span>
              <span className="mono" style={{fontSize:11, color:"var(--crimson)", flex:1, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>
                {String(r.oldV ?? "").slice(0, 80) || "—"}
              </span>
              <span style={{color:"var(--fg-4)"}}>→</span>
              <span className="mono" style={{fontSize:11, color:"var(--live)", flex:1, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>
                {String(r.newV ?? "").slice(0, 80) || "—"}
              </span>
            </div>
          ))}
        </div>
      )}
      {err && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
      {ok && diff.length === 0 && <div className="banner" style={{borderColor:"oklch(0.74 0.135 150 / .35)"}}>
        <span className="dot live"/><span className="banner-title">UP TO DATE</span>
        <span className="banner-body" style={{fontSize:11.5}}>No changes to save.</span></div>}
      <div className="row" style={{justifyContent:"flex-end", marginTop:8}}>
        <button className="btn primary" onClick={save} disabled={busy}>
          {busy ? "Saving…" : (diff.length === 0 ? "Up to date" : `Save ${diff.length} change${diff.length === 1 ? "" : "s"}`)}
        </button>
      </div>
    </div>
  );
}

// AgentKvEditor — per-agent key/value store backed by
// /api/memory/agents/{id}/kv. Values are JSON-typed; the editor shows
// the raw JSON in the textarea so structured values (objects, arrays)
// can be edited without a complex tree UI. Submission either parses
// the input as JSON or falls back to a plain string.
function AgentKvEditor({ agent }) {
  const path = agent ? `/api/memory/agents/${agent.id}/kv` : null;
  const [resp, fetchErr, refresh] = useApi(path);
  const pairs = (resp && resp.kv) || [];
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [editingKey, setEditingKey] = useState(null);
  const [editDraft, setEditDraft] = useState("");
  const [busy, setBusy] = useState(false);

  const parseValue = (s) => {
    const trimmed = s.trim();
    if (trimmed === "") return "";
    try { return JSON.parse(trimmed); }
    catch (_) { return s; }
  };

  const save = async (k, v) => {
    if (!k.trim()) { toastErr("Key required"); return; }
    setBusy(true);
    try {
      await rhFetch(`/api/memory/agents/${agent.id}/kv/${encodeURIComponent(k.trim())}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ value: parseValue(v) }),
      });
      toastOk(`Saved ${k}`);
      setNewKey(""); setNewValue("");
      setEditingKey(null); setEditDraft("");
      refresh();
    } catch (e) { toastErr(`save failed: ${e.message || e}`); }
    finally { setBusy(false); }
  };

  const remove = async (k) => {
    if (!(await confirmDialog({ title: "Delete key", message: `Delete kv key "${k}"?`, danger: true, confirmLabel: "Delete" }))) return;
    setBusy(true);
    try {
      await rhFetch(`/api/memory/agents/${agent.id}/kv/${encodeURIComponent(k)}`, { method: "DELETE" });
      toastOk(`Deleted ${k}`);
      refresh();
    } catch (e) { toastErr(`delete failed: ${e.message || e}`); }
    finally { setBusy(false); }
  };

  return (
    <div className="col gap-8 mt-12">
      <div className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
        Environment / KV store
        <Tip>Key/value pairs stored per-agent and surfaced to skills as
          environment variables. Values are JSON-typed: numbers, strings,
          objects, arrays. Plain text is auto-quoted.</Tip>
      </div>
      {fetchErr && <div className="dim mono" style={{fontSize:11, color:"var(--crimson)"}}>{fetchErr}</div>}
      {!resp && <div className="dim mono" style={{fontSize:11.5}}>loading…</div>}
      {resp && pairs.length === 0 && (
        <div className="dim" style={{fontSize:11.5}}>No keys yet — add one below.</div>
      )}
      <div className="col gap-4">
        {pairs.map((p) => {
          const v = p.value;
          const display = typeof v === "string" ? v : JSON.stringify(v);
          const isEditing = editingKey === p.key;
          return (
            <div key={p.key} className="row gap-8" style={{padding:"6px 8px", background:"var(--bg-2)", borderRadius:6, alignItems:"flex-start"}}>
              <span className="mono" style={{fontSize:11.5, width:120, paddingTop:4}}>{p.key}</span>
              {isEditing ? (
                <textarea className="modal-field modal-textarea"
                          style={{flex:1, minHeight:36, fontSize:11.5, fontFamily:"var(--ff-mono)"}}
                          value={editDraft}
                          onChange={(e) => setEditDraft(e.target.value)}
                          autoFocus/>
              ) : (
                <span className="mono dim" style={{flex:1, fontSize:11.5, paddingTop:4, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{display}</span>
              )}
              {isEditing ? (
                <span className="row gap-4">
                  <button className="btn sm" onClick={() => save(p.key, editDraft)} disabled={busy}>save</button>
                  <button className="btn sm ghost" onClick={() => { setEditingKey(null); setEditDraft(""); }}>cancel</button>
                </span>
              ) : (
                <span className="row gap-4">
                  <button className="btn sm ghost" onClick={() => { setEditingKey(p.key); setEditDraft(typeof v === "string" ? v : JSON.stringify(v, null, 2)); }}>edit</button>
                  <button className="btn sm danger" onClick={() => remove(p.key)}><I.close/></button>
                </span>
              )}
            </div>
          );
        })}
      </div>
      <div className="row gap-6 mt-8">
        <input className="modal-field" style={{flex:0, width:140}} placeholder="new key" value={newKey} onChange={(e) => setNewKey(e.target.value)}/>
        <input className="modal-field" style={{flex:1}} placeholder='value (JSON or plain text)' value={newValue} onChange={(e) => setNewValue(e.target.value)}/>
        <button className="btn sm primary" onClick={() => save(newKey, newValue)} disabled={busy || !newKey.trim()}>
          <I.plus/> Add
        </button>
      </div>
    </div>
  );
}

function AgentIdentityForm({ agent, detail, onSaved }) {
  const id = (detail && detail.identity) || {};
  const [emoji, setEmoji] = useState(id.emoji || "");
  const [color, setColor] = useState(id.color || "");
  const [avatarUrl, setAvatarUrl] = useState(id.avatar_url || "");
  const [archetype, setArchetype] = useState(id.archetype || "");
  const [vibe, setVibe] = useState(id.vibe || "");
  const [greetingStyle, setGreetingStyle] = useState(id.greeting_style || "");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const [ok, setOk] = useState(false);

  const save = async () => {
    setBusy(true); setErr(null); setOk(false);
    try {
      await rhFetch(`/api/agents/${agent.id}/identity`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          emoji, color, avatar_url: avatarUrl, archetype, vibe, greeting_style: greetingStyle,
        }),
      });
      setOk(true);
      onSaved();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="col gap-8">
      <div className="row gap-12">
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Emoji</span>
          <input className="modal-field" value={emoji} maxLength={4} onChange={e => setEmoji(e.target.value)} placeholder="🦀"/></label>
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Color (hex, optional)</span>
          <div className="row gap-6">
            <input className="modal-field" style={{flex:1}} value={color} onChange={e => setColor(e.target.value)} placeholder="#d4541b"/>
            <input type="color" value={/^#[0-9a-fA-F]{6}$/.test(color) ? color : "#d4541b"}
                   onChange={e => setColor(e.target.value)}
                   style={{width:32, height:32, padding:0, border:"1px solid var(--border)", borderRadius:6, background:"var(--bg-2)"}}
                   title="Pick color"/>
          </div>
          <div className="row gap-4 mt-4">
            {["#d4541b", "#e0a52e", "#5d9c4d", "#3b82c0", "#9b6cd1", "#c44a73", "#6b6f74"].map(swatch => (
              <button key={swatch}
                      type="button"
                      onClick={() => setColor(swatch)}
                      className="swatch"
                      style={{background: swatch, outline: color === swatch ? "2px solid var(--rust)" : "none"}}
                      title={swatch}/>
            ))}
          </div>
        </label>
      </div>
      <label className="t-row col"><span className="t-lbl">Avatar URL (http/https/data)</span>
        <input className="modal-field" value={avatarUrl} onChange={e => setAvatarUrl(e.target.value)} placeholder="https://…"/></label>
      <div className="row gap-12">
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Archetype</span>
          <input className="modal-field" value={archetype} onChange={e => setArchetype(e.target.value)} placeholder="sage / sentinel / artisan"/></label>
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Vibe</span>
          <input className="modal-field" value={vibe} onChange={e => setVibe(e.target.value)} placeholder="calm / sharp / playful"/></label>
      </div>
      <label className="t-row col"><span className="t-lbl">Greeting style</span>
        <input className="modal-field" value={greetingStyle} onChange={e => setGreetingStyle(e.target.value)} placeholder="terse, formal, etc."/></label>
      {err && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
      {ok && <div className="banner" style={{borderColor:"oklch(0.74 0.135 150 / .35)"}}>
        <span className="dot live"/><span className="banner-title">SAVED</span></div>}
      <div className="row" style={{justifyContent:"flex-end", marginTop:8}}>
        <button className="btn primary" onClick={save} disabled={busy}>{busy ? "Saving…" : "Save identity"}</button>
      </div>
    </div>
  );
}

/* ============================== CHAT ============================== */
//
// Chat is wired straight to the kernel via HTTP. Switching agents loads
// their last session over GET /api/agents/{id}/session; sending a message
// hits POST /api/agents/{id}/message. Streaming (WS / SSE) is on the
// roadmap but the round-trip endpoint already renders the design's bubble
// + tool-trace layout once the agent's response lands.
function ChatPage() {
  const [agentsResp] = usePolling("/api/agents?limit=200", 20000);
  const agents = (agentsResp && agentsResp.agents) ? agentsResp.agents.map(normalizeAgent) : D.agents;
  const [activeId, setActiveId] = useState(null);
  const active = agents.find(a => a.id === activeId) || agents[0];

  const [session, sessionErr, refreshSession] = useApi(active ? `/api/agents/${active.id}/session` : null);
  const [budget] = useApi(active ? `/api/budget/agents/${active.id}` : null);
  const [toolsResp] = useApi("/api/tools");
  // /api/tools wraps the list in `{tools: [...]}`. Some older builds
  // returned a bare array — accept both rather than rely on the envelope
  // exclusively, so the chat side-panel doesn't break when the API tweaks.
  const tools = Array.isArray(toolsResp) ? toolsResp : (toolsResp && toolsResp.tools);

  // pendingMessages is the local optimistic buffer: user bubbles, the
  // currently-streaming assistant bubble, and any tool traces that have
  // arrived over WS but aren't yet reflected in the persisted session.
  const [pendingMessages, setPendingMessages] = useState([]);
  const [streamingText, setStreamingText] = useState("");
  const [streamingTools, setStreamingTools] = useState([]);
  const [sending, setSending] = useState(false);
  const [typed, setTyped] = useState("");
  const streamRef = useRef(null);

  useEffect(() => {
    // Reset everything on agent switch — session history takes over.
    setPendingMessages([]);
    setStreamingText("");
    setStreamingTools([]);
    setSending(false);
  }, [active && active.id]);

  useEffect(() => {
    if (streamRef.current) streamRef.current.scrollTop = streamRef.current.scrollHeight;
  }, [session, pendingMessages, streamingText, streamingTools, sending]);

  // WS event handler — dispatches into local optimistic buffers.
  const onWs = React.useCallback((msg) => {
    if (!msg || !msg.type) return;
    switch (msg.type) {
      case "text_delta":
        setStreamingText((prev) => prev + (msg.content || msg.text || ""));
        break;
      case "tool_start":
        setStreamingTools((prev) => prev.concat([{ name: msg.name || msg.tool, input: msg.args || msg.input, running: true }]));
        break;
      case "tool_end":
      case "tool_result":
        setStreamingTools((prev) => prev.map((t, i, arr) => i === arr.length - 1 && t.running
          ? { ...t, running: false, result: msg.result, is_error: !!msg.is_error }
          : t));
        break;
      case "response":
        // Final assistant message; flush to pending buffer + clear stream.
        setPendingMessages((prev) => prev.concat([{ role: "assistant", content: msg.content || streamingText, _local: true }]));
        setStreamingText("");
        setStreamingTools([]);
        setSending(false);
        refreshSession();
        break;
      case "silent_complete":
        setStreamingText("");
        setStreamingTools([]);
        setSending(false);
        refreshSession();
        break;
      case "error":
        setPendingMessages((prev) => prev.concat([{ role: "assistant", content: `[error] ${msg.content || msg.error || "unknown"}`, _local: true, error: true }]));
        setStreamingText("");
        setStreamingTools([]);
        setSending(false);
        break;
      case "command_result":
        setPendingMessages((prev) => prev.concat([{ role: "assistant", content: msg.message || msg.content || "ok", _local: true, command: true }]));
        setSending(false);
        break;
      default:
        // Quiet on connected/typing/phase/agents_updated/canvas — they don't
        // change the bubble layout.
        break;
    }
  }, [streamingText, refreshSession]);

  const ws = useAgentWs(active && active.id, onWs);

  if (!active) {
    return (
      <div className="muted mono" style={{padding:"40px 14px", fontSize:13}}>
        No agents loaded. Spawn one from the Agents page first.
      </div>
    );
  }

  const items = coalesceToolTraces(
    sessionToItems(session && session.messages).concat(pendingMessages)
  );

  const send = async () => {
    const text = typed.trim();
    if (!text || sending) return;
    setTyped("");
    setPendingMessages((prev) => prev.concat([{ role: "user", content: text, _local: true }]));
    setStreamingText("");
    setStreamingTools([]);
    setSending(true);
    // Slash commands: route through the WS command channel rather than
    // sending as a chat message. The kernel handles the dispatch with
    // no LLM round-trip and replies via command_result events.
    if (text.startsWith("/") && ws.connected) {
      const rest = text.slice(1).trim();
      const sp = rest.indexOf(" ");
      const command = sp < 0 ? rest : rest.slice(0, sp);
      const args = sp < 0 ? "" : rest.slice(sp + 1);
      if (ws.sendCommand(command, args)) return;
    }
    if (ws.connected && ws.send(text)) return;
    // Fallback to HTTP roundtrip if WS isn't connected.
    try {
      const resp = await rhFetch(`/api/agents/${active.id}/message`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message: text }),
      });
      if (resp && resp.response) {
        setPendingMessages((prev) => prev.concat([{ role: "assistant", content: resp.response, _local: true }]));
      }
    } catch (e) {
      setPendingMessages((prev) => prev.concat([{ role: "assistant", content: `[error] ${e.message || e}`, _local: true, error: true }]));
    } finally {
      setSending(false);
      refreshSession();
    }
  };

  const onKeyDown = (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  // Operator-resizable chat columns. Widths persist to localStorage so
  // the same operator sees their preferred layout next visit. We use
  // mousedown→mousemove on document to drag — keeps the handle cheap
  // and avoids the React state churn of onMouseMove on the element.
  const chatWrapRef = React.useRef(null);
  const startResize = (side) => (e) => {
    const wrap = chatWrapRef.current;
    if (!wrap) return;
    e.preventDefault();
    const rect = wrap.getBoundingClientRect();
    const startX = e.clientX;
    const startWidth = parseInt(getComputedStyle(wrap).getPropertyValue(`--chat-${side}`)) || (side === "left" ? 280 : 340);
    document.body.style.cursor = "col-resize";
    const onMove = (ev) => {
      const delta = ev.clientX - startX;
      // Left handle grows on rightward drag; right handle shrinks on
      // rightward drag (the column lives to the right of the handle).
      const next = side === "left"
        ? Math.max(180, Math.min(560, startWidth + delta))
        : Math.max(220, Math.min(560, startWidth - delta));
      wrap.style.setProperty(`--chat-${side}`, `${next}px`);
    };
    const onUp = () => {
      document.body.style.cursor = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      try {
        localStorage.setItem(`rh.panel.chat${side === "left" ? "Left" : "Right"}`,
          wrap.style.getPropertyValue(`--chat-${side}`));
      } catch (_) {}
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };
  // Restore persisted widths on mount.
  React.useEffect(() => {
    const wrap = chatWrapRef.current;
    if (!wrap) return;
    try {
      const l = localStorage.getItem("rh.panel.chatLeft");
      const r = localStorage.getItem("rh.panel.chatRight");
      if (l) wrap.style.setProperty("--chat-left", l);
      if (r) wrap.style.setProperty("--chat-right", r);
    } catch (_) {}
  }, []);

  return (
    <div className="chat-wrap" ref={chatWrapRef}>
      {/* sidebar list */}
      <div className="chat-list">
        <div className="chat-list-head row between">
          <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>Sessions</span>
          <button className="icon-btn"><I.plus/></button>
        </div>
        <div className="chat-list-body">
          {agents.slice(0, 16).map(a => (
            <div key={a.id} className={"chat-list-item " + (active.id === a.id ? "active":"")} onClick={() => setActiveId(a.id)}>
              <Avatar agent={a}/>
              <div style={{flex:1, minWidth:0}}>
                <div className="row between"><span className="name">{a.name}</span><span className="time">{a.updated}</span></div>
                <div className="last">{a.last}</div>
              </div>
            </div>
          ))}
        </div>
      </div>

      <div className="chat-resize" onMouseDown={startResize("left")} aria-hidden/>

      {/* main chat */}
      <div className="chat-panel">
        <div className="chat-head">
          <Avatar agent={active}/>
          <div className="col" style={{gap:2}}>
            <div className="row gap-6">
              <b className="mono">{active.name}</b>
              <StateBadge state={active.state}/>
              <span className={"badge " + (ws.connected ? "live" : "idle")} title={ws.connected ? "WebSocket streaming" : "WebSocket disconnected (HTTP fallback)"}>
                {ws.connected ? "WS" : "HTTP"}
              </span>
            </div>
            <div className="dim mono" style={{fontSize:11}}>{active.model} · {active.provider} · session {(session && session.session_id) ? `#${String(session.session_id).slice(0,4)}` : "—"}</div>
          </div>
          <div className="actions">
            <button className="btn sm ghost" onClick={refreshSession} title="Refresh session"><I.refresh/></button>
            <button className="btn sm"
                    onClick={() => {
                      if (!ws.connected) { toastErr("WebSocket not connected"); return; }
                      ws.sendCommand("retry", "");
                      toast("Regenerating last response…");
                    }}
                    title="Regenerate the last assistant response"
                    disabled={!ws.connected || sending}>
              <I.refresh/> Regenerate
            </button>
            <button className="btn sm"><I.download/> Export</button>
            <button className="icon-btn"><I.more/></button>
          </div>
        </div>

        <div className="chat-stream" ref={streamRef}>
          {!session && !sessionErr && (
            <div className="dim mono" style={{padding:"24px 6px", fontSize:12}}>Loading session…</div>
          )}
          {sessionErr && pendingMessages.length === 0 && (
            <div className="dim mono" style={{padding:"24px 6px", fontSize:12, color:"var(--crimson)"}}>
              Session load failed: {sessionErr}
            </div>
          )}
          {items.map((it, i) => {
            if (it.role === "user") return (
              <div key={i} className="msg user">
                <Avatar agent={{ name: "you", hue: 22, emoji: "Y" }}/>
                <div>
                  <div className="who" style={{textAlign:"right"}}>operator · just now</div>
                  <div className="bubble" style={{whiteSpace:"pre-wrap"}}>{it.content}</div>
                </div>
              </div>
            );
            if (it.role === "tool") return <ToolTraceCard key={i} tool={it}/>;
            return (
              <div key={i} className="msg">
                <Avatar agent={active}/>
                <div>
                  <div className="who">{active.name}</div>
                  <div className="bubble" style={{color: it.error ? "var(--crimson)" : undefined}}>
                    {it.error || it.command ? it.content : renderMarkdown(it.content)}
                  </div>
                </div>
              </div>
            );
          })}
          {streamingTools.map((t, i) => <ToolTraceCard key={`stool-${i}`} tool={t}/>)}
          {(sending || streamingText) && (
            <div className="msg">
              <Avatar agent={active}/>
              <div>
                <div className="who">{active.name} · streaming</div>
                <div className="bubble">
                  {streamingText ? renderMarkdown(streamingText) : null}
                  <span className="cursor"/>
                </div>
              </div>
            </div>
          )}
        </div>

        <ChatInput
          typed={typed} setTyped={setTyped} sending={sending}
          send={send} active={active}
          ws={ws}
        />
      </div>

      <div className="chat-resize" onMouseDown={startResize("right")} aria-hidden/>

      {/* side: context */}
      <div className="chat-side">
        <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Run context</div>
        <div className="kv mb-16">
          <dt>session</dt><dd>{session ? String(session.session_id || "—").slice(0, 12) : "…"}</dd>
          <dt>messages</dt><dd>{session && session.messages ? session.messages.length : 0}</dd>
          <dt>pressure</dt><dd style={{color:"var(--live)"}}>{(session && session.context_pressure) || "—"}</dd>
          <dt>budget · day</dt><dd>{budget && budget.daily ? `$${Number(budget.daily.spend || 0).toFixed(2)} / $${Number(budget.daily.limit || 0).toFixed(2)}` : "—"}</dd>
          <dt>budget · hour</dt><dd>{budget && budget.hourly ? `$${Number(budget.hourly.spend || 0).toFixed(2)} / $${Number(budget.hourly.limit || 0).toFixed(2)}` : "—"}</dd>
        </div>

        <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Memory · session</div>
        <pre className="codebox mb-16" style={{maxHeight:120}}>
{`session_id = ${session ? (session.session_id || "—") : "loading…"}\nagent_id   = ${active.id}\nmessages   = ${session && session.messages ? session.messages.length : 0}\nmodel      = ${active.model}\nprovider   = ${active.provider}`}
        </pre>

        <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Allowed tools</div>
        <div className="col gap-4" style={{maxHeight:200, overflow:"auto"}}>
          {(tools || []).slice(0, 16).map(t => (
            <div key={t.name} className="row between" style={{padding:"4px 8px",background:"var(--bg-2)",borderRadius:5}}>
              <span className="mono" style={{fontSize:11.5}}>{t.name}</span>
              <span className="badge live" style={{padding:"1px 5px"}}>ok</span>
            </div>
          ))}
          {!tools && (
            <div className="dim mono" style={{fontSize:11, padding:"4px 8px"}}>loading…</div>
          )}
        </div>
      </div>
    </div>
  );
}

// Slash-command catalog the chat input surfaces when the user types `/`.
// Matches the commands the WS handler accepts on the server side
// (kernel ws::handle_client_message dispatches by `command` text).
// Adding a new command means adding a row here and ensuring the
// server handler recognizes it; the panel is otherwise a thin
// pass-through.
const CHAT_SLASH_COMMANDS = [
  { cmd: "/workflow run", help: "Run a workflow with the next args as input", usage: "/workflow run <name> [input…]" },
  { cmd: "/workflow list", help: "List all workflows", usage: "/workflow list" },
  { cmd: "/tool", help: "Show the agent's allowed tool list" },
  { cmd: "/memory recall", help: "Recall a memory by key or substring", usage: "/memory recall <key>" },
  { cmd: "/memory remember", help: "Store a memory under a key", usage: "/memory remember <key> <value…>" },
  { cmd: "/memory forget", help: "Drop a memory by key", usage: "/memory forget <key>" },
  { cmd: "/model", help: "Switch the agent's model", usage: "/model <model-id>" },
  { cmd: "/temp", help: "Set sampling temperature (0–2)", usage: "/temp 0.7" },
  { cmd: "/system", help: "Update the system prompt", usage: "/system <new prompt>" },
  { cmd: "/thinking", help: "Toggle extended thinking" },
  { cmd: "/reset", help: "Reset the conversation" },
  { cmd: "/help", help: "Show server-side command help" },
];

function ChatInput({ typed, setTyped, sending, send, active, ws }) {
  const inputRef = React.useRef(null);
  const open = typed.startsWith("/") && !typed.includes("\n");
  // Filter commands by the leading token of the typed text. Once the
  // user typed past the command (e.g. "/workflow run leadgen"), keep
  // the matching command pinned at the top with no other suggestions
  // so they can see which command they're invoking.
  const candidates = React.useMemo(() => {
    if (!open) return [];
    const ql = typed.toLowerCase();
    return CHAT_SLASH_COMMANDS
      .filter((c) => c.cmd.toLowerCase().startsWith(ql) || ql.startsWith(c.cmd.toLowerCase() + " "))
      .sort((a, b) => {
        // Exact-prefix matches first.
        const ap = ql.startsWith(a.cmd.toLowerCase()) ? 0 : 1;
        const bp = ql.startsWith(b.cmd.toLowerCase()) ? 0 : 1;
        return ap - bp;
      });
  }, [typed, open]);
  const [highlight, setHighlight] = React.useState(0);
  React.useEffect(() => { setHighlight(0); }, [candidates.length]);
  const pick = (c) => {
    // Pre-fill the input with the command + a trailing space if it has
    // arguments, so the user can keep typing.
    const next = c.usage ? c.cmd + " " : c.cmd;
    setTyped(next);
    setTimeout(() => inputRef.current && inputRef.current.focus(), 0);
  };
  const onKeyDown = (e) => {
    if (open && candidates.length > 0) {
      if (e.key === "ArrowDown") { e.preventDefault(); setHighlight(h => Math.min(candidates.length - 1, h + 1)); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); setHighlight(h => Math.max(0, h - 1)); return; }
      if (e.key === "Tab") { e.preventDefault(); pick(candidates[highlight]); return; }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };
  return (
    <div className="chat-input" style={{position:"relative"}}>
      {open && candidates.length > 0 && (
        <div className="slash-popup">
          <div className="slash-popup-head">
            <span className="mono dim" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
              Slash commands
            </span>
            <span className="dim mono" style={{fontSize:10.5}}>Tab / Enter to insert</span>
          </div>
          {candidates.map((c, i) => (
            <button key={c.cmd}
                    className={"slash-popup-row" + (i === highlight ? " active" : "")}
                    onMouseEnter={() => setHighlight(i)}
                    onMouseDown={(e) => e.preventDefault()}
                    onClick={() => pick(c)}>
              <span className="mono" style={{fontSize:12.5, color:"var(--rust)", width:170}}>{c.cmd}</span>
              <span className="dim" style={{fontSize:11.5, flex:1}}>{c.help}</span>
              {c.usage && <span className="dim mono" style={{fontSize:10.5}}>{c.usage}</span>}
            </button>
          ))}
        </div>
      )}
      <div className="field">
        <I.zap/>
        <input ref={inputRef}
               placeholder={`Message ${active.name}…  (type / for commands)`}
               value={typed} disabled={sending}
               onChange={e => setTyped(e.target.value)}
               onKeyDown={onKeyDown}/>
        <span className="kbd">↵ send</span>
      </div>
      <button className="btn primary" onClick={send} disabled={sending || !typed.trim()}><I.send/> {sending ? "…" : "Send"}</button>
    </div>
  );
}

// Flatten a session.messages payload into the items shape the chat
// stream renders. Each message becomes one user/assistant bubble, and
// each tool inside an assistant message becomes a tool-trace row in
// front of it. Tool results sit on `tool.result`/`tool.is_error`.
function sessionToItems(messages) {
  if (!Array.isArray(messages)) return [];
  const items = [];
  for (const m of messages) {
    const role = (m.role || "").toLowerCase();
    if (Array.isArray(m.tools)) {
      for (const t of m.tools) {
        items.push({ role: "tool", name: t.name, input: t.input, result: t.result, is_error: !!t.is_error, running: !!t.running });
      }
    }
    if (m.content) {
      items.push({ role: role === "user" ? "user" : "assistant", content: m.content });
    }
  }
  return items;
}

// ToolTraceCard — a single tool invocation in the chat stream. Click
// the row to expand input + result; collapsed by default to keep the
// stream readable. `tool.input` is whatever the agent passed (string
// or object); `tool.result` is the truncated response string the
// kernel returns. `tool.duration_ms` (if present) shows next to the
// status badge.
//
// We intentionally avoid an animated expand — the panel runs in
// dense ops contexts where reflow churn matters more than smoothness.
function ToolTraceCard({ tool }) {
  const [open, setOpen] = useState(false);
  const t = tool || {};
  const running = !!t.running;
  const isError = !!t.is_error;
  const hasDetail = !!t.input || !!t.result;
  const argStr = String(typeof t.input === "string" ? t.input : (t.input ? JSON.stringify(t.input) : "")).slice(0, 240);
  const resultStr = t.result ? String(t.result).slice(0, 2000) : "";
  const elapsed = t.elapsed || (t.duration_ms != null ? `${(Number(t.duration_ms) / 1000).toFixed(2)}s` : null);
  const status = running ? "…" : (isError ? "err" : (elapsed || "ok"));
  return (
    <div className={"tool-trace " + (running ? "" : (isError ? "fail" : "done"))}>
      <button
        className="tool-trace-head"
        onClick={() => hasDetail && setOpen(o => !o)}
        disabled={!hasDetail}
        title={hasDetail ? "Show full input + result" : "No detail captured"}>
        <span style={{display:"inline-flex",width:14,height:14}}>{running ? <Spinner/> : <I.check/>}</span>
        <span className="tool-trace-label">
          {running ? "⚙" : (isError ? "✗" : "✓")} <span className="tool-name">{t.name}</span>
          {argStr && <span className="dim"> ({argStr.length > 80 ? argStr.slice(0, 80) + "…" : argStr})</span>}
          {(t.count && t.count > 1) ? <span className="badge plain" style={{marginLeft:6}}>×{t.count}</span> : null}
        </span>
        <span className="elapsed">{status}</span>
        {hasDetail && <span className="tool-trace-caret">{open ? "−" : "+"}</span>}
      </button>
      {open && hasDetail && (
        <div className="tool-trace-detail">
          {t.input && (
            <>
              <div className="tool-trace-detail-label">Input</div>
              <pre className="codebox" style={{maxHeight:140, marginBottom:6}}>{typeof t.input === "string" ? t.input : JSON.stringify(t.input, null, 2)}</pre>
            </>
          )}
          {t.result && (
            <>
              <div className="tool-trace-detail-label">{isError ? "Error" : "Result"}</div>
              <pre className="codebox" style={{maxHeight:240, color: isError ? "var(--crimson)" : undefined}}>{resultStr}</pre>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// Coalesce consecutive tool calls with the same name into a single card
// with a count badge. Repeated probes (e.g. polling a queue) clutter
// the stream otherwise; the operator can still expand to see each call
// since we keep the latest input/result attached. Items not in `tool`
// role pass through unchanged.
function coalesceToolTraces(items) {
  const out = [];
  for (const it of items) {
    const last = out[out.length - 1];
    if (it && it.role === "tool" && last && last.role === "tool" && last.name === it.name && !last.running && !it.running && !last.is_error && !it.is_error) {
      last.count = (last.count || 1) + 1;
      // Keep the latest input/result so expanding shows the most recent
      // sample of that group. Older samples are dropped — acceptable
      // for the chat-stream use case.
      last.input = it.input;
      last.result = it.result;
      continue;
    }
    out.push(it && it.role === "tool" ? { ...it, count: 1 } : it);
  }
  return out;
}

const Spinner = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none">
    <circle cx="12" cy="12" r="9" stroke="currentColor" strokeWidth="2.4" opacity=".2"/>
    <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round">
      <animateTransform attributeName="transform" type="rotate" from="0 12 12" to="360 12 12" dur="0.9s" repeatCount="indefinite"/>
    </path>
  </svg>
);

/* ============================== WORKFLOWS ============================== */
//
// /api/workflows returns a bare array of workflow definitions. Each item
// carries id, name, steps (an array, not a count), and metadata. The
// `/api/workflows/runs?workflow_id=…` endpoint returns recent runs.
//
// Pre-API state was a list of synthetic pipelines; we now show the kernel's
// real registry. The DAG renders the actual step list as a horizontal
// pipeline (no fancy graph layout yet — that's a follow-up).
function WorkflowsPage() {
  const [wfList, , refreshList] = usePolling("/api/workflows", 15000);
  const workflows = Array.isArray(wfList) ? wfList : (wfList && wfList.workflows) || [];
  const [activeId, setActiveId] = useState(null);
  const active = workflows.find(w => w.id === activeId) || workflows[0];
  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showRunInput, setShowRunInput] = useState(false);
  const [inspectingRun, setInspectingRun] = useState(null);
  const [showEditYaml, setShowEditYaml] = useState(false);

  React.useEffect(() => {
    const onNew = (e) => { if (e.detail && e.detail.page === "workflows") setShowCreate(true); };
    const onRefresh = (e) => { if (e.detail && e.detail.page === "workflows") refreshList(); };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [refreshList]);

  // The real path is GET /api/workflows/{id}/runs (path param, NOT a
  // ?workflow_id= query — the handler reads it as Path<String>).
  // Poll faster when we're awaiting a freshly-kicked run so a long-
  // running workflow updates its step count visibly; slower at rest.
  const [liveRun, setLiveRun] = useState(false);
  const [runsResp, , refreshRuns] = usePolling(
    active ? `/api/workflows/${encodeURIComponent(active.id)}/runs` : null,
    liveRun ? 1500 : 8000,
  );
  const runs = Array.isArray(runsResp) ? runsResp : (runsResp && runsResp.runs) || [];

  const runWith = async (input) => {
    if (!active) return;
    setShowRunInput(false);
    setLiveRun(true);
    const startedAt = Date.now();
    try {
      const r = await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}/run`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input }),
      });
      const dur = ((Date.now() - startedAt) / 1000).toFixed(2);
      toastOk(`Run completed in ${dur}s`);
      // Refresh once, then open the inspector on the new run so the
      // operator sees per-step output without scrolling the list.
      await refreshRuns();
      if (r && r.run_id) {
        setInspectingRun({ run_id: r.run_id, id: r.run_id, output: r.output });
      }
    } catch (e) {
      toastErr(`run failed: ${e.message || e}`);
    } finally {
      setLiveRun(false);
    }
  };
  const runNow = () => setShowRunInput(true);

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Workflows <span className="dim mono" style={{fontSize:14}}>· {workflows.length}</span></h1>
          <p className="page-sub">Pipeline definitions persist across daemon restart · live from <span className="mono">/api/workflows</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refreshList}><I.refresh/></button>
          <button className="btn ghost" onClick={() => setShowImport(true)}><I.copy/> Import YAML</button>
          <button className="btn primary" onClick={() => setShowCreate(true)}><I.plus/> New workflow</button>
        </div>
      </div>

      <div className="grid-12">
        <div className="col-4 col">
          <div className="card flush">
            <div className="card-head"><span>All workflows</span><span className="mono">{workflows.length}</span></div>
            <div>
              {!wfList && Array.from({length:3}).map((_,i) => (
                <div key={`s-${i}`} style={{padding:"10px 14px", borderBottom:"1px solid var(--border)"}}>
                  <Skel w="50%" h={10}/><div style={{marginTop:6}}><Skel w="30%" h={9}/></div>
                </div>
              ))}
              {wfList && workflows.length === 0 && (
                <div style={{padding:"24px 14px", textAlign:"center"}}>
                  <div className="dim" style={{fontSize:12, marginBottom:8}}>No workflows yet.</div>
                  <button className="btn primary sm" onClick={() => setShowCreate(true)}><I.plus/> Create your first workflow</button>
                </div>
              )}
              {workflows.map(w => {
                const stepCount = Array.isArray(w.steps) ? w.steps.length : (w.steps || 0);
                return (
                  <div key={w.id} onClick={() => setActiveId(w.id)}
                       className="row between"
                       style={{padding:"10px 14px",borderBottom:"1px solid var(--border)",cursor:"pointer",
                               background: active && active.id===w.id ? "var(--surface-2)" : "transparent"}}>
                    <div>
                      <div className="mono" style={{fontSize:12.5}}>{w.name || w.id}</div>
                      <div className="dim mono" style={{fontSize:10.5}}>{stepCount} steps{w.kind ? ` · ${w.kind}` : ""}</div>
                    </div>
                    <div className="col" style={{alignItems:"flex-end",gap:2}}>
                      <span className="mono nums" style={{fontSize:12}}>{(w.runs_24h != null) ? w.runs_24h : "—"}</span>
                      <span className="dim mono" style={{fontSize:10.5}}>runs · 24h</span>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        </div>

        <div className="col-8 col">
          {active && (
            <div className="card">
              <div className="row between mb-12">
                <div>
                  <div className="page-title" style={{fontSize:15, marginBottom:2}}>{active.name || active.id}</div>
                  <div className="dim mono" style={{fontSize:11}}>id={active.id}{active.description ? ` · ${active.description}` : ""}</div>
                </div>
                <div className="row gap-6">
                  {liveRun && (
                    <span className="badge live" title="A run is in flight — list polls at 1.5s">
                      <span className="dot live"/>running
                    </span>
                  )}
                  <button className="btn sm" onClick={refreshRuns}><I.refresh/></button>
                  <button className="btn sm" onClick={() => exportWorkflowYaml(active)} title="Download workflow as YAML"><I.download/> YAML</button>
                  <button className="btn sm" onClick={() => setShowEditYaml(true)} title="Edit workflow YAML inline (delete + recreate)"><I.copy/> Edit YAML</button>
                  <button className="btn sm ghost"
                          title="Delete this workflow (run history is preserved)"
                          onClick={async () => {
                            const ok = await confirmDialog({
                              title: "Delete workflow?",
                              message: `Remove '${active.name || active.id}'? Past run history is kept, but the workflow definition will be gone.`,
                              danger: true,
                              confirmLabel: "Delete",
                            });
                            if (!ok) return;
                            try {
                              await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}`, { method: "DELETE" });
                              toastOk(`Deleted ${active.name || active.id}`);
                              setActiveId(null);
                              refreshList();
                            } catch (err) { toastErr(`delete failed: ${err.message || err}`); }
                          }}
                          style={{color:"var(--crimson)"}}><I.trash/></button>
                  <button className="btn sm primary" onClick={runNow} disabled={liveRun}>
                    <I.play/> {liveRun ? "Running…" : "Run now"}
                  </button>
                </div>
              </div>
              <WorkflowDAG workflow={active}/>
              <div className="row gap-12 mt-16">
                <Stat label="Steps" value={Array.isArray(active.steps) ? active.steps.length : (active.steps || 0)}/>
                <Stat label="Runs (recent)" value={runs.length}/>
                <Stat label="p50" value={active.p50_ms ? `${active.p50_ms}ms` : "—"}/>
                <Stat label="OK rate" value={active.ok != null ? `${active.ok}%` : "—"}/>
                <Stat label="Updated" value={relativeTime(active.updated_at)}/>
              </div>
            </div>
          )}

          <div className="card flush">
            <div className="card-head"><span>Recent runs</span></div>
            <table className="tbl">
              <thead><tr>
                <th>Run</th><th>Trigger</th><th>Started</th><th>Duration</th><th>Status</th><th className="right">Tokens</th>
              </tr></thead>
              <tbody>
                {runs.length === 0 && (
                  <tr><td colSpan={6} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>
                    {!active ? "Select a workflow." : "No runs yet."}
                  </td></tr>
                )}
                {runs.map(r => {
                  const id = r.id || r.run_id || "—";
                  const trig = r.trigger || r.source || "manual";
                  const t = formatTime(r.started_at || r.created_at);
                  // Compute duration from started/completed timestamps if
                  // duration_ms wasn't sent (older API). Sum step times
                  // as a last resort.
                  let dur = "—";
                  if (r.duration_ms != null) dur = `${(r.duration_ms / 1000).toFixed(2)}s`;
                  else if (r.started_at && r.completed_at) {
                    const ms = Date.parse(r.completed_at) - Date.parse(r.started_at);
                    if (!Number.isNaN(ms) && ms >= 0) dur = `${(ms / 1000).toFixed(2)}s`;
                  } else if (Array.isArray(r.step_results)) {
                    const ms = r.step_results.reduce((s, x) => s + (x.duration_ms || 0), 0);
                    if (ms) dur = `${(ms / 1000).toFixed(2)}s`;
                  }
                  const st = (typeof r.state === "string" ? r.state : (r.status || r.outcome || "ok")).toLowerCase();
                  // Sum tokens across steps when present.
                  let tok = r.total_tokens || r.tokens;
                  if (tok == null && Array.isArray(r.step_results)) {
                    tok = r.step_results.reduce((s, x) => s + (x.input_tokens || 0) + (x.output_tokens || 0), 0);
                  }
                  return (
                    <tr key={id} style={{cursor:"pointer"}} onClick={() => setInspectingRun(r)} title="Click to inspect step-by-step output">
                      <td className="mono">{String(id).slice(0, 8)}</td>
                      <td><span className="badge plain">{trig}</span></td>
                      <td className="mono muted">{t}</td>
                      <td className="mono">{dur}</td>
                      <td>{st === "ok" || st === "success" || st === "completed"
                        ? <span className="badge live">{st}</span>
                        : st === "failed" || st === "error"
                        ? <span className="badge error">{st}</span>
                        : <span className="badge warn">{st}</span>}</td>
                      <td className="num mono">{tok != null ? tok.toLocaleString() : "—"}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {showCreate && <WorkflowCreateModal onClose={() => setShowCreate(false)} onCreated={(id) => { setShowCreate(false); setActiveId(id); refreshList(); }}/>}
      {showImport && <WorkflowImportModal onClose={() => setShowImport(false)} onImported={(id) => { setShowImport(false); if (id) setActiveId(id); refreshList(); }}/>}
      {showRunInput && active && <WorkflowRunModal workflow={active} onClose={() => setShowRunInput(false)} onRun={runWith}/>}
      {inspectingRun && <WorkflowRunInspector run={inspectingRun} onClose={() => setInspectingRun(null)}/>}
      {showEditYaml && active && (
        <WorkflowEditYamlModal
          workflow={active}
          onClose={() => setShowEditYaml(false)}
          onSaved={(newId) => { setShowEditYaml(false); if (newId) setActiveId(newId); refreshList(); }}
        />
      )}
    </div>
  );
}

// Inline YAML editor for a workflow. The API doesn't expose a PUT/PATCH
// for workflows, so "save" is implemented as DELETE + import-yaml: the
// run history of the deleted definition is preserved (it lives in a
// separate run-id map on the kernel), but the workflow gets a new id.
// We surface that explicitly to the operator.
function WorkflowEditYamlModal({ workflow, onClose, onSaved }) {
  useEscapeKey(onClose);
  // Build the same JSON shape exportWorkflowYaml does, then YAML-ify it
  // for the editor. We reuse the helper's logic by calling toYaml().
  const initialYaml = React.useMemo(() => {
    if (!workflow) return "";
    const out = {
      name: workflow.name,
      description: workflow.description || "",
      steps: Array.isArray(workflow.steps) ? workflow.steps.map((s) => {
        const step = { name: s.name };
        if (s.agent && s.agent.id) step.agent_id = s.agent.id;
        else if (s.agent && s.agent.name) step.agent_name = s.agent.name;
        else if (s.agent_id) step.agent_id = s.agent_id;
        else if (s.agent_name) step.agent_name = s.agent_name;
        step.prompt = s.prompt_template || s.prompt || "{{input}}";
        if (s.mode) {
          if (typeof s.mode === "string") step.mode = s.mode;
          else if (s.mode.Conditional) { step.mode = "conditional"; step.condition = s.mode.Conditional.condition; }
          else if (s.mode.Loop) { step.mode = "loop"; step.max_iterations = s.mode.Loop.max_iterations; step.until = s.mode.Loop.until; }
          else step.mode = "sequential";
        }
        if (s.timeout_secs && s.timeout_secs !== 120) step.timeout_secs = s.timeout_secs;
        if (s.error_mode) {
          if (typeof s.error_mode === "string" && s.error_mode !== "fail") step.error_mode = s.error_mode;
          else if (s.error_mode && s.error_mode.Retry) { step.error_mode = "retry"; step.max_retries = s.error_mode.Retry.max_retries; }
        }
        if (s.output_var) step.output_var = s.output_var;
        return step;
      }) : [],
    };
    return toYaml(out);
  }, [workflow]);
  const [yamlText, setYamlText] = useState(initialYaml);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  const save = async () => {
    if (!yamlText.trim()) { setErr("YAML cannot be empty"); return; }
    setBusy(true); setErr(null);
    try {
      // Import first — if the YAML is bad, we haven't deleted yet, so
      // the user can fix and retry without losing the existing workflow.
      const importResp = await rhFetch("/api/workflows/import-yaml", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml: yamlText }),
      });
      const newId = importResp && (importResp.id || importResp.workflow_id);
      // Then delete the old one. If delete fails, both exist for a
      // moment — toast warns the user so they can clean up.
      try {
        await rhFetch(`/api/workflows/${encodeURIComponent(workflow.id)}`, { method: "DELETE" });
      } catch (deleteErr) {
        toastErr(`Saved as new workflow (id=${String(newId).slice(0,8)}) but failed to delete the old one: ${deleteErr.message || deleteErr}`);
        onSaved(newId);
        return;
      }
      toastOk(`Workflow saved — new id ${String(newId).slice(0, 8)} (old definition deleted, run history kept)`);
      onSaved(newId);
    } catch (e) {
      setErr(String(e.message || e));
    } finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <b className="mono">Edit workflow YAML</b>
            <div className="dim mono" style={{fontSize:11, marginTop:2}}>{workflow.name || workflow.id}</div>
          </div>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="banner mb-12" style={{borderColor:"oklch(0.78 0.14 88 / .35)"}}>
            <span className="dot warn"/>
            <span className="banner-title">SAVE = DELETE + RECREATE</span>
            <span className="banner-body" style={{fontSize:11}}>
              The API doesn't expose a workflow update endpoint. Saving will create a new
              workflow from this YAML and then delete the current one. The new workflow
              gets a fresh id; past run history stays accessible by run-id.
            </span>
          </div>
          <textarea
            className="modal-field modal-textarea mono"
            style={{minHeight:340, fontFamily:"var(--ff-mono)", fontSize:12, lineHeight:1.5}}
            value={yamlText}
            onChange={e => setYamlText(e.target.value)}
            spellCheck={false}
          />
          {err && (
            <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
              <span className="dot err"/><span className="banner-title">ERROR</span>
              <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
            </div>
          )}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn ghost" onClick={() => setYamlText(initialYaml)} disabled={busy}>Reset</button>
          <button className="btn primary" onClick={save} disabled={busy}>{busy ? "Saving…" : "Save"}</button>
        </div>
      </div>
    </div>
  );
}

// Client-side YAML serializer for the workflow subset. Handles strings
// (auto-quoted when they contain whitespace, colons, or YAML-reserved
// characters), numbers, booleans, null, plain objects, and arrays.
// Does NOT try to be a full YAML implementation — it's only ever called
// on workflow definitions which have a known shape, so anchors,
// references, and flow-style collections are out of scope.
function toYaml(value, indent) {
  const ind = indent || 0;
  const pad = "  ".repeat(ind);
  if (value === null || value === undefined) return "null";
  if (typeof value === "boolean" || typeof value === "number") return String(value);
  if (typeof value === "string") {
    // Quote if the string contains characters that would confuse a
    // YAML parser, otherwise emit as a plain scalar.
    if (/^\s|\s$|[:#\-?,&*!|>'"%@`]|^(true|false|null|yes|no)$|^-?\d/i.test(value) || value === "") {
      return JSON.stringify(value);
    }
    return value;
  }
  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";
    return value.map((item) => {
      const rendered = toYaml(item, ind + 1);
      if (typeof item === "object" && item !== null && !Array.isArray(item)) {
        // First field inline with the dash, the rest indented.
        const lines = rendered.split("\n");
        return `${pad}- ${lines[0].trimStart()}\n${lines.slice(1).join("\n")}`;
      }
      return `${pad}- ${rendered}`;
    }).join("\n");
  }
  if (typeof value === "object") {
    const keys = Object.keys(value);
    if (keys.length === 0) return "{}";
    return keys.map((k) => {
      const v = value[k];
      if (v == null || typeof v === "boolean" || typeof v === "number" || typeof v === "string") {
        return `${pad}${k}: ${toYaml(v, ind)}`;
      }
      if (Array.isArray(v) && v.length === 0) return `${pad}${k}: []`;
      if (typeof v === "object" && Object.keys(v).length === 0) return `${pad}${k}: {}`;
      return `${pad}${k}:\n${toYaml(v, ind + 1)}`;
    }).join("\n");
  }
  return JSON.stringify(value);
}

function exportWorkflowYaml(workflow) {
  if (!workflow) return;
  // Re-shape to the same JSON keys the import endpoint accepts so the
  // round-trip is symmetric (export → import yields an identical
  // workflow with a new id).
  const out = {
    name: workflow.name,
    description: workflow.description || "",
    steps: Array.isArray(workflow.steps) ? workflow.steps.map((s) => {
      const step = { name: s.name };
      if (s.agent && s.agent.id) step.agent_id = s.agent.id;
      else if (s.agent && s.agent.name) step.agent_name = s.agent.name;
      else if (s.agent_id) step.agent_id = s.agent_id;
      else if (s.agent_name) step.agent_name = s.agent_name;
      step.prompt = s.prompt_template || s.prompt || "{{input}}";
      if (s.mode) {
        if (typeof s.mode === "string") step.mode = s.mode;
        else if (s.mode.Conditional) { step.mode = "conditional"; step.condition = s.mode.Conditional.condition; }
        else if (s.mode.Loop) { step.mode = "loop"; step.max_iterations = s.mode.Loop.max_iterations; step.until = s.mode.Loop.until; }
        else step.mode = "sequential";
      }
      if (s.timeout_secs && s.timeout_secs !== 120) step.timeout_secs = s.timeout_secs;
      if (s.error_mode) {
        if (typeof s.error_mode === "string" && s.error_mode !== "fail") step.error_mode = s.error_mode;
        else if (s.error_mode && s.error_mode.Retry) { step.error_mode = "retry"; step.max_retries = s.error_mode.Retry.max_retries; }
      }
      if (s.output_var) step.output_var = s.output_var;
      return step;
    }) : [],
  };
  const yaml = toYaml(out);
  const fname = `workflow-${(workflow.name || workflow.id || "export").replace(/[^a-zA-Z0-9_-]/g, "_")}.yaml`;
  downloadBlob(fname, yaml, "application/yaml");
  toastOk(`Exported ${fname}`);
}

function WorkflowImportModal({ onClose, onImported }) {
  useEscapeKey(onClose);
  const [yamlText, setYamlText] = useState(`name: leadgen-funnel
description: Sample pipeline imported from YAML
steps:
  - name: research
    agent_name: rusty
    prompt: "Find candidates matching {{input}}"
    mode: sequential
  - name: rank
    agent_name: rusty
    prompt: "Rank these: {{input}}"
    mode: sequential
`);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  const submit = async () => {
    if (!yamlText.trim()) { setErr("YAML is empty"); return; }
    setBusy(true); setErr(null);
    try {
      const r = await rhFetch("/api/workflows/import-yaml", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml: yamlText }),
      });
      toastOk("Workflow imported");
      onImported(r && (r.workflow_id || r.id));
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Import workflow from YAML</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="dim" style={{fontSize:12, marginBottom:8}}>
            Paste a YAML workflow definition. Same schema as JSON create:
            <span className="mono"> name, description, steps:[{`{name, agent_id|agent_name, prompt, mode, ...}`}]</span>
          </div>
          <textarea
            className="modal-field modal-textarea"
            style={{minHeight:320, fontFamily:"var(--ff-mono)", fontSize:12}}
            value={yamlText}
            onChange={(e) => setYamlText(e.target.value)}
            autoFocus/>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Importing…" : "Import"}</button>
        </div>
      </div>
    </div>
  );
}

function WorkflowRunInspector({ run, onClose }) {
  useEscapeKey(onClose);
  const steps = Array.isArray(run.step_results) ? run.step_results : [];
  const totalDur = steps.reduce((s, x) => s + (x.duration_ms || 0), 0);
  const totalTokens = steps.reduce((s, x) => s + (x.input_tokens || 0) + (x.output_tokens || 0), 0);
  const state = typeof run.state === "string" ? run.state : "—";
  const isFailed = state.toLowerCase() === "failed" || !!run.error;
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <b className="mono">Run · {String(run.id || "").slice(0, 12)}</b>
            <div className="dim mono" style={{fontSize:11, marginTop:2}}>
              {run.workflow_name || run.workflow_id} · started {formatTime(run.started_at)}
              {run.completed_at ? ` · completed ${formatTime(run.completed_at)}` : " · running…"}
            </div>
          </div>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="row gap-12 mb-12">
            <Stat label="State" value={state}/>
            <Stat label="Steps" value={`${steps.length}`}/>
            <Stat label="Duration" value={totalDur ? `${(totalDur / 1000).toFixed(2)}s` : "—"}/>
            <Stat label="Tokens" value={totalTokens.toLocaleString()}/>
          </div>
          {run.input && (
            <>
              <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Initial input</div>
              <pre className="codebox mb-16" style={{maxHeight:100, whiteSpace:"pre-wrap"}}>{run.input}</pre>
            </>
          )}
          {isFailed && run.error && (
            <>
              <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", color:"var(--crimson)"}}>Error</div>
              <pre className="codebox mb-16" style={{color:"var(--crimson)", maxHeight:120}}>{run.error}</pre>
            </>
          )}
          <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Steps ({steps.length})</div>
          <div className="col gap-6" style={{maxHeight:420, overflow:"auto"}}>
            {steps.length === 0 && <div className="dim mono" style={{fontSize:11, padding:"6px 8px"}}>No step output recorded yet.</div>}
            {steps.map((s, i) => {
              const tokens = (s.input_tokens || 0) + (s.output_tokens || 0);
              return (
                <RunStepCard key={i} index={i} step={s} tokens={tokens}/>
              );
            })}
          </div>
          {run.output && (
            <>
              <div className="muted mono mb-8 mt-16" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Final output</div>
              <pre className="codebox" style={{maxHeight:160, whiteSpace:"pre-wrap"}}>{run.output}</pre>
            </>
          )}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

// Single step card inside the run inspector. Output is collapsed to the
// first 200 chars; click expands to show the full (server-truncated to
// 2000 chars) output. Failed steps borrowed the chat tool-trace `fail`
// styling for consistency.
function RunStepCard({ index, step, tokens }) {
  const [open, setOpen] = useState(false);
  const preview = (step.output || "").slice(0, 200);
  const dur = step.duration_ms != null ? `${(step.duration_ms / 1000).toFixed(2)}s` : "—";
  return (
    <div style={{border:"1px solid var(--border)", borderRadius:7, background:"var(--bg-2)", overflow:"hidden"}}>
      <button onClick={() => setOpen(o => !o)}
              className="row gap-8"
              style={{width:"100%", padding:"8px 10px", background:"none", color:"inherit", textAlign:"left", cursor:"pointer"}}>
        <span className="badge plain" style={{minWidth:32, textAlign:"center"}}>#{index + 1}</span>
        <span className="mono" style={{fontSize:12.5}}>{step.step_name || "step"}</span>
        <span className="dim mono" style={{fontSize:11}}>{step.agent_name || step.agent_id}</span>
        <span style={{marginLeft:"auto"}} className="row gap-12">
          <span className="dim mono" style={{fontSize:11}}>{tokens.toLocaleString()} tok</span>
          <span className="dim mono" style={{fontSize:11}}>{dur}</span>
          <span className="dim" style={{width:12, textAlign:"center"}}>{open ? "−" : "+"}</span>
        </span>
      </button>
      {!open && preview && (
        <div className="dim" style={{padding:"0 12px 8px 50px", fontSize:11.5, whiteSpace:"nowrap", overflow:"hidden", textOverflow:"ellipsis"}}>{preview}</div>
      )}
      {open && (
        <div style={{padding:"4px 12px 10px 12px"}}>
          <pre className="codebox" style={{maxHeight:240, whiteSpace:"pre-wrap"}}>{step.output || "(empty)"}</pre>
        </div>
      )}
    </div>
  );
}

// WorkflowCreateModal — visual step builder. Each step is an editable
// card; reorder via ↑/↓ buttons (HTML5 drag-and-drop would be richer but
// adds 80 LOC of cross-browser DnD handling for marginal UX gain).
//
// The server accepts a JSON envelope `{name, description, steps:[...]}`
// where each step has `{name, agent_id|agent_name, prompt, mode, timeout_secs,
// error_mode, output_var, condition?, max_iterations?, until?, max_retries?}`.
// We carry an internal `steps` array of those records and serialize on submit.
function WorkflowCreateModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const agents = (agentsResp && agentsResp.agents) || [];
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [steps, setSteps] = useState([
    { name: "step-1", agent_id: "", prompt: "{{input}}", mode: "sequential",
      timeout_secs: 120, error_mode: "fail", output_var: "" }
  ]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  // Seed the first step's agent once the list lands.
  React.useEffect(() => {
    if (agents.length && steps.length === 1 && !steps[0].agent_id) {
      setSteps([{ ...steps[0], agent_id: agents[0].id }]);
    }
  }, [agents.length]);

  const updateStep = (i, patch) => setSteps(prev => prev.map((s, j) => j === i ? { ...s, ...patch } : s));
  const removeStep = (i) => setSteps(prev => prev.filter((_, j) => j !== i));
  const moveStep = (i, delta) => {
    setSteps(prev => {
      const j = i + delta;
      if (j < 0 || j >= prev.length) return prev;
      const out = prev.slice();
      [out[i], out[j]] = [out[j], out[i]];
      return out;
    });
  };
  const duplicateStep = (i) => setSteps(prev => {
    const copy = { ...prev[i], name: `${prev[i].name}-copy` };
    return prev.slice(0, i + 1).concat([copy], prev.slice(i + 1));
  });
  // Drag-and-drop reorder: `from` is the dragged step's index, `to` is the
  // drop target's index. Moves the dragged step to the drop position and
  // shifts everything else. No-op if `from === to`.
  const reorderStep = (from, to) => setSteps(prev => {
    if (from === to || from < 0 || to < 0 || from >= prev.length || to >= prev.length) return prev;
    const out = prev.slice();
    const [moved] = out.splice(from, 1);
    out.splice(to, 0, moved);
    return out;
  });
  // Track which card is being dragged + which one is being hovered, so we
  // can render the drop indicator. State lives in the parent so cards can
  // coordinate without prop drilling between siblings.
  const [dragSrc, setDragSrc] = useState(null);
  const [dragOver, setDragOver] = useState(null);
  const addStep = () => setSteps(prev => prev.concat([{
    name: `step-${prev.length + 1}`,
    agent_id: agents[0] ? agents[0].id : "",
    prompt: "{{input}}",
    mode: "sequential",
    timeout_secs: 120,
    error_mode: "fail",
    output_var: "",
  }]));

  const submit = async () => {
    if (!name.trim()) { setErr("name required"); return; }
    if (steps.length === 0) { setErr("at least one step required"); return; }
    for (const s of steps) {
      if (!s.agent_id && !s.agent_name) {
        setErr(`step '${s.name}' is missing agent_id / agent_name`);
        return;
      }
    }
    // Strip empty optional fields so the server's serde defaults kick in.
    const payload = steps.map(s => {
      const out = { name: s.name, prompt: s.prompt, mode: s.mode };
      if (s.agent_id) out.agent_id = s.agent_id;
      else if (s.agent_name) out.agent_name = s.agent_name;
      if (s.timeout_secs && s.timeout_secs !== 120) out.timeout_secs = Number(s.timeout_secs);
      if (s.error_mode && s.error_mode !== "fail") out.error_mode = s.error_mode;
      if (s.error_mode === "retry") out.max_retries = Number(s.max_retries || 3);
      if (s.mode === "conditional") out.condition = s.condition || "";
      if (s.mode === "loop") {
        out.max_iterations = Number(s.max_iterations || 5);
        out.until = s.until || "";
      }
      if (s.output_var) out.output_var = s.output_var;
      return out;
    });
    setBusy(true); setErr(null);
    try {
      const r = await rhFetch("/api/workflows", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, description, steps: payload }),
      });
      onCreated(r.id || r.workflow_id);
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">New workflow</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="col gap-8">
            <div className="row gap-12">
              <label className="t-row col" style={{flex:1}}><span className="t-lbl">Name</span>
                <input className="modal-field" value={name} onChange={e => setName(e.target.value)} placeholder="my-pipeline"/></label>
              <label className="t-row col" style={{flex:2}}><span className="t-lbl">Description</span>
                <input className="modal-field" value={description} onChange={e => setDescription(e.target.value)}/></label>
            </div>
            <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", marginTop:6}}>
              Steps · {steps.length}
            </span>
            <div className="col gap-6">
              {steps.map((s, i) => (
                <WorkflowStepCard
                  key={i}
                  index={i}
                  total={steps.length}
                  step={s}
                  agents={agents}
                  isDragSrc={dragSrc === i}
                  isDragOver={dragOver === i && dragSrc !== null && dragSrc !== i}
                  onDragStart={() => setDragSrc(i)}
                  onDragOver={() => setDragOver(i)}
                  onDrop={() => { if (dragSrc !== null) reorderStep(dragSrc, i); setDragSrc(null); setDragOver(null); }}
                  onDragEnd={() => { setDragSrc(null); setDragOver(null); }}
                  onChange={(patch) => updateStep(i, patch)}
                  onRemove={() => removeStep(i)}
                  onMove={(delta) => moveStep(i, delta)}
                  onDuplicate={() => duplicateStep(i)}
                />
              ))}
              <button className="btn ghost" onClick={addStep} style={{alignSelf:"flex-start"}}>
                <I.plus/> Add step
              </button>
            </div>
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Creating…" : "Create"}</button>
        </div>
      </div>
    </div>
  );
}

function WorkflowStepCard({
  index, total, step, agents,
  isDragSrc, isDragOver,
  onDragStart, onDragOver, onDrop, onDragEnd,
  onChange, onRemove, onMove, onDuplicate,
}) {
  const [advanced, setAdvanced] = useState(false);
  // HTML5 DnD wired at the card root: `draggable` is set true only when
  // the user grabs the ☰ handle (mousedown sets a ref flag); without
  // that, text/buttons inside the card stay clickable without spuriously
  // initiating drags.
  const dragHandleArmed = React.useRef(false);
  return (
    <div
      draggable
      onDragStart={(e) => {
        if (!dragHandleArmed.current) { e.preventDefault(); return; }
        e.dataTransfer.effectAllowed = "move";
        try { e.dataTransfer.setData("text/plain", String(index)); } catch (_) {}
        onDragStart && onDragStart();
      }}
      onDragOver={(e) => {
        // preventDefault is required to allow dropping.
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        onDragOver && onDragOver();
      }}
      onDrop={(e) => {
        e.preventDefault();
        dragHandleArmed.current = false;
        onDrop && onDrop();
      }}
      onDragEnd={() => { dragHandleArmed.current = false; onDragEnd && onDragEnd(); }}
      style={{
        border: "1px solid " + (isDragOver ? "var(--rust)" : "var(--border)"),
        boxShadow: isDragOver ? "0 0 0 1px var(--rust) inset" : undefined,
        borderRadius: 8,
        background: "var(--bg-2)",
        padding: "10px 12px",
        opacity: isDragSrc ? 0.55 : 1,
        transition: "opacity .12s, border-color .12s, box-shadow .12s",
      }}>
      <div className="row between mb-8">
        <div className="row gap-8">
          <span
            className="dnd-handle"
            title="Drag to reorder"
            onMouseDown={() => { dragHandleArmed.current = true; }}
            onMouseUp={() => { dragHandleArmed.current = false; }}
          >☰</span>
          <span className="badge plain" style={{minWidth:32, textAlign:"center"}}>#{index + 1}</span>
          <span className="mono" style={{fontSize:12.5}}>{step.name || "(unnamed)"}</span>
          {step.mode !== "sequential" && <span className="badge violet">{step.mode}</span>}
        </div>
        <div className="row gap-4">
          <button className="btn sm ghost" onClick={() => onMove(-1)} disabled={index === 0} title="Move up">↑</button>
          <button className="btn sm ghost" onClick={() => onMove(1)} disabled={index === total - 1} title="Move down">↓</button>
          <button className="btn sm ghost" onClick={onDuplicate} title="Duplicate"><I.copy/></button>
          <button className="btn sm ghost" onClick={onRemove} title="Remove" style={{color:"var(--crimson)"}} disabled={total === 1}><I.close/></button>
        </div>
      </div>
      <div className="row gap-12 mb-8">
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Step name</span>
          <input className="modal-field" value={step.name} onChange={e => onChange({ name: e.target.value })}/></label>
        <label className="t-row col" style={{flex:1}}><span className="t-lbl">Agent</span>
          <select className="t-select" value={step.agent_id || ""} onChange={e => onChange({ agent_id: e.target.value, agent_name: "" })}>
            <option value="">— pick agent —</option>
            {agents.map(a => <option key={a.id} value={a.id}>{a.name} ({String(a.id).slice(0, 8)})</option>)}
          </select></label>
        <label className="t-row col" style={{width:140}}><span className="t-lbl">Mode</span>
          <select className="t-select" value={step.mode} onChange={e => onChange({ mode: e.target.value })}>
            {["sequential","fan_out","collect","conditional","loop"].map(m => <option key={m} value={m}>{m}</option>)}
          </select></label>
      </div>
      <label className="t-row col"><span className="t-lbl">Prompt template (use {"{{input}}"} for previous step output)</span>
        <textarea className="modal-field modal-textarea" style={{minHeight:60}} value={step.prompt} onChange={e => onChange({ prompt: e.target.value })}/></label>
      {step.mode === "conditional" && (
        <label className="t-row col mt-8"><span className="t-lbl">Condition (run only if previous output contains)</span>
          <input className="modal-field" value={step.condition || ""} onChange={e => onChange({ condition: e.target.value })}/></label>
      )}
      {step.mode === "loop" && (
        <div className="row gap-12 mt-8">
          <label className="t-row col" style={{flex:1}}><span className="t-lbl">Max iterations</span>
            <input className="modal-field" type="number" min="1" value={step.max_iterations || 5} onChange={e => onChange({ max_iterations: Number(e.target.value) })}/></label>
          <label className="t-row col" style={{flex:2}}><span className="t-lbl">Until (output contains)</span>
            <input className="modal-field" value={step.until || ""} onChange={e => onChange({ until: e.target.value })}/></label>
        </div>
      )}
      <div className="row between mt-8">
        <button className="btn sm ghost" onClick={() => setAdvanced(a => !a)}>{advanced ? "− Hide advanced" : "+ Advanced"}</button>
      </div>
      {advanced && (
        <div className="row gap-12 mt-8">
          <label className="t-row col" style={{flex:1}}><span className="t-lbl">Timeout (s)</span>
            <input className="modal-field" type="number" min="10" value={step.timeout_secs || 120} onChange={e => onChange({ timeout_secs: Number(e.target.value) })}/></label>
          <label className="t-row col" style={{flex:1}}><span className="t-lbl">Error mode</span>
            <select className="t-select" value={step.error_mode || "fail"} onChange={e => onChange({ error_mode: e.target.value })}>
              {["fail","skip","retry"].map(m => <option key={m} value={m}>{m}</option>)}
            </select></label>
          {step.error_mode === "retry" && (
            <label className="t-row col" style={{flex:1}}><span className="t-lbl">Max retries</span>
              <input className="modal-field" type="number" min="1" max="10" value={step.max_retries || 3} onChange={e => onChange({ max_retries: Number(e.target.value) })}/></label>
          )}
          <label className="t-row col" style={{flex:1}}><span className="t-lbl">Output var (optional)</span>
            <input className="modal-field" value={step.output_var || ""} onChange={e => onChange({ output_var: e.target.value })} placeholder="e.g. ranked_list"/></label>
        </div>
      )}
    </div>
  );
}

function WorkflowRunModal({ workflow, onClose, onRun }) {
  useEscapeKey(onClose);
  const [inputJson, setInputJson] = useState("{}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const steps = Array.isArray(workflow.steps) ? workflow.steps : [];
  const run = async () => {
    let input;
    try { input = JSON.parse(inputJson); }
    catch (e) { setErr(`input must be valid JSON: ${e.message}`); return; }
    setBusy(true); setErr(null);
    try { await onRun(input); }
    catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <b className="mono">Run workflow · {workflow.name || workflow.id}</b>
            {workflow.description && <div className="dim" style={{fontSize:11.5, marginTop:2}}>{workflow.description}</div>}
          </div>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Input (JSON)</span>
          <textarea className="modal-field modal-textarea mt-8" style={{fontFamily:"var(--ff-mono)"}} value={inputJson} onChange={e => setInputJson(e.target.value)}/>

          <span className="muted mono mt-16" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", display:"block"}}>
            Preflight · {steps.length} step{steps.length === 1 ? "" : "s"}
            <Tip>Shown so you confirm which workflow + steps are about to run. Each row lists the step name, target agent, mode, and timeout.</Tip>
          </span>
          <div className="col gap-4 mt-8" style={{maxHeight:200, overflow:"auto"}}>
            {steps.length === 0 && <div className="dim mono" style={{fontSize:11.5}}>(workflow has no steps)</div>}
            {steps.map((s, i) => {
              const agent = (s.agent && (s.agent.id || s.agent.name)) || s.agent_id || s.agent_name || "—";
              const mode = typeof s.mode === "string" ? s.mode : (s.mode && Object.keys(s.mode)[0]) || "sequential";
              return (
                <div key={i} className="row gap-8" style={{padding:"5px 8px", background:"var(--bg-2)", borderRadius:5, fontFamily:"var(--ff-mono)", fontSize:11.5}}>
                  <span className="badge plain" style={{minWidth:32, textAlign:"center"}}>#{i + 1}</span>
                  <span style={{fontWeight:500}}>{s.name || "step"}</span>
                  <span className="dim">→ {agent}</span>
                  {mode !== "sequential" && <span className="badge violet" style={{marginLeft:"auto"}}>{mode}</span>}
                  {(s.timeout_secs && s.timeout_secs !== 120) ? <span className="dim" style={{marginLeft:mode!=="sequential"?6:"auto"}}>{s.timeout_secs}s</span> : null}
                </div>
              );
            })}
          </div>

          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={run} disabled={busy}>{busy ? "Running…" : `Run ${steps.length} step${steps.length === 1 ? "" : "s"}`}</button>
        </div>
      </div>
    </div>
  );
}

const Stat = ({ label, value }) => (
  <div className="col" style={{gap:2, paddingRight:18, borderRight:"1px solid var(--border)"}}>
    <span className="dim mono" style={{fontSize:10,letterSpacing:".12em",textTransform:"uppercase"}}>{label}</span>
    <span className="mono nums" style={{fontSize:16}}>{value}</span>
  </div>
);

const WorkflowDAG = ({ workflow }) => {
  // Real workflow definitions carry an ordered `steps` array. We render
  // them as a horizontal pipeline: first step gets the "start" border,
  // last gets "end", everything else neutral. The full DAG (branches,
  // joins) is a follow-up — for now linear flow matches the underlying
  // execution model.
  let nodes;
  if (workflow && Array.isArray(workflow.steps) && workflow.steps.length > 0) {
    const N = workflow.steps.length;
    nodes = workflow.steps.slice(0, 6).map((s, i) => ({
      x: 20 + i * 145,
      y: 130,
      kind: s.kind || s.type || (s.agent_id ? "agent" : s.tool ? "tool" : "step"),
      name: s.name || s.id || `step ${i + 1}`,
      tag: s.agent_id || s.tool || s.workflow_id || s.event || "—",
      cls: i === 0 ? "start" : (i === N - 1 ? "end" : ""),
    }));
  } else {
    // Sample wireframe — only shown when no workflow is selected.
    nodes = [
      { x: 20,  y: 130, kind: "trigger", name: "—",            tag: "select a workflow", cls: "start" },
    ];
  }
  const edges = nodes.slice(1).map((_, i) => [i, i + 1]);
  const centers = nodes.map(n => ({ x: n.x + 75, y: n.y + 22 }));

  return (
    <div className="dag">
      <svg width="100%" height="100%" style={{position:"absolute",inset:0}}>
        <defs>
          <marker id="arr" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="6" markerHeight="6" orient="auto">
            <path d="M0,0 L10,5 L0,10 z" fill="var(--rust)"/>
          </marker>
        </defs>
        {edges.map(([a,b], i) => {
          const A = centers[a], B = centers[b];
          const mid = (A.x + B.x)/2;
          return <path key={i} d={`M${A.x+72},${A.y} C${mid},${A.y} ${mid},${B.y} ${B.x-78},${B.y}`}
                       fill="none" stroke="var(--rust)" strokeWidth="1.4" opacity=".75"
                       strokeDasharray={i % 2 ? "4 3" : "0"}
                       markerEnd="url(#arr)"/>;
        })}
      </svg>
      {nodes.map((n, i) => (
        <div key={i} className={"node " + (n.cls || "")} style={{left: n.x, top: n.y, width: 150}}>
          <div className="node-kind">{n.kind}</div>
          <div className="node-name">{n.name}</div>
          <div className="node-tag">{n.tag}</div>
        </div>
      ))}
    </div>
  );
};

/* ============================== AUTOMATION ============================== */
function AutomationPage() {
  const [tab, setTab] = useState("cron");
  const [showCreate, setShowCreate] = useState(false);
  const [cronResp, , refreshCron] = usePolling("/api/cron/jobs", 15000);
  const [trigResp, , refreshTrig] = usePolling("/api/triggers", 15000);
  const [cronSelected, setCronSelected] = useState(() => new Set());
  const [trigSelected, setTrigSelected] = useState(() => new Set());

  React.useEffect(() => {
    const onNew = (e) => { if (e.detail && e.detail.page === "automation") setShowCreate(true); };
    const onRefresh = (e) => { if (e.detail && e.detail.page === "automation") (tab === "cron" ? refreshCron : refreshTrig)(); };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [tab, refreshCron, refreshTrig]);
  const cron = (cronResp && cronResp.jobs) || [];
  const triggers = Array.isArray(trigResp) ? trigResp : (trigResp && trigResp.triggers) || [];

  const toggleCron = async (id, enabled) => {
    try {
      // The kernel exposes this as PUT /api/cron/jobs/{id}/enable
      // (verb-named, despite acting as a toggle when invoked twice).
      await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/enable`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: !enabled }),
      });
      refreshCron();
    } catch (e) {
      console.warn("toggle failed", e);
    }
  };

  const runCronNow = async (id) => {
    try {
      // Endpoint is POST /api/cron/jobs/{id}/run (NOT /run-now —
      // the run_cron_job route doc-comment says /run, while the manual
      // documentation in routes.rs comments still mentions /run-now).
      await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/run`, { method: "POST" });
      refreshCron();
    } catch (e) {
      toastErr(`run failed: ${e.message || e}`);
    }
  };

  // Drop selections whose row disappeared (deleted elsewhere) so the
  // bulk-bar count stays honest.
  React.useEffect(() => {
    if (cronSelected.size === 0) return;
    const live = new Set(cron.map(c => c.id));
    const next = new Set([...cronSelected].filter(id => live.has(id)));
    if (next.size !== cronSelected.size) setCronSelected(next);
  }, [cron.map(c => c.id).join(",")]);
  const toggleCronSelect = (id) => {
    setCronSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const toggleAllCron = () => {
    setCronSelected(prev => {
      if (cron.length === 0) return prev;
      if (prev.size === cron.length) return new Set();
      return new Set(cron.map(c => c.id));
    });
  };
  const bulkSetEnabled = async (target) => {
    const ids = [...cronSelected];
    if (ids.length === 0) return;
    const label = target ? "Enable" : "Disable";
    const ok = await confirmDialog({
      title: `${label} ${ids.length} cron job(s)?`,
      message: `${label} all selected jobs. Disabled jobs do not fire on their schedule until re-enabled.`,
      confirmLabel: `${label} ${ids.length}`,
      danger: !target,
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/enable`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: target }),
        });
        okCount++;
      } catch (_) { failCount++; }
    }
    setCronSelected(new Set());
    if (failCount > 0) toastErr(`${label.toLowerCase()}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${label}d ${okCount} job(s)`);
    refreshCron();
  };

  // Same pattern as cron bulk for triggers — armed/disarmed via PUT
  // /api/triggers/{id} with {enabled: bool}. Falls back to "armed" label
  // when target=false because that's what the rest of the UI calls a
  // not-yet-fired trigger.
  React.useEffect(() => {
    if (trigSelected.size === 0) return;
    const live = new Set(triggers.map(t => t.id));
    const next = new Set([...trigSelected].filter(id => live.has(id)));
    if (next.size !== trigSelected.size) setTrigSelected(next);
  }, [triggers.map(t => t.id).join(",")]);
  const toggleTrigSelect = (id) => {
    setTrigSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const toggleAllTrig = () => {
    setTrigSelected(prev => {
      if (triggers.length === 0) return prev;
      if (prev.size === triggers.length) return new Set();
      return new Set(triggers.map(t => t.id));
    });
  };
  const bulkTrigSetEnabled = async (target) => {
    const ids = [...trigSelected];
    if (ids.length === 0) return;
    const label = target ? "Arm" : "Disarm";
    const ok = await confirmDialog({
      title: `${label} ${ids.length} trigger(s)?`,
      message: `${label} all selected triggers. Disarmed triggers do not match incoming events until re-armed.`,
      confirmLabel: `${label} ${ids.length}`,
      danger: !target,
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/triggers/${encodeURIComponent(id)}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: target }),
        });
        okCount++;
      } catch (_) { failCount++; }
    }
    setTrigSelected(new Set());
    if (failCount > 0) toastErr(`${label.toLowerCase()}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${label}ed ${okCount} trigger(s)`);
    refreshTrig();
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Automation</h1>
          <p className="page-sub">Cron jobs survive restart · 3 CronAction variants · trigger fire-counts persisted</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={() => (tab === "cron" ? refreshCron : refreshTrig)()}><I.refresh/></button>
          <button className="btn primary" onClick={() => setShowCreate(true)}><I.plus/> New {tab === "cron" ? "job" : "trigger"}</button>
        </div>
      </div>

      <div className="tabs">
        <button className={tab==="cron"?"on":""} onClick={()=>setTab("cron")}>Cron jobs · {cron.length}</button>
        <button className={tab==="triggers"?"on":""} onClick={()=>setTab("triggers")}>Triggers · {triggers.length}</button>
      </div>

      {tab === "cron" && (
        <>
          {cronSelected.size > 0 && (
            <div className="bulk-bar">
              <span className="mono" style={{fontSize:12}}>{cronSelected.size} selected</span>
              <button className="btn sm primary" onClick={() => bulkSetEnabled(true)}>
                <I.play/> Enable {cronSelected.size}
              </button>
              <button className="btn sm danger" onClick={() => bulkSetEnabled(false)}>
                <I.pause/> Disable {cronSelected.size}
              </button>
              <button className="btn sm ghost" onClick={() => setCronSelected(new Set())} style={{marginLeft:"auto"}}>Clear</button>
            </div>
          )}
          <div className="card flush">
            <table className="tbl">
              <thead><tr>
                <th style={{width:28}}>
                  <input type="checkbox"
                         checked={cron.length > 0 && cronSelected.size === cron.length}
                         ref={el => { if (el) el.indeterminate = cronSelected.size > 0 && cronSelected.size < cron.length; }}
                         onChange={toggleAllCron}
                         title={cronSelected.size === cron.length ? "Deselect all" : "Select all"}/>
                </th>
                <th>ID</th><th>Schedule</th><th>Action</th><th>Next</th><th className="right">Fires</th><th>Enabled</th><th></th>
              </tr></thead>
              <tbody>
                {!cronResp && (<tr><td colSpan={8} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
                {cronResp && cron.length === 0 && (
                  <tr><td colSpan={8} style={{padding:"24px 14px", textAlign:"center"}}>
                    <div className="dim" style={{fontSize:12, marginBottom:8}}>No cron jobs yet.</div>
                    <button className="btn primary sm" onClick={() => setShowCreate(true)}><I.plus/> Schedule your first job</button>
                  </td></tr>
                )}
                {cron.map(c => {
                  const actionLabel = c.action_label || c.action_summary
                    || (c.action && (c.action.kind || c.action.type || JSON.stringify(c.action).slice(0, 60)))
                    || "—";
                  const next = c.next_run || c.next_fire || c.next || "—";
                  const fires = c.fire_count != null ? c.fire_count : (c.fires || 0);
                  const isSel = cronSelected.has(c.id);
                  return (
                    <tr key={c.id} style={isSel ? {background:"var(--surface-2)"} : null}>
                      <td>
                        <input type="checkbox"
                               checked={isSel}
                               onChange={() => toggleCronSelect(c.id)}/>
                      </td>
                      <td className="mono">{c.id}</td>
                      <td><span className="mono" style={{color:"var(--rust)"}}>{c.schedule || c.cron || c.expression}</span></td>
                      <td className="mono" style={{maxWidth:300, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{actionLabel}</td>
                      <td className="mono muted">{next === "—" ? next : formatTime(next)}</td>
                      <td className="num mono">{Number(fires).toLocaleString()}</td>
                      <td><div className={"switch " + (c.enabled ? "on" : "")} onClick={() => toggleCron(c.id, c.enabled)}/></td>
                      <td className="right"><button className="btn sm ghost" onClick={() => runCronNow(c.id)}>Run now</button></td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}

      {tab === "triggers" && (
        <>
          {trigSelected.size > 0 && (
            <div className="bulk-bar">
              <span className="mono" style={{fontSize:12}}>{trigSelected.size} selected</span>
              <button className="btn sm primary" onClick={() => bulkTrigSetEnabled(true)}>
                <I.play/> Arm {trigSelected.size}
              </button>
              <button className="btn sm danger" onClick={() => bulkTrigSetEnabled(false)}>
                <I.pause/> Disarm {trigSelected.size}
              </button>
              <button className="btn sm ghost" onClick={() => setTrigSelected(new Set())} style={{marginLeft:"auto"}}>Clear</button>
            </div>
          )}
          <div className="card flush">
            <table className="tbl">
              <thead><tr>
                <th style={{width:28}}>
                  <input type="checkbox"
                         checked={triggers.length > 0 && trigSelected.size === triggers.length}
                         ref={el => { if (el) el.indeterminate = trigSelected.size > 0 && trigSelected.size < triggers.length; }}
                         onChange={toggleAllTrig}
                         title={trigSelected.size === triggers.length ? "Deselect all" : "Select all"}/>
                </th>
                <th>ID</th><th>Kind</th><th>Target</th><th className="right">Fired</th><th>Last</th><th>Status</th><th></th>
              </tr></thead>
              <tbody>
                {!trigResp && (<tr><td colSpan={8} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
                {trigResp && triggers.length === 0 && (
                  <tr><td colSpan={8} style={{padding:"24px 14px", textAlign:"center"}}>
                    <div className="dim" style={{fontSize:12, marginBottom:8}}>No triggers configured.</div>
                    <button className="btn primary sm" onClick={() => setShowCreate(true)}><I.plus/> Add your first trigger</button>
                  </td></tr>
                )}
                {triggers.map(t => {
                  const kind = (t.kind || t.type || "—").toString();
                  const target = t.target || t.agent_id || t.workflow_id || "—";
                  const fired = t.fire_count != null ? t.fire_count : (t.fired || 0);
                  const last = t.last_fired || t.last || null;
                  const status = (t.status || (t.enabled ? "active" : "armed")).toString();
                  const isSel = trigSelected.has(t.id);
                  return (
                    <tr key={t.id} style={isSel ? {background:"var(--surface-2)"} : null}>
                      <td>
                        <input type="checkbox"
                               checked={isSel}
                               onChange={() => toggleTrigSelect(t.id)}/>
                      </td>
                      <td className="mono">{t.id}</td>
                      <td>
                        <span className="row gap-6" style={{color:"var(--fg-2)"}}>
                          <span style={{color:"var(--rust)",display:"inline-flex"}}><ChannelIcon kind={kind}/></span>
                          <span className="mono">{kind}</span>
                        </span>
                      </td>
                      <td className="mono">{target}</td>
                      <td className="num mono">{Number(fired).toLocaleString()}</td>
                      <td className="mono muted">{last ? formatTime(last) : "—"}</td>
                      <td>{status === "active" ? <span className="badge live">active</span> : <span className="badge violet">{status}</span>}</td>
                      <td className="right"><button className="btn sm ghost"><I.more/></button></td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}

      {showCreate && tab === "cron" && <CronJobModal onClose={() => setShowCreate(false)} onCreated={() => { setShowCreate(false); refreshCron(); }}/>}
      {showCreate && tab === "triggers" && <TriggerModal onClose={() => setShowCreate(false)} onCreated={() => { setShowCreate(false); refreshTrig(); }}/>}
    </div>
  );
}

function CronJobModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const [workflowsResp] = useApi("/api/workflows");
  const agents = (agentsResp && agentsResp.agents) || [];
  const workflows = Array.isArray(workflowsResp) ? workflowsResp : (workflowsResp && workflowsResp.workflows) || [];

  const [name, setName] = useState("");
  const [scheduleKind, setScheduleKind] = useState("cron"); // cron | every | at
  const [cronExpr, setCronExpr] = useState("0 9 * * *");
  const [everySecs, setEverySecs] = useState(3600);
  const [atIso, setAtIso] = useState(new Date(Date.now() + 3600000).toISOString().slice(0, 16));
  const [actionKind, setActionKind] = useState("system_event"); // system_event | agent_turn | workflow_run
  const [agentId, setAgentId] = useState("");
  const [message, setMessage] = useState("Scheduled check-in");
  const [systemText, setSystemText] = useState("scheduled event");
  const [workflowId, setWorkflowId] = useState("");
  const [workflowInput, setWorkflowInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  React.useEffect(() => { if (!agentId && agents.length) setAgentId(agents[0].id); }, [agents.length]);
  React.useEffect(() => { if (!workflowId && workflows.length) setWorkflowId(workflows[0].id); }, [workflows.length]);

  const buildSchedule = () => {
    if (scheduleKind === "cron") return { kind: "cron", expr: cronExpr };
    if (scheduleKind === "every") return { kind: "every", every_secs: Number(everySecs) };
    return { kind: "at", at: new Date(atIso).toISOString() };
  };
  const buildAction = () => {
    if (actionKind === "system_event") return { kind: "system_event", text: systemText };
    if (actionKind === "agent_turn") return { kind: "agent_turn", message };
    return { kind: "workflow_run", workflow_id: workflowId, input: workflowInput };
  };

  const submit = async () => {
    if (!name.trim()) { setErr("name is required"); return; }
    if (!agentId) { setErr("pick an owning agent"); return; }
    setBusy(true); setErr(null);
    try {
      await rhFetch("/api/cron/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          agent_id: agentId,
          schedule: buildSchedule(),
          action: buildAction(),
          enabled: true,
        }),
      });
      onCreated();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">New cron job</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="col gap-8">
            <label className="t-row col"><span className="t-lbl">Name</span>
              <input className="modal-field" value={name} onChange={e => setName(e.target.value)} placeholder="daily-digest"/></label>
            <label className="t-row col"><span className="t-lbl">Owning agent</span>
              <select className="t-select" value={agentId} onChange={e => setAgentId(e.target.value)}>
                <option value="">— pick agent —</option>
                {agents.map(a => <option key={a.id} value={a.id}>{a.name} ({String(a.id).slice(0, 8)})</option>)}
              </select></label>
            <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", marginTop:6}}>
              Schedule
              <Tip>
                <b>Cron</b> — 5-field expression (min hour dom month dow), optional IANA tz.<br/>
                <b>Every</b> — fixed interval in seconds (60..86400).<br/>
                <b>At</b> — fire once at the specified UTC time.
              </Tip>
            </span>
            <div className="seg">
              <button className={scheduleKind==="cron"?"on":""} onClick={() => setScheduleKind("cron")}>Cron</button>
              <button className={scheduleKind==="every"?"on":""} onClick={() => setScheduleKind("every")}>Every</button>
              <button className={scheduleKind==="at"?"on":""} onClick={() => setScheduleKind("at")}>At</button>
            </div>
            {scheduleKind === "cron" && <input className="modal-field" placeholder="0 9 * * *" value={cronExpr} onChange={e => setCronExpr(e.target.value)}/>}
            {scheduleKind === "every" && <input className="modal-field" type="number" min={60} value={everySecs} onChange={e => setEverySecs(e.target.value)} placeholder="seconds (60..86400)"/>}
            {scheduleKind === "at" && <input className="modal-field" type="datetime-local" value={atIso} onChange={e => setAtIso(e.target.value)}/>}

            <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", marginTop:6}}>
              Action
              <Tip>
                <b>System event</b> — publishes a kernel event with the given text.<br/>
                <b>Agent turn</b> — sends a message to the owning agent.<br/>
                <b>Workflow run</b> — executes a workflow with input.
              </Tip>
            </span>
            <div className="seg">
              <button className={actionKind==="system_event"?"on":""} onClick={() => setActionKind("system_event")}>System event</button>
              <button className={actionKind==="agent_turn"?"on":""} onClick={() => setActionKind("agent_turn")}>Agent turn</button>
              <button className={actionKind==="workflow_run"?"on":""} onClick={() => setActionKind("workflow_run")}>Workflow run</button>
            </div>
            {actionKind === "system_event" && <input className="modal-field" value={systemText} onChange={e => setSystemText(e.target.value)} placeholder="event text"/>}
            {actionKind === "agent_turn" && <textarea className="modal-field modal-textarea" value={message} onChange={e => setMessage(e.target.value)} placeholder="message to send"/>}
            {actionKind === "workflow_run" && (
              <>
                <select className="t-select" value={workflowId} onChange={e => setWorkflowId(e.target.value)}>
                  <option value="">— pick workflow —</option>
                  {workflows.map(w => <option key={w.id} value={w.id}>{w.name || w.id}</option>)}
                </select>
                <textarea className="modal-field modal-textarea" value={workflowInput} onChange={e => setWorkflowInput(e.target.value)} placeholder="workflow input (text)"/>
              </>
            )}
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Creating…" : "Create"}</button>
        </div>
      </div>
    </div>
  );
}

function TriggerModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const agents = (agentsResp && agentsResp.agents) || [];
  const [agentId, setAgentId] = useState("");
  const [patternKind, setPatternKind] = useState("system");
  const [namePattern, setNamePattern] = useState("");
  const [keyword, setKeyword] = useState("");
  const [substring, setSubstring] = useState("");
  const [keyPattern, setKeyPattern] = useState("");
  const [promptTemplate, setPromptTemplate] = useState("Event fired: {{event}}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  React.useEffect(() => { if (!agentId && agents.length) setAgentId(agents[0].id); }, [agents.length]);

  const buildPattern = () => {
    switch (patternKind) {
      case "lifecycle": return "lifecycle";
      case "agent_spawned": return { agent_spawned: { name_pattern: namePattern || ".*" } };
      case "agent_terminated": return "agent_terminated";
      case "system": return "system";
      case "system_keyword": return { system_keyword: { keyword } };
      case "memory_update": return "memory_update";
      case "memory_key_pattern": return { memory_key_pattern: { key_pattern: keyPattern } };
      case "all": return "all";
      case "content_match": return { content_match: { substring } };
      default: return "all";
    }
  };

  const submit = async () => {
    if (!agentId) { setErr("pick an agent"); return; }
    setBusy(true); setErr(null);
    try {
      await rhFetch("/api/triggers", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          agent_id: agentId,
          pattern: buildPattern(),
          prompt_template: promptTemplate,
        }),
      });
      onCreated();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">New trigger</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="col gap-8">
            <label className="t-row col"><span className="t-lbl">Owning agent</span>
              <select className="t-select" value={agentId} onChange={e => setAgentId(e.target.value)}>
                <option value="">— pick agent —</option>
                {agents.map(a => <option key={a.id} value={a.id}>{a.name}</option>)}
              </select></label>
            <label className="t-row col"><span className="t-lbl">Pattern</span>
              <select className="t-select" value={patternKind} onChange={e => setPatternKind(e.target.value)}>
                {["all","lifecycle","agent_spawned","agent_terminated","system","system_keyword","memory_update","memory_key_pattern","content_match"].map(p =>
                  <option key={p} value={p}>{p}</option>)}
              </select></label>
            {patternKind === "agent_spawned" && <input className="modal-field" placeholder="name pattern (regex)" value={namePattern} onChange={e => setNamePattern(e.target.value)}/>}
            {patternKind === "system_keyword" && <input className="modal-field" placeholder="keyword" value={keyword} onChange={e => setKeyword(e.target.value)}/>}
            {patternKind === "memory_key_pattern" && <input className="modal-field" placeholder="key pattern" value={keyPattern} onChange={e => setKeyPattern(e.target.value)}/>}
            {patternKind === "content_match" && <input className="modal-field" placeholder="substring" value={substring} onChange={e => setSubstring(e.target.value)}/>}
            <label className="t-row col"><span className="t-lbl">Prompt template (use {"{{event}}"})</span>
              <textarea className="modal-field modal-textarea" value={promptTemplate} onChange={e => setPromptTemplate(e.target.value)}/></label>
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Creating…" : "Create"}</button>
        </div>
      </div>
    </div>
  );
}

/* ============================== CHANNELS ============================== */
function ChannelsPage() {
  const [chResp, , refresh] = usePolling("/api/channels", 20000);
  const channels = (chResp && chResp.channels) || [];
  const [configuring, setConfiguring] = useState(null);
  const [testResult, setTestResult] = useState(null);
  const [activityFor, setActivityFor] = useState(null);
  // Audit window used to derive per-channel recent activity (last 10
  // matches). 200 entries is the page-wide default and is plenty for
  // "who pinged us in the last few minutes" answers.
  const [auditResp] = usePolling("/api/audit/recent?n=200", 15000);

  const testChannel = async (name) => {
    setTestResult({ name, busy: true });
    try {
      const r = await rhFetch(`/api/channels/${encodeURIComponent(name)}/test`, { method: "POST" });
      setTestResult({ name, ...r, busy: false });
    } catch (e) {
      setTestResult({ name, ok: false, message: e.message || String(e), busy: false });
    }
  };
  const reload = async () => {
    try { await rhFetch("/api/channels/reload", { method: "POST" }); refresh(); }
    catch (e) { toastErr(`reload failed: ${e.message || e}`); }
  };
  const removeChannel = async (name) => {
    if (!(await confirmDialog({ title: "Disconnect", message: `Disconnect ${name}?`, danger: true, confirmLabel: "Disconnect" }))) return;
    try { await rhFetch(`/api/channels/${encodeURIComponent(name)}/configure`, { method: "DELETE" }); refresh(); }
    catch (e) { toastErr(`disconnect failed: ${e.message || e}`); }
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Channels <span className="dim mono" style={{fontSize:14}}>· {channels.filter(c=>c.configured).length} of {channels.length} configured</span></h1>
          <p className="page-sub">Telegram-first · streaming via editMessageText · 500ms throttle · Discord + Slack adapters available</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
          <button className="btn ghost" onClick={reload}>Reload</button>
        </div>
      </div>
      {!chResp && <div className="muted mono" style={{padding:"24px", fontSize:12}}>loading…</div>}
      <div className="grid-12">
        {channels.map(ch => {
          const kind = ch.icon || ch.name;
          const state = ch.configured
            ? (ch.auth_status === "ok" ? "live" : (ch.auth_status === "auth_failed" ? "auth_failed" : "idle"))
            : "idle";
          const stateLabel = ch.configured
            ? (ch.auth_status === "ok" ? "live" : ch.auth_status === "auth_failed" ? "auth failed" : "configured")
            : "not configured";
          return (
            <div key={ch.name} className="col-6 card">
              <div className="row between mb-12">
                <div className="row gap-12">
                  <div className="avatar lg" style={{background:"linear-gradient(135deg,var(--rust),oklch(0.42 0.10 50))"}}>
                    <ChannelIcon kind={kind}/>
                  </div>
                  <div>
                    <div className="mono" style={{fontSize:14,fontWeight:500}}>{ch.display_name || ch.name}</div>
                    <div className="dim mono" style={{fontSize:11}}>{ch.name}{ch.difficulty ? ` · ${ch.difficulty}` : ""}{ch.setup_time ? ` · ${ch.setup_time}` : ""}</div>
                  </div>
                </div>
                <span className={"badge " + (state === "live" ? "live" : state === "auth_failed" ? "error" : "idle")}>{stateLabel}</span>
              </div>
              <div className="dim" style={{fontSize:11.5, marginBottom:12}}>{ch.description}</div>
              {testResult && testResult.name === ch.name && (
                <ChannelTestCard result={testResult}/>
              )}
              {activityFor === ch.name && (
                <ChannelActivityCard
                  channelName={ch.name}
                  entries={(auditResp && auditResp.entries) || []}
                  onClose={() => setActivityFor(null)}
                />
              )}
              <div className="row gap-6">
                {ch.configured ? (
                  <>
                    <button className="btn sm" onClick={() => testChannel(ch.name)}>Test</button>
                    <button className="btn sm ghost" onClick={() => setActivityFor(activityFor === ch.name ? null : ch.name)}>
                      {activityFor === ch.name ? "Hide activity" : "Activity"}
                    </button>
                    <button className="btn sm ghost" onClick={() => setConfiguring(ch)}>Reconfigure</button>
                    <button className="btn sm ghost" onClick={() => removeChannel(ch.name)} style={{color:"var(--crimson)"}}>Disconnect</button>
                  </>
                ) : (
                  <button className="btn sm primary" onClick={() => setConfiguring(ch)}><I.plus/> Connect</button>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {configuring && <ChannelConfigModal channel={configuring} onClose={() => setConfiguring(null)} onSaved={() => { setConfiguring(null); refresh(); }}/>}
    </div>
  );
}

// Channel test result rendered as a structured card. The server returns
// adapter-specific shapes (telegram: {ok, me:{username,first_name}};
// discord: {ok, gateway:{ms}}; webhook: {status, latency_ms}) — we
// surface known fields with labels and fall through to a JSON dump for
// anything we don't recognize, instead of showing raw stringified JSON.
// Inline activity card: filters the audit-log window to entries whose
// action / detail / agent reference the given channel name. No backend
// changes — uses the same /api/audit/recent feed all pages share. Shows
// last 10 matches; deep-links each into the Audit page on click.
function ChannelActivityCard({ channelName, entries, onClose }) {
  const lc = (channelName || "").toLowerCase();
  if (!lc) return null;
  const matches = (entries || []).filter(e => {
    const haystack = [e.action || "", e.detail || "", e.agent_name || "", e.agent_id || "", e.outcome || ""]
      .join(" ").toLowerCase();
    return haystack.includes(lc);
  }).slice(0, 10);
  return (
    <div style={{
      marginTop: 8,
      padding: "10px 12px",
      background: "var(--bg-2)",
      borderRadius: 6,
      border: "1px solid var(--border)",
    }}>
      <div className="row between mb-8">
        <div className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
          Recent activity <span className="dim" style={{marginLeft:6}}>{matches.length} of last 200</span>
        </div>
        <button className="kbd" onClick={onClose} style={{cursor:"pointer"}}>close</button>
      </div>
      {matches.length === 0 && (
        <div className="dim mono" style={{fontSize:11, textAlign:"center", padding:"6px 0"}}>
          No matches in the last 200 audit entries.
        </div>
      )}
      {matches.length > 0 && (
        <div className="col gap-4" style={{maxHeight:200, overflow:"auto"}}>
          {matches.map((e, i) => {
            const hash = e.hash ? String(e.hash).slice(0, 8) : "";
            const link = hash ? `#/audit?h=${encodeURIComponent(hash)}` : "#/audit";
            return (
              <a key={`${e.seq || i}`} href={link} style={{
                display:"block",
                padding:"5px 8px",
                fontFamily:"var(--ff-mono)",
                fontSize:11,
                background:"var(--surface)",
                borderRadius:5,
                color:"var(--fg-2)",
                textDecoration:"none",
              }}>
                <span style={{color:"var(--rust)"}}>{e.action || "—"}</span>
                {e.outcome && <span className="dim" style={{marginLeft:6}}>· {e.outcome}</span>}
                <span className="dim" style={{marginLeft:6}}>· {relativeTime(e.timestamp || e.created_at)}</span>
                {e.detail && (
                  <div className="dim" style={{
                    marginTop:2,
                    whiteSpace:"nowrap",
                    overflow:"hidden",
                    textOverflow:"ellipsis",
                  }}>{e.detail}</div>
                )}
              </a>
            );
          })}
        </div>
      )}
    </div>
  );
}

function ChannelTestCard({ result }) {
  const r = result || {};
  const ok = !!r.ok;
  const known = [];
  if (r.latency_ms != null) known.push(["Latency", `${r.latency_ms} ms`]);
  if (r.status != null) known.push(["HTTP status", String(r.status)]);
  if (r.me) {
    if (r.me.username) known.push(["Bot username", `@${r.me.username}`]);
    if (r.me.first_name) known.push(["Bot name", r.me.first_name]);
    if (r.me.id != null) known.push(["Bot ID", String(r.me.id)]);
  }
  if (r.gateway) {
    if (r.gateway.ms != null) known.push(["Gateway latency", `${r.gateway.ms} ms`]);
    if (r.gateway.session_id) known.push(["Session", r.gateway.session_id]);
  }
  if (r.workspace) known.push(["Workspace", r.workspace]);
  // Anything else worth pretty-printing (e.g. echo of a sample message).
  const otherKnown = new Set(["ok", "busy", "name", "message", "detail", "latency_ms", "status", "me", "gateway", "workspace"]);
  const extras = Object.entries(r).filter(([k]) => !otherKnown.has(k) && r[k] != null);
  const message = r.message || r.detail;
  return (
    <div className="banner mb-12" style={{
      flexDirection:"column", alignItems:"stretch", gap:6,
      borderColor: r.busy ? "var(--border-hi)" : (ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)"),
    }}>
      <div className="row gap-8">
        <span className={"dot " + (r.busy ? "warn" : ok ? "live" : "err")}/>
        <span className="banner-title">{r.busy ? "TESTING" : ok ? "OK" : "FAIL"}</span>
        {message && <span className="banner-body" style={{fontSize:11.5}}>{message}</span>}
      </div>
      {known.length > 0 && (
        <div className="kv" style={{gridTemplateColumns:"110px 1fr", fontSize:11.5, padding:"4px 0 0 16px"}}>
          {known.map(([k, v]) => (
            <React.Fragment key={k}>
              <dt style={{fontFamily:"var(--ff-mono)", color:"var(--fg-4)"}}>{k}</dt>
              <dd className="mono">{v}</dd>
            </React.Fragment>
          ))}
        </div>
      )}
      {extras.length > 0 && (
        <pre className="codebox" style={{fontSize:10.5, marginTop:6, maxHeight:120, marginLeft:16}}>
{extras.map(([k, v]) => `${k}: ${typeof v === "string" ? v : JSON.stringify(v, null, 2)}`).join("\n")}
        </pre>
      )}
    </div>
  );
}

function ChannelConfigModal({ channel, onClose, onSaved }) {
  useEscapeKey(onClose);
  const [values, setValues] = useState({});
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const fields = channel.fields || [];

  const save = async () => {
    setBusy(true); setErr(null);
    try {
      await rhFetch(`/api/channels/${encodeURIComponent(channel.name)}/configure`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fields: values }),
      });
      onSaved();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Configure · {channel.display_name || channel.name}</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          {channel.description && <div className="dim" style={{fontSize:12, marginBottom:12}}>{channel.description}</div>}
          <div className="col gap-8">
            {fields.length === 0 && <div className="dim mono" style={{fontSize:11}}>No fields defined for this channel.</div>}
            {fields.map(f => (
              <label key={f.name} className="t-row col">
                <span className="t-lbl">
                  {f.label || f.name}
                  {f.required && <span style={{color:"var(--rust)"}}> *</span>}
                  {f.env_var && <span className="dim mono" style={{marginLeft:6, fontSize:10}}>({f.env_var})</span>}
                </span>
                <input
                  className="modal-field"
                  type={f.secret ? "password" : "text"}
                  placeholder={f.placeholder || ""}
                  value={values[f.name] || ""}
                  onChange={e => setValues({ ...values, [f.name]: e.target.value })}
                />
                {f.description && <span className="dim" style={{fontSize:10.5, marginTop:2}}>{f.description}</span>}
              </label>
            ))}
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={save} disabled={busy}>{busy ? "Saving…" : "Save & connect"}</button>
        </div>
      </div>
    </div>
  );
}

/* ============================== ANALYTICS ============================== */
function AnalyticsPage() {
  // /api/usage/daily returns up to 30 daily buckets: {days:[{date, cost_usd, requests}], today_cost_usd, first_event_date}.
  // /api/usage/by-model returns per-model spend totals.
  // /api/usage returns aggregate stats including cache hit-rate.
  const [daily] = usePolling("/api/usage/daily", 30000);
  const [byModel] = usePolling("/api/usage/by-model", 30000);
  const [stats] = usePolling("/api/usage", 30000);
  const [agentsResp] = usePolling("/api/agents?limit=200", 30000);
  const agents = (agentsResp && agentsResp.agents) ? agentsResp.agents.map(normalizeAgent) : [];

  const days = (daily && daily.days) || [];
  const dailyCosts = days.map(d => Number(d.cost_usd || 0));
  const totalSpend7d = dailyCosts.slice(-7).reduce((s, v) => s + v, 0);
  const totalRequests7d = days.slice(-7).reduce((s, d) => s + (d.requests || 0), 0);
  const avgPerDay = days.length ? (totalRequests7d / Math.min(7, days.length)) : 0;
  const modelRows = Array.isArray(byModel) ? byModel : (byModel && byModel.models) || [];
  const maxModelSpend = Math.max(0.0001, ...modelRows.map(m => Number(m.spend || m.cost_usd || 0)));

  const hourSeries = dailyCosts.slice(-24);
  const seriesForChart = hourSeries.length ? hourSeries : Array(24).fill(0);
  const totalForChart = seriesForChart.reduce((s, v) => s + v, 0);

  const cacheHitRate = stats && stats.cache_hit_rate != null
    ? `${Math.round(Number(stats.cache_hit_rate) * 100)}%`
    : "—";
  const p95 = stats && stats.p95_latency_ms != null
    ? `${(Number(stats.p95_latency_ms) / 1000).toFixed(2)}s`
    : "—";

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Analytics</h1>
          <p className="page-sub">Token spend, latency, cache hits · sourced from <span className="mono">/api/usage</span> · <span className="mono">/api/usage/daily</span> · <span className="mono">/api/usage/by-model</span></p>
        </div>
        <div className="page-actions">
          <div className="seg"><button className="on">7d</button></div>
          <button className="btn ghost" onClick={() => {
            if (!daily) return;
            const csv = rowsToCsv(days, [
              { key: "date", label: "date" },
              { key: "cost_usd", label: "cost_usd", format: v => Number(v || 0).toFixed(6) },
              { key: "requests", label: "requests" },
            ]);
            downloadBlob(`rustyhand-usage-${new Date().toISOString().slice(0, 10)}.csv`, csv, "text/csv");
          }}><I.download/> CSV</button>
        </div>
      </div>

      <div className="tiles">
        <Tile label="Total spend · 7d"  value={`$${totalSpend7d.toFixed(2)}`}  foot={daily ? `${days.length} day(s) of data` : "loading…"} spark={dailyCosts.slice(-12)}/>
        <Tile label="LLM requests · 7d" value={totalRequests7d.toLocaleString()} foot={daily ? `${avgPerDay.toFixed(0)} / day avg` : "loading…"} spark={days.slice(-12).map(d => d.requests || 0)}/>
        <Tile label="Cache hit-rate"    value={cacheHitRate} foot={stats ? "LLM cache · 24h TTL" : "loading…"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
        <Tile label="p95 latency"       value={p95} foot={stats ? "kernel telemetry" : "loading…"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
      </div>

      <div className="grid-12">
        <div className="col-8 card">
          <div className="row between mb-12">
            <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>Cost · daily (last 24 buckets)</span>
            <span className="mono dim" style={{fontSize:11}}>${totalForChart.toFixed(2)}</span>
          </div>
          <CostChart data={seriesForChart}/>
        </div>

        <div className="col-4 card">
          <div className="row between mb-12">
            <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>Spend by model</span>
            <span className="mono dim" style={{fontSize:11}}>${modelRows.reduce((s, m) => s + Number(m.spend || m.cost_usd || 0), 0).toFixed(2)}</span>
          </div>
          <div className="col" style={{gap:4}}>
            {!byModel && <div className="muted mono" style={{fontSize:11, padding:"6px 0"}}>loading…</div>}
            {byModel && modelRows.length === 0 && <div className="muted mono" style={{fontSize:11, padding:"6px 0"}}>no model usage data yet.</div>}
            {modelRows.slice(0, 8).map(m => (
              <BarRow key={m.model || m.name} label={m.model || m.name} value={Number(m.spend || m.cost_usd || 0)} max={maxModelSpend} unit="$"/>
            ))}
          </div>
        </div>

        <div className="col-6 card flush">
          <div className="card-head"><span>Agents · by activity</span></div>
          <table className="tbl">
            <thead><tr><th>Agent</th><th>Model</th><th>State</th><th className="right">Updated</th></tr></thead>
            <tbody>
              {agents.length === 0 && (<tr><td colSpan={4} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
              {agents.slice(0, 8).map(a => (
                <tr key={a.id}>
                  <td><div className="agent-row"><Avatar agent={a}/><span className="name">{a.name}</span></div></td>
                  <td className="mono muted">{a.model}</td>
                  <td><StateBadge state={a.state}/></td>
                  <td className="num mono muted">{a.updated}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <div className="col-6 card">
          <div className="row between mb-12">
            <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>Provider state</span>
            <span className="dim mono" style={{fontSize:11}}>from /api/providers</span>
          </div>
          <ProviderState/>
        </div>
      </div>
    </div>
  );
}

// Provider authentication & circuit-breaker state, fed by /api/providers.
function ProviderState() {
  const [resp] = usePolling("/api/providers", 30000);
  const providers = (resp && resp.providers) || [];
  if (!resp) return <div className="muted mono" style={{fontSize:11}}>loading…</div>;
  if (providers.length === 0) return <div className="muted mono" style={{fontSize:11}}>no providers configured</div>;
  return (
    <div className="col gap-6">
      {providers.slice(0, 10).map(p => {
        const auth = (p.auth_status || "").toLowerCase();
        let label = auth || "—";
        let err = false, warn = false;
        if (auth === "ok") label = "Connected";
        else if (auth === "missing") { label = "No key"; warn = p.key_required !== false; }
        else if (auth === "invalid") { label = "Invalid"; err = true; }
        else if (auth === "rate_limited") { label = "Rate-limited"; warn = true; }
        const tail = p.model_count != null ? `${p.model_count} model${p.model_count === 1 ? "" : "s"}` : "";
        return <BreakerRow key={p.id || p.name} name={p.display_name || p.id || p.name} state={label} tail={tail} err={err} warn={warn}/>;
      })}
    </div>
  );
}

const BreakerRow = ({ name, state, tail, warn, err }) => (
  <div className="row between" style={{padding:"7px 10px",background:"var(--bg-2)",borderRadius:6,border:"1px solid var(--border)"}}>
    <span className="mono" style={{fontSize:12}}>{name}</span>
    <span className="row gap-8">
      <span className={"badge " + (err ? "error" : warn ? "warn" : "live")}>{state}</span>
      <span className="dim mono" style={{fontSize:11}}>{tail}</span>
    </span>
  </div>
);

const CostChart = ({ data }) => {
  const W = 800, H = 220, P = 28;
  const max = Math.max(...data), min = 0;
  const x = i => P + (i / (data.length - 1)) * (W - P*2);
  const y = v => H - P - ((v - min) / (max - min)) * (H - P*2);
  const path = data.map((v,i) => (i===0?`M${x(i)},${y(v)}`:`L${x(i)},${y(v)}`)).join(" ");
  const area = `${path} L${x(data.length-1)},${H-P} L${x(0)},${H-P} Z`;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" height="220" style={{display:"block"}}>
      <defs>
        <linearGradient id="g1" x1="0" x2="0" y1="0" y2="1">
          <stop offset="0%" stopColor="var(--rust)" stopOpacity=".35"/>
          <stop offset="100%" stopColor="var(--rust)" stopOpacity="0"/>
        </linearGradient>
      </defs>
      {/* y gridlines */}
      {[0,.25,.5,.75,1].map(t => {
        const yy = P + (1-t)*(H-P*2);
        return <g key={t}>
          <line x1={P} x2={W-P} y1={yy} y2={yy} stroke="var(--border)" strokeWidth="1"/>
          <text x={4} y={yy+3} fill="var(--fg-4)" fontFamily="var(--ff-mono)" fontSize="9.5">${(max*t).toFixed(1)}</text>
        </g>;
      })}
      <path d={area} fill="url(#g1)"/>
      <path d={path} fill="none" stroke="var(--rust)" strokeWidth="1.8" strokeLinejoin="round"/>
      {data.map((v,i) => i % 4 === 0 && (
        <text key={i} x={x(i)} y={H-8} fill="var(--fg-4)" fontFamily="var(--ff-mono)" fontSize="9.5" textAnchor="middle">{i.toString().padStart(2,"0")}h</text>
      ))}
    </svg>
  );
};

/* ============================== KNOWLEDGE ============================== */
function KnowledgePage() {
  // The full-graph GET drives the default view; once the user submits a
  // mini-cypher query (e.g. `source:linder relation:works_at depth:3`), we
  // POST to /api/knowledge/query and render that result instead.
  const [graph, , refresh] = usePolling("/api/knowledge", 30000);
  const [showAdd, setShowAdd] = useState(false);
  const [showRel, setShowRel] = useState(false);
  const [query, setQuery] = useState("");
  const [submittedQuery, setSubmittedQuery] = useState(null);
  const [serverResult, setServerResult] = useState(null);
  const [queryErr, setQueryErr] = useState(null);

  // Parse the mini-DSL: tokens separated by whitespace, each `key:value`.
  // Supported keys: source, relation, target, depth. Returns
  // {pattern, isStructured}. If no `:` present we treat as substring
  // filter (client-side) and stay on GET.
  const parseMiniCypher = (q) => {
    if (!q.trim()) return { pattern: null, isStructured: false, fallbackText: "" };
    const tokens = q.match(/(\w+):("[^"]*"|\S+)/g);
    if (!tokens || !tokens.length) return { pattern: null, isStructured: false, fallbackText: q };
    const pat = {};
    for (const t of tokens) {
      const [k, vRaw] = t.split(":");
      const v = vRaw.replace(/^"|"$/g, "");
      if (k === "source") pat.source = v;
      else if (k === "target") pat.target = v;
      else if (k === "relation") pat.relation = v;
      else if (k === "depth" || k === "max_depth") pat.max_depth = Number(v);
    }
    return { pattern: pat, isStructured: true, fallbackText: "" };
  };

  const runQuery = async () => {
    setQueryErr(null);
    const parsed = parseMiniCypher(query);
    if (!parsed.isStructured) {
      // No `:` operator — clear server result, let substring filter handle it.
      setSubmittedQuery(null);
      setServerResult(null);
      return;
    }
    try {
      const r = await rhFetch("/api/knowledge/query", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(parsed.pattern),
      });
      setSubmittedQuery(query);
      setServerResult(r);
    } catch (e) {
      setQueryErr(String(e.message || e));
      setServerResult(null);
    }
  };
  const clearQuery = () => { setQuery(""); setSubmittedQuery(null); setServerResult(null); setQueryErr(null); };

  // Decide which source to render: server query result, or full graph with
  // optional substring filter applied.
  const usingServer = !!serverResult;
  const allNodes = (usingServer ? serverResult.nodes : (graph && graph.nodes)) || [];
  const allEdges = (usingServer ? serverResult.edges : (graph && graph.edges)) || [];

  // Substring filter still applies when the query is plain text (no `:`).
  const filterLow = (!usingServer && query && !query.includes(":")) ? query.trim().toLowerCase() : "";
  const nodes = !filterLow ? allNodes
    : allNodes.filter(n => (n.name || "").toLowerCase().includes(filterLow)
        || (n.type || "").toLowerCase().includes(filterLow)
        || (n.id || "").toLowerCase().includes(filterLow));
  const keepIds = new Set(nodes.map(n => n.id));
  const edges = allEdges.filter(e =>
    keepIds.has(e.source || e.source_id) && keepIds.has(e.target || e.target_id)
    || (filterLow && (e.relation || "").toLowerCase().includes(filterLow)));

  const [activeId, setActiveId] = useState(null);
  const active = nodes.find(n => n.id === activeId) || nodes[0] || null;
  const activeEdges = edges.filter(e =>
    e.source === (active && active.id) || e.target === (active && active.id)
    || e.source_id === (active && active.id) || e.target_id === (active && active.id));

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Knowledge graph <span className="dim mono" style={{fontSize:14}}>
            · {filterLow ? `${nodes.length} of ${allNodes.length}` : allNodes.length} nodes
            · {filterLow ? `${edges.length} of ${allEdges.length}` : allEdges.length} edges
            {usingServer && <span> · <span style={{color:"var(--rust)"}}>query mode</span></span>}
          </span></h1>
          <p className="page-sub">
            Backend <span className="mono">/api/knowledge/query</span> for structured queries (<span className="mono">source:foo relation:works_at depth:3</span>); plain text falls back to client-side substring filter.
          </p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
          <div className="search-field" style={{minWidth:340}}>
            <I.search/>
            <input
              placeholder="source:… relation:works_at depth:3 — or plain substring"
              value={query}
              onChange={e => setQuery(e.target.value)}
              onKeyDown={e => { if (e.key === "Enter") runQuery(); }}
            />
            {(query || submittedQuery) && <button className="kbd" onClick={clearQuery} style={{cursor:"pointer"}}>clear</button>}
          </div>
          <button className="btn primary" onClick={runQuery} disabled={!query.trim()}><I.play/> Run</button>
          <button className="btn ghost" onClick={() => setShowAdd(true)}><I.plus/> Add node</button>
          <button className="btn ghost" onClick={() => setShowRel(true)}><I.link/> Add relation</button>
        </div>
      </div>
      {queryErr && (
        <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
          <span className="dot err"/><span className="banner-title">QUERY FAILED</span>
          <span className="banner-body mono" style={{fontSize:11}}>{queryErr}</span>
        </div>
      )}

      <div className="grid-12">
        <div className="col-8 card" style={{padding:0,overflow:"hidden"}}>
          {!graph && <div className="muted mono" style={{padding:"40px", fontSize:12, textAlign:"center"}}>loading graph…</div>}
          {graph && nodes.length === 0 && <div className="muted mono" style={{padding:"40px", fontSize:12, textAlign:"center"}}>No knowledge graph data yet.</div>}
          {graph && nodes.length > 0 && <KGViz nodes={nodes} edges={edges} onSelect={setActiveId} activeId={active && active.id}/>}
        </div>
        <div className="col-4 col">
          <div className="card">
            <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
              {active ? `Node · ${active.type || "entity"} · ${active.name || active.id}` : "Select a node"}
            </div>
            {active && (
              <>
                <div className="row between mb-8">
                  <div className="kv" style={{flex:1}}>
                    <dt>id</dt><dd>{active.id}</dd>
                    <dt>kind</dt><dd>{active.type || "—"}</dd>
                    <dt>degree</dt><dd>{activeEdges.length}</dd>
                  </div>
                  <button
                    className="btn ghost"
                    title="Delete this entity (and all its relations)"
                    onClick={async () => {
                      const ok = await confirmDialog({
                        title: "Delete entity?",
                        body: `This will remove '${active.name || active.id}' and ${activeEdges.length} relation(s).`,
                        confirmLabel: "Delete",
                        danger: true,
                      });
                      if (!ok) return;
                      try {
                        await rhFetch(`/api/knowledge/entities/${encodeURIComponent(active.id)}`, { method: "DELETE" });
                        toastOk("Entity deleted");
                        setActiveId(null);
                        refresh();
                      } catch (err) { toastErr(`Delete failed: ${err.message || err}`); }
                    }}
                  ><I.trash/></button>
                </div>
                <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Edges ({activeEdges.length})</div>
                <div className="col gap-4" style={{maxHeight:240, overflow:"auto"}}>
                  {activeEdges.slice(0, 16).map((e, i) => {
                    const src = e.source || e.source_id;
                    const dst = e.target || e.target_id;
                    const other = src === active.id ? dst : src;
                    const relId = e.id || e.relation_id || "";
                    return (
                      <div key={relId || i} className="row between" style={{padding:"5px 8px",background:"var(--bg-2)",borderRadius:5,fontSize:11.5,fontFamily:"var(--ff-mono)"}}>
                        <span style={{color:"var(--rust)"}}>{e.relation || e.label || "→"}</span>
                        <span className="muted" style={{flex:1, textAlign:"right"}}>→ {nodes.find(n => n.id === other)?.name || other}</span>
                        {relId && (
                          <button
                            className="kbd"
                            style={{marginLeft:6, cursor:"pointer", color:"oklch(0.66 0.18 25)"}}
                            title="Delete this relation"
                            onClick={async (ev) => {
                              ev.stopPropagation();
                              const ok = await confirmDialog({
                                title: "Delete relation?",
                                body: `${e.relation || "→"} between these two entities will be removed.`,
                                confirmLabel: "Delete",
                                danger: true,
                              });
                              if (!ok) return;
                              try {
                                await rhFetch(`/api/knowledge/relations/${encodeURIComponent(relId)}`, { method: "DELETE" });
                                toastOk("Relation deleted");
                                refresh();
                              } catch (err) { toastErr(`Delete failed: ${err.message || err}`); }
                            }}
                          >del</button>
                        )}
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </div>
        </div>
      </div>
      {showAdd && <KnowledgeAddNodeModal onClose={() => setShowAdd(false)} onAdded={() => { setShowAdd(false); refresh(); }}/>}
      {showRel && <KnowledgeAddRelationModal nodes={allNodes} onClose={() => setShowRel(false)} onAdded={() => { setShowRel(false); refresh(); }}/>}
    </div>
  );
}

function KnowledgeAddRelationModal({ nodes, onClose, onAdded }) {
  useEscapeKey(onClose);
  const sortedNodes = React.useMemo(() => (nodes || []).slice().sort((a, b) => (a.name || a.id).localeCompare(b.name || b.id)), [nodes]);
  const [source, setSource] = useState(sortedNodes[0] ? sortedNodes[0].id : "");
  const [target, setTarget] = useState(sortedNodes[1] ? sortedNodes[1].id : (sortedNodes[0] ? sortedNodes[0].id : ""));
  const [relation, setRelation] = useState("works_at");
  const [confidence, setConfidence] = useState("1.0");
  const [propsJson, setPropsJson] = useState("{}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  // RelationType variants — keep in sync with rusty-hand-types/src/memory.rs.
  const RELATIONS = [
    "works_at", "knows_about", "related_to", "depends_on", "owned_by",
    "created_by", "located_in", "part_of", "uses", "produces", "manages",
    "collaborates_with", "mentions", "cites", "implements", "other",
  ];

  const submit = async () => {
    if (!source || !target) { setErr("Both source and target are required"); return; }
    if (source === target) { setErr("Source and target must differ"); return; }
    let properties = {};
    if (propsJson.trim()) {
      try { properties = JSON.parse(propsJson); }
      catch (e) { setErr(`Properties must be valid JSON: ${e.message}`); return; }
      if (typeof properties !== "object" || Array.isArray(properties)) {
        setErr("Properties must be a JSON object"); return;
      }
    }
    const c = Number(confidence);
    if (Number.isNaN(c) || c < 0 || c > 1) { setErr("Confidence must be 0..1"); return; }
    setBusy(true); setErr(null);
    try {
      await rhFetch("/api/knowledge/relations", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source, target, relation, confidence: c, properties }),
      });
      toastOk(`Added ${relation} relation`);
      onAdded();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Add knowledge relation</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          {sortedNodes.length < 2 && (
            <div className="banner mb-12" style={{borderColor:"oklch(0.78 0.14 88 / .35)"}}>
              <span className="dot warn"/>
              <span className="banner-body" style={{fontSize:11.5}}>
                Need at least 2 nodes to create a relation. Add a node first.
              </span>
            </div>
          )}
          <div className="col gap-8">
            <label className="t-row col"><span className="t-lbl">Source</span>
              <select className="t-select" value={source} onChange={(e) => setSource(e.target.value)}>
                {sortedNodes.map(n => <option key={n.id} value={n.id}>{n.name || n.id} <span>({n.type || "?"})</span></option>)}
              </select></label>
            <label className="t-row col">
              <span className="t-lbl">
                Relation
                <Tip>One of the RelationType variants — these are the same labels the kernel uses for graph traversal queries.</Tip>
              </span>
              <select className="t-select" value={relation} onChange={(e) => setRelation(e.target.value)}>
                {RELATIONS.map(r => <option key={r} value={r}>{r}</option>)}
              </select>
            </label>
            <label className="t-row col"><span className="t-lbl">Target</span>
              <select className="t-select" value={target} onChange={(e) => setTarget(e.target.value)}>
                {sortedNodes.map(n => <option key={n.id} value={n.id}>{n.name || n.id} <span>({n.type || "?"})</span></option>)}
              </select></label>
            <label className="t-row col">
              <span className="t-lbl">
                Confidence (0..1)
                <Tip>Float weight on the relation. Used by graph-query ranking. Default 1.0 = certain.</Tip>
              </span>
              <input className="modal-field" type="number" step="0.05" min="0" max="1" value={confidence}
                     onChange={(e) => setConfidence(e.target.value)}/></label>
            <label className="t-row col"><span className="t-lbl">Properties (JSON object)</span>
              <textarea className="modal-field modal-textarea" style={{minHeight:80, fontFamily:"var(--ff-mono)"}}
                        value={propsJson} onChange={(e) => setPropsJson(e.target.value)}/></label>
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy || sortedNodes.length < 2}>
            {busy ? "Adding…" : "Add relation"}
          </button>
        </div>
      </div>
    </div>
  );
}

function KnowledgeAddNodeModal({ onClose, onAdded }) {
  useEscapeKey(onClose);
  const [id, setId] = useState("");
  const [type, setType] = useState("person");
  const [name, setName] = useState("");
  const [propsJson, setPropsJson] = useState("{}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  // EntityType variants from rusty-hand-types/src/memory.rs. Adding a
  // new variant in Rust means appending it here so the dropdown stays
  // in sync.
  const TYPES = ["person", "organization", "project", "concept", "event", "location", "document", "tool", "other"];

  const submit = async () => {
    if (!name.trim()) { setErr("Name required"); return; }
    let properties = {};
    if (propsJson.trim()) {
      try { properties = JSON.parse(propsJson); }
      catch (e) { setErr(`Properties must be valid JSON: ${e.message}`); return; }
      if (typeof properties !== "object" || Array.isArray(properties)) {
        setErr("Properties must be a JSON object");
        return;
      }
    }
    setBusy(true); setErr(null);
    try {
      await rhFetch("/api/knowledge/entities", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: id.trim() || undefined, type, name: name.trim(), properties }),
      });
      toastOk(`Added ${name.trim()}`);
      onAdded();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Add knowledge node</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="col gap-8">
            <label className="t-row col"><span className="t-lbl">Name</span>
              <input className="modal-field" value={name} onChange={(e) => setName(e.target.value)} placeholder="A. Linder" autoFocus/></label>
            <label className="t-row col">
              <span className="t-lbl">
                Type
                <Tip>One of the EntityType variants: person, organization, project, concept, event, location, document, tool, other.</Tip>
              </span>
              <select className="t-select" value={type} onChange={(e) => setType(e.target.value)}>
                {TYPES.map(t => <option key={t} value={t}>{t}</option>)}
              </select>
            </label>
            <label className="t-row col"><span className="t-lbl">ID (optional — kernel assigns UUID if empty)</span>
              <input className="modal-field" value={id} onChange={(e) => setId(e.target.value)} placeholder="p_linder"/></label>
            <label className="t-row col"><span className="t-lbl">Properties (JSON object)</span>
              <textarea className="modal-field modal-textarea" style={{minHeight:100, fontFamily:"var(--ff-mono)"}}
                        value={propsJson} onChange={(e) => setPropsJson(e.target.value)}/></label>
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Adding…" : "Add node"}</button>
        </div>
      </div>
    </div>
  );
}

const KGViz = ({ nodes: rawNodes, edges: rawEdges, onSelect, activeId }) => {
  // Lay out the first N nodes on a circle. Real force-directed layout is
  // a follow-up — for now we just need a legible cluster.
  const N = Math.min(rawNodes.length, 24);
  const cx = 350, cy = 180, r = 130;
  const typeColors = {
    person: "var(--rust)",
    project: "var(--live)",
    file: "var(--sky)",
    company: "var(--violet)",
    document: "var(--fg-3)",
    event: "var(--amber)",
  };
  const nodes = rawNodes.slice(0, N).map((n, i) => ({
    id: n.id,
    x: cx + r * Math.cos((i / N) * Math.PI * 2 - Math.PI / 2),
    y: cy + r * Math.sin((i / N) * Math.PI * 2 - Math.PI / 2),
    r: 11,
    label: (n.name || n.id || "").slice(0, 14),
    c: typeColors[(n.type || "").toLowerCase()] || "var(--rust)",
  }));
  // Normalize edges to source/target id pairs.
  const edges = rawEdges
    .map(e => [e.source || e.source_id, e.target || e.target_id])
    .filter(([a, b]) => a && b);
  const by = {}; nodes.forEach(n => by[n.id] = n);
  return (
    <div style={{height:420, position:"relative",
      background:`radial-gradient(600px 320px at 50% 40%, oklch(0.665 0.165 50 / .08), transparent 60%),
                  linear-gradient(var(--border) 1px, transparent 1px) 0 0/24px 24px,
                  linear-gradient(90deg, var(--border) 1px, transparent 1px) 0 0/24px 24px,
                  var(--bg-2)`}}>
      <svg viewBox="0 0 700 360" width="100%" height="100%">
        {edges.map(([a,b],i) => {
          if (!by[a] || !by[b]) return null;
          return <line key={i} x1={by[a].x} y1={by[a].y} x2={by[b].x} y2={by[b].y}
                stroke="var(--border-hi)" strokeWidth="1" opacity=".8"/>;
        })}
        {nodes.map(n => {
          const isActive = n.id === activeId;
          return (
            <g key={n.id} style={{cursor: onSelect ? "pointer" : undefined}} onClick={() => onSelect && onSelect(n.id)}>
              <circle cx={n.x} cy={n.y} r={n.r + (isActive ? 8 : 5)} fill={n.c} opacity={isActive ? ".22" : ".12"}/>
              <circle cx={n.x} cy={n.y} r={n.r} fill="var(--bg)" stroke={n.c} strokeWidth={isActive ? "3" : "2"}/>
              <text x={n.x} y={n.y + n.r + 12} textAnchor="middle"
                    fill="var(--fg-2)" fontFamily="var(--ff-mono)" fontSize="10.5">{n.label}</text>
            </g>
          );
        })}
      </svg>
      <div style={{position:"absolute",bottom:10,left:14,fontFamily:"var(--ff-mono)",fontSize:10.5,color:"var(--fg-4)"}}>
        {rawNodes.length} nodes · {rawEdges.length} edges · live from <span style={{color:"var(--rust)"}}>/api/knowledge</span>
      </div>
    </div>
  );
};

/* ============================== SKILLS ============================== */
function SkillsPage() {
  const [skillsResp, , refresh] = usePolling("/api/skills", 30000);
  const skills = Array.isArray(skillsResp) ? skillsResp : (skillsResp && skillsResp.skills) || [];
  const [showCustom, setShowCustom] = useState(false);
  const [showClawHub, setShowClawHub] = useState(false);
  const [rowMenu, setRowMenu] = useState(null);
  const [inspect, setInspect] = useState(null);

  const uninstall = async (name) => {
    if (!(await confirmDialog({ title: "Uninstall skill", message: `Uninstall skill ${name}?`, danger: true, confirmLabel: "Uninstall" }))) return;
    try {
      await rhFetch("/api/skills/uninstall", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name }),
      });
      refresh();
    } catch (e) { toastErr(`uninstall failed: ${e.message || e}`); }
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Skills <span className="dim mono" style={{fontSize:14}}>· {skills.length}</span></h1>
          <p className="page-sub">Bundled + <span className="mono" style={{color:"var(--rust)"}}>ClawHub</span> marketplace · WASM sandbox · capability gating</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}>Reload</button>
          <button className="btn ghost" onClick={() => setShowCustom(true)}><I.plus/> Install custom</button>
          <button className="btn primary" onClick={() => setShowClawHub(true)}><I.plus/> ClawHub</button>
        </div>
      </div>

      <div className="card flush">
        <table className="tbl">
          <thead><tr><th>Skill</th><th>Origin</th><th>Runtime</th><th>Version</th><th>Enabled</th><th></th></tr></thead>
          <tbody>
            {!skillsResp && (<tr><td colSpan={6} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
            {skillsResp && skills.length === 0 && (
              <tr><td colSpan={6} style={{padding:"24px 14px", textAlign:"center"}}>
                <div className="dim" style={{fontSize:12, marginBottom:8}}>No skills installed yet — bundled tools work without registration.</div>
                <span className="row gap-6" style={{justifyContent:"center"}}>
                  <button className="btn sm" onClick={() => setShowCustom(true)}><I.plus/> Custom</button>
                  <button className="btn primary sm" onClick={() => setShowClawHub(true)}><I.plus/> Browse ClawHub</button>
                </span>
              </td></tr>
            )}
            {skills.map(s => {
              const origin = (s.source || s.origin || (s.privileged ? "privileged" : "builtin")).toString().toLowerCase();
              const cat = s.category || s.runtime || s.type || "—";
              const ver = s.version || "—";
              const en = s.enabled !== false;
              const isBundled = origin === "bundled" || origin === "builtin";
              return (
                <tr key={s.name} style={{cursor:"pointer"}} onClick={() => setInspect(s)}>
                  <td><span className="mono"><span style={{color:"var(--rust)"}}>›</span> {s.name}</span></td>
                  <td>
                    {origin === "clawhub" || origin === "claw" ? <span className="badge violet">ClawHub</span>
                      : origin === "privileged" ? <span className="badge warn">privileged</span>
                      : <span className="badge plain">{origin}</span>}
                  </td>
                  <td className="muted mono">{s.runtime || cat}</td>
                  <td className="mono">{ver}</td>
                  <td><div className={"switch " + (en ? "on" : "")}/></td>
                  <td className="right" style={{position:"relative"}} onClick={(e) => e.stopPropagation()}>
                    <button className="btn sm ghost" onClick={() => setRowMenu(rowMenu === s.name ? null : s.name)}><I.more/></button>
                    {rowMenu === s.name && (
                      <div className="row-menu" onClick={e => e.stopPropagation()}>
                        {isBundled
                          ? <button disabled title="bundled skills cannot be uninstalled">Bundled — cannot remove</button>
                          : <button onClick={() => { setRowMenu(null); uninstall(s.name); }} style={{color:"var(--crimson)"}}><I.close/> Uninstall</button>}
                      </div>
                    )}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {showCustom && <SkillInstallModal onClose={() => setShowCustom(false)} onInstalled={() => { setShowCustom(false); refresh(); }}/>}
      {showClawHub && <ClawHubModal onClose={() => setShowClawHub(false)} onInstalled={() => { setShowClawHub(false); refresh(); }}/>}
      {inspect && <SkillDetailModal skill={inspect} onClose={() => setInspect(null)} onUninstall={uninstall}/>}
    </div>
  );
}

// Skill detail modal — surfaces every field the list endpoint exposes,
// plus a quick "Recent invocations" list scoped to this skill via audit
// substring (best-effort: the audit action format isn't pinned, so we
// match on the skill name appearing in `action`).
function SkillDetailModal({ skill, onClose, onUninstall }) {
  useEscapeKey(onClose);
  const s = skill || {};
  const source = (s.source && typeof s.source === "object") ? s.source : { type: s.source || "—" };
  const sourceType = (source.type || "").toLowerCase();
  const isBundled = sourceType === "bundled" || sourceType === "builtin";
  const [audit] = useApi(s.name ? `/api/audit/recent?n=50` : null);
  const recent = ((audit && audit.entries) || [])
    .filter(e => (e.action || "").toLowerCase().includes(String(s.name).toLowerCase()))
    .slice(0, 8);
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <b className="mono">{s.name}</b>
            <div className="dim mono" style={{fontSize:11, marginTop:2}}>{s.author || "—"} · v{s.version || "—"}</div>
          </div>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="row gap-8 mb-12">
            {sourceType === "clawhub" && <span className="badge violet">ClawHub</span>}
            {sourceType === "openclaw" && <span className="badge violet">OpenClaw</span>}
            {isBundled && <span className="badge plain">bundled</span>}
            {!isBundled && sourceType === "local" && <span className="badge plain">local</span>}
            <span className="badge plain">{s.runtime || "—"}</span>
            <span className={"badge " + (s.enabled !== false ? "live" : "idle")}>{s.enabled !== false ? "enabled" : "disabled"}</span>
            {s.has_prompt_context && <span className="badge sky">prompt context</span>}
          </div>
          <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Description</div>
          <div className="codebox mb-16" style={{whiteSpace:"pre-wrap"}}>{s.description || "(no description)"}</div>
          <div className="kv mb-16">
            <dt>tools exposed</dt><dd>{s.tools_count != null ? s.tools_count : "—"}</dd>
            <dt>tags</dt><dd className="mono">{(s.tags && s.tags.length) ? s.tags.join(", ") : "—"}</dd>
            {source.slug && <><dt>clawhub slug</dt><dd className="mono">{source.slug}</dd></>}
            {source.version && <><dt>clawhub version</dt><dd className="mono">{source.version}</dd></>}
          </div>
          <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
            Recent invocations <span className="dim" style={{marginLeft:6, fontSize:10}}>(audit substring match)</span>
          </div>
          <div className="col gap-4" style={{maxHeight:200, overflow:"auto"}}>
            {!audit && <div className="dim mono" style={{fontSize:11, padding:"6px 8px"}}>loading audit…</div>}
            {audit && recent.length === 0 && <div className="dim mono" style={{fontSize:11, padding:"6px 8px"}}>No recent invocations in the loaded window.</div>}
            {recent.map((e, i) => (
              <div key={e.hash || e.seq || i} className="row" style={{padding:"5px 8px", background:"var(--bg-2)", borderRadius:5}}>
                <span className="mono dim" style={{fontSize:11, width:70}}>{formatTime(e.timestamp)}</span>
                <span className="mono" style={{fontSize:12, flex:1}}>{e.action}</span>
                <span className="mono dim" style={{fontSize:11}}>{e.agent_name || e.agent_id || "—"}</span>
              </div>
            ))}
          </div>
        </div>
        <div className="modal-foot">
          {!isBundled && (
            <button className="btn danger" onClick={() => onUninstall(s.name).then(onClose)} style={{marginRight:"auto"}}>
              <I.close/> Uninstall
            </button>
          )}
          <button className="btn primary" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

function ClawHubModal({ onClose, onInstalled }) {
  useEscapeKey(onClose);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("trending");
  const path = query.trim()
    ? `/api/clawhub/search?q=${encodeURIComponent(query)}&limit=30`
    : `/api/clawhub/browse?sort=${encodeURIComponent(sort)}&limit=30`;
  const [resp, fetchErr, refresh] = useApi(path);
  const items = (resp && resp.items) || [];
  const [installing, setInstalling] = useState(null);
  const [result, setResult] = useState(null);
  // Peek state: slug currently expanded, plus a per-slug cache of the
  // detail response so toggling open/closed doesn't re-fetch.
  const [peekSlug, setPeekSlug] = useState(null);
  const [peekCache, setPeekCache] = useState({});
  const [peekErr, setPeekErr] = useState(null);
  const peek = async (slug) => {
    if (peekSlug === slug) { setPeekSlug(null); return; }
    setPeekSlug(slug); setPeekErr(null);
    if (peekCache[slug]) return;
    try {
      const r = await rhFetch(`/api/clawhub/skill/${encodeURIComponent(slug)}`);
      setPeekCache(prev => ({ ...prev, [slug]: r }));
    } catch (e) {
      setPeekErr(String(e.message || e));
    }
  };

  const install = async (slug) => {
    setInstalling(slug); setResult(null);
    try {
      const r = await rhFetch("/api/clawhub/install", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ slug }),
      });
      setResult({ slug, ok: true, message: (r && (r.message || r.status)) || "Installed" });
      onInstalled();
    } catch (e) {
      setResult({ slug, ok: false, message: String(e.message || e) });
    } finally { setInstalling(null); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">ClawHub marketplace</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="row gap-8 mb-12">
            <div className="search-field" style={{flex:1}}>
              <I.search/>
              <input placeholder="Search skills (empty = browse trending)…" value={query} onChange={e => setQuery(e.target.value)}/>
            </div>
            {!query.trim() && (
              <select className="t-select" value={sort} onChange={e => setSort(e.target.value)}>
                {["trending","downloads","stars","updated","rating"].map(s => <option key={s} value={s}>{s}</option>)}
              </select>
            )}
            <button className="btn ghost" onClick={refresh}><I.refresh/></button>
          </div>
          {fetchErr && <div className="banner mb-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{fetchErr}</span>
          </div>}
          {!resp && !fetchErr && <div className="dim mono" style={{fontSize:11.5, padding:"12px"}}>loading…</div>}
          {resp && items.length === 0 && <div className="dim mono" style={{fontSize:11.5, padding:"12px"}}>No skills found.</div>}
          <div className="col gap-6" style={{maxHeight:420, overflow:"auto"}}>
            {items.map(it => {
              const slug = it.slug || it.id || it.name;
              const isInstalling = installing === slug;
              const ok = result && result.slug === slug && result.ok;
              const err = result && result.slug === slug && !result.ok;
              const isOpen = peekSlug === slug;
              const detail = peekCache[slug];
              return (
                <div key={slug} style={{padding:"10px 12px", border:"1px solid var(--border)", borderRadius:7, background:"var(--bg-2)"}}>
                  <div className="row gap-12">
                    <div className="col" style={{flex:1, gap:3, minWidth:0}}>
                      <div className="row gap-8">
                        <span className="mono" style={{fontSize:13}}>{it.name || slug}</span>
                        {it.version && <span className="badge plain">{it.version}</span>}
                        {it.author && <span className="dim mono" style={{fontSize:10.5}}>{it.author}</span>}
                      </div>
                      <span className="dim" style={{fontSize:11.5, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{it.description || it.summary || "—"}</span>
                      <div className="row gap-12 dim mono" style={{fontSize:10.5}}>
                        {it.downloads != null && <span>↓ {Number(it.downloads).toLocaleString()}</span>}
                        {it.stars != null && <span>★ {it.stars}</span>}
                        {it.rating != null && <span>{Number(it.rating).toFixed(1)}/5</span>}
                        {it.updated && <span>upd {relativeTime(it.updated)}</span>}
                      </div>
                    </div>
                    <div className="row gap-6">
                      {ok && <span className="badge live">installed</span>}
                      {err && <span className="badge error" title={result.message}>failed</span>}
                      <button className="btn sm" onClick={() => peek(slug)} title="Peek manifest without installing">
                        {isOpen ? "Hide" : "Peek"}
                      </button>
                      <button className="btn sm primary" onClick={() => install(slug)} disabled={!!installing}>
                        {isInstalling ? "Installing…" : "Install"}
                      </button>
                    </div>
                  </div>
                  {isOpen && (
                    <div style={{
                      marginTop:10,
                      padding:"10px 12px",
                      background:"var(--surface)",
                      borderRadius:5,
                      border:"1px solid var(--border)",
                    }}>
                      {!detail && !peekErr && (
                        <div className="dim mono" style={{fontSize:11, textAlign:"center"}}>loading…</div>
                      )}
                      {peekErr && (
                        <div className="dim mono" style={{fontSize:11, color:"var(--crimson)"}}>{peekErr}</div>
                      )}
                      {detail && (
                        <>
                          {detail.description && (
                            <div className="dim" style={{fontSize:11.5, lineHeight:1.5, marginBottom:8}}>
                              {detail.description}
                            </div>
                          )}
                          {Array.isArray(detail.tools) && detail.tools.length > 0 && (
                            <>
                              <div className="muted mono mb-4" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
                                Tools <span className="dim" style={{marginLeft:6}}>{detail.tools.length}</span>
                              </div>
                              <div className="col gap-4" style={{maxHeight:160, overflow:"auto"}}>
                                {detail.tools.map((t, i) => (
                                  <div key={i} className="mono" style={{
                                    fontSize:11,
                                    padding:"3px 6px",
                                    background:"var(--bg-2)",
                                    borderRadius:4,
                                  }}>
                                    <span style={{color:"var(--rust)"}}>{t.name || t.id || `tool-${i}`}</span>
                                    {t.description && <span className="dim" style={{marginLeft:8}}>{t.description}</span>}
                                  </div>
                                ))}
                              </div>
                            </>
                          )}
                          {Array.isArray(detail.capabilities) && detail.capabilities.length > 0 && (
                            <div className="mt-8">
                              <div className="muted mono mb-4" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Capabilities</div>
                              <div className="row gap-4" style={{flexWrap:"wrap"}}>
                                {detail.capabilities.map((c, i) => (
                                  <span key={i} className="badge plain" style={{fontSize:10}}>{c}</span>
                                ))}
                              </div>
                            </div>
                          )}
                          {detail.homepage && (
                            <div className="mt-8 dim mono" style={{fontSize:10.5}}>
                              <span>homepage: </span>
                              <a href={detail.homepage} target="_blank" rel="noreferrer" style={{color:"var(--rust)"}}>{detail.homepage}</a>
                            </div>
                          )}
                        </>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
          {result && !result.ok && (
            <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
              <span className="dot err"/><span className="banner-title">FAIL</span>
              <span className="banner-body mono" style={{fontSize:11}}>{result.message}</span>
            </div>
          )}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}

function SkillInstallModal({ onClose, onInstalled }) {
  useEscapeKey(onClose);
  const [name, setName] = useState("");
  const [language, setLanguage] = useState("python");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState(`def run(input):
    """Echo skill — returns its input."""
    return {"echo": input}
`);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);

  const submit = async () => {
    if (!/^[a-z][a-z0-9_]{0,63}$/.test(name)) {
      setErr("name must match ^[a-z][a-z0-9_]{0,63}$");
      return;
    }
    if (!content.trim()) { setErr("content is empty"); return; }
    setBusy(true); setErr(null);
    try {
      await rhFetch("/api/skills/install-custom", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, language, description, content }),
      });
      onInstalled();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal wide" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Install custom skill</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <div className="col gap-8">
            <label className="t-row col"><span className="t-lbl">Name (^[a-z][a-z0-9_]{"{0,63}"}$)</span>
              <input className="modal-field" value={name} onChange={e => setName(e.target.value)} placeholder="echo_skill"/></label>
            <label className="t-row col"><span className="t-lbl">Language</span>
              <select className="t-select" value={language} onChange={e => setLanguage(e.target.value)}>
                <option value="python">python</option>
                <option value="javascript">javascript (node)</option>
              </select></label>
            <label className="t-row col"><span className="t-lbl">Description</span>
              <input className="modal-field" value={description} onChange={e => setDescription(e.target.value)}/></label>
            <label className="t-row col"><span className="t-lbl">Body (define `run(input)`; wrapper boilerplate auto-added)</span>
              <textarea className="modal-field modal-textarea" style={{minHeight:240, fontFamily:"var(--ff-mono)"}} value={content} onChange={e => setContent(e.target.value)}/></label>
          </div>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={submit} disabled={busy}>{busy ? "Installing…" : "Install"}</button>
        </div>
      </div>
    </div>
  );
}

/* ============================== APPROVALS ============================== */
function ApprovalsPage() {
  const [resp, , refresh] = usePolling("/api/approvals", 10000);
  const [inspect, setInspect] = useState(null);
  const [selected, setSelected] = useState(() => new Set());
  const rows = (resp && resp.approvals) || [];
  // Drop selections whose row disappeared (decided elsewhere, cleared on
  // refresh) so the bulk-bar count stays honest.
  React.useEffect(() => {
    if (selected.size === 0) return;
    const live = new Set(rows.map(r => r.id));
    const next = new Set([...selected].filter(id => live.has(id)));
    if (next.size !== selected.size) setSelected(next);
  }, [rows.map(r => r.id).join(",")]);
  const toggle = (id) => {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id); else next.add(id);
      return next;
    });
  };
  const toggleAll = (visible) => {
    setSelected(prev => {
      if (visible.every(r => prev.has(r.id))) {
        const next = new Set(prev);
        for (const r of visible) next.delete(r.id);
        return next;
      }
      const next = new Set(prev);
      for (const r of visible) next.add(r.id);
      return next;
    });
  };
  const bulkDecide = async (verdict) => {
    const ids = [...selected];
    if (ids.length === 0) return;
    const ok = await confirmDialog({
      title: `${verdict === "approve" ? "Approve" : "Reject"} ${ids.length} request(s)?`,
      message: `${verdict === "approve" ? "Approving" : "Rejecting"} all selected approvals. Decisions are recorded in the audit chain and cannot be undone.`,
      danger: verdict === "reject",
      confirmLabel: verdict === "approve" ? `Approve ${ids.length}` : `Reject ${ids.length}`,
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/approvals/${id}/${verdict}`, { method: "POST" });
        okCount++;
      } catch (_) { failCount++; }
    }
    setSelected(new Set());
    if (failCount > 0) toastErr(`${verdict}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${verdict === "approve" ? "Approved" : "Rejected"} ${okCount} request(s)`);
    refresh();
  };
  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Approvals <span className="dim mono" style={{fontSize:14}}>· {resp ? rows.length : "…"}</span></h1>
          <p className="page-sub">Inline keyboard buttons push to bound channels · decisions written to audit chain</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
        </div>
      </div>
      {selected.size > 0 && (
        <div className="bulk-bar">
          <span className="mono" style={{fontSize:12}}>{selected.size} selected</span>
          <button className="btn sm primary" onClick={() => bulkDecide("approve")}>
            <I.check/> Approve {selected.size}
          </button>
          <button className="btn sm danger" onClick={() => bulkDecide("reject")}>
            <I.close/> Reject {selected.size}
          </button>
          <button className="btn sm ghost" onClick={() => setSelected(new Set())} style={{marginLeft:"auto"}}>Clear</button>
        </div>
      )}
      <div className="card flush">
        {!resp && <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>loading…</div>}
        {resp && rows.length === 0 && <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>No approvals waiting.</div>}
        {rows.length > 0 && (
          <ApprovalsTable rows={rows} onChange={refresh} onInspect={setInspect}
                          selectable selected={selected} onToggle={toggle} onToggleAll={toggleAll}/>
        )}
      </div>
      {inspect && <ApprovalContextModal approval={inspect} onClose={() => setInspect(null)} onChange={() => { setInspect(null); refresh(); }}/>}
    </div>
  );
}

// Context modal opened by clicking an approval row. Shows the full
// payload as pretty-printed JSON + risk + age + decision buttons. The
// existing ApprovalsTable already has inline Approve/Reject buttons, but
// those don't surface what's actually being decided. This modal is the
// place to read the full request before acting on it.
function ApprovalContextModal({ approval, onClose, onChange }) {
  useEscapeKey(onClose);
  const r = approval || {};
  const decide = async (verdict) => {
    try {
      await rhFetch(`/api/approvals/${r.id}/${verdict}`, { method: "POST" });
      toastOk(`Approval ${verdict}d`);
      onChange();
    } catch (e) { toastErr(`${verdict} failed: ${e.message || e}`); }
  };
  const payload = r.payload || r.details || r.context;
  const payloadStr = payload != null
    ? (typeof payload === "string" ? payload : JSON.stringify(payload, null, 2))
    : null;
  const risk = (r.risk || "low").toLowerCase();
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <div>
            <b className="mono">Approval · {r.id}</b>
            <div className="dim mono" style={{fontSize:11, marginTop:2}}>
              {(r.agent_name || r.agent || r.agent_id || "agent")} · requested {relativeTime(r.requested_at || r.created_at)}
            </div>
          </div>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="row gap-12 mb-12">
            <span className={"badge " + (risk === "high" ? "error" : risk === "medium" ? "warn" : "idle")}>{risk} risk</span>
            {r.status && <span className="badge plain">{r.status}</span>}
          </div>
          <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Action</div>
          <div className="codebox mb-16" style={{whiteSpace:"pre-wrap"}}>{r.action || "—"}</div>
          {payloadStr && (
            <>
              <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Payload</div>
              <pre className="codebox mb-16" style={{maxHeight:240}}>{payloadStr}</pre>
            </>
          )}
          {r.reason && (
            <>
              <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Reason</div>
              <div className="codebox mb-16" style={{whiteSpace:"pre-wrap"}}>{r.reason}</div>
            </>
          )}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose} style={{marginRight:"auto"}}>Close</button>
          <button className="btn danger" onClick={() => decide("reject")}>Reject</button>
          <button className="btn primary" onClick={() => decide("approve")}>Approve</button>
        </div>
      </div>
    </div>
  );
}

/* ============================== AUDIT ============================== */
function AuditPage() {
  const route = useHashRoute();
  // Read ?h=<hashPrefix> from the URL hash to support deep-links from
  // other pages (or shared URLs). When present, the matching entry gets
  // scrolled into view + highlighted for 3s, then the highlight fades.
  const focusHash = (route && route.query && route.query.h) ? route.query.h : "";
  // Persisted window size — operators investigating an incident often
  // want a larger window; the kernel caps n=1000 on the server side.
  const [windowSize, setWindowSize] = useState(() => {
    try {
      const stored = parseInt(localStorage.getItem("rh.panel.auditWindow") || "200", 10);
      return [50, 200, 500, 1000].includes(stored) ? stored : 200;
    } catch (e) { return 200; }
  });
  const setWindow = (n) => {
    setWindowSize(n);
    try { localStorage.setItem("rh.panel.auditWindow", String(n)); } catch (e) {}
  };
  const [audit, , refresh] = usePolling(`/api/audit/recent?n=${windowSize}`, 8000);
  const [verify, verifyErr, verifyRefresh] = useApi("/api/audit/verify");
  // Track an explicit user-triggered re-verify so the UI can show
  // "verifying…" and a wall-clock timestamp of the last manual check.
  const [verifyingNow, setVerifyingNow] = useState(false);
  const [lastVerifiedAt, setLastVerifiedAt] = useState(null);
  const forceReverify = async () => {
    setVerifyingNow(true);
    try {
      await verifyRefresh();
      setLastVerifiedAt(new Date());
    } finally {
      setVerifyingNow(false);
    }
  };
  // Toast on each verify response so the operator gets explicit feedback
  // even when the result happened to match the previous one (no banner
  // diff to notice). Guard against initial mount with a ref.
  const verifyToastedRef = React.useRef(null);
  React.useEffect(() => {
    if (!verify) return;
    if (!lastVerifiedAt) return; // only toast user-triggered runs, not first auto-fetch
    const key = `${verify.valid}-${verify.entries}-${lastVerifiedAt.getTime()}`;
    if (verifyToastedRef.current === key) return;
    verifyToastedRef.current = key;
    if (verify.valid) {
      toastOk(`Chain verified · ${verify.entries || 0} entries`);
    } else {
      toastErr(`Chain MISMATCH: ${verify.error || "see banner"}`);
    }
  }, [verify, lastVerifiedAt]);
  const [q, setQ] = useState("");
  // `pulse` is the hash prefix currently flashing in the list — set
  // by the route's ?h= query, cleared after 3.5s so the row relaxes.
  // Named differently from the search-highlight() function below to
  // avoid a shadowing footgun.
  const [pulse, setPulse] = useState(focusHash);
  React.useEffect(() => {
    if (!pulse) return;
    const id = setTimeout(() => setPulse(""), 3500);
    return () => clearTimeout(id);
  }, [pulse]);
  React.useEffect(() => { setPulse(focusHash); }, [focusHash]);
  const rowRefs = React.useRef({});
  React.useEffect(() => {
    if (!pulse || !audit) return;
    const found = Object.entries(rowRefs.current)
      .find(([h]) => h && h.startsWith(pulse));
    const el = found && found[1];
    if (el && typeof el.scrollIntoView === "function") {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [pulse, audit]);

  const copyDeepLink = (hash) => {
    if (!hash) return;
    const link = `${window.location.origin}${window.location.pathname}#/audit?h=${encodeURIComponent(String(hash).slice(0, 12))}`;
    try {
      navigator.clipboard.writeText(link);
      toastOk("Link copied to clipboard");
    } catch (e) { toastErr(`copy failed: ${e.message || e}`); }
  };

  // Level filter chips (info/warn/error/all). Persisted in localStorage so
  // operators investigating an incident return to the same filter on
  // refresh. Chip choice is applied *before* the substring filter, so
  // "level=error and q=panic" works the way you'd expect.
  const [levelFilter, setLevelFilterState] = useState(() => {
    try {
      const stored = localStorage.getItem("rh.panel.auditLevel") || "all";
      return ["all", "info", "warn", "error"].includes(stored) ? stored : "all";
    } catch (e) { return "all"; }
  });
  const setLevelFilter = (lvl) => {
    setLevelFilterState(lvl);
    try { localStorage.setItem("rh.panel.auditLevel", lvl); } catch (e) {}
  };

  const rawEntries = (audit && audit.entries) || [];
  // Per-level counts derived from the unfiltered window so chip labels
  // show "warn · 8" even when the active filter would exclude them.
  const levelCounts = { info: 0, warn: 0, error: 0 };
  for (const e of rawEntries) {
    const lvl = auditLevelOf(e);
    levelCounts[lvl] = (levelCounts[lvl] || 0) + 1;
  }
  const levelFiltered = levelFilter === "all"
    ? rawEntries
    : rawEntries.filter(e => auditLevelOf(e) === levelFilter);

  // Client-side substring filter across the loaded window. Real
  // full-text search would need a server-side index; for typical ops
  // windows of ~200 entries this is plenty fast and renders instantly.
  const ql = q.trim().toLowerCase();
  const entries = !ql ? levelFiltered : levelFiltered.filter(e =>
    (e.action || "").toLowerCase().includes(ql) ||
    (e.agent_name || "").toLowerCase().includes(ql) ||
    (e.agent_id || "").toLowerCase().includes(ql) ||
    (e.detail || "").toLowerCase().includes(ql) ||
    (e.outcome || "").toLowerCase().includes(ql) ||
    (e.hash || "").toLowerCase().includes(ql)
  );

  const actorCounts = {};
  for (const e of entries) {
    const a = e.agent_name || e.agent_id || "kernel";
    actorCounts[a] = (actorCounts[a] || 0) + 1;
  }
  const topActor = Object.entries(actorCounts).sort((a, b) => b[1] - a[1])[0];

  // Highlight matches inline. Tiny helper — splits a string around the
  // (case-insensitive) needle so we can wrap matches in <mark>.
  const highlight = React.useCallback((text) => {
    if (!ql || !text) return text;
    const s = String(text);
    const idx = s.toLowerCase().indexOf(ql);
    if (idx < 0) return text;
    return (
      <>
        {s.slice(0, idx)}
        <mark className="audit-match">{s.slice(idx, idx + ql.length)}</mark>
        {s.slice(idx + ql.length)}
      </>
    );
  }, [ql]);

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Audit log</h1>
          <p className="page-sub">Merkle hash chain · <span className="mono">~/.rustyhand/data/audit.jsonl</span> · replayed on boot</p>
        </div>
        <div className="page-actions">
          <div className="search-field" style={{minWidth:260}}>
            <I.search/>
            <input placeholder="filter by action / actor / hash / detail…" value={q} onChange={e => setQ(e.target.value)}/>
            {q && <button className="kbd" onClick={() => setQ("")} style={{cursor:"pointer"}}>clear</button>}
          </div>
          <div className="seg" title="Filter by classified severity">
            <button className={levelFilter === "all" ? "on" : ""} onClick={() => setLevelFilter("all")}>
              all · {rawEntries.length}
            </button>
            <button className={levelFilter === "info" ? "on" : ""} onClick={() => setLevelFilter("info")}>
              info · {levelCounts.info}
            </button>
            <button className={levelFilter === "warn" ? "on" : ""}
                    style={levelFilter === "warn" ? {color:"var(--amber)"} : {}}
                    onClick={() => setLevelFilter("warn")}>
              warn · {levelCounts.warn}
            </button>
            <button className={levelFilter === "error" ? "on" : ""}
                    style={levelFilter === "error" ? {color:"var(--crimson)"} : {}}
                    onClick={() => setLevelFilter("error")}>
              error · {levelCounts.error}
            </button>
          </div>
          <div className="seg" title="Audit window size">
            {[50, 200, 500, 1000].map(n => (
              <button key={n} className={windowSize === n ? "on" : ""} onClick={() => setWindow(n)}>{n}</button>
            ))}
          </div>
          <button className="btn ghost" onClick={() => { refresh(); verifyRefresh(); }}><I.refresh/></button>
          <button className="btn ghost" onClick={forceReverify} disabled={verifyingNow}
                  title="Force re-verify the audit chain end-to-end and toast the result">
            <I.shield/> {verifyingNow ? "Verifying…" : "Verify chain"}
          </button>
          <button className="btn ghost"
                  title={ql ? `Export ${entries.length} filtered entries as CSV` : "Export the loaded audit window as CSV"}
                  onClick={() => {
                    if (entries.length === 0) { toastErr("Nothing to export"); return; }
                    const rows = entries.map(e => ({
                      seq: e.seq != null ? e.seq : "",
                      timestamp: e.timestamp || e.created_at || "",
                      agent_id: e.agent_id || "",
                      agent_name: e.agent_name || "",
                      action: e.action || "",
                      outcome: e.outcome || "",
                      detail: (e.detail || "").replace(/\s+/g, " ").trim(),
                      hash: e.hash || "",
                    }));
                    const csv = rowsToCsv(rows);
                    const tag = ql ? "filtered" : "window";
                    downloadBlob(
                      `rustyhand-audit-${tag}-${new Date().toISOString().slice(0, 10)}.csv`,
                      csv,
                      "text/csv",
                    );
                  }}><I.download/> CSV</button>
          <button className="btn ghost" onClick={() => {
            if (!audit) return;
            downloadBlob(`rustyhand-audit-${new Date().toISOString().slice(0, 10)}.json`,
              JSON.stringify(audit, null, 2), "application/json");
          }}><I.download/> JSON</button>
        </div>
      </div>

      <div className="grid-12">
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Chain head</div>
          <div className="mono" style={{fontSize:14, color:"var(--rust)", wordBreak:"break-all"}}>{audit && audit.tip_hash ? String(audit.tip_hash).slice(0, 16) : "—"}</div>
          <div className="dim mono mt-4" style={{fontSize:11}}>depth {audit ? (audit.total != null ? audit.total.toLocaleString() : "—") : "…"}</div>
          <div className="divider"/>
          {verify ? (
            <>
              <div className="row gap-6">
                {verify.valid
                  ? <span className="badge live"><I.check/> verified</span>
                  : <span className="badge error"><I.warn/> mismatch</span>}
                <span className="dim mono" style={{fontSize:11}}>{(verify.entries || []).length || verify.total || 0} entries</span>
              </div>
              {lastVerifiedAt && (
                <div className="dim mono mt-4" style={{fontSize:10.5}}>
                  last manual check: {lastVerifiedAt.toLocaleTimeString("en-GB", { hour12: false })}
                </div>
              )}
              {!verify.valid && verify.error && (
                <div className="mono mt-4" style={{fontSize:10.5, color:"var(--crimson)", wordBreak:"break-word"}}>
                  {verify.error}
                </div>
              )}
            </>
          ) : <div className="dim mono" style={{fontSize:11}}>{verifyErr || "verifying…"}</div>}
        </div>
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Loaded window</div>
          <div className="mono" style={{fontSize:20}}>{entries.length} <span className="dim" style={{fontSize:13}}>entries</span></div>
          <div className="dim mono mt-4" style={{fontSize:11}}>from /api/audit/recent?n={windowSize}</div>
        </div>
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Top actor</div>
          <div className="mono" style={{fontSize:14}}>{topActor ? topActor[0] : "—"}</div>
          <div className="dim mono mt-4" style={{fontSize:11}}>{topActor ? `${topActor[1]} entries (in window)` : "no activity"}</div>
        </div>
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Warning</div>
          <div className="mono" style={{fontSize:14, color: verify && verify.warning ? "var(--amber)" : "var(--live)"}}>
            {verify && verify.warning ? "see below" : "none"}
          </div>
          <div className="dim mono mt-4" style={{fontSize:11}}>{verify && verify.warning ? verify.warning : "audit chain stable"}</div>
        </div>
      </div>

      <div className="card flush mt-16">
        <div className="card-head"><span>Chain · most recent</span><span className="mono dim">descending</span></div>
        <div>
          {!audit && Array.from({length:6}).map((_,i) => (
            <div key={`sa-${i}`} className="merkle-row">
              <div className="chain"/>
              <Skel w={50} h={10}/>
              <span><Skel w="50%" h={10}/><div style={{marginTop:4}}><Skel w="70%" h={9}/></div></span>
              <Skel w={70} h={10}/>
              <Skel w={50} h={10}/>
            </div>
          ))}
          {audit && entries.length === 0 && (
            <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>
              {ql ? `No entries matching "${q}" in the loaded window of ${rawEntries.length}.` : "No audit entries yet."}
            </div>
          )}
          {entries.map((a) => {
            const hash = a.hash ? String(a.hash).slice(0, 12) : "—";
            const pulsing = pulse && a.hash && String(a.hash).startsWith(pulse);
            return (
              <div key={a.hash || a.seq}
                   ref={(el) => { if (a.hash) rowRefs.current[a.hash] = el; }}
                   className={"merkle-row" + (pulsing ? " audit-pulse" : "")}
                   onClick={() => copyDeepLink(a.hash)}
                   title="Click to copy deep-link to this entry">
                <div className="chain"/>
                <span className="time">{formatTime(a.timestamp)}</span>
                <span>
                  <span className="action">{highlight(a.action)}</span>{" "}
                  <span className="dim">·</span>{" "}
                  <span className="actor">{highlight(a.agent_name || a.agent_id || "kernel")}</span>
                  <div className="dim" style={{fontSize:11,marginTop:2}}>{highlight(a.detail || a.outcome || "")}</div>
                </span>
                <span className="hash">{highlight(hash)}</span>
                <span className="dim">seq {a.seq}</span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/* ============================== SETTINGS ============================== */
function SettingsPage() {
  const [providersResp, , refreshProviders] = usePolling("/api/providers", 30000);
  const [config] = useApi("/api/config");
  const [health] = useApi("/api/health/detail");
  const [onboarding] = useApi("/api/onboarding");
  const [usersResp] = usePolling("/api/auth/users", 30000);
  const [editing, setEditing] = useState(null);

  const providers = (providersResp && providersResp.providers) || [];
  const users = (usersResp && usersResp.users) || [];

  const apiListen = (config && (config.api_listen || (config.api && config.api.listen))) || "—";
  const proxy = (config && (config.proxy_url || (config.proxy && config.proxy.url))) || null;
  const version = (health && health.version) || "0.7.61";
  const uptime = health && health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "—";
  const agentCount = health && health.agent_count != null ? health.agent_count : "—";

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-sub">Config at <span className="mono">~/.rustyhand/config.toml</span> · 50+ fields with serde defaults · live from <span className="mono">/api/config</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost"
                  title="Download a copy of config.toml with secrets redacted"
                  onClick={async () => {
                    try {
                      const text = await rhFetch("/api/config/export");
                      const stamp = new Date().toISOString().slice(0, 10);
                      downloadBlob(`rustyhand-config-${stamp}.toml`, text, "text/plain");
                      toastOk("Config exported (secrets redacted)");
                    } catch (e) { toastErr(`export failed: ${e.message || e}`); }
                  }}>
            <I.download/> Export config.toml
          </button>
        </div>
      </div>

      <div className="grid-12">
        <div className="col-8 card">
          <div className="row between mb-12">
            <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>LLM providers</span>
            <span className="dim mono" style={{fontSize:11}}>{providers.length} loaded · auto-probe at boot</span>
          </div>
          <table className="tbl">
            <thead><tr><th>Provider</th><th>Env var</th><th>State</th><th className="right">Models</th><th></th></tr></thead>
            <tbody>
              {!providersResp && (<tr><td colSpan={5} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
              {providers.map(p => {
                const auth = (p.auth_status || "").toLowerCase();
                let badge;
                if (auth === "ok") badge = <span className="badge live">set</span>;
                else if (auth === "missing" && p.key_required === false) badge = <span className="badge sky">local</span>;
                else if (p.id === "mock" || auth === "fallback") badge = <span className="badge demo">fallback</span>;
                else if (auth === "invalid") badge = <span className="badge error">invalid</span>;
                else badge = <span className="badge idle">not set</span>;
                return (
                  <tr key={p.id || p.name}>
                    <td className="mono">{p.display_name || p.id || p.name}</td>
                    <td className="mono muted">{p.api_key_env || "—"}</td>
                    <td>{badge}</td>
                    <td className="num mono">{p.model_count != null ? p.model_count : "—"}</td>
                    <td className="right"><button className="btn sm ghost" onClick={() => setEditing(p)}>Edit</button></td>
                  </tr>
                );
              })}
            </tbody>
          </table>

          <div className="row between mb-12 mt-16">
            <span className="mono dim" style={{fontSize:11,letterSpacing:".12em",textTransform:"uppercase"}}>Authorized users</span>
            <span className="dim mono" style={{fontSize:11}}>{users.length} configured · read-only here</span>
          </div>
          <table className="tbl">
            <thead><tr><th>Name</th><th>User ID</th><th>Role</th></tr></thead>
            <tbody>
              {!usersResp && (<tr><td colSpan={3} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
              {usersResp && users.length === 0 && (
                <tr><td colSpan={3} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>
                  No RBAC users — auth is in localhost-only mode. Add users in <span className="mono">~/.rustyhand/config.toml</span> under <span className="mono">[[auth.users]]</span>.
                </td></tr>
              )}
              {users.map((u) => {
                const role = (u.role || "viewer").toLowerCase();
                return (
                  <tr key={u.user_id || u.name}>
                    <td className="mono">{u.name || "—"}</td>
                    <td className="mono muted" style={{maxWidth:200, overflow:"hidden", textOverflow:"ellipsis"}}>{u.user_id || "—"}</td>
                    <td>
                      <span className={"badge " + (role === "owner" || role === "admin" ? "violet" : role === "operator" ? "live" : "plain")}>{role}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>

        <div className="col-4 col">
          <div className="card">
            <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>API</div>
            <div className="kv">
              <dt>listen</dt><dd className="mono">{apiListen}</dd>
              <dt>auth</dt><dd className="mono">{config && config.bearer_token ? "•••••••" : "localhost-only"}</dd>
              <dt>ws origins</dt><dd className="mono">{(config && (config.allowed_ws_origins || []).join(", ")) || "localhost"}</dd>
              <dt>proxy</dt><dd className={proxy ? "mono" : "dim"}>{proxy || "none"}</dd>
            </div>
          </div>
          <div className="card">
            <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Demo mode</div>
            <div className="kv">
              <dt>active</dt><dd>{onboarding ? (onboarding.demo_mode ? "yes" : "no") : "…"}</dd>
              <dt>provider</dt><dd className="mono">{onboarding ? onboarding.provider : "…"}</dd>
              <dt>api_key</dt><dd>{onboarding ? (onboarding.api_key_set ? "set" : "missing") : "…"}</dd>
              <dt>seeded</dt><dd>{onboarding && onboarding.demo_seeded ? "yes" : "no"}</dd>
              <dt>agents</dt><dd>{onboarding ? onboarding.agent_count : "…"}</dd>
            </div>
            <div className="dim mt-8" style={{fontSize:11}}>set <span className="mono">RUSTYHAND_DISABLE_DEMO_MODE=1</span> to fall back to NullDriver</div>
            {onboarding && onboarding.demo_mode && (
              <div className="row gap-6 mt-8">
                <button className="btn sm"
                        title="Removes the .rustyhand_demo_seeded marker so the daemon re-seeds the welcome agent + sample workflow on next start"
                        onClick={async () => {
                          const ok = await confirmDialog({
                            title: "Reset demo seed?",
                            message: "Removes the demo-seed marker. On the next daemon restart, the welcome agent, sample workflow, trigger and disabled cron job will be re-created. Existing resources stay in place — you may want to delete them first to avoid duplicates.",
                            confirmLabel: "Remove marker",
                          });
                          if (!ok) return;
                          try {
                            const r = await rhFetch("/api/onboarding/reset-demo", { method: "POST" });
                            toastOk(r.message || "Marker removed. Restart the daemon to re-seed.");
                          } catch (err) { toastErr(`reset failed: ${err.message || err}`); }
                        }}>
                  <I.refresh/> Re-seed on restart
                </button>
              </div>
            )}
          </div>
          <div className="card">
            <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Build</div>
            <div className="kv">
              <dt>version</dt><dd className="mono">{version}</dd>
              <dt>agents</dt><dd>{agentCount}</dd>
              <dt>uptime</dt><dd>{uptime}</dd>
              <dt>panics</dt><dd>{health && health.panic_count != null ? health.panic_count : "—"}</dd>
              <dt>restarts</dt><dd>{health && health.restart_count != null ? health.restart_count : "—"}</dd>
            </div>
          </div>
          <LogLevelCard config={config}/>
        </div>
      </div>

      {editing && <ProviderKeyModal provider={editing} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); refreshProviders(); }}/>}
    </div>
  );
}

// LogLevelCard — segmented control that POSTs to /api/config/set so
// operators can dial verbosity without editing config.toml. Values
// match tracing/EnvFilter levels accepted by the kernel.
function LogLevelCard({ config }) {
  const current = (config && config.log_level) || "info";
  const [busy, setBusy] = useState(false);
  const apply = async (level) => {
    setBusy(true);
    try {
      await rhFetch("/api/config/set", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "log_level", value: level }),
      });
      toastOk(`Log level set to ${level}. Restart daemon for effect.`);
    } catch (e) { toastErr(`set failed: ${e.message || e}`); }
    finally { setBusy(false); }
  };
  return (
    <div className="card">
      <div className="muted mono mb-8" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>
        Log verbosity
        <Tip>Persisted to config.toml.log_level. Takes effect on daemon restart — the running tracing EnvFilter is fixed at boot. Use trace/debug for troubleshooting, info for normal operation.</Tip>
      </div>
      <div className="seg" style={{flexWrap:"wrap"}}>
        {["error", "warn", "info", "debug", "trace"].map(l => (
          <button key={l}
                  className={current === l ? "on" : ""}
                  disabled={busy}
                  onClick={() => apply(l)}>
            {l}
          </button>
        ))}
      </div>
      <div className="dim mt-8" style={{fontSize:11}}>
        Current: <span className="mono">{current}</span>
      </div>
    </div>
  );
}

function ProviderKeyModal({ provider, onClose, onSaved }) {
  useEscapeKey(onClose);
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const [testResult, setTestResult] = useState(null);
  const name = provider.id || provider.name;

  const save = async () => {
    if (!key.trim()) { setErr("key is empty"); return; }
    setBusy(true); setErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(name)}/key`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key }),
      });
      onSaved();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };
  const clear = async () => {
    if (!(await confirmDialog({ title: "Delete API key", message: `Delete API key for ${name}?`, danger: true, confirmLabel: "Delete" }))) return;
    setBusy(true); setErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(name)}/key`, { method: "DELETE" });
      onSaved();
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };
  const test = async () => {
    setBusy(true); setErr(null); setTestResult(null);
    try {
      const r = await rhFetch(`/api/providers/${encodeURIComponent(name)}/test`, { method: "POST" });
      setTestResult(r);
    } catch (e) { setErr(String(e.message || e)); }
    finally { setBusy(false); }
  };

  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-head">
          <b className="mono">Edit provider · {provider.display_name || name}</b>
          <button className="icon-btn" onClick={onClose}><I.close/></button>
        </div>
        <div className="modal-body">
          <div className="kv mb-12">
            <dt>id</dt><dd className="mono">{name}</dd>
            <dt>env var</dt><dd className="mono">{provider.api_key_env || "—"}</dd>
            <dt>auth status</dt><dd className="mono">{provider.auth_status || "—"}</dd>
            <dt>models</dt><dd>{provider.model_count != null ? provider.model_count : "—"}</dd>
          </div>
          <label className="t-row col">
            <span className="t-lbl">New API key (will overwrite stored value, zeroized after use)</span>
            <input className="modal-field" type="password" placeholder={provider.api_key_env || "sk-…"} value={key} onChange={e => setKey(e.target.value)}/>
          </label>
          {testResult && (
            <div className="banner mt-12" style={{borderColor: testResult.ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)"}}>
              <span className={"dot " + (testResult.ok ? "live" : "err")}/>
              <span className="banner-title">{testResult.ok ? "OK" : "FAIL"}</span>
              <span className="banner-body mono" style={{fontSize:11}}>{testResult.message || testResult.detail || JSON.stringify(testResult).slice(0, 200)}</span>
            </div>
          )}
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span>
          </div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={clear} disabled={busy} style={{marginRight:"auto", color:"var(--crimson)"}}>Delete key</button>
          <button className="btn" onClick={test} disabled={busy}>Test</button>
          <button className="btn primary" onClick={save} disabled={busy || !key.trim()}>
            {busy ? "Saving…" : "Save"}
          </button>
        </div>
      </div>
    </div>
  );
}

/* ============================== MEMORY ============================== */
function MemoryPage() {
  const pg = usePagination(50);
  const [resp, fetchErr, refresh] = usePolling(
    `/api/sessions?limit=${pg.pageSize}&offset=${pg.offset}`, 20000);
  React.useEffect(() => { if (resp && resp.total != null) pg.setTotal(resp.total); }, [resp && resp.total]);
  const sessions = (resp && resp.sessions) || [];
  const [labelEditing, setLabelEditing] = useState(null);
  const [labelDraft, setLabelDraft] = useState("");
  const fileInputRef = useRef(null);
  const [selected, setSelected] = useState(() => new Set());
  // Drop selections that no longer appear in the current page (e.g. after
  // a delete or filter change). Keeps the "Delete N selected" badge honest.
  React.useEffect(() => {
    if (selected.size === 0) return;
    const visible = new Set(sessions.map(s => s.session_id));
    const next = new Set([...selected].filter(id => visible.has(id)));
    if (next.size !== selected.size) setSelected(next);
  }, [sessions]);
  const toggle = (id) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id); else next.add(id);
    setSelected(next);
  };
  const toggleAll = () => {
    if (selected.size === sessions.length && sessions.length > 0) setSelected(new Set());
    else setSelected(new Set(sessions.map(s => s.session_id)));
  };
  const bulkDelete = async () => {
    if (selected.size === 0) return;
    const ok = await confirmDialog({
      title: `Delete ${selected.size} session(s)?`,
      message: "This cannot be undone. Sessions and their messages will be permanently removed.",
      danger: true,
      confirmLabel: `Delete ${selected.size}`,
    });
    if (!ok) return;
    let okCount = 0;
    let failCount = 0;
    for (const id of selected) {
      try {
        await rhFetch(`/api/sessions/${encodeURIComponent(id)}`, { method: "DELETE" });
        okCount++;
      } catch (_) { failCount++; }
    }
    setSelected(new Set());
    if (failCount > 0) toastErr(`Deleted ${okCount}, failed ${failCount}`);
    else toastOk(`Deleted ${okCount} session(s)`);
    refresh();
  };
  const bulkExport = async () => {
    if (selected.size === 0) return;
    const parts = [];
    let okCount = 0;
    let failCount = 0;
    for (const id of selected) {
      try {
        const md = await rhFetch(`/api/sessions/${encodeURIComponent(id)}/export.md`);
        parts.push(`# Session ${String(id).slice(0, 8)}\n\n${md}\n\n---\n`);
        okCount++;
      } catch (_) { failCount++; }
    }
    if (parts.length === 0) {
      toastErr("Bulk export produced no content");
      return;
    }
    const stamp = new Date().toISOString().slice(0, 10);
    downloadBlob(`rustyhand-sessions-${stamp}.md`, parts.join(""), "text/markdown");
    if (failCount > 0) toastErr(`Exported ${okCount}, failed ${failCount}`);
    else toastOk(`Exported ${okCount} session(s)`);
  };

  const remove = async (id) => {
    if (!(await confirmDialog({ title: "Delete session", message: `Delete session ${String(id).slice(0, 8)}? This cannot be undone.`, danger: true, confirmLabel: "Delete" }))) return;
    try { await rhFetch(`/api/sessions/${encodeURIComponent(id)}`, { method: "DELETE" }); refresh(); }
    catch (e) { toastErr(`delete failed: ${e.message || e}`); }
  };
  const saveLabel = async (id) => {
    try {
      await rhFetch(`/api/sessions/${encodeURIComponent(id)}/label`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ label: labelDraft }),
      });
      setLabelEditing(null);
      refresh();
    } catch (e) { toastErr(`label set failed: ${e.message || e}`); }
  };
  const exportMarkdown = (id) => {
    // Server returns markdown directly; just hit the URL with the auth header.
    rhFetch(`/api/sessions/${encodeURIComponent(id)}/export.md`)
      .then(md => downloadBlob(`session-${String(id).slice(0, 8)}.md`, md, "text/markdown"))
      .catch(e => toastErr(`export failed: ${e.message || e}`));
  };
  const backupMemory = () => {
    rhFetch("/api/memory/export?format=json")
      .then(data => {
        const blob = typeof data === "string" ? data : JSON.stringify(data, null, 2);
        downloadBlob(`rustyhand-memory-${new Date().toISOString().slice(0, 10)}.json`, blob, "application/json");
      })
      .catch(e => toastErr(`export failed: ${e.message || e}`));
  };
  const importMemory = async (file) => {
    if (!file) return;
    const text = await file.text();
    try {
      const r = await rhFetch("/api/memory/import", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: text,
      });
      toastOk(`Imported: ${r.imported || r.message || "ok"}`);
      refresh();
    } catch (e) { toastErr(`import failed: ${e.message || e}`); }
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Memory <span className="dim mono" style={{fontSize:14}}>· {sessions.length} session(s)</span></h1>
          <p className="page-sub">SQLite-backed sessions · backup at <span className="mono">/api/memory/export</span> · restore at <span className="mono">/api/memory/import</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
          {selected.size > 0 && (
            <>
              <button className="btn sm ghost" onClick={bulkExport} title="Export selected sessions as one markdown file">
                <I.download/> Export {selected.size}
              </button>
              <button className="btn sm danger" onClick={bulkDelete} title="Delete the selected sessions">
                <I.trash/> Delete {selected.size}
              </button>
            </>
          )}
          <button className="btn ghost" onClick={() => fileInputRef.current && fileInputRef.current.click()}><I.copy/> Restore</button>
          <input ref={fileInputRef} type="file" accept="application/json,.json"
                 style={{display:"none"}}
                 onChange={e => importMemory(e.target.files && e.target.files[0])}/>
          <button className="btn primary" onClick={backupMemory}><I.download/> Backup</button>
        </div>
      </div>

      {fetchErr && (
        <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
          <span className="dot err"/><span className="banner-title">API ERROR</span>
          <span className="banner-body mono" style={{fontSize:11}}>{fetchErr}</span>
        </div>
      )}

      <div className="card flush">
        <table className="tbl">
          <thead><tr>
            <th style={{width:28}}>
              <input type="checkbox"
                     checked={sessions.length > 0 && selected.size === sessions.length}
                     onChange={toggleAll}
                     title="Select all on this page"/>
            </th>
            <th>Session</th><th>Agent</th><th>Label</th><th className="right">Messages</th><th>Created</th><th>Updated</th><th></th>
          </tr></thead>
          <tbody>
            {!resp && <tr><td colSpan={8} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>}
            {resp && sessions.length === 0 && <tr><td colSpan={8} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>No sessions yet.</td></tr>}
            {sessions.map(s => (
              <tr key={s.session_id} style={selected.has(s.session_id) ? {background:"var(--bg-2)"} : null}>
                <td>
                  <input type="checkbox"
                         checked={selected.has(s.session_id)}
                         onChange={() => toggle(s.session_id)}/>
                </td>
                <td className="mono" style={{maxWidth:120, overflow:"hidden", textOverflow:"ellipsis"}}>{String(s.session_id).slice(0, 8)}</td>
                <td className="mono">{s.agent_name || s.agent_id || "—"}</td>
                <td>
                  {labelEditing === s.session_id ? (
                    <span className="row gap-4">
                      <input className="modal-field" style={{padding:"3px 6px", fontSize:11.5}} autoFocus
                             value={labelDraft} onChange={e => setLabelDraft(e.target.value)}
                             onKeyDown={e => { if (e.key === "Enter") saveLabel(s.session_id); if (e.key === "Escape") setLabelEditing(null); }}/>
                      <button className="btn sm" onClick={() => saveLabel(s.session_id)}>save</button>
                    </span>
                  ) : (
                    <span style={{cursor:"pointer"}} onClick={() => { setLabelEditing(s.session_id); setLabelDraft(s.label || ""); }}>
                      {s.label || <span className="dim">— click to set —</span>}
                    </span>
                  )}
                </td>
                <td className="num mono">{Number(s.message_count || 0).toLocaleString()}</td>
                <td className="mono muted">{relativeTime(s.created_at)}</td>
                <td className="mono muted">{relativeTime(s.updated_at)}</td>
                <td className="right">
                  <button className="btn sm ghost" onClick={() => exportMarkdown(s.session_id)} title="Export markdown"><I.download/></button>
                  <button className="btn sm danger" onClick={() => remove(s.session_id)} title="Delete"><I.close/></button>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
        <Pagination pg={pg}/>
      </div>
    </div>
  );
}

/* ============================== MCP ============================== */
function McpPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/mcp/servers", 30000);
  const configured = (resp && resp.configured) || [];
  const connected = (resp && resp.connected) || [];
  const connectedByName = new Map(connected.map(c => [c.name, c]));
  const [expanded, setExpanded] = useState(() => new Set());
  const toggleExpand = (name) => {
    setExpanded(prev => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name); else next.add(name);
      return next;
    });
  };
  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">MCP servers <span className="dim mono" style={{fontSize:14}}>· {configured.length} configured · {connected.length} connected</span></h1>
          <p className="page-sub">Model-Context-Protocol bridges · live from <span className="mono">/api/mcp/servers</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
        </div>
      </div>
      {fetchErr && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">API ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{fetchErr}</span>
      </div>}
      {!resp && <div className="muted mono" style={{padding:"24px", fontSize:12}}>loading…</div>}
      {resp && configured.length === 0 && <div className="muted mono" style={{padding:"24px", fontSize:12}}>No MCP servers configured. Add them in <span className="mono">~/.rustyhand/config.toml</span> under <span className="mono">[[mcp.servers]]</span>.</div>}
      <div className="grid-12">
        {configured.map(s => {
          const live = connectedByName.get(s.name);
          const isConnected = !!live;
          const transport = s.transport || {};
          const tools = (live && Array.isArray(live.tools)) ? live.tools : [];
          const isOpen = expanded.has(s.name);
          return (
            <div key={s.name} className="col-6 card">
              <div className="row between mb-12">
                <div>
                  <div className="mono" style={{fontSize:14, fontWeight:500}}>{s.name}</div>
                  <div className="dim mono" style={{fontSize:11}}>{transport.type || "—"} · {s.timeout_secs ? `${s.timeout_secs}s timeout` : "no timeout"}</div>
                </div>
                <span className={"badge " + (isConnected ? "live" : "idle")}>{isConnected ? "connected" : "idle"}</span>
              </div>
              <div className="kv">
                {transport.type === "stdio" && <>
                  <dt>command</dt><dd className="mono">{transport.command}</dd>
                  <dt>args</dt><dd className="mono">{(transport.args || []).join(" ") || "—"}</dd>
                </>}
                {transport.type === "sse" && <>
                  <dt>url</dt><dd className="mono">{transport.url}</dd>
                </>}
                {s.env && Object.keys(s.env).length > 0 && <>
                  <dt>env</dt><dd className="mono dim">{Object.keys(s.env).join(", ")}</dd>
                </>}
                {isConnected && <>
                  <dt>tools</dt>
                  <dd className="mono">
                    {tools.length}
                    {tools.length > 0 && (
                      <button className="kbd"
                              style={{marginLeft:6, cursor:"pointer"}}
                              onClick={() => toggleExpand(s.name)}>
                        {isOpen ? "hide" : "show"}
                      </button>
                    )}
                  </dd>
                </>}
              </div>
              {isOpen && tools.length > 0 && (
                <div className="col gap-4 mt-8" style={{
                  maxHeight: 240,
                  overflow: "auto",
                  background: "var(--bg-2)",
                  borderRadius: 6,
                  padding: "8px 10px",
                }}>
                  {tools.map((t, i) => (
                    <div key={t.name || i} style={{
                      fontFamily: "var(--ff-mono)",
                      fontSize: 11.5,
                      padding: "3px 0",
                      borderBottom: i < tools.length - 1 ? "1px solid var(--border)" : "none",
                    }}>
                      <span style={{color:"var(--rust)"}}>{t.name}</span>
                      {t.description && (
                        <div className="dim" style={{
                          marginTop: 2,
                          whiteSpace: "nowrap",
                          overflow: "hidden",
                          textOverflow: "ellipsis",
                        }}>{t.description}</div>
                      )}
                    </div>
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/* ============================== NETWORK ============================== */
function NetworkPage() {
  const [status, , refreshStatus] = usePolling("/api/network/status", 15000);
  const [peersResp, , refreshPeers] = usePolling("/api/peers", 15000);
  const peers = (peersResp && peersResp.peers) || [];
  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Network</h1>
          <p className="page-sub">RHP peer-to-peer protocol (JSON-RPC over TCP) · live from <span className="mono">/api/network/status</span> · <span className="mono">/api/peers</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={() => { refreshStatus(); refreshPeers(); }}><I.refresh/></button>
        </div>
      </div>
      <div className="tiles">
        <Tile label="Network state" value={status ? (status.enabled ? "enabled" : "disabled") : "…"} foot={status && status.node_id ? `node ${String(status.node_id).slice(0, 12)}` : "no node id"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
        <Tile label="Listen address" value={status && status.listen_address ? String(status.listen_address) : "—"} foot={status ? "TCP" : "loading…"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
        <Tile label="Connected peers" value={status ? (status.connected_peers != null ? String(status.connected_peers) : "—") : "…"} foot={status ? `${status.total_peers || 0} known` : "loading…"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
        <Tile label="Loaded peers" value={`${peers.length}`} foot={peers.length === 0 ? "no peers" : "see below"} spark={[0,0,0,0,0,0,0,0,0,0,0,0]}/>
      </div>
      <div className="card flush">
        <div className="card-head"><span>Known peers</span></div>
        <table className="tbl">
          <thead><tr><th>Node ID</th><th>Address</th><th>State</th><th>Last seen</th></tr></thead>
          <tbody>
            {!peersResp && <tr><td colSpan={4} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>}
            {peersResp && peers.length === 0 && <tr><td colSpan={4} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>No peers — network may be disabled or no peer has connected yet.</td></tr>}
            {peers.map((p, i) => (
              <tr key={p.node_id || p.id || i}>
                <td className="mono" style={{maxWidth:200, overflow:"hidden", textOverflow:"ellipsis"}}>{p.node_id || p.id || "—"}</td>
                <td className="mono">{p.address || p.endpoint || "—"}</td>
                <td>{p.state ? <span className={"badge " + (String(p.state).toLowerCase() === "connected" ? "live" : "idle")}>{p.state}</span> : <span className="badge idle">—</span>}</td>
                <td className="mono muted">{p.last_seen ? relativeTime(p.last_seen) : "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}

/* ============================== HEALTH ============================== */
// Read-only system diagnostics page. Aggregates the various status
// endpoints (health/detail, audit/verify, network/status, mcp/servers,
// onboarding) into a single dashboard so an operator can answer
// "is the daemon healthy right now?" without bouncing between pages.
function HealthPage() {
  const [health, , refreshHealth] = usePolling("/api/health/detail", 10000);
  const [audit, , refreshAudit] = usePolling("/api/audit/verify", 30000);
  const [net] = usePolling("/api/network/status", 15000);
  const [mcp] = usePolling("/api/mcp/servers", 30000);
  const [onb] = usePolling("/api/onboarding", 30000);
  const [usage] = usePolling("/api/usage/daily", 20000);
  const allWarns = (health && health.config_warnings) || [];

  const refresh = () => { refreshHealth(); refreshAudit(); };

  // Roll-up traffic light: red if anything is failing or critical
  // warnings present; amber if degraded or non-critical warnings;
  // green if everything is normal.
  let overall = "green", overallLabel = "all systems normal";
  if (health) {
    if (health.status !== "ok") { overall = "red"; overallLabel = `kernel status: ${health.status}`; }
    else if (audit && audit.valid === false) { overall = "red"; overallLabel = "audit chain mismatch"; }
    else if (health.panic_count > 0 || health.restart_count > 0) {
      overall = "amber";
      overallLabel = `${health.panic_count || 0} panic(s), ${health.restart_count || 0} restart(s)`;
    }
    else if (allWarns.length > 0) {
      overall = "amber";
      overallLabel = `${allWarns.length} config warning(s)`;
    }
  } else {
    overallLabel = "checking…";
  }

  const dotCls = overall === "red" ? "err" : overall === "amber" ? "warn" : "live";
  const badgeCls = overall === "red" ? "error" : overall === "amber" ? "warn" : "live";
  const todayCost = (usage && (usage.cost_usd_today || usage.cost_usd || usage.total_cost_usd)) || 0;

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Health</h1>
          <p className="page-sub">Live diagnostics · health/detail + audit/verify + network/status + mcp/servers</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
        </div>
      </div>

      <div className="banner mb-12" style={{
        borderColor: overall === "red" ? "oklch(0.66 0.18 25 / .35)"
          : overall === "amber" ? "oklch(0.78 0.14 88 / .35)"
          : "oklch(0.66 0.15 155 / .35)",
      }}>
        <span className={"dot " + dotCls}/>
        <span className="banner-title">{overall === "green" ? "HEALTHY" : overall === "amber" ? "DEGRADED" : "ATTENTION"}</span>
        <span className="banner-body mono" style={{fontSize:11}}>{overallLabel}</span>
        <span className="dim mono" style={{fontSize:10.5, marginLeft:"auto"}}>polls 10s</span>
      </div>

      <div className="grid-12">
        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Kernel</div>
          {!health && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {health && (
            <div className="kv">
              <dt>status</dt><dd><span className={"badge " + badgeCls}>{health.status}</span></dd>
              <dt>version</dt><dd className="mono">{health.version}</dd>
              <dt>uptime</dt><dd className="mono">{health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "—"}</dd>
              <dt>database</dt><dd>{health.database === "connected"
                ? <span className="badge live">connected</span>
                : <span className="badge error">{health.database}</span>}</dd>
              <dt>agents</dt><dd className="num mono">{health.agent_count}</dd>
              <dt>panics</dt><dd className="num mono" style={{color: health.panic_count > 0 ? "var(--crimson)" : undefined}}>{health.panic_count}</dd>
              <dt>restarts</dt><dd className="num mono" style={{color: health.restart_count > 0 ? "var(--amber)" : undefined}}>{health.restart_count}</dd>
            </div>
          )}
        </div>

        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Audit chain</div>
          {!audit && <div className="dim mono" style={{fontSize:11}}>verifying…</div>}
          {audit && (
            <div className="kv">
              <dt>integrity</dt><dd>{audit.valid
                ? <span className="badge live"><I.check/> verified</span>
                : <span className="badge error"><I.warn/> mismatch</span>}</dd>
              <dt>entries</dt><dd className="num mono">{(audit.entries || []).length || audit.total || 0}</dd>
              <dt>tip hash</dt><dd className="mono" style={{wordBreak:"break-all", fontSize:11}}>
                {audit.tip_hash ? String(audit.tip_hash).slice(0, 24) + "…" : "—"}
              </dd>
              {audit.warning && <><dt>warning</dt><dd className="mono dim" style={{fontSize:11}}>{audit.warning}</dd></>}
            </div>
          )}
        </div>

        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Network</div>
          {!net && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {net && (
            <div className="kv">
              <dt>state</dt><dd>{net.enabled
                ? <span className="badge live">enabled</span>
                : <span className="badge idle">disabled</span>}</dd>
              <dt>listen</dt><dd className="mono">{net.listen_address || "—"}</dd>
              <dt>node id</dt><dd className="mono" style={{wordBreak:"break-all", fontSize:11}}>
                {net.node_id ? String(net.node_id).slice(0, 24) + "…" : "—"}
              </dd>
              <dt>peers</dt><dd className="num mono">{net.connected_peers != null ? net.connected_peers : "—"}{net.total_peers != null && <span className="dim"> / {net.total_peers}</span>}</dd>
            </div>
          )}
        </div>

        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>MCP servers</div>
          {!mcp && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {mcp && (
            <div className="kv">
              <dt>configured</dt><dd className="num mono">{mcp.total_configured != null ? mcp.total_configured : (mcp.configured || []).length}</dd>
              <dt>connected</dt><dd className="num mono">
                {mcp.total_connected != null ? mcp.total_connected : (mcp.connected || []).length}
                {mcp.total_configured > 0 && mcp.total_connected === 0 && (
                  <span className="badge warn" style={{marginLeft:6}}>none live</span>
                )}
              </dd>
              <dt>tools</dt><dd className="num mono">
                {(mcp.connected || []).reduce((s, c) => s + (c.tools_count || (c.tools || []).length || 0), 0)}
              </dd>
            </div>
          )}
        </div>

        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Mode</div>
          {!onb && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {onb && (
            <div className="kv">
              <dt>demo</dt><dd>{onb.demo_mode
                ? <span className="badge demo">yes · {onb.provider}</span>
                : <span className="badge live">no · {onb.provider}</span>}</dd>
              <dt>api key</dt><dd>{onb.api_key_set
                ? <span className="badge live">set</span>
                : <span className="badge idle">missing</span>}</dd>
              <dt>model</dt><dd className="mono">{onb.model || "—"}</dd>
              <dt>seeded</dt><dd>{onb.demo_seeded ? "yes" : "no"}</dd>
            </div>
          )}
        </div>

        <div className="col-6 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Today's spend</div>
          {!usage && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {usage && (
            <div>
              <div className="mono" style={{fontSize:22, color: todayCost > 5 ? "var(--amber)" : "var(--rust)"}}>
                ${Number(todayCost).toFixed(4)}
              </div>
              <div className="dim mono mt-4" style={{fontSize:11}}>
                Tokens: {Number((usage.input_tokens || 0) + (usage.output_tokens || 0)).toLocaleString()} ·{" "}
                Calls: {Number(usage.tool_calls || 0).toLocaleString()}
              </div>
            </div>
          )}
        </div>

        {allWarns.length > 0 && (
          <div className="col-12 card">
            <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>
              Config warnings <span className="dim" style={{marginLeft:6}}>{allWarns.length}</span>
            </div>
            <div className="col gap-4">
              {allWarns.map((w, i) => (
                <div key={i} className="row gap-6" style={{
                  padding:"5px 8px",
                  background:"var(--bg-2)",
                  borderRadius:5,
                  fontFamily:"var(--ff-mono)",
                  fontSize:11.5,
                }}>
                  <span className="dot warn"/>
                  <span>{typeof w === "string" ? w : (w.message || JSON.stringify(w))}</span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/* ============================== BINDINGS ============================== */
function BindingsPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/bindings", 30000);
  const bindings = (resp && resp.bindings) || [];
  // Indices are positional and shift on each delete, so the Set tracks
  // the raw index values from the current render. We reset on refresh.
  const [selected, setSelected] = useState(() => new Set());
  React.useEffect(() => { setSelected(new Set()); }, [resp && bindings.length]);

  const remove = async (index) => {
    if (!(await confirmDialog({ title: "Remove binding", message: `Remove binding #${index}?`, danger: true, confirmLabel: "Remove" }))) return;
    try {
      await rhFetch(`/api/bindings/${index}`, { method: "DELETE" });
      toastOk("Binding removed");
      refresh();
    } catch (e) { toastErr(`Remove failed: ${e.message || e}`); }
  };
  const toggle = (i) => {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i); else next.add(i);
      return next;
    });
  };
  const toggleAll = () => {
    setSelected(prev => {
      if (bindings.length === 0) return prev;
      if (prev.size === bindings.length) return new Set();
      return new Set(bindings.map((_, i) => i));
    });
  };
  // Bulk delete must walk indices descending — each successful DELETE
  // shifts later indices down by one on the server, so descending order
  // keeps untouched indices stable until we reach them.
  const bulkRemove = async () => {
    const ids = [...selected].sort((a, b) => b - a);
    if (ids.length === 0) return;
    const ok = await confirmDialog({
      title: `Remove ${ids.length} binding(s)?`,
      message: "Bindings are positional — this loops indices from high to low so partial failure can't desync the remaining selection.",
      danger: true,
      confirmLabel: `Remove ${ids.length}`,
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const i of ids) {
      try {
        await rhFetch(`/api/bindings/${i}`, { method: "DELETE" });
        okCount++;
      } catch (_) { failCount++; }
    }
    setSelected(new Set());
    if (failCount > 0) toastErr(`Removed ${okCount}, failed ${failCount}`);
    else toastOk(`Removed ${okCount} binding(s)`);
    refresh();
  };

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Bindings <span className="dim mono" style={{fontSize:14}}>· {bindings.length}</span></h1>
          <p className="page-sub">Agent → channel/trigger bindings · live from <span className="mono">/api/bindings</span></p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={refresh}><I.refresh/></button>
        </div>
      </div>
      {selected.size > 0 && (
        <div className="bulk-bar">
          <span className="mono" style={{fontSize:12}}>{selected.size} selected</span>
          <button className="btn sm danger" onClick={bulkRemove}>
            <I.trash/> Remove {selected.size}
          </button>
          <button className="btn sm ghost" onClick={() => setSelected(new Set())} style={{marginLeft:"auto"}}>Clear</button>
        </div>
      )}
      {fetchErr && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">API ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{fetchErr}</span>
      </div>}
      <div className="card flush">
        <table className="tbl">
          <thead><tr>
            <th style={{width:28}}>
              <input type="checkbox"
                     checked={bindings.length > 0 && selected.size === bindings.length}
                     ref={el => { if (el) el.indeterminate = selected.size > 0 && selected.size < bindings.length; }}
                     onChange={toggleAll}
                     title={selected.size === bindings.length ? "Deselect all" : "Select all"}/>
            </th>
            <th>#</th><th>Agent</th><th>Kind</th><th>Target</th><th>Pattern</th><th></th>
          </tr></thead>
          <tbody>
            {!resp && <tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>}
            {resp && bindings.length === 0 && <tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>No bindings configured.</td></tr>}
            {bindings.map((b, i) => {
              const isSel = selected.has(i);
              return (
                <tr key={i} style={isSel ? {background:"var(--surface-2)"} : null}>
                  <td>
                    <input type="checkbox"
                           checked={isSel}
                           onChange={() => toggle(i)}/>
                  </td>
                  <td className="num mono">{i}</td>
                  <td className="mono">{b.agent_id || b.agent || "—"}</td>
                  <td className="mono">{b.kind || b.type || "—"}</td>
                  <td className="mono">{b.target || b.channel || b.trigger || "—"}</td>
                  <td className="mono dim" style={{maxWidth:280, overflow:"hidden", textOverflow:"ellipsis"}}>{b.pattern ? JSON.stringify(b.pattern) : "—"}</td>
                  <td className="right">
                    <button className="btn sm danger" onClick={() => remove(i)} title="Remove"><I.close/></button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}

Object.assign(window, {
  OverviewPage, AgentsPage, AgentDrawer, ChatPage, WorkflowsPage,
  AutomationPage, ChannelsPage, AnalyticsPage, KnowledgePage,
  SkillsPage, ApprovalsPage, AuditPage, SettingsPage, MemoryPage,
  McpPage, NetworkPage, BindingsPage, HealthPage,
});
