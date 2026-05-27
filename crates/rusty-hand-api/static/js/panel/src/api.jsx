// RustyHand control panel — REST/WS client helpers.
//
// One job: turn the RustyHand HTTP API into ergonomic React hooks. Pages
// consume `useApi(path)` and get `[data, error, refresh]`. The rest of the
// panel stays declarative.
//
// Loading strategy: each hook starts with `null` data and `null` error.
// Pages show empty state (skeletons / "loading…") until the first fetch
// resolves. No mock fixtures — every number on screen is live.
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

// formatUptimeShort(seconds) — "3d 4h", "2h 15m", "42m", "12s".
// Used in the sidebar status row where horizontal space is tight.
function formatUptimeShort(s) {
  if (s == null) return null;
  const d = Math.floor(s / 86400);
  const h = Math.floor((s % 86400) / 3600);
  const m = Math.floor((s % 3600) / 60);
  if (d > 0) return `${d}d ${h}h`;
  if (h > 0) return `${h}h ${m}m`;
  if (m > 0) return `${m}m`;
  return `${Math.floor(s)}s`;
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

// useEventSource(path, onMessage) — subscribe to a Server-Sent Events
// stream. EventSource has no built-in auth header support, so we
// append `?token=…` when an API key is set (the kernel accepts it on
// /api/logs/stream the same way it does on /api/agents/{id}/ws).
//
// Auto-reconnect is built into the browser's EventSource for transient
// errors, but on 401/404 it gives up — we listen for `error` events
// and surface a `connected` boolean so the UI can show a stale-feed
// indicator.
function useEventSource(path, onMessage) {
  const [connected, setConnected] = React.useState(false);
  const handlerRef = React.useRef(onMessage);
  handlerRef.current = onMessage;

  React.useEffect(() => {
    if (!path) return;
    const key = getApiKey();
    const sep = path.includes("?") ? "&" : "?";
    const url = key ? `${path}${sep}token=${encodeURIComponent(key)}` : path;
    let es;
    try { es = new EventSource(url); } catch (e) { console.warn("SSE open failed", e); return; }
    es.onopen = () => setConnected(true);
    es.onerror = () => setConnected(false);
    es.onmessage = (e) => {
      let msg = e.data;
      // SSE data is often JSON; try to parse but fall through to raw
      // string if the producer ships plain text.
      try { msg = JSON.parse(e.data); } catch (_) {}
      if (handlerRef.current) handlerRef.current(msg);
    };
    return () => { try { es.close(); } catch (_) {} };
  }, [path]);

  return { connected };
}

// useAgentWs(agentId, onEvent) — open a WebSocket to /api/agents/{id}/ws,
// dispatch parsed JSON events to `onEvent`, and expose `send(content)`.
//
// Reconnects automatically with exponential backoff (1s, 2s, 4s, 8s,
// 16s, capped at 30s) until the agent changes or the component
// unmounts. Surfaces `connected` + `reconnecting` flags so the UI can
// show a clear state. Stops attempts when the document is hidden to
// avoid a flurry of failed connects in background tabs.
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
      try { ws = new WebSocket(url); }
      catch (e) { console.warn("ws open failed", e); scheduleRetry(); return; }
      wsRef.current = ws;
      ws.onopen = () => {
        if (aborted) { try { ws.close(); } catch (_) {} return; }
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
        try { msg = JSON.parse(e.data); } catch (_) { return; }
        if (handlerRef.current) handlerRef.current(msg);
      };
    };

    const scheduleRetry = () => {
      if (aborted) return;
      // Only reconnect while visible — background tabs would burn
      // attempts and immediately exhaust the cap. We still attempt
      // once the document becomes visible (listener below).
      if (document.hidden) {
        setReconnecting(false);
        return;
      }
      const attempt = attemptRef.current++;
      const delay = Math.min(30000, 1000 * Math.pow(2, attempt));
      setReconnecting(true);
      retryTimer = setTimeout(connect, delay);
    };

    const onVis = () => {
      if (!document.hidden && !wsRef.current) {
        // Document came back; restart from a clean attempt counter so
        // the first foreground retry is immediate.
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
      try { wsRef.current && wsRef.current.close(); } catch (_) {}
      wsRef.current = null;
    };
  }, [agentId]);

  const send = React.useCallback((content) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return false;
    ws.send(JSON.stringify({ type: "message", content }));
    return true;
  }, []);
  // sendCommand routes through the kernel's command dispatcher rather
  // than the message path. Used for slash commands (`/workflow run …`,
  // `/model gpt-4o-mini`, etc) so the kernel can run them without an
  // LLM round-trip.
  const sendCommand = React.useCallback((command, args) => {
    const ws = wsRef.current;
    if (!ws || ws.readyState !== 1) return false;
    ws.send(JSON.stringify({ type: "command", command, args: args || "" }));
    return true;
  }, []);

  return { connected, reconnecting, send, sendCommand };
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

