//! Production middleware for the RustyHand API server.
//!
//! Provides:
//! - Request ID generation and propagation
//! - Per-endpoint structured request logging
//! - In-memory rate limiting (per IP)

use crate::routes::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, Response, StatusCode, Uri};
use axum::middleware::Next;
use rusty_hand_kernel::auth::UserRole;
use rusty_hand_runtime::audit::AuditAction;
use rusty_hand_types::agent::UserId;
use sha2::{Digest, Sha256};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;
use subtle::ConstantTimeEq;
use tracing::info;

/// Request ID header name (standard).
pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone)]
pub enum AuthSource {
    Localhost,
    GlobalApiKey,
    UserApiKey,
}

impl std::fmt::Display for AuthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthSource::Localhost => write!(f, "localhost"),
            AuthSource::GlobalApiKey => write!(f, "global_api_key"),
            AuthSource::UserApiKey => write!(f, "user_api_key"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Option<UserId>,
    pub name: String,
    pub role: UserRole,
    pub source: AuthSource,
}

impl AuthenticatedUser {
    fn implicit_owner(source: AuthSource) -> Self {
        let name = match source {
            AuthSource::Localhost => "localhost".to_string(),
            AuthSource::GlobalApiKey => "global-api-key".to_string(),
            AuthSource::UserApiKey => "user-api-key".to_string(),
        };
        Self {
            user_id: None,
            name,
            role: UserRole::Owner,
            source,
        }
    }
}

#[derive(Clone)]
struct RouteRequirement {
    min_role: UserRole,
    audit_action: AuditAction,
    audit_on_success: bool,
}

fn is_trusted_proxy(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private() || ip.is_link_local(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    }
}

fn strip_port(token: &str) -> Option<&str> {
    if token.starts_with('[') {
        return token
            .find(']')
            .and_then(|end| token.get(1..end))
            .filter(|ip| !ip.is_empty());
    }

    if token.matches(':').count() == 1 && token.contains('.') {
        return token.rsplit_once(':').map(|(ip, _)| ip);
    }

    None
}

fn parse_ip_token(token: &str) -> Option<IpAddr> {
    let token = token.trim().trim_matches('"').trim();
    if token.is_empty() || token.eq_ignore_ascii_case("unknown") {
        return None;
    }

    let token = token
        .strip_prefix("for=")
        .or_else(|| token.strip_prefix("For="))
        .unwrap_or(token)
        .trim()
        .split(';')
        .next()
        .unwrap_or(token)
        .trim();

    token
        .parse::<IpAddr>()
        .ok()
        .or_else(|| strip_port(token).and_then(|ip| ip.parse::<IpAddr>().ok()))
}

fn forwarded_ip<B>(request: &Request<B>) -> Option<IpAddr> {
    if let Some(header) = request
        .headers()
        .get("forwarded")
        .and_then(|v| v.to_str().ok())
    {
        for part in header.split(',') {
            for kv in part.split(';') {
                if let Some(ip) = parse_ip_token(kv) {
                    return Some(ip);
                }
            }
        }
    }

    if let Some(header) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    {
        if let Some(ip) = header.split(',').find_map(parse_ip_token) {
            return Some(ip);
        }
    }

    request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .and_then(parse_ip_token)
}

pub(crate) fn client_ip<B>(request: &Request<B>) -> Option<IpAddr> {
    let peer_ip = request
        .extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip());

    match peer_ip {
        Some(peer_ip) if is_trusted_proxy(peer_ip) => forwarded_ip(request).or(Some(peer_ip)),
        Some(peer_ip) => Some(peer_ip),
        None => forwarded_ip(request),
    }
}

/// Middleware: inject a unique request ID and log the request/response.
pub async fn request_logging(request: Request<Body>, next: Next) -> Response<Body> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let method = request.method().clone();
    let uri = request.uri().path().to_string();
    let auth_user = request.extensions().get::<AuthenticatedUser>().cloned();
    let start = Instant::now();

    let mut response = next.run(request).await;

    let elapsed = start.elapsed();
    let status = response.status().as_u16();
    let auth_name = auth_user.as_ref().map(|u| u.name.as_str()).unwrap_or("-");
    let auth_role = auth_user
        .as_ref()
        .map(|u| u.role.to_string())
        .unwrap_or_else(|| "-".to_string());
    let auth_source = auth_user
        .as_ref()
        .map(|u| u.source.to_string())
        .unwrap_or_else(|| "-".to_string());

    info!(
        request_id = %request_id,
        method = %method,
        path = %uri,
        status = status,
        latency_ms = elapsed.as_millis() as u64,
        auth_user = %auth_name,
        auth_role = %auth_role,
        auth_source = %auth_source,
        "API request"
    );

    // Inject the request ID into the response
    if let Ok(header_val) = request_id.parse() {
        response.headers_mut().insert(REQUEST_ID_HEADER, header_val);
    }

    response
}

