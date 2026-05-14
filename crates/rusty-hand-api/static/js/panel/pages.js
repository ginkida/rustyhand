const { useState, useEffect, useMemo, useRef } = React;
const D = window.RH_DATA;
function OverviewPage({ go }) {
  var _a;
  const [agentsResp, , refreshAgents] = usePolling("/api/agents?limit=100", 15e3);
  const [health] = usePolling("/api/health/detail", 1e4);
  const [audit, , refreshAudit] = usePolling("/api/audit/recent?n=12", 8e3);
  const [approvalsResp, , refreshApprovals] = usePolling("/api/approvals", 15e3);
  const [onboarding] = useApi("/api/onboarding");
  const [usage] = usePolling("/api/usage/daily", 6e4);
  const [providersResp] = useApi("/api/providers");
  const agents = agentsResp && agentsResp.agents ? agentsResp.agents.map(normalizeAgent) : D.agents;
  const totalAgents = (_a = agentsResp && agentsResp.total) != null ? _a : agents.length;
  const live = agents.filter((a) => a.state === "running").length;
  const errors = agents.filter((a) => a.state === "error").length;
  const days = usage && usage.days || [];
  const cost24 = usage ? usage.today_cost_usd || 0 : D.costSeries.reduce((s, v) => s + v, 0);
  const ticks24 = usage && usage.ticks_today || 0;
  const refresh = () => {
    refreshAgents();
    refreshAudit();
    refreshApprovals();
  };
  const approvalRows = approvalsResp && approvalsResp.approvals || D.approvals;
  const version = health && health.version || "0.7.46";
  const uptime = health && health.uptime_seconds ? formatUptime(health.uptime_seconds) : null;
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement(Banner, { go, onboarding }), /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Overview"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "System pulse \xB7 kernel ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "v", version), uptime && /* @__PURE__ */ React.createElement(React.Fragment, null, " \xB7 uptime ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, uptime)), " ", "\xB7 schema ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "v8"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null), " Refresh"), /* @__PURE__ */ React.createElement("button", { className: "btn primary" }, /* @__PURE__ */ React.createElement(I.plus, null), " New agent"))), /* @__PURE__ */ React.createElement("div", { className: "tiles" }, /* @__PURE__ */ React.createElement(Tile, { label: "Agents running", value: `${live}`, foot: `of ${totalAgents} total`, spark: sparkOf(agents.map((_, i) => live + (i % 3 - 1)), 12) }), /* @__PURE__ */ React.createElement(Tile, { label: "Cost \xB7 today", value: `$${(cost24 || 0).toFixed(2)}`, foot: usage ? "from /api/usage/daily" : "loading\u2026", spark: days.slice(-12).map((d) => d.cost_usd || 0) }), /* @__PURE__ */ React.createElement(Tile, { label: "Audit entries", value: audit && audit.total != null ? audit.total.toLocaleString() : "\u2014", foot: audit && audit.tip_hash ? `tip ${String(audit.tip_hash).slice(0, 8)}` : "loading\u2026", spark: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1] }), /* @__PURE__ */ React.createElement(Tile, { label: "Errors \xB7 agents", value: `${errors}`, foot: errors ? `${errors} crashed agent(s)` : "all green", spark: [0, 0, 1, 0, 0, 0, 0, 0, errors, 0, 0, 0], deltaCls: "up", warn: errors > 0 })), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 col" }, /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Live activity"), /* @__PURE__ */ React.createElement("div", { className: "ch-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: refreshAudit }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost" }, /* @__PURE__ */ React.createElement(I.download, null), " Export"))), /* @__PURE__ */ React.createElement(ActivityFeed, { entries: audit && audit.entries })), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Approvals \xB7 waiting"), /* @__PURE__ */ React.createElement("div", { className: "ch-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => go("approvals") }, "Open queue"))), approvalRows.length === 0 ? /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No approvals waiting.") : /* @__PURE__ */ React.createElement(ApprovalsTable, { rows: approvalRows.slice(0, 3), compact: true, onChange: refreshApprovals }))), /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, onboarding && onboarding.demo_mode && /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Demo seed"), /* @__PURE__ */ React.createElement("span", { className: "badge demo" }, /* @__PURE__ */ React.createElement("span", { className: "dot demo" }), "active")), /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement(SeedRow, { icon: /* @__PURE__ */ React.createElement(I.agents, null), title: "rusty", sub: "welcome agent \xB7 chat-ready", onClick: () => go("chat") }), /* @__PURE__ */ React.createElement(SeedRow, { icon: /* @__PURE__ */ React.createElement(I.workflows, null), title: "demo-pipeline", sub: "2-step sample workflow \xB7 click to run", onClick: () => go("workflows") }), /* @__PURE__ */ React.createElement(SeedRow, { icon: /* @__PURE__ */ React.createElement(I.event, null), title: "sample trigger", sub: "agent-spawn on webhook", onClick: () => go("automation") }), /* @__PURE__ */ React.createElement(SeedRow, { icon: /* @__PURE__ */ React.createElement(I.cron, null), title: "demo-daily-ping", sub: "cron 0 9 * * * \xB7 disabled", onClick: () => go("automation") }))), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Providers"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => go("settings") }, "Manage")), /* @__PURE__ */ React.createElement(ProvidersList, { providers: providersResp && providersResp.providers })), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Audit chain"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => go("audit") }, "View")), /* @__PURE__ */ React.createElement("div", { className: "col gap-4" }, /* @__PURE__ */ React.createElement(AuditSummary, { audit }))))));
}
function formatUptime(s) {
  if (s == null) return null;
  const d = Math.floor(s / 86400);
  const h = Math.floor(s % 86400 / 3600);
  const m = Math.floor(s % 3600 / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${s}s`;
}
function sparkOf(arr, n) {
  if (!arr || arr.length === 0) return Array(n).fill(0);
  if (arr.length >= n) return arr.slice(-n);
  return Array(n - arr.length).fill(arr[0]).concat(arr);
}
const Banner = ({ go, onboarding }) => {
  if (!onboarding || !onboarding.demo_mode) return null;
  return /* @__PURE__ */ React.createElement("div", { className: "banner" }, /* @__PURE__ */ React.createElement("span", { className: "dot demo" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "DEMO MODE"), /* @__PURE__ */ React.createElement("span", { className: "banner-body" }, "Provider ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, onboarding.provider || "mock"), onboarding.api_key_set === false && /* @__PURE__ */ React.createElement(React.Fragment, null, " \u2014 no API key set"), " ", "\xB7 ", onboarding.agent_count || 0, " agent(s) loaded."), /* @__PURE__ */ React.createElement("span", { className: "banner-cta" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => go("settings") }, "Add API key")));
};
const ProvidersList = ({ providers }) => {
  if (!providers) {
    return /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, ["anthropic", "openai", "deepseek", "ollama", "mock"].map((n) => /* @__PURE__ */ React.createElement(ProviderRow, { key: n, name: n, state: "idle", detail: "loading\u2026" })));
  }
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, providers.slice(0, 6).map((p) => {
    const id = p.id || p.name || "\u2014";
    const auth = (p.auth_status || "").toLowerCase();
    let state = "idle";
    if (auth === "ok" || auth === "configured" || auth === "set") state = "connected";
    else if (auth === "local") state = "local";
    else if (auth === "fallback" || id === "mock") state = "fallback";
    else if (auth === "missing" && p.key_required === false) state = "local";
    const models = p.model_count != null ? `${p.model_count} model${p.model_count === 1 ? "" : "s"}` : "";
    const env = p.api_key_env && p.api_key_env !== "\u2014" ? p.api_key_env : "";
    const detail = [models, env].filter(Boolean).join(" \xB7 ") || "\u2014";
    return /* @__PURE__ */ React.createElement(
      ProviderRow,
      {
        key: id,
        name: p.display_name || id,
        state,
        detail
      }
    );
  }));
};
const AuditSummary = ({ audit }) => {
  if (!audit) {
    return /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "head"), /* @__PURE__ */ React.createElement("dd", { className: "dim" }, "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "length"), /* @__PURE__ */ React.createElement("dd", { className: "dim" }, "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "verified"), /* @__PURE__ */ React.createElement("dd", { className: "dim" }, "checking\u2026"), /* @__PURE__ */ React.createElement("dt", null, "file"), /* @__PURE__ */ React.createElement("dd", { className: "dim" }, "~/.rustyhand/data/audit.jsonl"));
  }
  const head = audit.tip_hash ? String(audit.tip_hash).slice(0, 8) : "\u2014";
  return /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "head"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, head), /* @__PURE__ */ React.createElement("dt", null, "length"), /* @__PURE__ */ React.createElement("dd", null, (audit.total != null ? audit.total : (audit.entries || []).length).toLocaleString()), /* @__PURE__ */ React.createElement("dt", null, "verified"), /* @__PURE__ */ React.createElement("dd", { style: { color: "var(--live)" } }, "\u2713 live"), /* @__PURE__ */ React.createElement("dt", null, "file"), /* @__PURE__ */ React.createElement("dd", { className: "dim" }, "~/.rustyhand/data/audit.jsonl"));
};
const Tile = ({ label, value, foot, spark, delta, deltaCls = "up", warn }) => /* @__PURE__ */ React.createElement("div", { className: "tile" }, /* @__PURE__ */ React.createElement("div", { className: "tile-label" }, label, warn && /* @__PURE__ */ React.createElement(I.warn, null)), /* @__PURE__ */ React.createElement("div", { className: "tile-value" }, value), /* @__PURE__ */ React.createElement("div", { className: "tile-foot" }, delta && /* @__PURE__ */ React.createElement("span", { className: `delta ${deltaCls}` }, delta), " ", /* @__PURE__ */ React.createElement("span", { className: "dim" }, "\xB7 ", foot)), /* @__PURE__ */ React.createElement("div", { className: "tile-spark" }, /* @__PURE__ */ React.createElement(Spark, { data: spark, width: 88, height: 28, color: "var(--rust)" })));
const SeedRow = ({ icon, title, sub, onClick }) => /* @__PURE__ */ React.createElement("button", { onClick, className: "row gap-8", style: {
  padding: "8px 10px",
  borderRadius: 7,
  border: "1px solid var(--border)",
  background: "var(--bg-2)",
  textAlign: "left",
  width: "100%"
} }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)", display: "grid", placeItems: "center", width: 22 } }, icon), /* @__PURE__ */ React.createElement("span", { className: "col", style: { gap: 1 } }, /* @__PURE__ */ React.createElement("span", { style: { fontFamily: "var(--ff-mono)", fontSize: 12 } }, title), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11 } }, sub)), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto", color: "var(--fg-3)" } }, /* @__PURE__ */ React.createElement(I.arrowR, null)));
const ProviderRow = ({ name, state, detail }) => {
  const map = { connected: "live", local: "sky", fallback: "demo" };
  return /* @__PURE__ */ React.createElement("div", { className: "row between", style: { padding: "6px 0", borderBottom: "1px solid var(--border)" } }, /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2 } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, name), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11 } }, detail)), /* @__PURE__ */ React.createElement("span", { className: `badge ${map[state] || "idle"}` }, state));
};
function actionColor(action) {
  const a = (action || "").toLowerCase();
  if (a.includes("spawn") || a.includes("boot") || a.includes("closed")) return "live";
  if (a.includes("trigger") || a.includes("workflow")) return "violet";
  if (a.includes("approval") || a.includes("denied") || a.includes("reject")) return "amber";
  if (a.includes("error") || a.includes("panic") || a.includes("crash")) return "muted";
  return "muted";
}
function formatTime(ts) {
  if (!ts) return "\u2014";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return String(ts).slice(11, 19);
  return d.toLocaleTimeString("en-GB", { hour12: false });
}
const ActivityFeed = ({ entries }) => {
  const cls = (c) => ({ live: "var(--live)", violet: "var(--violet)", amber: "var(--amber)", muted: "var(--fg-3)" })[c];
  if (!entries) {
    return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "Loading audit chain\u2026");
  }
  if (entries.length === 0) {
    return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No audit entries yet \u2014 they appear as agents act.");
  }
  return /* @__PURE__ */ React.createElement("div", { style: { maxHeight: 360, overflow: "auto" } }, entries.map((it, i) => {
    const color = actionColor(it.action);
    const detail = it.detail || it.outcome || "";
    return /* @__PURE__ */ React.createElement(
      "div",
      {
        key: it.hash || it.seq || i,
        className: "row",
        style: { padding: "9px 14px", borderBottom: "1px solid var(--border)", gap: 12 }
      },
      /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, width: 70 } }, formatTime(it.timestamp)),
      /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12, width: 140, color: cls(color), overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, it.agent_name || it.agent_id || "kernel"),
      /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12, width: 170, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, it.action),
      /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11.5, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, detail)
    );
  }));
};
const ApprovalsTable = ({ rows, compact, onChange }) => {
  const decide = async (id, verdict) => {
    try {
      await rhFetch(`/api/approvals/${id}/${verdict}`, { method: "POST" });
      if (onChange) onChange();
    } catch (e) {
      console.warn(`approval ${verdict} failed`, e);
    }
  };
  return /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Action"), /* @__PURE__ */ React.createElement("th", null, "Risk"), /* @__PURE__ */ React.createElement("th", null, "Age"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Decide"))), /* @__PURE__ */ React.createElement("tbody", null, rows.map((r) => {
    const agent = r.agent_name || r.agent || r.agent_id || "\u2014";
    const age = r.age || relativeTime(r.requested_at || r.created_at);
    const risk = (r.risk || "low").toLowerCase();
    return /* @__PURE__ */ React.createElement("tr", { key: r.id }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, r.id), /* @__PURE__ */ React.createElement("td", { className: "mono" }, agent), /* @__PURE__ */ React.createElement("td", null, r.action), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: `badge ${risk === "high" ? "error" : risk === "medium" ? "warn" : "idle"}` }, risk)), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, age), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", style: { height: 24, padding: "2px 8px" }, onClick: () => decide(r.id, "approve") }, "Approve"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", style: { height: 24, padding: "2px 8px", marginLeft: 6 }, onClick: () => decide(r.id, "reject") }, "Reject")));
  })));
};
function Pagination({ pg }) {
  if (!pg || pg.total === 0) return null;
  return /* @__PURE__ */ React.createElement("div", { className: "row gap-8", style: { padding: "10px 14px", borderTop: "1px solid var(--border)", justifyContent: "flex-end" } }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, pg.offset + 1, "\u2013", Math.min(pg.offset + pg.pageSize, pg.total), " of ", pg.total.toLocaleString()), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: pg.prev, disabled: !pg.hasPrev }, "\u2190 Prev"), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5, minWidth: 60, textAlign: "center" } }, pg.page, " / ", pg.totalPages), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: pg.next, disabled: !pg.hasNext }, "Next \u2192"));
}
function AgentsPage({ openAgent }) {
  const [filter, setFilter] = useState("all");
  const [q, setQ] = useState("");
  const [showSpawn, setShowSpawn] = useState(false);
  const [rowMenu, setRowMenu] = useState(null);
  const pg = usePagination(50);
  const [resp, fetchErr, refresh] = usePolling(
    `/api/agents?limit=${pg.pageSize}&offset=${pg.offset}`,
    15e3
  );
  React.useEffect(() => {
    if (resp && resp.total != null) pg.setTotal(resp.total);
  }, [resp && resp.total]);
  const agents = resp && resp.agents ? resp.agents.map(normalizeAgent) : D.agents;
  const filtered = agents.filter((a) => {
    if (filter !== "all" && a.state !== filter && !(filter === "running" && a.state === "running")) return false;
    if (q && !a.name.toLowerCase().includes(q.toLowerCase()) && !a.model.toLowerCase().includes(q.toLowerCase())) return false;
    return true;
  });
  const killAgent = async (id) => {
    if (!confirm(`Kill agent ${id}?`)) return;
    try {
      await rhFetch(`/api/agents/${encodeURIComponent(id)}`, { method: "DELETE" });
      refresh();
    } catch (e) {
      toastErr(`kill failed: ${e.message || e}`);
    }
  };
  const restartAgent = async (id) => {
    try {
      await rhFetch(`/api/agents/${encodeURIComponent(id)}/restart`, { method: "POST" });
      refresh();
    } catch (e) {
      toastErr(`restart failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Agents ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", agents.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Manifests live in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "agents/<name>/agent.toml"), ". Hot-reloaded on save.")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowSpawn(true) }, /* @__PURE__ */ React.createElement(I.copy, null), " Templates"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowSpawn(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Spawn agent"))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body" }, "Failed to load agents from kernel: ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, fetchErr))), /* @__PURE__ */ React.createElement("div", { className: "filter-bar" }, /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: filter === "all" ? "on" : "", onClick: () => setFilter("all") }, "All \xB7 ", agents.length), /* @__PURE__ */ React.createElement("button", { className: filter === "running" ? "on" : "", onClick: () => setFilter("running") }, "Live \xB7 ", agents.filter((a) => a.state === "running").length), /* @__PURE__ */ React.createElement("button", { className: filter === "error" ? "on" : "", onClick: () => setFilter("error") }, "Errors \xB7 ", agents.filter((a) => a.state === "error").length), /* @__PURE__ */ React.createElement("button", { className: filter === "idle" ? "on" : "", onClick: () => setFilter("idle") }, "Idle \xB7 ", agents.filter((a) => a.state === "idle").length)), /* @__PURE__ */ React.createElement("div", { className: "search-field" }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("input", { placeholder: "Find agent, group, model\u2026", value: q, onChange: (e) => setQ(e.target.value) }), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u2318K"))), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Group"), /* @__PURE__ */ React.createElement("th", null, "Model"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Msgs"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Cost \xB7 24h"), /* @__PURE__ */ React.createElement("th", null, "Last activity"), /* @__PURE__ */ React.createElement("th", null, "Updated"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, filtered.map((a) => /* @__PURE__ */ React.createElement("tr", { key: a.id, style: { cursor: "pointer" }, onClick: () => openAgent(a) }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "agent-row" }, /* @__PURE__ */ React.createElement(Avatar, { agent: a }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "name" }, a.name), /* @__PURE__ */ React.createElement("div", { className: "meta" }, a.id)))), /* @__PURE__ */ React.createElement("td", { className: "muted mono" }, a.group), /* @__PURE__ */ React.createElement("td", { className: "mono" }, a.model, /* @__PURE__ */ React.createElement("div", { className: "meta dim" }, a.provider)), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(StateBadge, { state: a.state })), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, a.messages.toLocaleString()), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, "$", a.cost.toFixed(2)), /* @__PURE__ */ React.createElement("td", { className: "muted", style: { maxWidth: 280, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" } }, a.last), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, a.updated), /* @__PURE__ */ React.createElement("td", { style: { position: "relative" } }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: (e) => {
    e.stopPropagation();
    setRowMenu(rowMenu === a.id ? null : a.id);
  } }, /* @__PURE__ */ React.createElement(I.more, null)), rowMenu === a.id && /* @__PURE__ */ React.createElement("div", { className: "row-menu", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("button", { onClick: () => {
    setRowMenu(null);
    restartAgent(a.id);
  } }, /* @__PURE__ */ React.createElement(I.refresh, null), " Restart"), /* @__PURE__ */ React.createElement("button", { onClick: () => {
    setRowMenu(null);
    killAgent(a.id);
  }, style: { color: "var(--crimson)" } }, /* @__PURE__ */ React.createElement(I.close, null), " Kill"))))))), /* @__PURE__ */ React.createElement(Pagination, { pg })), showSpawn && /* @__PURE__ */ React.createElement(SpawnAgentModal, { onClose: () => setShowSpawn(false), onSpawned: () => {
    setShowSpawn(false);
    refresh();
  } }));
}
function SpawnAgentModal({ onClose, onSpawned }) {
  useEscapeKey(onClose);
  const [templates] = useApi("/api/templates");
  const [profiles] = useApi("/api/profiles");
  const [mode, setMode] = useState("template");
  const [selectedTemplate, setSelectedTemplate] = useState(null);
  const [customName, setCustomName] = useState("");
  const [customProfile, setCustomProfile] = useState("research");
  const [customModel, setCustomModel] = useState("claude-sonnet-4");
  const [customProvider, setCustomProvider] = useState("anthropic");
  const [customManifest, setCustomManifest] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const tmplList = Array.isArray(templates) ? templates : templates && templates.templates || [];
  const profileList = Array.isArray(profiles) ? profiles : profiles && profiles.profiles || [];
  React.useEffect(() => {
    if (mode !== "template" || !selectedTemplate) return;
    let aborted = false;
    rhFetch(`/api/templates/${encodeURIComponent(selectedTemplate)}`).then((d) => {
      if (!aborted) setCustomManifest(d.manifest_toml || "");
    }).catch((e) => {
      if (!aborted) setErr(String(e.message || e));
    });
    return () => {
      aborted = true;
    };
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
      const manifest_toml = mode === "template" && customManifest || (mode === "custom" ? generateManifest() : customManifest);
      if (!manifest_toml.trim()) throw new Error("manifest is empty");
      await rhFetch("/api/agents", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ manifest_toml })
      });
      onSpawned();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Spawn agent"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "tabs", style: { marginBottom: 14 } }, /* @__PURE__ */ React.createElement("button", { className: mode === "template" ? "on" : "", onClick: () => setMode("template") }, "From template"), /* @__PURE__ */ React.createElement("button", { className: mode === "custom" ? "on" : "", onClick: () => setMode("custom") }, "Custom")), mode === "template" && /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Templates (", tmplList.length, ")"), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 200, overflow: "auto" } }, tmplList.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading templates\u2026"), tmplList.map((t) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: t.name,
      onClick: () => setSelectedTemplate(t.name),
      className: "row gap-8",
      style: {
        padding: "6px 10px",
        border: "1px solid var(--border)",
        borderRadius: 6,
        background: selectedTemplate === t.name ? "var(--surface-2)" : "var(--bg-2)",
        textAlign: "left"
      }
    },
    /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, t.name),
    t.description && /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11, marginLeft: "auto", maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, t.description)
  ))), customManifest && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Manifest preview"), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 160, overflow: "auto" } }, customManifest))), mode === "custom" && /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "my-agent", value: customName, onChange: (e) => setCustomName(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Provider"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: customProvider, onChange: (e) => setCustomProvider(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Model"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: customModel, onChange: (e) => setCustomModel(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Tool profile"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: customProfile, onChange: (e) => setCustomProfile(e.target.value) }, profileList.map((p) => /* @__PURE__ */ React.createElement("option", { key: p.name || p, value: p.name || p }, p.name || p)))), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Manifest preview"), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 160, overflow: "auto" } }, generateManifest())), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { marginTop: 12, borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", disabled: busy || mode === "template" && !selectedTemplate, onClick: spawn }, busy ? "Spawning\u2026" : "Spawn"))));
}
function AgentDrawer({ agent, onClose }) {
  const [tab, setTab] = useState("info");
  useEffect(() => {
    setTab("info");
  }, [agent && agent.id]);
  if (!agent) return null;
  const [detail, , refreshDetail] = useApi(agent ? `/api/agents/${agent.id}` : null);
  const [recent] = useApi(agent ? `/api/audit/recent?n=10&agent_id=${agent.id}` : null);
  const [budget] = useApi(agent ? `/api/budget/agents/${agent.id}` : null);
  const turns = recent && recent.entries || null;
  const cost24 = budget && budget.daily && budget.daily.spend != null ? Number(budget.daily.spend) : agent.cost;
  return /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "drawer-back " + (agent ? "open" : ""), onClick: onClose }), /* @__PURE__ */ React.createElement("aside", { className: "drawer " + (agent ? "open" : "") }, /* @__PURE__ */ React.createElement("div", { className: "drawer-head" }, /* @__PURE__ */ React.createElement(Avatar, { agent, size: "lg" }), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2 } }, /* @__PURE__ */ React.createElement("div", { style: { fontFamily: "var(--ff-mono)", fontSize: 15 } }, agent.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, agent.id, " \xB7 ", agent.group)), /* @__PURE__ */ React.createElement("div", { style: { marginLeft: "auto" }, className: "row gap-6" }, /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null)))), /* @__PURE__ */ React.createElement("div", { className: "tabs", style: { margin: "0 16px" } }, /* @__PURE__ */ React.createElement("button", { className: tab === "info" ? "on" : "", onClick: () => setTab("info") }, "Info"), /* @__PURE__ */ React.createElement("button", { className: tab === "config" ? "on" : "", onClick: () => setTab("config") }, "Config"), /* @__PURE__ */ React.createElement("button", { className: tab === "identity" ? "on" : "", onClick: () => setTab("identity") }, "Identity"), /* @__PURE__ */ React.createElement("button", { className: tab === "activity" ? "on" : "", onClick: () => setTab("activity") }, "Activity")), /* @__PURE__ */ React.createElement("div", { className: "drawer-body" }, tab === "info" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mb-12" }, /* @__PURE__ */ React.createElement(StateBadge, { state: agent.state }), /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, agent.model), /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, agent.provider)), /* @__PURE__ */ React.createElement("div", { className: "kv mb-16" }, /* @__PURE__ */ React.createElement("dt", null, "messages"), /* @__PURE__ */ React.createElement("dd", null, (agent.messages || 0).toLocaleString()), /* @__PURE__ */ React.createElement("dt", null, "cost 24h"), /* @__PURE__ */ React.createElement("dd", null, "$", (cost24 || 0).toFixed(2)), /* @__PURE__ */ React.createElement("dt", null, "last"), /* @__PURE__ */ React.createElement("dd", null, agent.updated, " ago"), /* @__PURE__ */ React.createElement("dt", null, "circuit"), /* @__PURE__ */ React.createElement("dd", { style: { color: agent.state === "error" ? "var(--crimson)" : "var(--live)" } }, agent.state === "error" ? "OPEN" : "CLOSED")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Description"), /* @__PURE__ */ React.createElement("div", { className: "codebox mb-16", style: { whiteSpace: "pre-wrap" } }, detail && detail.description ? detail.description : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "System prompt"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 200 } }, detail && detail.model && detail.model.system_prompt ? detail.model.system_prompt : "(loading or no system prompt)")), tab === "config" && detail && /* @__PURE__ */ React.createElement(AgentConfigForm, { agent, detail, onSaved: refreshDetail }), tab === "config" && !detail && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), tab === "identity" && /* @__PURE__ */ React.createElement(AgentIdentityForm, { agent, detail, onSaved: refreshDetail }), tab === "activity" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Recent turns"), /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, !turns && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "6px 8px" } }, "loading audit\u2026"), turns && turns.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "6px 8px" } }, "no audit entries for this agent yet."), turns && turns.map((r, i) => /* @__PURE__ */ React.createElement("div", { key: r.hash || r.seq || i, className: "row", style: { padding: "6px 8px", borderRadius: 6, background: "var(--bg-2)" } }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, width: 70 } }, formatTime(r.timestamp)), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, r.action), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, marginLeft: 8, maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, r.detail || r.outcome || ""))))))));
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
  const save = async () => {
    setBusy(true);
    setErr(null);
    setOk(false);
    try {
      await rhFetch(`/api/agents/${agent.id}/config`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          group,
          description,
          system_prompt: systemPrompt,
          temperature: Number(temperature),
          max_tokens: Number(maxTokens),
          thinking_enabled: thinkingEnabled
        })
      });
      if (modelName && modelName !== model.model) {
        await rhFetch(`/api/agents/${agent.id}/model`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ model: modelName })
        });
      }
      setOk(true);
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Group"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: group, onChange: (e) => setGroup(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Description"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 60 }, value: description, onChange: (e) => setDescription(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "System prompt"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 120 }, value: systemPrompt, onChange: (e) => setSystemPrompt(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Model"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: modelName, onChange: (e) => setModelName(e.target.value) })), /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Temperature (0\u20132)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", step: "0.05", min: "0", max: "2", value: temperature, onChange: (e) => setTemperature(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Max tokens"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: "1", value: maxTokens, onChange: (e) => setMaxTokens(e.target.value) }))), /* @__PURE__ */ React.createElement("div", { className: "t-row" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Thinking enabled"), /* @__PURE__ */ React.createElement("div", { className: "switch " + (thinkingEnabled ? "on" : ""), onClick: () => setThinkingEnabled((v) => !v) })), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), ok && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.74 0.135 150 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "SAVED"), /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11.5 } }, "Hot-reloaded \u2014 next turn uses the new config.")), /* @__PURE__ */ React.createElement("div", { className: "row", style: { justifyContent: "flex-end", marginTop: 8 } }, /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : "Save config")));
}
function AgentIdentityForm({ agent, detail, onSaved }) {
  const id = detail && detail.identity || {};
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
    setBusy(true);
    setErr(null);
    setOk(false);
    try {
      await rhFetch(`/api/agents/${agent.id}/identity`, {
        method: "PATCH",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          emoji,
          color,
          avatar_url: avatarUrl,
          archetype,
          vibe,
          greeting_style: greetingStyle
        })
      });
      setOk(true);
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Emoji"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: emoji, maxLength: 4, onChange: (e) => setEmoji(e.target.value), placeholder: "\u{1F980}" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Color (hex, optional)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: color, onChange: (e) => setColor(e.target.value), placeholder: "#d4541b" }))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Avatar URL (http/https/data)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: avatarUrl, onChange: (e) => setAvatarUrl(e.target.value), placeholder: "https://\u2026" })), /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Archetype"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: archetype, onChange: (e) => setArchetype(e.target.value), placeholder: "sage / sentinel / artisan" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Vibe"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: vibe, onChange: (e) => setVibe(e.target.value), placeholder: "calm / sharp / playful" }))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Greeting style"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: greetingStyle, onChange: (e) => setGreetingStyle(e.target.value), placeholder: "terse, formal, etc." })), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), ok && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.74 0.135 150 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "SAVED")), /* @__PURE__ */ React.createElement("div", { className: "row", style: { justifyContent: "flex-end", marginTop: 8 } }, /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : "Save identity")));
}
function ChatPage() {
  const [agentsResp] = usePolling("/api/agents?limit=200", 2e4);
  const agents = agentsResp && agentsResp.agents ? agentsResp.agents.map(normalizeAgent) : D.agents;
  const [activeId, setActiveId] = useState(null);
  const active = agents.find((a) => a.id === activeId) || agents[0];
  const [session, sessionErr, refreshSession] = useApi(active ? `/api/agents/${active.id}/session` : null);
  const [budget] = useApi(active ? `/api/budget/agents/${active.id}` : null);
  const [toolsResp] = useApi("/api/tools");
  const tools = Array.isArray(toolsResp) ? toolsResp : toolsResp && toolsResp.tools;
  const [pendingMessages, setPendingMessages] = useState([]);
  const [streamingText, setStreamingText] = useState("");
  const [streamingTools, setStreamingTools] = useState([]);
  const [sending, setSending] = useState(false);
  const [typed, setTyped] = useState("");
  const streamRef = useRef(null);
  useEffect(() => {
    setPendingMessages([]);
    setStreamingText("");
    setStreamingTools([]);
    setSending(false);
  }, [active && active.id]);
  useEffect(() => {
    if (streamRef.current) streamRef.current.scrollTop = streamRef.current.scrollHeight;
  }, [session, pendingMessages, streamingText, streamingTools, sending]);
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
        setStreamingTools((prev) => prev.map((t, i, arr) => i === arr.length - 1 && t.running ? { ...t, running: false, result: msg.result, is_error: !!msg.is_error } : t));
        break;
      case "response":
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
        break;
    }
  }, [streamingText, refreshSession]);
  const ws = useAgentWs(active && active.id, onWs);
  if (!active) {
    return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "40px 14px", fontSize: 13 } }, "No agents loaded. Spawn one from the Agents page first.");
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
    try {
      const resp = await rhFetch(`/api/agents/${active.id}/message`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message: text })
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
  return /* @__PURE__ */ React.createElement("div", { className: "chat-wrap" }, /* @__PURE__ */ React.createElement("div", { className: "chat-list" }, /* @__PURE__ */ React.createElement("div", { className: "chat-list-head row between" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Sessions"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn" }, /* @__PURE__ */ React.createElement(I.plus, null))), /* @__PURE__ */ React.createElement("div", { className: "chat-list-body" }, agents.slice(0, 16).map((a) => /* @__PURE__ */ React.createElement("div", { key: a.id, className: "chat-list-item " + (active.id === a.id ? "active" : ""), onClick: () => setActiveId(a.id) }, /* @__PURE__ */ React.createElement(Avatar, { agent: a }), /* @__PURE__ */ React.createElement("div", { style: { flex: 1, minWidth: 0 } }, /* @__PURE__ */ React.createElement("div", { className: "row between" }, /* @__PURE__ */ React.createElement("span", { className: "name" }, a.name), /* @__PURE__ */ React.createElement("span", { className: "time" }, a.updated)), /* @__PURE__ */ React.createElement("div", { className: "last" }, a.last)))))), /* @__PURE__ */ React.createElement("div", { className: "chat-panel" }, /* @__PURE__ */ React.createElement("div", { className: "chat-head" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2 } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, active.name), /* @__PURE__ */ React.createElement(StateBadge, { state: active.state }), /* @__PURE__ */ React.createElement("span", { className: "badge " + (ws.connected ? "live" : "idle"), title: ws.connected ? "WebSocket streaming" : "WebSocket disconnected (HTTP fallback)" }, ws.connected ? "WS" : "HTTP")), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, active.model, " \xB7 ", active.provider, " \xB7 session ", session && session.session_id ? `#${String(session.session_id).slice(0, 4)}` : "\u2014")), /* @__PURE__ */ React.createElement("div", { className: "actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: refreshSession }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm" }, /* @__PURE__ */ React.createElement(I.download, null), " Export"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn" }, /* @__PURE__ */ React.createElement(I.more, null)))), /* @__PURE__ */ React.createElement("div", { className: "chat-stream", ref: streamRef }, !session && !sessionErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { padding: "24px 6px", fontSize: 12 } }, "Loading session\u2026"), sessionErr && pendingMessages.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { padding: "24px 6px", fontSize: 12, color: "var(--crimson)" } }, "Session load failed: ", sessionErr), items.map((it, i) => {
    if (it.role === "user") return /* @__PURE__ */ React.createElement("div", { key: i, className: "msg user" }, /* @__PURE__ */ React.createElement(Avatar, { agent: { name: "you", hue: 22, emoji: "Y" } }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who", style: { textAlign: "right" } }, "operator \xB7 just now"), /* @__PURE__ */ React.createElement("div", { className: "bubble", style: { whiteSpace: "pre-wrap" } }, it.content)));
    if (it.role === "tool") return /* @__PURE__ */ React.createElement("div", { key: i, className: "tool-trace " + (it.running ? "" : "done") }, /* @__PURE__ */ React.createElement("span", { style: { display: "inline-flex", width: 14, height: 14 } }, it.running ? /* @__PURE__ */ React.createElement(Spinner, null) : /* @__PURE__ */ React.createElement(I.check, null)), /* @__PURE__ */ React.createElement("span", null, it.running ? "\u2699" : "\u2713", " ", /* @__PURE__ */ React.createElement("span", { className: "tool-name" }, it.name), " ", /* @__PURE__ */ React.createElement("span", { className: "dim" }, "(", String(it.input || "").slice(0, 80), ")")), /* @__PURE__ */ React.createElement("span", { className: "elapsed" }, it.is_error ? "err" : "ok"));
    return /* @__PURE__ */ React.createElement("div", { key: i, className: "msg" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who" }, active.name), /* @__PURE__ */ React.createElement("div", { className: "bubble", style: { color: it.error ? "var(--crimson)" : void 0 } }, it.error || it.command ? it.content : renderMarkdown(it.content))));
  }), streamingTools.map((t, i) => /* @__PURE__ */ React.createElement("div", { key: `stool-${i}`, className: "tool-trace " + (t.running ? "" : "done") }, /* @__PURE__ */ React.createElement("span", { style: { display: "inline-flex", width: 14, height: 14 } }, t.running ? /* @__PURE__ */ React.createElement(Spinner, null) : /* @__PURE__ */ React.createElement(I.check, null)), /* @__PURE__ */ React.createElement("span", null, t.running ? "\u2699" : "\u2713", " ", /* @__PURE__ */ React.createElement("span", { className: "tool-name" }, t.name), " ", t.input && /* @__PURE__ */ React.createElement("span", { className: "dim" }, "(", String(typeof t.input === "string" ? t.input : JSON.stringify(t.input)).slice(0, 80), ")")), /* @__PURE__ */ React.createElement("span", { className: "elapsed" }, t.is_error ? "err" : t.running ? "\u2026" : "ok"))), (sending || streamingText) && /* @__PURE__ */ React.createElement("div", { className: "msg" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who" }, active.name, " \xB7 streaming"), /* @__PURE__ */ React.createElement("div", { className: "bubble" }, streamingText ? renderMarkdown(streamingText) : null, /* @__PURE__ */ React.createElement("span", { className: "cursor" }))))), /* @__PURE__ */ React.createElement("div", { className: "chat-input" }, /* @__PURE__ */ React.createElement("div", { className: "field" }, /* @__PURE__ */ React.createElement(I.zap, null), /* @__PURE__ */ React.createElement(
    "input",
    {
      placeholder: `Message ${active.name}\u2026  /workflow  /tool  /memory`,
      value: typed,
      disabled: sending,
      onChange: (e) => setTyped(e.target.value),
      onKeyDown
    }
  ), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u21B5 send")), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: send, disabled: sending || !typed.trim() }, /* @__PURE__ */ React.createElement(I.send, null), " ", sending ? "\u2026" : "Send"))), /* @__PURE__ */ React.createElement("div", { className: "chat-side" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Run context"), /* @__PURE__ */ React.createElement("div", { className: "kv mb-16" }, /* @__PURE__ */ React.createElement("dt", null, "session"), /* @__PURE__ */ React.createElement("dd", null, session ? String(session.session_id || "\u2014").slice(0, 12) : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "messages"), /* @__PURE__ */ React.createElement("dd", null, session && session.messages ? session.messages.length : 0), /* @__PURE__ */ React.createElement("dt", null, "pressure"), /* @__PURE__ */ React.createElement("dd", { style: { color: "var(--live)" } }, session && session.context_pressure || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "budget \xB7 day"), /* @__PURE__ */ React.createElement("dd", null, budget && budget.daily ? `$${Number(budget.daily.spend || 0).toFixed(2)} / $${Number(budget.daily.limit || 0).toFixed(2)}` : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "budget \xB7 hour"), /* @__PURE__ */ React.createElement("dd", null, budget && budget.hourly ? `$${Number(budget.hourly.spend || 0).toFixed(2)} / $${Number(budget.hourly.limit || 0).toFixed(2)}` : "\u2014")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Memory \xB7 session"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 120 } }, `session_id = ${session ? session.session_id || "\u2014" : "loading\u2026"}
agent_id   = ${active.id}
messages   = ${session && session.messages ? session.messages.length : 0}
model      = ${active.model}
provider   = ${active.provider}`), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Allowed tools"), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 200, overflow: "auto" } }, (tools || []).slice(0, 16).map((t) => /* @__PURE__ */ React.createElement("div", { key: t.name, className: "row between", style: { padding: "4px 8px", background: "var(--bg-2)", borderRadius: 5 } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5 } }, t.name), /* @__PURE__ */ React.createElement("span", { className: "badge live", style: { padding: "1px 5px" } }, "ok"))), !tools && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, padding: "4px 8px" } }, "loading\u2026"))));
}
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
const Spinner = () => /* @__PURE__ */ React.createElement("svg", { width: "14", height: "14", viewBox: "0 0 24 24", fill: "none" }, /* @__PURE__ */ React.createElement("circle", { cx: "12", cy: "12", r: "9", stroke: "currentColor", strokeWidth: "2.4", opacity: ".2" }), /* @__PURE__ */ React.createElement("path", { d: "M21 12a9 9 0 0 0-9-9", stroke: "currentColor", strokeWidth: "2.4", strokeLinecap: "round" }, /* @__PURE__ */ React.createElement("animateTransform", { attributeName: "transform", type: "rotate", from: "0 12 12", to: "360 12 12", dur: "0.9s", repeatCount: "indefinite" })));
function WorkflowsPage() {
  const [wfList, , refreshList] = usePolling("/api/workflows", 15e3);
  const workflows = Array.isArray(wfList) ? wfList : wfList && wfList.workflows || [];
  const [activeId, setActiveId] = useState(null);
  const active = workflows.find((w) => w.id === activeId) || workflows[0];
  const [showCreate, setShowCreate] = useState(false);
  const [showRunInput, setShowRunInput] = useState(false);
  const [runsResp, , refreshRuns] = useApi(active ? `/api/workflows/${encodeURIComponent(active.id)}/runs` : null);
  const runs = Array.isArray(runsResp) ? runsResp : runsResp && runsResp.runs || [];
  const runWith = async (input) => {
    if (!active) return;
    try {
      await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}/run`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input })
      });
      setShowRunInput(false);
      refreshRuns();
    } catch (e) {
      toastErr(`run failed: ${e.message || e}`);
    }
  };
  const runNow = () => setShowRunInput(true);
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Workflows ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", workflows.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Pipeline definitions persist across daemon restart \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/workflows"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refreshList }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " New workflow"))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "All workflows"), /* @__PURE__ */ React.createElement("span", { className: "mono" }, workflows.length)), /* @__PURE__ */ React.createElement("div", null, !wfList && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "12px 14px", fontSize: 12 } }, "loading\u2026"), wfList && workflows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "12px 14px", fontSize: 12 } }, "No workflows yet."), workflows.map((w) => {
    const stepCount = Array.isArray(w.steps) ? w.steps.length : w.steps || 0;
    return /* @__PURE__ */ React.createElement(
      "div",
      {
        key: w.id,
        onClick: () => setActiveId(w.id),
        className: "row between",
        style: {
          padding: "10px 14px",
          borderBottom: "1px solid var(--border)",
          cursor: "pointer",
          background: active && active.id === w.id ? "var(--surface-2)" : "transparent"
        }
      },
      /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 12.5 } }, w.name || w.id), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 10.5 } }, stepCount, " steps", w.kind ? ` \xB7 ${w.kind}` : "")),
      /* @__PURE__ */ React.createElement("div", { className: "col", style: { alignItems: "flex-end", gap: 2 } }, /* @__PURE__ */ React.createElement("span", { className: "mono nums", style: { fontSize: 12 } }, w.runs_24h != null ? w.runs_24h : "\u2014"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, "runs \xB7 24h"))
    );
  })))), /* @__PURE__ */ React.createElement("div", { className: "col-8 col" }, active && /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-title", style: { fontSize: 15, marginBottom: 2 } }, active.name || active.id), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "id=", active.id, active.description ? ` \xB7 ${active.description}` : "")), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: refreshRuns }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: runNow }, /* @__PURE__ */ React.createElement(I.play, null), " Run now"))), /* @__PURE__ */ React.createElement(WorkflowDAG, { workflow: active }), /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mt-16" }, /* @__PURE__ */ React.createElement(Stat, { label: "Steps", value: Array.isArray(active.steps) ? active.steps.length : active.steps || 0 }), /* @__PURE__ */ React.createElement(Stat, { label: "Runs (recent)", value: runs.length }), /* @__PURE__ */ React.createElement(Stat, { label: "p50", value: active.p50_ms ? `${active.p50_ms}ms` : "\u2014" }), /* @__PURE__ */ React.createElement(Stat, { label: "OK rate", value: active.ok != null ? `${active.ok}%` : "\u2014" }), /* @__PURE__ */ React.createElement(Stat, { label: "Updated", value: relativeTime(active.updated_at) }))), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Recent runs")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Run"), /* @__PURE__ */ React.createElement("th", null, "Trigger"), /* @__PURE__ */ React.createElement("th", null, "Started"), /* @__PURE__ */ React.createElement("th", null, "Duration"), /* @__PURE__ */ React.createElement("th", null, "Status"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Tokens"))), /* @__PURE__ */ React.createElement("tbody", null, runs.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, !active ? "Select a workflow." : "No runs yet.")), runs.map((r) => {
    const id = r.id || r.run_id || "\u2014";
    const trig = r.trigger || r.source || "manual";
    const t = formatTime(r.started_at || r.created_at);
    const dur = r.duration_ms ? `${(r.duration_ms / 1e3).toFixed(2)}s` : "\u2014";
    const st = (r.status || r.outcome || "ok").toLowerCase();
    const tok = r.total_tokens || r.tokens || "\u2014";
    return /* @__PURE__ */ React.createElement("tr", { key: id }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, id), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, trig)), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, t), /* @__PURE__ */ React.createElement("td", { className: "mono" }, dur), /* @__PURE__ */ React.createElement("td", null, st === "ok" || st === "success" ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "ok") : /* @__PURE__ */ React.createElement("span", { className: "badge warn" }, st)), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, tok));
  })))))), showCreate && /* @__PURE__ */ React.createElement(WorkflowCreateModal, { onClose: () => setShowCreate(false), onCreated: (id) => {
    setShowCreate(false);
    setActiveId(id);
    refreshList();
  } }), showRunInput && active && /* @__PURE__ */ React.createElement(WorkflowRunModal, { workflow: active, onClose: () => setShowRunInput(false), onRun: runWith }));
}
function WorkflowCreateModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const agents = agentsResp && agentsResp.agents || [];
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [steps, setSteps] = useState([
    {
      name: "step-1",
      agent_id: "",
      prompt: "{{input}}",
      mode: "sequential",
      timeout_secs: 120,
      error_mode: "fail",
      output_var: ""
    }
  ]);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  React.useEffect(() => {
    if (agents.length && steps.length === 1 && !steps[0].agent_id) {
      setSteps([{ ...steps[0], agent_id: agents[0].id }]);
    }
  }, [agents.length]);
  const updateStep = (i, patch) => setSteps((prev) => prev.map((s, j) => j === i ? { ...s, ...patch } : s));
  const removeStep = (i) => setSteps((prev) => prev.filter((_, j) => j !== i));
  const moveStep = (i, delta) => {
    setSteps((prev) => {
      const j = i + delta;
      if (j < 0 || j >= prev.length) return prev;
      const out = prev.slice();
      [out[i], out[j]] = [out[j], out[i]];
      return out;
    });
  };
  const duplicateStep = (i) => setSteps((prev) => {
    const copy = { ...prev[i], name: `${prev[i].name}-copy` };
    return prev.slice(0, i + 1).concat([copy], prev.slice(i + 1));
  });
  const reorderStep = (from, to) => setSteps((prev) => {
    if (from === to || from < 0 || to < 0 || from >= prev.length || to >= prev.length) return prev;
    const out = prev.slice();
    const [moved] = out.splice(from, 1);
    out.splice(to, 0, moved);
    return out;
  });
  const [dragSrc, setDragSrc] = useState(null);
  const [dragOver, setDragOver] = useState(null);
  const addStep = () => setSteps((prev) => prev.concat([{
    name: `step-${prev.length + 1}`,
    agent_id: agents[0] ? agents[0].id : "",
    prompt: "{{input}}",
    mode: "sequential",
    timeout_secs: 120,
    error_mode: "fail",
    output_var: ""
  }]));
  const submit = async () => {
    if (!name.trim()) {
      setErr("name required");
      return;
    }
    if (steps.length === 0) {
      setErr("at least one step required");
      return;
    }
    for (const s of steps) {
      if (!s.agent_id && !s.agent_name) {
        setErr(`step '${s.name}' is missing agent_id / agent_name`);
        return;
      }
    }
    const payload = steps.map((s) => {
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
    setBusy(true);
    setErr(null);
    try {
      const r = await rhFetch("/api/workflows", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, description, steps: payload })
      });
      onCreated(r.id || r.workflow_id);
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "New workflow"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value), placeholder: "my-pipeline" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 2 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Description"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: description, onChange: (e) => setDescription(e.target.value) }))), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", marginTop: 6 } }, "Steps \xB7 ", steps.length), /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, steps.map((s, i) => /* @__PURE__ */ React.createElement(
    WorkflowStepCard,
    {
      key: i,
      index: i,
      total: steps.length,
      step: s,
      agents,
      isDragSrc: dragSrc === i,
      isDragOver: dragOver === i && dragSrc !== null && dragSrc !== i,
      onDragStart: () => setDragSrc(i),
      onDragOver: () => setDragOver(i),
      onDrop: () => {
        if (dragSrc !== null) reorderStep(dragSrc, i);
        setDragSrc(null);
        setDragOver(null);
      },
      onDragEnd: () => {
        setDragSrc(null);
        setDragOver(null);
      },
      onChange: (patch) => updateStep(i, patch),
      onRemove: () => removeStep(i),
      onMove: (delta) => moveStep(i, delta),
      onDuplicate: () => duplicateStep(i)
    }
  )), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: addStep, style: { alignSelf: "flex-start" } }, /* @__PURE__ */ React.createElement(I.plus, null), " Add step"))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Creating\u2026" : "Create"))));
}
function WorkflowStepCard({
  index,
  total,
  step,
  agents,
  isDragSrc,
  isDragOver,
  onDragStart,
  onDragOver,
  onDrop,
  onDragEnd,
  onChange,
  onRemove,
  onMove,
  onDuplicate
}) {
  const [advanced, setAdvanced] = useState(false);
  const dragHandleArmed = React.useRef(false);
  return /* @__PURE__ */ React.createElement(
    "div",
    {
      draggable: true,
      onDragStart: (e) => {
        if (!dragHandleArmed.current) {
          e.preventDefault();
          return;
        }
        e.dataTransfer.effectAllowed = "move";
        try {
          e.dataTransfer.setData("text/plain", String(index));
        } catch (_) {
        }
        onDragStart && onDragStart();
      },
      onDragOver: (e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        onDragOver && onDragOver();
      },
      onDrop: (e) => {
        e.preventDefault();
        dragHandleArmed.current = false;
        onDrop && onDrop();
      },
      onDragEnd: () => {
        dragHandleArmed.current = false;
        onDragEnd && onDragEnd();
      },
      style: {
        border: "1px solid " + (isDragOver ? "var(--rust)" : "var(--border)"),
        boxShadow: isDragOver ? "0 0 0 1px var(--rust) inset" : void 0,
        borderRadius: 8,
        background: "var(--bg-2)",
        padding: "10px 12px",
        opacity: isDragSrc ? 0.55 : 1,
        transition: "opacity .12s, border-color .12s, box-shadow .12s"
      }
    },
    /* @__PURE__ */ React.createElement("div", { className: "row between mb-8" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement(
      "span",
      {
        className: "dnd-handle",
        title: "Drag to reorder",
        onMouseDown: () => {
          dragHandleArmed.current = true;
        },
        onMouseUp: () => {
          dragHandleArmed.current = false;
        }
      },
      "\u2630"
    ), /* @__PURE__ */ React.createElement("span", { className: "badge plain", style: { minWidth: 32, textAlign: "center" } }, "#", index + 1), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12.5 } }, step.name || "(unnamed)"), step.mode !== "sequential" && /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, step.mode)), /* @__PURE__ */ React.createElement("div", { className: "row gap-4" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => onMove(-1), disabled: index === 0, title: "Move up" }, "\u2191"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => onMove(1), disabled: index === total - 1, title: "Move down" }, "\u2193"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: onDuplicate, title: "Duplicate" }, /* @__PURE__ */ React.createElement(I.copy, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: onRemove, title: "Remove", style: { color: "var(--crimson)" }, disabled: total === 1 }, /* @__PURE__ */ React.createElement(I.close, null)))),
    /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mb-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Step name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: step.name, onChange: (e) => onChange({ name: e.target.value }) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Agent"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: step.agent_id || "", onChange: (e) => onChange({ agent_id: e.target.value, agent_name: "" }) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick agent \u2014"), agents.map((a) => /* @__PURE__ */ React.createElement("option", { key: a.id, value: a.id }, a.name, " (", String(a.id).slice(0, 8), ")")))), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { width: 140 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Mode"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: step.mode, onChange: (e) => onChange({ mode: e.target.value }) }, ["sequential", "fan_out", "collect", "conditional", "loop"].map((m) => /* @__PURE__ */ React.createElement("option", { key: m, value: m }, m))))),
    /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Prompt template (use ", "{{input}}", " for previous step output)"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 60 }, value: step.prompt, onChange: (e) => onChange({ prompt: e.target.value }) })),
    step.mode === "conditional" && /* @__PURE__ */ React.createElement("label", { className: "t-row col mt-8" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Condition (run only if previous output contains)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: step.condition || "", onChange: (e) => onChange({ condition: e.target.value }) })),
    step.mode === "loop" && /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mt-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Max iterations"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: "1", value: step.max_iterations || 5, onChange: (e) => onChange({ max_iterations: Number(e.target.value) }) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 2 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Until (output contains)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: step.until || "", onChange: (e) => onChange({ until: e.target.value }) }))),
    /* @__PURE__ */ React.createElement("div", { className: "row between mt-8" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setAdvanced((a) => !a) }, advanced ? "\u2212 Hide advanced" : "+ Advanced")),
    advanced && /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mt-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Timeout (s)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: "10", value: step.timeout_secs || 120, onChange: (e) => onChange({ timeout_secs: Number(e.target.value) }) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Error mode"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: step.error_mode || "fail", onChange: (e) => onChange({ error_mode: e.target.value }) }, ["fail", "skip", "retry"].map((m) => /* @__PURE__ */ React.createElement("option", { key: m, value: m }, m)))), step.error_mode === "retry" && /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Max retries"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: "1", max: "10", value: step.max_retries || 3, onChange: (e) => onChange({ max_retries: Number(e.target.value) }) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Output var (optional)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: step.output_var || "", onChange: (e) => onChange({ output_var: e.target.value }), placeholder: "e.g. ranked_list" })))
  );
}
function WorkflowRunModal({ workflow, onClose, onRun }) {
  useEscapeKey(onClose);
  const [inputJson, setInputJson] = useState("{}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const run = async () => {
    let input;
    try {
      input = JSON.parse(inputJson);
    } catch (e) {
      setErr(`input must be valid JSON: ${e.message}`);
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await onRun(input);
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Run workflow \xB7 ", workflow.name || workflow.id), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Input (JSON)"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea mt-8", style: { fontFamily: "var(--ff-mono)" }, value: inputJson, onChange: (e) => setInputJson(e.target.value) }), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: run, disabled: busy }, busy ? "Running\u2026" : "Run"))));
}
const Stat = ({ label, value }) => /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2, paddingRight: 18, borderRight: "1px solid var(--border)" } }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10, letterSpacing: ".12em", textTransform: "uppercase" } }, label), /* @__PURE__ */ React.createElement("span", { className: "mono nums", style: { fontSize: 16 } }, value));
const WorkflowDAG = ({ workflow }) => {
  let nodes;
  if (workflow && Array.isArray(workflow.steps) && workflow.steps.length > 0) {
    const N = workflow.steps.length;
    nodes = workflow.steps.slice(0, 6).map((s, i) => ({
      x: 20 + i * 145,
      y: 130,
      kind: s.kind || s.type || (s.agent_id ? "agent" : s.tool ? "tool" : "step"),
      name: s.name || s.id || `step ${i + 1}`,
      tag: s.agent_id || s.tool || s.workflow_id || s.event || "\u2014",
      cls: i === 0 ? "start" : i === N - 1 ? "end" : ""
    }));
  } else {
    nodes = [
      { x: 20, y: 130, kind: "trigger", name: "\u2014", tag: "select a workflow", cls: "start" }
    ];
  }
  const edges = nodes.slice(1).map((_, i) => [i, i + 1]);
  const centers = nodes.map((n) => ({ x: n.x + 75, y: n.y + 22 }));
  return /* @__PURE__ */ React.createElement("div", { className: "dag" }, /* @__PURE__ */ React.createElement("svg", { width: "100%", height: "100%", style: { position: "absolute", inset: 0 } }, /* @__PURE__ */ React.createElement("defs", null, /* @__PURE__ */ React.createElement("marker", { id: "arr", viewBox: "0 0 10 10", refX: "8", refY: "5", markerWidth: "6", markerHeight: "6", orient: "auto" }, /* @__PURE__ */ React.createElement("path", { d: "M0,0 L10,5 L0,10 z", fill: "var(--rust)" }))), edges.map(([a, b], i) => {
    const A = centers[a], B = centers[b];
    const mid = (A.x + B.x) / 2;
    return /* @__PURE__ */ React.createElement(
      "path",
      {
        key: i,
        d: `M${A.x + 72},${A.y} C${mid},${A.y} ${mid},${B.y} ${B.x - 78},${B.y}`,
        fill: "none",
        stroke: "var(--rust)",
        strokeWidth: "1.4",
        opacity: ".75",
        strokeDasharray: i % 2 ? "4 3" : "0",
        markerEnd: "url(#arr)"
      }
    );
  })), nodes.map((n, i) => /* @__PURE__ */ React.createElement("div", { key: i, className: "node " + (n.cls || ""), style: { left: n.x, top: n.y, width: 150 } }, /* @__PURE__ */ React.createElement("div", { className: "node-kind" }, n.kind), /* @__PURE__ */ React.createElement("div", { className: "node-name" }, n.name), /* @__PURE__ */ React.createElement("div", { className: "node-tag" }, n.tag))));
};
function AutomationPage() {
  const [tab, setTab] = useState("cron");
  const [showCreate, setShowCreate] = useState(false);
  const [cronResp, , refreshCron] = usePolling("/api/cron/jobs", 15e3);
  const [trigResp, , refreshTrig] = usePolling("/api/triggers", 15e3);
  const cron = cronResp && cronResp.jobs || [];
  const triggers = Array.isArray(trigResp) ? trigResp : trigResp && trigResp.triggers || [];
  const toggleCron = async (id, enabled) => {
    try {
      await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/enable`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ enabled: !enabled })
      });
      refreshCron();
    } catch (e) {
      console.warn("toggle failed", e);
    }
  };
  const runCronNow = async (id) => {
    try {
      await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/run`, { method: "POST" });
      refreshCron();
    } catch (e) {
      toastErr(`run failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Automation"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Cron jobs survive restart \xB7 3 CronAction variants \xB7 trigger fire-counts persisted")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => (tab === "cron" ? refreshCron : refreshTrig)() }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " New ", tab === "cron" ? "job" : "trigger"))), /* @__PURE__ */ React.createElement("div", { className: "tabs" }, /* @__PURE__ */ React.createElement("button", { className: tab === "cron" ? "on" : "", onClick: () => setTab("cron") }, "Cron jobs \xB7 ", cron.length), /* @__PURE__ */ React.createElement("button", { className: tab === "triggers" ? "on" : "", onClick: () => setTab("triggers") }, "Triggers \xB7 ", triggers.length)), tab === "cron" && /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Schedule"), /* @__PURE__ */ React.createElement("th", null, "Action"), /* @__PURE__ */ React.createElement("th", null, "Next"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Fires"), /* @__PURE__ */ React.createElement("th", null, "Enabled"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !cronResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), cronResp && cron.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No cron jobs yet.")), cron.map((c) => {
    const actionLabel = c.action_label || c.action_summary || c.action && (c.action.kind || c.action.type || JSON.stringify(c.action).slice(0, 60)) || "\u2014";
    const next = c.next_run || c.next_fire || c.next || "\u2014";
    const fires = c.fire_count != null ? c.fire_count : c.fires || 0;
    return /* @__PURE__ */ React.createElement("tr", { key: c.id }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, c.id), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { color: "var(--rust)" } }, c.schedule || c.cron || c.expression)), /* @__PURE__ */ React.createElement("td", { className: "mono", style: { maxWidth: 300, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, actionLabel), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, next === "\u2014" ? next : formatTime(next)), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, Number(fires).toLocaleString()), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "switch " + (c.enabled ? "on" : ""), onClick: () => toggleCron(c.id, c.enabled) })), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => runCronNow(c.id) }, "Run now")));
  })))), tab === "triggers" && /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Kind"), /* @__PURE__ */ React.createElement("th", null, "Target"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Fired"), /* @__PURE__ */ React.createElement("th", null, "Last"), /* @__PURE__ */ React.createElement("th", null, "Status"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !trigResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), trigResp && triggers.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No triggers configured.")), triggers.map((t) => {
    const kind = (t.kind || t.type || "\u2014").toString();
    const target = t.target || t.agent_id || t.workflow_id || "\u2014";
    const fired = t.fire_count != null ? t.fire_count : t.fired || 0;
    const last = t.last_fired || t.last || null;
    const status = (t.status || (t.enabled ? "active" : "armed")).toString();
    return /* @__PURE__ */ React.createElement("tr", { key: t.id }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, t.id), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "row gap-6", style: { color: "var(--fg-2)" } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)", display: "inline-flex" } }, /* @__PURE__ */ React.createElement(ChannelIcon, { kind })), /* @__PURE__ */ React.createElement("span", { className: "mono" }, kind))), /* @__PURE__ */ React.createElement("td", { className: "mono" }, target), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, Number(fired).toLocaleString()), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, last ? formatTime(last) : "\u2014"), /* @__PURE__ */ React.createElement("td", null, status === "active" ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "active") : /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, status)), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost" }, /* @__PURE__ */ React.createElement(I.more, null))));
  })))), showCreate && tab === "cron" && /* @__PURE__ */ React.createElement(CronJobModal, { onClose: () => setShowCreate(false), onCreated: () => {
    setShowCreate(false);
    refreshCron();
  } }), showCreate && tab === "triggers" && /* @__PURE__ */ React.createElement(TriggerModal, { onClose: () => setShowCreate(false), onCreated: () => {
    setShowCreate(false);
    refreshTrig();
  } }));
}
function CronJobModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const [workflowsResp] = useApi("/api/workflows");
  const agents = agentsResp && agentsResp.agents || [];
  const workflows = Array.isArray(workflowsResp) ? workflowsResp : workflowsResp && workflowsResp.workflows || [];
  const [name, setName] = useState("");
  const [scheduleKind, setScheduleKind] = useState("cron");
  const [cronExpr, setCronExpr] = useState("0 9 * * *");
  const [everySecs, setEverySecs] = useState(3600);
  const [atIso, setAtIso] = useState(new Date(Date.now() + 36e5).toISOString().slice(0, 16));
  const [actionKind, setActionKind] = useState("system_event");
  const [agentId, setAgentId] = useState("");
  const [message, setMessage] = useState("Scheduled check-in");
  const [systemText, setSystemText] = useState("scheduled event");
  const [workflowId, setWorkflowId] = useState("");
  const [workflowInput, setWorkflowInput] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  React.useEffect(() => {
    if (!agentId && agents.length) setAgentId(agents[0].id);
  }, [agents.length]);
  React.useEffect(() => {
    if (!workflowId && workflows.length) setWorkflowId(workflows[0].id);
  }, [workflows.length]);
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
    if (!name.trim()) {
      setErr("name is required");
      return;
    }
    if (!agentId) {
      setErr("pick an owning agent");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await rhFetch("/api/cron/jobs", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          name,
          agent_id: agentId,
          schedule: buildSchedule(),
          action: buildAction(),
          enabled: true
        })
      });
      onCreated();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "New cron job"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value), placeholder: "daily-digest" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Owning agent"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: agentId, onChange: (e) => setAgentId(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick agent \u2014"), agents.map((a) => /* @__PURE__ */ React.createElement("option", { key: a.id, value: a.id }, a.name, " (", String(a.id).slice(0, 8), ")")))), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", marginTop: 6 } }, "Schedule"), /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "cron" ? "on" : "", onClick: () => setScheduleKind("cron") }, "Cron"), /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "every" ? "on" : "", onClick: () => setScheduleKind("every") }, "Every"), /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "at" ? "on" : "", onClick: () => setScheduleKind("at") }, "At")), scheduleKind === "cron" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "0 9 * * *", value: cronExpr, onChange: (e) => setCronExpr(e.target.value) }), scheduleKind === "every" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: 60, value: everySecs, onChange: (e) => setEverySecs(e.target.value), placeholder: "seconds (60..86400)" }), scheduleKind === "at" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "datetime-local", value: atIso, onChange: (e) => setAtIso(e.target.value) }), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", marginTop: 6 } }, "Action"), /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: actionKind === "system_event" ? "on" : "", onClick: () => setActionKind("system_event") }, "System event"), /* @__PURE__ */ React.createElement("button", { className: actionKind === "agent_turn" ? "on" : "", onClick: () => setActionKind("agent_turn") }, "Agent turn"), /* @__PURE__ */ React.createElement("button", { className: actionKind === "workflow_run" ? "on" : "", onClick: () => setActionKind("workflow_run") }, "Workflow run")), actionKind === "system_event" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: systemText, onChange: (e) => setSystemText(e.target.value), placeholder: "event text" }), actionKind === "agent_turn" && /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", value: message, onChange: (e) => setMessage(e.target.value), placeholder: "message to send" }), actionKind === "workflow_run" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("select", { className: "t-select", value: workflowId, onChange: (e) => setWorkflowId(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick workflow \u2014"), workflows.map((w) => /* @__PURE__ */ React.createElement("option", { key: w.id, value: w.id }, w.name || w.id))), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", value: workflowInput, onChange: (e) => setWorkflowInput(e.target.value), placeholder: "workflow input (text)" }))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Creating\u2026" : "Create"))));
}
function TriggerModal({ onClose, onCreated }) {
  useEscapeKey(onClose);
  const [agentsResp] = useApi("/api/agents?limit=200");
  const agents = agentsResp && agentsResp.agents || [];
  const [agentId, setAgentId] = useState("");
  const [patternKind, setPatternKind] = useState("system");
  const [namePattern, setNamePattern] = useState("");
  const [keyword, setKeyword] = useState("");
  const [substring, setSubstring] = useState("");
  const [keyPattern, setKeyPattern] = useState("");
  const [promptTemplate, setPromptTemplate] = useState("Event fired: {{event}}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  React.useEffect(() => {
    if (!agentId && agents.length) setAgentId(agents[0].id);
  }, [agents.length]);
  const buildPattern = () => {
    switch (patternKind) {
      case "lifecycle":
        return "lifecycle";
      case "agent_spawned":
        return { agent_spawned: { name_pattern: namePattern || ".*" } };
      case "agent_terminated":
        return "agent_terminated";
      case "system":
        return "system";
      case "system_keyword":
        return { system_keyword: { keyword } };
      case "memory_update":
        return "memory_update";
      case "memory_key_pattern":
        return { memory_key_pattern: { key_pattern: keyPattern } };
      case "all":
        return "all";
      case "content_match":
        return { content_match: { substring } };
      default:
        return "all";
    }
  };
  const submit = async () => {
    if (!agentId) {
      setErr("pick an agent");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await rhFetch("/api/triggers", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          agent_id: agentId,
          pattern: buildPattern(),
          prompt_template: promptTemplate
        })
      });
      onCreated();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "New trigger"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Owning agent"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: agentId, onChange: (e) => setAgentId(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick agent \u2014"), agents.map((a) => /* @__PURE__ */ React.createElement("option", { key: a.id, value: a.id }, a.name)))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Pattern"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: patternKind, onChange: (e) => setPatternKind(e.target.value) }, ["all", "lifecycle", "agent_spawned", "agent_terminated", "system", "system_keyword", "memory_update", "memory_key_pattern", "content_match"].map((p) => /* @__PURE__ */ React.createElement("option", { key: p, value: p }, p)))), patternKind === "agent_spawned" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "name pattern (regex)", value: namePattern, onChange: (e) => setNamePattern(e.target.value) }), patternKind === "system_keyword" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "keyword", value: keyword, onChange: (e) => setKeyword(e.target.value) }), patternKind === "memory_key_pattern" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "key pattern", value: keyPattern, onChange: (e) => setKeyPattern(e.target.value) }), patternKind === "content_match" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "substring", value: substring, onChange: (e) => setSubstring(e.target.value) }), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Prompt template (use ", "{{event}}", ")"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", value: promptTemplate, onChange: (e) => setPromptTemplate(e.target.value) }))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Creating\u2026" : "Create"))));
}
function ChannelsPage() {
  const [chResp, , refresh] = usePolling("/api/channels", 2e4);
  const channels = chResp && chResp.channels || [];
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
    try {
      await rhFetch("/api/channels/reload", { method: "POST" });
      refresh();
    } catch (e) {
      toastErr(`reload failed: ${e.message || e}`);
    }
  };
  const removeChannel = async (name) => {
    if (!confirm(`Disconnect ${name}?`)) return;
    try {
      await rhFetch(`/api/channels/${encodeURIComponent(name)}/configure`, { method: "DELETE" });
      refresh();
    } catch (e) {
      toastErr(`disconnect failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Channels ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", channels.filter((c) => c.configured).length, " of ", channels.length, " configured")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Telegram-first \xB7 streaming via editMessageText \xB7 500ms throttle \xB7 Discord + Slack adapters available")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: reload }, "Reload"))), !chResp && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px", fontSize: 12 } }, "loading\u2026"), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, channels.map((ch) => {
    const kind = ch.icon || ch.name;
    const state = ch.configured ? ch.auth_status === "ok" ? "live" : ch.auth_status === "auth_failed" ? "auth_failed" : "idle" : "idle";
    const stateLabel = ch.configured ? ch.auth_status === "ok" ? "live" : ch.auth_status === "auth_failed" ? "auth failed" : "configured" : "not configured";
    return /* @__PURE__ */ React.createElement("div", { key: ch.name, className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("div", { className: "avatar lg", style: { background: "linear-gradient(135deg,var(--rust),oklch(0.42 0.10 50))" } }, /* @__PURE__ */ React.createElement(ChannelIcon, { kind })), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, fontWeight: 500 } }, ch.display_name || ch.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, ch.name, ch.difficulty ? ` \xB7 ${ch.difficulty}` : "", ch.setup_time ? ` \xB7 ${ch.setup_time}` : ""))), /* @__PURE__ */ React.createElement("span", { className: "badge " + (state === "live" ? "live" : state === "auth_failed" ? "error" : "idle") }, stateLabel)), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5, marginBottom: 12 } }, ch.description), testResult && testResult.name === ch.name && /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: { borderColor: testResult.ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot " + (testResult.busy ? "warn" : testResult.ok ? "live" : "err") }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, testResult.busy ? "TESTING" : testResult.ok ? "OK" : "FAIL"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, testResult.message || testResult.detail || (testResult.busy ? "\u2026" : ""))), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, ch.configured ? /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => testChannel(ch.name) }, "Test"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setConfiguring(ch) }, "Reconfigure"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => removeChannel(ch.name), style: { color: "var(--crimson)" } }, "Disconnect")) : /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => setConfiguring(ch) }, /* @__PURE__ */ React.createElement(I.plus, null), " Connect")));
  })), configuring && /* @__PURE__ */ React.createElement(ChannelConfigModal, { channel: configuring, onClose: () => setConfiguring(null), onSaved: () => {
    setConfiguring(null);
    refresh();
  } }));
}
function ChannelConfigModal({ channel, onClose, onSaved }) {
  useEscapeKey(onClose);
  const [values, setValues] = useState({});
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const fields = channel.fields || [];
  const save = async () => {
    setBusy(true);
    setErr(null);
    try {
      await rhFetch(`/api/channels/${encodeURIComponent(channel.name)}/configure`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ fields: values })
      });
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Configure \xB7 ", channel.display_name || channel.name), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, channel.description && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 12 } }, channel.description), /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, fields.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "No fields defined for this channel."), fields.map((f) => /* @__PURE__ */ React.createElement("label", { key: f.name, className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, f.label || f.name, f.required && /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, " *"), f.env_var && /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { marginLeft: 6, fontSize: 10 } }, "(", f.env_var, ")")), /* @__PURE__ */ React.createElement(
    "input",
    {
      className: "modal-field",
      type: f.secret ? "password" : "text",
      placeholder: f.placeholder || "",
      value: values[f.name] || "",
      onChange: (e) => setValues({ ...values, [f.name]: e.target.value })
    }
  ), f.description && /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 10.5, marginTop: 2 } }, f.description)))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : "Save & connect"))));
}
function AnalyticsPage() {
  const [daily] = usePolling("/api/usage/daily", 3e4);
  const [byModel] = usePolling("/api/usage/by-model", 3e4);
  const [stats] = usePolling("/api/usage", 3e4);
  const [agentsResp] = usePolling("/api/agents?limit=200", 3e4);
  const agents = agentsResp && agentsResp.agents ? agentsResp.agents.map(normalizeAgent) : [];
  const days = daily && daily.days || [];
  const dailyCosts = days.map((d) => Number(d.cost_usd || 0));
  const totalSpend7d = dailyCosts.slice(-7).reduce((s, v) => s + v, 0);
  const totalRequests7d = days.slice(-7).reduce((s, d) => s + (d.requests || 0), 0);
  const avgPerDay = days.length ? totalRequests7d / Math.min(7, days.length) : 0;
  const modelRows = Array.isArray(byModel) ? byModel : byModel && byModel.models || [];
  const maxModelSpend = Math.max(1e-4, ...modelRows.map((m) => Number(m.spend || m.cost_usd || 0)));
  const hourSeries = dailyCosts.slice(-24);
  const seriesForChart = hourSeries.length ? hourSeries : Array(24).fill(0);
  const totalForChart = seriesForChart.reduce((s, v) => s + v, 0);
  const cacheHitRate = stats && stats.cache_hit_rate != null ? `${Math.round(Number(stats.cache_hit_rate) * 100)}%` : "\u2014";
  const p95 = stats && stats.p95_latency_ms != null ? `${(Number(stats.p95_latency_ms) / 1e3).toFixed(2)}s` : "\u2014";
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Analytics"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Token spend, latency, cache hits \xB7 sourced from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/usage"), " \xB7 ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/usage/daily"), " \xB7 ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/usage/by-model"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: "on" }, "7d")), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    if (!daily) return;
    const csv = rowsToCsv(days, [
      { key: "date", label: "date" },
      { key: "cost_usd", label: "cost_usd", format: (v) => Number(v || 0).toFixed(6) },
      { key: "requests", label: "requests" }
    ]);
    downloadBlob(`rustyhand-usage-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.csv`, csv, "text/csv");
  } }, /* @__PURE__ */ React.createElement(I.download, null), " CSV"))), /* @__PURE__ */ React.createElement("div", { className: "tiles" }, /* @__PURE__ */ React.createElement(Tile, { label: "Total spend \xB7 7d", value: `$${totalSpend7d.toFixed(2)}`, foot: daily ? `${days.length} day(s) of data` : "loading\u2026", spark: dailyCosts.slice(-12) }), /* @__PURE__ */ React.createElement(Tile, { label: "LLM requests \xB7 7d", value: totalRequests7d.toLocaleString(), foot: daily ? `${avgPerDay.toFixed(0)} / day avg` : "loading\u2026", spark: days.slice(-12).map((d) => d.requests || 0) }), /* @__PURE__ */ React.createElement(Tile, { label: "Cache hit-rate", value: cacheHitRate, foot: stats ? "LLM cache \xB7 24h TTL" : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }), /* @__PURE__ */ React.createElement(Tile, { label: "p95 latency", value: p95, foot: stats ? "kernel telemetry" : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Cost \xB7 daily (last 24 buckets)"), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11 } }, "$", totalForChart.toFixed(2))), /* @__PURE__ */ React.createElement(CostChart, { data: seriesForChart })), /* @__PURE__ */ React.createElement("div", { className: "col-4 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Spend by model"), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11 } }, "$", modelRows.reduce((s, m) => s + Number(m.spend || m.cost_usd || 0), 0).toFixed(2))), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 4 } }, !byModel && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11, padding: "6px 0" } }, "loading\u2026"), byModel && modelRows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11, padding: "6px 0" } }, "no model usage data yet."), modelRows.slice(0, 8).map((m) => /* @__PURE__ */ React.createElement(BarRow, { key: m.model || m.name, label: m.model || m.name, value: Number(m.spend || m.cost_usd || 0), max: maxModelSpend, unit: "$" })))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Agents \xB7 by activity")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Model"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Updated"))), /* @__PURE__ */ React.createElement("tbody", null, agents.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 4, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), agents.slice(0, 8).map((a) => /* @__PURE__ */ React.createElement("tr", { key: a.id }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "agent-row" }, /* @__PURE__ */ React.createElement(Avatar, { agent: a }), /* @__PURE__ */ React.createElement("span", { className: "name" }, a.name))), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, a.model), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(StateBadge, { state: a.state })), /* @__PURE__ */ React.createElement("td", { className: "num mono muted" }, a.updated)))))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Provider state"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, "from /api/providers")), /* @__PURE__ */ React.createElement(ProviderState, null))));
}
function ProviderState() {
  const [resp] = usePolling("/api/providers", 3e4);
  const providers = resp && resp.providers || [];
  if (!resp) return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11 } }, "loading\u2026");
  if (providers.length === 0) return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11 } }, "no providers configured");
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, providers.slice(0, 10).map((p) => {
    const auth = (p.auth_status || "").toLowerCase();
    let label = auth || "\u2014";
    let err = false, warn = false;
    if (auth === "ok") label = "Connected";
    else if (auth === "missing") {
      label = "No key";
      warn = p.key_required !== false;
    } else if (auth === "invalid") {
      label = "Invalid";
      err = true;
    } else if (auth === "rate_limited") {
      label = "Rate-limited";
      warn = true;
    }
    const tail = p.model_count != null ? `${p.model_count} model${p.model_count === 1 ? "" : "s"}` : "";
    return /* @__PURE__ */ React.createElement(BreakerRow, { key: p.id || p.name, name: p.display_name || p.id || p.name, state: label, tail, err, warn });
  }));
}
const BreakerRow = ({ name, state, tail, warn, err }) => /* @__PURE__ */ React.createElement("div", { className: "row between", style: { padding: "7px 10px", background: "var(--bg-2)", borderRadius: 6, border: "1px solid var(--border)" } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, name), /* @__PURE__ */ React.createElement("span", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "badge " + (err ? "error" : warn ? "warn" : "live") }, state), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, tail)));
const CostChart = ({ data }) => {
  const W = 800, H = 220, P = 28;
  const max = Math.max(...data), min = 0;
  const x = (i) => P + i / (data.length - 1) * (W - P * 2);
  const y = (v) => H - P - (v - min) / (max - min) * (H - P * 2);
  const path = data.map((v, i) => i === 0 ? `M${x(i)},${y(v)}` : `L${x(i)},${y(v)}`).join(" ");
  const area = `${path} L${x(data.length - 1)},${H - P} L${x(0)},${H - P} Z`;
  return /* @__PURE__ */ React.createElement("svg", { viewBox: `0 0 ${W} ${H}`, width: "100%", height: "220", style: { display: "block" } }, /* @__PURE__ */ React.createElement("defs", null, /* @__PURE__ */ React.createElement("linearGradient", { id: "g1", x1: "0", x2: "0", y1: "0", y2: "1" }, /* @__PURE__ */ React.createElement("stop", { offset: "0%", stopColor: "var(--rust)", stopOpacity: ".35" }), /* @__PURE__ */ React.createElement("stop", { offset: "100%", stopColor: "var(--rust)", stopOpacity: "0" }))), [0, 0.25, 0.5, 0.75, 1].map((t) => {
    const yy = P + (1 - t) * (H - P * 2);
    return /* @__PURE__ */ React.createElement("g", { key: t }, /* @__PURE__ */ React.createElement("line", { x1: P, x2: W - P, y1: yy, y2: yy, stroke: "var(--border)", strokeWidth: "1" }), /* @__PURE__ */ React.createElement("text", { x: 4, y: yy + 3, fill: "var(--fg-4)", fontFamily: "var(--ff-mono)", fontSize: "9.5" }, "$", (max * t).toFixed(1)));
  }), /* @__PURE__ */ React.createElement("path", { d: area, fill: "url(#g1)" }), /* @__PURE__ */ React.createElement("path", { d: path, fill: "none", stroke: "var(--rust)", strokeWidth: "1.8", strokeLinejoin: "round" }), data.map((v, i) => i % 4 === 0 && /* @__PURE__ */ React.createElement("text", { key: i, x: x(i), y: H - 8, fill: "var(--fg-4)", fontFamily: "var(--ff-mono)", fontSize: "9.5", textAnchor: "middle" }, i.toString().padStart(2, "0"), "h")));
};
function KnowledgePage() {
  const [graph, , refresh] = usePolling("/api/knowledge", 3e4);
  const [query, setQuery] = useState("");
  const [submittedQuery, setSubmittedQuery] = useState(null);
  const [serverResult, setServerResult] = useState(null);
  const [queryErr, setQueryErr] = useState(null);
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
      setSubmittedQuery(null);
      setServerResult(null);
      return;
    }
    try {
      const r = await rhFetch("/api/knowledge/query", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(parsed.pattern)
      });
      setSubmittedQuery(query);
      setServerResult(r);
    } catch (e) {
      setQueryErr(String(e.message || e));
      setServerResult(null);
    }
  };
  const clearQuery = () => {
    setQuery("");
    setSubmittedQuery(null);
    setServerResult(null);
    setQueryErr(null);
  };
  const usingServer = !!serverResult;
  const allNodes = (usingServer ? serverResult.nodes : graph && graph.nodes) || [];
  const allEdges = (usingServer ? serverResult.edges : graph && graph.edges) || [];
  const filterLow = !usingServer && query && !query.includes(":") ? query.trim().toLowerCase() : "";
  const nodes = !filterLow ? allNodes : allNodes.filter((n) => (n.name || "").toLowerCase().includes(filterLow) || (n.type || "").toLowerCase().includes(filterLow) || (n.id || "").toLowerCase().includes(filterLow));
  const keepIds = new Set(nodes.map((n) => n.id));
  const edges = allEdges.filter((e) => keepIds.has(e.source || e.source_id) && keepIds.has(e.target || e.target_id) || filterLow && (e.relation || "").toLowerCase().includes(filterLow));
  const [activeId, setActiveId] = useState(null);
  const active = nodes.find((n) => n.id === activeId) || nodes[0] || null;
  const activeEdges = edges.filter((e) => e.source === (active && active.id) || e.target === (active && active.id) || e.source_id === (active && active.id) || e.target_id === (active && active.id));
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Knowledge graph ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", filterLow ? `${nodes.length} of ${allNodes.length}` : allNodes.length, " nodes \xB7 ", filterLow ? `${edges.length} of ${allEdges.length}` : allEdges.length, " edges", usingServer && /* @__PURE__ */ React.createElement("span", null, " \xB7 ", /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, "query mode")))), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Backend ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/knowledge/query"), " for structured queries (", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "source:foo relation:works_at depth:3"), "); plain text falls back to client-side substring filter.")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("div", { className: "search-field", style: { minWidth: 340 } }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement(
    "input",
    {
      placeholder: "source:\u2026 relation:works_at depth:3 \u2014 or plain substring",
      value: query,
      onChange: (e) => setQuery(e.target.value),
      onKeyDown: (e) => {
        if (e.key === "Enter") runQuery();
      }
    }
  ), (query || submittedQuery) && /* @__PURE__ */ React.createElement("button", { className: "kbd", onClick: clearQuery, style: { cursor: "pointer" } }, "clear")), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: runQuery, disabled: !query.trim() }, /* @__PURE__ */ React.createElement(I.play, null), " Run"))), queryErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "QUERY FAILED"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, queryErr)), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card", style: { padding: 0, overflow: "hidden" } }, !graph && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "40px", fontSize: 12, textAlign: "center" } }, "loading graph\u2026"), graph && nodes.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "40px", fontSize: 12, textAlign: "center" } }, "No knowledge graph data yet."), graph && nodes.length > 0 && /* @__PURE__ */ React.createElement(KGViz, { nodes, edges, onSelect: setActiveId, activeId: active && active.id })), /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, active ? `Node \xB7 ${active.type || "entity"} \xB7 ${active.name || active.id}` : "Select a node"), active && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "kv mb-12" }, /* @__PURE__ */ React.createElement("dt", null, "id"), /* @__PURE__ */ React.createElement("dd", null, active.id), /* @__PURE__ */ React.createElement("dt", null, "kind"), /* @__PURE__ */ React.createElement("dd", null, active.type || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "degree"), /* @__PURE__ */ React.createElement("dd", null, activeEdges.length)), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Edges (", activeEdges.length, ")"), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 240, overflow: "auto" } }, activeEdges.slice(0, 16).map((e, i) => {
    var _a;
    const src = e.source || e.source_id;
    const dst = e.target || e.target_id;
    const other = src === active.id ? dst : src;
    return /* @__PURE__ */ React.createElement("div", { key: i, className: "row between", style: { padding: "5px 8px", background: "var(--bg-2)", borderRadius: 5, fontSize: 11.5, fontFamily: "var(--ff-mono)" } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, e.relation || e.label || "\u2192"), /* @__PURE__ */ React.createElement("span", { className: "muted" }, "\u2192 ", ((_a = nodes.find((n) => n.id === other)) == null ? void 0 : _a.name) || other));
  })))))));
}
const KGViz = ({ nodes: rawNodes, edges: rawEdges, onSelect, activeId }) => {
  const N = Math.min(rawNodes.length, 24);
  const cx = 350, cy = 180, r = 130;
  const typeColors = {
    person: "var(--rust)",
    project: "var(--live)",
    file: "var(--sky)",
    company: "var(--violet)",
    document: "var(--fg-3)",
    event: "var(--amber)"
  };
  const nodes = rawNodes.slice(0, N).map((n, i) => ({
    id: n.id,
    x: cx + r * Math.cos(i / N * Math.PI * 2 - Math.PI / 2),
    y: cy + r * Math.sin(i / N * Math.PI * 2 - Math.PI / 2),
    r: 11,
    label: (n.name || n.id || "").slice(0, 14),
    c: typeColors[(n.type || "").toLowerCase()] || "var(--rust)"
  }));
  const edges = rawEdges.map((e) => [e.source || e.source_id, e.target || e.target_id]).filter(([a, b]) => a && b);
  const by = {};
  nodes.forEach((n) => by[n.id] = n);
  return /* @__PURE__ */ React.createElement("div", { style: {
    height: 420,
    position: "relative",
    background: `radial-gradient(600px 320px at 50% 40%, oklch(0.665 0.165 50 / .08), transparent 60%),
                  linear-gradient(var(--border) 1px, transparent 1px) 0 0/24px 24px,
                  linear-gradient(90deg, var(--border) 1px, transparent 1px) 0 0/24px 24px,
                  var(--bg-2)`
  } }, /* @__PURE__ */ React.createElement("svg", { viewBox: "0 0 700 360", width: "100%", height: "100%" }, edges.map(([a, b], i) => {
    if (!by[a] || !by[b]) return null;
    return /* @__PURE__ */ React.createElement(
      "line",
      {
        key: i,
        x1: by[a].x,
        y1: by[a].y,
        x2: by[b].x,
        y2: by[b].y,
        stroke: "var(--border-hi)",
        strokeWidth: "1",
        opacity: ".8"
      }
    );
  }), nodes.map((n) => {
    const isActive = n.id === activeId;
    return /* @__PURE__ */ React.createElement("g", { key: n.id, style: { cursor: onSelect ? "pointer" : void 0 }, onClick: () => onSelect && onSelect(n.id) }, /* @__PURE__ */ React.createElement("circle", { cx: n.x, cy: n.y, r: n.r + (isActive ? 8 : 5), fill: n.c, opacity: isActive ? ".22" : ".12" }), /* @__PURE__ */ React.createElement("circle", { cx: n.x, cy: n.y, r: n.r, fill: "var(--bg)", stroke: n.c, strokeWidth: isActive ? "3" : "2" }), /* @__PURE__ */ React.createElement(
      "text",
      {
        x: n.x,
        y: n.y + n.r + 12,
        textAnchor: "middle",
        fill: "var(--fg-2)",
        fontFamily: "var(--ff-mono)",
        fontSize: "10.5"
      },
      n.label
    ));
  })), /* @__PURE__ */ React.createElement("div", { style: { position: "absolute", bottom: 10, left: 14, fontFamily: "var(--ff-mono)", fontSize: 10.5, color: "var(--fg-4)" } }, rawNodes.length, " nodes \xB7 ", rawEdges.length, " edges \xB7 live from ", /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, "/api/knowledge")));
};
function SkillsPage() {
  const [skillsResp, , refresh] = usePolling("/api/skills", 3e4);
  const skills = Array.isArray(skillsResp) ? skillsResp : skillsResp && skillsResp.skills || [];
  const [showCustom, setShowCustom] = useState(false);
  const [showClawHub, setShowClawHub] = useState(false);
  const [rowMenu, setRowMenu] = useState(null);
  const uninstall = async (name) => {
    if (!confirm(`Uninstall skill ${name}?`)) return;
    try {
      await rhFetch("/api/skills/uninstall", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name })
      });
      refresh();
    } catch (e) {
      toastErr(`uninstall failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Skills ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", skills.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Bundled + ", /* @__PURE__ */ React.createElement("span", { className: "mono", style: { color: "var(--rust)" } }, "ClawHub"), " marketplace \xB7 WASM sandbox \xB7 capability gating")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, "Reload"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowCustom(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Install custom"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowClawHub(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " ClawHub"))), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Skill"), /* @__PURE__ */ React.createElement("th", null, "Origin"), /* @__PURE__ */ React.createElement("th", null, "Runtime"), /* @__PURE__ */ React.createElement("th", null, "Version"), /* @__PURE__ */ React.createElement("th", null, "Enabled"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !skillsResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), skillsResp && skills.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No skills installed.")), skills.map((s) => {
    const origin = (s.source || s.origin || (s.privileged ? "privileged" : "builtin")).toString().toLowerCase();
    const cat = s.category || s.runtime || s.type || "\u2014";
    const ver = s.version || "\u2014";
    const en = s.enabled !== false;
    const isBundled = origin === "bundled" || origin === "builtin";
    return /* @__PURE__ */ React.createElement("tr", { key: s.name }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "mono" }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, "\u203A"), " ", s.name)), /* @__PURE__ */ React.createElement("td", null, origin === "clawhub" || origin === "claw" ? /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, "ClawHub") : origin === "privileged" ? /* @__PURE__ */ React.createElement("span", { className: "badge warn" }, "privileged") : /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, origin)), /* @__PURE__ */ React.createElement("td", { className: "muted mono" }, s.runtime || cat), /* @__PURE__ */ React.createElement("td", { className: "mono" }, ver), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "switch " + (en ? "on" : "") })), /* @__PURE__ */ React.createElement("td", { className: "right", style: { position: "relative" } }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setRowMenu(rowMenu === s.name ? null : s.name) }, /* @__PURE__ */ React.createElement(I.more, null)), rowMenu === s.name && /* @__PURE__ */ React.createElement("div", { className: "row-menu", onClick: (e) => e.stopPropagation() }, isBundled ? /* @__PURE__ */ React.createElement("button", { disabled: true, title: "bundled skills cannot be uninstalled" }, "Bundled \u2014 cannot remove") : /* @__PURE__ */ React.createElement("button", { onClick: () => {
      setRowMenu(null);
      uninstall(s.name);
    }, style: { color: "var(--crimson)" } }, /* @__PURE__ */ React.createElement(I.close, null), " Uninstall"))));
  })))), showCustom && /* @__PURE__ */ React.createElement(SkillInstallModal, { onClose: () => setShowCustom(false), onInstalled: () => {
    setShowCustom(false);
    refresh();
  } }), showClawHub && /* @__PURE__ */ React.createElement(ClawHubModal, { onClose: () => setShowClawHub(false), onInstalled: () => {
    setShowClawHub(false);
    refresh();
  } }));
}
function ClawHubModal({ onClose, onInstalled }) {
  useEscapeKey(onClose);
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState("trending");
  const path = query.trim() ? `/api/clawhub/search?q=${encodeURIComponent(query)}&limit=30` : `/api/clawhub/browse?sort=${encodeURIComponent(sort)}&limit=30`;
  const [resp, fetchErr, refresh] = useApi(path);
  const items = resp && resp.items || [];
  const [installing, setInstalling] = useState(null);
  const [result, setResult] = useState(null);
  const install = async (slug) => {
    setInstalling(slug);
    setResult(null);
    try {
      const r = await rhFetch("/api/clawhub/install", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ slug })
      });
      setResult({ slug, ok: true, message: r && (r.message || r.status) || "Installed" });
      onInstalled();
    } catch (e) {
      setResult({ slug, ok: false, message: String(e.message || e) });
    } finally {
      setInstalling(null);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "ClawHub marketplace"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mb-12" }, /* @__PURE__ */ React.createElement("div", { className: "search-field", style: { flex: 1 } }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("input", { placeholder: "Search skills (empty = browse trending)\u2026", value: query, onChange: (e) => setQuery(e.target.value) })), !query.trim() && /* @__PURE__ */ React.createElement("select", { className: "t-select", value: sort, onChange: (e) => setSort(e.target.value) }, ["trending", "downloads", "stars", "updated", "rating"].map((s) => /* @__PURE__ */ React.createElement("option", { key: s, value: s }, s))), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), !resp && !fetchErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "12px" } }, "loading\u2026"), resp && items.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "12px" } }, "No skills found."), /* @__PURE__ */ React.createElement("div", { className: "col gap-6", style: { maxHeight: 420, overflow: "auto" } }, items.map((it) => {
    const slug = it.slug || it.id || it.name;
    const isInstalling = installing === slug;
    const ok = result && result.slug === slug && result.ok;
    const err = result && result.slug === slug && !result.ok;
    return /* @__PURE__ */ React.createElement("div", { key: slug, className: "row gap-12", style: { padding: "10px 12px", border: "1px solid var(--border)", borderRadius: 7, background: "var(--bg-2)" } }, /* @__PURE__ */ React.createElement("div", { className: "col", style: { flex: 1, gap: 3, minWidth: 0 } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 13 } }, it.name || slug), it.version && /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, it.version), it.author && /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, it.author)), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, it.description || it.summary || "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "row gap-12 dim mono", style: { fontSize: 10.5 } }, it.downloads != null && /* @__PURE__ */ React.createElement("span", null, "\u2193 ", Number(it.downloads).toLocaleString()), it.stars != null && /* @__PURE__ */ React.createElement("span", null, "\u2605 ", it.stars), it.rating != null && /* @__PURE__ */ React.createElement("span", null, Number(it.rating).toFixed(1), "/5"), it.updated && /* @__PURE__ */ React.createElement("span", null, "upd ", relativeTime(it.updated)))), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, ok && /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "installed"), err && /* @__PURE__ */ React.createElement("span", { className: "badge error", title: result.message }, "failed"), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => install(slug), disabled: !!installing }, isInstalling ? "Installing\u2026" : "Install")));
  })), result && !result.ok && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "FAIL"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, result.message))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Close"))));
}
function SkillInstallModal({ onClose, onInstalled }) {
  useEscapeKey(onClose);
  const [name, setName] = useState("");
  const [language, setLanguage] = useState("python");
  const [description, setDescription] = useState("");
  const [content, setContent] = useState(`def run(input):
    """Echo skill \u2014 returns its input."""
    return {"echo": input}
`);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const submit = async () => {
    if (!/^[a-z][a-z0-9_]{0,63}$/.test(name)) {
      setErr("name must match ^[a-z][a-z0-9_]{0,63}$");
      return;
    }
    if (!content.trim()) {
      setErr("content is empty");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await rhFetch("/api/skills/install-custom", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ name, language, description, content })
      });
      onInstalled();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Install custom skill"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name (^[a-z][a-z0-9_]", "{0,63}", "$)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value), placeholder: "echo_skill" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Language"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: language, onChange: (e) => setLanguage(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "python" }, "python"), /* @__PURE__ */ React.createElement("option", { value: "javascript" }, "javascript (node)"))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Description"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: description, onChange: (e) => setDescription(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Body (define `run(input)`; wrapper boilerplate auto-added)"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 240, fontFamily: "var(--ff-mono)" }, value: content, onChange: (e) => setContent(e.target.value) }))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Installing\u2026" : "Install"))));
}
function ApprovalsPage() {
  const [resp, , refresh] = usePolling("/api/approvals", 1e4);
  const rows = resp && resp.approvals || [];
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Approvals ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", resp ? rows.length : "\u2026")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Inline keyboard buttons push to bound channels \xB7 decisions written to audit chain")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, !resp && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "loading\u2026"), resp && rows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No approvals waiting."), rows.length > 0 && /* @__PURE__ */ React.createElement(ApprovalsTable, { rows, onChange: refresh })));
}
function AuditPage() {
  const [audit, , refresh] = usePolling("/api/audit/recent?n=50", 8e3);
  const [verify, verifyErr, verifyRefresh] = useApi("/api/audit/verify");
  const entries = audit && audit.entries || [];
  const actorCounts = {};
  for (const e of entries) {
    const a = e.agent_name || e.agent_id || "kernel";
    actorCounts[a] = (actorCounts[a] || 0) + 1;
  }
  const topActor = Object.entries(actorCounts).sort((a, b) => b[1] - a[1])[0];
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Audit log"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Merkle hash chain \xB7 ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/data/audit.jsonl"), " \xB7 replayed on boot")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    refresh();
    verifyRefresh();
  } }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: verifyRefresh }, /* @__PURE__ */ React.createElement(I.shield, null), " Verify chain"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    if (!audit) return;
    downloadBlob(
      `rustyhand-audit-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.json`,
      JSON.stringify(audit, null, 2),
      "application/json"
    );
  } }, /* @__PURE__ */ React.createElement(I.download, null), " Export"))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Chain head"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, color: "var(--rust)", wordBreak: "break-all" } }, audit && audit.tip_hash ? String(audit.tip_hash).slice(0, 16) : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, "depth ", audit ? audit.total != null ? audit.total.toLocaleString() : "\u2014" : "\u2026"), /* @__PURE__ */ React.createElement("div", { className: "divider" }), verify ? /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, verify.valid ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, /* @__PURE__ */ React.createElement(I.check, null), " verified") : /* @__PURE__ */ React.createElement("span", { className: "badge error" }, /* @__PURE__ */ React.createElement(I.warn, null), " mismatch"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, (verify.entries || []).length || verify.total || 0, " entries")) : /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, verifyErr || "verifying\u2026")), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Loaded window"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 20 } }, entries.length, " ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 13 } }, "entries")), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, "from /api/audit/recent?n=50")), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Top actor"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14 } }, topActor ? topActor[0] : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, topActor ? `${topActor[1]} entries (in window)` : "no activity")), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Warning"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, color: verify && verify.warning ? "var(--amber)" : "var(--live)" } }, verify && verify.warning ? "see below" : "none"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, verify && verify.warning ? verify.warning : "audit chain stable"))), /* @__PURE__ */ React.createElement("div", { className: "card flush mt-16" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Chain \xB7 most recent"), /* @__PURE__ */ React.createElement("span", { className: "mono dim" }, "descending")), /* @__PURE__ */ React.createElement("div", null, !audit && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "loading\u2026"), audit && entries.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No audit entries yet."), entries.map((a) => {
    const hash = a.hash ? String(a.hash).slice(0, 12) : "\u2014";
    return /* @__PURE__ */ React.createElement("div", { key: a.hash || a.seq, className: "merkle-row" }, /* @__PURE__ */ React.createElement("div", { className: "chain" }), /* @__PURE__ */ React.createElement("span", { className: "time" }, formatTime(a.timestamp)), /* @__PURE__ */ React.createElement("span", null, /* @__PURE__ */ React.createElement("span", { className: "action" }, a.action), " ", /* @__PURE__ */ React.createElement("span", { className: "dim" }, "\xB7"), " ", /* @__PURE__ */ React.createElement("span", { className: "actor" }, a.agent_name || a.agent_id || "kernel"), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11, marginTop: 2 } }, a.detail || a.outcome || "")), /* @__PURE__ */ React.createElement("span", { className: "hash" }, hash), /* @__PURE__ */ React.createElement("span", { className: "dim" }, "seq ", a.seq));
  }))));
}
function SettingsPage() {
  const [providersResp, , refreshProviders] = usePolling("/api/providers", 3e4);
  const [config] = useApi("/api/config");
  const [health] = useApi("/api/health/detail");
  const [onboarding] = useApi("/api/onboarding");
  const [editing, setEditing] = useState(null);
  const providers = providersResp && providersResp.providers || [];
  const apiListen = config && (config.api_listen || config.api && config.api.listen) || "\u2014";
  const proxy = config && (config.proxy_url || config.proxy && config.proxy.url) || null;
  const version = health && health.version || "0.7.46";
  const uptime = health && health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "\u2014";
  const agentCount = health && health.agent_count != null ? health.agent_count : "\u2014";
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Settings"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Config at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " \xB7 50+ fields with serde defaults \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/config")))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "LLM providers"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, providers.length, " loaded \xB7 auto-probe at boot")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Provider"), /* @__PURE__ */ React.createElement("th", null, "Env var"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Models"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !providersResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 5, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), providers.map((p) => {
    const auth = (p.auth_status || "").toLowerCase();
    let badge;
    if (auth === "ok") badge = /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "set");
    else if (auth === "missing" && p.key_required === false) badge = /* @__PURE__ */ React.createElement("span", { className: "badge sky" }, "local");
    else if (p.id === "mock" || auth === "fallback") badge = /* @__PURE__ */ React.createElement("span", { className: "badge demo" }, "fallback");
    else if (auth === "invalid") badge = /* @__PURE__ */ React.createElement("span", { className: "badge error" }, "invalid");
    else badge = /* @__PURE__ */ React.createElement("span", { className: "badge idle" }, "not set");
    return /* @__PURE__ */ React.createElement("tr", { key: p.id || p.name }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, p.display_name || p.id || p.name), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, p.api_key_env || "\u2014"), /* @__PURE__ */ React.createElement("td", null, badge), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, p.model_count != null ? p.model_count : "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setEditing(p) }, "Edit")));
  })))), /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "API"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "listen"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, apiListen), /* @__PURE__ */ React.createElement("dt", null, "auth"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, config && config.bearer_token ? "\u2022\u2022\u2022\u2022\u2022\u2022\u2022" : "localhost-only"), /* @__PURE__ */ React.createElement("dt", null, "ws origins"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, config && (config.allowed_ws_origins || []).join(", ") || "localhost"), /* @__PURE__ */ React.createElement("dt", null, "proxy"), /* @__PURE__ */ React.createElement("dd", { className: proxy ? "mono" : "dim" }, proxy || "none"))), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Demo mode"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "active"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.demo_mode ? "yes" : "no" : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "provider"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, onboarding ? onboarding.provider : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "api_key"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.api_key_set ? "set" : "missing" : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "agents"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.agent_count : "\u2026")), /* @__PURE__ */ React.createElement("div", { className: "dim mt-8", style: { fontSize: 11 } }, "set ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "RUSTYHAND_DISABLE_DEMO_MODE=1"), " to fall back to NullDriver")), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Build"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "version"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, version), /* @__PURE__ */ React.createElement("dt", null, "agents"), /* @__PURE__ */ React.createElement("dd", null, agentCount), /* @__PURE__ */ React.createElement("dt", null, "uptime"), /* @__PURE__ */ React.createElement("dd", null, uptime), /* @__PURE__ */ React.createElement("dt", null, "panics"), /* @__PURE__ */ React.createElement("dd", null, health && health.panic_count != null ? health.panic_count : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "restarts"), /* @__PURE__ */ React.createElement("dd", null, health && health.restart_count != null ? health.restart_count : "\u2014"))))), editing && /* @__PURE__ */ React.createElement(ProviderKeyModal, { provider: editing, onClose: () => setEditing(null), onSaved: () => {
    setEditing(null);
    refreshProviders();
  } }));
}
function ProviderKeyModal({ provider, onClose, onSaved }) {
  useEscapeKey(onClose);
  const [key, setKey] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const [testResult, setTestResult] = useState(null);
  const name = provider.id || provider.name;
  const save = async () => {
    if (!key.trim()) {
      setErr("key is empty");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(name)}/key`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ key })
      });
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  const clear = async () => {
    if (!confirm(`Delete API key for ${name}?`)) return;
    setBusy(true);
    setErr(null);
    try {
      await rhFetch(`/api/providers/${encodeURIComponent(name)}/key`, { method: "DELETE" });
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  const test = async () => {
    setBusy(true);
    setErr(null);
    setTestResult(null);
    try {
      const r = await rhFetch(`/api/providers/${encodeURIComponent(name)}/test`, { method: "POST" });
      setTestResult(r);
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Edit provider \xB7 ", provider.display_name || name), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "kv mb-12" }, /* @__PURE__ */ React.createElement("dt", null, "id"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, name), /* @__PURE__ */ React.createElement("dt", null, "env var"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, provider.api_key_env || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "auth status"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, provider.auth_status || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "models"), /* @__PURE__ */ React.createElement("dd", null, provider.model_count != null ? provider.model_count : "\u2014")), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "New API key (will overwrite stored value, zeroized after use)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "password", placeholder: provider.api_key_env || "sk-\u2026", value: key, onChange: (e) => setKey(e.target.value) })), testResult && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: testResult.ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot " + (testResult.ok ? "live" : "err") }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, testResult.ok ? "OK" : "FAIL"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, testResult.message || testResult.detail || JSON.stringify(testResult).slice(0, 200))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: clear, disabled: busy, style: { marginRight: "auto", color: "var(--crimson)" } }, "Delete key"), /* @__PURE__ */ React.createElement("button", { className: "btn", onClick: test, disabled: busy }, "Test"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy || !key.trim() }, busy ? "Saving\u2026" : "Save"))));
}
function MemoryPage() {
  const pg = usePagination(50);
  const [resp, fetchErr, refresh] = usePolling(
    `/api/sessions?limit=${pg.pageSize}&offset=${pg.offset}`,
    2e4
  );
  React.useEffect(() => {
    if (resp && resp.total != null) pg.setTotal(resp.total);
  }, [resp && resp.total]);
  const sessions = resp && resp.sessions || [];
  const [labelEditing, setLabelEditing] = useState(null);
  const [labelDraft, setLabelDraft] = useState("");
  const fileInputRef = useRef(null);
  const remove = async (id) => {
    if (!confirm(`Delete session ${String(id).slice(0, 8)}? This cannot be undone.`)) return;
    try {
      await rhFetch(`/api/sessions/${encodeURIComponent(id)}`, { method: "DELETE" });
      refresh();
    } catch (e) {
      toastErr(`delete failed: ${e.message || e}`);
    }
  };
  const saveLabel = async (id) => {
    try {
      await rhFetch(`/api/sessions/${encodeURIComponent(id)}/label`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ label: labelDraft })
      });
      setLabelEditing(null);
      refresh();
    } catch (e) {
      toastErr(`label set failed: ${e.message || e}`);
    }
  };
  const exportMarkdown = (id) => {
    rhFetch(`/api/sessions/${encodeURIComponent(id)}/export.md`).then((md) => downloadBlob(`session-${String(id).slice(0, 8)}.md`, md, "text/markdown")).catch((e) => toastErr(`export failed: ${e.message || e}`));
  };
  const backupMemory = () => {
    rhFetch("/api/memory/export?format=json").then((data) => {
      const blob = typeof data === "string" ? data : JSON.stringify(data, null, 2);
      downloadBlob(`rustyhand-memory-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.json`, blob, "application/json");
    }).catch((e) => toastErr(`export failed: ${e.message || e}`));
  };
  const importMemory = async (file) => {
    if (!file) return;
    const text = await file.text();
    try {
      const r = await rhFetch("/api/memory/import", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: text
      });
      toastOk(`Imported: ${r.imported || r.message || "ok"}`);
      refresh();
    } catch (e) {
      toastErr(`import failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Memory ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", sessions.length, " session(s)")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "SQLite-backed sessions \xB7 backup at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/memory/export"), " \xB7 restore at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/memory/import"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => fileInputRef.current && fileInputRef.current.click() }, /* @__PURE__ */ React.createElement(I.copy, null), " Restore"), /* @__PURE__ */ React.createElement(
    "input",
    {
      ref: fileInputRef,
      type: "file",
      accept: "application/json,.json",
      style: { display: "none" },
      onChange: (e) => importMemory(e.target.files && e.target.files[0])
    }
  ), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: backupMemory }, /* @__PURE__ */ React.createElement(I.download, null), " Backup"))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Session"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Label"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Messages"), /* @__PURE__ */ React.createElement("th", null, "Created"), /* @__PURE__ */ React.createElement("th", null, "Updated"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !resp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), resp && sessions.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No sessions yet.")), sessions.map((s) => /* @__PURE__ */ React.createElement("tr", { key: s.session_id }, /* @__PURE__ */ React.createElement("td", { className: "mono", style: { maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis" } }, String(s.session_id).slice(0, 8)), /* @__PURE__ */ React.createElement("td", { className: "mono" }, s.agent_name || s.agent_id || "\u2014"), /* @__PURE__ */ React.createElement("td", null, labelEditing === s.session_id ? /* @__PURE__ */ React.createElement("span", { className: "row gap-4" }, /* @__PURE__ */ React.createElement(
    "input",
    {
      className: "modal-field",
      style: { padding: "3px 6px", fontSize: 11.5 },
      autoFocus: true,
      value: labelDraft,
      onChange: (e) => setLabelDraft(e.target.value),
      onKeyDown: (e) => {
        if (e.key === "Enter") saveLabel(s.session_id);
        if (e.key === "Escape") setLabelEditing(null);
      }
    }
  ), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => saveLabel(s.session_id) }, "save")) : /* @__PURE__ */ React.createElement("span", { style: { cursor: "pointer" }, onClick: () => {
    setLabelEditing(s.session_id);
    setLabelDraft(s.label || "");
  } }, s.label || /* @__PURE__ */ React.createElement("span", { className: "dim" }, "\u2014 click to set \u2014"))), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, Number(s.message_count || 0).toLocaleString()), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, relativeTime(s.created_at)), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, relativeTime(s.updated_at)), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => exportMarkdown(s.session_id), title: "Export markdown" }, /* @__PURE__ */ React.createElement(I.download, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => remove(s.session_id), title: "Delete" }, /* @__PURE__ */ React.createElement(I.close, null))))))), /* @__PURE__ */ React.createElement(Pagination, { pg })));
}
function McpPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/mcp/servers", 3e4);
  const configured = resp && resp.configured || [];
  const connected = resp && resp.connected || [];
  const connectedNames = new Set(connected.map((c) => c.name));
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "MCP servers ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", configured.length, " configured \xB7 ", connected.length, " connected")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Model-Context-Protocol bridges \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/mcp/servers"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), !resp && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px", fontSize: 12 } }, "loading\u2026"), resp && configured.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px", fontSize: 12 } }, "No MCP servers configured. Add them in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " under ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "[[mcp.servers]]"), "."), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, configured.map((s) => {
    const isConnected = connectedNames.has(s.name);
    const transport = s.transport || {};
    return /* @__PURE__ */ React.createElement("div", { key: s.name, className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, fontWeight: 500 } }, s.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, transport.type || "\u2014", " \xB7 ", s.timeout_secs ? `${s.timeout_secs}s timeout` : "no timeout")), /* @__PURE__ */ React.createElement("span", { className: "badge " + (isConnected ? "live" : "idle") }, isConnected ? "connected" : "idle")), /* @__PURE__ */ React.createElement("div", { className: "kv" }, transport.type === "stdio" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "command"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, transport.command), /* @__PURE__ */ React.createElement("dt", null, "args"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, (transport.args || []).join(" ") || "\u2014")), transport.type === "sse" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "url"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, transport.url)), s.env && Object.keys(s.env).length > 0 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "env"), /* @__PURE__ */ React.createElement("dd", { className: "mono dim" }, Object.keys(s.env).join(", ")))));
  })));
}
function NetworkPage() {
  const [status, , refreshStatus] = usePolling("/api/network/status", 15e3);
  const [peersResp, , refreshPeers] = usePolling("/api/peers", 15e3);
  const peers = peersResp && peersResp.peers || [];
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Network"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "RHP peer-to-peer protocol (JSON-RPC over TCP) \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/network/status"), " \xB7 ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/peers"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    refreshStatus();
    refreshPeers();
  } }, /* @__PURE__ */ React.createElement(I.refresh, null)))), /* @__PURE__ */ React.createElement("div", { className: "tiles" }, /* @__PURE__ */ React.createElement(Tile, { label: "Network state", value: status ? status.enabled ? "enabled" : "disabled" : "\u2026", foot: status && status.node_id ? `node ${String(status.node_id).slice(0, 12)}` : "no node id", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }), /* @__PURE__ */ React.createElement(Tile, { label: "Listen address", value: status && status.listen_address ? String(status.listen_address) : "\u2014", foot: status ? "TCP" : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }), /* @__PURE__ */ React.createElement(Tile, { label: "Connected peers", value: status ? status.connected_peers != null ? String(status.connected_peers) : "\u2014" : "\u2026", foot: status ? `${status.total_peers || 0} known` : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }), /* @__PURE__ */ React.createElement(Tile, { label: "Loaded peers", value: `${peers.length}`, foot: peers.length === 0 ? "no peers" : "see below", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Known peers")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Node ID"), /* @__PURE__ */ React.createElement("th", null, "Address"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", null, "Last seen"))), /* @__PURE__ */ React.createElement("tbody", null, !peersResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 4, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), peersResp && peers.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 4, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No peers \u2014 network may be disabled or no peer has connected yet.")), peers.map((p, i) => /* @__PURE__ */ React.createElement("tr", { key: p.node_id || p.id || i }, /* @__PURE__ */ React.createElement("td", { className: "mono", style: { maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" } }, p.node_id || p.id || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono" }, p.address || p.endpoint || "\u2014"), /* @__PURE__ */ React.createElement("td", null, p.state ? /* @__PURE__ */ React.createElement("span", { className: "badge " + (String(p.state).toLowerCase() === "connected" ? "live" : "idle") }, p.state) : /* @__PURE__ */ React.createElement("span", { className: "badge idle" }, "\u2014")), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, p.last_seen ? relativeTime(p.last_seen) : "\u2014")))))));
}
function BindingsPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/bindings", 3e4);
  const bindings = resp && resp.bindings || [];
  const remove = async (index) => {
    if (!confirm(`Remove binding #${index}?`)) return;
    try {
      await rhFetch(`/api/bindings/${index}`, { method: "DELETE" });
      toastOk("Binding removed");
      refresh();
    } catch (e) {
      toastErr(`Remove failed: ${e.message || e}`);
    }
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Bindings ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", bindings.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Agent \u2192 channel/trigger bindings \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/bindings"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "#"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Kind"), /* @__PURE__ */ React.createElement("th", null, "Target"), /* @__PURE__ */ React.createElement("th", null, "Pattern"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !resp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), resp && bindings.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No bindings configured.")), bindings.map((b, i) => /* @__PURE__ */ React.createElement("tr", { key: i }, /* @__PURE__ */ React.createElement("td", { className: "num mono" }, i), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.agent_id || b.agent || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.kind || b.type || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.target || b.channel || b.trigger || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono dim", style: { maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis" } }, b.pattern ? JSON.stringify(b.pattern) : "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => remove(i), title: "Remove" }, /* @__PURE__ */ React.createElement(I.close, null)))))))));
}
Object.assign(window, {
  OverviewPage,
  AgentsPage,
  AgentDrawer,
  ChatPage,
  WorkflowsPage,
  AutomationPage,
  ChannelsPage,
  AnalyticsPage,
  KnowledgePage,
  SkillsPage,
  ApprovalsPage,
  AuditPage,
  SettingsPage,
  MemoryPage,
  McpPage,
  NetworkPage,
  BindingsPage
});