// Styled confirmation dialog. Replaces the browser's `confirm()` (which
// is jarring, theme-less, and on some browsers blocks the main thread).
// Use the imperative helper `confirmDialog({...})` which returns a
// Promise<boolean> so existing call sites can swap `confirm(msg)` for
// `await confirmDialog({ title, message })` with minimal restructuring.
//
// State lives in module scope (same pattern as toasts) so any caller
// can dispatch without prop drilling.
const __CONFIRMS = { items: [], subs: new Set(), nextId: 1 };
function confirmDialog(opts) {
  return new Promise((resolve) => {
    const id = __CONFIRMS.nextId++;
    const entry = {
      id,
      title: (opts && opts.title) || "Confirm",
      message: (opts && opts.message) || "",
      confirmLabel: (opts && opts.confirmLabel) || "OK",
      cancelLabel: (opts && opts.cancelLabel) || "Cancel",
      danger: !!(opts && opts.danger),
      resolve,
    };
    __CONFIRMS.items = __CONFIRMS.items.concat([entry]);
    __CONFIRMS.subs.forEach((fn) => fn(__CONFIRMS.items));
  });
}
function ConfirmHost() {
  const [items, setItems] = React.useState(__CONFIRMS.items);
  React.useEffect(() => {
    __CONFIRMS.subs.add(setItems);
    return () => { __CONFIRMS.subs.delete(setItems); };
  }, []);
  const dismiss = (id, value) => {
    const it = __CONFIRMS.items.find((x) => x.id === id);
    if (it) it.resolve(value);
    __CONFIRMS.items = __CONFIRMS.items.filter((x) => x.id !== id);
    __CONFIRMS.subs.forEach((fn) => fn(__CONFIRMS.items));
  };
  // Only one dialog at a time; the rest queue.
  const cur = items[0];
  // Escape cancels, Enter confirms — wire here so the imperative API
  // doesn't need callers to think about keyboard.
  React.useEffect(() => {
    if (!cur) return;
    const onKey = (e) => {
      if (e.key === "Escape") dismiss(cur.id, false);
      else if (e.key === "Enter" && !e.target.matches?.("textarea")) dismiss(cur.id, true);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [cur && cur.id]);
  if (!cur) return null;
  return (
    <div className="modal-back" onClick={() => dismiss(cur.id, false)}>
      <div className="modal" style={{maxWidth:460}} onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <b className="mono">{cur.title}</b>
        </div>
        <div className="modal-body">
          <div style={{fontSize:13, color:"var(--fg-2)", whiteSpace:"pre-wrap"}}>{cur.message}</div>
        </div>
        <div className="modal-foot">
          <button className="btn ghost" onClick={() => dismiss(cur.id, false)}>{cur.cancelLabel}</button>
          <button className={"btn " + (cur.danger ? "danger" : "primary")} autoFocus onClick={() => dismiss(cur.id, true)}>
            {cur.confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}

// Skeleton primitives — placeholders shaped like the real content so
// loading states don't shove the layout around. The shimmer animation
// is CSS-only; see `panel.css`. Use `<Skel/>` for a single bar,
// `<SkelRow n=…/>` for a table-row placeholder, `<SkelCard/>` for a
// card-sized rectangle.
function Skel({ w, h, radius }) {
  return <span className="skel" style={{
    display: "inline-block",
    width: typeof w === "number" ? `${w}px` : (w || "100%"),
    height: typeof h === "number" ? `${h}px` : (h || "10px"),
    borderRadius: radius != null ? radius : 4,
    verticalAlign: "middle",
  }}/>;
}
function SkelRow({ cols }) {
  // cols can be an array of widths (number/string) or just a count.
  const widths = Array.isArray(cols)
    ? cols
    : Array.from({ length: cols || 4 }, () => `${30 + Math.floor(Math.random() * 50)}%`);
  return (
    <tr>
      {widths.map((w, i) => (
        <td key={i}><Skel w={w} h={10}/></td>
      ))}
    </tr>
  );
}
function SkelCard({ height }) {
  return (
    <div className="card">
      <Skel w={120} h={11}/>
      <div style={{marginTop:10}}><Skel w="60%" h={18}/></div>
      <div style={{marginTop:10}}><Skel w="40%" h={10}/></div>
      <div style={{marginTop:14}}><Skel w="100%" h={height || 40} radius={6}/></div>
    </div>
  );
}

// useAsyncAction(fn) — returns `[run, busy]`. Wraps an async callback so
// the button can `disabled={busy}` and show a spinner; prevents
// double-submit. Errors propagate to the caller (toast / banner).
function useAsyncAction(fn) {
  const [busy, setBusy] = React.useState(false);
  const run = React.useCallback(async (...args) => {
    if (busy) return;
    setBusy(true);
    try { return await fn(...args); }
    finally { setBusy(false); }
  }, [fn, busy]);
  return [run, busy];
}

// useHashRoute() — hash-based routing returning `{page, params, navigate}`.
//
// Routes look like `#/agents`, `#/chat`, `#/agents/{uuid}` (opens the
// drawer), `#/audit?n=100` (query string supported). Plain `#` (no path)
// maps to overview. The hook syncs both ways: setting `route.navigate(...)`
// writes to `location.hash`, and the browser's back button / popstate
// updates state.
//
// We deliberately keep this hand-rolled — pulling in react-router would
// add ~30 KB to a bundle that's otherwise dependency-free.
function parseHash(hash) {
  const raw = (hash || "").replace(/^#\/?/, "");
  if (!raw) return { page: "overview", params: {}, query: {} };
  const [path, qs] = raw.split("?");
  const parts = path.split("/").filter(Boolean);
  const page = parts[0] || "overview";
  const params = {};
  // Per-page sub-route conventions:
  //   /agents/{id}        -> drawer for that agent
  //   /chat/{agentId}     -> chat with that agent active
  //   /workflows/{id}     -> select that workflow in the list
  //   /audit/{hashPrefix} -> highlight that audit entry (future)
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
    const qs = Object.entries(query)
      .filter(([, v]) => v !== undefined && v !== null && v !== "")
      .map(([k, v]) => `${encodeURIComponent(k)}=${encodeURIComponent(v)}`)
      .join("&");
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
      // Use replaceState for same-page param updates (drawer open/close)
      // so the back button doesn't accumulate noise; pushState for cross-
      // page navigation so back works as expected.
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

// Tip — small `?` hover tooltip for non-obvious labels (CronAction
// variants, StepMode, etc.). Pure CSS would suffice for hover-only,
// but we also want focus-triggered display for keyboard users, so it
// becomes a tiny React component.
function Tip({ children }) {
  const [open, setOpen] = React.useState(false);
  return (
    <span className="tip-wrap"
          tabIndex={0}
          onMouseEnter={() => setOpen(true)}
          onMouseLeave={() => setOpen(false)}
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}>
      <span className="tip-glyph">?</span>
      {open && <span className="tip-bubble" role="tooltip">{children}</span>}
    </span>
  );
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
const __TOASTS = { items: [], timers: new Map(), subs: new Set(), nextId: 1 };
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
  if (t.ttl > 0) {
    // Track the timer so manual dismiss can cancel it. Without this,
    // dismissing a toast early still left a pending no-op timer in the
    // event loop — harmless but a slow leak under high toast throughput.
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
  rhFetch, useApi, usePolling, useAgentWs, useEventSource, usePagination, useEscapeKey, useHashRoute, useAsyncAction,
  mapAgentState, hueFromId, relativeTime, formatUptimeShort, normalizeAgent,
  renderMarkdown, downloadBlob, rowsToCsv,
  getApiKey, setApiKey, clearApiKey,
  toast, toastOk, toastWarn, toastErr, dismissToast, ToastHost, ErrorBoundary,
  Skel, SkelRow, SkelCard, Tip,
  confirmDialog, ConfirmHost,
});
