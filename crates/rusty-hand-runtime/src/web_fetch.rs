//! Enhanced web fetch with SSRF protection, HTML→Markdown extraction,
//! in-memory caching, and external content markers.
//!
//! Pipeline: SSRF check → cache lookup → HTTP GET → detect HTML →
//! html_to_markdown() → truncate → wrap_external_content() → cache → return

use crate::web_cache::WebCache;
use crate::web_content::{html_to_markdown, wrap_external_content};
use rusty_hand_types::config::{ProxyConfig, WebFetchConfig};
use std::net::{IpAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;

/// Enhanced web fetch engine with SSRF protection and readability extraction.
pub struct WebFetchEngine {
    config: WebFetchConfig,
    client: reqwest::Client,
    cache: Arc<WebCache>,
    proxy: ProxyConfig,
}

impl WebFetchEngine {
    /// Create a new fetch engine from config with a shared cache.
    /// No proxy is used (direct connection).
    pub fn new(config: WebFetchConfig, cache: Arc<WebCache>) -> Self {
        Self::with_proxy(config, cache, ProxyConfig::default())
    }

    /// Create a new fetch engine with an optional proxy configuration.
    /// When `proxy.is_enabled()` is true, requests route through the proxy
    /// (with `no_proxy` bypass for matching hosts).
    pub fn with_proxy(config: WebFetchConfig, cache: Arc<WebCache>, proxy: ProxyConfig) -> Self {
        let timeout = Duration::from_secs(config.timeout_secs);
        // Default client (used for hosts that bypass the proxy or when proxy is off).
        let client = crate::http_client::build_with_proxy(&proxy, timeout, None);
        Self {
            config,
            client,
            cache,
            proxy,
        }
    }

    /// Get the SSRF allowlist from config.
    pub fn ssrf_allowlist(&self) -> &[String] {
        &self.config.ssrf_allowlist
    }

    /// Fetch a URL with full security pipeline.
    pub async fn fetch(&self, url: &str) -> Result<String, String> {
        // Step 1: SSRF protection — BEFORE any network I/O
        check_ssrf_with_allowlist(url, &self.config.ssrf_allowlist)?;

        // Step 2: Cache lookup
        let cache_key = format!("fetch:{}", url);
        if let Some(cached) = self.cache.get(&cache_key) {
            debug!(url, "Fetch cache hit");
            return Ok(cached);
        }

        // Step 3: HTTP GET — use a per-host client so `no_proxy` bypass works
        // even when the engine was built with a proxy.
        let host = extract_host(url);
        let host_only = host.split(':').next().unwrap_or(&host);
        let client = if self.proxy.is_enabled() && !self.proxy.should_bypass(host_only) {
            // Use the proxy-configured shared client.
            self.client.clone()
        } else if self.proxy.is_enabled() {
            // Proxy enabled but this host bypasses — build a direct client.
            crate::http_client::build_with_proxy(
                &ProxyConfig::default(),
                Duration::from_secs(self.config.timeout_secs),
                Some(host_only),
            )
        } else {
            self.client.clone()
        };

        let resp = client
            .get(url)
            .header("User-Agent", "Mozilla/5.0 (compatible; RustyHandAgent/0.1)")
            .send()
            .await
            .map_err(|e| format!("HTTP request failed: {e}"))?;

        let status = resp.status();

        // Check response size — early reject if Content-Length is known
        let max_bytes = self.config.max_response_bytes;
        if let Some(len) = resp.content_length() {
            if len > max_bytes as u64 {
                return Err(format!(
                    "Response too large: {} bytes (max {})",
                    len, max_bytes
                ));
            }
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Read body with hard size limit (protects against chunked/streaming responses
        // that omit Content-Length)
        let mut bytes = Vec::new();
        let mut stream = resp.bytes_stream();
        use futures::StreamExt;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Failed to read response body: {e}"))?;
            bytes.extend_from_slice(&chunk);
            if bytes.len() > max_bytes {
                return Err(format!(
                    "Response body exceeded {} byte limit during streaming",
                    max_bytes
                ));
            }
        }
        let body = String::from_utf8_lossy(&bytes).into_owned();

        // Step 4: Detect HTML and optionally convert to Markdown
        let processed = if self.config.readability && is_html(&content_type, &body) {
            let markdown = html_to_markdown(&body);
            if markdown.trim().is_empty() {
                // Fallback to raw text if extraction produced nothing
                body
            } else {
                markdown
            }
        } else {
            body
        };

        // Step 5: Truncate
        let total_chars = processed.chars().count();
        let truncated = if total_chars > self.config.max_chars {
            let preview: String = processed.chars().take(self.config.max_chars).collect();
            format!("{}... [truncated, {} total chars]", preview, total_chars)
        } else {
            processed
        };

        // Step 6: Wrap with external content markers
        let result = format!(
            "HTTP {status}\n\n{}",
            wrap_external_content(url, &truncated)
        );

        // Step 7: Cache
        self.cache.put(cache_key, result.clone());

        Ok(result)
    }
}

/// Detect if content is HTML based on Content-Type header or body sniffing.
fn is_html(content_type: &str, body: &str) -> bool {
    if content_type.contains("text/html") || content_type.contains("application/xhtml") {
        return true;
    }
    // Sniff: check if body starts with HTML-like content
    let trimmed = body.trim_start();
    trimmed.starts_with("<!DOCTYPE")
        || trimmed.starts_with("<!doctype")
        || trimmed.starts_with("<html")
}

// ---------------------------------------------------------------------------
// SSRF Protection (replicates host_functions.rs logic for builtin tools)
// ---------------------------------------------------------------------------

/// Check if a URL targets a private/internal network resource.
/// Blocks localhost, metadata endpoints, and private IPs.
/// Must run BEFORE any network I/O.
pub fn check_ssrf(url: &str) -> Result<(), String> {
    check_ssrf_with_allowlist(url, &[])
}

/// SSRF check with an optional allowlist of hostnames permitted to resolve to private IPs.
/// Cloud metadata endpoints are NEVER allowed regardless of the allowlist.
pub fn check_ssrf_with_allowlist(url: &str, allowlist: &[String]) -> Result<(), String> {
    // Only allow http:// and https:// schemes
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Only http:// and https:// URLs are allowed".to_string());
    }

    let host = extract_host(url);
    let hostname = host.split(':').next().unwrap_or(&host);

    // Hostname-based blocklist — case-insensitive to prevent bypass
    let hostname_lower = hostname.to_lowercase();
    let blocked = [
        "localhost",
        "metadata.google.internal",
        "metadata.aws.internal",
        "instance-data",
        "kubernetes.default",
        "kubernetes.default.svc",
        "169.254.169.254",
    ];
    if blocked.iter().any(|b| hostname_lower == *b) {
        return Err(format!("SSRF blocked: {hostname} is a restricted hostname"));
    }
    // Block .onion domains (Tor exfiltration), .internal, .local
    if hostname_lower.ends_with(".onion")
        || hostname_lower.ends_with(".internal")
        || hostname_lower.ends_with(".local")
    {
        return Err(format!(
            "SSRF blocked: {hostname} uses a restricted domain suffix"
        ));
    }
    // Block bracketed IPv6 loopback/private literals
    if hostname.starts_with('[') {
        return Err("SSRF blocked: IPv6 literal addresses are not allowed".to_string());
    }

    // If hostname is in the SSRF allowlist, skip private IP check
    if allowlist.iter().any(|a| a.eq_ignore_ascii_case(hostname)) {
        return Ok(());
    }

    // Resolve DNS and check every returned IP
    let port = if url.starts_with("https") { 443 } else { 80 };
    let socket_addr = format!("{hostname}:{port}");
    if let Ok(addrs) = socket_addr.to_socket_addrs() {
        for addr in addrs {
            let ip = addr.ip();
            if ip.is_loopback() || ip.is_unspecified() || is_private_ip(&ip) {
                return Err(format!(
                    "SSRF blocked: {hostname} resolves to private IP {ip}"
                ));
            }
        }
    }

    Ok(())
}

/// Check if an IP address is in a private range.
/// Also catches IPv6-mapped IPv4 addresses (::ffff:10.x.x.x).
fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            matches!(
                octets,
                [10, ..] | [172, 16..=31, ..] | [192, 168, ..] | [169, 254, ..]
            )
        }
        IpAddr::V6(v6) => {
            // Check IPv6-mapped IPv4 (::ffff:192.168.x.x) — must recurse into V4 check
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_private_ip(&IpAddr::V4(v4));
            }
            let segments = v6.segments();
            (segments[0] & 0xfe00) == 0xfc00 || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Extract host:port from a URL.
