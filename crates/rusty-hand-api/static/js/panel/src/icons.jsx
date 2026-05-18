// Icons & tiny visual primitives for RustyHand panel.
// Plain SVG components, kept small and consistent (16px default stroke 2).

const Icon = ({ children, size = 16, className = "" }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="none"
       stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"
       className={className}>{children}</svg>
);

const I = {
  overview:   () => <Icon><rect x="3" y="3" width="7" height="9"/><rect x="14" y="3" width="7" height="5"/><rect x="14" y="12" width="7" height="9"/><rect x="3" y="16" width="7" height="5"/></Icon>,
  agents:     () => <Icon><circle cx="9" cy="8" r="3.2"/><path d="M3 20c1.2-3.2 3.6-5 6-5s4.8 1.8 6 5"/><circle cx="17" cy="6" r="2.4"/><path d="M15 12c2 .2 4 1.6 4.8 3.6"/></Icon>,
  chat:       () => <Icon><path d="M21 12a8 8 0 1 1-3.4-6.6L21 4l-1 4.4A8 8 0 0 1 21 12Z"/><path d="M8 11h.01M12 11h.01M16 11h.01"/></Icon>,
  workflows:  () => <Icon><rect x="3" y="3" width="6" height="6" rx="1"/><rect x="15" y="3" width="6" height="6" rx="1"/><rect x="9" y="15" width="6" height="6" rx="1"/><path d="M6 9v3a3 3 0 0 0 3 3h3M18 9v3a3 3 0 0 1-3 3h-3"/></Icon>,
  automation: () => <Icon><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"/></Icon>,
  channels:   () => <Icon><path d="M4 4h16v12H5l-1 4z"/><path d="M8 9h8M8 12h5"/></Icon>,
  analytics:  () => <Icon><path d="M3 3v18h18"/><path d="M7 14l4-4 3 3 5-6"/></Icon>,
  knowledge:  () => <Icon><circle cx="6" cy="6" r="2.2"/><circle cx="18" cy="6" r="2.2"/><circle cx="12" cy="14" r="2.2"/><circle cx="6" cy="18" r="2.2"/><circle cx="18" cy="18" r="2.2"/><path d="M8 7l3 6M16 7l-3 6M12 16v2M7 17l4-2M17 17l-4-2"/></Icon>,
  skills:     () => <Icon><path d="M12 2L4 6v6c0 5 3.6 8.8 8 10 4.4-1.2 8-5 8-10V6l-8-4Z"/><path d="m9 12 2 2 4-4"/></Icon>,
  approvals:  () => <Icon><path d="M9 11l3 3 7-7"/><path d="M20 12v6a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h7"/></Icon>,
  audit:      () => <Icon><path d="M14 3v4a1 1 0 0 0 1 1h4"/><path d="M19 8v11a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7Z"/><path d="M9 13h6M9 17h4"/></Icon>,
  settings:   () => <Icon><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1A2 2 0 1 1 4.3 17l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1A2 2 0 1 1 7 4.3l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/></Icon>,
  search:     () => <Icon><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></Icon>,
  plus:       () => <Icon><path d="M12 5v14M5 12h14"/></Icon>,
  play:       () => <Icon><path d="M6 4l14 8-14 8z"/></Icon>,
  pause:      () => <Icon><rect x="6" y="5" width="4" height="14"/><rect x="14" y="5" width="4" height="14"/></Icon>,
  more:       () => <Icon><circle cx="5" cy="12" r="1.5"/><circle cx="12" cy="12" r="1.5"/><circle cx="19" cy="12" r="1.5"/></Icon>,
  send:       () => <Icon><path d="M22 2L11 13"/><path d="M22 2L15 22l-4-9-9-4 20-7Z"/></Icon>,
  filter:     () => <Icon><path d="M3 4h18l-7 9v6l-4 2v-8z"/></Icon>,
  refresh:    () => <Icon><path d="M21 12a9 9 0 1 1-3-6.7"/><path d="M21 4v5h-5"/></Icon>,
  close:      () => <Icon><path d="M18 6L6 18M6 6l12 12"/></Icon>,
  download:   () => <Icon><path d="M12 3v12M6 11l6 6 6-6M5 21h14"/></Icon>,
  copy:       () => <Icon><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V5a2 2 0 0 1 2-2h10"/></Icon>,
  shield:     () => <Icon><path d="M12 2L4 6v6c0 5 3.6 8.8 8 10 4.4-1.2 8-5 8-10V6l-8-4Z"/></Icon>,
  zap:        () => <Icon><path d="M13 2L3 14h8l-1 8 10-12h-8z"/></Icon>,
  telegram:   () => <Icon><path d="M22 2L11 13"/><path d="M22 2L15 22l-4-9-9-4 20-7Z"/></Icon>,
  discord:    () => <Icon><path d="M8 4c-1 2-2 4-2 8s1 6 2 8c1 0 2-1 3-2-2-1-3-3-3-5s1-4 3-5c-1-2-2-4-3-4Z"/><path d="M16 4c1 2 2 4 2 8s-1 6-2 8c-1 0-2-1-3-2 2-1 3-3 3-5s-1-4-3-5c1-2 2-4 3-4Z"/></Icon>,
  slack:      () => <Icon><rect x="3" y="10" width="8" height="4" rx="2"/><rect x="13" y="10" width="8" height="4" rx="2"/><rect x="10" y="3" width="4" height="8" rx="2"/><rect x="10" y="13" width="4" height="8" rx="2"/></Icon>,
  webhook:    () => <Icon><circle cx="6" cy="18" r="3"/><circle cx="18" cy="6" r="3"/><path d="M9 17l5-9M12 8l6 0M7 15l-3 3"/></Icon>,
  event:      () => <Icon><path d="M13 2L3 14h8l-1 8 10-12h-8z"/></Icon>,
  cron:       () => <Icon><circle cx="12" cy="12" r="9"/><path d="M12 7v5l3 2"/></Icon>,
  check:      () => <Icon><path d="M5 12l5 5 9-12"/></Icon>,
  warn:       () => <Icon><path d="M12 3l10 18H2L12 3z"/><path d="M12 10v4M12 18h.01"/></Icon>,
  link:       () => <Icon><path d="M10 14a5 5 0 0 0 7 0l3-3a5 5 0 0 0-7-7l-1 1"/><path d="M14 10a5 5 0 0 0-7 0l-3 3a5 5 0 0 0 7 7l1-1"/></Icon>,
  arrowR:     () => <Icon><path d="M5 12h14M13 6l6 6-6 6"/></Icon>,
  cpu:        () => <Icon><rect x="6" y="6" width="12" height="12" rx="2"/><rect x="10" y="10" width="4" height="4"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M1 9h3M1 15h3M20 9h3M20 15h3"/></Icon>,
  trash:      () => <Icon><path d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M5 6l1 14a2 2 0 0 0 2 2h8a2 2 0 0 0 2-2l1-14M10 11v6M14 11v6"/></Icon>,
  edit:       () => <Icon><path d="M12 20h9"/><path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4Z"/></Icon>,
};

