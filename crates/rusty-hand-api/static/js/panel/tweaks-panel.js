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
    const edits = typeof keyOrEdits === "object" && keyOrEdits !== null ? keyOrEdits : { [keyOrEdits]: val };
    setValues((prev) => {
      const next = { ...prev, ...edits };
      try {
        localStorage.setItem(__TWEAKS_STORAGE_KEY, JSON.stringify(next));
      } catch (e) {
      }
      return next;
    });
  }, []);
  return [values, setTweak];
}
function TweaksPanel({ title = "Tweaks", children }) {
  const [open, setOpen] = React.useState(false);
  return /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("button", { className: "t-fab", onClick: () => setOpen((o) => !o), title: "Tweaks" }, /* @__PURE__ */ React.createElement("svg", { width: "14", height: "14", viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: "2", strokeLinecap: "round", strokeLinejoin: "round" }, /* @__PURE__ */ React.createElement("circle", { cx: "12", cy: "12", r: "3" }), /* @__PURE__ */ React.createElement("path", { d: "M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1A2 2 0 1 1 4.3 17l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1A2 2 0 1 1 7 4.3l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1Z" })), /* @__PURE__ */ React.createElement("span", { style: { marginLeft: 6, fontFamily: "var(--ff-mono)", fontSize: 10.5 } }, "tweaks")), open && /* @__PURE__ */ React.createElement("div", { className: "t-panel" }, /* @__PURE__ */ React.createElement("div", { className: "t-panel-head" }, /* @__PURE__ */ React.createElement("b", null, title), /* @__PURE__ */ React.createElement("button", { className: "t-panel-x", onClick: () => setOpen(false) }, "\u2715")), /* @__PURE__ */ React.createElement("div", { className: "t-panel-body" }, children)));
}
function TweakSection({ label, children }) {
  return /* @__PURE__ */ React.createElement(React.Fragment, null, /* @__PURE__ */ React.createElement("div", { className: "t-sect" }, label), children);
}
function TweakToggle({ label, value, onChange }) {
  return /* @__PURE__ */ React.createElement("div", { className: "t-row" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, label), /* @__PURE__ */ React.createElement("div", { className: "switch " + (value ? "on" : ""), onClick: () => onChange(!value) }));
}
function TweakRadio({ label, value, options, onChange }) {
  return /* @__PURE__ */ React.createElement("div", { className: "t-row col" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, label), /* @__PURE__ */ React.createElement("div", { className: "seg", style: { height: 26 } }, options.map((o) => {
    const v = typeof o === "object" ? o.value : o;
    const lbl = typeof o === "object" ? o.label : o;
    return /* @__PURE__ */ React.createElement("button", { key: v, className: value === v ? "on" : "", onClick: () => onChange(v) }, lbl);
  })));
}
function TweakSelect({ label, value, options, onChange }) {
  return /* @__PURE__ */ React.createElement("div", { className: "t-row" }, /* @__PURE__ */ React.createElement("span", { className: "t-lbl" }, label), /* @__PURE__ */ React.createElement("select", { className: "t-select", value, onChange: (e) => onChange(e.target.value) }, options.map((o) => {
    const v = typeof o === "object" ? o.value : o;
    const lbl = typeof o === "object" ? o.label : o;
    return /* @__PURE__ */ React.createElement("option", { key: v, value: v }, lbl);
  })));
}
Object.assign(window, { useTweaks, TweaksPanel, TweakSection, TweakToggle, TweakRadio, TweakSelect });