/// Bearer token authentication middleware.
///
/// Supports both a legacy global bearer token and per-user API keys from
/// `[[users]]`. Query-string `?token=` auth is restricted to streaming routes.
pub async fn auth(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let path = request.uri().path().to_string();

    if !auth_enabled(&state) {
        let is_loopback = client_ip(&request)
            .map(|ip| ip.is_loopback())
            .unwrap_or(false);

        if !is_loopback {
            tracing::warn!(
                "Rejected non-localhost request: no API key configured. \
                 Set api_key in config.toml for remote access."
            );
            return json_error_response(
                StatusCode::FORBIDDEN,
                "No API key configured. Remote access denied. Configure api_key in ~/.rustyhand/config.toml",
            );
        }

        request
            .extensions_mut()
            .insert(AuthenticatedUser::implicit_owner(AuthSource::Localhost));
        return next.run(request).await;
    }

    if is_public_path(&path) {
        return next.run(request).await;
    }

    let header_token = bearer_token(request.headers());
    let raw_query_token = query_token(request.uri());
    let allow_query_token = query_token_allowed(&path);
    let query_token = if allow_query_token {
        raw_query_token
    } else {
        None
    };

    if raw_query_token.is_some() && !allow_query_token {
        return unauthorized_response(
            "Query-string token auth is only supported for streaming endpoints",
        );
    }

    let authenticated = header_token
        .and_then(|token| authenticate_token(&state, token))
        .or_else(|| query_token.and_then(|token| authenticate_token(&state, token)));

    if let Some(user) = authenticated {
        request.extensions_mut().insert(user);
        return next.run(request).await;
    }

    let credential_provided = header_token.is_some() || raw_query_token.is_some();
    if credential_provided {
        unauthorized_response("Invalid API key")
    } else {
        unauthorized_response("Missing Authorization: Bearer <api_key> header")
    }
}

/// Route-level authorization middleware for HTTP/dashboard access.
pub async fn authorize_http(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let principal = request.extensions().get::<AuthenticatedUser>().cloned();
    let requirement = route_requirement(&method, &path);

    let (Some(principal), Some(requirement)) = (principal, requirement) else {
        return next.run(request).await;
    };

    if principal.role < requirement.min_role {
        state.kernel.audit_log.record(
            principal.name.clone(),
            AuditAction::AuthAttempt,
            format!("{method} {path}"),
            format!(
                "denied: role {} requires {}",
                principal.role, requirement.min_role
            ),
        );
        return json_error_response(
            StatusCode::FORBIDDEN,
            &format!(
                "Access denied: {} role required for this endpoint",
                requirement.min_role
            ),
        );
    }

    let actor = principal.name.clone();
    let response = next.run(request).await;

    if requirement.audit_on_success {
        state.kernel.audit_log.record(
            actor,
            requirement.audit_action,
            format!("{method} {path}"),
            format!("http {}", response.status().as_u16()),
        );
    }

    response
}