const ChannelIcon = ({ kind }) => {
  const k = (kind || "").toLowerCase();
  if (k === "telegram") return <I.telegram/>;
  if (k === "discord")  return <I.discord/>;
  if (k === "slack")    return <I.slack/>;
  if (k === "webhook")  return <I.webhook/>;
  if (k === "event")    return <I.event/>;
  return <I.link/>;
};

// Avatar circle with color from agent hue.
const Avatar = ({ agent, size = "md" }) => {
  const cls = size === "lg" ? "avatar lg" : size === "xl" ? "avatar xl" : "avatar";
  const bg = `linear-gradient(135deg, hsl(${agent.hue} 70% 60%), hsl(${(agent.hue + 35) % 360} 70% 42%))`;
  return (
    <div className={cls} style={{ background: bg }}>
      <span>{agent.emoji || agent.name.charAt(0).toUpperCase()}</span>
    </div>
  );
};

const StateBadge = ({ state }) => {
  const map = {
    running:   { cls: "live",   text: "running" },
    idle:      { cls: "idle",   text: "idle" },
    error:     { cls: "error",  text: "error" },
    waiting:   { cls: "warn",   text: "waiting" },
    scheduled: { cls: "violet", text: "scheduled" },
    stopped:   { cls: "idle",   text: "stopped" },
  };
  const m = map[state] || { cls: "idle", text: state };
  return <span className={`badge ${m.cls}`}><span className={`dot ${m.cls === "live" ? "live" : m.cls === "warn" ? "warn" : m.cls === "error" ? "err" : "idle"}`}/>{m.text}</span>;
};

// Sparkline.
const Spark = ({ data, width = 88, height = 28, color = "var(--rust)", fill = true }) => {
  // Defensive: coerce to numbers, drop NaN/null/undefined entries, and
  // bail out on degenerate inputs (no points, or single point — both
  // would emit NaN coordinates from the 1/(n-1) divisor).
  const clean = Array.isArray(data) ? data.map((v) => Number(v)).filter((v) => Number.isFinite(v)) : [];
  if (clean.length < 2) return null;
  const max = Math.max(...clean), min = Math.min(...clean);
  const span = Math.max(0.0001, max - min);
  const pts = clean.map((v, i) => {
    const x = (i / (clean.length - 1)) * (width - 2) + 1;
    const y = height - 2 - ((v - min) / span) * (height - 4);
    return [x, y];
  });
  const path = pts.map(([x, y], i) => (i === 0 ? `M${x},${y}` : `L${x},${y}`)).join(" ");
  const area = `${path} L${width - 1},${height - 1} L1,${height - 1} Z`;
  return (
    <svg className="spark" width={width} height={height}>
      {fill && <path d={area} fill={color} opacity="0.15"/>}
      <path d={path} fill="none" stroke={color} strokeWidth="1.5" strokeLinejoin="round" strokeLinecap="round"/>
    </svg>
  );
};

// Bar chart (horizontal label rows).
const BarRow = ({ label, value, max, unit = "" }) => {
  const w = Math.max(2, Math.round((value / max) * 100));
  return (
    <div className="bar-row">
      <span className="lbl">{label}</span>
      <span className="bar"><span style={{ width: `${w}%` }}/></span>
      <span className="val">{value.toFixed(2)}{unit}</span>
    </div>
  );
};

Object.assign(window, { I, ChannelIcon, Avatar, StateBadge, Spark, BarRow });
