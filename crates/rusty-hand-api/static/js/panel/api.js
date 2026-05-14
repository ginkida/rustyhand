const __API_BASE = "";
const __API_KEY_STORAGE = "rh.panel.apiKey";
function getApiKey() {
  try {
    return localStorage.getItem(__API_KEY_STORAGE) || "";
  } catch (e) {
    return "";
  }
}
function setApiKey(key) {
  try {
    if (key) localStorage.setItem(__API_KEY_STORAGE, key);
    else localStorage.removeItem(__API_KEY_STORAGE);
  } catch (e) {
  }
}
function clearApiKey() {
  setApiKey("");
}
async function rhFetch(path, init) {
  const headers = { "Accept": "application/json", ...init && init.headers || {} };
  const key = getApiKey();
  if (key) headers["Authorization"] = `Bearer ${key}`;
  const resp = await fetch(__API_BASE + path, {
    credentials: "same-origin",
    ...init || {},
    headers
  });
  if (!resp.ok) {
    let body = "";
    try {
      body = await resp.text();
    } catch (e) {
    }
    const err = new Error(`HTTP ${resp.status} ${resp.statusText}${body ? `: ${body.slice(0, 200)}` : ""}`);
    err.status = resp.status;
    throw err;
  }
  const ct = resp.headers.get("content-type") || "";
  if (ct.includes("application/json")) return resp.json();
  return resp.text();
}
function useApi(path, deps) {
  const [data, setData] = React.useState(null);
  const [err, setErr] = React.useState(null);
  const [tick, setTick] = React.useState(0);
  const depsKey = deps == null ? [path, tick] : [path, tick, ...deps];
  React.useEffect(() => {
    let aborted = false;
    if (!path) return;
    rhFetch(path).then((d) => {
      if (!aborted) {
        setData(d);
        setErr(null);
      }
    }).catch((e) => {
      if (!aborted) {
        setErr(String(e.message || e));
      }
    });
    return () => {
      aborted = true;
    };
  }, depsKey);
  const refresh = React.useCallback(() => setTick((t) => t + 1), []);
  return [data, err, refresh];
}
function usePolling(path, intervalMs) {
  const [data, err, refresh] = useApi(path);
  React.useEffect(() => {
    if (!path || !intervalMs) return;
    let id = null;
    const start = () => {
      id = setInterval(refresh, intervalMs);
    };
    const stop = () => {
      if (id) clearInterval(id);
      id = null;
    };
    const onVis = () => document.hidden ? stop() : start();
    if (!document.hidden) start();
    document.addEventListener("visibilitychange", onVis);
    return () => {
      stop();
      document.removeEventListener("visibilitychange", onVis);
    };
  }, [path, intervalMs, refresh]);
  return [data, err, refresh];
}
function mapAgentState(s) {
  switch ((s || "").toLowerCase()) {
    case "running":
      return "running";
    case "created":
      return "idle";
    case "suspended":
      return "idle";
    case "terminated":
      return "stopped";
    case "crashed":
      return "error";
    default:
      return s || "idle";
  }
}
function hueFromId(id) {
  let h = 0;
  for (let i = 0; i < id.length; i++) h = (h << 5) - h + id.charCodeAt(i) | 0;
  return Math.abs(h) % 360;
}
function formatUptimeShort(s) {
  if (s == null) return null;
  const d = Math.floor(s / 86400);
  const h = Math.floor(s % 86400 / 3600);
  const m = Math.floor(s % 3600 / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${Math.floor(s)}s`;
}
function relativeTime(iso) {
  if (!iso) return "\u2014";
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return "\u2014";
  const ds = Math.max(0, Math.floor((Date.now() - ts) / 1e3));
  if (ds < 60) return `${ds}s`;
  if (ds < 3600) return `${Math.floor(ds / 60)}m`;
  if (ds < 86400) return `${Math.floor(ds / 3600)}h`;
  return `${Math.floor(ds / 86400)}d`;
}
function normalizeAgent(a) {
  return {
    id: a.id,
    name: a.name,
    emoji: a.identity && a.identity.emoji || a.name.charAt(0).toUpperCase(),
    group: a.group || "\u2014",
    state: mapAgentState(a.state),
    model: a.model_name || "\u2014",
    provider: a.model_provider || "\u2014",
    messages: a.message_count || 0,
    cost: 0,
    // populated by per-agent budget endpoint, not list
    last: a.last_message_preview || "\u2014",
    updated: relativeTime(a.last_activity),
    hue: hueFromId(a.id)
  };
}
function useAgentWs(agentId, onEvent) {
  const [connected, setConnected] = React.useState(false);
  const [reconnecting, setReconnecting] = React.useState(false);
  const wsRef = React.useRef(null);
  const handlerRef = React.useRef(onEvent);
  handlerRef.current = onEvent;
  const attemptRef = React.useRef(0);
  React.useEffect(() => {
    if (!agentId) return;
    let aborted = false;
    let retryTimer = null;
    attemptRef.current = 0;
    const connect = () => {
      if (aborted) return;
      const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
      const key = getApiKey();
      const tokenQs = key ? `?token=${encodeURIComponent(key)}` : "";
      const url = `${proto}//${window.location.host}/api/agents/${encodeURIComponent(agentId)}/ws${tokenQs}`;
      let ws;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        console.warn("ws open failed", e);
        scheduleRetry();
        return;
      }
      wsRef.current = ws;
      ws.onopen = () => {
        if (aborted) {
          try {
            ws.close();
          } catch (_) {
          }
          return;
        }
        attemptRef.current = 0;
        setConnected(true);
        setReconnecting(false);
      };
      ws.onclose = () => {
        setConnected(false);
        if (!aborted) scheduleRetry();
      };
      ws.onerror = () => setConnected(false);
      ws.onmessage = (e) => {
        let msg;
        try {
          msg = JSON.parse(e.data);
        } catch (_) {
          return;
        }
        if (handlerRef.current) handlerRef.current(msg);
      };
    };
    const scheduleRetry = () => {
      if (aborted) return;
      if (document.hidden) {
        setReconnecting(false);
        return;
      }
      const attempt = attemptRef.current++;
      const delay = Math.min(3e4, 1e3 * Math.pow(2, attempt));
      setReconnecting(true);
      retryTimer = setTimeout(connect, delay);
    };
    const onVis = () => {
      if (!document.hidden && !wsRef.current) {
        attemptRef.current = 0;
        connect();
      }
    };
    document.addEventListener("visibilitychange", onVis);
    connect();
    return () => {
      aborted = true;
      if (retryTimer) clearTimeout(retryTimer);
      document.removeEventListener("visibilitychange", onVis);
      try {
        wsRef.current && wsRef.current.close();
      } catch (_) {
      }
      wsRef.current = null;
    };
  }, [agentId]);
  const send = React.useCallback((content) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return false;
    ws.send(JSON.stringify({ type: "message", content }));
    return true;
  }, []);
  return { connected, reconnecting, send };
}
function renderMarkdown(src) {
  if (!src) return null;
  const lines = String(src).split("\n");
  const out = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const line = lines[i];
    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) {
        buf.push(lines[i]);
        i++;
      }
      if (i < lines.length) i++;
      out.push(React.createElement(
        "pre",
        { key: key++, className: "md-pre" },
        React.createElement("code", { className: lang ? `lang-${lang}` : "" }, buf.join("\n"))
      ));
      continue;
    }
    const hm = /^(#{1,3})\s+(.*)$/.exec(line);
    if (hm) {
      const tag = `h${hm[1].length + 2}`;
      out.push(React.createElement(tag, { key: key++, className: "md-h" }, renderInline(hm[2], key * 1e3)));
      i++;
      continue;
    }
    if (/^\s*[-*]\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ""));
        i++;
      }
      out.push(React.createElement(
        "ul",
        { key: key++, className: "md-ul" },
        items.map((t, j) => React.createElement("li", { key: j }, renderInline(t, j * 100)))
      ));
      continue;
    }
    if (/^\s*\d+\.\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ""));
        i++;
      }
      out.push(React.createElement(
        "ol",
        { key: key++, className: "md-ol" },
        items.map((t, j) => React.createElement("li", { key: j }, renderInline(t, j * 100)))
      ));
      continue;
    }
    if (line.trim() === "") {
      i++;
      continue;
    }
    const paraLines = [];
    while (i < lines.length && lines[i].trim() !== "" && !lines[i].startsWith("```") && !/^(#{1,3})\s+/.test(lines[i]) && !/^\s*[-*]\s+/.test(lines[i]) && !/^\s*\d+\.\s+/.test(lines[i])) {
      paraLines.push(lines[i]);
      i++;
    }
    out.push(React.createElement("p", { key: key++, className: "md-p" }, renderInline(paraLines.join(" "), key * 1e3)));
  }
  return out;
}
function renderInline(text, keyBase) {
  const out = [];
  let cursor = 0;
  let k = keyBase || 0;
  const re = /(`[^`\n]+`)|(\*\*[^*\n]+\*\*)|(\*[^*\n]+\*)|(\[[^\]]+\]\([^)]+\))/g;
  let m;
  while ((m = re.exec(text)) !== null) {
    if (m.index > cursor) out.push(text.slice(cursor, m.index));
    const tok = m[0];
    if (tok.startsWith("`")) {
      out.push(React.createElement("code", { key: k++, className: "md-code" }, tok.slice(1, -1)));
    } else if (tok.startsWith("**")) {
      out.push(React.createElement("b", { key: k++ }, tok.slice(2, -2)));
    } else if (tok.startsWith("*")) {
      out.push(React.createElement("i", { key: k++ }, tok.slice(1, -1)));
    } else if (tok.startsWith("[")) {
      const lm = /\[([^\]]+)\]\(([^)]+)\)/.exec(tok);
      const href = lm[2];
      const safe = /^(https?:|\/|#)/i.test(href) ? href : null;
      if (safe) {
        out.push(React.createElement("a", { key: k++, href: safe, target: "_blank", rel: "noopener noreferrer" }, lm[1]));
      } else {
        out.push(`[${lm[1]}](${href})`);
      }
    }
    cursor = m.index + tok.length;
  }
  if (cursor < text.length) out.push(text.slice(cursor));
  return out;
}
const __CONFIRMS = { items: [], subs: /* @__PURE__ */ new Set(), nextId: 1 };
function confirmDialog(opts) {
  return new Promise((resolve) => {
    const id = __CONFIRMS.nextId++;
    const entry = {
      id,
      title: opts && opts.title || "Confirm",
      message: opts && opts.message || "",
      confirmLabel: opts && opts.confirmLabel || "OK",
      cancelLabel: opts && opts.cancelLabel || "Cancel",
      danger: !!(opts && opts.danger),
      resolve
    };
    __CONFIRMS.items = __CONFIRMS.items.concat([entry]);
    __CONFIRMS.subs.forEach((fn) => fn(__CONFIRMS.items));
  });
}
function ConfirmHost() {
  const [items, setItems] = React.useState(__CONFIRMS.items);
  React.useEffect(() => {
    __CONFIRMS.subs.add(setItems);
    return () => {
      __CONFIRMS.subs.delete(setItems);
    };
  }, []);
  const dismiss = (id, value) => {
    const it = __CONFIRMS.items.find((x) => x.id === id);
    if (it) it.resolve(value);
    __CONFIRMS.items = __CONFIRMS.items.filter((x) => x.id !== id);
    __CONFIRMS.subs.forEach((fn) => fn(__CONFIRMS.items));
  };
  const cur = items[0];
  React.useEffect(() => {
    if (!cur) return;
    const onKey = (e) => {
      var _a, _b;
      if (e.key === "Escape") dismiss(cur.id, false);
      else if (e.key === "Enter" && !((_b = (_a = e.target).matches) == null ? void 0 : _b.call(_a, "textarea"))) dismiss(cur.id, true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cur && cur.id]);
  if (!cur) return null;
  return /* @__PURE__ */ React.createElement("div", { className: "modal-back", onClick: () => dismiss(cur.id, false) }, /* @__PURE__ */ React.createElement("div", { className: "modal", style: { maxWidth: 460 }, onClick: (e) => e.stopPropagation() }, /* @__PURE__ */ React.createElement("div", { className: "modal-head" }, /* @__PURE__ */ React.createElement("b", { className: "mono" }, cur.title)), /* @__PURE__ */ React.createElement("div", { className: "modal-body" }, /* @__PURE__ */ React.createElement("div", { style: { fontSize: 13, color: "var(--fg-2)", whiteSpace: "pre-wrap" } }, cur.message)), /* @__PURE__ */ React.createElement("div", { className: "modal-foot" }, /* @__PURE__ */ React.createElement("button", { className: "btn ghost", onClick: () => dismiss(cur.id, false) }, cur.cancelLabel), /* @__PURE__ */ React.createElement("button", { className: "btn " + (cur.danger ? "danger" : "primary"), autoFocus: true, onClick: () => dismiss(cur.id, true) }, cur.confirmLabel))));
}
function Skel({ w, h, radius }) {
  return /* @__PURE__ */ React.createElement("span", { className: "skel", style: {
    display: "inline-block",
    width: typeof w === "number" ? `${w}px` : w || "100%",
    height: typeof h === "number" ? `${h}px` : h || "10px",
    borderRadius: radius != null ? radius : 4,
    verticalAlign: "middle"
  } });
}
function SkelRow({ cols }) {
  const widths = Array.isArray(cols) ? cols : Array.from({ length: cols || 4 }, () => `${30 + Math.floor(Math.random() * 50)}%`);
  return /* @__PURE__ */ React.createElement("tr", null, widths.map((w, i) => /* @__PURE__ */ React.createElement("td", { key: i }, /* @__PURE__ */ React.createElement(Skel, { w, h: 10 }))));
}
function SkelCard({ height }) {
  return /* @__PURE__ */ React.createElement("div", { className: "card" }, /* @__PURE__ */ React.createElement(Skel, { w: 120, h: 11 }), /* @__PURE__ */ React.createElement("div", { style: { marginTop: 10 } }, /* @__PURE__ */ React.createElement(Skel, { w: "60%", h: 18 })), /* @__PURE__ */ React.createElement("div", { style: { marginTop: 10 } }, /* @__PURE__ */ React.createElement(Skel, { w: "40%", h: 10 })), /* @__PURE__ */ React.createElement("div", { style: { marginTop: 14 } }, /* @__PURE__ */ React.createElement(Skel, { w: "100%", h: height || 40, radius: 6 })));
}
function useAsyncAction(fn) {
  const [busy, setBusy] = React.useState(false);
  const run = React.useCallback(async (...args) => {
    if (busy) return;
    setBusy(true);
    try {
      return await fn(...args);
    } finally {
      setBusy(false);
    }
  }, [fn, busy]);
  return [run, busy];
}
function parseHash(hash) {
  const raw = (hash || "").replace(/^#\/?/, "");
  if (!raw) return { page: "overview", params: {}, query: {} };
  const [path, qs] = raw.split("?");
  const parts = path.split("/").filter(Boolean);
  const page = parts[0] || "overview";
  const params = {};
  if (parts.length >= 2) params.id = parts[1];
  const query = {};
  if (qs) {
    for (const pair of qs.split("&")) {
      const [k, v] = pair.split("=");
      if (k) query[decodeURIComponent(k)] = v ? decodeURIComponent(v) : "";
    }
  }
  return { page, params, query };
}
function buildHash(page, params, query) {
  let h = "#/" + page;
  if (params && params.id) h += "/" + encodeURIComponent(params.id);
  if (query && Object.keys(query).length) {
    const qs = Object.entries(query).filter(([, v]) => v !== void 0 && v !== null && v !== "").map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`).join("&");
    if (qs) h += "?" + qs;
  }
  return h;
}
function useHashRoute() {
  const [route, setRoute] = React.useState(() => parseHash(window.location.hash));
  React.useEffect(() => {
    const onHash = () => setRoute(parseHash(window.location.hash));
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);
  const navigate = React.useCallback((page, params, query) => {
    const next = buildHash(page, params, query);
    if (window.location.hash !== next) {
      const cur = parseHash(window.location.hash);
      if (cur.page === page && (!params || !params.id) && (!cur.params || !cur.params.id)) {
        window.history.replaceState(null, "", next);
        setRoute(parseHash(next));
      } else {
        window.location.hash = next;
      }
    }
  }, []);
  return { ...route, navigate };
}
function usePagination(pageSize) {
  const [offset, setOffset] = React.useState(0);
  const [total, setTotal] = React.useState(0);
  const page = Math.floor(offset / pageSize) + 1;
  const totalPages = Math.max(1, Math.ceil(total / pageSize));
  const next = React.useCallback(() => {
    setOffset((o) => {
      if (o + pageSize >= total) return o;
      return o + pageSize;
    });
  }, [pageSize, total]);
  const prev = React.useCallback(() => {
    setOffset((o) => Math.max(0, o - pageSize));
  }, [pageSize]);
  const reset = React.useCallback(() => setOffset(0), []);
  return {
    offset,
    pageSize,
    page,
    totalPages,
    total,
    setTotal,
    setOffset,
    prev,
    next,
    reset,
    hasPrev: offset > 0,
    hasNext: offset + pageSize < total
  };
}
function Tip({ children }) {
  const [open, setOpen] = React.useState(false);
  return /* @__PURE__ */ React.createElement(
    "span",
    {
      className: "tip-wrap",
      tabIndex: 0,
      onMouseEnter: () => setOpen(true),
      onMouseLeave: () => setOpen(false),
      onFocus: () => setOpen(true),
      onBlur: () => setOpen(false)
    },
    /* @__PURE__ */ React.createElement("span", { className: "tip-glyph" }, "?"),
    open && /* @__PURE__ */ React.createElement("span", { className: "tip-bubble", role: "tooltip" }, children)
  );
}
function useEscapeKey(handler) {
  React.useEffect(() => {
    if (!handler) return;
    const onKey = (e) => {
      if (e.key === "Escape") handler();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}
const __TOASTS = { items: [], timers: /* @__PURE__ */ new Map(), subs: /* @__PURE__ */ new Set(), nextId: 1 };
function toast(msg, opts) {
  const id = __TOASTS.nextId++;
  const t = {
    id,
    msg: String(msg),
    kind: opts && opts.kind || "info",
    // info | ok | warn | err
    ttl: (opts && opts.ttl) != null ? opts.ttl : 4500
  };
  __TOASTS.items = __TOASTS.items.concat([t]);
  __TOASTS.subs.forEach((fn) => fn(__TOASTS.items));
  if (t.ttl > 0) {
    const timerId = setTimeout(() => dismissToast(id), t.ttl);
    __TOASTS.timers.set(id, timerId);
  }
  return id;
}
function dismissToast(id) {
  const timerId = __TOASTS.timers.get(id);
  if (timerId != null) {
    clearTimeout(timerId);
    __TOASTS.timers.delete(id);
  }
  __TOASTS.items = __TOASTS.items.filter((t) => t.id !== id);
  __TOASTS.subs.forEach((fn) => fn(__TOASTS.items));
}
function toastOk(msg, opts) {
  return toast(msg, { ...opts || {}, kind: "ok" });
}
function toastWarn(msg, opts) {
  return toast(msg, { ...opts || {}, kind: "warn" });
}
function toastErr(msg, opts) {
  return toast(msg, { ...opts || {}, kind: "err", ttl: 7e3 });
}
function ToastHost() {
  const [items, setItems] = React.useState(__TOASTS.items);
  React.useEffect(() => {
    __TOASTS.subs.add(setItems);
    return () => {
      __TOASTS.subs.delete(setItems);
    };
  }, []);
  if (!items || items.length === 0) return null;
  return /* @__PURE__ */ React.createElement("div", { className: "toast-host" }, items.map((t) => /* @__PURE__ */ React.createElement("div", { key: t.id, className: `toast toast-${t.kind}` }, /* @__PURE__ */ React.createElement("span", { className: `dot ${t.kind === "ok" ? "live" : t.kind === "err" ? "err" : t.kind === "warn" ? "warn" : "idle"}` }), /* @__PURE__ */ React.createElement("span", { className: "toast-msg" }, t.msg), /* @__PURE__ */ React.createElement("button", { className: "toast-x", onClick: () => dismissToast(t.id) }, "\u2715"))));
}
class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { err: null };
  }
  static getDerivedStateFromError(err) {
    return { err };
  }
  componentDidCatch(err, info) {
    console.error("[panel] component crashed", err, info);
  }
  reset() {
    this.setState({ err: null });
  }
  render() {
    if (this.state.err) {
      const e = this.state.err;
      return /* @__PURE__ */ React.createElement("div", { className: "auth-splash" }, /* @__PURE__ */ React.createElement("div", { className: "auth-card" }, /* @__PURE__ */ React.createElement("div", { className: "auth-mark" }, "!"), /* @__PURE__ */ React.createElement("div", { className: "auth-title" }, "Panel crashed"), /* @__PURE__ */ React.createElement("div", { className: "auth-sub" }, "A React component threw during render. The kernel is unaffected \u2014 refresh to retry."), /* @__PURE__ */ React.createElement("pre", { className: "codebox", style: { maxHeight: 200, fontSize: 11 } }, String(e && (e.stack || e.message || e))), /* @__PURE__ */ React.createElement("div", { className: "row gap-8" }, /* @__PURE__ */ React.createElement("button", { className: "btn", onClick: () => this.reset() }, "Reset"), /* @__PURE__ */ React.createElement("button", { className: "btn primary", onClick: () => window.location.reload() }, "Reload page"))));
    }
    return this.props.children;
  }
}
function downloadBlob(filename, content, type) {
  const blob = new Blob([content], { type: type || "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1e3);
}
function rowsToCsv(rows, columns) {
  const cols = (columns || []).map((c) => typeof c === "string" ? { key: c, label: c } : c);
  const escape = (v) => {
    if (v == null) return "";
    const s = String(v);
    return /[,"\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const head = cols.map((c) => escape(c.label || c.key)).join(",");
  const body = rows.map((r) => cols.map((c) => escape(c.format ? c.format(r[c.key], r) : r[c.key])).join(",")).join("\n");
  return head + "\n" + body + "\n";
}
Object.assign(window, {
  rhFetch,
  useApi,
  usePolling,
  useAgentWs,
  usePagination,
  useEscapeKey,
  useHashRoute,
  useAsyncAction,
  mapAgentState,
  hueFromId,
  relativeTime,
  formatUptimeShort,
  normalizeAgent,
  renderMarkdown,
  downloadBlob,
  rowsToCsv,
  getApiKey,
  setApiKey,
  clearApiKey,
  toast,
  toastOk,
  toastWarn,
  toastErr,
  dismissToast,
  ToastHost,
  ErrorBoundary,
  Skel,
  SkelRow,
  SkelCard,
  Tip,
  confirmDialog,
  ConfirmHost
});
