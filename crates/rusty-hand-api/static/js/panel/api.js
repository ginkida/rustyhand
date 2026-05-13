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
  const wsRef = React.useRef(null);
  const handlerRef = React.useRef(onEvent);
  handlerRef.current = onEvent;
  React.useEffect(() => {
    if (!agentId) return;
    const proto = window.location.protocol === "https:" ? "wss:" : "ws:";
    const key = getApiKey();
    const tokenQs = key ? `?token=${encodeURIComponent(key)}` : "";
    const url = `${proto}//${window.location.host}/api/agents/${encodeURIComponent(agentId)}/ws${tokenQs}`;
    let ws;
    try {
      ws = new WebSocket(url);
    } catch (e) {
      console.warn("ws open failed", e);
      return;
    }
    wsRef.current = ws;
    ws.onopen = () => setConnected(true);
    ws.onclose = () => setConnected(false);
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
    return () => {
      try {
        ws.close();
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
  return { connected, send };
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
Object.assign(window, { rhFetch, useApi, usePolling, useAgentWs, mapAgentState, hueFromId, relativeTime, normalizeAgent, renderMarkdown, downloadBlob, rowsToCsv, getApiKey, setApiKey, clearApiKey });
