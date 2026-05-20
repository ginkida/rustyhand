//! Static audit of the panel JSX sources for IIFE-scope bugs.
//!
//! Background: v0.7.65 wrapped every compiled `static/js/panel/*.js`
//! file in `(function(){ ... })();` so top-level `const` doesn't leak
//! across script tags. The trade-off is that any bare hook call inside
//! a `.jsx` source (e.g. `useRef(null)`) must be either:
//!   (a) destructured at the top of that file, e.g.
//!       `const { useState, useEffect, useRef, useMemo } = React;`
//!   (b) called as `React.useRef(...)` (qualified)
//!
//! v0.7.65 → v0.7.68 hit this twice: pages.jsx and app.jsx independently
//! drifted out of sync with their bare-hook usage. The runtime smoke
//! test (`panel_jsx_smoke.rs`) only invokes the public Page components
//! — so bugs hiding in CommandPalette / LoginScreen / hooks helpers
//! slip past it. This test reads the .jsx sources directly and fails
//! the build if any bare hook is referenced without a matching
//! destructure entry. Cheap, exhaustive, file-static.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

const HOOKS: &[&str] = &[
    "useState",
    "useEffect",
    "useRef",
    "useMemo",
    "useCallback",
    "useReducer",
    "useContext",
    "useLayoutEffect",
    "useImperativeHandle",
];

fn panel_src_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("static")
        .join("js")
        .join("panel")
        .join("src")
}

/// Extract the names destructured from React at the top of the file.
/// Looks for the first `const { ... } = React;` line.
fn destructured_hooks(src: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("const {") {
            if let Some(inner) = rest.split('}').next() {
                if t.contains("= React;") || t.contains("= React") {
                    for name in inner.split(',') {
                        let n = name.trim();
                        if !n.is_empty() {
                            out.insert(n.to_string());
                        }
                    }
                    break;
                }
            }
        }
    }
    out
}

/// Find bare hook calls (e.g. `useState(`, `useRef(`) NOT preceded by
/// `React.` and NOT inside the destructure line itself. Returns a list
/// of `(hook_name, line_number)` so the failure message points at the
/// exact source location.
fn bare_hook_calls(src: &str) -> Vec<(String, usize)> {
    let mut hits = Vec::new();
    for (idx, line) in src.lines().enumerate() {
        let t = line.trim_start();
        // Skip the destructure line(s).
        if t.starts_with("const {") && line.contains("= React") {
            continue;
        }
        // Skip comments — single-line `//` and `*` block-comment bodies.
        if t.starts_with("//") || t.starts_with("*") {
            continue;
        }
        for &hook in HOOKS {
            // Find `<hook>(` where the char just before <hook> is not a
            // word-char and not `.` (which would be `React.useRef(`).
            let needle = format!("{hook}(");
            let mut start = 0;
            while let Some(pos) = line[start..].find(&needle) {
                let abs = start + pos;
                let before = if abs == 0 {
                    '\0'
                } else {
                    line.as_bytes()[abs - 1] as char
                };
                let is_word_boundary = !(before.is_ascii_alphanumeric() || before == '_');
                let is_dot_qualified = before == '.';
                if is_word_boundary && !is_dot_qualified {
                    hits.push((hook.to_string(), idx + 1));
                }
                start = abs + needle.len();
            }
        }
    }
    hits
}

