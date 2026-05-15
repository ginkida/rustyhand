(function(){
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
  const version = health && health.version || "0.7.75";
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
function auditLevelOf(entry) {
  const a = ((entry && (entry.action || "")) + " " + (entry && entry.outcome || "")).toLowerCase();
  if (a.includes("error") || a.includes("panic") || a.includes("crash") || a.includes("fail")) return "error";
  if (a.includes("approval") || a.includes("denied") || a.includes("reject") || a.includes("warn")) return "warn";
  return "info";
}
function formatTime(ts) {
  if (!ts) return "\u2014";
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return String(ts).slice(11, 19);
  return d.toLocaleTimeString("en-GB", { hour12: false });
}
const ActivityFeed = ({ entries }) => {
  const cls = (c) => ({ live: "var(--live)", violet: "var(--violet)", amber: "var(--amber)", muted: "var(--fg-3)" })[c];
  const [live, setLive] = useState([]);
  const stream = useEventSource("/api/logs/stream", React.useCallback((msg) => {
    if (!msg || typeof msg !== "object") return;
    setLive((prev) => {
      const key = msg.hash || msg.seq;
      if (prev.some((p) => (p.hash || p.seq) === key)) return prev;
      return [msg, ...prev].slice(0, 200);
    });
  }, []));
  const polled = entries || [];
  const merged = React.useMemo(() => {
    const seen = /* @__PURE__ */ new Set();
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
    return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "Loading audit chain\u2026");
  }
  if (merged.length === 0) {
    return /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No audit entries yet \u2014 they appear as agents act.");
  }
  return /* @__PURE__ */ React.createElement("div", { style: { maxHeight: 360, overflow: "auto" } }, /* @__PURE__ */ React.createElement("div", { className: "row", style: { padding: "4px 14px", borderBottom: "1px solid var(--border)", gap: 8, background: "var(--bg-2)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot " + (stream.connected ? "live" : "warn") }), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, stream.connected ? "live \xB7 sse" : "stale \xB7 sse disconnected"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5, marginLeft: "auto" } }, merged.length, " events")), merged.map((it, i) => {
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
  const allChecked = selectable && rows.length > 0 && rows.every((r) => selected && selected.has(r.id));
  return /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, selectable && /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: allChecked,
      ref: (el) => {
        if (el) el.indeterminate = !!(selected && selected.size > 0 && !allChecked);
      },
      onChange: () => onToggleAll && onToggleAll(rows),
      title: allChecked ? "Deselect all" : "Select all"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Action"), /* @__PURE__ */ React.createElement("th", null, "Risk"), /* @__PURE__ */ React.createElement("th", null, "Age"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Decide"))), /* @__PURE__ */ React.createElement("tbody", null, rows.map((r) => {
    const agent = r.agent_name || r.agent || r.agent_id || "\u2014";
    const age = r.age || relativeTime(r.requested_at || r.created_at);
    const risk = (r.risk || "low").toLowerCase();
    const isSel = selectable && selected && selected.has(r.id);
    return /* @__PURE__ */ React.createElement(
      "tr",
      {
        key: r.id,
        style: { cursor: onInspect ? "pointer" : "default", background: isSel ? "var(--surface-2)" : void 0 },
        onClick: () => onInspect && onInspect(r),
        title: onInspect ? "Click to inspect full payload" : ""
      },
      selectable && /* @__PURE__ */ React.createElement("td", { onClick: (e) => {
        e.stopPropagation();
        onToggle && onToggle(r.id);
      } }, /* @__PURE__ */ React.createElement("input", { type: "checkbox", checked: !!isSel, readOnly: true, tabIndex: -1 })),
      /* @__PURE__ */ React.createElement("td", { className: "mono" }, r.id),
      /* @__PURE__ */ React.createElement("td", { className: "mono" }, agent),
      /* @__PURE__ */ React.createElement("td", null, r.action),
      /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: `badge ${risk === "high" ? "error" : risk === "medium" ? "warn" : "idle"}` }, risk)),
      /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, age),
      /* @__PURE__ */ React.createElement("td", { className: "right", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", style: { height: 24, padding: "2px 8px" }, onClick: () => decide(r.id, "approve") }, "Approve"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", style: { height: 24, padding: "2px 8px", marginLeft: 6 }, onClick: () => decide(r.id, "reject") }, "Reject"))
    );
  })));
};
function Pagination({ pg }) {
  if (!pg || pg.total === 0) return null;
  return /* @__PURE__ */ React.createElement("div", { className: "row gap-8", style: { padding: "10px 14px", borderTop: "1px solid var(--border)", justifyContent: "flex-end" } }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, pg.offset + 1, "\u2013", Math.min(pg.offset + pg.pageSize, pg.total), " of ", pg.total.toLocaleString()), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: pg.prev, disabled: !pg.hasPrev }, "\u2190 Prev"), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5, minWidth: 60, textAlign: "center" } }, pg.page, " / ", pg.totalPages), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: pg.next, disabled: !pg.hasNext }, "Next \u2192"));
}
function AgentsPage({ openAgent }) {
  const [filter, setFilterState] = useState(() => {
    try {
      const stored = localStorage.getItem("rh.panel.agentsFilter") || "all";
      return ["all", "running", "error", "idle"].includes(stored) ? stored : "all";
    } catch (e) {
      return "all";
    }
  });
  const setFilter = (v) => {
    setFilterState(v);
    try {
      localStorage.setItem("rh.panel.agentsFilter", v);
    } catch (e) {
    }
  };
  const [q, setQState] = useState(() => {
    try {
      return sessionStorage.getItem("rh.panel.agentsQ") || "";
    } catch (e) {
      return "";
    }
  });
  const setQ = (v) => {
    setQState(v);
    try {
      sessionStorage.setItem("rh.panel.agentsQ", v || "");
    } catch (e) {
    }
  };
  const [showSpawn, setShowSpawn] = useState(false);
  const [rowMenu, setRowMenu] = useState(null);
  const [selected, setSelected] = useState(() => /* @__PURE__ */ new Set());
  const [showDiff, setShowDiff] = useState(false);
  const [grouped, setGroupedState] = useState(() => {
    try {
      return localStorage.getItem("rh.panel.agentsGrouped") !== "0";
    } catch (e) {
      return true;
    }
  });
  const setGrouped = (v) => {
    setGroupedState((prev) => {
      const next = typeof v === "function" ? v(prev) : v;
      try {
        localStorage.setItem("rh.panel.agentsGrouped", next ? "1" : "0");
      } catch (e) {
      }
      return next;
    });
  };
  const [collapsedGroups, setCollapsedGroups] = useState(() => /* @__PURE__ */ new Set());
  const [compact, setCompactState] = useState(() => {
    try {
      return localStorage.getItem("rh.panel.agentsCompact") === "1";
    } catch (e) {
      return false;
    }
  });
  const setCompact = React.useCallback((v) => {
    setCompactState((prev) => {
      const next = typeof v === "function" ? v(prev) : v;
      try {
        localStorage.setItem("rh.panel.agentsCompact", next ? "1" : "0");
      } catch (e) {
      }
      return next;
    });
  }, []);
  const [resp, fetchErr, refresh] = usePolling("/api/agents?limit=200", 15e3);
  React.useEffect(() => {
    const onNew = (e) => {
      if (e.detail && e.detail.page === "agents") setShowSpawn(true);
    };
    const onRefresh = (e) => {
      if (e.detail && e.detail.page === "agents") refresh();
    };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [refresh]);
  const agents = resp && resp.agents ? resp.agents.map(normalizeAgent) : D.agents;
  const filtered = agents.filter((a) => {
    if (filter !== "all" && a.state !== filter && !(filter === "running" && a.state === "running")) return false;
    if (q && !a.name.toLowerCase().includes(q.toLowerCase()) && !a.model.toLowerCase().includes(q.toLowerCase()) && !(a.group || "").toLowerCase().includes(q.toLowerCase())) return false;
    return true;
  });
  React.useEffect(() => {
    const live = new Set(agents.map((a) => a.id));
    setSelected((prev) => {
      const next = new Set([...prev].filter((id) => live.has(id)));
      return next.size === prev.size ? prev : next;
    });
  }, [agents.map((a) => a.id).join(",")]);
  const groupBuckets = React.useMemo(() => {
    if (!grouped) return null;
    const map = /* @__PURE__ */ new Map();
    for (const a of filtered) {
      const g = a.group || "\u2014";
      if (!map.has(g)) map.set(g, []);
      map.get(g).push(a);
    }
    const ordered = [...map.entries()].sort((a, b) => a[0].localeCompare(b[0]));
    for (const [, arr] of ordered) arr.sort((x, y) => x.name.localeCompare(y.name));
    return ordered;
  }, [filtered, grouped]);
  const toggleSelect = (id) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };
  const toggleSelectAll = (visible) => {
    setSelected((prev) => {
      if (visible.every((a) => prev.has(a.id))) {
        const next2 = new Set(prev);
        for (const a of visible) next2.delete(a.id);
        return next2;
      }
      const next = new Set(prev);
      for (const a of visible) next.add(a.id);
      return next;
    });
  };
  const toggleGroup = (g) => {
    setCollapsedGroups((prev) => {
      const next = new Set(prev);
      if (next.has(g)) next.delete(g);
      else next.add(g);
      return next;
    });
  };
  const killAgent = async (id) => {
    if (!await confirmDialog({ title: "Kill agent", message: `Kill agent ${id}?`, danger: true, confirmLabel: "Kill" })) return;
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
  const forkAgent = async (id, name) => {
    const proposed = `${name}-fork`;
    const ans = window.prompt(`Fork agent ${name} as:`, proposed);
    if (!ans || !ans.trim()) return;
    try {
      const r = await rhFetch(`/api/agents/${encodeURIComponent(id)}/clone`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ new_name: ans.trim() })
      });
      toastOk(`Forked ${name} \u2192 ${ans.trim()}`);
      refresh();
    } catch (e) {
      toastErr(`fork failed: ${e.message || e}`);
    }
  };
  const bulkKill = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    if (!await confirmDialog({ title: "Kill agents", message: `Kill ${ids.length} agent(s)? This cannot be undone.`, danger: true, confirmLabel: "Kill all" })) return;
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
    setSelected(/* @__PURE__ */ new Set());
    refresh();
  };
  const bulkRestart = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    if (!await confirmDialog({ title: "Restart agents", message: `Restart ${ids.length} agent(s)?`, confirmLabel: "Restart all" })) return;
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
  const bulkMoveToGroup = async () => {
    const ids = [...selected];
    if (ids.length === 0) return;
    const known = [...new Set(agents.map((a) => a.group).filter(Boolean))].sort();
    const promptText = known.length > 0 ? `Move ${ids.length} agent(s) to group:

Existing groups: ${known.join(", ")}
(empty = ungrouped)` : `Move ${ids.length} agent(s) to group:
(empty = ungrouped)`;
    const ans = window.prompt(promptText, known[0] || "");
    if (ans == null) return;
    const targetGroup = ans.trim();
    let ok = 0, fail = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/agents/${encodeURIComponent(id)}/config`, {
          method: "PATCH",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ group: targetGroup })
        });
        ok++;
      } catch (e) {
        fail++;
        toastErr(`move ${String(id).slice(0, 8)}: ${e.message || e}`);
      }
    }
    if (ok > 0) {
      toastOk(`Moved ${ok} agent${ok === 1 ? "" : "s"} \u2192 ${targetGroup || "(ungrouped)"}${fail ? ` (${fail} failed)` : ""}`);
    }
    setSelected(/* @__PURE__ */ new Set());
    refresh();
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Agents ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", agents.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Manifests live in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "agents/<name>/agent.toml"), ". Hot-reloaded on save.")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowSpawn(true) }, /* @__PURE__ */ React.createElement(I.copy, null), " Templates"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowSpawn(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Spawn agent"))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body" }, "Failed to load agents from kernel: ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, fetchErr))), /* @__PURE__ */ React.createElement("div", { className: "filter-bar" }, /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: filter === "all" ? "on" : "", onClick: () => setFilter("all") }, "All \xB7 ", agents.length), /* @__PURE__ */ React.createElement("button", { className: filter === "running" ? "on" : "", onClick: () => setFilter("running") }, "Live \xB7 ", agents.filter((a) => a.state === "running").length), /* @__PURE__ */ React.createElement("button", { className: filter === "error" ? "on" : "", onClick: () => setFilter("error") }, "Errors \xB7 ", agents.filter((a) => a.state === "error").length), /* @__PURE__ */ React.createElement("button", { className: filter === "idle" ? "on" : "", onClick: () => setFilter("idle") }, "Idle \xB7 ", agents.filter((a) => a.state === "idle").length)), /* @__PURE__ */ React.createElement("div", { className: "search-field" }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("input", { placeholder: "Find agent, group, model\u2026", value: q, onChange: (e) => setQ(e.target.value) }), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u2318K")), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setGrouped((g) => !g), title: grouped ? "Show flat list" : "Group by team" }, grouped ? "Flat" : "Group"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setCompact((c) => !c), title: compact ? "Comfortable rows" : "Dense rows" }, compact ? "Compact" : "Cosy")), selected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, selected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: bulkRestart }, /* @__PURE__ */ React.createElement(I.refresh, null), " Restart"), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: bulkMoveToGroup, title: "Move all selected agents to a single group" }, /* @__PURE__ */ React.createElement(I.link, null), " Move to group"), selected.size === 2 && /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => setShowDiff(true), title: "Compare the two selected agents" }, /* @__PURE__ */ React.createElement(I.copy, null), " Diff"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: bulkKill }, /* @__PURE__ */ React.createElement(I.close, null), " Kill"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), /* @__PURE__ */ React.createElement("div", { className: "card flush", "data-density": compact ? "compact" : "" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 30 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: filtered.length > 0 && filtered.every((a) => selected.has(a.id)),
      ref: (el) => {
        if (el) el.indeterminate = selected.size > 0 && !filtered.every((a) => selected.has(a.id));
      },
      onChange: () => toggleSelectAll(filtered),
      title: filtered.every((a) => selected.has(a.id)) ? "Deselect all" : "Select all"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "Agent"), !grouped && /* @__PURE__ */ React.createElement("th", null, "Group"), /* @__PURE__ */ React.createElement("th", null, "Model"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Msgs"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Cost \xB7 24h"), /* @__PURE__ */ React.createElement("th", null, "Last activity"), /* @__PURE__ */ React.createElement("th", null, "Updated"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !resp && Array.from({ length: 5 }).map((_, i) => /* @__PURE__ */ React.createElement(SkelRow, { key: `s-${i}`, cols: [20, 160, 140, 90, 60, 60, 240, 50, 24] })), !grouped && filtered.map((a) => /* @__PURE__ */ React.createElement(
    AgentRow,
    {
      key: a.id,
      agent: a,
      selected: selected.has(a.id),
      onSelect: toggleSelect,
      openAgent,
      rowMenu,
      setRowMenu,
      restart: restartAgent,
      kill: killAgent,
      fork: forkAgent,
      showGroup: true
    }
  )), grouped && groupBuckets && groupBuckets.map(([g, arr]) => {
    const collapsed = collapsedGroups.has(g);
    return /* @__PURE__ */ React.createElement(React.Fragment, { key: g }, /* @__PURE__ */ React.createElement("tr", { className: "group-row", onClick: () => toggleGroup(g), style: { cursor: "pointer" } }, /* @__PURE__ */ React.createElement("td", { colSpan: 9, style: { padding: "6px 14px", background: "var(--bg-2)", borderBottom: "1px solid var(--border)" } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--fg-3)" } }, collapsed ? "\u25B8" : "\u25BE", " ", g, " ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, arr.length)))), !collapsed && arr.map((a) => /* @__PURE__ */ React.createElement(
      AgentRow,
      {
        key: a.id,
        agent: a,
        selected: selected.has(a.id),
        onSelect: toggleSelect,
        openAgent,
        rowMenu,
        setRowMenu,
        restart: restartAgent,
        kill: killAgent,
        fork: forkAgent,
        showGroup: false
      }
    )));
  })))), showSpawn && /* @__PURE__ */ React.createElement(SpawnAgentModal, { onClose: () => setShowSpawn(false), onSpawned: () => {
    setShowSpawn(false);
    refresh();
  } }), showDiff && selected.size === 2 && /* @__PURE__ */ React.createElement(
    AgentDiffModal,
    {
      agents: [...selected].map((id) => agents.find((a) => a.id === id)).filter(Boolean),
      onClose: () => setShowDiff(false)
    }
  ));
}
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
          rhFetch(`/api/agents/${encodeURIComponent(b.id)}/files/agent.toml`)
        ]);
        if (cancelled) return;
        setContentA(typeof ra === "string" ? ra : ra && ra.content || "");
        setContentB(typeof rb === "string" ? rb : rb && rb.content || "");
      } catch (e) {
        if (!cancelled) setErr(String(e.message || e));
      }
    };
    load();
    return () => {
      cancelled = true;
    };
  }, [a.id, b.id]);
  const linesA = (contentA || "").split("\n");
  const linesB = (contentB || "").split("\n");
  const setB = new Set(linesB);
  const setA = new Set(linesA);
  const onlyA = linesA.filter((l) => l.trim() && !setB.has(l)).length;
  const onlyB = linesB.filter((l) => l.trim() && !setA.has(l)).length;
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal lg", onClick: (e) => e.stopPropagation(), style: { maxWidth: 1100, width: "95%" } }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h3", { className: "modal-title" }, "Diff: ", a.name, " \u2194 ", b.name), /* @__PURE__ */ React.createElement("p", { className: "modal-sub mono", style: { fontSize: 11 } }, contentA == null || contentB == null ? "loading\u2026" : /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("span", { style: { color: "oklch(0.66 0.18 25)" } }, "\u2212", onlyA), " only in ", a.name, " \xB7", " ", /* @__PURE__ */ React.createElement("span", { style: { color: "oklch(0.66 0.15 155)" } }, "+", onlyB), " only in ", b.name))), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)", margin: "8px 16px" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "LOAD FAILED"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), !err && (contentA == null || contentB == null) && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: 24, fontSize: 12, textAlign: "center" } }, "loading manifests\u2026"), !err && contentA != null && contentB != null && /* @__PURE__ */ React.createElement("div", { className: "grid-12", style: { margin: "0 16px 16px" } }, /* @__PURE__ */ React.createElement("div", { className: "col-6" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, a.name, " ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, linesA.length, " lines")), /* @__PURE__ */ React.createElement("pre", { className: "mono", style: {
    fontSize: 11.5,
    lineHeight: 1.5,
    maxHeight: 520,
    overflow: "auto",
    background: "var(--bg-2)",
    padding: "10px 12px",
    borderRadius: 6,
    margin: 0
  } }, linesA.map((l, i) => {
    const inB = setB.has(l);
    return /* @__PURE__ */ React.createElement("div", { key: i, style: {
      background: !inB && l.trim() ? "oklch(0.66 0.18 25 / .12)" : "transparent",
      borderLeft: !inB && l.trim() ? "2px solid oklch(0.66 0.18 25 / .7)" : "2px solid transparent",
      padding: "0 6px",
      whiteSpace: "pre-wrap",
      wordBreak: "break-word"
    } }, l || " ");
  }))), /* @__PURE__ */ React.createElement("div", { className: "col-6" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, b.name, " ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, linesB.length, " lines")), /* @__PURE__ */ React.createElement("pre", { className: "mono", style: {
    fontSize: 11.5,
    lineHeight: 1.5,
    maxHeight: 520,
    overflow: "auto",
    background: "var(--bg-2)",
    padding: "10px 12px",
    borderRadius: 6,
    margin: 0
  } }, linesB.map((l, i) => {
    const inA = setA.has(l);
    return /* @__PURE__ */ React.createElement("div", { key: i, style: {
      background: !inA && l.trim() ? "oklch(0.66 0.15 155 / .12)" : "transparent",
      borderLeft: !inA && l.trim() ? "2px solid oklch(0.66 0.15 155 / .7)" : "2px solid transparent",
      padding: "0 6px",
      whiteSpace: "pre-wrap",
      wordBreak: "break-word"
    } }, l || " ");
  }))))));
}
function AgentRow({ agent, selected, onSelect, openAgent, rowMenu, setRowMenu, restart, kill, fork, showGroup }) {
  const a = agent;
  return /* @__PURE__ */ React.createElement(
    "tr",
    {
      key: a.id,
      style: { cursor: "pointer", background: selected ? "var(--surface-2)" : void 0 },
      onClick: () => openAgent(a)
    },
    /* @__PURE__ */ React.createElement("td", { onClick: (e) => {
      e.stopPropagation();
      onSelect(a.id);
    }, style: { width: 30 } }, /* @__PURE__ */ React.createElement("input", { type: "checkbox", checked: selected, readOnly: true, tabIndex: -1 })),
    /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "agent-row" }, /* @__PURE__ */ React.createElement(Avatar, { agent: a }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "name" }, a.name), /* @__PURE__ */ React.createElement("div", { className: "meta" }, a.id)))),
    showGroup && /* @__PURE__ */ React.createElement("td", { className: "muted mono" }, a.group),
    /* @__PURE__ */ React.createElement("td", { className: "mono" }, a.model, /* @__PURE__ */ React.createElement("div", { className: "meta dim" }, a.provider)),
    /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(StateBadge, { state: a.state })),
    /* @__PURE__ */ React.createElement("td", { className: "num mono" }, a.messages.toLocaleString()),
    /* @__PURE__ */ React.createElement("td", { className: "num mono" }, "$", a.cost.toFixed(2)),
    /* @__PURE__ */ React.createElement("td", { className: "muted", style: { maxWidth: 280, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" } }, a.last),
    /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, a.updated),
    /* @__PURE__ */ React.createElement("td", { style: { position: "relative" } }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: (e) => {
      e.stopPropagation();
      setRowMenu(rowMenu === a.id ? null : a.id);
    } }, /* @__PURE__ */ React.createElement(I.more, null)), rowMenu === a.id && /* @__PURE__ */ React.createElement("div", { className: "row-menu", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("button", { onClick: () => {
      setRowMenu(null);
      restart(a.id);
    } }, /* @__PURE__ */ React.createElement(I.refresh, null), " Restart"), /* @__PURE__ */ React.createElement("button", { onClick: () => {
      setRowMenu(null);
      fork && fork(a.id, a.name);
    } }, /* @__PURE__ */ React.createElement(I.copy, null), " Fork\u2026"), /* @__PURE__ */ React.createElement("button", { onClick: () => {
      setRowMenu(null);
      kill(a.id);
    }, style: { color: "var(--crimson)" } }, /* @__PURE__ */ React.createElement(I.close, null), " Kill")))
  );
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
  const [detail, , refreshDetail] = useApi(agent ? `/api/agents/${agent.id}` : null);
  const [recent] = useApi(agent ? `/api/audit/recent?n=10&agent_id=${agent.id}` : null);
  const [budget] = useApi(agent ? `/api/budget/agents/${agent.id}` : null);
  if (!agent) return null;
  const turns = recent && recent.entries || null;
  const cost24 = budget && budget.daily && budget.daily.spend != null ? Number(budget.daily.spend) : agent.cost;
  return /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "drawer-back " + (agent ? "open" : ""), onClick: onClose }), /* @__PURE__ */ React.createElement("aside", { className: "drawer " + (agent ? "open" : "") }, /* @__PURE__ */ React.createElement("div", { className: "drawer-head" }, /* @__PURE__ */ React.createElement(Avatar, { agent, size: "lg" }), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2 } }, /* @__PURE__ */ React.createElement("div", { style: { fontFamily: "var(--ff-mono)", fontSize: 15 } }, agent.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, agent.id, " \xB7 ", agent.group)), /* @__PURE__ */ React.createElement("div", { style: { marginLeft: "auto" }, className: "row gap-6" }, /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null)))), /* @__PURE__ */ React.createElement("div", { className: "tabs", style: { margin: "0 16px" } }, /* @__PURE__ */ React.createElement("button", { className: tab === "info" ? "on" : "", onClick: () => setTab("info") }, "Info"), /* @__PURE__ */ React.createElement("button", { className: tab === "config" ? "on" : "", onClick: () => setTab("config") }, "Config"), /* @__PURE__ */ React.createElement("button", { className: tab === "identity" ? "on" : "", onClick: () => setTab("identity") }, "Identity"), /* @__PURE__ */ React.createElement("button", { className: tab === "activity" ? "on" : "", onClick: () => setTab("activity") }, "Activity")), /* @__PURE__ */ React.createElement("div", { className: "drawer-body" }, tab === "info" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mb-12" }, /* @__PURE__ */ React.createElement(StateBadge, { state: agent.state }), /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, agent.model), /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, agent.provider)), /* @__PURE__ */ React.createElement("div", { className: "kv mb-16" }, /* @__PURE__ */ React.createElement("dt", null, "messages"), /* @__PURE__ */ React.createElement("dd", null, (agent.messages || 0).toLocaleString()), /* @__PURE__ */ React.createElement("dt", null, "cost 24h"), /* @__PURE__ */ React.createElement("dd", null, "$", (cost24 || 0).toFixed(2)), /* @__PURE__ */ React.createElement("dt", null, "last"), /* @__PURE__ */ React.createElement("dd", null, agent.updated, " ago"), /* @__PURE__ */ React.createElement("dt", null, "circuit"), /* @__PURE__ */ React.createElement("dd", { style: { color: agent.state === "error" ? "var(--crimson)" : "var(--live)" } }, agent.state === "error" ? "OPEN" : "CLOSED")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Description"), /* @__PURE__ */ React.createElement("div", { className: "codebox mb-16", style: { whiteSpace: "pre-wrap" } }, detail && detail.description ? detail.description : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "System prompt"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 200 } }, detail && detail.model && detail.model.system_prompt ? detail.model.system_prompt : "(loading or no system prompt)")), tab === "config" && detail && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement(AgentConfigForm, { agent, detail, onSaved: refreshDetail }), /* @__PURE__ */ React.createElement("div", { className: "divider" }), /* @__PURE__ */ React.createElement(AgentKvEditor, { agent })), tab === "config" && !detail && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), tab === "identity" && /* @__PURE__ */ React.createElement(AgentIdentityForm, { agent, detail, onSaved: refreshDetail }), tab === "activity" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement(AgentActivityCharts, { agent, budget, turns }), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8 mt-16", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Recent turns"), /* @__PURE__ */ React.createElement("div", { className: "col gap-6" }, !turns && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "6px 8px" } }, "loading audit\u2026"), turns && turns.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5, padding: "6px 8px" } }, "no audit entries for this agent yet."), turns && turns.map((r, i) => /* @__PURE__ */ React.createElement("div", { key: r.hash || r.seq || i, className: "row", style: { padding: "6px 8px", borderRadius: 6, background: "var(--bg-2)" } }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, width: 70 } }, formatTime(r.timestamp)), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12, flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, r.action), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, marginLeft: 8, maxWidth: 160, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, r.detail || r.outcome || ""))))))));
}
function AgentActivityCharts({ agent, budget, turns }) {
  const [metricsResp] = usePolling(agent ? `/api/agents/${agent.id}/metrics` : null, 5e3);
  const metrics = metricsResp || {};
  const [history, setHistory] = React.useState([]);
  React.useEffect(() => {
    if (!metricsResp || metricsResp.total_tokens == null) return;
    setHistory((prev) => {
      const next = [...prev, {
        t: Date.now(),
        tokens: Number(metricsResp.total_tokens || 0),
        msgs: Number(metricsResp.message_count || 0)
      }];
      return next.length > 60 ? next.slice(next.length - 60) : next;
    });
  }, [metricsResp && metricsResp.total_tokens, metricsResp && metricsResp.message_count]);
  React.useEffect(() => {
    setHistory([]);
  }, [agent && agent.id]);
  const deltas = React.useMemo(() => {
    if (history.length < 2) return [];
    const out = [];
    for (let i = 1; i < history.length; i++) {
      out.push({
        t: history[i].t,
        tokens: Math.max(0, history[i].tokens - history[i - 1].tokens),
        msgs: Math.max(0, history[i].msgs - history[i - 1].msgs)
      });
    }
    return out;
  }, [history]);
  const hourly = budget && budget.hourly ? budget.hourly : { spend: 0, limit: 0 };
  const daily = budget && budget.daily ? budget.daily : { spend: 0, limit: 0 };
  const monthly = budget && budget.monthly ? budget.monthly : { spend: 0, limit: 0 };
  const actionCounts = React.useMemo(() => {
    if (!Array.isArray(turns)) return [];
    const map = /* @__PURE__ */ new Map();
    for (const t of turns) {
      const k = t.action || "(unknown)";
      map.set(k, (map.get(k) || 0) + 1);
    }
    return [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  }, [turns]);
  const maxCount = Math.max(1, ...actionCounts.map(([, n]) => n));
  const BudgetBar = ({ label, info }) => {
    const spend = Number(info.spend || 0);
    const limit = Number(info.limit || 0);
    const pct = limit > 0 ? Math.min(100, Math.round(spend / limit * 100)) : 0;
    const danger = pct >= 80;
    return /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mb-4" }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5, width: 70, letterSpacing: ".08em", textTransform: "uppercase" } }, label), /* @__PURE__ */ React.createElement("span", { className: "bar", style: { flex: 1, height: 8, background: "var(--bg-2)", border: "1px solid var(--border)", borderRadius: 3, position: "relative", overflow: "hidden" } }, /* @__PURE__ */ React.createElement("span", { style: {
      display: "block",
      height: "100%",
      width: `${pct}%`,
      background: danger ? "linear-gradient(90deg, var(--crimson), oklch(0.6 0.15 25))" : "linear-gradient(90deg, var(--rust), var(--rust-2))"
    } })), /* @__PURE__ */ React.createElement("span", { className: "mono nums", style: { fontSize: 11, width: 120, textAlign: "right" } }, "$", spend.toFixed(2), " / ", limit > 0 ? `$${limit.toFixed(2)}` : "\u2014"));
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Budget usage"), /* @__PURE__ */ React.createElement("div", { className: "card", style: { padding: 10, marginBottom: 12 } }, /* @__PURE__ */ React.createElement(BudgetBar, { label: "hour", info: hourly }), /* @__PURE__ */ React.createElement(BudgetBar, { label: "day", info: daily }), /* @__PURE__ */ React.createElement(BudgetBar, { label: "month", info: monthly })), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Activity histogram ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6, fontSize: 10 } }, "(last ", turns ? turns.length : 0, " audit events)")), /* @__PURE__ */ React.createElement("div", { className: "card", style: { padding: 10 } }, actionCounts.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5 } }, "no audit events to plot."), actionCounts.map(([action, n]) => /* @__PURE__ */ React.createElement("div", { key: action, className: "row gap-8 mb-4" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11, width: 120, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, action), /* @__PURE__ */ React.createElement("span", { style: { flex: 1, height: 8, background: "var(--bg-2)", border: "1px solid var(--border)", borderRadius: 3, overflow: "hidden" } }, /* @__PURE__ */ React.createElement("span", { style: { display: "block", height: "100%", width: `${n / maxCount * 100}%`, background: "linear-gradient(90deg, var(--violet), oklch(0.55 0.12 295))" } })), /* @__PURE__ */ React.createElement("span", { className: "mono nums", style: { fontSize: 11, width: 30, textAlign: "right" } }, n)))), metrics && metrics.total_tokens != null && /* @__PURE__ */ React.createElement("div", { className: "kv mt-12", style: { fontSize: 12 } }, /* @__PURE__ */ React.createElement("dt", null, "total tokens"), /* @__PURE__ */ React.createElement("dd", null, Number(metrics.total_tokens || 0).toLocaleString()), /* @__PURE__ */ React.createElement("dt", null, "messages"), /* @__PURE__ */ React.createElement("dd", null, Number(metrics.message_count || 0).toLocaleString()), /* @__PURE__ */ React.createElement("dt", null, "last activity"), /* @__PURE__ */ React.createElement("dd", { className: "mono dim" }, metrics.last_activity ? relativeTime(metrics.last_activity) : "\u2014")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mt-12 mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Live throughput ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6, fontSize: 10 } }, "(per 5s tick \xB7 since drawer opened)")), /* @__PURE__ */ React.createElement("div", { className: "card", style: { padding: 10 } }, deltas.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5, padding: "6px 0" } }, "collecting samples\u2026"), deltas.length > 0 && /* @__PURE__ */ React.createElement(AgentLiveSpark, { deltas })));
}
function AgentLiveSpark({ deltas }) {
  if (!deltas || deltas.length === 0) return null;
  const W = 320, H = 56, PAD = 4;
  const tokens = deltas.map((d) => d.tokens);
  const msgs = deltas.map((d) => d.msgs);
  const maxTokens = Math.max(1, ...tokens);
  const maxMsgs = Math.max(1, ...msgs);
  const xStep = (W - 2 * PAD) / Math.max(1, deltas.length - 1);
  const tokenPath = deltas.map((d, i) => {
    const x = PAD + i * xStep;
    const y = H - PAD - d.tokens / maxTokens * (H - 2 * PAD);
    return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const tokenArea = `${tokenPath} L${(W - PAD).toFixed(1)},${H - PAD} L${PAD},${H - PAD} Z`;
  const msgPath = deltas.map((d, i) => {
    const x = PAD + i * xStep;
    const y = H - PAD - d.msgs / maxMsgs * (H - 2 * PAD);
    return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`;
  }).join(" ");
  const totalTokens = tokens.reduce((s, x) => s + x, 0);
  const totalMsgs = msgs.reduce((s, x) => s + x, 0);
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("svg", { width: "100%", height: H, viewBox: `0 0 ${W} ${H}`, preserveAspectRatio: "none" }, /* @__PURE__ */ React.createElement("path", { d: tokenArea, fill: "oklch(0.665 0.165 50 / .2)" }), /* @__PURE__ */ React.createElement("path", { d: tokenPath, fill: "none", stroke: "var(--rust)", strokeWidth: "1.5" }), /* @__PURE__ */ React.createElement("path", { d: msgPath, fill: "none", stroke: "var(--violet)", strokeWidth: "1", strokeDasharray: "2 2" })), /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mt-4 dim mono", style: { fontSize: 10.5 } }, /* @__PURE__ */ React.createElement("span", null, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, "\u25CF"), " tokens \xB7 ", totalTokens.toLocaleString(), " (", maxTokens, "/tick peak)"), /* @__PURE__ */ React.createElement("span", null, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--violet)" } }, "\u25CF"), " msgs \xB7 ", totalMsgs, " (", maxMsgs, "/tick peak)"), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" } }, deltas.length, " samples")));
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
    model: model.model || agent.model
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
  const [modelsResp] = useApi("/api/models");
  const allModels = modelsResp && modelsResp.models || [];
  const provider = model && model.provider || agent.provider;
  const modelOptions = allModels.filter((m) => !provider || m.provider === provider).sort((a, b) => (a.display_name || a.id).localeCompare(b.display_name || b.id));
  const diff = React.useMemo(() => {
    const rows = [];
    const push = (label, oldV, newV) => {
      if (String(oldV != null ? oldV : "") !== String(newV != null ? newV : "")) rows.push({ label, oldV, newV });
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
    if (diff.length === 0) {
      setOk(true);
      return;
    }
    setBusy(true);
    setErr(null);
    setOk(false);
    try {
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
          body: JSON.stringify(patch)
        });
      }
      const modelChanged = diff.some((r) => r.label === "model");
      if (modelChanged && modelName) {
        await rhFetch(`/api/agents/${agent.id}/model`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ model: modelName })
        });
      }
      toastOk(`Saved ${diff.length} change${diff.length === 1 ? "" : "s"} to ${agent.name}`);
      setOk(true);
      onSaved();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Group"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: group, onChange: (e) => setGroup(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Description"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 60 }, value: description, onChange: (e) => setDescription(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "System prompt"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", style: { minHeight: 120 }, value: systemPrompt, onChange: (e) => setSystemPrompt(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Model", modelOptions.length === 0 && modelsResp && /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6, fontSize: 10 } }, "(catalog empty for provider ", provider, ")")), modelOptions.length > 0 ? /* @__PURE__ */ React.createElement("select", { className: "t-select", value: modelName, onChange: (e) => setModelName(e.target.value) }, !modelOptions.some((m) => m.id === modelName) && modelName && /* @__PURE__ */ React.createElement("option", { value: modelName }, modelName, " (custom)"), modelOptions.map((m) => /* @__PURE__ */ React.createElement("option", { key: m.id, value: m.id }, m.display_name || m.id, m.tier ? ` \xB7 ${m.tier}` : "", m.available === false ? " \xB7 not configured" : ""))) : /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: modelName, onChange: (e) => setModelName(e.target.value) })), /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Temperature (0\u20132)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", step: "0.05", min: "0", max: "2", value: temperature, onChange: (e) => setTemperature(e.target.value) })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Max tokens"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: "1", value: maxTokens, onChange: (e) => setMaxTokens(e.target.value) }))), /* @__PURE__ */ React.createElement("div", { className: "t-row" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Thinking enabled"), /* @__PURE__ */ React.createElement("div", { className: "switch " + (thinkingEnabled ? "on" : ""), onClick: () => setThinkingEnabled((v) => !v) })), diff.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "config-diff" }, /* @__PURE__ */ React.createElement("div", { className: "config-diff-head" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--fg-3)" } }, "Pending changes \xB7 ", diff.length)), diff.map((r) => {
    var _a, _b;
    return /* @__PURE__ */ React.createElement("div", { key: r.label, className: "config-diff-row" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5, color: "var(--fg-2)", width: 140 } }, r.label), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11, color: "var(--crimson)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, String((_a = r.oldV) != null ? _a : "").slice(0, 80) || "\u2014"), /* @__PURE__ */ React.createElement("span", { style: { color: "var(--fg-4)" } }, "\u2192"), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11, color: "var(--live)", flex: 1, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, String((_b = r.newV) != null ? _b : "").slice(0, 80) || "\u2014"));
  })), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), ok && diff.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.74 0.135 150 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "UP TO DATE"), /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11.5 } }, "No changes to save.")), /* @__PURE__ */ React.createElement("div", { className: "row", style: { justifyContent: "flex-end", marginTop: 8 } }, /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : diff.length === 0 ? "Up to date" : `Save ${diff.length} change${diff.length === 1 ? "" : "s"}`)));
}
function AgentKvEditor({ agent }) {
  const path = agent ? `/api/memory/agents/${agent.id}/kv` : null;
  const [resp, fetchErr, refresh] = useApi(path);
  const pairs = resp && resp.kv || [];
  const [newKey, setNewKey] = useState("");
  const [newValue, setNewValue] = useState("");
  const [editingKey, setEditingKey] = useState(null);
  const [editDraft, setEditDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const parseValue = (s) => {
    const trimmed = s.trim();
    if (trimmed === "") return "";
    try {
      return JSON.parse(trimmed);
    } catch (_) {
      return s;
    }
  };
  const save = async (k, v) => {
    if (!k.trim()) {
      toastErr("Key required");
      return;
    }
    setBusy(true);
    try {
      await rhFetch(`/api/memory/agents/${agent.id}/kv/${encodeURIComponent(k.trim())}`, {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ value: parseValue(v) })
      });
      toastOk(`Saved ${k}`);
      setNewKey("");
      setNewValue("");
      setEditingKey(null);
      setEditDraft("");
      refresh();
    } catch (e) {
      toastErr(`save failed: ${e.message || e}`);
    } finally {
      setBusy(false);
    }
  };
  const remove = async (k) => {
    if (!await confirmDialog({ title: "Delete key", message: `Delete kv key "${k}"?`, danger: true, confirmLabel: "Delete" })) return;
    setBusy(true);
    try {
      await rhFetch(`/api/memory/agents/${agent.id}/kv/${encodeURIComponent(k)}`, { method: "DELETE" });
      toastOk(`Deleted ${k}`);
      refresh();
    } catch (e) {
      toastErr(`delete failed: ${e.message || e}`);
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-8 mt-12" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Environment / KV store", /* @__PURE__ */ React.createElement(Tip, null, "Key/value pairs stored per-agent and surfaced to skills as environment variables. Values are JSON-typed: numbers, strings, objects, arrays. Plain text is auto-quoted.")), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, color: "var(--crimson)" } }, fetchErr), !resp && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5 } }, "loading\u2026"), resp && pairs.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5 } }, "No keys yet \u2014 add one below."), /* @__PURE__ */ React.createElement("div", { className: "col gap-4" }, pairs.map((p) => {
    const v = p.value;
    const display = typeof v === "string" ? v : JSON.stringify(v);
    const isEditing = editingKey === p.key;
    return /* @__PURE__ */ React.createElement("div", { key: p.key, className: "row gap-8", style: { padding: "6px 8px", background: "var(--bg-2)", borderRadius: 6, alignItems: "flex-start" } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5, width: 120, paddingTop: 4 } }, p.key), isEditing ? /* @__PURE__ */ React.createElement(
      "textarea",
      {
        className: "modal-field modal-textarea",
        style: { flex: 1, minHeight: 36, fontSize: 11.5, fontFamily: "var(--ff-mono)" },
        value: editDraft,
        onChange: (e) => setEditDraft(e.target.value),
        autoFocus: true
      }
    ) : /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { flex: 1, fontSize: 11.5, paddingTop: 4, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, display), isEditing ? /* @__PURE__ */ React.createElement("span", { className: "row gap-4" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => save(p.key, editDraft), disabled: busy }, "save"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => {
      setEditingKey(null);
      setEditDraft("");
    } }, "cancel")) : /* @__PURE__ */ React.createElement("span", { className: "row gap-4" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => {
      setEditingKey(p.key);
      setEditDraft(typeof v === "string" ? v : JSON.stringify(v, null, 2));
    } }, "edit"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => remove(p.key) }, /* @__PURE__ */ React.createElement(I.close, null))));
  })), /* @__PURE__ */ React.createElement("div", { className: "row gap-6 mt-8" }, /* @__PURE__ */ React.createElement("input", { className: "modal-field", style: { flex: 0, width: 140 }, placeholder: "new key", value: newKey, onChange: (e) => setNewKey(e.target.value) }), /* @__PURE__ */ React.createElement("input", { className: "modal-field", style: { flex: 1 }, placeholder: "value (JSON or plain text)", value: newValue, onChange: (e) => setNewValue(e.target.value) }), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => save(newKey, newValue), disabled: busy || !newKey.trim() }, /* @__PURE__ */ React.createElement(I.plus, null), " Add")));
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
  return /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Emoji"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: emoji, maxLength: 4, onChange: (e) => setEmoji(e.target.value), placeholder: "\u{1F980}" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Color (hex, optional)"), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, /* @__PURE__ */ React.createElement("input", { className: "modal-field", style: { flex: 1 }, value: color, onChange: (e) => setColor(e.target.value), placeholder: "#d4541b" }), /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "color",
      value: /^#[0-9a-fA-F]{6}$/.test(color) ? color : "#d4541b",
      onChange: (e) => setColor(e.target.value),
      style: { width: 32, height: 32, padding: 0, border: "1px solid var(--border)", borderRadius: 6, background: "var(--bg-2)" },
      title: "Pick color"
    }
  )), /* @__PURE__ */ React.createElement("div", { className: "row gap-4 mt-4" }, ["#d4541b", "#e0a52e", "#5d9c4d", "#3b82c0", "#9b6cd1", "#c44a73", "#6b6f74"].map((swatch) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: swatch,
      type: "button",
      onClick: () => setColor(swatch),
      className: "swatch",
      style: { background: swatch, outline: color === swatch ? "2px solid var(--rust)" : "none" },
      title: swatch
    }
  ))))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Avatar URL (http/https/data)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: avatarUrl, onChange: (e) => setAvatarUrl(e.target.value), placeholder: "https://\u2026" })), /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Archetype"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: archetype, onChange: (e) => setArchetype(e.target.value), placeholder: "sage / sentinel / artisan" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Vibe"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: vibe, onChange: (e) => setVibe(e.target.value), placeholder: "calm / sharp / playful" }))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Greeting style"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: greetingStyle, onChange: (e) => setGreetingStyle(e.target.value), placeholder: "terse, formal, etc." })), err && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err)), ok && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.74 0.135 150 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "SAVED")), /* @__PURE__ */ React.createElement("div", { className: "row", style: { justifyContent: "flex-end", marginTop: 8 } }, /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : "Save identity")));
}
function ChatPage() {
  const [agentsResp] = usePolling("/api/agents?limit=200", 2e4);
  const agents = agentsResp && agentsResp.agents ? agentsResp.agents.map(normalizeAgent) : [];
  const [activeId, setActiveId] = useState(null);
  const active = agents.find((a) => a.id === activeId) || agents[0] || null;
  const [session, sessionErr, refreshSession] = useApi(active ? `/api/agents/${active.id}/session` : null);
  const [budget] = useApi(active ? `/api/budget/agents/${active.id}` : null);
  const [toolsResp] = useApi("/api/tools");
  const tools = Array.isArray(toolsResp) ? toolsResp : toolsResp && toolsResp.tools;
  const [pendingMessages, setPendingMessages] = useState([]);
  const pendingKeyRef = React.useRef(0);
  const nextPendingKey = () => `p${++pendingKeyRef.current}`;
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
        setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "assistant", content: msg.content || streamingText, _local: true }]));
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
        setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "assistant", content: `[error] ${msg.content || msg.error || "unknown"}`, _local: true, error: true }]));
        setStreamingText("");
        setStreamingTools([]);
        setSending(false);
        break;
      case "command_result":
        setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "assistant", content: msg.message || msg.content || "ok", _local: true, command: true }]));
        setSending(false);
        break;
      default:
        break;
    }
  }, [streamingText, refreshSession]);
  const ws = useAgentWs(active && active.id, onWs);
  const items = coalesceToolTraces(
    sessionToItems(session && session.messages).concat(pendingMessages)
  );
  const send = async () => {
    const text = typed.trim();
    if (!text || sending) return;
    setTyped("");
    setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "user", content: text, _local: true }]));
    setStreamingText("");
    setStreamingTools([]);
    setSending(true);
    if (text.startsWith("/") && ws.connected) {
      const rest = text.slice(1).trim();
      const sp = rest.indexOf(" ");
      const command = sp < 0 ? rest : rest.slice(0, sp);
      const args = sp < 0 ? "" : rest.slice(sp + 1);
      if (ws.sendCommand(command, args)) return;
    }
    if (ws.connected && ws.send(text)) return;
    try {
      const resp = await rhFetch(`/api/agents/${active.id}/message`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ message: text })
      });
      if (resp && resp.response) {
        setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "assistant", content: resp.response, _local: true }]));
      }
    } catch (e) {
      setPendingMessages((prev) => prev.concat([{ _key: nextPendingKey(), role: "assistant", content: `[error] ${e.message || e}`, _local: true, error: true }]));
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
      const next = side === "left" ? Math.max(180, Math.min(560, startWidth + delta)) : Math.max(220, Math.min(560, startWidth - delta));
      wrap.style.setProperty(`--chat-${side}`, `${next}px`);
    };
    const onUp = () => {
      document.body.style.cursor = "";
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
      try {
        localStorage.setItem(
          `rh.panel.chat${side === "left" ? "Left" : "Right"}`,
          wrap.style.getPropertyValue(`--chat-${side}`)
        );
      } catch (_) {
      }
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  };
  const [contextCollapsed, setContextCollapsedState] = useState(() => {
    try {
      const v = localStorage.getItem("rh.panel.chatContextCollapsed");
      return v == null ? true : v === "1";
    } catch (_) {
      return true;
    }
  });
  const setContextCollapsed = (v) => {
    setContextCollapsedState(v);
    try {
      localStorage.setItem("rh.panel.chatContextCollapsed", v ? "1" : "0");
    } catch (_) {
    }
  };
  React.useEffect(() => {
    const wrap = chatWrapRef.current;
    if (!wrap) return;
    try {
      const l = localStorage.getItem("rh.panel.chatLeft");
      const r = localStorage.getItem("rh.panel.chatRight");
      if (l) wrap.style.setProperty("--chat-left", l);
      if (r && !contextCollapsed) wrap.style.setProperty("--chat-right", r);
    } catch (_) {
    }
  }, []);
  React.useEffect(() => {
    const wrap = chatWrapRef.current;
    if (!wrap) return;
    if (contextCollapsed) {
      wrap.style.setProperty("--chat-right", "32px");
    } else {
      let r = "340px";
      try {
        r = localStorage.getItem("rh.panel.chatRight") || "340px";
      } catch (_) {
      }
      wrap.style.setProperty("--chat-right", r);
    }
  }, [contextCollapsed]);
  if (!active) {
    return /* @__PURE__ */ React.createElement("div", { className: "chat-wrap", ref: chatWrapRef }, /* @__PURE__ */ React.createElement("div", { className: "chat-list" }, /* @__PURE__ */ React.createElement("div", { className: "chat-list-head row between" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Sessions")), /* @__PURE__ */ React.createElement("div", { className: "chat-list-body" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px 14px", fontSize: 12, textAlign: "center" } }, agentsResp ? "No agents yet. Spawn one from the Agents page." : "loading agents\u2026"))), /* @__PURE__ */ React.createElement("div", { "aria-hidden": true, style: { width: 0 } }), /* @__PURE__ */ React.createElement("div", { className: "chat-panel" }), /* @__PURE__ */ React.createElement("div", { "aria-hidden": true, style: { width: 0 } }), /* @__PURE__ */ React.createElement("div", { className: "chat-side" }));
  }
  return /* @__PURE__ */ React.createElement("div", { className: "chat-wrap", ref: chatWrapRef }, /* @__PURE__ */ React.createElement("div", { className: "chat-list" }, /* @__PURE__ */ React.createElement("div", { className: "chat-list-head row between" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Sessions"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn" }, /* @__PURE__ */ React.createElement(I.plus, null))), /* @__PURE__ */ React.createElement("div", { className: "chat-list-body" }, agents.slice(0, 16).map((a) => /* @__PURE__ */ React.createElement("div", { key: a.id, className: "chat-list-item " + (active.id === a.id ? "active" : ""), onClick: () => setActiveId(a.id) }, /* @__PURE__ */ React.createElement(Avatar, { agent: a }), /* @__PURE__ */ React.createElement("div", { style: { flex: 1, minWidth: 0 } }, /* @__PURE__ */ React.createElement("div", { className: "row between" }, /* @__PURE__ */ React.createElement("span", { className: "name" }, a.name), /* @__PURE__ */ React.createElement("span", { className: "time" }, a.updated)), /* @__PURE__ */ React.createElement("div", { className: "last" }, a.last)))))), /* @__PURE__ */ React.createElement("div", { className: "chat-resize", onMouseDown: startResize("left"), "aria-hidden": true }), /* @__PURE__ */ React.createElement("div", { className: "chat-panel" }, /* @__PURE__ */ React.createElement("div", { className: "chat-head" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 2 } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, active.name), /* @__PURE__ */ React.createElement(StateBadge, { state: active.state }), /* @__PURE__ */ React.createElement("span", { className: "badge " + (ws.connected ? "live" : "idle"), title: ws.connected ? "WebSocket streaming" : "WebSocket disconnected (HTTP fallback)" }, ws.connected ? "WS" : "HTTP")), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, active.model, " \xB7 ", active.provider, " \xB7 session ", session && session.session_id ? `#${String(session.session_id).slice(0, 4)}` : "\u2014")), /* @__PURE__ */ React.createElement("div", { className: "actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: refreshSession, title: "Refresh session" }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn sm",
      onClick: () => {
        if (!ws.connected) {
          toastErr("WebSocket not connected");
          return;
        }
        ws.sendCommand("retry", "");
        toast("Regenerating last response\u2026");
      },
      title: "Regenerate the last assistant response",
      disabled: !ws.connected || sending
    },
    /* @__PURE__ */ React.createElement(I.refresh, null),
    " Regenerate"
  ), /* @__PURE__ */ React.createElement("button", { className: "btn sm" }, /* @__PURE__ */ React.createElement(I.download, null), " Export"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn" }, /* @__PURE__ */ React.createElement(I.more, null)))), /* @__PURE__ */ React.createElement("div", { className: "chat-stream", ref: streamRef }, !session && !sessionErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { padding: "24px 6px", fontSize: 12 } }, "Loading session\u2026"), sessionErr && pendingMessages.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { padding: "24px 6px", fontSize: 12, color: "var(--crimson)" } }, "Session load failed: ", sessionErr), items.map((it, i) => {
    const k = it._key || `i${i}`;
    if (it.role === "user") return /* @__PURE__ */ React.createElement("div", { key: k, className: "msg user" }, /* @__PURE__ */ React.createElement(Avatar, { agent: { name: "you", hue: 22, emoji: "Y" } }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who", style: { textAlign: "right" } }, "operator \xB7 just now"), /* @__PURE__ */ React.createElement("div", { className: "bubble", style: { whiteSpace: "pre-wrap" } }, it.content)));
    if (it.role === "tool") return /* @__PURE__ */ React.createElement(ToolTraceCard, { key: k, tool: it });
    return /* @__PURE__ */ React.createElement("div", { key: k, className: "msg" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who" }, active.name), /* @__PURE__ */ React.createElement("div", { className: "bubble", style: { color: it.error ? "var(--crimson)" : void 0 } }, it.error || it.command ? it.content : renderMarkdown(it.content))));
  }), streamingTools.map((t, i) => /* @__PURE__ */ React.createElement(ToolTraceCard, { key: `stool-${i}`, tool: t })), (sending || streamingText) && /* @__PURE__ */ React.createElement("div", { className: "msg" }, /* @__PURE__ */ React.createElement(Avatar, { agent: active }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "who" }, active.name, " \xB7 streaming"), /* @__PURE__ */ React.createElement("div", { className: "bubble" }, streamingText ? renderMarkdown(streamingText) : null, /* @__PURE__ */ React.createElement("span", { className: "cursor" }))))), /* @__PURE__ */ React.createElement(
    ChatInput,
    {
      typed,
      setTyped,
      sending,
      send,
      active,
      ws
    }
  )), !contextCollapsed && /* @__PURE__ */ React.createElement("div", { className: "chat-resize", onMouseDown: startResize("right"), "aria-hidden": true }), contextCollapsed && /* @__PURE__ */ React.createElement("div", { "aria-hidden": true, style: { width: 0 } }), contextCollapsed ? /* @__PURE__ */ React.createElement(
    "div",
    {
      className: "chat-side",
      style: {
        display: "flex",
        alignItems: "flex-start",
        justifyContent: "center",
        padding: "8px 4px",
        cursor: "pointer",
        background: "var(--surface)",
        borderLeft: "1px solid var(--border)"
      },
      onClick: () => setContextCollapsed(false),
      title: "Show run context"
    },
    /* @__PURE__ */ React.createElement(
      "span",
      {
        className: "mono dim",
        style: {
          fontSize: 10,
          writingMode: "vertical-rl",
          transform: "rotate(180deg)",
          letterSpacing: ".2em",
          textTransform: "uppercase",
          userSelect: "none"
        }
      },
      "\u2039 context"
    )
  ) : /* @__PURE__ */ React.createElement("div", { className: "chat-side" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-8" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Run context"), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "kbd",
      style: { cursor: "pointer" },
      title: "Hide run context",
      onClick: () => setContextCollapsed(true)
    },
    "\u203A"
  )), /* @__PURE__ */ React.createElement("div", { className: "kv mb-16" }, /* @__PURE__ */ React.createElement("dt", null, "session"), /* @__PURE__ */ React.createElement("dd", null, session ? String(session.session_id || "\u2014").slice(0, 12) : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "messages"), /* @__PURE__ */ React.createElement("dd", null, session && session.messages ? session.messages.length : 0), /* @__PURE__ */ React.createElement("dt", null, "pressure"), /* @__PURE__ */ React.createElement("dd", { style: { color: "var(--live)" } }, session && session.context_pressure || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "budget \xB7 day"), /* @__PURE__ */ React.createElement("dd", null, budget && budget.daily ? `$${Number(budget.daily.spend || 0).toFixed(2)} / $${Number(budget.daily.limit || 0).toFixed(2)}` : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "budget \xB7 hour"), /* @__PURE__ */ React.createElement("dd", null, budget && budget.hourly ? `$${Number(budget.hourly.spend || 0).toFixed(2)} / $${Number(budget.hourly.limit || 0).toFixed(2)}` : "\u2014")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Memory \xB7 session"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 120 } }, `session_id = ${session ? session.session_id || "\u2014" : "loading\u2026"}
agent_id   = ${active.id}
messages   = ${session && session.messages ? session.messages.length : 0}
model      = ${active.model}
provider   = ${active.provider}`), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Allowed tools"), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 200, overflow: "auto" } }, (tools || []).slice(0, 16).map((t) => /* @__PURE__ */ React.createElement("div", { key: t.name, className: "row between", style: { padding: "4px 8px", background: "var(--bg-2)", borderRadius: 5 } }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 11.5 } }, t.name), /* @__PURE__ */ React.createElement("span", { className: "badge live", style: { padding: "1px 5px" } }, "ok"))), !tools && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, padding: "4px 8px" } }, "loading\u2026"))));
}
const CHAT_SLASH_COMMANDS = [
  { cmd: "/workflow run", help: "Run a workflow with the next args as input", usage: "/workflow run <name> [input\u2026]" },
  { cmd: "/workflow list", help: "List all workflows", usage: "/workflow list" },
  { cmd: "/tool", help: "Show the agent's allowed tool list" },
  { cmd: "/memory recall", help: "Recall a memory by key or substring", usage: "/memory recall <key>" },
  { cmd: "/memory remember", help: "Store a memory under a key", usage: "/memory remember <key> <value\u2026>" },
  { cmd: "/memory forget", help: "Drop a memory by key", usage: "/memory forget <key>" },
  { cmd: "/model", help: "Switch the agent's model", usage: "/model <model-id>" },
  { cmd: "/temp", help: "Set sampling temperature (0\u20132)", usage: "/temp 0.7" },
  { cmd: "/system", help: "Update the system prompt", usage: "/system <new prompt>" },
  { cmd: "/thinking", help: "Toggle extended thinking" },
  { cmd: "/reset", help: "Reset the conversation" },
  { cmd: "/help", help: "Show server-side command help" }
];
function ChatInput({ typed, setTyped, sending, send, active, ws }) {
  const inputRef = React.useRef(null);
  const open = typed.startsWith("/") && !typed.includes("\n");
  const candidates = React.useMemo(() => {
    if (!open) return [];
    const ql = typed.toLowerCase();
    return CHAT_SLASH_COMMANDS.filter((c) => c.cmd.toLowerCase().startsWith(ql) || ql.startsWith(c.cmd.toLowerCase() + " ")).sort((a, b) => {
      const ap = ql.startsWith(a.cmd.toLowerCase()) ? 0 : 1;
      const bp = ql.startsWith(b.cmd.toLowerCase()) ? 0 : 1;
      return ap - bp;
    });
  }, [typed, open]);
  const [highlight, setHighlight] = React.useState(0);
  React.useEffect(() => {
    setHighlight(0);
  }, [candidates.length]);
  const pick = (c) => {
    const next = c.usage ? c.cmd + " " : c.cmd;
    setTyped(next);
    setTimeout(() => inputRef.current && inputRef.current.focus(), 0);
  };
  const onKeyDown = (e) => {
    if (open && candidates.length > 0) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setHighlight((h) => Math.min(candidates.length - 1, h + 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setHighlight((h) => Math.max(0, h - 1));
        return;
      }
      if (e.key === "Tab") {
        e.preventDefault();
        pick(candidates[highlight]);
        return;
      }
    }
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "chat-input", style: { position: "relative" } }, open && candidates.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "slash-popup" }, /* @__PURE__ */ React.createElement("div", { className: "slash-popup-head" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Slash commands"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, "Tab / Enter to insert")), candidates.map((c, i) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: c.cmd,
      className: "slash-popup-row" + (i === highlight ? " active" : ""),
      onMouseEnter: () => setHighlight(i),
      onMouseDown: (e) => e.preventDefault(),
      onClick: () => pick(c)
    },
    /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12.5, color: "var(--rust)", width: 170 } }, c.cmd),
    /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11.5, flex: 1 } }, c.help),
    c.usage && /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, c.usage)
  ))), /* @__PURE__ */ React.createElement("div", { className: "field" }, /* @__PURE__ */ React.createElement(I.zap, null), /* @__PURE__ */ React.createElement(
    "input",
    {
      ref: inputRef,
      placeholder: `Message ${active.name}\u2026  (type / for commands)`,
      value: typed,
      disabled: sending,
      onChange: (e) => setTyped(e.target.value),
      onKeyDown
    }
  ), /* @__PURE__ */ React.createElement("span", { className: "kbd" }, "\u21B5 send")), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: send, disabled: sending || !typed.trim() }, /* @__PURE__ */ React.createElement(I.send, null), " ", sending ? "\u2026" : "Send"));
}
function sessionToItems(messages) {
  if (!Array.isArray(messages)) return [];
  const items = [];
  let counter = 0;
  for (const m of messages) {
    const role = (m.role || "").toLowerCase();
    const baseId = m.id || m.message_id || m.seq != null ? `m${m.id || m.message_id || m.seq}` : `i${counter}`;
    if (Array.isArray(m.tools)) {
      for (let ti = 0; ti < m.tools.length; ti++) {
        const t = m.tools[ti];
        items.push({
          _key: `${baseId}-t${ti}`,
          role: "tool",
          name: t.name,
          input: t.input,
          result: t.result,
          is_error: !!t.is_error,
          running: !!t.running
        });
      }
    }
    if (m.content) {
      items.push({
        _key: `${baseId}-c`,
        role: role === "user" ? "user" : "assistant",
        content: m.content
      });
    }
    counter++;
  }
  return items;
}
function ToolTraceCard({ tool }) {
  const [open, setOpen] = useState(false);
  const t = tool || {};
  const running = !!t.running;
  const isError = !!t.is_error;
  const hasDetail = !!t.input || !!t.result;
  const argStr = String(typeof t.input === "string" ? t.input : t.input ? JSON.stringify(t.input) : "").slice(0, 240);
  const resultStr = t.result ? String(t.result).slice(0, 2e3) : "";
  const elapsed = t.elapsed || (t.duration_ms != null ? `${(Number(t.duration_ms) / 1e3).toFixed(2)}s` : null);
  const status = running ? "\u2026" : isError ? "err" : elapsed || "ok";
  return /* @__PURE__ */ React.createElement("div", { className: "tool-trace " + (running ? "" : isError ? "fail" : "done") }, /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "tool-trace-head",
      onClick: () => hasDetail && setOpen((o) => !o),
      disabled: !hasDetail,
      title: hasDetail ? "Show full input + result" : "No detail captured"
    },
    /* @__PURE__ */ React.createElement("span", { style: { display: "inline-flex", width: 14, height: 14 } }, running ? /* @__PURE__ */ React.createElement(Spinner, null) : /* @__PURE__ */ React.createElement(I.check, null)),
    /* @__PURE__ */ React.createElement("span", { className: "tool-trace-label" }, running ? "\u2699" : isError ? "\u2717" : "\u2713", " ", /* @__PURE__ */ React.createElement("span", { className: "tool-name" }, t.name), argStr && /* @__PURE__ */ React.createElement("span", { className: "dim" }, " (", argStr.length > 80 ? argStr.slice(0, 80) + "\u2026" : argStr, ")"), t.count && t.count > 1 ? /* @__PURE__ */ React.createElement("span", { className: "badge plain", style: { marginLeft: 6 } }, "\xD7", t.count) : null),
    /* @__PURE__ */ React.createElement("span", { className: "elapsed" }, status),
    hasDetail && /* @__PURE__ */ React.createElement("span", { className: "tool-trace-caret" }, open ? "\u2212" : "+")
  ), open && hasDetail && /* @__PURE__ */ React.createElement("div", { className: "tool-trace-detail" }, t.input && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "tool-trace-detail-label" }, "Input"), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 140, marginBottom: 6 } }, typeof t.input === "string" ? t.input : JSON.stringify(t.input, null, 2))), t.result && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "tool-trace-detail-label" }, isError ? "Error" : "Result"), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 240, color: isError ? "var(--crimson)" : void 0 } }, resultStr))));
}
function coalesceToolTraces(items) {
  const out = [];
  for (const it of items) {
    const last = out[out.length - 1];
    if (it && it.role === "tool" && last && last.role === "tool" && last.name === it.name && !last.running && !it.running && !last.is_error && !it.is_error) {
      last.count = (last.count || 1) + 1;
      last.input = it.input;
      last.result = it.result;
      continue;
    }
    out.push(it && it.role === "tool" ? { ...it, count: 1 } : it);
  }
  return out;
}
const Spinner = () => /* @__PURE__ */ React.createElement("svg", { width: "14", height: "14", viewBox: "0 0 24 24", fill: "none" }, /* @__PURE__ */ React.createElement("circle", { cx: "12", cy: "12", r: "9", stroke: "currentColor", strokeWidth: "2.4", opacity: ".2" }), /* @__PURE__ */ React.createElement("path", { d: "M21 12a9 9 0 0 0-9-9", stroke: "currentColor", strokeWidth: "2.4", strokeLinecap: "round" }, /* @__PURE__ */ React.createElement("animateTransform", { attributeName: "transform", type: "rotate", from: "0 12 12", to: "360 12 12", dur: "0.9s", repeatCount: "indefinite" })));
function WorkflowsPage() {
  const [wfList, , refreshList] = usePolling("/api/workflows", 15e3);
  const workflows = Array.isArray(wfList) ? wfList : wfList && wfList.workflows || [];
  const [activeId, setActiveId] = useState(null);
  const active = workflows.find((w) => w.id === activeId) || workflows[0];
  const [showCreate, setShowCreate] = useState(false);
  const [showImport, setShowImport] = useState(false);
  const [showRunInput, setShowRunInput] = useState(false);
  const [inspectingRun, setInspectingRun] = useState(null);
  const [showEditYaml, setShowEditYaml] = useState(false);
  React.useEffect(() => {
    const onNew = (e) => {
      if (e.detail && e.detail.page === "workflows") setShowCreate(true);
    };
    const onRefresh = (e) => {
      if (e.detail && e.detail.page === "workflows") refreshList();
    };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [refreshList]);
  const [liveRun, setLiveRun] = useState(false);
  const [runsResp, , refreshRuns] = usePolling(
    active ? `/api/workflows/${encodeURIComponent(active.id)}/runs` : null,
    liveRun ? 1500 : 8e3
  );
  const runs = Array.isArray(runsResp) ? runsResp : runsResp && runsResp.runs || [];
  const runWith = async (input) => {
    if (!active) return;
    setShowRunInput(false);
    setLiveRun(true);
    const startedAt = Date.now();
    try {
      const r = await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}/run`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ input })
      });
      const dur = ((Date.now() - startedAt) / 1e3).toFixed(2);
      toastOk(`Run completed in ${dur}s`);
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
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Workflows ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", workflows.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Pipeline definitions persist across daemon restart \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/workflows"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refreshList }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowImport(true) }, /* @__PURE__ */ React.createElement(I.copy, null), " Import YAML"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " New workflow"))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "All workflows"), /* @__PURE__ */ React.createElement("span", { className: "mono" }, workflows.length)), /* @__PURE__ */ React.createElement("div", null, !wfList && Array.from({ length: 3 }).map((_, i) => /* @__PURE__ */ React.createElement("div", { key: `s-${i}`, style: { padding: "10px 14px", borderBottom: "1px solid var(--border)" } }, /* @__PURE__ */ React.createElement(Skel, { w: "50%", h: 10 }), /* @__PURE__ */ React.createElement("div", { style: { marginTop: 6 } }, /* @__PURE__ */ React.createElement(Skel, { w: "30%", h: 9 })))), wfList && workflows.length === 0 && /* @__PURE__ */ React.createElement("div", { style: { padding: "24px 14px", textAlign: "center" } }, /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 8 } }, "No workflows yet."), /* @__PURE__ */ React.createElement("button", { className: "btn primary sm", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Create your first workflow")), workflows.map((w) => {
    const stepCount = Array.isArray(w.steps) ? w.steps.length : w.steps || 0;
    const lastState = (w.last_run_state || "").toLowerCase();
    const dotCls = lastState === "completed" ? "live" : lastState === "failed" ? "err" : lastState === "running" ? "warn" : "idle";
    const dotTitle = w.last_run_state ? `Last run: ${w.last_run_state}${w.last_run_at ? ` (${relativeTime(w.last_run_at)})` : ""}` : "Never run";
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
      /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "dot " + dotCls, title: dotTitle }), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 12.5 } }, w.name || w.id), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 10.5 } }, stepCount, " steps", w.kind ? ` \xB7 ${w.kind}` : ""))),
      /* @__PURE__ */ React.createElement("div", { className: "col", style: { alignItems: "flex-end", gap: 2 } }, /* @__PURE__ */ React.createElement("span", { className: "mono nums", style: { fontSize: 12 } }, w.runs_24h != null ? w.runs_24h : "\u2014"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, "runs \xB7 24h"))
    );
  })))), /* @__PURE__ */ React.createElement("div", { className: "col-8 col" }, active && /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-title", style: { fontSize: 15, marginBottom: 2 } }, active.name || active.id), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "id=", active.id, active.description ? ` \xB7 ${active.description}` : "")), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, liveRun && /* @__PURE__ */ React.createElement("span", { className: "badge live", title: "A run is in flight \u2014 list polls at 1.5s" }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), "running"), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: refreshRuns }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => exportWorkflowYaml(active), title: "Download workflow as YAML" }, /* @__PURE__ */ React.createElement(I.download, null), " YAML"), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => setShowEditYaml(true), title: "Edit workflow YAML inline (delete + recreate)" }, /* @__PURE__ */ React.createElement(I.copy, null), " Edit YAML"), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn sm ghost",
      title: "Delete this workflow (run history is preserved)",
      onClick: async () => {
        const ok = await confirmDialog({
          title: "Delete workflow?",
          message: `Remove '${active.name || active.id}'? Past run history is kept, but the workflow definition will be gone.`,
          danger: true,
          confirmLabel: "Delete"
        });
        if (!ok) return;
        try {
          await rhFetch(`/api/workflows/${encodeURIComponent(active.id)}`, { method: "DELETE" });
          toastOk(`Deleted ${active.name || active.id}`);
          setActiveId(null);
          refreshList();
        } catch (err) {
          toastErr(`delete failed: ${err.message || err}`);
        }
      },
      style: { color: "var(--crimson)" }
    },
    /* @__PURE__ */ React.createElement(I.trash, null)
  ), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: runNow, disabled: liveRun }, /* @__PURE__ */ React.createElement(I.play, null), " ", liveRun ? "Running\u2026" : "Run now"))), /* @__PURE__ */ React.createElement(WorkflowDAG, { workflow: active }), /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mt-16" }, /* @__PURE__ */ React.createElement(Stat, { label: "Steps", value: Array.isArray(active.steps) ? active.steps.length : active.steps || 0 }), /* @__PURE__ */ React.createElement(Stat, { label: "Runs (recent)", value: runs.length }), /* @__PURE__ */ React.createElement(Stat, { label: "p50", value: active.p50_ms ? `${active.p50_ms}ms` : "\u2014" }), /* @__PURE__ */ React.createElement(Stat, { label: "OK rate", value: active.ok != null ? `${active.ok}%` : "\u2014" }), /* @__PURE__ */ React.createElement(Stat, { label: "Updated", value: relativeTime(active.updated_at) }))), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Recent runs")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Run"), /* @__PURE__ */ React.createElement("th", null, "Trigger"), /* @__PURE__ */ React.createElement("th", null, "Started"), /* @__PURE__ */ React.createElement("th", null, "Duration"), /* @__PURE__ */ React.createElement("th", null, "Status"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Tokens"))), /* @__PURE__ */ React.createElement("tbody", null, runs.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 6, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, !active ? "Select a workflow." : "No runs yet.")), runs.map((r) => {
    const id = r.id || r.run_id || "\u2014";
    const trig = r.trigger || r.source || "manual";
    const t = formatTime(r.started_at || r.created_at);
    let dur = "\u2014";
    if (r.duration_ms != null) dur = `${(r.duration_ms / 1e3).toFixed(2)}s`;
    else if (r.started_at && r.completed_at) {
      const ms = Date.parse(r.completed_at) - Date.parse(r.started_at);
      if (!Number.isNaN(ms) && ms >= 0) dur = `${(ms / 1e3).toFixed(2)}s`;
    } else if (Array.isArray(r.step_results)) {
      const ms = r.step_results.reduce((s, x) => s + (x.duration_ms || 0), 0);
      if (ms) dur = `${(ms / 1e3).toFixed(2)}s`;
    }
    const st = (typeof r.state === "string" ? r.state : r.status || r.outcome || "ok").toLowerCase();
    let tok = r.total_tokens || r.tokens;
    if (tok == null && Array.isArray(r.step_results)) {
      tok = r.step_results.reduce((s, x) => s + (x.input_tokens || 0) + (x.output_tokens || 0), 0);
    }
    return /* @__PURE__ */ React.createElement("tr", { key: id, style: { cursor: "pointer" }, onClick: () => setInspectingRun(r), title: "Click to inspect step-by-step output" }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, String(id).slice(0, 8)), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, trig)), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, t), /* @__PURE__ */ React.createElement("td", { className: "mono" }, dur), /* @__PURE__ */ React.createElement("td", null, st === "ok" || st === "success" || st === "completed" ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, st) : st === "failed" || st === "error" ? /* @__PURE__ */ React.createElement("span", { className: "badge error" }, st) : /* @__PURE__ */ React.createElement("span", { className: "badge warn" }, st)), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, tok != null ? tok.toLocaleString() : "\u2014"));
  })))))), showCreate && /* @__PURE__ */ React.createElement(WorkflowCreateModal, { onClose: () => setShowCreate(false), onCreated: (id) => {
    setShowCreate(false);
    setActiveId(id);
    refreshList();
  } }), showImport && /* @__PURE__ */ React.createElement(WorkflowImportModal, { onClose: () => setShowImport(false), onImported: (id) => {
    setShowImport(false);
    if (id) setActiveId(id);
    refreshList();
  } }), showRunInput && active && /* @__PURE__ */ React.createElement(WorkflowRunModal, { workflow: active, onClose: () => setShowRunInput(false), onRun: runWith }), inspectingRun && /* @__PURE__ */ React.createElement(WorkflowRunInspector, { run: inspectingRun, onClose: () => setInspectingRun(null) }), showEditYaml && active && /* @__PURE__ */ React.createElement(
    WorkflowEditYamlModal,
    {
      workflow: active,
      onClose: () => setShowEditYaml(false),
      onSaved: (newId) => {
        setShowEditYaml(false);
        if (newId) setActiveId(newId);
        refreshList();
      }
    }
  ));
}
function WorkflowEditYamlModal({ workflow, onClose, onSaved }) {
  useEscapeKey(onClose);
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
          else if (s.mode.Conditional) {
            step.mode = "conditional";
            step.condition = s.mode.Conditional.condition;
          } else if (s.mode.Loop) {
            step.mode = "loop";
            step.max_iterations = s.mode.Loop.max_iterations;
            step.until = s.mode.Loop.until;
          } else step.mode = "sequential";
        }
        if (s.timeout_secs && s.timeout_secs !== 120) step.timeout_secs = s.timeout_secs;
        if (s.error_mode) {
          if (typeof s.error_mode === "string" && s.error_mode !== "fail") step.error_mode = s.error_mode;
          else if (s.error_mode && s.error_mode.Retry) {
            step.error_mode = "retry";
            step.max_retries = s.error_mode.Retry.max_retries;
          }
        }
        if (s.output_var) step.output_var = s.output_var;
        return step;
      }) : []
    };
    return toYaml(out);
  }, [workflow]);
  const [yamlText, setYamlText] = useState(initialYaml);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const save = async () => {
    if (!yamlText.trim()) {
      setErr("YAML cannot be empty");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const importResp = await rhFetch("/api/workflows/import-yaml", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml: yamlText })
      });
      const newId = importResp && (importResp.id || importResp.workflow_id);
      try {
        await rhFetch(`/api/workflows/${encodeURIComponent(workflow.id)}`, { method: "DELETE" });
      } catch (deleteErr) {
        toastErr(`Saved as new workflow (id=${String(newId).slice(0, 8)}) but failed to delete the old one: ${deleteErr.message || deleteErr}`);
        onSaved(newId);
        return;
      }
      toastOk(`Workflow saved \u2014 new id ${String(newId).slice(0, 8)} (old definition deleted, run history kept)`);
      onSaved(newId);
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Edit workflow YAML"), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, marginTop: 2 } }, workflow.name || workflow.id)), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: { borderColor: "oklch(0.78 0.14 88 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot warn" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "SAVE = DELETE + RECREATE"), /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11 } }, "The API doesn't expose a workflow update endpoint. Saving will create a new workflow from this YAML and then delete the current one. The new workflow gets a fresh id; past run history stays accessible by run-id.")), /* @__PURE__ */ React.createElement(
    "textarea",
    {
      className: "modal-field modal-textarea mono",
      style: { minHeight: 340, fontFamily: "var(--ff-mono)", fontSize: 12, lineHeight: 1.5 },
      value: yamlText,
      onChange: (e) => setYamlText(e.target.value),
      spellCheck: false
    }
  ), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setYamlText(initialYaml), disabled: busy }, "Reset"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: save, disabled: busy }, busy ? "Saving\u2026" : "Save"))));
}
function toYaml(value, indent) {
  const ind = indent || 0;
  const pad = "  ".repeat(ind);
  if (value === null || value === void 0) return "null";
  if (typeof value === "boolean" || typeof value === "number") return String(value);
  if (typeof value === "string") {
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
        const lines = rendered.split("\n");
        return `${pad}- ${lines[0].trimStart()}
${lines.slice(1).join("\n")}`;
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
      return `${pad}${k}:
${toYaml(v, ind + 1)}`;
    }).join("\n");
  }
  return JSON.stringify(value);
}
function exportWorkflowYaml(workflow) {
  if (!workflow) return;
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
        else if (s.mode.Conditional) {
          step.mode = "conditional";
          step.condition = s.mode.Conditional.condition;
        } else if (s.mode.Loop) {
          step.mode = "loop";
          step.max_iterations = s.mode.Loop.max_iterations;
          step.until = s.mode.Loop.until;
        } else step.mode = "sequential";
      }
      if (s.timeout_secs && s.timeout_secs !== 120) step.timeout_secs = s.timeout_secs;
      if (s.error_mode) {
        if (typeof s.error_mode === "string" && s.error_mode !== "fail") step.error_mode = s.error_mode;
        else if (s.error_mode && s.error_mode.Retry) {
          step.error_mode = "retry";
          step.max_retries = s.error_mode.Retry.max_retries;
        }
      }
      if (s.output_var) step.output_var = s.output_var;
      return step;
    }) : []
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
    if (!yamlText.trim()) {
      setErr("YAML is empty");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const r = await rhFetch("/api/workflows/import-yaml", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ yaml: yamlText })
      });
      toastOk("Workflow imported");
      onImported(r && (r.workflow_id || r.id));
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Import workflow from YAML"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 8 } }, "Paste a YAML workflow definition. Same schema as JSON create:", /* @__PURE__ */ React.createElement("span", { className: "mono" }, " name, description, steps:[", `{name, agent_id|agent_name, prompt, mode, ...}`, "]")), /* @__PURE__ */ React.createElement(
    "textarea",
    {
      className: "modal-field modal-textarea",
      style: { minHeight: 320, fontFamily: "var(--ff-mono)", fontSize: 12 },
      value: yamlText,
      onChange: (e) => setYamlText(e.target.value),
      autoFocus: true
    }
  ), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Importing\u2026" : "Import"))));
}
function WorkflowRunInspector({ run, onClose }) {
  useEscapeKey(onClose);
  const [liveRun, setLiveRun] = useState(run);
  React.useEffect(() => {
    setLiveRun(run);
  }, [run && (run.id || run.run_id)]);
  const liveState = typeof liveRun.state === "string" ? liveRun.state.toLowerCase() : "";
  const terminal = liveState === "completed" || liveState === "failed" || liveState === "cancelled";
  const runWorkflowId = liveRun.workflow_id || run.workflow_id || run && run.workflow && run.workflow.id;
  const pollPath = !terminal && runWorkflowId ? `/api/workflows/${encodeURIComponent(runWorkflowId)}/runs` : null;
  const [runsResp] = usePolling(pollPath, 1500);
  React.useEffect(() => {
    if (!runsResp) return;
    const list = Array.isArray(runsResp) ? runsResp : runsResp && runsResp.runs || [];
    const myId = liveRun.id || liveRun.run_id;
    const hit = list.find((r2) => (r2.id || r2.run_id) === myId);
    if (hit) setLiveRun((prev) => ({ ...prev, ...hit }));
  }, [runsResp]);
  const steps = Array.isArray(liveRun.step_results) ? liveRun.step_results : [];
  const totalDur = steps.reduce((s, x) => s + (x.duration_ms || 0), 0);
  const totalTokens = steps.reduce((s, x) => s + (x.input_tokens || 0) + (x.output_tokens || 0), 0);
  const state = typeof liveRun.state === "string" ? liveRun.state : "\u2014";
  const isFailed = state.toLowerCase() === "failed" || !!liveRun.error;
  const isRunning = !terminal && pollPath != null;
  const r = liveRun;
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Run \xB7 ", String(r.id || r.run_id || "").slice(0, 12)), isRunning && /* @__PURE__ */ React.createElement("span", { className: "badge live", style: { marginLeft: 8 }, title: "Polling /api/workflows/.../runs every 1.5s" }, /* @__PURE__ */ React.createElement("span", { className: "dot live" }), "live"), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, marginTop: 2 } }, r.workflow_name || r.workflow_id, " \xB7 started ", formatTime(r.started_at), r.completed_at ? ` \xB7 completed ${formatTime(r.completed_at)}` : " \xB7 running\u2026")), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mb-12" }, /* @__PURE__ */ React.createElement(Stat, { label: "State", value: state }), /* @__PURE__ */ React.createElement(Stat, { label: "Steps", value: `${steps.length}` }), /* @__PURE__ */ React.createElement(Stat, { label: "Duration", value: totalDur ? `${(totalDur / 1e3).toFixed(2)}s` : "\u2014" }), /* @__PURE__ */ React.createElement(Stat, { label: "Tokens", value: totalTokens.toLocaleString() })), r.input && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Initial input"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 100, whiteSpace: "pre-wrap" } }, r.input)), isFailed && r.error && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", color: "var(--crimson)" } }, "Error"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { color: "var(--crimson)", maxHeight: 120 } }, r.error)), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Steps (", steps.length, ")"), /* @__PURE__ */ React.createElement("div", { className: "col gap-6", style: { maxHeight: 420, overflow: "auto" } }, steps.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, padding: "6px 8px" } }, isRunning ? "Awaiting first step\u2026" : "No step output recorded yet."), steps.map((s, i) => {
    const tokens = (s.input_tokens || 0) + (s.output_tokens || 0);
    return /* @__PURE__ */ React.createElement(RunStepCard, { key: i, index: i, step: s, tokens });
  })), r.output && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8 mt-16", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Final output"), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 160, whiteSpace: "pre-wrap" } }, r.output))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Close"))));
}
function RunStepCard({ index, step, tokens }) {
  const [open, setOpen] = useState(false);
  const preview = (step.output || "").slice(0, 200);
  const dur = step.duration_ms != null ? `${(step.duration_ms / 1e3).toFixed(2)}s` : "\u2014";
  return /* @__PURE__ */ React.createElement("div", { style: { border: "1px solid var(--border)", borderRadius: 7, background: "var(--bg-2)", overflow: "hidden" } }, /* @__PURE__ */ React.createElement(
    "button",
    {
      onClick: () => setOpen((o) => !o),
      className: "row gap-8",
      style: { width: "100%", padding: "8px 10px", background: "none", color: "inherit", textAlign: "left", cursor: "pointer" }
    },
    /* @__PURE__ */ React.createElement("span", { className: "badge plain", style: { minWidth: 32, textAlign: "center" } }, "#", index + 1),
    /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12.5 } }, step.step_name || "step"),
    /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, step.agent_name || step.agent_id),
    /* @__PURE__ */ React.createElement("span", { style: { marginLeft: "auto" }, className: "row gap-12" }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, tokens.toLocaleString(), " tok"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, dur), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { width: 12, textAlign: "center" } }, open ? "\u2212" : "+"))
  ), !open && preview && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { padding: "0 12px 8px 50px", fontSize: 11.5, whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" } }, preview), open && /* @__PURE__ */ React.createElement("div", { style: { padding: "4px 12px 10px 12px" } }, /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 240, whiteSpace: "pre-wrap" } }, step.output || "(empty)")));
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
  const steps = Array.isArray(workflow.steps) ? workflow.steps : [];
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
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Run workflow \xB7 ", workflow.name || workflow.id), workflow.description && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5, marginTop: 2 } }, workflow.description)), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Input (JSON)"), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea mt-8", style: { fontFamily: "var(--ff-mono)" }, value: inputJson, onChange: (e) => setInputJson(e.target.value) }), /* @__PURE__ */ React.createElement("span", { className: "muted mono mt-16", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", display: "block" } }, "Preflight \xB7 ", steps.length, " step", steps.length === 1 ? "" : "s", /* @__PURE__ */ React.createElement(Tip, null, "Shown so you confirm which workflow + steps are about to run. Each row lists the step name, target agent, mode, and timeout.")), /* @__PURE__ */ React.createElement("div", { className: "col gap-4 mt-8", style: { maxHeight: 200, overflow: "auto" } }, steps.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11.5 } }, "(workflow has no steps)"), steps.map((s, i) => {
    const agent = s.agent && (s.agent.id || s.agent.name) || s.agent_id || s.agent_name || "\u2014";
    const mode = typeof s.mode === "string" ? s.mode : s.mode && Object.keys(s.mode)[0] || "sequential";
    return /* @__PURE__ */ React.createElement("div", { key: i, className: "row gap-8", style: { padding: "5px 8px", background: "var(--bg-2)", borderRadius: 5, fontFamily: "var(--ff-mono)", fontSize: 11.5 } }, /* @__PURE__ */ React.createElement("span", { className: "badge plain", style: { minWidth: 32, textAlign: "center" } }, "#", i + 1), /* @__PURE__ */ React.createElement("span", { style: { fontWeight: 500 } }, s.name || "step"), /* @__PURE__ */ React.createElement("span", { className: "dim" }, "\u2192 ", agent), mode !== "sequential" && /* @__PURE__ */ React.createElement("span", { className: "badge violet", style: { marginLeft: "auto" } }, mode), s.timeout_secs && s.timeout_secs !== 120 ? /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: mode !== "sequential" ? 6 : "auto" } }, s.timeout_secs, "s") : null);
  })), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: run, disabled: busy }, busy ? "Running\u2026" : `Run ${steps.length} step${steps.length === 1 ? "" : "s"}`))));
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
  const [cronSelected, setCronSelected] = useState(() => /* @__PURE__ */ new Set());
  const [trigSelected, setTrigSelected] = useState(() => /* @__PURE__ */ new Set());
  React.useEffect(() => {
    const onNew = (e) => {
      if (e.detail && e.detail.page === "automation") setShowCreate(true);
    };
    const onRefresh = (e) => {
      if (e.detail && e.detail.page === "automation") (tab === "cron" ? refreshCron : refreshTrig)();
    };
    window.addEventListener("rh:hotkey:new", onNew);
    window.addEventListener("rh:hotkey:refresh", onRefresh);
    return () => {
      window.removeEventListener("rh:hotkey:new", onNew);
      window.removeEventListener("rh:hotkey:refresh", onRefresh);
    };
  }, [tab, refreshCron, refreshTrig]);
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
  React.useEffect(() => {
    if (cronSelected.size === 0) return;
    const live = new Set(cron.map((c) => c.id));
    const next = new Set([...cronSelected].filter((id) => live.has(id)));
    if (next.size !== cronSelected.size) setCronSelected(next);
  }, [cron.map((c) => c.id).join(",")]);
  const toggleCronSelect = (id) => {
    setCronSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };
  const toggleAllCron = () => {
    setCronSelected((prev) => {
      if (cron.length === 0) return prev;
      if (prev.size === cron.length) return /* @__PURE__ */ new Set();
      return new Set(cron.map((c) => c.id));
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
      danger: !target
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/cron/jobs/${encodeURIComponent(id)}/enable`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: target })
        });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setCronSelected(/* @__PURE__ */ new Set());
    if (failCount > 0) toastErr(`${label.toLowerCase()}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${label}d ${okCount} job(s)`);
    refreshCron();
  };
  React.useEffect(() => {
    if (trigSelected.size === 0) return;
    const live = new Set(triggers.map((t) => t.id));
    const next = new Set([...trigSelected].filter((id) => live.has(id)));
    if (next.size !== trigSelected.size) setTrigSelected(next);
  }, [triggers.map((t) => t.id).join(",")]);
  const toggleTrigSelect = (id) => {
    setTrigSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };
  const toggleAllTrig = () => {
    setTrigSelected((prev) => {
      if (triggers.length === 0) return prev;
      if (prev.size === triggers.length) return /* @__PURE__ */ new Set();
      return new Set(triggers.map((t) => t.id));
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
      danger: !target
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/triggers/${encodeURIComponent(id)}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ enabled: target })
        });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setTrigSelected(/* @__PURE__ */ new Set());
    if (failCount > 0) toastErr(`${label.toLowerCase()}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${label}ed ${okCount} trigger(s)`);
    refreshTrig();
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Automation"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Cron jobs survive restart \xB7 3 CronAction variants \xB7 trigger fire-counts persisted")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => (tab === "cron" ? refreshCron : refreshTrig)() }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " New ", tab === "cron" ? "job" : "trigger"))), /* @__PURE__ */ React.createElement("div", { className: "tabs" }, /* @__PURE__ */ React.createElement("button", { className: tab === "cron" ? "on" : "", onClick: () => setTab("cron") }, "Cron jobs \xB7 ", cron.length), /* @__PURE__ */ React.createElement("button", { className: tab === "triggers" ? "on" : "", onClick: () => setTab("triggers") }, "Triggers \xB7 ", triggers.length)), tab === "cron" && /* @__PURE__ */ React.createElement(React.Fragment, null, cronSelected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, cronSelected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => bulkSetEnabled(true) }, /* @__PURE__ */ React.createElement(I.play, null), " Enable ", cronSelected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => bulkSetEnabled(false) }, /* @__PURE__ */ React.createElement(I.pause, null), " Disable ", cronSelected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setCronSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: cron.length > 0 && cronSelected.size === cron.length,
      ref: (el) => {
        if (el) el.indeterminate = cronSelected.size > 0 && cronSelected.size < cron.length;
      },
      onChange: toggleAllCron,
      title: cronSelected.size === cron.length ? "Deselect all" : "Select all"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Schedule"), /* @__PURE__ */ React.createElement("th", null, "Action"), /* @__PURE__ */ React.createElement("th", null, "Next"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Fires"), /* @__PURE__ */ React.createElement("th", null, "Enabled"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !cronResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), cronResp && cron.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, style: { padding: "24px 14px", textAlign: "center" } }, /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 8 } }, "No cron jobs yet."), /* @__PURE__ */ React.createElement("button", { className: "btn primary sm", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Schedule your first job"))), cron.map((c) => {
    const actionLabel = c.action_label || c.action_summary || c.action && (c.action.kind || c.action.type || JSON.stringify(c.action).slice(0, 60)) || "\u2014";
    const next = c.next_run || c.next_fire || c.next || "\u2014";
    const fires = c.fire_count != null ? c.fire_count : c.fires || 0;
    const isSel = cronSelected.has(c.id);
    return /* @__PURE__ */ React.createElement("tr", { key: c.id, style: isSel ? { background: "var(--surface-2)" } : null }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(
      "input",
      {
        type: "checkbox",
        checked: isSel,
        onChange: () => toggleCronSelect(c.id)
      }
    )), /* @__PURE__ */ React.createElement("td", { className: "mono" }, c.id), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { color: "var(--rust)" } }, c.schedule || c.cron || c.expression)), /* @__PURE__ */ React.createElement("td", { className: "mono", style: { maxWidth: 300, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, actionLabel), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, next === "\u2014" ? next : formatTime(next)), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, Number(fires).toLocaleString()), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "switch " + (c.enabled ? "on" : ""), onClick: () => toggleCron(c.id, c.enabled) })), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => runCronNow(c.id) }, "Run now")));
  }))))), tab === "triggers" && /* @__PURE__ */ React.createElement(React.Fragment, null, trigSelected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, trigSelected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => bulkTrigSetEnabled(true) }, /* @__PURE__ */ React.createElement(I.play, null), " Arm ", trigSelected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => bulkTrigSetEnabled(false) }, /* @__PURE__ */ React.createElement(I.pause, null), " Disarm ", trigSelected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setTrigSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: triggers.length > 0 && trigSelected.size === triggers.length,
      ref: (el) => {
        if (el) el.indeterminate = trigSelected.size > 0 && trigSelected.size < triggers.length;
      },
      onChange: toggleAllTrig,
      title: trigSelected.size === triggers.length ? "Deselect all" : "Select all"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "ID"), /* @__PURE__ */ React.createElement("th", null, "Kind"), /* @__PURE__ */ React.createElement("th", null, "Target"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Fired"), /* @__PURE__ */ React.createElement("th", null, "Last"), /* @__PURE__ */ React.createElement("th", null, "Status"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !trigResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), trigResp && triggers.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, style: { padding: "24px 14px", textAlign: "center" } }, /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 8 } }, "No triggers configured."), /* @__PURE__ */ React.createElement("button", { className: "btn primary sm", onClick: () => setShowCreate(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Add your first trigger"))), triggers.map((t) => {
    const kind = (t.kind || t.type || "\u2014").toString();
    const target = t.target || t.agent_id || t.workflow_id || "\u2014";
    const fired = t.fire_count != null ? t.fire_count : t.fired || 0;
    const last = t.last_fired || t.last || null;
    const status = (t.status || (t.enabled ? "active" : "armed")).toString();
    const isSel = trigSelected.has(t.id);
    return /* @__PURE__ */ React.createElement("tr", { key: t.id, style: isSel ? { background: "var(--surface-2)" } : null }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(
      "input",
      {
        type: "checkbox",
        checked: isSel,
        onChange: () => toggleTrigSelect(t.id)
      }
    )), /* @__PURE__ */ React.createElement("td", { className: "mono" }, t.id), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "row gap-6", style: { color: "var(--fg-2)" } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)", display: "inline-flex" } }, /* @__PURE__ */ React.createElement(ChannelIcon, { kind })), /* @__PURE__ */ React.createElement("span", { className: "mono" }, kind))), /* @__PURE__ */ React.createElement("td", { className: "mono" }, target), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, Number(fired).toLocaleString()), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, last ? formatTime(last) : "\u2014"), /* @__PURE__ */ React.createElement("td", null, status === "active" ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "active") : /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, status)), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost" }, /* @__PURE__ */ React.createElement(I.more, null))));
  }))))), showCreate && tab === "cron" && /* @__PURE__ */ React.createElement(CronJobModal, { onClose: () => setShowCreate(false), onCreated: () => {
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
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal wide", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "New cron job"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value), placeholder: "daily-digest" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Owning agent"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: agentId, onChange: (e) => setAgentId(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick agent \u2014"), agents.map((a) => /* @__PURE__ */ React.createElement("option", { key: a.id, value: a.id }, a.name, " (", String(a.id).slice(0, 8), ")")))), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", marginTop: 6 } }, "Schedule", /* @__PURE__ */ React.createElement(Tip, null, /* @__PURE__ */ React.createElement("b", null, "Cron"), " \u2014 5-field expression (min hour dom month dow), optional IANA tz.", /* @__PURE__ */ React.createElement("br", null), /* @__PURE__ */ React.createElement("b", null, "Every"), " \u2014 fixed interval in seconds (60..86400).", /* @__PURE__ */ React.createElement("br", null), /* @__PURE__ */ React.createElement("b", null, "At"), " \u2014 fire once at the specified UTC time.")), /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "cron" ? "on" : "", onClick: () => setScheduleKind("cron") }, "Cron"), /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "every" ? "on" : "", onClick: () => setScheduleKind("every") }, "Every"), /* @__PURE__ */ React.createElement("button", { className: scheduleKind === "at" ? "on" : "", onClick: () => setScheduleKind("at") }, "At")), scheduleKind === "cron" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", placeholder: "0 9 * * *", value: cronExpr, onChange: (e) => setCronExpr(e.target.value) }), scheduleKind === "every" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "number", min: 60, value: everySecs, onChange: (e) => setEverySecs(e.target.value), placeholder: "seconds (60..86400)" }), scheduleKind === "at" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", type: "datetime-local", value: atIso, onChange: (e) => setAtIso(e.target.value) }), /* @__PURE__ */ React.createElement("span", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", marginTop: 6 } }, "Action", /* @__PURE__ */ React.createElement(Tip, null, /* @__PURE__ */ React.createElement("b", null, "System event"), " \u2014 publishes a kernel event with the given text.", /* @__PURE__ */ React.createElement("br", null), /* @__PURE__ */ React.createElement("b", null, "Agent turn"), " \u2014 sends a message to the owning agent.", /* @__PURE__ */ React.createElement("br", null), /* @__PURE__ */ React.createElement("b", null, "Workflow run"), " \u2014 executes a workflow with input.")), /* @__PURE__ */ React.createElement("div", { className: "seg" }, /* @__PURE__ */ React.createElement("button", { className: actionKind === "system_event" ? "on" : "", onClick: () => setActionKind("system_event") }, "System event"), /* @__PURE__ */ React.createElement("button", { className: actionKind === "agent_turn" ? "on" : "", onClick: () => setActionKind("agent_turn") }, "Agent turn"), /* @__PURE__ */ React.createElement("button", { className: actionKind === "workflow_run" ? "on" : "", onClick: () => setActionKind("workflow_run") }, "Workflow run")), actionKind === "system_event" && /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: systemText, onChange: (e) => setSystemText(e.target.value), placeholder: "event text" }), actionKind === "agent_turn" && /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", value: message, onChange: (e) => setMessage(e.target.value), placeholder: "message to send" }), actionKind === "workflow_run" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("select", { className: "t-select", value: workflowId, onChange: (e) => setWorkflowId(e.target.value) }, /* @__PURE__ */ React.createElement("option", { value: "" }, "\u2014 pick workflow \u2014"), workflows.map((w) => /* @__PURE__ */ React.createElement("option", { key: w.id, value: w.id }, w.name || w.id))), /* @__PURE__ */ React.createElement("textarea", { className: "modal-field modal-textarea", value: workflowInput, onChange: (e) => setWorkflowInput(e.target.value), placeholder: "workflow input (text)" }))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Creating\u2026" : "Create"))));
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
  const [activityFor, setActivityFor] = useState(null);
  const [auditResp] = usePolling("/api/audit/recent?n=200", 15e3);
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
    if (!await confirmDialog({ title: "Disconnect", message: `Disconnect ${name}?`, danger: true, confirmLabel: "Disconnect" })) return;
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
    return /* @__PURE__ */ React.createElement("div", { key: ch.name, className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("div", { className: "avatar lg", style: { background: "linear-gradient(135deg,var(--rust),oklch(0.42 0.10 50))" } }, /* @__PURE__ */ React.createElement(ChannelIcon, { kind })), /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, fontWeight: 500 } }, ch.display_name || ch.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, ch.name, ch.difficulty ? ` \xB7 ${ch.difficulty}` : "", ch.setup_time ? ` \xB7 ${ch.setup_time}` : ""))), /* @__PURE__ */ React.createElement("span", { className: "badge " + (state === "live" ? "live" : state === "auth_failed" ? "error" : "idle") }, stateLabel)), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5, marginBottom: 12 } }, ch.description), testResult && testResult.name === ch.name && /* @__PURE__ */ React.createElement(ChannelTestCard, { result: testResult }), activityFor === ch.name && /* @__PURE__ */ React.createElement(
      ChannelActivityCard,
      {
        channelName: ch.name,
        entries: auditResp && auditResp.entries || [],
        onClose: () => setActivityFor(null)
      }
    ), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, ch.configured ? /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => testChannel(ch.name) }, "Test"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setActivityFor(activityFor === ch.name ? null : ch.name) }, activityFor === ch.name ? "Hide activity" : "Activity"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setConfiguring(ch) }, "Reconfigure"), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => removeChannel(ch.name), style: { color: "var(--crimson)" } }, "Disconnect")) : /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => setConfiguring(ch) }, /* @__PURE__ */ React.createElement(I.plus, null), " Connect")));
  })), configuring && /* @__PURE__ */ React.createElement(ChannelConfigModal, { channel: configuring, onClose: () => setConfiguring(null), onSaved: () => {
    setConfiguring(null);
    refresh();
  } }));
}
function ChannelActivityCard({ channelName, entries, onClose }) {
  const lc = (channelName || "").toLowerCase();
  if (!lc) return null;
  const matches = (entries || []).filter((e) => {
    const haystack = [e.action || "", e.detail || "", e.agent_name || "", e.agent_id || "", e.outcome || ""].join(" ").toLowerCase();
    return haystack.includes(lc);
  }).slice(0, 10);
  return /* @__PURE__ */ React.createElement("div", { style: {
    marginTop: 8,
    padding: "10px 12px",
    background: "var(--bg-2)",
    borderRadius: 6,
    border: "1px solid var(--border)"
  } }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-8" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Recent activity ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, matches.length, " of last 200")), /* @__PURE__ */ React.createElement("button", { className: "kbd", onClick: onClose, style: { cursor: "pointer" } }, "close")), matches.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, textAlign: "center", padding: "6px 0" } }, "No matches in the last 200 audit entries."), matches.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 200, overflow: "auto" } }, matches.map((e, i) => {
    const hash = e.hash ? String(e.hash).slice(0, 8) : "";
    const link = hash ? `#/audit?h=${encodeURIComponent(hash)}` : "#/audit";
    return /* @__PURE__ */ React.createElement("a", { key: `${e.seq || i}`, href: link, style: {
      display: "block",
      padding: "5px 8px",
      fontFamily: "var(--ff-mono)",
      fontSize: 11,
      background: "var(--surface)",
      borderRadius: 5,
      color: "var(--fg-2)",
      textDecoration: "none"
    } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, e.action || "\u2014"), e.outcome && /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, "\xB7 ", e.outcome), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, "\xB7 ", relativeTime(e.timestamp || e.created_at)), e.detail && /* @__PURE__ */ React.createElement("div", { className: "dim", style: {
      marginTop: 2,
      whiteSpace: "nowrap",
      overflow: "hidden",
      textOverflow: "ellipsis"
    } }, e.detail));
  })));
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
  const otherKnown = /* @__PURE__ */ new Set(["ok", "busy", "name", "message", "detail", "latency_ms", "status", "me", "gateway", "workspace"]);
  const extras = Object.entries(r).filter(([k]) => !otherKnown.has(k) && r[k] != null);
  const message = r.message || r.detail;
  return /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: {
    flexDirection: "column",
    alignItems: "stretch",
    gap: 6,
    borderColor: r.busy ? "var(--border-hi)" : ok ? "oklch(0.74 0.135 150 / .35)" : "oklch(0.66 0.18 25 / .35)"
  } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "dot " + (r.busy ? "warn" : ok ? "live" : "err") }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, r.busy ? "TESTING" : ok ? "OK" : "FAIL"), message && /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11.5 } }, message)), known.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "kv", style: { gridTemplateColumns: "110px 1fr", fontSize: 11.5, padding: "4px 0 0 16px" } }, known.map(([k, v]) => /* @__PURE__ */ React.createElement(React.Fragment, { key: k }, /* @__PURE__ */ React.createElement("dt", { style: { fontFamily: "var(--ff-mono)", color: "var(--fg-4)" } }, k), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, v)))), extras.length > 0 && /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { fontSize: 10.5, marginTop: 6, maxHeight: 120, marginLeft: 16 } }, extras.map(([k, v]) => `${k}: ${typeof v === "string" ? v : JSON.stringify(v, null, 2)}`).join("\n")));
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
  const [usageAgents] = usePolling("/api/usage", 3e4);
  const agents = agentsResp && agentsResp.agents ? agentsResp.agents.map(normalizeAgent) : [];
  const usageList = usageAgents && Array.isArray(usageAgents.agents) ? usageAgents.agents : [];
  const agentById = new Map(agents.map((a) => [a.id, a]));
  const usageJoined = usageList.map((u) => {
    const a = agentById.get(u.agent_id);
    return {
      id: u.agent_id,
      name: u.name || a && a.name || u.agent_id,
      model: a ? a.model : "\u2014",
      state: a ? a.state : "unknown",
      total_tokens: Number(u.total_tokens || 0),
      tool_calls: Number(u.tool_calls || 0)
    };
  }).filter((u) => u.total_tokens > 0 || u.tool_calls > 0).sort((x, y) => y.total_tokens - x.total_tokens);
  const maxTokens = Math.max(1, ...usageJoined.map((u) => u.total_tokens));
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
  } }, /* @__PURE__ */ React.createElement(I.download, null), " CSV"))), /* @__PURE__ */ React.createElement("div", { className: "tiles" }, /* @__PURE__ */ React.createElement(Tile, { label: "Total spend \xB7 7d", value: `$${totalSpend7d.toFixed(2)}`, foot: daily ? `${days.length} day(s) of data` : "loading\u2026", spark: dailyCosts.slice(-12) }), /* @__PURE__ */ React.createElement(Tile, { label: "LLM requests \xB7 7d", value: totalRequests7d.toLocaleString(), foot: daily ? `${avgPerDay.toFixed(0)} / day avg` : "loading\u2026", spark: days.slice(-12).map((d) => d.requests || 0) }), /* @__PURE__ */ React.createElement(Tile, { label: "Cache hit-rate", value: cacheHitRate, foot: stats ? "LLM cache \xB7 24h TTL" : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] }), /* @__PURE__ */ React.createElement(Tile, { label: "p95 latency", value: p95, foot: stats ? "kernel telemetry" : "loading\u2026", spark: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] })), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Cost \xB7 daily (last 24 buckets)"), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11 } }, "$", totalForChart.toFixed(2))), /* @__PURE__ */ React.createElement(CostChart, { data: seriesForChart })), /* @__PURE__ */ React.createElement("div", { className: "col-4 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Spend by model"), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11 } }, "$", modelRows.reduce((s, m) => s + Number(m.spend || m.cost_usd || 0), 0).toFixed(2))), /* @__PURE__ */ React.createElement("div", { className: "col", style: { gap: 4 } }, !byModel && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11, padding: "6px 0" } }, "loading\u2026"), byModel && modelRows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { fontSize: 11, padding: "6px 0" } }, "no model usage data yet."), modelRows.slice(0, 8).map((m) => /* @__PURE__ */ React.createElement(BarRow, { key: m.model || m.name, label: m.model || m.name, value: Number(m.spend || m.cost_usd || 0), max: maxModelSpend, unit: "$" })))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card flush" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Agents \xB7 by spend"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, "top ", Math.min(usageJoined.length, 10), " of ", usageJoined.length)), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Model"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Tokens"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Tool calls"), /* @__PURE__ */ React.createElement("th", null, "State"))), /* @__PURE__ */ React.createElement("tbody", null, !usageAgents && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 5, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), usageAgents && usageJoined.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 5, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No usage recorded yet \u2014 agents start showing here after their first LLM call.")), usageJoined.slice(0, 10).map((u) => {
    const pct = u.total_tokens / maxTokens * 100;
    return /* @__PURE__ */ React.createElement("tr", { key: u.id }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "agent-row" }, /* @__PURE__ */ React.createElement(Avatar, { agent: { name: u.name, hue: hueFromId(u.id), emoji: u.name.charAt(0).toUpperCase() } }), /* @__PURE__ */ React.createElement("span", { className: "name" }, u.name))), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, u.model), /* @__PURE__ */ React.createElement("td", { className: "num mono", style: { position: "relative", minWidth: 90 } }, /* @__PURE__ */ React.createElement("span", { style: {
      position: "absolute",
      inset: 0,
      background: `linear-gradient(90deg, transparent ${100 - pct}%, oklch(0.665 0.165 50 / .12) ${100 - pct}%)`,
      pointerEvents: "none"
    } }), /* @__PURE__ */ React.createElement("span", { style: { position: "relative" } }, u.total_tokens.toLocaleString())), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, u.tool_calls.toLocaleString()), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(StateBadge, { state: u.state })));
  })))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Provider state"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, "from /api/providers")), /* @__PURE__ */ React.createElement(ProviderState, null))));
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
  const clean = Array.isArray(data) ? data.map((v) => Number(v)).filter((v) => Number.isFinite(v)) : [];
  if (clean.length < 2) {
    return /* @__PURE__ */ React.createElement("svg", { viewBox: `0 0 ${W} ${H}`, width: "100%", height: "220", style: { display: "block" } });
  }
  const max = Math.max(...clean), min = 0;
  const span = max - min || 1;
  const x = (i) => P + i / (clean.length - 1) * (W - P * 2);
  const y = (v) => H - P - (v - min) / span * (H - P * 2);
  const path = clean.map((v, i) => i === 0 ? `M${x(i)},${y(v)}` : `L${x(i)},${y(v)}`).join(" ");
  const area = `${path} L${x(clean.length - 1)},${H - P} L${x(0)},${H - P} Z`;
  return /* @__PURE__ */ React.createElement("svg", { viewBox: `0 0 ${W} ${H}`, width: "100%", height: "220", style: { display: "block" } }, /* @__PURE__ */ React.createElement("defs", null, /* @__PURE__ */ React.createElement("linearGradient", { id: "g1", x1: "0", x2: "0", y1: "0", y2: "1" }, /* @__PURE__ */ React.createElement("stop", { offset: "0%", stopColor: "var(--rust)", stopOpacity: ".35" }), /* @__PURE__ */ React.createElement("stop", { offset: "100%", stopColor: "var(--rust)", stopOpacity: "0" }))), [0, 0.25, 0.5, 0.75, 1].map((t) => {
    const yy = P + (1 - t) * (H - P * 2);
    return /* @__PURE__ */ React.createElement("g", { key: t }, /* @__PURE__ */ React.createElement("line", { x1: P, x2: W - P, y1: yy, y2: yy, stroke: "var(--border)", strokeWidth: "1" }), /* @__PURE__ */ React.createElement("text", { x: 4, y: yy + 3, fill: "var(--fg-4)", fontFamily: "var(--ff-mono)", fontSize: "9.5" }, "$", (max * t).toFixed(1)));
  }), /* @__PURE__ */ React.createElement("path", { d: area, fill: "url(#g1)" }), /* @__PURE__ */ React.createElement("path", { d: path, fill: "none", stroke: "var(--rust)", strokeWidth: "1.8", strokeLinejoin: "round" }), clean.map((v, i) => i % 4 === 0 && /* @__PURE__ */ React.createElement("text", { key: i, x: x(i), y: H - 8, fill: "var(--fg-4)", fontFamily: "var(--ff-mono)", fontSize: "9.5", textAnchor: "middle" }, i.toString().padStart(2, "0"), "h")));
};
function KnowledgePage() {
  const [graph, , refresh] = usePolling("/api/knowledge", 3e4);
  const [showAdd, setShowAdd] = useState(false);
  const [showRel, setShowRel] = useState(false);
  const [editingRel, setEditingRel] = useState(null);
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
  ), (query || submittedQuery) && /* @__PURE__ */ React.createElement("button", { className: "kbd", onClick: clearQuery, style: { cursor: "pointer" } }, "clear")), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: runQuery, disabled: !query.trim() }, /* @__PURE__ */ React.createElement(I.play, null), " Run"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowAdd(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Add node"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowRel(true) }, /* @__PURE__ */ React.createElement(I.link, null), " Add relation"))), queryErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "QUERY FAILED"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, queryErr)), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card", style: { padding: 0, overflow: "hidden" } }, !graph && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "40px", fontSize: 12, textAlign: "center" } }, "loading graph\u2026"), graph && nodes.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "40px", fontSize: 12, textAlign: "center" } }, "No knowledge graph data yet."), graph && nodes.length > 0 && /* @__PURE__ */ React.createElement(KGViz, { nodes, edges, onSelect: setActiveId, activeId: active && active.id })), /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, active ? `Node \xB7 ${active.type || "entity"} \xB7 ${active.name || active.id}` : "Select a node"), active && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "row between mb-8" }, /* @__PURE__ */ React.createElement("div", { className: "kv", style: { flex: 1 } }, /* @__PURE__ */ React.createElement("dt", null, "id"), /* @__PURE__ */ React.createElement("dd", null, active.id), /* @__PURE__ */ React.createElement("dt", null, "kind"), /* @__PURE__ */ React.createElement("dd", null, active.type || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "degree"), /* @__PURE__ */ React.createElement("dd", null, activeEdges.length)), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn ghost",
      title: "Delete this entity (and all its relations)",
      onClick: async () => {
        const ok = await confirmDialog({
          title: "Delete entity?",
          body: `This will remove '${active.name || active.id}' and ${activeEdges.length} relation(s).`,
          confirmLabel: "Delete",
          danger: true
        });
        if (!ok) return;
        try {
          await rhFetch(`/api/knowledge/entities/${encodeURIComponent(active.id)}`, { method: "DELETE" });
          toastOk("Entity deleted");
          setActiveId(null);
          refresh();
        } catch (err) {
          toastErr(`Delete failed: ${err.message || err}`);
        }
      }
    },
    /* @__PURE__ */ React.createElement(I.trash, null)
  )), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Edges (", activeEdges.length, ")"), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 240, overflow: "auto" } }, activeEdges.slice(0, 16).map((e, i) => {
    var _a;
    const src = e.source || e.source_id;
    const dst = e.target || e.target_id;
    const other = src === active.id ? dst : src;
    const relId = e.id || e.relation_id || "";
    return /* @__PURE__ */ React.createElement("div", { key: relId || i, className: "row between", style: { padding: "5px 8px", background: "var(--bg-2)", borderRadius: 5, fontSize: 11.5, fontFamily: "var(--ff-mono)" } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, e.relation || e.label || "\u2192"), /* @__PURE__ */ React.createElement("span", { className: "muted", style: { flex: 1, textAlign: "right" } }, "\u2192 ", ((_a = nodes.find((n) => n.id === other)) == null ? void 0 : _a.name) || other), relId && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement(
      "button",
      {
        className: "kbd",
        style: { marginLeft: 6, cursor: "pointer" },
        title: "Edit this relation",
        onClick: (ev) => {
          ev.stopPropagation();
          setEditingRel({
            id: relId,
            source: src,
            target: dst,
            relation: e.relation || "related_to",
            confidence: e.confidence != null ? e.confidence : 1,
            properties: e.properties || {}
          });
        }
      },
      "edit"
    ), /* @__PURE__ */ React.createElement(
      "button",
      {
        className: "kbd",
        style: { marginLeft: 6, cursor: "pointer", color: "oklch(0.66 0.18 25)" },
        title: "Delete this relation",
        onClick: async (ev) => {
          ev.stopPropagation();
          const ok = await confirmDialog({
            title: "Delete relation?",
            body: `${e.relation || "\u2192"} between these two entities will be removed.`,
            confirmLabel: "Delete",
            danger: true
          });
          if (!ok) return;
          try {
            await rhFetch(`/api/knowledge/relations/${encodeURIComponent(relId)}`, { method: "DELETE" });
            toastOk("Relation deleted");
            refresh();
          } catch (err) {
            toastErr(`Delete failed: ${err.message || err}`);
          }
        }
      },
      "del"
    )));
  })))))), showAdd && /* @__PURE__ */ React.createElement(KnowledgeAddNodeModal, { onClose: () => setShowAdd(false), onAdded: () => {
    setShowAdd(false);
    refresh();
  } }), showRel && /* @__PURE__ */ React.createElement(KnowledgeAddRelationModal, { nodes: allNodes, onClose: () => setShowRel(false), onAdded: () => {
    setShowRel(false);
    refresh();
  } }), editingRel && /* @__PURE__ */ React.createElement(
    KnowledgeAddRelationModal,
    {
      nodes: allNodes,
      edit: editingRel,
      onClose: () => setEditingRel(null),
      onAdded: () => {
        setEditingRel(null);
        refresh();
      }
    }
  ));
}
function KnowledgeAddRelationModal({ nodes, onClose, onAdded, edit }) {
  useEscapeKey(onClose);
  const isEdit = !!edit;
  const sortedNodes = React.useMemo(() => (nodes || []).slice().sort((a, b) => (a.name || a.id).localeCompare(b.name || b.id)), [nodes]);
  const [source, setSource] = useState(
    isEdit ? edit.source : sortedNodes[0] ? sortedNodes[0].id : ""
  );
  const [target, setTarget] = useState(
    isEdit ? edit.target : sortedNodes[1] ? sortedNodes[1].id : sortedNodes[0] ? sortedNodes[0].id : ""
  );
  const [relation, setRelation] = useState(isEdit ? edit.relation : "works_at");
  const [confidence, setConfidence] = useState(
    isEdit ? String(edit.confidence != null ? edit.confidence : 1) : "1.0"
  );
  const [propsJson, setPropsJson] = useState(
    isEdit ? JSON.stringify(edit.properties || {}, null, 2) : "{}"
  );
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const RELATIONS = [
    "works_at",
    "knows_about",
    "related_to",
    "depends_on",
    "owned_by",
    "created_by",
    "located_in",
    "part_of",
    "uses",
    "produces",
    "manages",
    "collaborates_with",
    "mentions",
    "cites",
    "implements",
    "other"
  ];
  const submit = async () => {
    if (!source || !target) {
      setErr("Both source and target are required");
      return;
    }
    if (source === target) {
      setErr("Source and target must differ");
      return;
    }
    let properties = {};
    if (propsJson.trim()) {
      try {
        properties = JSON.parse(propsJson);
      } catch (e) {
        setErr(`Properties must be valid JSON: ${e.message}`);
        return;
      }
      if (typeof properties !== "object" || Array.isArray(properties)) {
        setErr("Properties must be a JSON object");
        return;
      }
    }
    const c = Number(confidence);
    if (Number.isNaN(c) || c < 0 || c > 1) {
      setErr("Confidence must be 0..1");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const body = JSON.stringify({ source, target, relation, confidence: c, properties });
      if (isEdit) {
        await rhFetch(`/api/knowledge/relations/${encodeURIComponent(edit.id)}`, {
          method: "PUT",
          headers: { "Content-Type": "application/json" },
          body
        });
        toastOk(`Updated ${relation} relation`);
      } else {
        await rhFetch("/api/knowledge/relations", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body
        });
        toastOk(`Added ${relation} relation`);
      }
      onAdded();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, isEdit ? "Edit knowledge relation" : "Add knowledge relation"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, sortedNodes.length < 2 && /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: { borderColor: "oklch(0.78 0.14 88 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot warn" }), /* @__PURE__ */ React.createElement("span", { className: "banner-body", style: { fontSize: 11.5 } }, "Need at least 2 nodes to create a relation. Add a node first.")), /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Source"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: source, onChange: (e) => setSource(e.target.value) }, sortedNodes.map((n) => /* @__PURE__ */ React.createElement("option", { key: n.id, value: n.id }, n.name || n.id, " ", /* @__PURE__ */ React.createElement("span", null, "(", n.type || "?", ")"))))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Relation", /* @__PURE__ */ React.createElement(Tip, null, "One of the RelationType variants \u2014 these are the same labels the kernel uses for graph traversal queries.")), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: relation, onChange: (e) => setRelation(e.target.value) }, RELATIONS.map((r) => /* @__PURE__ */ React.createElement("option", { key: r, value: r }, r)))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Target"), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: target, onChange: (e) => setTarget(e.target.value) }, sortedNodes.map((n) => /* @__PURE__ */ React.createElement("option", { key: n.id, value: n.id }, n.name || n.id, " ", /* @__PURE__ */ React.createElement("span", null, "(", n.type || "?", ")"))))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Confidence (0..1)", /* @__PURE__ */ React.createElement(Tip, null, "Float weight on the relation. Used by graph-query ranking. Default 1.0 = certain.")), /* @__PURE__ */ React.createElement(
    "input",
    {
      className: "modal-field",
      type: "number",
      step: "0.05",
      min: "0",
      max: "1",
      value: confidence,
      onChange: (e) => setConfidence(e.target.value)
    }
  )), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Properties (JSON object)"), /* @__PURE__ */ React.createElement(
    "textarea",
    {
      className: "modal-field modal-textarea",
      style: { minHeight: 80, fontFamily: "var(--ff-mono)" },
      value: propsJson,
      onChange: (e) => setPropsJson(e.target.value)
    }
  ))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy || sortedNodes.length < 2 }, busy ? isEdit ? "Saving\u2026" : "Adding\u2026" : isEdit ? "Save changes" : "Add relation"))));
}
function KnowledgeAddNodeModal({ onClose, onAdded }) {
  useEscapeKey(onClose);
  const [id, setId] = useState("");
  const [type, setType] = useState("person");
  const [name, setName] = useState("");
  const [propsJson, setPropsJson] = useState("{}");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState(null);
  const TYPES = ["person", "organization", "project", "concept", "event", "location", "document", "tool", "other"];
  const submit = async () => {
    if (!name.trim()) {
      setErr("Name required");
      return;
    }
    let properties = {};
    if (propsJson.trim()) {
      try {
        properties = JSON.parse(propsJson);
      } catch (e) {
        setErr(`Properties must be valid JSON: ${e.message}`);
        return;
      }
      if (typeof properties !== "object" || Array.isArray(properties)) {
        setErr("Properties must be a JSON object");
        return;
      }
    }
    setBusy(true);
    setErr(null);
    try {
      await rhFetch("/api/knowledge/entities", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ id: id.trim() || void 0, type, name: name.trim(), properties })
      });
      toastOk(`Added ${name.trim()}`);
      onAdded();
    } catch (e) {
      setErr(String(e.message || e));
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Add knowledge node"), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "col gap-8" }, /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Name"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: name, onChange: (e) => setName(e.target.value), placeholder: "A. Linder", autoFocus: true })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Type", /* @__PURE__ */ React.createElement(Tip, null, "One of the EntityType variants: person, organization, project, concept, event, location, document, tool, other.")), /* @__PURE__ */ React.createElement("select", { className: "t-select", value: type, onChange: (e) => setType(e.target.value) }, TYPES.map((t) => /* @__PURE__ */ React.createElement("option", { key: t, value: t }, t)))), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "ID (optional \u2014 kernel assigns UUID if empty)"), /* @__PURE__ */ React.createElement("input", { className: "modal-field", value: id, onChange: (e) => setId(e.target.value), placeholder: "p_linder" })), /* @__PURE__ */ React.createElement("label", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, "Properties (JSON object)"), /* @__PURE__ */ React.createElement(
    "textarea",
    {
      className: "modal-field modal-textarea",
      style: { minHeight: 100, fontFamily: "var(--ff-mono)" },
      value: propsJson,
      onChange: (e) => setPropsJson(e.target.value)
    }
  ))), err && /* @__PURE__ */ React.createElement("div", { className: "banner mt-12", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, err))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose }, "Cancel"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: submit, disabled: busy }, busy ? "Adding\u2026" : "Add node"))));
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
  const [inspect, setInspect] = useState(null);
  const [selected, setSelected] = useState(() => /* @__PURE__ */ new Set());
  React.useEffect(() => {
    if (selected.size === 0) return;
    const live = new Set(skills.map((s) => s.name));
    const next = new Set([...selected].filter((n) => live.has(n)));
    if (next.size !== selected.size) setSelected(next);
  }, [skills.map((s) => s.name).join(",")]);
  const isUninstallable = (s) => {
    const o = (s.source || s.origin || (s.privileged ? "privileged" : "builtin")).toString().toLowerCase();
    return o !== "bundled" && o !== "builtin";
  };
  const uninstallableSkills = skills.filter(isUninstallable);
  const uninstall = async (name) => {
    if (!await confirmDialog({ title: "Uninstall skill", message: `Uninstall skill ${name}?`, danger: true, confirmLabel: "Uninstall" })) return;
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
  const toggle = (name) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };
  const toggleAllUninstallable = () => {
    setSelected((prev) => {
      if (uninstallableSkills.length === 0) return prev;
      if (prev.size === uninstallableSkills.length) return /* @__PURE__ */ new Set();
      return new Set(uninstallableSkills.map((s) => s.name));
    });
  };
  const bulkUninstall = async () => {
    const names = [...selected];
    if (names.length === 0) return;
    const ok = await confirmDialog({
      title: `Uninstall ${names.length} skill(s)?`,
      message: `Uninstall all selected non-bundled skills. This cannot be undone.`,
      danger: true,
      confirmLabel: `Uninstall ${names.length}`
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const name of names) {
      try {
        await rhFetch("/api/skills/uninstall", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ name })
        });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setSelected(/* @__PURE__ */ new Set());
    if (failCount > 0) toastErr(`Uninstalled ${okCount}, failed ${failCount}`);
    else toastOk(`Uninstalled ${okCount} skill(s)`);
    refresh();
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Skills ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", skills.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Bundled + ", /* @__PURE__ */ React.createElement("span", { className: "mono", style: { color: "var(--rust)" } }, "ClawHub"), " marketplace \xB7 WASM sandbox \xB7 capability gating")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, "Reload"), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => setShowCustom(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Install custom"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => setShowClawHub(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " ClawHub"))), selected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, selected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: bulkUninstall }, /* @__PURE__ */ React.createElement(I.trash, null), " Uninstall ", selected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: uninstallableSkills.length > 0 && selected.size === uninstallableSkills.length,
      ref: (el) => {
        if (el) el.indeterminate = selected.size > 0 && selected.size < uninstallableSkills.length;
      },
      onChange: toggleAllUninstallable,
      disabled: uninstallableSkills.length === 0,
      title: uninstallableSkills.length === 0 ? "Only bundled skills installed" : "Toggle all non-bundled skills"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "Skill"), /* @__PURE__ */ React.createElement("th", null, "Origin"), /* @__PURE__ */ React.createElement("th", null, "Runtime"), /* @__PURE__ */ React.createElement("th", null, "Version"), /* @__PURE__ */ React.createElement("th", null, "Enabled"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !skillsResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), skillsResp && skills.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, style: { padding: "24px 14px", textAlign: "center" } }, /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 12, marginBottom: 8 } }, "No skills installed yet \u2014 bundled tools work without registration."), /* @__PURE__ */ React.createElement("span", { className: "row gap-6", style: { justifyContent: "center" } }, /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => setShowCustom(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Custom"), /* @__PURE__ */ React.createElement("button", { className: "btn primary sm", onClick: () => setShowClawHub(true) }, /* @__PURE__ */ React.createElement(I.plus, null), " Browse ClawHub")))), skills.map((s) => {
    const origin = (s.source || s.origin || (s.privileged ? "privileged" : "builtin")).toString().toLowerCase();
    const cat = s.category || s.runtime || s.type || "\u2014";
    const ver = s.version || "\u2014";
    const en = s.enabled !== false;
    const isBundled = origin === "bundled" || origin === "builtin";
    const isSel = selected.has(s.name);
    return /* @__PURE__ */ React.createElement("tr", { key: s.name, style: { cursor: "pointer", background: isSel ? "var(--surface-2)" : void 0 }, onClick: () => setInspect(s) }, /* @__PURE__ */ React.createElement("td", { onClick: (e) => {
      e.stopPropagation();
      if (!isBundled) toggle(s.name);
    } }, /* @__PURE__ */ React.createElement(
      "input",
      {
        type: "checkbox",
        checked: isSel,
        disabled: isBundled,
        readOnly: true,
        tabIndex: -1,
        title: isBundled ? "Bundled skills can't be uninstalled" : ""
      }
    )), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "mono" }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, "\u203A"), " ", s.name)), /* @__PURE__ */ React.createElement("td", null, origin === "clawhub" || origin === "claw" ? /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, "ClawHub") : origin === "privileged" ? /* @__PURE__ */ React.createElement("span", { className: "badge warn" }, "privileged") : /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, origin)), /* @__PURE__ */ React.createElement("td", { className: "muted mono" }, s.runtime || cat), /* @__PURE__ */ React.createElement("td", { className: "mono" }, ver), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("div", { className: "switch " + (en ? "on" : "") })), /* @__PURE__ */ React.createElement("td", { className: "right", style: { position: "relative" }, onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setRowMenu(rowMenu === s.name ? null : s.name) }, /* @__PURE__ */ React.createElement(I.more, null)), rowMenu === s.name && /* @__PURE__ */ React.createElement("div", { className: "row-menu", onClick: (e) => e.stopPropagation() }, isBundled ? /* @__PURE__ */ React.createElement("button", { disabled: true, title: "bundled skills cannot be uninstalled" }, "Bundled \u2014 cannot remove") : /* @__PURE__ */ React.createElement("button", { onClick: () => {
      setRowMenu(null);
      uninstall(s.name);
    }, style: { color: "var(--crimson)" } }, /* @__PURE__ */ React.createElement(I.close, null), " Uninstall"))));
  })))), showCustom && /* @__PURE__ */ React.createElement(SkillInstallModal, { onClose: () => setShowCustom(false), onInstalled: () => {
    setShowCustom(false);
    refresh();
  } }), showClawHub && /* @__PURE__ */ React.createElement(ClawHubModal, { onClose: () => setShowClawHub(false), onInstalled: () => {
    setShowClawHub(false);
    refresh();
  } }), inspect && /* @__PURE__ */ React.createElement(SkillDetailModal, { skill: inspect, onClose: () => setInspect(null), onUninstall: uninstall }));
}
function SkillDetailModal({ skill, onClose, onUninstall }) {
  useEscapeKey(onClose);
  const s = skill || {};
  const source = s.source && typeof s.source === "object" ? s.source : { type: s.source || "\u2014" };
  const sourceType = (source.type || "").toLowerCase();
  const isBundled = sourceType === "bundled" || sourceType === "builtin";
  const [audit] = useApi(s.name ? `/api/audit/recent?n=50` : null);
  const recent = (audit && audit.entries || []).filter((e) => (e.action || "").toLowerCase().includes(String(s.name).toLowerCase())).slice(0, 8);
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono" }, s.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, marginTop: 2 } }, s.author || "\u2014", " \xB7 v", s.version || "\u2014")), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8 mb-12" }, sourceType === "clawhub" && /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, "ClawHub"), sourceType === "openclaw" && /* @__PURE__ */ React.createElement("span", { className: "badge violet" }, "OpenClaw"), isBundled && /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, "bundled"), !isBundled && sourceType === "local" && /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, "local"), /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, s.runtime || "\u2014"), /* @__PURE__ */ React.createElement("span", { className: "badge " + (s.enabled !== false ? "live" : "idle") }, s.enabled !== false ? "enabled" : "disabled"), s.has_prompt_context && /* @__PURE__ */ React.createElement("span", { className: "badge sky" }, "prompt context")), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Description"), /* @__PURE__ */ React.createElement("div", { className: "codebox mb-16", style: { whiteSpace: "pre-wrap" } }, s.description || "(no description)"), /* @__PURE__ */ React.createElement("div", { className: "kv mb-16" }, /* @__PURE__ */ React.createElement("dt", null, "tools exposed"), /* @__PURE__ */ React.createElement("dd", null, s.tools_count != null ? s.tools_count : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "tags"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, s.tags && s.tags.length ? s.tags.join(", ") : "\u2014"), source.slug && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "clawhub slug"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, source.slug)), source.version && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "clawhub version"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, source.version))), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Recent invocations ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6, fontSize: 10 } }, "(audit substring match)")), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 200, overflow: "auto" } }, !audit && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, padding: "6px 8px" } }, "loading audit\u2026"), audit && recent.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, padding: "6px 8px" } }, "No recent invocations in the loaded window."), recent.map((e, i) => /* @__PURE__ */ React.createElement("div", { key: e.hash || e.seq || i, className: "row", style: { padding: "5px 8px", background: "var(--bg-2)", borderRadius: 5 } }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, width: 70 } }, formatTime(e.timestamp)), /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12, flex: 1 } }, e.action), /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11 } }, e.agent_name || e.agent_id || "\u2014"))))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, !isBundled && /* @__PURE__ */ React.createElement("button", { className: "btn danger", onClick: () => onUninstall(s.name).then(onClose), style: { marginRight: "auto" } }, /* @__PURE__ */ React.createElement(I.close, null), " Uninstall"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: onClose }, "Close"))));
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
  const [peekSlug, setPeekSlug] = useState(null);
  const [peekCache, setPeekCache] = useState({});
  const [peekErr, setPeekErr] = useState(null);
  const peek = async (slug) => {
    if (peekSlug === slug) {
      setPeekSlug(null);
      return;
    }
    setPeekSlug(slug);
    setPeekErr(null);
    if (peekCache[slug]) return;
    try {
      const r = await rhFetch(`/api/clawhub/skill/${encodeURIComponent(slug)}`);
      setPeekCache((prev) => ({ ...prev, [slug]: r }));
    } catch (e) {
      setPeekErr(String(e.message || e));
    }
  };
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
    const isOpen = peekSlug === slug;
    const detail = peekCache[slug];
    return /* @__PURE__ */ React.createElement("div", { key: slug, style: { padding: "10px 12px", border: "1px solid var(--border)", borderRadius: 7, background: "var(--bg-2)" } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12" }, /* @__PURE__ */ React.createElement("div", { className: "col", style: { flex: 1, gap: 3, minWidth: 0 } }, /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 13 } }, it.name || slug), it.version && /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, it.version), it.author && /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5 } }, it.author)), /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 11.5, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" } }, it.description || it.summary || "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "row gap-12 dim mono", style: { fontSize: 10.5 } }, it.downloads != null && /* @__PURE__ */ React.createElement("span", null, "\u2193 ", Number(it.downloads).toLocaleString()), it.stars != null && /* @__PURE__ */ React.createElement("span", null, "\u2605 ", it.stars), it.rating != null && /* @__PURE__ */ React.createElement("span", null, Number(it.rating).toFixed(1), "/5"), it.updated && /* @__PURE__ */ React.createElement("span", null, "upd ", relativeTime(it.updated)))), /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, ok && /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "installed"), err && /* @__PURE__ */ React.createElement("span", { className: "badge error", title: result.message }, "failed"), /* @__PURE__ */ React.createElement("button", { className: "btn sm", onClick: () => peek(slug), title: "Peek manifest without installing" }, isOpen ? "Hide" : "Peek"), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => install(slug), disabled: !!installing }, isInstalling ? "Installing\u2026" : "Install"))), isOpen && /* @__PURE__ */ React.createElement("div", { style: {
      marginTop: 10,
      padding: "10px 12px",
      background: "var(--surface)",
      borderRadius: 5,
      border: "1px solid var(--border)"
    } }, !detail && !peekErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, textAlign: "center" } }, "loading\u2026"), peekErr && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, color: "var(--crimson)" } }, peekErr), detail && /* @__PURE__ */ React.createElement(React.Fragment, null, detail.description && /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11.5, lineHeight: 1.5, marginBottom: 8 } }, detail.description), Array.isArray(detail.tools) && detail.tools.length > 0 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-4", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Tools ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, detail.tools.length)), /* @__PURE__ */ React.createElement("div", { className: "col gap-4", style: { maxHeight: 160, overflow: "auto" } }, detail.tools.map((t, i) => /* @__PURE__ */ React.createElement("div", { key: i, className: "mono", style: {
      fontSize: 11,
      padding: "3px 6px",
      background: "var(--bg-2)",
      borderRadius: 4
    } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, t.name || t.id || `tool-${i}`), t.description && /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 8 } }, t.description))))), Array.isArray(detail.capabilities) && detail.capabilities.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "mt-8" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-4", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Capabilities"), /* @__PURE__ */ React.createElement("div", { className: "row gap-4", style: { flexWrap: "wrap" } }, detail.capabilities.map((c, i) => /* @__PURE__ */ React.createElement("span", { key: i, className: "badge plain", style: { fontSize: 10 } }, c)))), detail.homepage && /* @__PURE__ */ React.createElement("div", { className: "mt-8 dim mono", style: { fontSize: 10.5 } }, /* @__PURE__ */ React.createElement("span", null, "homepage: "), /* @__PURE__ */ React.createElement("a", { href: detail.homepage, target: "_blank", rel: "noreferrer", style: { color: "var(--rust)" } }, detail.homepage)))));
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
  const [inspect, setInspect] = useState(null);
  const [selected, setSelected] = useState(() => /* @__PURE__ */ new Set());
  const rows = resp && resp.approvals || [];
  React.useEffect(() => {
    if (selected.size === 0) return;
    const live = new Set(rows.map((r) => r.id));
    const next = new Set([...selected].filter((id) => live.has(id)));
    if (next.size !== selected.size) setSelected(next);
  }, [rows.map((r) => r.id).join(",")]);
  const toggle = (id) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };
  const toggleAll = (visible) => {
    setSelected((prev) => {
      if (visible.every((r) => prev.has(r.id))) {
        const next2 = new Set(prev);
        for (const r of visible) next2.delete(r.id);
        return next2;
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
      confirmLabel: verdict === "approve" ? `Approve ${ids.length}` : `Reject ${ids.length}`
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const id of ids) {
      try {
        await rhFetch(`/api/approvals/${id}/${verdict}`, { method: "POST" });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setSelected(/* @__PURE__ */ new Set());
    if (failCount > 0) toastErr(`${verdict}: ${okCount} ok / ${failCount} failed`);
    else toastOk(`${verdict === "approve" ? "Approved" : "Rejected"} ${okCount} request(s)`);
    refresh();
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Approvals ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", resp ? rows.length : "\u2026")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Inline keyboard buttons push to bound channels \xB7 decisions written to audit chain")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), selected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, selected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm primary", onClick: () => bulkDecide("approve") }, /* @__PURE__ */ React.createElement(I.check, null), " Approve ", selected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => bulkDecide("reject") }, /* @__PURE__ */ React.createElement(I.close, null), " Reject ", selected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, !resp && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "loading\u2026"), resp && rows.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, "No approvals waiting."), rows.length > 0 && /* @__PURE__ */ React.createElement(
    ApprovalsTable,
    {
      rows,
      onChange: refresh,
      onInspect: setInspect,
      selectable: true,
      selected,
      onToggle: toggle,
      onToggleAll: toggleAll
    }
  )), inspect && /* @__PURE__ */ React.createElement(ApprovalContextModal, { approval: inspect, onClose: () => setInspect(null), onChange: () => {
    setInspect(null);
    refresh();
  } }));
}
function ApprovalContextModal({ approval, onClose, onChange }) {
  useEscapeKey(onClose);
  const r = approval || {};
  const decide = async (verdict) => {
    try {
      await rhFetch(`/api/approvals/${r.id}/${verdict}`, { method: "POST" });
      toastOk(`Approval ${verdict}d`);
      onChange();
    } catch (e) {
      toastErr(`${verdict} failed: ${e.message || e}`);
    }
  };
  const payload = r.payload || r.details || r.context;
  const payloadStr = payload != null ? typeof payload === "string" ? payload : JSON.stringify(payload, null, 2) : null;
  const risk = (r.risk || "low").toLowerCase();
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: onClose }, /* @__PURE__ */ React.createElement("div", { className: "modal", onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("b", { className: "mono" }, "Approval \xB7 ", r.id), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11, marginTop: 2 } }, r.agent_name || r.agent || r.agent_id || "agent", " \xB7 requested ", relativeTime(r.requested_at || r.created_at))), /* @__PURE__ */ React.createElement("button", { className: "icon-btn", onClick: onClose }, /* @__PURE__ */ React.createElement(I.close, null))), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { className: "row gap-12 mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "badge " + (risk === "high" ? "error" : risk === "medium" ? "warn" : "idle") }, risk, " risk"), r.status && /* @__PURE__ */ React.createElement("span", { className: "badge plain" }, r.status)), /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Action"), /* @__PURE__ */ React.createElement("div", { className: "codebox mb-16", style: { whiteSpace: "pre-wrap" } }, r.action || "\u2014"), payloadStr && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Payload"), /* @__PURE__ */ React.createElement("pre", { className: "codebox mb-16", style: { maxHeight: 240 } }, payloadStr)), r.reason && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Reason"), /* @__PURE__ */ React.createElement("div", { className: "codebox mb-16", style: { whiteSpace: "pre-wrap" } }, r.reason))), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: onClose, style: { marginRight: "auto" } }, "Close"), /* @__PURE__ */ React.createElement("button", { className: "btn danger", onClick: () => decide("reject") }, "Reject"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => decide("approve") }, "Approve"))));
}
function AuditPage() {
  const route = useHashRoute();
  const focusHash = route && route.query && route.query.h ? route.query.h : "";
  const [windowSize, setWindowSize] = useState(() => {
    try {
      const stored = parseInt(localStorage.getItem("rh.panel.auditWindow") || "200", 10);
      return [50, 200, 500, 1e3].includes(stored) ? stored : 200;
    } catch (e) {
      return 200;
    }
  });
  const setWindow = (n) => {
    setWindowSize(n);
    try {
      localStorage.setItem("rh.panel.auditWindow", String(n));
    } catch (e) {
    }
  };
  const [audit, , refresh] = usePolling(`/api/audit/recent?n=${windowSize}`, 8e3);
  const [verify, verifyErr, verifyRefresh] = useApi("/api/audit/verify");
  const [verifyingNow, setVerifyingNow] = useState(false);
  const [lastVerifiedAt, setLastVerifiedAt] = useState(null);
  const forceReverify = async () => {
    setVerifyingNow(true);
    try {
      await verifyRefresh();
      setLastVerifiedAt(/* @__PURE__ */ new Date());
    } finally {
      setVerifyingNow(false);
    }
  };
  const verifyToastedRef = React.useRef(null);
  React.useEffect(() => {
    if (!verify) return;
    if (!lastVerifiedAt) return;
    const key = `${verify.valid}-${verify.entries}-${lastVerifiedAt.getTime()}`;
    if (verifyToastedRef.current === key) return;
    verifyToastedRef.current = key;
    if (verify.valid) {
      toastOk(`Chain verified \xB7 ${verify.entries || 0} entries`);
    } else {
      toastErr(`Chain MISMATCH: ${verify.error || "see banner"}`);
    }
  }, [verify, lastVerifiedAt]);
  const [q, setQ] = useState("");
  const [pulse, setPulse] = useState(focusHash);
  React.useEffect(() => {
    if (!pulse) return;
    const id = setTimeout(() => setPulse(""), 3500);
    return () => clearTimeout(id);
  }, [pulse]);
  React.useEffect(() => {
    setPulse(focusHash);
  }, [focusHash]);
  const rowRefs = React.useRef({});
  React.useEffect(() => {
    if (!pulse || !audit) return;
    const found = Object.entries(rowRefs.current).find(([h]) => h && h.startsWith(pulse));
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
    } catch (e) {
      toastErr(`copy failed: ${e.message || e}`);
    }
  };
  const [levelFilter, setLevelFilterState] = useState(() => {
    try {
      const stored = localStorage.getItem("rh.panel.auditLevel") || "all";
      return ["all", "info", "warn", "error"].includes(stored) ? stored : "all";
    } catch (e) {
      return "all";
    }
  });
  const setLevelFilter = (lvl) => {
    setLevelFilterState(lvl);
    try {
      localStorage.setItem("rh.panel.auditLevel", lvl);
    } catch (e) {
    }
  };
  const rawEntries = audit && audit.entries || [];
  const levelCounts = { info: 0, warn: 0, error: 0 };
  for (const e of rawEntries) {
    const lvl = auditLevelOf(e);
    levelCounts[lvl] = (levelCounts[lvl] || 0) + 1;
  }
  const levelFiltered = levelFilter === "all" ? rawEntries : rawEntries.filter((e) => auditLevelOf(e) === levelFilter);
  const [actorFilter, setActorFilterState] = useState(() => {
    try {
      return sessionStorage.getItem("rh.panel.auditActor") || "all";
    } catch (e) {
      return "all";
    }
  });
  const setActorFilter = (a) => {
    setActorFilterState(a);
    try {
      sessionStorage.setItem("rh.panel.auditActor", a);
    } catch (e) {
    }
  };
  const actorCountsAll = React.useMemo(() => {
    const m = /* @__PURE__ */ new Map();
    for (const e of levelFiltered) {
      const a = e.agent_name || e.agent_id || "kernel";
      m.set(a, (m.get(a) || 0) + 1);
    }
    return [...m.entries()].sort((a, b) => b[1] - a[1]);
  }, [levelFiltered]);
  const topActorsForChips = actorCountsAll.slice(0, 5);
  const actorFiltered = actorFilter === "all" ? levelFiltered : levelFiltered.filter((e) => (e.agent_name || e.agent_id || "kernel") === actorFilter);
  const ql = q.trim().toLowerCase();
  const entries = !ql ? actorFiltered : actorFiltered.filter(
    (e) => (e.action || "").toLowerCase().includes(ql) || (e.agent_name || "").toLowerCase().includes(ql) || (e.agent_id || "").toLowerCase().includes(ql) || (e.detail || "").toLowerCase().includes(ql) || (e.outcome || "").toLowerCase().includes(ql) || (e.hash || "").toLowerCase().includes(ql)
  );
  const actorCounts = {};
  for (const e of entries) {
    const a = e.agent_name || e.agent_id || "kernel";
    actorCounts[a] = (actorCounts[a] || 0) + 1;
  }
  const topActor = Object.entries(actorCounts).sort((a, b) => b[1] - a[1])[0];
  const highlight = React.useCallback((text) => {
    if (!ql || !text) return text;
    const s = String(text);
    const idx = s.toLowerCase().indexOf(ql);
    if (idx < 0) return text;
    return /* @__PURE__ */ React.createElement(React.Fragment, null, s.slice(0, idx), /* @__PURE__ */ React.createElement("mark", { className: "audit-match" }, s.slice(idx, idx + ql.length)), s.slice(idx + ql.length));
  }, [ql]);
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Audit log"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Merkle hash chain \xB7 ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/data/audit.jsonl"), " \xB7 replayed on boot")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("div", { className: "search-field", style: { minWidth: 260 } }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("input", { placeholder: "filter by action / actor / hash / detail\u2026", value: q, onChange: (e) => setQ(e.target.value) }), q && /* @__PURE__ */ React.createElement("button", { className: "kbd", onClick: () => setQ(""), style: { cursor: "pointer" } }, "clear")), /* @__PURE__ */ React.createElement("div", { className: "seg", title: "Filter by classified severity" }, /* @__PURE__ */ React.createElement("button", { className: levelFilter === "all" ? "on" : "", onClick: () => setLevelFilter("all") }, "all \xB7 ", rawEntries.length), /* @__PURE__ */ React.createElement("button", { className: levelFilter === "info" ? "on" : "", onClick: () => setLevelFilter("info") }, "info \xB7 ", levelCounts.info), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: levelFilter === "warn" ? "on" : "",
      style: levelFilter === "warn" ? { color: "var(--amber)" } : {},
      onClick: () => setLevelFilter("warn")
    },
    "warn \xB7 ",
    levelCounts.warn
  ), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: levelFilter === "error" ? "on" : "",
      style: levelFilter === "error" ? { color: "var(--crimson)" } : {},
      onClick: () => setLevelFilter("error")
    },
    "error \xB7 ",
    levelCounts.error
  )), /* @__PURE__ */ React.createElement("div", { className: "seg", title: "Audit window size" }, [50, 200, 500, 1e3].map((n) => /* @__PURE__ */ React.createElement("button", { key: n, className: windowSize === n ? "on" : "", onClick: () => setWindow(n) }, n))), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    refresh();
    verifyRefresh();
  } }, /* @__PURE__ */ React.createElement(I.refresh, null)), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn ghost",
      onClick: forceReverify,
      disabled: verifyingNow,
      title: "Force re-verify the audit chain end-to-end and toast the result"
    },
    /* @__PURE__ */ React.createElement(I.shield, null),
    " ",
    verifyingNow ? "Verifying\u2026" : "Verify chain"
  ), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn ghost",
      title: ql ? `Export ${entries.length} filtered entries as CSV` : "Export the loaded audit window as CSV",
      onClick: () => {
        if (entries.length === 0) {
          toastErr("Nothing to export");
          return;
        }
        const rows = entries.map((e) => ({
          seq: e.seq != null ? e.seq : "",
          timestamp: e.timestamp || e.created_at || "",
          agent_id: e.agent_id || "",
          agent_name: e.agent_name || "",
          action: e.action || "",
          outcome: e.outcome || "",
          detail: (e.detail || "").replace(/\s+/g, " ").trim(),
          hash: e.hash || ""
        }));
        const csv = rowsToCsv(rows);
        const tag = ql ? "filtered" : "window";
        downloadBlob(
          `rustyhand-audit-${tag}-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.csv`,
          csv,
          "text/csv"
        );
      }
    },
    /* @__PURE__ */ React.createElement(I.download, null),
    " CSV"
  ), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => {
    if (!audit) return;
    downloadBlob(
      `rustyhand-audit-${(/* @__PURE__ */ new Date()).toISOString().slice(0, 10)}.json`,
      JSON.stringify(audit, null, 2),
      "application/json"
    );
  } }, /* @__PURE__ */ React.createElement(I.download, null), " JSON"))), topActorsForChips.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "row gap-6 mb-8", style: { flexWrap: "wrap" } }, /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase", alignSelf: "center", marginRight: 4 } }, "Actor"), /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn sm " + (actorFilter === "all" ? "primary" : "ghost"),
      onClick: () => setActorFilter("all")
    },
    "all \xB7 ",
    levelFiltered.length
  ), topActorsForChips.map(([name, n]) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: name,
      className: "btn sm " + (actorFilter === name ? "primary" : "ghost"),
      onClick: () => setActorFilter(actorFilter === name ? "all" : name),
      title: `Restrict the audit list to entries by ${name}`
    },
    name,
    " \xB7 ",
    n
  ))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Chain head"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, color: "var(--rust)", wordBreak: "break-all" } }, audit && audit.tip_hash ? String(audit.tip_hash).slice(0, 16) : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, "depth ", audit ? audit.total != null ? audit.total.toLocaleString() : "\u2014" : "\u2026"), /* @__PURE__ */ React.createElement("div", { className: "divider" }), verify ? /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "row gap-6" }, verify.valid ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, /* @__PURE__ */ React.createElement(I.check, null), " verified") : /* @__PURE__ */ React.createElement("span", { className: "badge error" }, /* @__PURE__ */ React.createElement(I.warn, null), " mismatch"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, (verify.entries || []).length || verify.total || 0, " entries")), lastVerifiedAt && /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 10.5 } }, "last manual check: ", lastVerifiedAt.toLocaleTimeString("en-GB", { hour12: false })), !verify.valid && verify.error && /* @__PURE__ */ React.createElement("div", { className: "mono mt-4", style: { fontSize: 10.5, color: "var(--crimson)", wordBreak: "break-word" } }, verify.error)) : /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, verifyErr || "verifying\u2026")), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Loaded window"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 20 } }, entries.length, " ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { fontSize: 13 } }, "entries")), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, "from /api/audit/recent?n=", windowSize)), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Top actor"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14 } }, topActor ? topActor[0] : "\u2014"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, topActor ? `${topActor[1]} entries (in window)` : "no activity")), /* @__PURE__ */ React.createElement("div", { className: "col-3 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Warning"), /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, color: verify && verify.warning ? "var(--amber)" : "var(--live)" } }, verify && verify.warning ? "see below" : "none"), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, verify && verify.warning ? verify.warning : "audit chain stable"))), /* @__PURE__ */ React.createElement("div", { className: "card flush mt-16" }, /* @__PURE__ */ React.createElement("div", { className: "card-head" }, /* @__PURE__ */ React.createElement("span", null, "Chain \xB7 most recent"), /* @__PURE__ */ React.createElement("span", { className: "mono dim" }, "descending")), /* @__PURE__ */ React.createElement("div", null, !audit && Array.from({ length: 6 }).map((_, i) => /* @__PURE__ */ React.createElement("div", { key: `sa-${i}`, className: "merkle-row" }, /* @__PURE__ */ React.createElement("div", { className: "chain" }), /* @__PURE__ */ React.createElement(Skel, { w: 50, h: 10 }), /* @__PURE__ */ React.createElement("span", null, /* @__PURE__ */ React.createElement(Skel, { w: "50%", h: 10 }), /* @__PURE__ */ React.createElement("div", { style: { marginTop: 4 } }, /* @__PURE__ */ React.createElement(Skel, { w: "70%", h: 9 }))), /* @__PURE__ */ React.createElement(Skel, { w: 70, h: 10 }), /* @__PURE__ */ React.createElement(Skel, { w: 50, h: 10 }))), audit && entries.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "16px 14px", fontSize: 12 } }, ql ? `No entries matching "${q}" in the loaded window of ${rawEntries.length}.` : "No audit entries yet."), entries.map((a) => {
    const hash = a.hash ? String(a.hash).slice(0, 12) : "\u2014";
    const pulsing = pulse && a.hash && String(a.hash).startsWith(pulse);
    return /* @__PURE__ */ React.createElement(
      "div",
      {
        key: a.hash || a.seq,
        ref: (el) => {
          if (a.hash) rowRefs.current[a.hash] = el;
        },
        className: "merkle-row" + (pulsing ? " audit-pulse" : ""),
        onClick: () => copyDeepLink(a.hash),
        title: "Click to copy deep-link to this entry"
      },
      /* @__PURE__ */ React.createElement("div", { className: "chain" }),
      /* @__PURE__ */ React.createElement("span", { className: "time" }, formatTime(a.timestamp)),
      /* @__PURE__ */ React.createElement("span", null, /* @__PURE__ */ React.createElement("span", { className: "action" }, highlight(a.action)), " ", /* @__PURE__ */ React.createElement("span", { className: "dim" }, "\xB7"), " ", /* @__PURE__ */ React.createElement("span", { className: "actor" }, highlight(a.agent_name || a.agent_id || "kernel")), /* @__PURE__ */ React.createElement("div", { className: "dim", style: { fontSize: 11, marginTop: 2 } }, highlight(a.detail || a.outcome || ""))),
      /* @__PURE__ */ React.createElement("span", { className: "hash" }, highlight(hash)),
      /* @__PURE__ */ React.createElement("span", { className: "dim" }, "seq ", a.seq)
    );
  }))));
}
function SettingsPage() {
  const [providersResp, , refreshProviders] = usePolling("/api/providers", 3e4);
  const [config] = useApi("/api/config");
  const [health] = useApi("/api/health/detail");
  const [onboarding] = useApi("/api/onboarding");
  const [usersResp] = usePolling("/api/auth/users", 3e4);
  const [editing, setEditing] = useState(null);
  const providers = providersResp && providersResp.providers || [];
  const users = usersResp && usersResp.users || [];
  const apiListen = config && (config.api_listen || config.api && config.api.listen) || "\u2014";
  const proxy = config && (config.proxy_url || config.proxy && config.proxy.url) || null;
  const version = health && health.version || "0.7.75";
  const uptime = health && health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "\u2014";
  const agentCount = health && health.agent_count != null ? health.agent_count : "\u2014";
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Settings"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Config at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " \xB7 50+ fields with serde defaults \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/config"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn ghost",
      title: "Download a copy of config.toml with secrets redacted",
      onClick: async () => {
        try {
          const text = await rhFetch("/api/config/export");
          const stamp = (/* @__PURE__ */ new Date()).toISOString().slice(0, 10);
          downloadBlob(`rustyhand-config-${stamp}.toml`, text, "text/plain");
          toastOk("Config exported (secrets redacted)");
        } catch (e) {
          toastErr(`export failed: ${e.message || e}`);
        }
      }
    },
    /* @__PURE__ */ React.createElement(I.download, null),
    " Export config.toml"
  ))), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-8 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "LLM providers"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, providers.length, " loaded \xB7 auto-probe at boot")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Provider"), /* @__PURE__ */ React.createElement("th", null, "Env var"), /* @__PURE__ */ React.createElement("th", null, "State"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Models"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !providersResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 5, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), providers.map((p) => {
    const auth = (p.auth_status || "").toLowerCase();
    let badge;
    if (auth === "ok") badge = /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "set");
    else if (auth === "missing" && p.key_required === false) badge = /* @__PURE__ */ React.createElement("span", { className: "badge sky" }, "local");
    else if (p.id === "mock" || auth === "fallback") badge = /* @__PURE__ */ React.createElement("span", { className: "badge demo" }, "fallback");
    else if (auth === "invalid") badge = /* @__PURE__ */ React.createElement("span", { className: "badge error" }, "invalid");
    else badge = /* @__PURE__ */ React.createElement("span", { className: "badge idle" }, "not set");
    return /* @__PURE__ */ React.createElement("tr", { key: p.id || p.name }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, p.display_name || p.id || p.name), /* @__PURE__ */ React.createElement("td", { className: "mono muted" }, p.api_key_env || "\u2014"), /* @__PURE__ */ React.createElement("td", null, badge), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, p.model_count != null ? p.model_count : "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setEditing(p) }, "Edit")));
  }))), /* @__PURE__ */ React.createElement("div", { className: "row between mb-12 mt-16" }, /* @__PURE__ */ React.createElement("span", { className: "mono dim", style: { fontSize: 11, letterSpacing: ".12em", textTransform: "uppercase" } }, "Authorized users"), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 11 } }, users.length, " configured \xB7 read-only here")), /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", null, "Name"), /* @__PURE__ */ React.createElement("th", null, "User ID"), /* @__PURE__ */ React.createElement("th", null, "Role"))), /* @__PURE__ */ React.createElement("tbody", null, !usersResp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 3, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), usersResp && users.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 3, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No RBAC users \u2014 auth is in localhost-only mode. Add users in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " under ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "[[auth.users]]"), ".")), users.map((u) => {
    const role = (u.role || "viewer").toLowerCase();
    return /* @__PURE__ */ React.createElement("tr", { key: u.user_id || u.name }, /* @__PURE__ */ React.createElement("td", { className: "mono" }, u.name || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono muted", style: { maxWidth: 200, overflow: "hidden", textOverflow: "ellipsis" } }, u.user_id || "\u2014"), /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement("span", { className: "badge " + (role === "owner" || role === "admin" ? "violet" : role === "operator" ? "live" : "plain") }, role)));
  })))), /* @__PURE__ */ React.createElement("div", { className: "col-4 col" }, /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "API"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "listen"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, apiListen), /* @__PURE__ */ React.createElement("dt", null, "auth"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, config && config.bearer_token ? "\u2022\u2022\u2022\u2022\u2022\u2022\u2022" : "localhost-only"), /* @__PURE__ */ React.createElement("dt", null, "ws origins"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, config && (config.allowed_ws_origins || []).join(", ") || "localhost"), /* @__PURE__ */ React.createElement("dt", null, "proxy"), /* @__PURE__ */ React.createElement("dd", { className: proxy ? "mono" : "dim" }, proxy || "none"))), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Demo mode"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "active"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.demo_mode ? "yes" : "no" : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "provider"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, onboarding ? onboarding.provider : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "api_key"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.api_key_set ? "set" : "missing" : "\u2026"), /* @__PURE__ */ React.createElement("dt", null, "seeded"), /* @__PURE__ */ React.createElement("dd", null, onboarding && onboarding.demo_seeded ? "yes" : "no"), /* @__PURE__ */ React.createElement("dt", null, "agents"), /* @__PURE__ */ React.createElement("dd", null, onboarding ? onboarding.agent_count : "\u2026")), /* @__PURE__ */ React.createElement("div", { className: "dim mt-8", style: { fontSize: 11 } }, "set ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "RUSTYHAND_DISABLE_DEMO_MODE=1"), " to fall back to NullDriver"), onboarding && onboarding.demo_mode && /* @__PURE__ */ React.createElement("div", { className: "row gap-6 mt-8" }, /* @__PURE__ */ React.createElement(
    "button",
    {
      className: "btn sm",
      title: "Removes the .rustyhand_demo_seeded marker so the daemon re-seeds the welcome agent + sample workflow on next start",
      onClick: async () => {
        const ok = await confirmDialog({
          title: "Reset demo seed?",
          message: "Removes the demo-seed marker. On the next daemon restart, the welcome agent, sample workflow, trigger and disabled cron job will be re-created. Existing resources stay in place \u2014 you may want to delete them first to avoid duplicates.",
          confirmLabel: "Remove marker"
        });
        if (!ok) return;
        try {
          const r = await rhFetch("/api/onboarding/reset-demo", { method: "POST" });
          toastOk(r.message || "Marker removed. Restart the daemon to re-seed.");
        } catch (err) {
          toastErr(`reset failed: ${err.message || err}`);
        }
      }
    },
    /* @__PURE__ */ React.createElement(I.refresh, null),
    " Re-seed on restart"
  ))), /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Build"), /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "version"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, version), /* @__PURE__ */ React.createElement("dt", null, "agents"), /* @__PURE__ */ React.createElement("dd", null, agentCount), /* @__PURE__ */ React.createElement("dt", null, "uptime"), /* @__PURE__ */ React.createElement("dd", null, uptime), /* @__PURE__ */ React.createElement("dt", null, "panics"), /* @__PURE__ */ React.createElement("dd", null, health && health.panic_count != null ? health.panic_count : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "restarts"), /* @__PURE__ */ React.createElement("dd", null, health && health.restart_count != null ? health.restart_count : "\u2014"))), /* @__PURE__ */ React.createElement(LogLevelCard, { config }))), editing && /* @__PURE__ */ React.createElement(ProviderKeyModal, { provider: editing, onClose: () => setEditing(null), onSaved: () => {
    setEditing(null);
    refreshProviders();
  } }));
}
function LogLevelCard({ config }) {
  const current = config && config.log_level || "info";
  const [busy, setBusy] = useState(false);
  const apply = async (level) => {
    setBusy(true);
    try {
      await rhFetch("/api/config/set", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ path: "log_level", value: level })
      });
      toastOk(`Log level set to ${level}. Restart daemon for effect.`);
    } catch (e) {
      toastErr(`set failed: ${e.message || e}`);
    } finally {
      setBusy(false);
    }
  };
  return /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Log verbosity", /* @__PURE__ */ React.createElement(Tip, null, "Persisted to config.toml.log_level. Takes effect on daemon restart \u2014 the running tracing EnvFilter is fixed at boot. Use trace/debug for troubleshooting, info for normal operation.")), /* @__PURE__ */ React.createElement("div", { className: "seg", style: { flexWrap: "wrap" } }, ["error", "warn", "info", "debug", "trace"].map((l) => /* @__PURE__ */ React.createElement(
    "button",
    {
      key: l,
      className: current === l ? "on" : "",
      disabled: busy,
      onClick: () => apply(l)
    },
    l
  ))), /* @__PURE__ */ React.createElement("div", { className: "dim mt-8", style: { fontSize: 11 } }, "Current: ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, current)));
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
    if (!await confirmDialog({ title: "Delete API key", message: `Delete API key for ${name}?`, danger: true, confirmLabel: "Delete" })) return;
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
  const rawSessions = resp && resp.sessions || [];
  const [q, setQState] = useState(() => {
    try {
      return sessionStorage.getItem("rh.panel.memoryQ") || "";
    } catch (e) {
      return "";
    }
  });
  const setQ = (v) => {
    setQState(v);
    try {
      sessionStorage.setItem("rh.panel.memoryQ", v || "");
    } catch (e) {
    }
  };
  const ql = q.trim().toLowerCase();
  const sessions = !ql ? rawSessions : rawSessions.filter(
    (s) => (s.label || "").toLowerCase().includes(ql) || (s.agent_name || "").toLowerCase().includes(ql) || (s.agent_id || "").toLowerCase().includes(ql) || String(s.session_id || "").toLowerCase().includes(ql)
  );
  const [labelEditing, setLabelEditing] = useState(null);
  const [labelDraft, setLabelDraft] = useState("");
  const fileInputRef = useRef(null);
  const [selected, setSelected] = useState(() => /* @__PURE__ */ new Set());
  React.useEffect(() => {
    if (selected.size === 0) return;
    const visible = new Set(sessions.map((s) => s.session_id));
    const next = new Set([...selected].filter((id) => visible.has(id)));
    if (next.size !== selected.size) setSelected(next);
  }, [sessions]);
  const toggle = (id) => {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    setSelected(next);
  };
  const toggleAll = () => {
    if (selected.size === sessions.length && sessions.length > 0) setSelected(/* @__PURE__ */ new Set());
    else setSelected(new Set(sessions.map((s) => s.session_id)));
  };
  const bulkDelete = async () => {
    if (selected.size === 0) return;
    const ok = await confirmDialog({
      title: `Delete ${selected.size} session(s)?`,
      message: "This cannot be undone. Sessions and their messages will be permanently removed.",
      danger: true,
      confirmLabel: `Delete ${selected.size}`
    });
    if (!ok) return;
    let okCount = 0;
    let failCount = 0;
    for (const id of selected) {
      try {
        await rhFetch(`/api/sessions/${encodeURIComponent(id)}`, { method: "DELETE" });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setSelected(/* @__PURE__ */ new Set());
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
        parts.push(`# Session ${String(id).slice(0, 8)}

${md}

---
`);
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    if (parts.length === 0) {
      toastErr("Bulk export produced no content");
      return;
    }
    const stamp = (/* @__PURE__ */ new Date()).toISOString().slice(0, 10);
    downloadBlob(`rustyhand-sessions-${stamp}.md`, parts.join(""), "text/markdown");
    if (failCount > 0) toastErr(`Exported ${okCount}, failed ${failCount}`);
    else toastOk(`Exported ${okCount} session(s)`);
  };
  const remove = async (id) => {
    if (!await confirmDialog({ title: "Delete session", message: `Delete session ${String(id).slice(0, 8)}? This cannot be undone.`, danger: true, confirmLabel: "Delete" })) return;
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
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Memory ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", ql ? `${sessions.length} of ${rawSessions.length}` : sessions.length, " session(s)")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "SQLite-backed sessions \xB7 backup at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/memory/export"), " \xB7 restore at ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/memory/import"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("div", { className: "search-field", style: { minWidth: 240 } }, /* @__PURE__ */ React.createElement(I.search, null), /* @__PURE__ */ React.createElement("input", { placeholder: "filter by label / agent / session id\u2026", value: q, onChange: (e) => setQ(e.target.value) }), q && /* @__PURE__ */ React.createElement("button", { className: "kbd", onClick: () => setQ(""), style: { cursor: "pointer" } }, "clear")), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)), selected.size > 0 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: bulkExport, title: "Export selected sessions as one markdown file" }, /* @__PURE__ */ React.createElement(I.download, null), " Export ", selected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: bulkDelete, title: "Delete the selected sessions" }, /* @__PURE__ */ React.createElement(I.trash, null), " Delete ", selected.size)), /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => fileInputRef.current && fileInputRef.current.click() }, /* @__PURE__ */ React.createElement(I.copy, null), " Restore"), /* @__PURE__ */ React.createElement(
    "input",
    {
      ref: fileInputRef,
      type: "file",
      accept: "application/json,.json",
      style: { display: "none" },
      onChange: (e) => importMemory(e.target.files && e.target.files[0])
    }
  ), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: backupMemory }, /* @__PURE__ */ React.createElement(I.download, null), " Backup"))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: sessions.length > 0 && selected.size === sessions.length,
      onChange: toggleAll,
      title: "Select all on this page"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "Session"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Label"), /* @__PURE__ */ React.createElement("th", { className: "right" }, "Messages"), /* @__PURE__ */ React.createElement("th", null, "Created"), /* @__PURE__ */ React.createElement("th", null, "Updated"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !resp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), resp && sessions.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 8, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No sessions yet.")), sessions.map((s) => /* @__PURE__ */ React.createElement("tr", { key: s.session_id, style: selected.has(s.session_id) ? { background: "var(--bg-2)" } : null }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: selected.has(s.session_id),
      onChange: () => toggle(s.session_id)
    }
  )), /* @__PURE__ */ React.createElement("td", { className: "mono", style: { maxWidth: 120, overflow: "hidden", textOverflow: "ellipsis" } }, String(s.session_id).slice(0, 8)), /* @__PURE__ */ React.createElement("td", { className: "mono" }, s.agent_name || s.agent_id || "\u2014"), /* @__PURE__ */ React.createElement("td", null, labelEditing === s.session_id ? /* @__PURE__ */ React.createElement("span", { className: "row gap-4" }, /* @__PURE__ */ React.createElement(
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
  const connectedByName = new Map(connected.map((c) => [c.name, c]));
  const [expanded, setExpanded] = useState(() => /* @__PURE__ */ new Set());
  const toggleExpand = (name) => {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "MCP servers ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", configured.length, " configured \xB7 ", connected.length, " connected")), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Model-Context-Protocol bridges \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/mcp/servers"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), !resp && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px", fontSize: 12 } }, "loading\u2026"), resp && configured.length === 0 && /* @__PURE__ */ React.createElement("div", { className: "muted mono", style: { padding: "24px", fontSize: 12 } }, "No MCP servers configured. Add them in ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "~/.rustyhand/config.toml"), " under ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "[[mcp.servers]]"), "."), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, configured.map((s) => {
    const live = connectedByName.get(s.name);
    const isConnected = !!live;
    const transport = s.transport || {};
    const tools = live && Array.isArray(live.tools) ? live.tools : [];
    const isOpen = expanded.has(s.name);
    return /* @__PURE__ */ React.createElement("div", { key: s.name, className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "row between mb-12" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 14, fontWeight: 500 } }, s.name), /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, transport.type || "\u2014", " \xB7 ", s.timeout_secs ? `${s.timeout_secs}s timeout` : "no timeout")), /* @__PURE__ */ React.createElement("span", { className: "badge " + (isConnected ? "live" : "idle") }, isConnected ? "connected" : "idle")), /* @__PURE__ */ React.createElement("div", { className: "kv" }, transport.type === "stdio" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "command"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, transport.command), /* @__PURE__ */ React.createElement("dt", null, "args"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, (transport.args || []).join(" ") || "\u2014")), transport.type === "sse" && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "url"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, transport.url)), s.env && Object.keys(s.env).length > 0 && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "env"), /* @__PURE__ */ React.createElement("dd", { className: "mono dim" }, Object.keys(s.env).join(", "))), isConnected && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "tools"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, tools.length, tools.length > 0 && /* @__PURE__ */ React.createElement(
      "button",
      {
        className: "kbd",
        style: { marginLeft: 6, cursor: "pointer" },
        onClick: () => toggleExpand(s.name)
      },
      isOpen ? "hide" : "show"
    )))), isOpen && tools.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "col gap-4 mt-8", style: {
      maxHeight: 240,
      overflow: "auto",
      background: "var(--bg-2)",
      borderRadius: 6,
      padding: "8px 10px"
    } }, tools.map((t, i) => /* @__PURE__ */ React.createElement("div", { key: t.name || i, style: {
      fontFamily: "var(--ff-mono)",
      fontSize: 11.5,
      padding: "3px 0",
      borderBottom: i < tools.length - 1 ? "1px solid var(--border)" : "none"
    } }, /* @__PURE__ */ React.createElement("span", { style: { color: "var(--rust)" } }, t.name), t.description && /* @__PURE__ */ React.createElement("div", { className: "dim", style: {
      marginTop: 2,
      whiteSpace: "nowrap",
      overflow: "hidden",
      textOverflow: "ellipsis"
    } }, t.description)))));
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
function HealthPage() {
  const [health, , refreshHealth] = usePolling("/api/health/detail", 1e4);
  const [audit, , refreshAudit] = usePolling("/api/audit/verify", 3e4);
  const [net] = usePolling("/api/network/status", 15e3);
  const [mcp] = usePolling("/api/mcp/servers", 3e4);
  const [onb] = usePolling("/api/onboarding", 3e4);
  const [usage] = usePolling("/api/usage/daily", 2e4);
  const allWarns = health && health.config_warnings || [];
  const refresh = () => {
    refreshHealth();
    refreshAudit();
  };
  let overall = "green", overallLabel = "all systems normal";
  if (health) {
    if (health.status !== "ok") {
      overall = "red";
      overallLabel = `kernel status: ${health.status}`;
    } else if (audit && audit.valid === false) {
      overall = "red";
      overallLabel = "audit chain mismatch";
    } else if (health.panic_count > 0 || health.restart_count > 0) {
      overall = "amber";
      overallLabel = `${health.panic_count || 0} panic(s), ${health.restart_count || 0} restart(s)`;
    } else if (allWarns.length > 0) {
      overall = "amber";
      overallLabel = `${allWarns.length} config warning(s)`;
    }
  } else {
    overallLabel = "checking\u2026";
  }
  const dotCls = overall === "red" ? "err" : overall === "amber" ? "warn" : "live";
  const badgeCls = overall === "red" ? "error" : overall === "amber" ? "warn" : "live";
  const todayCost = usage && (usage.cost_usd_today || usage.cost_usd || usage.total_cost_usd) || 0;
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Health"), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Live diagnostics \xB7 health/detail + audit/verify + network/status + mcp/servers")), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), /* @__PURE__ */ React.createElement("div", { className: "banner mb-12", style: {
    borderColor: overall === "red" ? "oklch(0.66 0.18 25 / .35)" : overall === "amber" ? "oklch(0.78 0.14 88 / .35)" : "oklch(0.66 0.15 155 / .35)"
  } }, /* @__PURE__ */ React.createElement("span", { className: "dot " + dotCls }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, overall === "green" ? "HEALTHY" : overall === "amber" ? "DEGRADED" : "ATTENTION"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, overallLabel), /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 10.5, marginLeft: "auto" } }, "polls 10s")), /* @__PURE__ */ React.createElement("div", { className: "grid-12" }, /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Kernel"), !health && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), health && /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "status"), /* @__PURE__ */ React.createElement("dd", null, /* @__PURE__ */ React.createElement("span", { className: "badge " + badgeCls }, health.status)), /* @__PURE__ */ React.createElement("dt", null, "version"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, health.version), /* @__PURE__ */ React.createElement("dt", null, "uptime"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, health.uptime_seconds != null ? formatUptime(health.uptime_seconds) : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "database"), /* @__PURE__ */ React.createElement("dd", null, health.database === "connected" ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "connected") : /* @__PURE__ */ React.createElement("span", { className: "badge error" }, health.database)), /* @__PURE__ */ React.createElement("dt", null, "agents"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, health.agent_count), /* @__PURE__ */ React.createElement("dt", null, "panics"), /* @__PURE__ */ React.createElement("dd", { className: "num mono", style: { color: health.panic_count > 0 ? "var(--crimson)" : void 0 } }, health.panic_count), /* @__PURE__ */ React.createElement("dt", null, "restarts"), /* @__PURE__ */ React.createElement("dd", { className: "num mono", style: { color: health.restart_count > 0 ? "var(--amber)" : void 0 } }, health.restart_count))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Audit chain"), !audit && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "verifying\u2026"), audit && /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "integrity"), /* @__PURE__ */ React.createElement("dd", null, audit.valid ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, /* @__PURE__ */ React.createElement(I.check, null), " verified") : /* @__PURE__ */ React.createElement("span", { className: "badge error" }, /* @__PURE__ */ React.createElement(I.warn, null), " mismatch")), /* @__PURE__ */ React.createElement("dt", null, "entries"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, (audit.entries || []).length || audit.total || 0), /* @__PURE__ */ React.createElement("dt", null, "tip hash"), /* @__PURE__ */ React.createElement("dd", { className: "mono", style: { wordBreak: "break-all", fontSize: 11 } }, audit.tip_hash ? String(audit.tip_hash).slice(0, 24) + "\u2026" : "\u2014"), audit.warning && /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("dt", null, "warning"), /* @__PURE__ */ React.createElement("dd", { className: "mono dim", style: { fontSize: 11 } }, audit.warning)))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Network"), !net && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), net && /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "state"), /* @__PURE__ */ React.createElement("dd", null, net.enabled ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "enabled") : /* @__PURE__ */ React.createElement("span", { className: "badge idle" }, "disabled")), /* @__PURE__ */ React.createElement("dt", null, "listen"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, net.listen_address || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "node id"), /* @__PURE__ */ React.createElement("dd", { className: "mono", style: { wordBreak: "break-all", fontSize: 11 } }, net.node_id ? String(net.node_id).slice(0, 24) + "\u2026" : "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "peers"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, net.connected_peers != null ? net.connected_peers : "\u2014", net.total_peers != null && /* @__PURE__ */ React.createElement("span", { className: "dim" }, " / ", net.total_peers)))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "MCP servers"), !mcp && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), mcp && /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "configured"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, mcp.total_configured != null ? mcp.total_configured : (mcp.configured || []).length), /* @__PURE__ */ React.createElement("dt", null, "connected"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, mcp.total_connected != null ? mcp.total_connected : (mcp.connected || []).length, mcp.total_configured > 0 && mcp.total_connected === 0 && /* @__PURE__ */ React.createElement("span", { className: "badge warn", style: { marginLeft: 6 } }, "none live")), /* @__PURE__ */ React.createElement("dt", null, "tools"), /* @__PURE__ */ React.createElement("dd", { className: "num mono" }, (mcp.connected || []).reduce((s, c) => s + (c.tools_count || (c.tools || []).length || 0), 0)))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Mode"), !onb && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), onb && /* @__PURE__ */ React.createElement("div", { className: "kv" }, /* @__PURE__ */ React.createElement("dt", null, "demo"), /* @__PURE__ */ React.createElement("dd", null, onb.demo_mode ? /* @__PURE__ */ React.createElement("span", { className: "badge demo" }, "yes \xB7 ", onb.provider) : /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "no \xB7 ", onb.provider)), /* @__PURE__ */ React.createElement("dt", null, "api key"), /* @__PURE__ */ React.createElement("dd", null, onb.api_key_set ? /* @__PURE__ */ React.createElement("span", { className: "badge live" }, "set") : /* @__PURE__ */ React.createElement("span", { className: "badge idle" }, "missing")), /* @__PURE__ */ React.createElement("dt", null, "model"), /* @__PURE__ */ React.createElement("dd", { className: "mono" }, onb.model || "\u2014"), /* @__PURE__ */ React.createElement("dt", null, "seeded"), /* @__PURE__ */ React.createElement("dd", null, onb.demo_seeded ? "yes" : "no"))), /* @__PURE__ */ React.createElement("div", { className: "col-6 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Today's spend"), !usage && /* @__PURE__ */ React.createElement("div", { className: "dim mono", style: { fontSize: 11 } }, "loading\u2026"), usage && /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "mono", style: { fontSize: 22, color: todayCost > 5 ? "var(--amber)" : "var(--rust)" } }, "$", Number(todayCost).toFixed(4)), /* @__PURE__ */ React.createElement("div", { className: "dim mono mt-4", style: { fontSize: 11 } }, "Tokens: ", Number((usage.input_tokens || 0) + (usage.output_tokens || 0)).toLocaleString(), " \xB7", " ", "Calls: ", Number(usage.tool_calls || 0).toLocaleString()))), allWarns.length > 0 && /* @__PURE__ */ React.createElement("div", { className: "col-12 card" }, /* @__PURE__ */ React.createElement("div", { className: "muted mono mb-8", style: { fontSize: 10.5, letterSpacing: ".12em", textTransform: "uppercase" } }, "Config warnings ", /* @__PURE__ */ React.createElement("span", { className: "dim", style: { marginLeft: 6 } }, allWarns.length)), /* @__PURE__ */ React.createElement("div", { className: "col gap-4" }, allWarns.map((w, i) => /* @__PURE__ */ React.createElement("div", { key: i, className: "row gap-6", style: {
    padding: "5px 8px",
    background: "var(--bg-2)",
    borderRadius: 5,
    fontFamily: "var(--ff-mono)",
    fontSize: 11.5
  } }, /* @__PURE__ */ React.createElement("span", { className: "dot warn" }), /* @__PURE__ */ React.createElement("span", null, typeof w === "string" ? w : w.message || JSON.stringify(w))))))));
}
function BindingsPage() {
  const [resp, fetchErr, refresh] = usePolling("/api/bindings", 3e4);
  const bindings = resp && resp.bindings || [];
  const [selected, setSelected] = useState(() => /* @__PURE__ */ new Set());
  React.useEffect(() => {
    setSelected(/* @__PURE__ */ new Set());
  }, [resp && bindings.length]);
  const remove = async (index) => {
    if (!await confirmDialog({ title: "Remove binding", message: `Remove binding #${index}?`, danger: true, confirmLabel: "Remove" })) return;
    try {
      await rhFetch(`/api/bindings/${index}`, { method: "DELETE" });
      toastOk("Binding removed");
      refresh();
    } catch (e) {
      toastErr(`Remove failed: ${e.message || e}`);
    }
  };
  const toggle = (i) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (next.has(i)) next.delete(i);
      else next.add(i);
      return next;
    });
  };
  const toggleAll = () => {
    setSelected((prev) => {
      if (bindings.length === 0) return prev;
      if (prev.size === bindings.length) return /* @__PURE__ */ new Set();
      return new Set(bindings.map((_, i) => i));
    });
  };
  const bulkRemove = async () => {
    const ids = [...selected].sort((a, b) => b - a);
    if (ids.length === 0) return;
    const ok = await confirmDialog({
      title: `Remove ${ids.length} binding(s)?`,
      message: "Bindings are positional \u2014 this loops indices from high to low so partial failure can't desync the remaining selection.",
      danger: true,
      confirmLabel: `Remove ${ids.length}`
    });
    if (!ok) return;
    let okCount = 0, failCount = 0;
    for (const i of ids) {
      try {
        await rhFetch(`/api/bindings/${i}`, { method: "DELETE" });
        okCount++;
      } catch (_) {
        failCount++;
      }
    }
    setSelected(/* @__PURE__ */ new Set());
    if (failCount > 0) toastErr(`Removed ${okCount}, failed ${failCount}`);
    else toastOk(`Removed ${okCount} binding(s)`);
    refresh();
  };
  return /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("div", { className: "page-head" }, /* @__PURE__ */ React.createElement("div", null, /* @__PURE__ */ React.createElement("h1", { className: "page-title" }, "Bindings ", /* @__PURE__ */ React.createElement("span", { className: "dim mono", style: { fontSize: 14 } }, "\xB7 ", bindings.length)), /* @__PURE__ */ React.createElement("p", { className: "page-sub" }, "Agent \u2192 channel/trigger bindings \xB7 live from ", /* @__PURE__ */ React.createElement("span", { className: "mono" }, "/api/bindings"))), /* @__PURE__ */ React.createElement("div", { className: "page-actions" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: refresh }, /* @__PURE__ */ React.createElement(I.refresh, null)))), selected.size > 0 && /* @__PURE__ */ React.createElement("div", { className: "bulk-bar" }, /* @__PURE__ */ React.createElement("span", { className: "mono", style: { fontSize: 12 } }, selected.size, " selected"), /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: bulkRemove }, /* @__PURE__ */ React.createElement(I.trash, null), " Remove ", selected.size), /* @__PURE__ */ React.createElement("button", { className: "btn sm ghost", onClick: () => setSelected(/* @__PURE__ */ new Set()), style: { marginLeft: "auto" } }, "Clear")), fetchErr && /* @__PURE__ */ React.createElement("div", { className: "banner", style: { borderColor: "oklch(0.66 0.18 25 / .35)" } }, /* @__PURE__ */ React.createElement("span", { className: "dot err" }), /* @__PURE__ */ React.createElement("span", { className: "banner-title" }, "API ERROR"), /* @__PURE__ */ React.createElement("span", { className: "banner-body mono", style: { fontSize: 11 } }, fetchErr)), /* @__PURE__ */ React.createElement("div", { className: "card flush" }, /* @__PURE__ */ React.createElement("table", { className: "tbl" }, /* @__PURE__ */ React.createElement("thead", null, /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("th", { style: { width: 28 } }, /* @__PURE__ */ React.createElement(
    "input",
    {
      type: "checkbox",
      checked: bindings.length > 0 && selected.size === bindings.length,
      ref: (el) => {
        if (el) el.indeterminate = selected.size > 0 && selected.size < bindings.length;
      },
      onChange: toggleAll,
      title: selected.size === bindings.length ? "Deselect all" : "Select all"
    }
  )), /* @__PURE__ */ React.createElement("th", null, "#"), /* @__PURE__ */ React.createElement("th", null, "Agent"), /* @__PURE__ */ React.createElement("th", null, "Kind"), /* @__PURE__ */ React.createElement("th", null, "Target"), /* @__PURE__ */ React.createElement("th", null, "Pattern"), /* @__PURE__ */ React.createElement("th", null))), /* @__PURE__ */ React.createElement("tbody", null, !resp && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "loading\u2026")), resp && bindings.length === 0 && /* @__PURE__ */ React.createElement("tr", null, /* @__PURE__ */ React.createElement("td", { colSpan: 7, className: "muted mono", style: { padding: "12px 14px", fontSize: 12, textAlign: "center" } }, "No bindings configured.")), bindings.map((b, i) => {
    const isSel = selected.has(i);
    return /* @__PURE__ */ React.createElement("tr", { key: i, style: isSel ? { background: "var(--surface-2)" } : null }, /* @__PURE__ */ React.createElement("td", null, /* @__PURE__ */ React.createElement(
      "input",
      {
        type: "checkbox",
        checked: isSel,
        onChange: () => toggle(i)
      }
    )), /* @__PURE__ */ React.createElement("td", { className: "num mono" }, i), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.agent_id || b.agent || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.kind || b.type || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono" }, b.target || b.channel || b.trigger || "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "mono dim", style: { maxWidth: 280, overflow: "hidden", textOverflow: "ellipsis" } }, b.pattern ? JSON.stringify(b.pattern) : "\u2014"), /* @__PURE__ */ React.createElement("td", { className: "right" }, /* @__PURE__ */ React.createElement("button", { className: "btn sm danger", onClick: () => remove(i), title: "Remove" }, /* @__PURE__ */ React.createElement(I.close, null))));
  })))));
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
  BindingsPage,
  HealthPage
});

})();
