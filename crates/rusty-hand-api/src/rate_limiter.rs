//! Cost-aware rate limiting using GCRA (Generic Cell Rate Algorithm).
//!
//! Each API operation has a token cost (e.g., health=1, spawn=50, message=30).
//! The GCRA algorithm allows 500 tokens per minute per IP address.

use axum::body::Body;
use axum::http::{Request, Response, StatusCode};
use axum::middleware::Next;
use governor::{clock::DefaultClock, state::keyed::DashMapStateStore, Quota, RateLimiter};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

pub fn operation_cost(method: &str, path: &str) -> NonZeroU32 {
    match (method, path) {
        (_, "/api/health") => NonZeroU32::new(1).unwrap(),
        ("GET", "/api/status") => NonZeroU32::new(1).unwrap(),
        ("GET", "/api/version") => NonZeroU32::new(1).unwrap(),
        ("GET", "/api/tools") => NonZeroU32::new(1).unwrap(),
        ("GET", "/api/agents") => NonZeroU32::new(2).unwrap(),
        ("GET", "/api/skills") => NonZeroU32::new(2).unwrap(),
        ("GET", "/api/peers") => NonZeroU32::new(2).unwrap(),
        ("GET", "/api/config") => NonZeroU32::new(2).unwrap(),
        ("GET", "/api/usage") => NonZeroU32::new(3).unwrap(),
        ("GET", p) if p.starts_with("/api/audit") => NonZeroU32::new(5).unwrap(),
        ("GET", p) if p.starts_with("/api/marketplace") => NonZeroU32::new(10).unwrap(),
        ("POST", "/api/agents") => NonZeroU32::new(50).unwrap(),
        ("POST", p) if p.contains("/message") => NonZeroU32::new(30).unwrap(),
        ("POST", p) if p.contains("/run") => NonZeroU32::new(100).unwrap(),
        ("POST", "/api/skills/install") => NonZeroU32::new(50).unwrap(),
        ("POST", "/api/skills/uninstall") => NonZeroU32::new(10).unwrap(),
        ("POST", "/api/migrate") => NonZeroU32::new(100).unwrap(),
        ("PUT", p) if p.contains("/update") => NonZeroU32::new(10).unwrap(),
        _ => NonZeroU32::new(5).unwrap(),
    }
}

pub type KeyedRateLimiter = RateLimiter<IpAddr, DashMapStateStore<IpAddr>, DefaultClock>;

/// 500 tokens per minute per IP.
pub fn create_rate_limiter() -> Arc<KeyedRateLimiter> {
    Arc::new(RateLimiter::keyed(Quota::per_minute(
        NonZeroU32::new(500).unwrap(),
    )))
}

