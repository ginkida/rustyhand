//! Runtime smoke test for the React panel bundle.
//!
//! The other panel tests (`panel_dashboard_test.rs`) check that strings
//! exist in the assembled HTML — necessary but not sufficient: a JSX file
//! could parse fine, get `include_str!`'d, and still throw at runtime
//! (stale closure, undefined helper, wrong destructure). The user only
//! sees that as a blank `<div id="root">`.
//!
//! This test runs the *actual compiled JS* under Node with a minimal
//! React/DOM shim and asserts every page component invokes without
//! throwing. It catches the failure modes a string-match test can't.
//!
//! ### Skip behaviour
//!
//! If `node` is not installed (e.g. minimal Linux CI image), the test
//! prints a skip message and returns. We don't want to gate the whole
//! crate on Node availability — but where Node *is* available (the
//! GitHub Actions default runner has it), the test runs and catches
//! JSX runtime errors.

use std::path::PathBuf;
use std::process::Command;

fn panel_dir() -> PathBuf {
    // crate manifest dir → static/js/panel
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("static").join("js").join("panel")
}

#[test]
fn panel_js_evaluates_under_node_without_throws() {
    // Skip cleanly if node is not on PATH — keeps the test green on
    // minimal Linux images while still running everywhere node is
    // available (Mac dev boxes + the standard GitHub Actions ubuntu
    // runner).
    if Command::new("node").arg("--version").output().is_err() {
        eprintln!("skip: node not on PATH");
        return;
    }

    let panel = panel_dir();
    let script = format!(
        r#"
const fs = require('fs');
const path = require('path');

// Minimal browser + React shim. The real bundle ships React UMD but we
// don't load it here — components only need React.createElement / hooks
// returning predictable values for an evaluation-time smoke test.
global.window = global;
global.document = {{
  getElementById: () => ({{}}),
  querySelector: () => null,
  querySelectorAll: () => [],
  addEventListener: () => {{}},
  removeEventListener: () => {{}},
  hidden: false,
  documentElement: {{
    setAttribute: () => {{}},
    style: {{ setProperty: () => {{}} }},
  }},
  createElement: () => ({{ appendChild: () => {{}}, click: () => {{}}, remove: () => {{}}, style: {{}}, href: "", download: "" }}),
  body: {{ appendChild: () => {{}}, removeChild: () => {{}} }},
}};
global.localStorage = {{ getItem: () => null, setItem: () => {{}}, removeItem: () => {{}} }};
global.fetch = () => Promise.resolve({{ ok: false, status: 500, statusText: 'mock',
  text: () => Promise.resolve(''), json: () => Promise.resolve({{}}) }});
global.WebSocket = function () {{ return {{ close: () => {{}}, send: () => {{}}, readyState: 0 }}; }};
global.location = {{ protocol: 'http:', host: 'localhost' }};
global.URL = {{ createObjectURL: () => '', revokeObjectURL: () => {{}} }};
global.Blob = function () {{ return {{}}; }};
global.confirm = () => false;
global.alert = () => {{}};
global.console = console;
global.setTimeout = setTimeout;
global.clearTimeout = clearTimeout;
global.setInterval = setInterval;
global.clearInterval = clearInterval;
global.ReactDOM = {{ createRoot: () => ({{ render: () => {{}} }}) }};
// React shim. Class components (ErrorBoundary) require `Component` as a
// real ctor we can extend; hooks return predictable values so render
// passes without throwing.
class __RhComponent {{ constructor(props) {{ this.props = props || {{}}; this.state = {{}}; }} setState() {{}} render() {{ return null; }} }}
global.React = {{
  useState: (v) => [typeof v === 'function' ? v() : v, () => {{}}],
  useEffect: () => {{}},
  useMemo: (fn) => fn(),
  useRef: (v) => ({{ current: v }}),
  useCallback: (fn) => fn,
  createElement: (type) => ({{ type }}),
  Fragment: 'fragment',
  Component: __RhComponent,
}};

const dir = {panel:?};
const files = ['api.js', 'icons.js', 'tweaks-panel.js', 'pages.js', 'app.js'];
for (const f of files) {{
  const code = fs.readFileSync(path.join(dir, f), 'utf8');
  try {{ eval(code); }}
  catch (e) {{ console.error('LOAD-FAIL ' + f + ': ' + (e.stack || e.message || e)); process.exit(1); }}
}}

// Every page + helper that webchat.rs counts on must be on window.
const need = [
  'rhFetch', 'useApi', 'usePolling', 'useAgentWs', 'normalizeAgent',
  'renderMarkdown', 'downloadBlob', 'rowsToCsv', 'setApiKey', 'clearApiKey',
  'I', 'ChannelIcon', 'Avatar', 'StateBadge', 'Spark', 'BarRow',
  'useTweaks', 'TweaksPanel',
  'OverviewPage', 'AgentsPage', 'AgentDrawer', 'ChatPage',
  'WorkflowsPage', 'AutomationPage', 'ChannelsPage', 'AnalyticsPage',
  'KnowledgePage', 'SkillsPage', 'ApprovalsPage', 'AuditPage',
  'SettingsPage', 'MemoryPage',
  // Iter 19: toast + ErrorBoundary
  'toast', 'toastOk', 'toastErr', 'ToastHost', 'ErrorBoundary',
  // Iter 20: system pages
  'McpPage', 'NetworkPage', 'BindingsPage',
  // Iter 22: pagination + keyboard helpers
  'usePagination', 'useEscapeKey',
  // Iter 24-28: routing, skeletons, async helpers, confirm, recent picks
  'useHashRoute', 'useAsyncAction',
  'Skel', 'SkelRow', 'SkelCard',
  'confirmDialog', 'ConfirmHost',
  'formatUptimeShort',
  // Iter 30-33: chat tool-trace, run inspector, bulk agents, tooltips
  'Tip',
];
const missing = need.filter((n) => global[n] === undefined);
if (missing.length) {{
  console.error('MISSING-GLOBALS ' + missing.join(','));
  process.exit(1);
}}

// Invoke each component with permissive props. We're catching
// reference errors / typos / wrong destructures, not rendering.
const pages = [
  'OverviewPage', 'AgentsPage', 'AgentDrawer', 'ChatPage',
  'WorkflowsPage', 'AutomationPage', 'ChannelsPage', 'AnalyticsPage',
  'KnowledgePage', 'SkillsPage', 'ApprovalsPage', 'AuditPage',
  'SettingsPage', 'MemoryPage',
  'McpPage', 'NetworkPage', 'BindingsPage',
];
const probeProps = {{
  go: () => {{}},
  openAgent: () => {{}},
  agent: {{ id: 'x', name: 'x', state: 'idle', model: 'x', provider: 'x',
            messages: 0, cost: 0, last: '', updated: 'now', hue: 0, group: 'x' }},
  detail: {{ model: {{}}, identity: {{}} }},
  onClose: () => {{}},
}};
for (const name of pages) {{
  try {{ global[name](probeProps); }}
  catch (e) {{ console.error('INVOKE-FAIL ' + name + ': ' + (e.stack || e.message || e)); process.exit(1); }}
}}

console.log('PANEL-SMOKE-OK');
"#
    );

    let output = Command::new("node")
        .arg("--input-type=commonjs")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("spawn node");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success() && stdout.contains("PANEL-SMOKE-OK"),
        "panel JSX smoke failed.\nstatus: {:?}\nstdout: {stdout}\nstderr: {stderr}",
        output.status.code(),
    );
}
