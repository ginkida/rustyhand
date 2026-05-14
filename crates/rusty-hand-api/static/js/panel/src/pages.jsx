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
  const version = (health && health.version) || "0.7.48";
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

function formatTime(ts) {
  if (!ts) return "—";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return String(ts).slice(11, 19);
  return d.toLocaleTimeString("en-GB", { hour12: false });
}

const ActivityFeed = ({ entries }) => {
  const cls = c => ({ live: "var(--live)", violet: "var(--violet)", amber: "var(--amber)", muted: "var(--fg-3)" }[c]);
  if (!entries) {
    return (
      <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>
        Loading audit chain…
      </div>
    );
  }
  if (entries.length === 0) {
    return (
      <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>
        No audit entries yet — they appear as agents act.
      </div>
    );
  }
  return (
    <div style={{ maxHeight: 360, overflow: "auto" }}>
      {entries.map((it, i) => {
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
const ApprovalsTable = ({ rows, compact, onChange }) => {
  const decide = async (id, verdict) => {
    try {
      await rhFetch(`/api/approvals/${id}/${verdict}`, { method: "POST" });
      if (onChange) onChange();
    } catch (e) {
      console.warn(`approval ${verdict} failed`, e);
    }
  };
  return (
    <table className="tbl">
      <thead>
        <tr><th>ID</th><th>Agent</th><th>Action</th><th>Risk</th><th>Age</th><th className="right">Decide</th></tr>
      </thead>
      <tbody>
        {rows.map(r => {
          const agent = r.agent_name || r.agent || r.agent_id || "—";
          const age = r.age || relativeTime(r.requested_at || r.created_at);
          const risk = (r.risk || "low").toLowerCase();
          return (
            <tr key={r.id}>
              <td className="mono">{r.id}</td>
              <td className="mono">{agent}</td>
              <td>{r.action}</td>
              <td><span className={`badge ${risk === "high" ? "error" : risk === "medium" ? "warn" : "idle"}`}>{risk}</span></td>
              <td className="mono muted">{age}</td>
              <td className="right">
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
  // No server-side pagination on Agents: the typical N is small (5-50)
  // and the filter/search is client-side. Paginating the server response
  // makes the "1-50 of 200" counter incoherent with a state filter that
  // can only see the current page. If real ops hit 1000+ agents we'll
  // revisit — by then the API needs a `?state=` filter to keep the
  // counts honest.
  const [resp, fetchErr, refresh] = usePolling("/api/agents?limit=200", 15000);
  const agents = (resp && resp.agents) ? resp.agents.map(normalizeAgent) : D.agents;
  const filtered = agents.filter(a => {
    if (filter !== "all" && a.state !== filter && !(filter === "running" && a.state === "running")) return false;
    if (q && !a.name.toLowerCase().includes(q.toLowerCase()) && !a.model.toLowerCase().includes(q.toLowerCase())) return false;
    return true;
  });

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
      </div>

      <div className="card flush">
        <table className="tbl">
          <thead><tr>
            <th>Agent</th><th>Group</th><th>Model</th><th>State</th>
            <th className="right">Msgs</th><th className="right">Cost · 24h</th><th>Last activity</th><th>Updated</th><th></th>
          </tr></thead>
          <tbody>
            {!resp && Array.from({length:5}).map((_,i) => (
              <SkelRow key={`s-${i}`} cols={[160, 80, 140, 90, 60, 60, 240, 50, 24]}/>
            ))}
            {filtered.map(a => (
              <tr key={a.id} style={{cursor:"pointer"}} onClick={() => openAgent(a)}>
                <td>
                  <div className="agent-row">
                    <Avatar agent={a}/>
                    <div>
                      <div className="name">{a.name}</div>
                      <div className="meta">{a.id}</div>
                    </div>
                  </div>
                </td>
                <td className="muted mono">{a.group}</td>
                <td className="mono">{a.model}<div className="meta dim">{a.provider}</div></td>
                <td><StateBadge state={a.state}/></td>
                <td className="num mono">{a.messages.toLocaleString()}</td>
                <td className="num mono">${a.cost.toFixed(2)}</td>
                <td className="muted" style={{maxWidth:280,whiteSpace:"nowrap",overflow:"hidden",textOverflow:"ellipsis"}}>{a.last}</td>
                <td className="mono muted">{a.updated}</td>
                <td style={{position:"relative"}}>
                  <button className="btn sm ghost" onClick={e => { e.stopPropagation(); setRowMenu(rowMenu === a.id ? null : a.id); }}>
                    <I.more/>
                  </button>
                  {rowMenu === a.id && (
                    <div className="row-menu" onClick={e => e.stopPropagation()}>
                      <button onClick={() => { setRowMenu(null); restartAgent(a.id); }}><I.refresh/> Restart</button>
                      <button onClick={() => { setRowMenu(null); killAgent(a.id); }} style={{color:"var(--crimson)"}}><I.close/> Kill</button>
                    </div>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {showSpawn && <SpawnAgentModal onClose={() => setShowSpawn(false)} onSpawned={() => { setShowSpawn(false); refresh(); }}/>}
    </div>
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
          {tab === "config" && detail && <AgentConfigForm agent={agent} detail={detail} onSaved={refreshDetail}/>}
          {tab === "config" && !detail && <div className="dim mono" style={{fontSize:11}}>loading…</div>}
          {tab === "identity" && <AgentIdentityForm agent={agent} detail={detail} onSaved={refreshDetail}/>}
          {tab === "activity" && (
            <>
              <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Recent turns</div>
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

function AgentConfigForm({ agent, detail, onSaved }) {
  const current = detail || {};
  const model = current.model || {};
  const [name, setName] = useState(current.name || agent.name);
  const [group, setGroup] = useState(current.group || agent.group || "");
  const [description, setDescription] = useState(current.description || "");
  const [systemPrompt, setSystemPrompt] = useState(model.system_prompt || "");
  const [temperature, setTemperature] = useState(model.temperature != null ? String(model.temperature) : "0.4");
  const [maxTokens, setMaxTokens] = useState(model.max_tokens != null ? String(model.max_tokens) : "2048");
  const [thinkingEnabled, setThinkingEnabled] = useState(!!model.thinking);
  const [modelName, setModelName] = useState(model.model || agent.model);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const [ok, setOk] = useState(false);

  // Reset form when active agent changes (parent remounts on agent id change).
  // No re-fetch needed since detail prop changes drive useState re-init.

  const save = async () => {
    setBusy(true); setErr(null); setOk(false);
    try {
      await rhFetch(`/api/agents/${agent.id}/config`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name, group, description,
          system_prompt: systemPrompt,
          temperature: Number(temperature),
          max_tokens: Number(maxTokens),
          thinking_enabled: thinkingEnabled,
        }),
      });
      // Model is its own endpoint.
      if (modelName && modelName !== model.model) {
        await rhFetch(`/api/agents/${agent.id}/model`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ model: modelName }),
        });
      }
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
      <label className="t-row col"><span className="t-lbl">Model</span>
        <input className="modal-field" value={modelName} onChange={e => setModelName(e.target.value)}/></label>
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
      {err && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
      {ok && <div className="banner" style={{borderColor:"oklch(0.74 0.135 150 / .35)"}}>
        <span className="dot live"/><span className="banner-title">SAVED</span>
        <span className="banner-body" style={{fontSize:11.5}}>Hot-reloaded — next turn uses the new config.</span></div>}
      <div className="row" style={{justifyContent:"flex-end", marginTop:8}}>
        <button className="btn primary" onClick={save} disabled={busy}>{busy ? "Saving…" : "Save config"}</button>
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
          <input className="modal-field" value={color} onChange={e => setColor(e.target.value)} placeholder="#d4541b"/></label>
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

  const items = sessionToItems(session && session.messages).concat(pendingMessages);

  const send = async () => {
    const text = typed.trim();
    if (!text || sending) return;
    setTyped("");
    setPendingMessages((prev) => prev.concat([{ role: "user", content: text, _local: true }]));
    setStreamingText("");
    setStreamingTools([]);
    setSending(true);
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

  return (
    <div className="chat-wrap">
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
            <button className="btn sm ghost" onClick={refreshSession}><I.refresh/></button>
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
            if (it.role === "tool") return (
              <div key={i} className={"tool-trace " + (it.running ? "" : "done")}>
                <span style={{display:"inline-flex",width:14,height:14}}>{it.running ? <Spinner/> : <I.check/>}</span>
                <span>{it.running ? "⚙" : "✓"} <span className="tool-name">{it.name}</span> <span className="dim">({String(it.input || "").slice(0, 80)})</span></span>
                <span className="elapsed">{it.is_error ? "err" : "ok"}</span>
              </div>
            );
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
          {streamingTools.map((t, i) => (
            <div key={`stool-${i}`} className={"tool-trace " + (t.running ? "" : "done")}>
              <span style={{display:"inline-flex",width:14,height:14}}>{t.running ? <Spinner/> : <I.check/>}</span>
              <span>{t.running ? "⚙" : "✓"} <span className="tool-name">{t.name}</span> {t.input && <span className="dim">({String(typeof t.input === "string" ? t.input : JSON.stringify(t.input)).slice(0, 80)})</span>}</span>
              <span className="elapsed">{t.is_error ? "err" : (t.running ? "…" : "ok")}</span>
            </div>
          ))}
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

        <div className="chat-input">
          <div className="field">
            <I.zap/>
            <input placeholder={`Message ${active.name}…  /workflow  /tool  /memory`}
                   value={typed} disabled={sending} onChange={e=>setTyped(e.target.value)} onKeyDown={onKeyDown}/>
            <span className="kbd">↵ send</span>
          </div>
          <button className="btn primary" onClick={send} disabled={sending || !typed.trim()}><I.send/> {sending ? "…" : "Send"}</button>
        </div>
      </div>

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
  const [showRunInput, setShowRunInput] = useState(false);

  // The real path is GET /api/workflows/{id}/runs (path param, NOT a
  // ?workflow_id= query — the handler reads it as Path<String>).
  const [runsResp, , refreshRuns] = useApi(active ? `/api/workflows/${encodeURIComponent(active.id)}/runs` : null);
  const runs = Array.isArray(runsResp) ? runsResp : (runsResp && runsResp.runs) || [];

  const runWith = async (input) => {
    if (!active) return;
    try {
      await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}/run`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input }),
      });
      setShowRunInput(false);
      refreshRuns();
    } catch (e) {
      toastErr(`run failed: ${e.message || e}`);
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
                  <button className="btn sm" onClick={refreshRuns}><I.refresh/></button>
                  <button className="btn sm primary" onClick={runNow}><I.play/> Run now</button>
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
                  const dur = r.duration_ms ? `${(r.duration_ms / 1000).toFixed(2)}s` : "—";
                  const st = (r.status || r.outcome || "ok").toLowerCase();
                  const tok = r.total_tokens || r.tokens || "—";
                  return (
                    <tr key={id}>
                      <td className="mono">{id}</td>
                      <td><span className="badge plain">{trig}</span></td>
                      <td className="mono muted">{t}</td>
                      <td className="mono">{dur}</td>
                      <td>{st === "ok" || st === "success" ? <span className="badge live">ok</span> : <span className="badge warn">{st}</span>}</td>
                      <td className="num mono">{tok}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      </div>

      {showCreate && <WorkflowCreateModal onClose={() => setShowCreate(false)} onCreated={(id) => { setShowCreate(false); setActiveId(id); refreshList(); }}/>}
      {showRunInput && active && <WorkflowRunModal workflow={active} onClose={() => setShowRunInput(false)} onRun={runWith}/>}
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
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-head"><b className="mono">Run workflow · {workflow.name || workflow.id}</b><button className="icon-btn" onClick={onClose}><I.close/></button></div>
        <div className="modal-body">
          <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase"}}>Input (JSON)</span>
          <textarea className="modal-field modal-textarea mt-8" style={{fontFamily:"var(--ff-mono)"}} value={inputJson} onChange={e => setInputJson(e.target.value)}/>
          {err && <div className="banner mt-12" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
            <span className="dot err"/><span className="banner-title">ERROR</span>
            <span className="banner-body mono" style={{fontSize:11}}>{err}</span></div>}
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={onClose}>Cancel</button>
          <button className="btn primary" onClick={run} disabled={busy}>{busy ? "Running…" : "Run"}</button>
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
        <div className="card flush">
          <table className="tbl">
            <thead><tr>
              <th>ID</th><th>Schedule</th><th>Action</th><th>Next</th><th className="right">Fires</th><th>Enabled</th><th></th>
            </tr></thead>
            <tbody>
              {!cronResp && (<tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
              {cronResp && cron.length === 0 && (
                <tr><td colSpan={7} style={{padding:"24px 14px", textAlign:"center"}}>
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
                return (
                  <tr key={c.id}>
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
      )}

      {tab === "triggers" && (
        <div className="card flush">
          <table className="tbl">
            <thead><tr>
              <th>ID</th><th>Kind</th><th>Target</th><th className="right">Fired</th><th>Last</th><th>Status</th><th></th>
            </tr></thead>
            <tbody>
              {!trigResp && (<tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>)}
              {trigResp && triggers.length === 0 && (
                <tr><td colSpan={7} style={{padding:"24px 14px", textAlign:"center"}}>
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
                return (
                  <tr key={t.id}>
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
            <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", marginTop:6}}>Schedule</span>
            <div className="seg">
              <button className={scheduleKind==="cron"?"on":""} onClick={() => setScheduleKind("cron")}>Cron</button>
              <button className={scheduleKind==="every"?"on":""} onClick={() => setScheduleKind("every")}>Every</button>
              <button className={scheduleKind==="at"?"on":""} onClick={() => setScheduleKind("at")}>At</button>
            </div>
            {scheduleKind === "cron" && <input className="modal-field" placeholder="0 9 * * *" value={cronExpr} onChange={e => setCronExpr(e.target.value)}/>}
            {scheduleKind === "every" && <input className="modal-field" type="number" min={60} value={everySecs} onChange={e => setEverySecs(e.target.value)} placeholder="seconds (60..86400)"/>}
            {scheduleKind === "at" && <input className="modal-field" type="datetime-local" value={atIso} onChange={e => setAtIso(e.target.value)}/>}

            <span className="muted mono" style={{fontSize:10.5, letterSpacing:".12em", textTransform:"uppercase", marginTop:6}}>Action</span>
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
                <div className="banner mb-12" style={{borderColor: testResult.ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)"}}>
                  <span className={"dot " + (testResult.busy ? "warn" : testResult.ok ? "live" : "err")}/>
                  <span className="banner-title">{testResult.busy ? "TESTING" : testResult.ok ? "OK" : "FAIL"}</span>
                  <span className="banner-body mono" style={{fontSize:11}}>{testResult.message || testResult.detail || (testResult.busy ? "…" : "")}</span>
                </div>
              )}
              <div className="row gap-6">
                {ch.configured ? (
                  <>
                    <button className="btn sm" onClick={() => testChannel(ch.name)}>Test</button>
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
                <div className="kv mb-12">
                  <dt>id</dt><dd>{active.id}</dd>
                  <dt>kind</dt><dd>{active.type || "—"}</dd>
                  <dt>degree</dt><dd>{activeEdges.length}</dd>
                </div>
                <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Edges ({activeEdges.length})</div>
                <div className="col gap-4" style={{maxHeight:240, overflow:"auto"}}>
                  {activeEdges.slice(0, 16).map((e, i) => {
                    const src = e.source || e.source_id;
                    const dst = e.target || e.target_id;
                    const other = src === active.id ? dst : src;
                    return (
                      <div key={i} className="row between" style={{padding:"5px 8px",background:"var(--bg-2)",borderRadius:5,fontSize:11.5,fontFamily:"var(--ff-mono)"}}>
                        <span style={{color:"var(--rust)"}}>{e.relation || e.label || "→"}</span>
                        <span className="muted">→ {nodes.find(n => n.id === other)?.name || other}</span>
                      </div>
                    );
                  })}
                </div>
              </>
            )}
          </div>
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
                <tr key={s.name}>
                  <td><span className="mono"><span style={{color:"var(--rust)"}}>›</span> {s.name}</span></td>
                  <td>
                    {origin === "clawhub" || origin === "claw" ? <span className="badge violet">ClawHub</span>
                      : origin === "privileged" ? <span className="badge warn">privileged</span>
                      : <span className="badge plain">{origin}</span>}
                  </td>
                  <td className="muted mono">{s.runtime || cat}</td>
                  <td className="mono">{ver}</td>
                  <td><div className={"switch " + (en ? "on" : "")}/></td>
                  <td className="right" style={{position:"relative"}}>
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
              return (
                <div key={slug} className="row gap-12" style={{padding:"10px 12px", border:"1px solid var(--border)", borderRadius:7, background:"var(--bg-2)"}}>
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
                    <button className="btn sm primary" onClick={() => install(slug)} disabled={!!installing}>
                      {isInstalling ? "Installing…" : "Install"}
                    </button>
                  </div>
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
  const rows = (resp && resp.approvals) || [];
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
      <div className="card flush">
        {!resp && <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>loading…</div>}
        {resp && rows.length === 0 && <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>No approvals waiting.</div>}
        {rows.length > 0 && <ApprovalsTable rows={rows} onChange={refresh}/>}
      </div>
    </div>
  );
}

/* ============================== AUDIT ============================== */
function AuditPage() {
  const [audit, , refresh] = usePolling("/api/audit/recent?n=50", 8000);
  const [verify, verifyErr, verifyRefresh] = useApi("/api/audit/verify");

  const entries = (audit && audit.entries) || [];
  // Compute actor stats over the loaded window. Cheap heuristic — full
  // historical aggregation would need a server endpoint.
  const actorCounts = {};
  for (const e of entries) {
    const a = e.agent_name || e.agent_id || "kernel";
    actorCounts[a] = (actorCounts[a] || 0) + 1;
  }
  const topActor = Object.entries(actorCounts).sort((a, b) => b[1] - a[1])[0];

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Audit log</h1>
          <p className="page-sub">Merkle hash chain · <span className="mono">~/.rustyhand/data/audit.jsonl</span> · replayed on boot</p>
        </div>
        <div className="page-actions">
          <button className="btn ghost" onClick={() => { refresh(); verifyRefresh(); }}><I.refresh/></button>
          <button className="btn ghost" onClick={verifyRefresh}><I.shield/> Verify chain</button>
          <button className="btn ghost" onClick={() => {
            if (!audit) return;
            downloadBlob(`rustyhand-audit-${new Date().toISOString().slice(0, 10)}.json`,
              JSON.stringify(audit, null, 2), "application/json");
          }}><I.download/> Export</button>
        </div>
      </div>

      <div className="grid-12">
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Chain head</div>
          <div className="mono" style={{fontSize:14, color:"var(--rust)", wordBreak:"break-all"}}>{audit && audit.tip_hash ? String(audit.tip_hash).slice(0, 16) : "—"}</div>
          <div className="dim mono mt-4" style={{fontSize:11}}>depth {audit ? (audit.total != null ? audit.total.toLocaleString() : "—") : "…"}</div>
          <div className="divider"/>
          {verify ? (
            <div className="row gap-6">
              {verify.valid
                ? <span className="badge live"><I.check/> verified</span>
                : <span className="badge error"><I.warn/> mismatch</span>}
              <span className="dim mono" style={{fontSize:11}}>{(verify.entries || []).length || verify.total || 0} entries</span>
            </div>
          ) : <div className="dim mono" style={{fontSize:11}}>{verifyErr || "verifying…"}</div>}
        </div>
        <div className="col-3 card">
          <div className="muted mono mb-8" style={{fontSize:10.5,letterSpacing:".12em",textTransform:"uppercase"}}>Loaded window</div>
          <div className="mono" style={{fontSize:20}}>{entries.length} <span className="dim" style={{fontSize:13}}>entries</span></div>
          <div className="dim mono mt-4" style={{fontSize:11}}>from /api/audit/recent?n=50</div>
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
          {audit && entries.length === 0 && <div className="muted mono" style={{padding:"16px 14px", fontSize:12}}>No audit entries yet.</div>}
          {entries.map((a) => {
            const hash = a.hash ? String(a.hash).slice(0, 12) : "—";
            return (
              <div key={a.hash || a.seq} className="merkle-row">
                <div className="chain"/>
                <span className="time">{formatTime(a.timestamp)}</span>
                <span>
                  <span className="action">{a.action}</span>{" "}
                  <span className="dim">·</span>{" "}
                  <span className="actor">{a.agent_name || a.agent_id || "kernel"}</span>
                  <div className="dim" style={{fontSize:11,marginTop:2}}>{a.detail || a.outcome || ""}</div>
                </span>
                <span className="hash">{hash}</span>
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
  const [editing, setEditing] = useState(null);

  const providers = (providersResp && providersResp.providers) || [];

  const apiListen = (config && (config.api_listen || (config.api && config.api.listen))) || "—";
  const proxy = (config && (config.proxy_url || (config.proxy && config.proxy.url))) || null;
  const version = (health && health.version) || "0.7.48";
  const uptime = health && health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "—";
  const agentCount = health && health.agent_count != null ? health.agent_count : "—";

  return (
    <div>
      <div className="page-head">
        <div>
          <h1 className="page-title">Settings</h1>
          <p className="page-sub">Config at <span className="mono">~/.rustyhand/config.toml</span> · 50+ fields with serde defaults · live from <span className="mono">/api/config</span></p>
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
              <dt>agents</dt><dd>{onboarding ? onboarding.agent_count : "…"}</dd>
            </div>
            <div className="dim mt-8" style={{fontSize:11}}>set <span className="mono">RUSTYHAND_DISABLE_DEMO_MODE=1</span> to fall back to NullDriver</div>
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
        </div>
      </div>

      {editing && <ProviderKeyModal provider={editing} onClose={() => setEditing(null)} onSaved={() => { setEditing(null); refreshProviders(); }}/>}
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
            <th>Session</th><th>Agent</th><th>Label</th><th className="right">Messages</th><th>Created</th><th>Updated</th><th></th>
          </tr></thead>
          <tbody>
            {!resp && <tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>}
            {resp && sessions.length === 0 && <tr><td colSpan={7} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>No sessions yet.</td></tr>}
            {sessions.map(s => (
              <tr key={s.session_id}>
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
  const connectedNames = new Set(connected.map(c => c.name));
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
          const isConnected = connectedNames.has(s.name);
          const transport = s.transport || {};
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
              </div>
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

/* ============================== BINDINGS ============================== */
function BindingsPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/bindings", 30000);
  const bindings = (resp && resp.bindings) || [];

  const remove = async (index) => {
    if (!(await confirmDialog({ title: "Remove binding", message: `Remove binding #${index}?`, danger: true, confirmLabel: "Remove" }))) return;
    try {
      await rhFetch(`/api/bindings/${index}`, { method: "DELETE" });
      toastOk("Binding removed");
      refresh();
    } catch (e) { toastErr(`Remove failed: ${e.message || e}`); }
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
      {fetchErr && <div className="banner" style={{borderColor:"oklch(0.66 0.18 25 / .35)"}}>
        <span className="dot err"/><span className="banner-title">API ERROR</span>
        <span className="banner-body mono" style={{fontSize:11}}>{fetchErr}</span>
      </div>}
      <div className="card flush">
        <table className="tbl">
          <thead><tr><th>#</th><th>Agent</th><th>Kind</th><th>Target</th><th>Pattern</th><th></th></tr></thead>
          <tbody>
            {!resp && <tr><td colSpan={6} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>loading…</td></tr>}
            {resp && bindings.length === 0 && <tr><td colSpan={6} className="muted mono" style={{padding:"12px 14px", fontSize:12, textAlign:"center"}}>No bindings configured.</td></tr>}
            {bindings.map((b, i) => (
              <tr key={i}>
                <td className="num mono">{i}</td>
                <td className="mono">{b.agent_id || b.agent || "—"}</td>
                <td className="mono">{b.kind || b.type || "—"}</td>
                <td className="mono">{b.target || b.channel || b.trigger || "—"}</td>
                <td className="mono dim" style={{maxWidth:280, overflow:"hidden", textOverflow:"ellipsis"}}>{b.pattern ? JSON.stringify(b.pattern) : "—"}</td>
                <td className="right">
                  <button className="btn sm danger" onClick={() => remove(i)} title="Remove"><I.close/></button>
                </td>
              </tr>
            ))}
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
  McpPage, NetworkPage, BindingsPage,
});