fn route_requirement(method: &Method, path: &str) -> Option<RouteRequirement> {
    if is_public_path(path) {
        return None;
    }

    if path == "/api/auth/me" {
        return Some(viewer_only());
    }

    if path == "/api/auth/users"
        || path == "/api/shutdown"
        || path == "/api/memory/export"
        || path == "/api/memory/import"
        || path.starts_with("/api/bindings")
        || (path.starts_with("/api/config") && *method != Method::GET)
        || (path.starts_with("/api/providers/") && *method != Method::GET)
        || (path.starts_with("/api/channels/") && *method != Method::GET)
    {
        return Some(RouteRequirement {
            min_role: UserRole::Owner,
            audit_action: AuditAction::ConfigChange,
            audit_on_success: *method != Method::GET,
        });
    }

    if path.starts_with("/api/usage")
        || path.starts_with("/api/budget")
        || path.starts_with("/api/audit")
        || path == "/api/logs/stream"
        || path == "/api/security"
        || path.starts_with("/api/pairing")
    {
        return Some(RouteRequirement {
            min_role: UserRole::Admin,
            audit_action: AuditAction::ConfigChange,
            audit_on_success: *method != Method::GET,
        });
    }

    if path.starts_with("/api/skills/install")
        || path.starts_with("/api/skills/uninstall")
        || path.starts_with("/api/skills/create")
        || path.starts_with("/api/clawhub/install")
        || path.starts_with("/api/workflows") && *method != Method::GET
        || path.starts_with("/api/triggers") && *method != Method::GET
        || path.starts_with("/api/cron") && *method != Method::GET
        || path.starts_with("/api/approvals")
        || path.starts_with("/api/integrations") && *method != Method::GET
        || path.starts_with("/hooks/")
    {
        return Some(RouteRequirement {
            min_role: UserRole::Admin,
            audit_action: AuditAction::ConfigChange,
            audit_on_success: *method != Method::GET,
        });
    }

    if path == "/api/agents" && *method == Method::POST {
        return Some(RouteRequirement {
            min_role: UserRole::Admin,
            audit_action: AuditAction::AgentSpawn,
            audit_on_success: true,
        });
    }

    if path.starts_with("/api/agents/") {
        if path.ends_with("/message")
            || path.ends_with("/message/stream")
            || path.ends_with("/session")
            || path.ends_with("/session/reset")
            || path.ends_with("/session/compact")
            || path.ends_with("/upload")
            || path.ends_with("/ws")
            || path.contains("/sessions/")
            || path.ends_with("/sessions")
            || path.ends_with("/deliveries")
            || path.ends_with("/files")
            || path.contains("/files/")
        {
            let min_role = if path.contains("/files/") && *method != Method::GET {
                UserRole::Admin
            } else {
                UserRole::User
            };
            return Some(RouteRequirement {
                min_role,
                audit_action: if path.contains("/files/") {
                    AuditAction::FileAccess
                } else {
                    AuditAction::AgentMessage
                },
                audit_on_success: *method != Method::GET,
            });
        }

        if *method == Method::DELETE || path.ends_with("/stop") {
            return Some(RouteRequirement {
                min_role: UserRole::Admin,
                audit_action: AuditAction::AgentKill,
                audit_on_success: true,
            });
        }

        if *method != Method::GET {
            return Some(RouteRequirement {
                min_role: UserRole::Admin,
                audit_action: AuditAction::ConfigChange,
                audit_on_success: true,
            });
        }
    }

    if path.starts_with("/api/memory/agents/") {
        return Some(RouteRequirement {
            min_role: if *method == Method::GET {
                UserRole::User
            } else {
                UserRole::Admin
            },
            audit_action: AuditAction::MemoryAccess,
            audit_on_success: *method != Method::GET,
        });
    }

    if path == "/v1/chat/completions"
        || path == "/mcp"
        || path == "/api/a2a/send"
        || path == "/a2a/tasks/send"
        || path.ends_with("/cancel")
        || path.starts_with("/api/config")
    {
        return Some(RouteRequirement {
            min_role: UserRole::User,
            audit_action: AuditAction::AgentMessage,
            audit_on_success: *method != Method::GET,
        });
    }

    if *method == Method::GET || *method == Method::HEAD {
        return Some(viewer_only());
    }

    Some(RouteRequirement {
        min_role: UserRole::Admin,
        audit_action: AuditAction::ConfigChange,
        audit_on_success: true,
    })
}

fn viewer_only() -> RouteRequirement {
    RouteRequirement {
        min_role: UserRole::Viewer,
        audit_action: AuditAction::AuthAttempt,
        audit_on_success: false,
    }
}

fn auth_enabled(state: &AppState) -> bool {
    !state.kernel.config.api_key.trim().is_empty() || state.kernel.auth.has_api_keys()
}

pub(crate) fn is_public_path(path: &str) -> bool {
    matches!(
        path,
        "/" | "/logo.svg"
            | "/logo.png"
            | "/favicon.ico"
            | "/api/health"
            | "/api/status"
            | "/api/version"
            | "/api/onboarding"
    )
}

pub(crate) fn query_token_allowed(path: &str) -> bool {
    path == "/api/logs/stream" || path.ends_with("/message/stream") || path.ends_with("/ws")
}

