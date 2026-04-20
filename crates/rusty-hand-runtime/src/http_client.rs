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
/// If `cfg.is_enabled()` is false, returns a plain client (same defaults as `shared()`).
/// On proxy build failure (malformed URL, etc.), logs a warning and falls back to a
/// non-proxied client so the system stays usable.
///
/// `timeout` is the per-request timeout (use 60s for typical web fetches; longer for
/// scraping with high latency residential proxies).
///
/// `target_host` lets us check `no_proxy` rules — if set and host is bypassed,
/// returns a non-proxied client even when proxy is enabled.
pub fn build_with_proxy(
    cfg: &ProxyConfig,
    timeout: Duration,
    target_host: Option<&str>,
) -> reqwest::Client {
    let bypass = target_host.map(|h| cfg.should_bypass(h)).unwrap_or(false);

    if !cfg.is_enabled() || bypass {
        return reqwest::Client::builder()
            .timeout(timeout)
            .pool_idle_timeout(Duration::from_secs(60))
            .pool_max_idle_per_host(10)
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
            warn!(
                proxy_url = %cfg.url,
                error = %e,
                "Failed to build HTTP proxy — falling back to direct connection"
            );
            return reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .unwrap_or_else(|_| reqwest::Client::new());
        }
    };

    reqwest::Client::builder()
        .timeout(timeout)
        .pool_idle_timeout(Duration::from_secs(60))
        .pool_max_idle_per_host(10)
        .proxy(proxy)
        .build()
        .unwrap_or_else(|e| {
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
        let _client = build_with_proxy(&cfg, Duration::from_secs(30), None);
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
        let _client = build_with_proxy(&cfg, Duration::from_secs(30), None);
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
        let _client = build_with_proxy(&cfg, Duration::from_secs(30), None);
    }

    #[test]
    fn bypasses_host_in_no_proxy() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: String::new(),
            password: String::new(),
            no_proxy: vec!["internal.corp".to_string(), "*.local".to_string()],
        };
        // These should bypass the proxy:
        let _ = build_with_proxy(&cfg, Duration::from_secs(30), Some("internal.corp"));
        let _ = build_with_proxy(&cfg, Duration::from_secs(30), Some("foo.local"));
        // This should go through the proxy:
        let _ = build_with_proxy(&cfg, Duration::from_secs(30), Some("olx.kz"));
    }
}
