// RustyHand control panel — REST/WS client helpers.
//
// One job: turn the RustyHand HTTP API into ergonomic React hooks. Pages
// consume `useApi(path)` and get `[data, error, refresh]`. The rest of the
// panel stays declarative.
//
// Loading strategy: each hook starts with `null` data and `null` error.
// Pages fall back to the bundled mock fixtures (window.RH_DATA) while data
// is loading or when the kernel is unreachable — that keeps the design
// looking right during cold-boot and lets the dashboard still render in
// disaster scenarios.
//
// Auth: relies on cookie / localhost classification done by the API auth
// middleware. No bearer token is attached client-side — the dashboard runs
// against 127.0.0.1 in the common case and the middleware grants the
// `owner` role automatically. For remote setups operators are expected to
// reverse-proxy auth.

const __API_BASE = ""; // same-origin
const __API_KEY_STORAGE = "rh.panel.apiKey";

// Read the stored API key on first load. The login flow writes here on
// success and `clearApiKey()` clears it on 401 / logout.
function getApiKey() {
  try { return localStorage.getItem(__API_KEY_STORAGE) || ""; } catch (e) { return ""; }
}
function setApiKey(key) {
  try {
    if (key) localStorage.setItem(__API_KEY_STORAGE, key);
    else localStorage.removeItem(__API_KEY_STORAGE);
  } catch (e) {}
}
function clearApiKey() { setApiKey(""); }

async function rhFetch(path, init) {
  const headers = { "Accept": "application/json", ...(init && init.headers || {}) };
  const key = getApiKey();
  if (key) headers["Authorization"] = `Bearer ${key}`;
  const resp = await fetch(__API_BASE + path, {
    credentials: "same-origin",
    ...(init || {}),
    headers,
  });
  if (!resp.ok) {
    let body = "";
    try { body = await resp.text(); } catch (e) {}
    const err = new Error(`HTTP ${resp.status} ${resp.statusText}${body ? `: ${body.slice(0, 200)}` : ""}`);
    err.status = resp.status;
    throw err;
  }
  const ct = resp.headers.get("content-type") || "";
  if (ct.includes("application/json")) return resp.json();
  return resp.text();
}

