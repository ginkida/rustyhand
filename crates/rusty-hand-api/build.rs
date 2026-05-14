//! Build-time check: warn when a panel JSX source is newer than its
//! compiled JS counterpart.
//!
//! The React panel is precompiled at edit time by `static/js/panel/src/build.sh`
//! (esbuild). The Rust build never invokes esbuild — it `include_str!`s the
//! resulting `.js`. That means if a contributor edits `.jsx` but forgets to
//! run `build.sh`, the binary ships stale UI without any cargo signal.
//!
//! This build script catches that drift. For each `.jsx` source it pairs the
//! file with its compiled `.js` one directory up. If the JSX is newer (or
//! the JS doesn't exist), we emit a `cargo:warning=` — visible in `cargo
//! build` output and in CI logs. We don't fail the build; the warning is
//! the cheapest signal that surfaces the problem without blocking work.
//!
//! Also emits `cargo:rerun-if-changed` for every panel asset that
//! `webchat.rs` inlines, so `cargo build` re-triggers when assets move.

use std::path::Path;
use std::time::SystemTime;

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let panel_src = manifest.join("static/js/panel/src");
    let panel_out = manifest.join("static/js/panel");

    // Tell cargo to rebuild when any inlined asset changes. webchat.rs
    // does `include_str!` on these; without these directives a contributor
    // can edit a panel file and not see it in the next `cargo run`.
    for rel in [
        "static/index_head.html",
        "static/index_body.html",
        "static/css/panel.css",
        "static/js/panel/data.js",
        "static/js/panel/api.js",
        "static/js/panel/icons.js",
        "static/js/panel/tweaks-panel.js",
        "static/js/panel/pages.js",
        "static/js/panel/app.js",
        "static/vendor/react.production.min.js",
        "static/vendor/react-dom.production.min.js",
    ] {
        println!("cargo:rerun-if-changed={}", manifest.join(rel).display());
    }

    // Freshness check. Skip silently if the src/ directory doesn't exist
    // (shouldn't happen, but better than panicking in release builds).
    if !panel_src.is_dir() {
        return;
    }

    let entries = match std::fs::read_dir(&panel_src) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsx") {
            continue;
        }
        // Watch the JSX source so cargo re-runs build.rs when it changes.
        println!("cargo:rerun-if-changed={}", path.display());

        let stem = match path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s,
            None => continue,
        };
        let js_path = panel_out.join(format!("{stem}.js"));

        let jsx_mtime = mtime(&path);
        let js_mtime = mtime(&js_path);

        match (jsx_mtime, js_mtime) {
            (Some(jsx), Some(js)) if jsx > js => {
                println!(
                    "cargo:warning=panel JSX is newer than compiled JS — run `./build.sh` in static/js/panel/src/: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            (Some(_), None) => {
                println!(
                    "cargo:warning=panel JSX has no compiled JS counterpart — run `./build.sh` in static/js/panel/src/: {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
            }
            _ => {}
        }
    }
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).ok().and_then(|m| m.modified().ok())
}