/// GCRA rate limiting middleware.
///
/// Extracts the client IP from `ConnectInfo`, computes the cost for the
/// requested operation, and checks the GCRA limiter. Returns 429 if the
/// client has exhausted its token budget.
pub async fn gcra_rate_limit(
    axum::extract::State(limiter): axum::extract::State<Arc<KeyedRateLimiter>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let ip = crate::middleware::client_ip(&request).unwrap_or(IpAddr::from([127, 0, 0, 1]));

    let method = request.method().as_str().to_string();
    let path = request.uri().path().to_string();
    let cost = operation_cost(&method, &path);

    // governor::check_key_n returns a nested Result:
    //   Ok(Ok(()))              → request is within budget, allow.
    //   Ok(Err(NotUntil))       → rate-limited (bucket empty).
    //   Err(InsufficientCapacity) → cost exceeds the burst limit (cost > 500
    //                              with our Quota::per_minute(500)).
    // The previous `.is_err()` check only fired on InsufficientCapacity,
    // which never happens with our cost table (max 100). The rate-limited
    // case slipped through Ok(Err) and the request was admitted — so the
    // "500 tokens / minute per IP" doc above was never actually enforced.
    let allowed = matches!(limiter.check_key_n(&ip, cost), Ok(Ok(_)));
    if !allowed {
        tracing::warn!(ip = %ip, cost = cost.get(), path = %path, "GCRA rate limit exceeded");
        return Response::builder()
            .status(StatusCode::TOO_MANY_REQUESTS)
            .header("content-type", "application/json")
            .header("retry-after", "60")
            .body(Body::from(
                serde_json::json!({"error": "Rate limit exceeded"}).to_string(),
            ))
            .unwrap_or_default();
    }

    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: the previous middleware used
    /// `limiter.check_key_n(&ip, cost).is_err()` which fires only when
    /// cost > burst (InsufficientCapacity). The rate-limited case
    /// `Ok(Err(NotUntil))` slipped through and the request was allowed.
    /// In practice, with our cost table (max 100) and burst 500, the
    /// rate limit was effectively disabled — an attacker could hammer
    /// the API as fast as they wanted from one IP and never get a 429.
    ///
    /// This test drains a small-quota limiter and confirms the SECOND
    /// allow-check returns `Ok(Err(NotUntil))` — the case the old
    /// `.is_err()` predicate would miss but the new `matches!(
    /// Ok(Ok(_)))` correctly rejects.
    #[test]
    fn check_key_n_distinguishes_rate_limit_from_capacity_overflow() {
        use governor::{Quota, RateLimiter};
        use std::net::{IpAddr, Ipv4Addr};

        // Quota of 1 token per minute, burst 1, so the first request
        // costs the only token and the second is rate-limited.
        let q = Quota::per_minute(NonZeroU32::new(1).unwrap());
        let lim: KeyedRateLimiter = RateLimiter::keyed(q);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));
        let cost = NonZeroU32::new(1).unwrap();

        // First call drains the bucket.
        let first = lim.check_key_n(&ip, cost);
        assert!(
            matches!(first, Ok(Ok(_))),
            "first call must be allowed, got: {first:?}"
        );

        // Second call is rate-limited: outer Ok, inner Err.
        let second = lim.check_key_n(&ip, cost);
        assert!(
            matches!(second, Ok(Err(_))),
            "second call must surface as Ok(Err(_)) — \
             governor's rate-limited shape — got: {second:?}"
        );
        // The fix uses `matches!(_, Ok(Ok(_)))`, which correctly
        // rejects this. The old `.is_err()` would NOT reject it
        // (outer is Ok), letting the request through.
        let allowed = matches!(second, Ok(Ok(_)));
        assert!(!allowed, "post-fix predicate must deny the second call");
        // Pre-fix predicate for contrast — demonstrates the silent
        // bypass that shipped to every API install.
        let old_predicate = second.is_err();
        assert!(
            !old_predicate,
            "old `.is_err()` predicate was false for the rate-limited \
             case — that is exactly why the limit didn't apply"
        );

        // InsufficientCapacity (cost > burst=1) does still produce
        // outer Err.
        let too_big = lim.check_key_n(&ip, NonZeroU32::new(2).unwrap());
        assert!(
            too_big.is_err(),
            "cost > burst must produce outer Err, got: {too_big:?}"
        );
    }

    #[test]
    fn test_costs() {
        assert_eq!(operation_cost("GET", "/api/health").get(), 1);
        assert_eq!(operation_cost("GET", "/api/tools").get(), 1);
        assert_eq!(operation_cost("POST", "/api/agents/1/message").get(), 30);
        assert_eq!(operation_cost("POST", "/api/agents").get(), 50);
        assert_eq!(operation_cost("POST", "/api/workflows/1/run").get(), 100);
        assert_eq!(operation_cost("GET", "/api/agents/1/session").get(), 5);
        assert_eq!(operation_cost("GET", "/api/skills").get(), 2);
        assert_eq!(operation_cost("GET", "/api/peers").get(), 2);
        assert_eq!(operation_cost("GET", "/api/audit/recent").get(), 5);
        assert_eq!(operation_cost("POST", "/api/skills/install").get(), 50);
        assert_eq!(operation_cost("POST", "/api/migrate").get(), 100);
    }
}
