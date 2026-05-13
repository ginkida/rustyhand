// Production replacement for the Claude Design tweaks-panel.
//
// The design bundle ships a tweaks panel that talks to the design tool's
// host frame via postMessage. We don't have that host in the real dashboard
// so we replace it with a localStorage-backed equivalent that surfaces the
// same controls via a button in the bottom-right corner.

const __TWEAKS_STORAGE_KEY = "rh.panel.tweaks";

function useTweaks(defaults) {
  const [values, setValues] = React.useState(() => {
    try {
      const raw = localStorage.getItem(__TWEAKS_STORAGE_KEY);
      if (!raw) return defaults;
      return { ...defaults, ...JSON.parse(raw) };
    } catch (e) {
      return defaults;
    }
  });
  const setTweak = React.useCallback((keyOrEdits, val) => {
    const edits = typeof keyOrEdits === "object" && keyOrEdits !== null
      ? keyOrEdits : { [keyOrEdits]: val };
    setValues((prev) => {
      const next = { ...prev, ...edits };
      try { localStorage.setItem(__TWEAKS_STORAGE_KEY, JSON.stringify(next)); } catch (e) {}
      return next;
    });
  }, []);
  return [values, setTweak];
}

function TweaksPanel({ title = "Tweaks", children }) {
  const [open, setOpen] = React.useState(false);
  return (
    <>
      <button className="t-fab" onClick={() => setOpen(o => !o)} title="Tweaks">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1A2 2 0 1 1 4.3 17l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1A2 2 0 1 1 7 4.3l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z"/></svg>
        <span style={{marginLeft:6,fontFamily:"var(--ff-mono)",fontSize:10.5}}>tweaks</span>
      </button>
      {open && (
        <div className="t-panel">
          <div className="t-panel-head">
            <b>{title}</b>
            <button className="t-panel-x" onClick={() => setOpen(false)}>✕</button>
          </div>
          <div className="t-panel-body">{children}</div>
        </div>
      )}
    </>
  );
}

function TweakSection({ label, children }) {
  return (
    <>
      <div className="t-sect">{label}</div>
      {children}
    </>
  );
}

function TweakToggle({ label, value, onChange }) {
  return (
    <div className="t-row">
      <span className="t-lbl">{label}</span>
      <div className={"switch " + (value ? "on" : "")} onClick={() => onChange(!value)}/>
    </div>
  );
}

function TweakRadio({ label, value, options, onChange }) {
  return (
    <div className="t-row col">
      <span className="t-lbl">{label}</span>
      <div className="seg" style={{height:26}}>
        {options.map(o => {
          const v = typeof o === "object" ? o.value : o;
          const lbl = typeof o === "object" ? o.label : o;
          return (
            <button key={v} className={value === v ? "on" : ""} onClick={() => onChange(v)}>{lbl}</button>
          );
        })}
      </div>
    </div>
  );
}

function TweakSelect({ label, value, options, onChange }) {
  return (
    <div className="t-row">
      <span className="t-lbl">{label}</span>
      <select className="t-select" value={value} onChange={(e) => onChange(e.target.value)}>
        {options.map(o => {
          const v = typeof o === "object" ? o.value : o;
          const lbl = typeof o === "object" ? o.label : o;
          return <option key={v} value={v}>{lbl}</option>;
        })}
      </select>
    </div>
  );
}

Object.assign(window, { useTweaks, TweaksPanel, TweakSection, TweakToggle, TweakRadio, TweakSelect });
