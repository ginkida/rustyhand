//! Embedded WebChat UI served as static HTML.
//!
//! The production dashboard is assembled at compile time from separate
//! HTML/CSS/JS files under `static/` using `include_str!()`. This keeps
//! single-binary deployment while allowing organized source files.
//!
//! Features:
//! - Alpine.js SPA with hash-based routing (10 panels)
//! - Dark/light theme toggle with system preference detection
//! - Responsive layout with collapsible sidebar
//! - Markdown rendering + syntax highlighting (bundled locally)
//! - WebSocket real-time chat with HTTP fallback
//! - Agent management, workflows, memory browser, audit log, and more

use axum::http::header;
use axum::response::IntoResponse;

/// Compile-time ETag based on the crate version.
const ETAG: &str = concat!("\"rusty-hand-", env!("CARGO_PKG_VERSION"), "\"");

/// Embedded logo SVG for single-binary deployment.
const LOGO_SVG: &[u8] = include_bytes!("../static/logo.svg");

/// Embedded logo PNG (legacy, kept for backward compat).
const LOGO_PNG: &[u8] = include_bytes!("../static/logo.png");

/// GET /logo.svg — Serve the RustyHand SVG logo.
pub async fn logo_svg() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
        ],
        LOGO_SVG,
    )
}

/// GET /logo.png — Serve the RustyHand logo (legacy).
pub async fn logo_png() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/png"),
            (header::CACHE_CONTROL, "public, max-age=86400, immutable"),
        ],
        LOGO_PNG,
    )
}

/// GET /favicon.ico — Serve the RustyHand favicon (redirects to SVG logo).
pub async fn favicon_ico() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=3600"),
        ],
        LOGO_SVG,
    )
}

/// GET / — Serve the RustyHand Dashboard single-page application.
///
/// Returns the full SPA with ETag header based on package version for caching.
pub async fn webchat_page() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "text/html; charset=utf-8"),
            (header::ETAG, ETAG),
            (
                header::CACHE_CONTROL,
                "public, max-age=3600, must-revalidate",
            ),
        ],
        WEBCHAT_HTML,
    )
}

/// The embedded HTML/CSS/JS for the RustyHand Control Panel.
///
/// Single-binary deploy: React 18 (UMD), ReactDOM 18 (UMD), the panel CSS,
/// and the precompiled JSX modules are all stitched together at compile time.
/// JSX is precompiled into plain JS by tooling outside the build (esbuild)
/// so we don't pay the ~3MB Babel-standalone runtime cost.
///
/// Load order matters:
/// 1. React + ReactDOM globals must exist before any panel script runs.
/// 2. `data.js` exposes `window.RH_DATA` (mock fallback fixtures).
/// 3. `api.js` registers `useApi` / `usePolling` / `normalizeAgent` etc.
///    on `window` — pages call these to fetch live kernel data.
/// 4. `icons.js` registers shared visual primitives on `window`.
/// 5. `tweaks-panel.js` registers the tweaks helpers on `window`.
/// 6. `pages.js` registers per-route components on `window`.
/// 7. `app.js` is last — it mounts `<App/>` into `#root`.
const WEBCHAT_HTML: &str = concat!(
    include_str!("../static/index_head.html"),
    "<style>\n",
    include_str!("../static/css/panel.css"),
    "\n</style>\n",
    include_str!("../static/index_body.html"),
    "<script>\n",
    include_str!("../static/vendor/react.production.min.js"),
    "\n</script>\n",
    "<script>\n",
    include_str!("../static/vendor/react-dom.production.min.js"),
    "\n</script>\n",
    "<script>\n",
    include_str!("../static/js/panel/data.js"),
    "\n",
    include_str!("../static/js/panel/api.js"),
    "\n",
    include_str!("../static/js/panel/icons.js"),
    "\n",
    include_str!("../static/js/panel/tweaks-panel.js"),
    "\n",
    include_str!("../static/js/panel/pages.js"),
    "\n",
    include_str!("../static/js/panel/app.js"),
    "\n</script>\n",
    "</body></html>"
);