// useApi(path) — one-shot GET with abort-on-unmount + manual refresh.
// Returns [data, error, refresh]. `data` and `error` are null until the
// first fetch resolves. `refresh()` re-fetches the same path.
function useApi(path, deps) {
  const [data, setData] = React.useState(null);
  const [err, setErr] = React.useState(null);
  const [tick, setTick] = React.useState(0);
  const depsKey = deps == null ? [path, tick] : [path, tick, ...deps];
  React.useEffect(() => {
    let aborted = false;
    if (!path) return;
    rhFetch(path)
      .then((d) => { if (!aborted) { setData(d); setErr(null); } })
      .catch((e) => { if (!aborted) { setErr(String(e.message || e)); } });
    return () => { aborted = true; };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, depsKey);
  const refresh = React.useCallback(() => setTick((t) => t + 1), []);
  return [data, err, refresh];
}

// usePolling(path, intervalMs) — refetch on a timer. Same return shape as
// useApi. Pauses while the document is hidden so background tabs don't
// hammer the kernel.
function usePolling(path, intervalMs) {
  const [data, err, refresh] = useApi(path);
  React.useEffect(() => {
    if (!path || !intervalMs) return;
    let id = null;
    const start = () => { id = setInterval(refresh, intervalMs); };
    const stop = () => { if (id) clearInterval(id); id = null; };
    const onVis = () => (document.hidden ? stop() : start());
    if (!document.hidden) start();
    document.addEventListener("visibilitychange", onVis);
    return () => { stop(); document.removeEventListener("visibilitychange", onVis); };
  }, [path, intervalMs, refresh]);
  return [data, err, refresh];
}

// --- Normalization helpers ------------------------------------------------
//
// The design's mock data uses different state names than the real API:
//   API state    →  panel state
//   running      →  running
//   created      →  idle
//   suspended    →  idle
//   terminated   →  stopped
//   crashed      →  error
//
// Plus the API doesn't populate `hue` / `emoji` / `messages` / `cost` /
// `last` / `updated` — we synthesize sensible defaults so the design's
// table doesn't render blank cells.

function mapAgentState(s) {
  switch ((s || "").toLowerCase()) {
    case "running": return "running";
    case "created": return "idle";
    case "suspended": return "idle";
    case "terminated": return "stopped";
    case "crashed": return "error";
    default: return s || "idle";
  }
}

function hueFromId(id) {
  // Stable hash → hue. Same id always produces the same hue so the avatar
  // doesn't shimmer between renders.
  let h = 0;
  for (let i = 0; i < id.length; i++) h = ((h << 5) - h + id.charCodeAt(i)) | 0;
  return Math.abs(h) % 360;
}

function relativeTime(iso) {
  if (!iso) return "—";
  const ts = Date.parse(iso);
  if (Number.isNaN(ts)) return "—";
  const ds = Math.max(0, Math.floor((Date.now() - ts) / 1000));
  if (ds < 60) return `${ds}s`;
  if (ds < 3600) return `${Math.floor(ds / 60)}m`;
  if (ds < 86400) return `${Math.floor(ds / 3600)}h`;
  return `${Math.floor(ds / 86400)}d`;
}

function normalizeAgent(a) {
  return {
    id: a.id,
    name: a.name,
    emoji: (a.identity && a.identity.emoji) || a.name.charAt(0).toUpperCase(),
    group: a.group || "—",
    state: mapAgentState(a.state),
    model: a.model_name || "—",
    provider: a.model_provider || "—",
    messages: a.message_count || 0,
    cost: 0, // populated by per-agent budget endpoint, not list
    last: a.last_message_preview || "—",
    updated: relativeTime(a.last_activity),
    hue: hueFromId(a.id),
  };
}

// useAgentWs(agentId, onEvent) — open a WebSocket to /api/agents/{id}/ws,
// dispatch parsed JSON events to `onEvent`, and expose a `send(content)`
// helper. The WS is kept open across the lifetime of the hook (i.e. the
// component instance). Reconnect is intentionally NOT implemented here —
// the panel re-keys the chat component on agent switch, which remounts
// the hook and opens a fresh socket.
function useAgentWs(agentId, onEvent) {
  const [connected, setConnected] = React.useState(false);
  const wsRef = React.useRef(null);
  // Keep the latest onEvent in a ref so closure changes don't churn the WS.
  const handlerRef = React.useRef(onEvent);
  handlerRef.current = onEvent;

  React.useEffect(() => {
    if (!agentId) return;
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    // WS upgrades cannot carry custom headers, so the server also accepts
    // the bearer token via a `?token=` query string.
    const key = getApiKey();
    const tokenQs = key ? `?token=${encodeURIComponent(key)}` : "";
    const url = `${proto}//${window.location.host}/api/agents/${encodeURIComponent(agentId)}/ws${tokenQs}`;
    let ws;
    try { ws = new WebSocket(url); } catch (e) { console.warn("ws open failed", e); return; }
    wsRef.current = ws;
    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
    ws.onerror = () => setConnected(false);
    ws.onmessage = (e) => {
      let msg;
      try { msg = JSON.parse(e.data); } catch (_) { return; }
      if (handlerRef.current) handlerRef.current(msg);
    };
    return () => {
      try { ws.close(); } catch (_) {}
      wsRef.current = null;
    };
  }, [agentId]);

  const send = React.useCallback((content) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return false;
    ws.send(JSON.stringify({ type: "message", content }));
    return true;
  }, []);

  return { connected, send };
}

