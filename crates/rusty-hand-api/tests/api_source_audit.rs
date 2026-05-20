//! Static-source regression tests for env-var handling in HTTP routes.
//!
//! These tests pin specific patterns in `src/routes.rs` that have
//! silently regressed before: `std::env::var(...).ok()` returns
//! `Some("")` when a variable is declared but empty (a common shell
//! mistake), which slips past `Option::is_none()` checks. The fix
//! pattern is to chain `.filter(|s| !s.is_empty())` so the empty
//! string collapses to `None`.
//!
//! Lightweight static checks beat full integration tests here because
//! the bug shape is purely about a missing filter, and integration
//! tests that mutate `ANTHROPIC_API_KEY=""` would race other parallel
//! tests reading the same env var.

use std::fs;
use std::path::PathBuf;

fn routes_src() -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "routes.rs"]
        .iter()
        .collect();
    fs::read_to_string(&path).expect("read src/routes.rs")
}

/// Regression: `POST /api/providers/{name}/test` previously called
/// `std::env::var(&env_var).ok()` without an empty filter, so a
/// configured-but-empty env var like `ANTHROPIC_API_KEY=""` was
/// stored as `api_key = Some("")`. The subsequent
/// `api_key.is_none()` check missed it and the endpoint returned a
/// driver error instead of the expected 400 "API key not configured".
/// Pin the filter pattern so the bug can't sneak back in.
#[test]
fn test_provider_env_var_filters_empty_string() {
    let src = routes_src();
    let start = src
        .find("pub async fn test_provider(")
        .expect("test_provider must exist");
    // 3 KB window covers the env lookup + key_required gate.
    let window = &src[start..start + 3000];
    assert!(
        window.contains(".filter(|s| !s.is_empty())"),
        "test_provider must filter empty env var values via `.filter(|s| !s.is_empty())`"
    );
    // The bug shape: lookup chained with `.ok();` (terminator) and no
    // subsequent filter. Catch the exact regression form.
    assert!(
        !window.contains("std::env::var(&env_var).ok();"),
        "test_provider must not use bare `std::env::var(&env_var).ok();` (re-introduces Some(\"\") bug)"
    );
}