#[test]
fn every_bare_react_hook_call_is_destructured() {
    let src_dir = panel_src_dir();
    let mut problems: Vec<String> = Vec::new();
    for entry in fs::read_dir(&src_dir).expect("read panel src dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsx") {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let src = fs::read_to_string(&path).expect("read jsx");
        let destructured = destructured_hooks(&src);
        for (hook, line) in bare_hook_calls(&src) {
            if !destructured.contains(&hook) {
                problems.push(format!(
                    "{name}:{line} — bare `{hook}(` but `{hook}` not in `const {{ ... }} = React;` destructure (add it, or call as `React.{hook}(`)"
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "panel JSX sources have undeclared bare hook calls — these will throw `<hook> is not defined` at runtime inside the IIFE-wrapped bundle:\n{}",
        problems.join("\n")
    );
}

/// Sanity check the audit itself: if we accidentally swallow the
/// destructure of `useState` (the most common bare hook), the assertion
/// above would never fire. This test deliberately points the helpers at
/// a synthetic source and proves the detector works.
#[test]
fn detector_catches_bare_hook_without_destructure() {
    let src = "function Foo() {\n  const x = useState(0);\n  return null;\n}\n";
    let destructured = destructured_hooks(src);
    let hits = bare_hook_calls(src);
    assert!(destructured.is_empty(), "no destructure expected");
    assert!(
        hits.iter().any(|(h, _)| h == "useState"),
        "detector missed bare useState"
    );
}

#[test]
fn detector_ignores_react_qualified_call() {
    let src = "function Foo() {\n  const x = React.useState(0);\n  return null;\n}\n";
    let hits = bare_hook_calls(src);
    assert!(
        !hits.iter().any(|(h, _)| h == "useState"),
        "detector misflagged React.useState as bare"
    );
}

#[test]
fn detector_reads_destructure_correctly() {
    let src = "const { useState, useEffect, useRef } = React;\nfunction F() { useState(0); }\n";
    let d = destructured_hooks(src);
    assert!(d.contains("useState"));
    assert!(d.contains("useEffect"));
    assert!(d.contains("useRef"));
}

#[test]
fn spawn_modal_accepts_catalog_auth_statuses() {
    let src = fs::read_to_string(panel_src_dir().join("pages.jsx")).expect("read pages.jsx");
    assert!(
        src.contains("auth === \"configured\""),
        "SpawnAgentModal must treat /api/providers auth_status='configured' as usable"
    );
    assert!(
        src.contains("auth === \"not_required\""),
        "SpawnAgentModal must treat local/no-key providers as usable"
    );
}

#[test]
fn provider_status_views_handle_configured_and_not_required() {
    // Regression: ProvidersList (Overview), ProviderState (Health), and the
    // Settings provider table all read `p.auth_status` from /api/providers.
    // The backend's `AuthStatus` enum serialises as "configured" / "missing"
    // / "not_required" — never "ok". Pre-fix these three call sites matched
    // only "ok", so every configured provider rendered as "not set" /
    // unknown status / raw text. We pin all three together so future edits
    // that drift one branch trip a hard error.
    let src = fs::read_to_string(panel_src_dir().join("pages.jsx")).expect("read pages.jsx");

    // ProviderState (Health page) — Connected label
    let provider_state_idx = src
        .find("function ProviderState()")
        .expect("ProviderState component must exist");
    let provider_state_slice = &src[provider_state_idx..provider_state_idx + 1500];
    assert!(
        provider_state_slice.contains("\"configured\"") && provider_state_slice.contains("Connected"),
        "ProviderState must label auth_status='configured' as Connected"
    );
    assert!(
        provider_state_slice.contains("\"not_required\""),
        "ProviderState must handle local/no-key providers (auth_status='not_required')"
    );

    // Settings provider table — `set` badge. Anchor on the badge JSX
    // directly since "LLM providers" appears earlier in an empty-state hint.
    let settings_idx = src
        .find("badge live\">set")
        .expect("Settings provider table `set` badge must exist");
    // Walk backward to the start of the table's logic block (`auth_status`
    // destructure happens within ~400 chars of the badge JSX).
    let block_start = settings_idx.saturating_sub(400);
    let settings_slice = &src[block_start..settings_idx + 400];
    assert!(
        settings_slice.contains("\"configured\""),
        "Settings provider table must render the `set` badge for auth_status='configured'"
    );
    assert!(
        settings_slice.contains("\"not_required\""),
        "Settings provider table must render the `local` badge for auth_status='not_required'"
    );
}

#[test]
fn onboarding_uses_catalog_models_and_real_tools() {
    let src = fs::read_to_string(panel_src_dir().join("app.jsx")).expect("read app.jsx");
    assert!(
        src.contains("case \"kimi\": return \"kimi-for-coding\""),
        "onboarding must create Kimi agents with the catalog's canonical model id"
    );
    assert!(
        src.contains("case \"deepseek\": return \"deepseek-v4-flash\""),
        "onboarding must use the current DeepSeek default model"
    );
    assert!(
        src.contains("tools = [\"web_search\", \"web_fetch\", \"memory_recall\"]"),
        "onboarding must write real tool names, not a profile label like research"
    );
}