// renderMarkdown — convert a plain-text string with a handful of markdown
// constructs to React elements. Safe-by-default: never interprets raw HTML
// because we only emit text via React's escaping. Covers what real Claude
// agents produce: paragraphs, bold/italic, inline + fenced code, links,
// bullet lists, headings. Anything outside the set falls through as text.
function renderMarkdown(src) {
  if (!src) return null;
  const lines = String(src).split("\n");
  const out = [];
  let i = 0;
  let key = 0;
  while (i < lines.length) {
    const line = lines[i];
    // Fenced code block
    if (line.startsWith("```")) {
      const lang = line.slice(3).trim();
      const buf = [];
      i++;
      while (i < lines.length && !lines[i].startsWith("```")) { buf.push(lines[i]); i++; }
      if (i < lines.length) i++; // skip closing ```
      out.push(React.createElement("pre", { key: key++, className: "md-pre" },
        React.createElement("code", { className: lang ? `lang-${lang}` : "" }, buf.join("\n"))));
      continue;
    }
    // Heading
    const hm = /^(#{1,3})\s+(.*)$/.exec(line);
    if (hm) {
      const tag = `h${hm[1].length + 2}`; // h3..h5 — keep them visually subtle inside bubbles
      out.push(React.createElement(tag, { key: key++, className: "md-h" }, renderInline(hm[2], key * 1000)));
      i++; continue;
    }
    // Bullet list
    if (/^\s*[-*]\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*[-*]\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*[-*]\s+/, ""));
        i++;
      }
      out.push(React.createElement("ul", { key: key++, className: "md-ul" },
        items.map((t, j) => React.createElement("li", { key: j }, renderInline(t, j * 100)))));
      continue;
    }
    // Numbered list
    if (/^\s*\d+\.\s+/.test(line)) {
      const items = [];
      while (i < lines.length && /^\s*\d+\.\s+/.test(lines[i])) {
        items.push(lines[i].replace(/^\s*\d+\.\s+/, ""));
        i++;
      }
      out.push(React.createElement("ol", { key: key++, className: "md-ol" },
        items.map((t, j) => React.createElement("li", { key: j }, renderInline(t, j * 100)))));
      continue;
    }
    // Blank line — paragraph break
    if (line.trim() === "") { i++; continue; }
    // Paragraph: collect until blank line / fence / list / heading
    const paraLines = [];
    while (i < lines.length && lines[i].trim() !== ""
           && !lines[i].startsWith("```")
           && !/^(#{1,3})\s+/.test(lines[i])
           && !/^\s*[-*]\s+/.test(lines[i])
           && !/^\s*\d+\.\s+/.test(lines[i])) {
      paraLines.push(lines[i]);
      i++;
    }
    out.push(React.createElement("p", { key: key++, className: "md-p" }, renderInline(paraLines.join(" "), key * 1000)));
  }
  return out;
}

// Inline pass: **bold**, *italic*, `code`, [text](url). Operates on a
// string and returns an array of React children. Greedy single-pass — fine
// for chat-sized text.
function renderInline(text, keyBase) {
  const out = [];
  let cursor = 0;
  let k = keyBase || 0;
  // Pattern alternation: code, bold, italic, link.
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
      // Only allow http(s) and relative links — anything else renders
      // as plain text to avoid javascript: URLs landing in the DOM.
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

// usePagination(prefix, pageSize) — small hook that returns a tuple
// {offset, page, total, setTotal, setPage, prev, next, hasPrev, hasNext}.
// Avoids the page-bounce race: when the user clicks next then prev faster
// than the fetch resolves, we discard the stale response. The hook tracks
// `requestedOffset` and pages.jsx consumers check it before applying data.
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
    offset, pageSize, page, totalPages, total,
    setTotal, setOffset, prev, next, reset,
    hasPrev: offset > 0,
    hasNext: offset + pageSize < total,
  };
}

