//! Shared `reqwest::Client` with connection pooling and optional proxy support.
//!
//! Creating `reqwest::Client::new()` on every request is wasteful: it
//! spins up a fresh TLS/HTTP2 connection pool each time. This module
//! exposes a single process-wide client configured with sensible
//! pooling and timeout defaults. Call `shared()` to borrow it.
//!
//! For proxy support: call `build_with_proxy()` to get a one-off client
//! that routes through the configured proxy. Use this for scraping
//! hostile origins (e.g. OLX) where you need residential IPs.

use rusty_hand_types::config::ProxyConfig;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::warn;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return the shared process-wide HTTP client.
///
/// The first call initializes the client with:
/// - 60s request timeout
/// - 60s idle connection timeout (keep-alive reuse)
/// - 10 max idle connections per host
///
/// If custom configuration is required (e.g. a proxy, different TLS
/// root store, or disabled redirects), use `build_with_proxy()` or
/// create a dedicated `reqwest::Client::builder()` instead.
pub fn shared() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}

/// Build a `reqwest::Client` that routes through the given proxy when configured.
///
/// If `cfg.is_enabled()` is false, returns a plain client with standard pool defaults.
/// On proxy build failure (malformed URL, etc.), logs a warning and falls back to a
/// non-proxied client so the system stays usable.
///
/// `timeout` is the per-request timeout (use 60s for typical web fetches; longer for
/// scraping with high-latency residential proxies).
///
/// Note: per-host bypass via `no_proxy` is handled by callers — they pre-build both
/// a proxy client and a direct client and pick at request time. This keeps the
/// builder simple and avoids constructing a fresh Client per request.
pub fn build_with_proxy(cfg: &ProxyConfig, timeout: Duration) -> reqwest::Client {
    // Reusable builder for the standard pool defaults.
    //
    // SECURITY: install a redirect policy that re-validates every hop against
    // the SSRF rules (with an empty allowlist — an attacker-controlled redirect
    // must not inherit the initial URL's allowlist trust). Without this, a
    // public URL that passes the pre-flight SSRF check could 3xx-redirect to an
    // internal/loopback/cloud-metadata host and reqwest's default policy would
    // silently follow it. These clients are used only for web fetching.
    let direct_builder = || {
        reqwest::Client::builder()
            .timeout(timeout)
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 10 {
                    return attempt.error("too many redirects");
                }
                match crate::web_fetch::check_ssrf_with_allowlist(attempt.url().as_str(), &[]) {
                    Ok(()) => attempt.follow(),
                    Err(_) => attempt.error("SSRF blocked: redirect to a restricted target"),
                }
            }))
    };

    if !cfg.is_enabled() {
        return direct_builder()
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
    }

    let proxy_result = reqwest::Proxy::all(&cfg.url).map(|p| {
        if !cfg.username.is_empty() {
            p.basic_auth(&cfg.username, &cfg.password)
        } else {
            p
        }
    });

    let proxy = match proxy_result {
        Ok(p) => p,
        Err(e) => {
            // SECURITY: log only the URL (without embedded auth if any) and the error.
            // We do not log username/password fields.
            warn!(
                proxy_url = %cfg.url,
                error = %e,
                "Failed to build HTTP proxy — falling back to direct connection"
            );
            return direct_builder()
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
        }
    };

    direct_builder().proxy(proxy).build().unwrap_or_else(|e| {
        warn!(error = %e, "Failed to build proxy client — using direct connection");
        reqwest::Client::new()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_no_proxy_when_disabled() {
        let cfg = ProxyConfig::default();
        let _client = build_with_proxy(&cfg, Duration::from_secs(30));
        // Just verifies it builds without panicking.
    }

    #[test]
    fn build_with_basic_auth_proxy() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            no_proxy: vec![],
        };
        let _client = build_with_proxy(&cfg, Duration::from_secs(30));
    }

    #[test]
    fn build_falls_back_on_invalid_url() {
        let cfg = ProxyConfig {
            url: "not-a-valid-url".to_string(),
            username: String::new(),
            password: String::new(),
            no_proxy: vec![],
        };
        // Must not panic — warns and falls back to direct client.
        let _client = build_with_proxy(&cfg, Duration::from_secs(30));
    }

    #[test]
    fn build_with_socks5_proxy() {
        let cfg = ProxyConfig {
            url: "socks5://proxy.example.com:1080".to_string(),
            username: "user".to_string(),
            password: "pass".to_string(),
            no_proxy: vec![],
        };
        // SOCKS5 proxies should also build successfully.
        let _client = build_with_proxy(&cfg, Duration::from_secs(30));
    }
}