pub(crate) fn bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

pub(crate) fn query_token(uri: &Uri) -> Option<&str> {
    uri.query()
        .and_then(|q| q.split('&').find_map(|pair| pair.strip_prefix("token=")))
}

pub(crate) fn authenticate_token(state: &AppState, token: &str) -> Option<AuthenticatedUser> {
    if let Some(user) = state.kernel.auth.authenticate_api_key(token) {
        return Some(AuthenticatedUser {
            user_id: Some(user.id),
            name: user.name,
            role: user.role,
            source: AuthSource::UserApiKey,
        });
    }

    let api_key = state.kernel.config.api_key.trim();
    if api_key.is_empty() {
        return None;
    }

    // Hash both values so ct_eq always compares 32-byte slices,
    // preventing length-based timing side-channel leaks.
    let token_hash = Sha256::digest(token.as_bytes());
    let key_hash = Sha256::digest(api_key.as_bytes());

    if token_hash.ct_eq(&key_hash).into() {
        Some(AuthenticatedUser::implicit_owner(AuthSource::GlobalApiKey))
    } else {
        None
    }
}

fn unauthorized_response(message: &str) -> Response<Body> {
    Response::builder()
        .status(StatusCode::UNAUTHORIZED)
        .header("content-type", "application/json")
        .header("www-authenticate", "Bearer")
        .body(Body::from(
            serde_json::json!({ "error": message }).to_string(),
        ))
        .unwrap_or_default()
}

fn json_error_response(status: StatusCode, message: &str) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(
            serde_json::json!({ "error": message }).to_string(),
        ))
        .unwrap_or_default()
}

/// Security headers middleware — applied to ALL API responses.
pub async fn security_headers(request: Request<Body>, next: Next) -> Response<Body> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert("x-content-type-options", "nosniff".parse().unwrap());
    headers.insert("x-frame-options", "DENY".parse().unwrap());
    headers.insert("x-xss-protection", "1; mode=block".parse().unwrap());
    // All JS/CSS is bundled inline — only external resource is Google Fonts.
    headers.insert(
        "content-security-policy",
        "default-src 'self'; script-src 'self' 'unsafe-inline' 'unsafe-eval'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com https://fonts.gstatic.com; img-src 'self' data: blob:; connect-src 'self' ws://localhost:* ws://127.0.0.1:* wss://localhost:* wss://127.0.0.1:*; font-src 'self' https://fonts.gstatic.com; media-src 'self' blob:; frame-src 'self' blob:; object-src 'none'; base-uri 'self'; form-action 'self'"
            .parse()
            .unwrap(),
    );
    headers.insert(
        "referrer-policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    headers.insert(
        "cache-control",
        "no-store, no-cache, must-revalidate".parse().unwrap(),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    #[test]
    fn test_request_id_header_constant() {
        assert_eq!(REQUEST_ID_HEADER, "x-request-id");
    }

    #[test]
    fn test_client_ip_prefers_forwarded_from_trusted_proxy() {
        let mut request = Request::builder().uri("/").body(()).unwrap();
        request
            .headers_mut()
            .insert("x-forwarded-for", "203.0.113.10, 10.0.0.2".parse().unwrap());
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((
                Ipv4Addr::LOCALHOST,
                4200,
            ))));

        assert_eq!(
            client_ip(&request),
            Some(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 10)))
        );
    }

    #[test]
    fn test_client_ip_ignores_forwarded_from_untrusted_peer() {
        let mut request = Request::builder().uri("/").body(()).unwrap();
        request
            .headers_mut()
            .insert("x-forwarded-for", "203.0.113.10".parse().unwrap());
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((
                Ipv4Addr::new(198, 51, 100, 4),
                4200,
            ))));

        assert_eq!(
            client_ip(&request),
            Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 4)))
        );
    }

    #[test]
    fn test_parse_forwarded_header_with_port() {
        let mut request = Request::builder().uri("/").body(()).unwrap();
        request
            .headers_mut()
            .insert("forwarded", "for=198.51.100.8:8443".parse().unwrap());
        request
            .extensions_mut()
            .insert(axum::extract::ConnectInfo(SocketAddr::from((
                Ipv6Addr::LOCALHOST,
                4200,
            ))));

        assert_eq!(
            client_ip(&request),
            Some(IpAddr::V4(Ipv4Addr::new(198, 51, 100, 8)))
        );
    }
}