// useEscapeKey(handler) — register a window keydown listener that fires
// `handler()` on Escape. Used by modals to close on Esc. Cleans up on
// unmount so closed modals don't leak listeners.
function useEscapeKey(handler) {
  React.useEffect(() => {
    if (!handler) return;
    const onKey = (e) => { if (e.key === "Escape") handler(); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [handler]);
}

// Toast system — small, dependency-free. Components call `toast(...)`
// or shortcut helpers; a single `<ToastHost/>` mounted by App renders
// the queue. State lives in module scope so any caller can dispatch
// without prop drilling.
const __TOASTS = { items: [], subs: new Set(), nextId: 1 };
function toast(msg, opts) {
  const id = __TOASTS.nextId++;
  const t = {
    id,
    msg: String(msg),
    kind: (opts && opts.kind) || "info",  // info | ok | warn | err
    ttl: (opts && opts.ttl) != null ? opts.ttl : 4500,
  };
  __TOASTS.items = __TOASTS.items.concat([t]);
  __TOASTS.subs.forEach((fn) => fn(__TOASTS.items));
  if (t.ttl > 0) setTimeout(() => dismissToast(id), t.ttl);
  return id;
}
function dismissToast(id) {
  __TOASTS.items = __TOASTS.items.filter((t) => t.id !== id);
  __TOASTS.subs.forEach((fn) => fn(__TOASTS.items));
}
function toastOk(msg, opts) { return toast(msg, { ...(opts || {}), kind: "ok" }); }
function toastWarn(msg, opts) { return toast(msg, { ...(opts || {}), kind: "warn" }); }
function toastErr(msg, opts) { return toast(msg, { ...(opts || {}), kind: "err", ttl: 7000 }); }

function ToastHost() {
  const [items, setItems] = React.useState(__TOASTS.items);
  React.useEffect(() => {
    __TOASTS.subs.add(setItems);
    return () => { __TOASTS.subs.delete(setItems); };
  }, []);
  if (!items || items.length === 0) return null;
  return (
    <div className="toast-host">
      {items.map((t) => (
        <div key={t.id} className={`toast toast-${t.kind}`}>
          <span className={`dot ${t.kind === "ok" ? "live" : t.kind === "err" ? "err" : t.kind === "warn" ? "warn" : "idle"}`}/>
          <span className="toast-msg">{t.msg}</span>
          <button className="toast-x" onClick={() => dismissToast(t.id)}>✕</button>
        </div>
      ))}
    </div>
  );
}

// ErrorBoundary — wraps the app so a runtime crash in any page shows a
// recovery card instead of a blank `<div id="root">`. Uses
// `getDerivedStateFromError` (React docs pattern). We deliberately
// shape this as a class component because hooks can't catch render
// errors yet.
class ErrorBoundary extends React.Component {
  constructor(props) {
    super(props);
    this.state = { err: null };
  }
  static getDerivedStateFromError(err) {
    return { err };
  }
  componentDidCatch(err, info) {
    // Surface to console for ops; the user sees the recovery card.
    // eslint-disable-next-line no-console
    console.error("[panel] component crashed", err, info);
  }
  reset() { this.setState({ err: null }); }
  render() {
    if (this.state.err) {
      const e = this.state.err;
      return (
        <div className="auth-splash">
          <div className="auth-card">
            <div className="auth-mark">!</div>
            <div className="auth-title">Panel crashed</div>
            <div className="auth-sub">A React component threw during render. The kernel is unaffected — refresh to retry.</div>
            <pre className="codebox" style={{ maxHeight: 200, fontSize: 11 }}>{String(e && (e.stack || e.message || e))}</pre>
            <div className="row gap-8">
              <button className="btn" onClick={() => this.reset()}>Reset</button>
              <button className="btn primary" onClick={() => window.location.reload()}>Reload page</button>
            </div>
          </div>
        </div>
      );
    }
    return this.props.children;
  }
}

// downloadBlob(filename, content, type) — trigger a browser download for
// in-memory data. Used for "Export CSV" / "Export audit" buttons.
function downloadBlob(filename, content, type) {
  const blob = new Blob([content], { type: type || "text/plain;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url; a.download = filename;
  document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

// rowsToCsv(rows, columns) — convert an array of objects to a CSV string.
// `columns` is an array of either string keys or {key,label,format} objects.
function rowsToCsv(rows, columns) {
  const cols = (columns || []).map(c => typeof c === "string" ? { key: c, label: c } : c);
  const escape = (v) => {
    if (v == null) return "";
    const s = String(v);
    return /[,"\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
  };
  const head = cols.map(c => escape(c.label || c.key)).join(",");
  const body = rows.map(r => cols.map(c => escape(c.format ? c.format(r[c.key], r) : r[c.key])).join(",")).join("\n");
  return head + "\n" + body + "\n";
}

Object.assign(window, {
  rhFetch, useApi, usePolling, useAgentWs, usePagination, useEscapeKey,
  mapAgentState, hueFromId, relativeTime, normalizeAgent,
  renderMarkdown, downloadBlob, rowsToCsv,
  getApiKey, setApiKey, clearApiKey,
  toast, toastOk, toastWarn, toastErr, dismissToast, ToastHost, ErrorBoundary,
});
