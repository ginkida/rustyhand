//! Shared `reqwest::Client` with connection pooling.
//!
//! Creating `reqwest::Client::new()` on every request is wasteful: it
//! spins up a fresh TLS/HTTP2 connection pool each time. This module
//! exposes a single process-wide client configured with sensible
//! pooling and timeout defaults. Call `shared()` to borrow it.

use std::sync::OnceLock;

static HTTP_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// Return the shared process-wide HTTP client.
///
/// The first call initializes the client with:
/// - 60s request timeout
/// - 60s idle connection timeout (keep-alive reuse)
/// - 10 max idle connections per host
///
/// If custom configuration is required (e.g. a proxy, different TLS
/// root store, or disabled redirects), create a dedicated
/// `reqwest::Client::builder()` instead.
pub fn shared() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .pool_idle_timeout(std::time::Duration::from_secs(60))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new())
    })
}