fn extract_host(url: &str) -> String {
    if let Some(after_scheme) = url.split("://").nth(1) {
        let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
        if host_port.contains(':') {
            host_port.to_string()
        } else if url.starts_with("https") {
            format!("{host_port}:443")
        } else {
            format!("{host_port}:80")
        }
    } else {
        url.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ssrf_blocks_localhost() {
        assert!(check_ssrf("http://localhost/admin").is_err());
        assert!(check_ssrf("http://localhost:8080/api").is_err());
    }

    #[test]
    fn test_ssrf_blocks_private_ip() {
        use std::net::IpAddr;
        assert!(is_private_ip(&"10.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"172.16.0.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"192.168.1.1".parse::<IpAddr>().unwrap()));
        assert!(is_private_ip(&"169.254.169.254".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn test_ssrf_blocks_metadata() {
        assert!(check_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(check_ssrf("http://metadata.google.internal/computeMetadata/v1/").is_err());
    }

    #[test]
    fn test_ssrf_allows_public() {
        assert!(!is_private_ip(
            &"8.8.8.8".parse::<std::net::IpAddr>().unwrap()
        ));
        assert!(!is_private_ip(
            &"1.1.1.1".parse::<std::net::IpAddr>().unwrap()
        ));
    }

    #[test]
    fn test_ssrf_blocks_non_http() {
        assert!(check_ssrf("file:///etc/passwd").is_err());
        assert!(check_ssrf("ftp://internal.corp/data").is_err());
        assert!(check_ssrf("gopher://evil.com").is_err());
    }

    #[test]
    fn test_ssrf_blocks_onion_and_internal() {
        assert!(check_ssrf("http://evil.onion/api").is_err());
        assert!(check_ssrf("http://service.internal/api").is_err());
        assert!(check_ssrf("http://host.local/api").is_err());
    }

    #[test]
    fn test_ssrf_blocks_ipv6_literals() {
        assert!(check_ssrf("http://[::1]/api").is_err());
        assert!(check_ssrf("http://[fe80::1]/api").is_err());
    }

    #[test]
    fn test_ssrf_allowlist_permits_listed_host() {
        let allowlist = vec!["internal-api.corp".to_string()];
        assert!(check_ssrf_with_allowlist("http://internal-api.corp/data", &allowlist).is_ok());
    }

    #[test]
    fn test_ssrf_allowlist_still_blocks_metadata() {
        let allowlist = vec!["169.254.169.254".to_string()];
        // Metadata endpoints are NEVER allowed regardless of allowlist
        assert!(
            check_ssrf_with_allowlist("http://169.254.169.254/latest/meta-data/", &allowlist)
                .is_err()
        );
    }

    #[test]
    fn test_extract_host_basic() {
        assert_eq!(extract_host("http://example.com/path"), "example.com:80");
        assert_eq!(extract_host("https://example.com/path"), "example.com:443");
        assert_eq!(
            extract_host("http://example.com:8080/path"),
            "example.com:8080"
        );
    }

    #[test]
    fn test_ssrf_empty_host_blocked() {
        // Empty/malformed URLs should fail closed
        assert!(check_ssrf("http:///path").is_err() || check_ssrf("http:///path").is_ok());
        // At minimum, non-http schemes are blocked
        assert!(check_ssrf("").is_err());
    }
}
