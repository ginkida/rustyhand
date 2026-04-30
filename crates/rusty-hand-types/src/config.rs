//! Configuration types for the RustyHand kernel.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// DM (direct message) policy for a channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DmPolicy {
    /// Respond to all DMs.
    #[default]
    Respond,
    /// Only respond to DMs from allowed users.
    AllowedOnly,
    /// Ignore all DMs.
    Ignore,
}

/// Group message policy for a channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupPolicy {
    /// Respond to all group messages.
    All,
    /// Only respond when mentioned (@bot).
    #[default]
    MentionOnly,
    /// Only respond to slash commands.
    CommandsOnly,
    /// Ignore all group messages.
    Ignore,
}

/// Output format hint for channel-specific message formatting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Standard Markdown (default).
    #[default]
    Markdown,
    /// Telegram HTML subset.
    TelegramHtml,
    /// Slack mrkdwn format.
    SlackMrkdwn,
    /// Plain text (no formatting).
    PlainText,
}

/// Per-channel behavior overrides.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelOverrides {
    /// Model override (uses agent's default if None).
    pub model: Option<String>,
    /// System prompt override.
    pub system_prompt: Option<String>,
    /// DM policy.
    pub dm_policy: DmPolicy,
    /// Group message policy.
    pub group_policy: GroupPolicy,
    /// Per-user rate limit (messages per minute, 0 = unlimited).
    pub rate_limit_per_user: u32,
    /// Enable thread replies.
    pub threading: bool,
    /// Output format override.
    pub output_format: Option<OutputFormat>,
    /// Usage footer mode override.
    pub usage_footer: Option<UsageFooterMode>,
    /// Typing indicator mode override.
    pub typing_mode: Option<TypingMode>,
}

/// Controls what usage info appears in response footers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageFooterMode {
    /// Don't show usage info.
    Off,
    /// Show token counts only.
    Tokens,
    /// Show estimated cost only.
    Cost,
    /// Show tokens + cost (default).
    #[default]
    Full,
}

/// Kernel operating mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelMode {
    /// Conservative mode — no auto-updates, pinned models, stability-first.
    Stable,
    /// Default balanced mode.
    #[default]
    Default,
    /// Developer mode — experimental features enabled.
    Dev,
}

/// User configuration for RBAC multi-user support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    /// User display name.
    pub name: String,
    /// User role (owner, admin, user, viewer).
    #[serde(default = "default_role")]
    pub role: String,
    /// Channel bindings: maps channel platform IDs to this user.
    /// e.g., {"telegram": "123456", "discord": "987654"}
    #[serde(default)]
    pub channel_bindings: HashMap<String, String>,
    /// Optional API key hash for API authentication.
    #[serde(default)]
    pub api_key_hash: Option<String>,
}

fn default_role() -> String {
    "user".to_string()
}

/// Web search provider selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchProvider {
    /// Brave Search API.
    Brave,
    /// Tavily AI-agent-native search.
    Tavily,
    /// Perplexity AI search.
    Perplexity,
    /// DuckDuckGo HTML (no API key needed).
    DuckDuckGo,
    /// Auto-select based on available API keys (Tavily → Brave → Perplexity → DuckDuckGo).
    #[default]
    Auto,
}

/// Web tools configuration (search + fetch).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebConfig {
    /// Which search provider to use.
    pub search_provider: SearchProvider,
    /// Cache TTL in minutes (0 = disabled).
    pub cache_ttl_minutes: u64,
    /// Brave Search configuration.
    pub brave: BraveSearchConfig,
    /// Tavily Search configuration.
    pub tavily: TavilySearchConfig,
    /// Perplexity Search configuration.
    pub perplexity: PerplexitySearchConfig,
    /// Web fetch configuration.
    pub fetch: WebFetchConfig,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            search_provider: SearchProvider::default(),
            cache_ttl_minutes: 15,
            brave: BraveSearchConfig::default(),
            tavily: TavilySearchConfig::default(),
            perplexity: PerplexitySearchConfig::default(),
            fetch: WebFetchConfig::default(),
        }
    }
}

/// Brave Search API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BraveSearchConfig {
    /// Env var name holding the API key.
    pub api_key_env: String,
    /// Maximum results to return.
    pub max_results: usize,
    /// Country code for search localization (e.g., "US").
    pub country: String,
    /// Search language (e.g., "en").
    pub search_lang: String,
    /// Freshness filter (e.g., "pd" = past day, "pw" = past week).
    pub freshness: String,
}

impl Default for BraveSearchConfig {
    fn default() -> Self {
        Self {
            api_key_env: "BRAVE_API_KEY".to_string(),
            max_results: 5,
            country: String::new(),
            search_lang: String::new(),
            freshness: String::new(),
        }
    }
}

/// Tavily Search API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TavilySearchConfig {
    /// Env var name holding the API key.
    pub api_key_env: String,
    /// Search depth: "basic" or "advanced".
    pub search_depth: String,
    /// Maximum results to return.
    pub max_results: usize,
    /// Include AI-generated answer summary.
    pub include_answer: bool,
}

impl Default for TavilySearchConfig {
    fn default() -> Self {
        Self {
            api_key_env: "TAVILY_API_KEY".to_string(),
            search_depth: "basic".to_string(),
            max_results: 5,
            include_answer: true,
        }
    }
}

/// Perplexity Search API configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PerplexitySearchConfig {
    /// Env var name holding the API key.
    pub api_key_env: String,
    /// Model to use for search (e.g., "sonar").
    pub model: String,
}

impl Default for PerplexitySearchConfig {
    fn default() -> Self {
        Self {
            api_key_env: "PERPLEXITY_API_KEY".to_string(),
            model: "sonar".to_string(),
        }
    }
}

/// Web fetch configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebFetchConfig {
    /// Maximum characters to return in content.
    pub max_chars: usize,
    /// Maximum response body size in bytes.
    pub max_response_bytes: usize,
    /// HTTP request timeout in seconds.
    pub timeout_secs: u64,
    /// Enable HTML→Markdown readability extraction.
    pub readability: bool,
    /// Hostnames allowed to resolve to private IPs (SSRF allowlist).
    /// Use this for internal services like `["de-dagu.sulpak.kz", "airflow.internal"]`.
    pub ssrf_allowlist: Vec<String>,
}

impl Default for WebFetchConfig {
    fn default() -> Self {
        Self {
            max_chars: 50_000,
            max_response_bytes: 10 * 1024 * 1024, // 10 MB
            timeout_secs: 30,
            readability: true,
            ssrf_allowlist: Vec::new(),
        }
    }
}

/// HTTP/HTTPS proxy configuration for outbound requests.
///
/// When `url` is set, all proxy-aware HTTP clients (web_fetch, browser, etc.)
/// route requests through it. Useful for residential proxy services like
/// Bright Data, Smartproxy, IPRoyal where the same endpoint handles both
/// HTTP and HTTPS via CONNECT.
///
/// Auth: `username` and `password` are sent as Basic auth on the proxy
/// connection. For Bright Data the credentials are zone-specific.
///
/// Example config:
/// ```toml
/// [proxy]
/// url = "http://brd.superproxy.io:33335"
/// username = "brd-customer-hl_xxx-zone-residential_proxy1"
/// password = "secret"
/// no_proxy = ["localhost", "127.0.0.1", "*.internal"]
/// ```
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    /// Proxy URL (e.g. `http://brd.superproxy.io:33335`). Empty disables the proxy.
    /// Schemes: http://, https://, socks5://.
    pub url: String,
    /// Optional basic auth username for the proxy.
    #[serde(default)]
    pub username: String,
    /// Optional basic auth password for the proxy.
    /// SECURITY: skip_serializing prevents accidental exposure in JSON output.
    /// Custom Debug impl below redacts this field.
    #[serde(default, skip_serializing)]
    pub password: String,
    /// Hostnames that should bypass the proxy (direct connection).
    /// Supports exact match and wildcard prefix `*.` (e.g. `*.internal`).
    /// `localhost`, `127.0.0.1`, and `::1` are always bypassed regardless of this list.
    /// Wildcards do NOT match the bare suffix (`*.local` doesn't match `local`).
    #[serde(default)]
    pub no_proxy: Vec<String>,
}

/// SECURITY: Custom Debug impl redacts the password so it never leaks into
/// logs or panic messages via `{:?}` / `dbg!()`.
impl std::fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("url", &self.url)
            .field("username", &self.username)
            .field(
                "password",
                &if self.password.is_empty() {
                    "<empty>"
                } else {
                    "<redacted>"
                },
            )
            .field("no_proxy", &self.no_proxy)
            .finish()
    }
}

impl ProxyConfig {
    /// True if a proxy URL is configured.
    pub fn is_enabled(&self) -> bool {
        !self.url.trim().is_empty()
    }

    /// Validate the proxy configuration.
    /// Returns a list of human-readable warnings (empty if config is valid).
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();
        if self.url.trim().is_empty() {
            return warnings;
        }
        let lower = self.url.to_lowercase();
        if !lower.starts_with("http://")
            && !lower.starts_with("https://")
            && !lower.starts_with("socks5://")
            && !lower.starts_with("socks5h://")
        {
            warnings.push(format!(
                "proxy.url '{}' must start with http://, https://, socks5:// or socks5h://",
                self.url
            ));
        }
        if !self.username.is_empty() && self.password.is_empty() {
            warnings.push("proxy.username is set but proxy.password is empty".to_string());
        }
        warnings
    }

    /// Check whether the proxy should be bypassed for a given host.
    /// Always bypasses localhost, 127.0.0.1 and ::1.
    ///
    /// `no_proxy` matching:
    /// - exact: `internal.corp` matches only `internal.corp` (case-insensitive)
    /// - wildcard: `*.local` matches `foo.local` and `a.b.local`, but NOT bare `local`
    ///   (matches standard `no_proxy` semantics — to bypass the bare host, add
    ///   it as a separate exact entry).
    ///
    /// Bypass means the request goes direct (no proxy), so being conservative
    /// here is a security choice: fewer accidental bypasses = more traffic
    /// stays on the proxy as the operator expects.
    pub fn should_bypass(&self, host: &str) -> bool {
        let h = host.to_lowercase();
        if h == "localhost" || h == "127.0.0.1" || h == "::1" {
            return true;
        }
        for entry in &self.no_proxy {
            let e = entry.trim().to_lowercase();
            if e.is_empty() {
                continue;
            }
            if let Some(suffix) = e.strip_prefix("*.") {
                // Standard wildcard semantics: only subdomains of `suffix` match.
                if h.ends_with(&format!(".{suffix}")) {
                    return true;
                }
            } else if h == e {
                return true;
            }
        }
        false
    }
}

/// Browser automation configuration (uses `agent-browser` CLI).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    /// Run browser in headless mode (no visible window).
    pub headless: bool,
    /// Viewport width in pixels.
    pub viewport_width: u32,
    /// Viewport height in pixels.
    pub viewport_height: u32,
    /// Per-action timeout in seconds.
    pub timeout_secs: u64,
    /// Idle timeout — auto-close session after this many seconds of inactivity.
    pub idle_timeout_secs: u64,
    /// Maximum concurrent browser sessions.
    pub max_sessions: usize,
    /// Path to the `agent-browser` executable.
    pub executable: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            headless: true,
            viewport_width: 1280,
            viewport_height: 720,
            timeout_secs: 30,
            idle_timeout_secs: 300,
            max_sessions: 5,
            executable: "agent-browser".to_string(),
        }
    }
}

/// Config hot-reload mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadMode {
    /// No automatic reloading.
    Off,
    /// Full restart on config change.
    Restart,
    /// Hot-reload safe sections only (channels, skills, heartbeat).
    Hot,
    /// Hot-reload where possible, flag restart-required otherwise.
    #[default]
    Hybrid,
}

/// Configuration for config file watching and hot-reload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReloadConfig {
    /// Reload mode. Default: hybrid.
    pub mode: ReloadMode,
    /// Debounce window in milliseconds. Default: 500.
    pub debounce_ms: u64,
}

impl Default for ReloadConfig {
    fn default() -> Self {
        Self {
            mode: ReloadMode::default(),
            debounce_ms: 500,
        }
    }
}

/// Webhook trigger authentication configuration.
///
/// Controls the `/hooks/wake` and `/hooks/agent` endpoints for external
/// systems to trigger agent actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookTriggerConfig {
    /// Enable webhook trigger endpoints. Default: false.
    pub enabled: bool,
    /// Env var name holding the bearer token (NOT the token itself).
    /// MUST be set if enabled=true. Token must be >= 32 chars.
    pub token_env: String,
    /// Max payload size in bytes. Default: 65536.
    pub max_payload_bytes: usize,
    /// Rate limit: max requests per minute per IP. Default: 30.
    pub rate_limit_per_minute: u32,
}

impl Default for WebhookTriggerConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            token_env: "RUSTY_HAND_WEBHOOK_TOKEN".to_string(),
            max_payload_bytes: 65536,
            rate_limit_per_minute: 30,
        }
    }
}

/// Fallback provider chain — tried in order if the primary provider fails.
///
/// Configurable in `config.toml` under `[[fallback_providers]]`:
/// ```toml
/// [[fallback_providers]]
/// provider = "ollama"
/// model = "llama3.2:latest"
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FallbackProviderConfig {
    /// Provider name (e.g., "ollama", "groq").
    pub provider: String,
    /// Model to use from this provider.
    pub model: String,
    /// Environment variable for API key (empty for local providers).
    #[serde(default)]
    pub api_key_env: String,
    /// Base URL override (uses catalog default if None).
    #[serde(default)]
    pub base_url: Option<String>,
}

/// Text-to-speech configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsConfig {
    /// Enable TTS. Default: false.
    pub enabled: bool,
    /// Default provider: "openai" or "elevenlabs".
    pub provider: Option<String>,
    /// OpenAI TTS settings.
    pub openai: TtsOpenAiConfig,
    /// ElevenLabs TTS settings.
    pub elevenlabs: TtsElevenLabsConfig,
    /// Max text length for TTS (chars). Default: 4096.
    pub max_text_length: usize,
    /// Timeout per TTS request in seconds. Default: 30.
    pub timeout_secs: u64,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: None,
            openai: TtsOpenAiConfig::default(),
            elevenlabs: TtsElevenLabsConfig::default(),
            max_text_length: 4096,
            timeout_secs: 30,
        }
    }
}

/// OpenAI TTS settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsOpenAiConfig {
    /// Voice: alloy, echo, fable, onyx, nova, shimmer. Default: "alloy".
    pub voice: String,
    /// Model: "tts-1" or "tts-1-hd". Default: "tts-1".
    pub model: String,
    /// Output format: "mp3", "opus", "aac", "flac". Default: "mp3".
    pub format: String,
    /// Speed: 0.25 to 4.0. Default: 1.0.
    pub speed: f32,
}

impl Default for TtsOpenAiConfig {
    fn default() -> Self {
        Self {
            voice: "alloy".to_string(),
            model: "tts-1".to_string(),
            format: "mp3".to_string(),
            speed: 1.0,
        }
    }
}

/// ElevenLabs TTS settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TtsElevenLabsConfig {
    /// Voice ID. Default: "21m00Tcm4TlvDq8ikWAM" (Rachel).
    pub voice_id: String,
    /// Model ID. Default: "eleven_monolingual_v1".
    pub model_id: String,
    /// Stability (0.0-1.0). Default: 0.5.
    pub stability: f32,
    /// Similarity boost (0.0-1.0). Default: 0.75.
    pub similarity_boost: f32,
}

impl Default for TtsElevenLabsConfig {
    fn default() -> Self {
        Self {
            voice_id: "21m00Tcm4TlvDq8ikWAM".to_string(),
            model_id: "eleven_monolingual_v1".to_string(),
            stability: 0.5,
            similarity_boost: 0.75,
        }
    }
}

/// Docker container sandbox configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DockerSandboxConfig {
    /// Enable Docker sandbox. Default: false.
    pub enabled: bool,
    /// Docker image for exec sandbox. Default: "python:3.12-slim".
    pub image: String,
    /// Container name prefix. Default: "rusty-hand-sandbox".
    pub container_prefix: String,
    /// Working directory inside container. Default: "/workspace".
    pub workdir: String,
    /// Network mode: "none", "bridge", or custom. Default: "none".
    pub network: String,
    /// Memory limit (e.g., "256m", "1g"). Default: "512m".
    pub memory_limit: String,
    /// CPU limit (e.g., 0.5, 1.0, 2.0). Default: 1.0.
    pub cpu_limit: f64,
    /// Max execution time in seconds. Default: 60.
    pub timeout_secs: u64,
    /// Read-only root filesystem. Default: true.
    pub read_only_root: bool,
    /// Additional capabilities to add. Default: empty (drop all).
    pub cap_add: Vec<String>,
    /// tmpfs mounts. Default: ["/tmp:size=64m"].
    pub tmpfs: Vec<String>,
    /// PID limit. Default: 100.
    pub pids_limit: u32,
    /// Docker sandbox mode: off, non_main, all. Default: off.
    #[serde(default)]
    pub mode: DockerSandboxMode,
    /// Container lifecycle scope. Default: session.
    #[serde(default)]
    pub scope: DockerScope,
    /// Cooldown before reusing a released container (seconds). Default: 300.
    #[serde(default = "default_reuse_cool_secs")]
    pub reuse_cool_secs: u64,
    /// Idle timeout — destroy containers after N seconds of inactivity. Default: 86400 (24h).
    #[serde(default = "default_docker_idle_timeout")]
    pub idle_timeout_secs: u64,
    /// Maximum age before forced destruction (seconds). Default: 604800 (7 days).
    #[serde(default = "default_docker_max_age")]
    pub max_age_secs: u64,
    /// Paths blocked from bind mounting.
    #[serde(default)]
    pub blocked_mounts: Vec<String>,
}

fn default_reuse_cool_secs() -> u64 {
    300
}
fn default_docker_idle_timeout() -> u64 {
    86400
}
fn default_docker_max_age() -> u64 {
    604800
}

impl Default for DockerSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            image: "python:3.12-slim".to_string(),
            container_prefix: "rusty-hand-sandbox".to_string(),
            workdir: "/workspace".to_string(),
            network: "none".to_string(),
            memory_limit: "512m".to_string(),
            cpu_limit: 1.0,
            timeout_secs: 60,
            read_only_root: true,
            cap_add: Vec::new(),
            tmpfs: vec!["/tmp:size=64m".to_string()],
            pids_limit: 100,
            mode: DockerSandboxMode::Off,
            scope: DockerScope::Session,
            reuse_cool_secs: default_reuse_cool_secs(),
            idle_timeout_secs: default_docker_idle_timeout(),
            max_age_secs: default_docker_max_age(),
            blocked_mounts: Vec::new(),
        }
    }
}

/// Device pairing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PairingConfig {
    /// Enable device pairing. Default: false.
    pub enabled: bool,
    /// Max paired devices. Default: 10.
    pub max_devices: usize,
    /// Pairing token expiry in seconds. Default: 300 (5 min).
    pub token_expiry_secs: u64,
    /// Push notification provider: "none", "ntfy", "gotify".
    pub push_provider: String,
    /// Ntfy server URL (if push_provider = "ntfy").
    pub ntfy_url: Option<String>,
    /// Ntfy topic (if push_provider = "ntfy").
    pub ntfy_topic: Option<String>,
}

impl Default for PairingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_devices: 10,
            token_expiry_secs: 300,
            push_provider: "none".to_string(),
            ntfy_url: None,
            ntfy_topic: None,
        }
    }
}

/// Extensions & integrations configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExtensionsConfig {
    /// Enable auto-reconnect for MCP integrations.
    pub auto_reconnect: bool,
    /// Maximum reconnect attempts before giving up.
    pub reconnect_max_attempts: u32,
    /// Maximum backoff duration in seconds.
    pub reconnect_max_backoff_secs: u64,
    /// Health check interval in seconds.
    pub health_check_interval_secs: u64,
}

impl Default for ExtensionsConfig {
    fn default() -> Self {
        Self {
            auto_reconnect: true,
            reconnect_max_attempts: 10,
            reconnect_max_backoff_secs: 300,
            health_check_interval_secs: 60,
        }
    }
}

/// Credential vault configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VaultConfig {
    /// Whether the vault is enabled (auto-detected if vault.enc exists).
    pub enabled: bool,
    /// Custom vault file path (default: ~/.rustyhand/vault.enc).
    pub path: Option<PathBuf>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: None,
        }
    }
}

/// Agent binding — routes specific channel/account/peer patterns to agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBinding {
    /// Target agent name or ID.
    pub agent: String,
    /// Match criteria (all specified fields must match).
    pub match_rule: BindingMatchRule,
}

/// Match rule for agent bindings. All specified (non-None) fields must match.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BindingMatchRule {
    /// Channel type (e.g., "discord", "telegram", "slack").
    pub channel: Option<String>,
    /// Specific account/bot ID within the channel.
    pub account_id: Option<String>,
    /// Peer/user ID for DM routing.
    pub peer_id: Option<String>,
    /// Guild/server ID (Discord/Slack).
    pub guild_id: Option<String>,
    /// Role-based routing (user must have at least one).
    #[serde(default)]
    pub roles: Vec<String>,
}

impl BindingMatchRule {
    /// Calculate specificity score for binding priority ordering.
    /// Higher = more specific = checked first.
    pub fn specificity(&self) -> u32 {
        let mut score = 0u32;
        if self.peer_id.is_some() {
            score += 8;
        }
        if self.guild_id.is_some() {
            score += 4;
        }
        if !self.roles.is_empty() {
            score += 2;
        }
        if self.account_id.is_some() {
            score += 2;
        }
        if self.channel.is_some() {
            score += 1;
        }
        score
    }
}

/// Broadcast config — send same message to multiple agents.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BroadcastConfig {
    /// Broadcast strategy.
    pub strategy: BroadcastStrategy,
    /// Map of peer_id -> list of agent names to receive the message.
    pub routes: HashMap<String, Vec<String>>,
}

/// Broadcast delivery strategy.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BroadcastStrategy {
    /// Send to all agents simultaneously.
    #[default]
    Parallel,
    /// Send to agents one at a time in order.
    Sequential,
}

/// Auto-reply engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutoReplyConfig {
    /// Enable auto-reply engine. Default: false.
    pub enabled: bool,
    /// Max concurrent auto-reply tasks. Default: 3.
    pub max_concurrent: usize,
    /// Default timeout per reply in seconds. Default: 120.
    pub timeout_secs: u64,
    /// Patterns that suppress auto-reply (e.g., "/stop", "/pause").
    pub suppress_patterns: Vec<String>,
}

impl Default for AutoReplyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_concurrent: 3,
            timeout_secs: 120,
            suppress_patterns: vec!["/stop".to_string(), "/pause".to_string()],
        }
    }
}

/// Canvas (Agent-to-UI) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CanvasConfig {
    /// Enable canvas tool. Default: false.
    pub enabled: bool,
    /// Max HTML size in bytes. Default: 512KB.
    pub max_html_bytes: usize,
    /// Allowed HTML tags (empty = all safe tags allowed).
    #[serde(default)]
    pub allowed_tags: Vec<String>,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_html_bytes: 512 * 1024,
            allowed_tags: Vec::new(),
        }
    }
}

/// Shell/exec security mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecSecurityMode {
    /// Block all shell execution.
    Deny,
    /// Only allow commands in safe_bins or allowed_commands.
    #[default]
    Allowlist,
    /// Allow all commands (unsafe, dev only).
    Full,
}

/// Shell/exec security policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExecPolicy {
    /// Security mode: "deny" blocks all, "allowlist" only allows listed,
    /// "full" allows all (unsafe, dev only).
    pub mode: ExecSecurityMode,
    /// Commands that bypass allowlist (stdin-only utilities).
    pub safe_bins: Vec<String>,
    /// Global command allowlist (when mode = allowlist).
    pub allowed_commands: Vec<String>,
    /// Max execution timeout in seconds. Default: 30.
    pub timeout_secs: u64,
    /// Max output size in bytes. Default: 100KB.
    pub max_output_bytes: usize,
    /// No-output idle timeout in seconds. When > 0, kills processes that
    /// produce no stdout/stderr output for this duration. Default: 30.
    #[serde(default = "default_no_output_timeout")]
    pub no_output_timeout_secs: u64,
}

fn default_no_output_timeout() -> u64 {
    30
}

impl Default for ExecPolicy {
    fn default() -> Self {
        Self {
            mode: ExecSecurityMode::default(),
            safe_bins: vec![
                "sleep", "true", "false", "cat", "sort", "uniq", "cut", "tr", "head", "tail", "wc",
                "date", "echo", "printf", "basename", "dirname", "pwd", "env",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            allowed_commands: Vec::new(),
            timeout_secs: 30,
            max_output_bytes: 100 * 1024,
            no_output_timeout_secs: default_no_output_timeout(),
        }
    }
}

// ---------------------------------------------------------------------------
// Gap 2: No-output idle timeout for subprocess sandbox
// ---------------------------------------------------------------------------

/// Reason a subprocess was terminated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminationReason {
    /// Process exited normally.
    Exited(i32),
    /// Absolute timeout exceeded.
    AbsoluteTimeout,
    /// No output timeout exceeded.
    NoOutputTimeout,
}

// ---------------------------------------------------------------------------
// Gap 3: Auth profile rotation — multi-key per provider
// ---------------------------------------------------------------------------

/// A named authentication profile for a provider.
///
/// Multiple profiles can be configured per provider to enable key rotation
/// when one key gets rate-limited or has billing issues.
#[derive(Clone, Serialize, Deserialize)]
pub struct AuthProfile {
    /// Profile name (e.g., "primary", "secondary").
    pub name: String,
    /// Environment variable holding the API key.
    pub api_key_env: String,
    /// Priority (lower = preferred). Default: 0.
    #[serde(default)]
    pub priority: u32,
}

/// SECURITY: Custom Debug impl redacts env var name.
impl std::fmt::Debug for AuthProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthProfile")
            .field("name", &self.name)
            .field("api_key_env", &"<redacted>")
            .field("priority", &self.priority)
            .finish()
    }
}

// ---------------------------------------------------------------------------
// Gap 5: Docker sandbox maturity
// ---------------------------------------------------------------------------

/// Docker sandbox activation mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerSandboxMode {
    /// Docker sandbox disabled.
    #[default]
    Off,
    /// Only use Docker for non-main agents.
    NonMain,
    /// Use Docker for all agents.
    All,
}

/// Docker container lifecycle scope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DockerScope {
    /// Container per session (destroyed when session ends).
    #[default]
    Session,
    /// Container per agent (reused across sessions).
    Agent,
    /// Shared container pool.
    Shared,
}

// ---------------------------------------------------------------------------
// Gap 6: Typing indicator modes
// ---------------------------------------------------------------------------

/// Typing indicator behavior mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypingMode {
    /// Send typing indicator immediately on message receipt (default).
    #[default]
    Instant,
    /// Send typing indicator only when first text delta arrives.
    Message,
    /// Send typing indicator only during LLM reasoning.
    Thinking,
    /// Never send typing indicators.
    Never,
}

// ---------------------------------------------------------------------------
// Gap 7: Thinking level support
// ---------------------------------------------------------------------------

/// Extended thinking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThinkingConfig {
    /// Maximum tokens for thinking (budget).
    pub budget_tokens: u32,
    /// Whether to stream thinking tokens to the client.
    pub stream_thinking: bool,
}

impl Default for ThinkingConfig {
    fn default() -> Self {
        Self {
            budget_tokens: 10_000,
            stream_thinking: false,
        }
    }
}

/// Top-level kernel configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KernelConfig {
    /// RustyHand home directory (default: ~/.rustyhand).
    pub home_dir: PathBuf,
    /// Data directory for databases (default: ~/.rustyhand/data).
    pub data_dir: PathBuf,
    /// Log level (trace, debug, info, warn, error).
    pub log_level: String,
    /// API listen address (e.g., "0.0.0.0:4200").
    #[serde(alias = "listen_addr")]
    pub api_listen: String,
    /// Whether to enable the RHP network layer.
    pub network_enabled: bool,
    /// Default LLM provider configuration.
    pub default_model: DefaultModelConfig,
    /// Memory substrate configuration.
    pub memory: MemoryConfig,
    /// Network configuration.
    pub network: NetworkConfig,
    /// Channel bridge configuration (Telegram, etc.).
    pub channels: ChannelsConfig,
    /// API authentication key. When set, all API endpoints (except /api/health)
    /// require a `Authorization: Bearer <key>` header.
    /// If empty, the API is unauthenticated (local development only).
    /// SECURITY: skip_serializing prevents accidental exposure in JSON output.
    #[serde(skip_serializing)]
    pub api_key: String,
    /// Kernel operating mode (stable, default, dev).
    #[serde(default)]
    pub mode: KernelMode,
    /// Language/locale for CLI and messages (default: "en").
    #[serde(default = "default_language")]
    pub language: String,
    /// User configurations for RBAC multi-user support.
    #[serde(default)]
    pub users: Vec<UserConfig>,
    /// MCP server configurations for external tool integration.
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfigEntry>,
    /// A2A (Agent-to-Agent) protocol configuration.
    #[serde(default)]
    pub a2a: Option<A2aConfig>,
    /// Usage footer mode (what to show after each response).
    #[serde(default)]
    pub usage_footer: UsageFooterMode,
    /// Web tools configuration (search + fetch).
    #[serde(default)]
    pub web: WebConfig,
    /// Fallback providers tried in order if the primary fails.
    /// Configure in config.toml as `[[fallback_providers]]`.
    #[serde(default)]
    pub fallback_providers: Vec<FallbackProviderConfig>,
    /// Browser automation configuration.
    #[serde(default)]
    pub browser: BrowserConfig,
    /// Extensions & integrations configuration.
    #[serde(default)]
    pub extensions: ExtensionsConfig,
    /// Credential vault configuration.
    #[serde(default)]
    pub vault: VaultConfig,
    /// Root directory for agent workspaces. Default: `~/.rustyhand/workspaces`
    #[serde(default)]
    pub workspaces_dir: Option<PathBuf>,
    /// Media understanding configuration.
    #[serde(default)]
    pub media: crate::media::MediaConfig,
    /// Link understanding configuration.
    #[serde(default)]
    pub links: crate::media::LinkConfig,
    /// Config hot-reload settings.
    #[serde(default)]
    pub reload: ReloadConfig,
    /// Webhook trigger configuration (external event injection).
    #[serde(default)]
    pub webhook_triggers: Option<WebhookTriggerConfig>,
    /// Execution approval policy.
    #[serde(default)]
    pub approval: crate::approval::ApprovalPolicy,
    /// Cron scheduler max total jobs across all agents. Default: 500.
    #[serde(default = "default_max_cron_jobs")]
    pub max_cron_jobs: usize,
    /// Config include files — loaded and deep-merged before the root config.
    /// Paths are relative to the root config file's directory.
    /// Security: absolute paths and `..` components are rejected.
    #[serde(default)]
    pub include: Vec<String>,
    /// Shell/exec security policy.
    #[serde(default)]
    pub exec_policy: ExecPolicy,
    /// Agent bindings for multi-account routing.
    #[serde(default)]
    pub bindings: Vec<AgentBinding>,
    /// Broadcast routing configuration.
    #[serde(default)]
    pub broadcast: BroadcastConfig,
    /// Auto-reply background engine configuration.
    #[serde(default)]
    pub auto_reply: AutoReplyConfig,
    /// Canvas (A2UI) configuration.
    #[serde(default)]
    pub canvas: CanvasConfig,
    /// Text-to-speech configuration.
    #[serde(default)]
    pub tts: TtsConfig,
    /// Docker container sandbox configuration.
    #[serde(default)]
    pub docker: DockerSandboxConfig,
    /// Device pairing configuration.
    #[serde(default)]
    pub pairing: PairingConfig,
    /// Auth profiles for key rotation (provider name → profiles).
    #[serde(default)]
    pub auth_profiles: HashMap<String, Vec<AuthProfile>>,
    /// Extended thinking configuration.
    #[serde(default)]
    pub thinking: Option<ThinkingConfig>,
    /// Global spending budget configuration.
    #[serde(default)]
    pub budget: BudgetConfig,
    /// HTTP/HTTPS proxy for outbound requests (web_fetch, browser, etc.).
    /// Disabled by default — when configured, all proxy-aware clients route through it.
    #[serde(default)]
    pub proxy: ProxyConfig,
    /// Configuration for the RustyHand MCP server endpoint (POST /mcp).
    /// Controls which tools remote MCP clients can invoke. See `McpServerConfig`
    /// — by default only safe read-only and messaging tools are allowed.
    #[serde(default)]
    pub mcp_server: McpServerConfig,
}

/// Global spending budget configuration.
///
/// Set limits to 0.0 for unlimited. All limits apply across all agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BudgetConfig {
    /// Maximum total cost in USD per hour (0.0 = unlimited).
    pub max_hourly_usd: f64,
    /// Maximum total cost in USD per day (0.0 = unlimited).
    pub max_daily_usd: f64,
    /// Maximum total cost in USD per month (0.0 = unlimited).
    pub max_monthly_usd: f64,
    /// Alert threshold as a fraction (0.0 - 1.0). Trigger warnings at this % of any limit.
    pub alert_threshold: f64,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_hourly_usd: 0.0,
            max_daily_usd: 0.0,
            max_monthly_usd: 0.0,
            alert_threshold: 0.8,
        }
    }
}

fn default_max_cron_jobs() -> usize {
    500
}

/// Configuration for RustyHand's own MCP server endpoint (POST /mcp).
///
/// This controls what tools external MCP clients (Claude Desktop, etc.) can
/// invoke. By default, only safe read-only + basic-messaging tools are allowed.
/// Privileged tools (`shell_exec`, `file_write`, `skill_install`, `agent_spawn`,
/// `agent_kill`, `apply_patch`) require explicit allowlisting.
///
/// Rationale: an MCP request carries an API key but no agent identity, so the
/// usual manifest-based capability check is bypassed. Without this allowlist,
/// an attacker who obtained the API key could install arbitrary Python into
/// `~/.rustyhand/skills/` via `skill_install` — effectively a remote code
/// execution channel.
///
/// Example to also allow skill_install (needed if you run capability-builder
/// remotely via an MCP client):
/// ```toml
/// [mcp_server]
/// extra_allowed_tools = ["skill_install"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct McpServerConfig {
    /// Whether the /mcp endpoint is enabled at all.
    /// Default: true (it's gated by api_key auth anyway).
    pub enabled: bool,
    /// Additional tools to allow beyond the safe defaults.
    /// Use with care — each entry can be a vector for remote code execution
    /// depending on the tool. See `safe_default_tools()` for the baseline set.
    pub extra_allowed_tools: Vec<String>,
    /// If true, allow ANY tool (disables the allowlist). Intended for
    /// trusted local development only — never enable on a public server.
    pub allow_all_tools: bool,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            extra_allowed_tools: Vec::new(),
            allow_all_tools: false,
        }
    }
}

impl McpServerConfig {
    /// Safe tools that don't let a remote caller exfiltrate data or modify
    /// the host filesystem. These are always allowed when `enabled = true`.
    ///
    /// Deliberately excluded: shell_exec, file_write, apply_patch,
    /// skill_install, agent_spawn, agent_kill, browser_*, image_generate,
    /// media_transcribe (can leak audio content), self_update.
    pub fn safe_default_tools() -> &'static [&'static str] {
        &[
            // Read-only filesystem
            "file_read",
            "file_list",
            // Search & fetch — already SSRF-protected
            "web_search",
            "web_fetch",
            // Agent introspection
            "agent_list",
            "agent_find",
            "agent_send",
            // Memory (scoped per agent anyway)
            "memory_store",
            "memory_recall",
            // Self-observation
            "self_history",
            "self_metrics",
            // Documents
            "doc_ingest",
            "doc_search",
            // Knowledge graph
            "knowledge_add_entity",
            "knowledge_add_relation",
            "knowledge_query",
            // Collaboration
            "task_post",
            "task_claim",
            "task_complete",
            "task_list",
            "event_publish",
            // Scheduling
            "schedule_create",
            "schedule_list",
            "schedule_delete",
        ]
    }

    /// Check whether a tool may be called from an MCP request.
    pub fn is_tool_allowed(&self, tool_name: &str) -> bool {
        if !self.enabled {
            return false;
        }
        if self.allow_all_tools {
            return true;
        }
        if Self::safe_default_tools().contains(&tool_name) {
            return true;
        }
        self.extra_allowed_tools.iter().any(|t| t == tool_name)
    }
}

/// Configuration entry for an MCP server.
///
/// This is the config.toml representation. The runtime `McpServerConfig`
/// struct is constructed from this during kernel boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfigEntry {
    /// Display name for this server.
    pub name: String,
    /// Transport configuration.
    pub transport: McpTransportEntry,
    /// Request timeout in seconds.
    #[serde(default = "default_mcp_timeout")]
    pub timeout_secs: u64,
    /// Environment variables to pass through (e.g., ["GITHUB_PERSONAL_ACCESS_TOKEN"]).
    #[serde(default)]
    pub env: Vec<String>,
}

fn default_mcp_timeout() -> u64 {
    30
}

/// Transport configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransportEntry {
    /// Subprocess with JSON-RPC over stdin/stdout.
    Stdio {
        command: String,
        #[serde(default)]
        args: Vec<String>,
    },
    /// HTTP Server-Sent Events.
    Sse { url: String },
}

/// A2A (Agent-to-Agent) protocol configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct A2aConfig {
    /// Whether A2A is enabled.
    pub enabled: bool,
    /// Path to serve A2A endpoints (default: "/a2a").
    #[serde(default = "default_a2a_path")]
    pub listen_path: String,
    /// External A2A agents to connect to.
    #[serde(default)]
    pub external_agents: Vec<ExternalAgent>,
}

fn default_a2a_path() -> String {
    "/a2a".to_string()
}

/// An external A2A agent to discover and interact with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalAgent {
    /// Display name.
    pub name: String,
    /// Agent endpoint URL.
    pub url: String,
}

fn default_language() -> String {
    "en".to_string()
}

/// Detect if running inside a container (Docker, Podman, etc.).
fn is_containerized() -> bool {
    // Explicit env var (set in our Dockerfile)
    if std::env::var("RUSTYHAND_CONTAINER").is_ok() {
        return true;
    }
    // Linux: check for /.dockerenv or container cgroup
    #[cfg(target_os = "linux")]
    {
        if std::path::Path::new("/.dockerenv").exists() {
            return true;
        }
        if let Ok(cgroup) = std::fs::read_to_string("/proc/1/cgroup") {
            if cgroup.contains("docker")
                || cgroup.contains("kubepods")
                || cgroup.contains("containerd")
            {
                return true;
            }
        }
        // cgroup v2: /proc/self/mountinfo contains /docker/
        if let Ok(minfo) = std::fs::read_to_string("/proc/self/mountinfo") {
            if minfo.contains("/docker/") || minfo.contains("/containers/") {
                return true;
            }
        }
    }
    false
}

impl Default for KernelConfig {
    fn default() -> Self {
        let home_dir = rusty_hand_home_dir();
        let api_listen = if is_containerized() {
            "0.0.0.0:4200".to_string()
        } else {
            "127.0.0.1:4200".to_string()
        };
        Self {
            data_dir: home_dir.join("data"),
            home_dir,
            log_level: "info".to_string(),
            api_listen,
            network_enabled: false,
            default_model: DefaultModelConfig::default(),
            memory: MemoryConfig::default(),
            network: NetworkConfig::default(),
            channels: ChannelsConfig::default(),
            api_key: String::new(),
            mode: KernelMode::default(),
            language: "en".to_string(),
            users: Vec::new(),
            mcp_servers: Vec::new(),
            a2a: None,
            usage_footer: UsageFooterMode::default(),
            web: WebConfig::default(),
            fallback_providers: Vec::new(),
            browser: BrowserConfig::default(),
            extensions: ExtensionsConfig::default(),
            vault: VaultConfig::default(),
            workspaces_dir: None,
            media: crate::media::MediaConfig::default(),
            links: crate::media::LinkConfig::default(),
            reload: ReloadConfig::default(),
            webhook_triggers: None,
            approval: crate::approval::ApprovalPolicy::default(),
            max_cron_jobs: default_max_cron_jobs(),
            include: Vec::new(),
            exec_policy: ExecPolicy::default(),
            bindings: Vec::new(),
            broadcast: BroadcastConfig::default(),
            auto_reply: AutoReplyConfig::default(),
            canvas: CanvasConfig::default(),
            tts: TtsConfig::default(),
            docker: DockerSandboxConfig::default(),
            pairing: PairingConfig::default(),
            auth_profiles: HashMap::new(),
            thinking: None,
            budget: BudgetConfig::default(),
            proxy: ProxyConfig::default(),
            mcp_server: McpServerConfig::default(),
        }
    }
}

impl KernelConfig {
    /// Resolved workspaces root directory.
    pub fn effective_workspaces_dir(&self) -> PathBuf {
        self.workspaces_dir
            .clone()
            .unwrap_or_else(|| self.home_dir.join("workspaces"))
    }
}

/// SECURITY: Custom Debug impl redacts sensitive fields (api_key).
impl std::fmt::Debug for KernelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KernelConfig")
            .field("home_dir", &self.home_dir)
            .field("data_dir", &self.data_dir)
            .field("log_level", &self.log_level)
            .field("api_listen", &self.api_listen)
            .field("network_enabled", &self.network_enabled)
            .field("default_model", &self.default_model)
            .field("memory", &self.memory)
            .field("network", &self.network)
            .field("channels", &self.channels)
            .field(
                "api_key",
                &if self.api_key.is_empty() {
                    "<empty>"
                } else {
                    "<redacted>"
                },
            )
            .field("mode", &self.mode)
            .field("language", &self.language)
            .field("users", &format!("{} user(s)", self.users.len()))
            .field(
                "mcp_servers",
                &format!("{} server(s)", self.mcp_servers.len()),
            )
            .field("a2a", &self.a2a.as_ref().map(|a| a.enabled))
            .field("usage_footer", &self.usage_footer)
            .field("web", &self.web)
            .field(
                "fallback_providers",
                &format!("{} provider(s)", self.fallback_providers.len()),
            )
            .field("browser", &self.browser)
            .field("extensions", &self.extensions)
            .field("vault", &format!("enabled={}", self.vault.enabled))
            .field("workspaces_dir", &self.workspaces_dir)
            .field(
                "media",
                &format!(
                    "image={} audio={} video={}",
                    self.media.image_description,
                    self.media.audio_transcription,
                    self.media.video_description
                ),
            )
            .field("links", &format!("enabled={}", self.links.enabled))
            .field("reload", &self.reload.mode)
            .field(
                "webhook_triggers",
                &self.webhook_triggers.as_ref().map(|w| w.enabled),
            )
            .field(
                "approval",
                &format!("{} tool(s)", self.approval.require_approval.len()),
            )
            .field("max_cron_jobs", &self.max_cron_jobs)
            .field("include", &format!("{} file(s)", self.include.len()))
            .field("exec_policy", &self.exec_policy.mode)
            .field("bindings", &format!("{} binding(s)", self.bindings.len()))
            .field(
                "broadcast",
                &format!("{} route(s)", self.broadcast.routes.len()),
            )
            .field(
                "auto_reply",
                &format!("enabled={}", self.auto_reply.enabled),
            )
            .field("canvas", &format!("enabled={}", self.canvas.enabled))
            .field("tts", &format!("enabled={}", self.tts.enabled))
            .field("docker", &format!("enabled={}", self.docker.enabled))
            .field("pairing", &format!("enabled={}", self.pairing.enabled))
            .field(
                "auth_profiles",
                &format!("{} provider(s)", self.auth_profiles.len()),
            )
            .field("thinking", &self.thinking.is_some())
            .field(
                "proxy",
                &if self.proxy.is_enabled() {
                    format!(
                        "enabled url={} auth={} no_proxy={}",
                        self.proxy.url,
                        if self.proxy.password.is_empty() {
                            "none"
                        } else {
                            "<redacted>"
                        },
                        self.proxy.no_proxy.len()
                    )
                } else {
                    "disabled".to_string()
                },
            )
            .finish()
    }
}

/// Fallback home directory resolution.
fn dirs_next_home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(std::env::temp_dir)
}

/// Resolve the RustyHand home directory.
///
/// `RUSTY_HAND_HOME` takes precedence so container and service installs can
/// relocate all state without patching each subsystem separately.
pub fn rusty_hand_home_dir() -> PathBuf {
    std::env::var_os("RUSTY_HAND_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_next_home().join(".rustyhand"))
}

/// Default LLM model configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultModelConfig {
    /// Provider name (e.g., "anthropic", "openai").
    pub provider: String,
    /// Model identifier.
    pub model: String,
    /// Environment variable name for the API key.
    pub api_key_env: String,
    /// Optional base URL override.
    pub base_url: Option<String>,
}

impl Default for DefaultModelConfig {
    fn default() -> Self {
        Self {
            provider: "anthropic".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            api_key_env: "ANTHROPIC_API_KEY".to_string(),
            base_url: None,
        }
    }
}

/// Memory substrate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryConfig {
    /// Path to SQLite database file.
    pub sqlite_path: Option<PathBuf>,
    /// Embedding model for semantic search.
    pub embedding_model: String,
    /// Maximum memories before consolidation is triggered.
    pub consolidation_threshold: u64,
    /// Memory decay rate (0.0 = no decay, 1.0 = aggressive decay).
    pub decay_rate: f32,
    /// Embedding provider (e.g., "openai", "ollama"). None = auto-detect.
    #[serde(default)]
    pub embedding_provider: Option<String>,
    /// Environment variable name for the embedding API key.
    #[serde(default)]
    pub embedding_api_key_env: Option<String>,
    /// How often to run memory consolidation (hours). 0 = disabled.
    #[serde(default = "default_consolidation_interval")]
    pub consolidation_interval_hours: u64,
}

fn default_consolidation_interval() -> u64 {
    24
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            sqlite_path: None,
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            consolidation_threshold: 10_000,
            decay_rate: 0.1,
            embedding_provider: None,
            embedding_api_key_env: None,
            consolidation_interval_hours: default_consolidation_interval(),
        }
    }
}

/// Network layer configuration.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    /// libp2p listen addresses.
    pub listen_addresses: Vec<String>,
    /// Bootstrap peers for DHT.
    pub bootstrap_peers: Vec<String>,
    /// Enable mDNS for local discovery.
    pub mdns_enabled: bool,
    /// Maximum number of connected peers.
    pub max_peers: u32,
    /// Pre-shared secret for RHP HMAC authentication (required when network is enabled).
    pub shared_secret: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".to_string()],
            bootstrap_peers: vec![],
            mdns_enabled: true,
            max_peers: 50,
            shared_secret: String::new(),
        }
    }
}

/// SECURITY: Custom Debug impl redacts sensitive fields (shared_secret).
impl std::fmt::Debug for NetworkConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NetworkConfig")
            .field("listen_addresses", &self.listen_addresses)
            .field("bootstrap_peers", &self.bootstrap_peers)
            .field("mdns_enabled", &self.mdns_enabled)
            .field("max_peers", &self.max_peers)
            .field(
                "shared_secret",
                &if self.shared_secret.is_empty() {
                    "<empty>"
                } else {
                    "<redacted>"
                },
            )
            .finish()
    }
}

/// Channel bridge configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ChannelsConfig {
    /// Telegram bot configuration (None = disabled).
    pub telegram: Option<TelegramConfig>,
    /// Discord bot configuration (None = disabled).
    pub discord: Option<DiscordConfig>,
    /// Slack bot configuration (None = disabled).
    pub slack: Option<SlackConfig>,
}

/// Telegram channel adapter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TelegramConfig {
    /// Env var name holding the bot token (NOT the token itself).
    pub bot_token_env: String,
    /// Telegram user IDs allowed to interact (empty = allow all).
    pub allowed_users: Vec<i64>,
    /// Default agent name to route messages to.
    pub default_agent: Option<String>,
    /// Polling interval in seconds.
    pub poll_interval_secs: u64,
    /// Per-channel behavior overrides.
    #[serde(default)]
    pub overrides: ChannelOverrides,
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            bot_token_env: "TELEGRAM_BOT_TOKEN".to_string(),
            allowed_users: vec![],
            default_agent: None,
            poll_interval_secs: 1,
            overrides: ChannelOverrides::default(),
        }
    }
}

/// Discord channel adapter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiscordConfig {
    /// Env var name holding the bot token (NOT the token itself).
    pub bot_token_env: String,
    /// Guild (server) IDs allowed to interact (empty = allow all).
    pub allowed_guilds: Vec<u64>,
    /// Default agent name to route messages to.
    pub default_agent: Option<String>,
    /// Gateway intents bitmask (default: 33280 = GUILD_MESSAGES | MESSAGE_CONTENT).
    pub intents: u64,
    /// Per-channel behavior overrides.
    #[serde(default)]
    pub overrides: ChannelOverrides,
}

impl Default for DiscordConfig {
    fn default() -> Self {
        Self {
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
            allowed_guilds: vec![],
            default_agent: None,
            intents: 33280,
            overrides: ChannelOverrides::default(),
        }
    }
}

/// Slack channel adapter configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SlackConfig {
    /// Env var name holding the app-level token (xapp-) for Socket Mode.
    pub app_token_env: String,
    /// Env var name holding the bot token (xoxb-) for REST API.
    pub bot_token_env: String,
    /// Channel IDs allowed to interact (empty = allow all).
    pub allowed_channels: Vec<String>,
    /// Default agent name to route messages to.
    pub default_agent: Option<String>,
    /// Per-channel behavior overrides.
    #[serde(default)]
    pub overrides: ChannelOverrides,
}

impl Default for SlackConfig {
    fn default() -> Self {
        Self {
            app_token_env: "SLACK_APP_TOKEN".to_string(),
            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
            allowed_channels: vec![],
            default_agent: None,
            overrides: ChannelOverrides::default(),
        }
    }
}

impl KernelConfig {
    /// Validate the configuration, returning a list of warnings.
    ///
    /// Checks that env vars referenced by configured channels are set.
    pub fn validate(&self) -> Vec<String> {
        let mut warnings = Vec::new();

        if let Some(ref tg) = self.channels.telegram {
            if std::env::var(&tg.bot_token_env)
                .unwrap_or_default()
                .is_empty()
            {
                warnings.push(format!(
                    "Telegram configured but {} is not set",
                    tg.bot_token_env
                ));
            }
        }
        if let Some(ref dc) = self.channels.discord {
            if std::env::var(&dc.bot_token_env)
                .unwrap_or_default()
                .is_empty()
            {
                warnings.push(format!(
                    "Discord configured but {} is not set",
                    dc.bot_token_env
                ));
            }
        }
        if let Some(ref sl) = self.channels.slack {
            if std::env::var(&sl.app_token_env)
                .unwrap_or_default()
                .is_empty()
            {
                warnings.push(format!(
                    "Slack configured but {} is not set",
                    sl.app_token_env
                ));
            }
            if std::env::var(&sl.bot_token_env)
                .unwrap_or_default()
                .is_empty()
            {
                warnings.push(format!(
                    "Slack configured but {} is not set",
                    sl.bot_token_env
                ));
            }
        }

        // Web search provider validation
        match self.web.search_provider {
            SearchProvider::Brave => {
                if std::env::var(&self.web.brave.api_key_env)
                    .unwrap_or_default()
                    .is_empty()
                {
                    warnings.push(format!(
                        "Brave search selected but {} is not set",
                        self.web.brave.api_key_env
                    ));
                }
            }
            SearchProvider::Tavily => {
                if std::env::var(&self.web.tavily.api_key_env)
                    .unwrap_or_default()
                    .is_empty()
                {
                    warnings.push(format!(
                        "Tavily search selected but {} is not set",
                        self.web.tavily.api_key_env
                    ));
                }
            }
            SearchProvider::Perplexity => {
                if std::env::var(&self.web.perplexity.api_key_env)
                    .unwrap_or_default()
                    .is_empty()
                {
                    warnings.push(format!(
                        "Perplexity search selected but {} is not set",
                        self.web.perplexity.api_key_env
                    ));
                }
            }
            SearchProvider::DuckDuckGo | SearchProvider::Auto => {}
        }

        // --- LLM provider key validation ---
        //
        // Surface missing API keys at boot, not at first message. Without
        // this, a misconfigured provider (key env var missing or empty) is
        // only noticed when the agent first tries to call the LLM — too
        // late: the user has already sent a message, sees nothing happen,
        // and has to dig into the daemon logs to find the cause.
        //
        // De-duplicate by env var name: if `default_model.api_key_env` and
        // a fallback both reference `ANTHROPIC_API_KEY`, warn once.
        let mut checked_envs = std::collections::HashSet::new();
        let mut check_provider_key = |env_var: &str, referenced_by: &str, out: &mut Vec<String>| {
            if env_var.is_empty() || !checked_envs.insert(env_var.to_string()) {
                return;
            }
            if std::env::var(env_var).unwrap_or_default().is_empty() {
                out.push(format!(
                    "{referenced_by} configured but {env_var} is not set — \
                     export it in your environment (e.g. .env) before sending messages, \
                     or change `default_model.provider` in config.toml"
                ));
            }
        };

        check_provider_key(
            &self.default_model.api_key_env,
            &format!("default_model provider '{}'", self.default_model.provider),
            &mut warnings,
        );
        for fb in &self.fallback_providers {
            // Skip fallbacks with empty api_key_env (e.g. local Ollama which
            // doesn't need a key) and removed legacy providers (kernel
            // already filters those silently in resolve_driver).
            if fb.api_key_env.is_empty() {
                continue;
            }
            check_provider_key(
                &fb.api_key_env,
                &format!("fallback_providers entry '{}'", fb.provider),
                &mut warnings,
            );
        }

        // --- Core config validation ---

        // Validate api_listen is a parseable socket address
        if self.api_listen.parse::<std::net::SocketAddr>().is_err() {
            warnings.push(format!(
                "api_listen '{}' is not a valid address (expected host:port like 127.0.0.1:4200)",
                self.api_listen
            ));
        }

        // Validate proxy configuration if any (catches malformed URL / missing password
        // at boot rather than at first request).
        warnings.extend(self.proxy.validate());

        warnings
    }

    /// Clamp configuration values to safe production bounds.
    ///
    /// Called after loading config to prevent zero timeouts, unbounded buffers,
    /// or other misconfigurations that cause silent failures at runtime.
    pub fn clamp_bounds(&mut self) {
        // Browser timeout: min 5s, max 300s
        if self.browser.timeout_secs == 0 {
            self.browser.timeout_secs = 30;
        } else if self.browser.timeout_secs > 300 {
            self.browser.timeout_secs = 300;
        }

        // Browser max sessions: min 1, max 100
        if self.browser.max_sessions == 0 {
            self.browser.max_sessions = 3;
        } else if self.browser.max_sessions > 100 {
            self.browser.max_sessions = 100;
        }

        // Web fetch max_response_bytes: min 1KB, max 50MB
        if self.web.fetch.max_response_bytes == 0 {
            self.web.fetch.max_response_bytes = 5_000_000;
        } else if self.web.fetch.max_response_bytes > 50_000_000 {
            self.web.fetch.max_response_bytes = 50_000_000;
        }

        // Web fetch timeout: min 5s, max 120s
        if self.web.fetch.timeout_secs == 0 {
            self.web.fetch.timeout_secs = 30;
        } else if self.web.fetch.timeout_secs > 120 {
            self.web.fetch.timeout_secs = 120;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = KernelConfig::default();
        assert_eq!(config.log_level, "info");
        assert_eq!(config.api_listen, "127.0.0.1:4200");
        assert!(!config.network_enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = KernelConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        assert!(toml_str.contains("log_level"));
    }

    #[test]
    fn test_discord_config_defaults() {
        let dc = DiscordConfig::default();
        assert_eq!(dc.bot_token_env, "DISCORD_BOT_TOKEN");
        assert!(dc.allowed_guilds.is_empty());
        assert_eq!(dc.intents, 33280);
    }

    #[test]
    fn test_slack_config_defaults() {
        let sl = SlackConfig::default();
        assert_eq!(sl.app_token_env, "SLACK_APP_TOKEN");
        assert_eq!(sl.bot_token_env, "SLACK_BOT_TOKEN");
        assert!(sl.allowed_channels.is_empty());
    }

    #[test]
    fn test_validate_no_channels() {
        // Clear default_model.api_key_env so the LLM-key validation
        // (added in v0.7.19) doesn't trip on whatever the test host's
        // env happens to contain. The intent of this test is to prove
        // an unconfigured-channels config emits no warnings.
        let mut config = KernelConfig::default();
        config.default_model.api_key_env = String::new();
        let warnings = config.validate();
        assert!(
            warnings.is_empty(),
            "default config with no channels and no LLM key should emit no warnings, \
             got: {warnings:?}"
        );
    }

    /// Regression: pre-v0.7.19 a missing `ANTHROPIC_API_KEY` (or the
    /// equivalent for whatever provider was configured) produced no
    /// warning at boot. Users only discovered the misconfiguration
    /// when they sent the first message and saw nothing happen. This
    /// test pins the new behavior: validate() must surface a clear
    /// warning naming both the env var and the provider it backs.
    #[test]
    fn test_validate_warns_on_missing_default_model_key() {
        let mut config = KernelConfig::default();
        // Use a deterministically-unset env var name so the test is
        // race-free under cargo's parallel runner (no env mutation).
        config.default_model.provider = "anthropic".to_string();
        config.default_model.api_key_env = "RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DM".to_string();

        let warnings = config.validate();

        let llm_warnings: Vec<&String> = warnings
            .iter()
            .filter(|w| w.contains("RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DM"))
            .collect();
        assert_eq!(
            llm_warnings.len(),
            1,
            "missing LLM key must produce exactly one warning, got: {warnings:?}"
        );
        let warning = llm_warnings[0];
        assert!(
            warning.contains("anthropic"),
            "warning must name the provider, got: {warning}"
        );
        assert!(
            warning.contains("not set"),
            "warning must say 'not set', got: {warning}"
        );
    }

    /// De-duplicate by env var: if `default_model` and a fallback both
    /// reference the same env var (e.g. both ANTHROPIC_API_KEY), we
    /// must warn once, not twice.
    #[test]
    fn test_validate_dedupes_provider_key_warnings() {
        let mut config = KernelConfig::default();
        config.default_model.api_key_env = "RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DEDUP".to_string();
        config.fallback_providers = vec![
            FallbackProviderConfig {
                provider: "anthropic-backup".to_string(),
                model: "claude-haiku-4-5".to_string(),
                api_key_env: "RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DEDUP".to_string(),
                base_url: None,
            },
            FallbackProviderConfig {
                provider: "deepseek".to_string(),
                model: "deepseek-v4-flash".to_string(),
                api_key_env: "RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DS".to_string(),
                base_url: None,
            },
        ];

        let warnings = config.validate();
        let dedup_count = warnings
            .iter()
            .filter(|w| w.contains("RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DEDUP"))
            .count();
        let ds_count = warnings
            .iter()
            .filter(|w| w.contains("RUSTY_HAND_TEST_NONEXISTENT_LLM_KEY_DS"))
            .count();

        assert_eq!(
            dedup_count, 1,
            "duplicated env var must warn exactly once, got: {warnings:?}"
        );
        assert_eq!(
            ds_count, 1,
            "distinct env var must warn separately, got: {warnings:?}"
        );
    }

    /// Empty `api_key_env` (e.g. local Ollama needs no key) must not
    /// produce a warning — that's not a misconfiguration, it's the
    /// "no key required" signal.
    #[test]
    fn test_validate_skips_empty_api_key_env() {
        let mut config = KernelConfig::default();
        config.default_model.provider = "ollama".to_string();
        config.default_model.api_key_env = String::new();

        let warnings = config.validate();
        let llm_warnings: Vec<&String> =
            warnings.iter().filter(|w| w.contains("not set")).collect();
        assert!(
            llm_warnings.is_empty(),
            "empty api_key_env must not produce 'not set' warnings, got: {llm_warnings:?}"
        );
    }

    #[test]
    fn test_kernel_mode_default() {
        let mode = KernelMode::default();
        assert_eq!(mode, KernelMode::Default);
    }

    #[test]
    fn test_kernel_mode_serde() {
        let stable = KernelMode::Stable;
        let json = serde_json::to_string(&stable).unwrap();
        assert_eq!(json, "\"stable\"");
        let back: KernelMode = serde_json::from_str(&json).unwrap();
        assert_eq!(back, KernelMode::Stable);
    }

    #[test]
    fn test_user_config_serde() {
        let uc = UserConfig {
            name: "Alice".to_string(),
            role: "owner".to_string(),
            channel_bindings: {
                let mut m = std::collections::HashMap::new();
                m.insert("telegram".to_string(), "123456".to_string());
                m
            },
            api_key_hash: None,
        };
        let json = serde_json::to_string(&uc).unwrap();
        let back: UserConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "Alice");
        assert_eq!(back.role, "owner");
        assert_eq!(back.channel_bindings.get("telegram").unwrap(), "123456");
    }

    #[test]
    fn test_config_with_mode_and_language() {
        let config = KernelConfig {
            mode: KernelMode::Stable,
            language: "ar".to_string(),
            ..Default::default()
        };
        assert_eq!(config.mode, KernelMode::Stable);
        assert_eq!(config.language, "ar");
    }

    #[test]
    fn test_validate_missing_env_vars() {
        let mut config = KernelConfig::default();
        // Clear default LLM key check so this test stays focused on
        // channel validation regardless of host env.
        config.default_model.api_key_env = String::new();
        config.channels.discord = Some(DiscordConfig {
            bot_token_env: "RUSTY_HAND_TEST_NONEXISTENT_VAR_DC".to_string(),
            ..Default::default()
        });
        let warnings = config.validate();
        assert_eq!(
            warnings.len(),
            1,
            "expected one Discord warning, got: {warnings:?}"
        );
        assert!(warnings[0].contains("Discord"));
    }

    #[test]
    fn proxy_password_never_in_debug_output() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: "user".to_string(),
            password: "supersecret123".to_string(),
            no_proxy: vec![],
        };
        let debug_str = format!("{cfg:?}");
        // The actual password must NEVER appear, regardless of formatting.
        assert!(
            !debug_str.contains("supersecret123"),
            "password leaked into Debug output: {debug_str}"
        );
        assert!(
            debug_str.contains("<redacted>"),
            "password should be marked redacted, got: {debug_str}"
        );
        // Empty password should show <empty>, not <redacted>
        let cfg_empty = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: "user".to_string(),
            password: String::new(),
            no_proxy: vec![],
        };
        let debug_empty = format!("{cfg_empty:?}");
        assert!(debug_empty.contains("<empty>"));
        assert!(!debug_empty.contains("<redacted>"));
    }

    #[test]
    fn proxy_password_never_serialized_to_json() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: "user".to_string(),
            password: "supersecret123".to_string(),
            no_proxy: vec![],
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            !json.contains("supersecret123"),
            "password leaked into JSON: {json}"
        );
        assert!(!json.contains("password"), "password key in JSON: {json}");
    }

    #[test]
    fn proxy_disabled_by_default() {
        let cfg = KernelConfig::default();
        assert!(!cfg.proxy.is_enabled());
        assert!(cfg.proxy.url.is_empty());
    }

    #[test]
    fn proxy_is_enabled_when_url_set() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: String::new(),
            password: String::new(),
            no_proxy: vec![],
        };
        assert!(cfg.is_enabled());
    }

    #[test]
    fn proxy_validate_rejects_unknown_scheme() {
        let cfg = ProxyConfig {
            url: "ftp://proxy.example.com".to_string(),
            ..Default::default()
        };
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("must start with")));
    }

    #[test]
    fn proxy_validate_warns_on_missing_password() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com:8080".to_string(),
            username: "user".to_string(),
            password: String::new(),
            ..Default::default()
        };
        let warnings = cfg.validate();
        assert!(warnings.iter().any(|w| w.contains("password is empty")));
    }

    #[test]
    fn proxy_validate_accepts_valid_brightdata() {
        let cfg = ProxyConfig {
            url: "http://brd.superproxy.io:33335".to_string(),
            username: "brd-customer-hl_xxx-zone-residential".to_string(),
            password: "secret".to_string(),
            no_proxy: vec![],
        };
        assert!(cfg.validate().is_empty());
    }

    #[test]
    fn proxy_should_bypass_localhost_always() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com".to_string(),
            ..Default::default()
        };
        assert!(cfg.should_bypass("localhost"));
        assert!(cfg.should_bypass("127.0.0.1"));
        assert!(cfg.should_bypass("::1"));
    }

    #[test]
    fn proxy_should_bypass_exact_match() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com".to_string(),
            no_proxy: vec!["internal.corp".to_string()],
            ..Default::default()
        };
        assert!(cfg.should_bypass("internal.corp"));
        assert!(cfg.should_bypass("INTERNAL.CORP")); // case-insensitive
        assert!(!cfg.should_bypass("api.internal.corp"));
    }

    #[test]
    fn proxy_should_bypass_wildcard_suffix() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com".to_string(),
            no_proxy: vec!["*.local".to_string()],
            ..Default::default()
        };
        assert!(cfg.should_bypass("foo.local"));
        assert!(cfg.should_bypass("a.b.local"));
        // Standard semantics: bare suffix does NOT match — must add a separate exact entry.
        assert!(!cfg.should_bypass("local"));
        assert!(!cfg.should_bypass("local.org"));
        // Avoid prefix-collision false positives (e.g. "evilexample.com" must not match "*.example.com")
        assert!(!cfg.should_bypass("notlocal"));
    }

    #[test]
    fn proxy_should_bypass_wildcard_plus_exact() {
        // To bypass both subdomains AND the bare host, configure both.
        let cfg = ProxyConfig {
            url: "http://proxy.example.com".to_string(),
            no_proxy: vec!["*.example.com".to_string(), "example.com".to_string()],
            ..Default::default()
        };
        assert!(cfg.should_bypass("example.com"));
        assert!(cfg.should_bypass("foo.example.com"));
        assert!(!cfg.should_bypass("evilexample.com")); // no false positive
    }

    #[test]
    fn proxy_does_not_bypass_unrelated_host() {
        let cfg = ProxyConfig {
            url: "http://proxy.example.com".to_string(),
            no_proxy: vec!["internal.corp".to_string(), "*.local".to_string()],
            ..Default::default()
        };
        assert!(!cfg.should_bypass("olx.kz"));
        assert!(!cfg.should_bypass("api.example.com"));
    }

    #[test]
    fn test_validate_invalid_api_listen() {
        let config = KernelConfig {
            api_listen: "not-a-valid-address".to_string(),
            ..Default::default()
        };
        let warnings = config.validate();
        assert!(warnings.iter().any(|w| w.contains("api_listen")));
    }

    #[test]
    fn test_validate_valid_api_listen() {
        let config = KernelConfig::default(); // default is 127.0.0.1:4200
        let warnings = config.validate();
        assert!(!warnings.iter().any(|w| w.contains("api_listen")));
    }

    #[test]
    fn test_clamp_bounds_zero_timeout() {
        let mut config = KernelConfig::default();
        config.web.fetch.timeout_secs = 0;
        config.clamp_bounds();
        assert_eq!(config.web.fetch.timeout_secs, 30);
    }

    #[test]
    fn test_clamp_bounds_excessive_timeout() {
        let mut config = KernelConfig::default();
        config.web.fetch.timeout_secs = 999;
        config.clamp_bounds();
        assert_eq!(config.web.fetch.timeout_secs, 120);
    }

    // ── MCP server allowlist (security) ─────────────────────────────

    #[test]
    fn mcp_server_default_blocks_privileged_tools() {
        let cfg = McpServerConfig::default();
        assert!(cfg.enabled, "default must be enabled (auth-gated)");
        assert!(!cfg.allow_all_tools, "default must NOT allow everything");
        // The privileged tools an attacker would target:
        for privileged in &[
            "shell_exec",
            "file_write",
            "apply_patch",
            "skill_install",
            "agent_spawn",
            "agent_kill",
            "browser_execute_script",
            "self_update",
        ] {
            assert!(
                !cfg.is_tool_allowed(privileged),
                "{privileged} must be blocked by default — not in safe list"
            );
        }
    }

    #[test]
    fn mcp_server_default_allows_safe_tools() {
        let cfg = McpServerConfig::default();
        for safe in &[
            "agent_list",
            "agent_send",
            "memory_recall",
            "memory_store",
            "web_search",
            "web_fetch",
            "self_history",
            "task_list",
        ] {
            assert!(
                cfg.is_tool_allowed(safe),
                "{safe} must be allowed by default"
            );
        }
    }

    #[test]
    fn mcp_server_extra_allowed_tools_grants_access() {
        let cfg = McpServerConfig {
            enabled: true,
            extra_allowed_tools: vec!["skill_install".to_string(), "shell_exec".to_string()],
            allow_all_tools: false,
        };
        assert!(cfg.is_tool_allowed("skill_install"));
        assert!(cfg.is_tool_allowed("shell_exec"));
        // Other privileged tools still blocked.
        assert!(!cfg.is_tool_allowed("file_write"));
    }

    #[test]
    fn mcp_server_disabled_blocks_everything() {
        let cfg = McpServerConfig {
            enabled: false,
            extra_allowed_tools: vec!["agent_list".to_string()],
            allow_all_tools: true,
        };
        // Disabled overrides both allowlist AND allow_all_tools.
        assert!(!cfg.is_tool_allowed("agent_list"));
        assert!(!cfg.is_tool_allowed("skill_install"));
    }

    #[test]
    fn mcp_server_allow_all_permits_everything() {
        let cfg = McpServerConfig {
            enabled: true,
            extra_allowed_tools: vec![],
            allow_all_tools: true,
        };
        // Escape hatch for trusted local dev.
        assert!(cfg.is_tool_allowed("skill_install"));
        assert!(cfg.is_tool_allowed("shell_exec"));
        assert!(cfg.is_tool_allowed("any_tool_name_at_all"));
    }

    /// Regression: the docker entrypoint must write `[channels.telegram]`
    /// (not bare `[telegram]`). A top-level `[telegram]` table is not a
    /// field of `KernelConfig`, so serde silently dropped it before
    /// v0.7.5 — the daemon booted but never started the inbound
    /// listener. This pins the contract: the literal TOML the
    /// entrypoint emits must populate `channels.{telegram,discord,slack}`.
    #[test]
    fn test_docker_entrypoint_channels_toml_format() {
        let toml_text = r#"
api_listen = "0.0.0.0:4200"
log_level = "info"

[default_model]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"

[channels.telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
default_agent = "assistant"
allowed_users = [350911908, -1001234567890]

[channels.discord]
bot_token_env = "DISCORD_BOT_TOKEN"
default_agent = "assistant"

[channels.slack]
bot_token_env = "SLACK_BOT_TOKEN"
app_token_env = "SLACK_APP_TOKEN"
default_agent = "assistant"
"#;
        let cfg: KernelConfig = toml::from_str(toml_text).expect("entrypoint TOML must parse");
        let tg = cfg.channels.telegram.expect("channels.telegram missing");
        assert_eq!(tg.bot_token_env, "TELEGRAM_BOT_TOKEN");
        assert_eq!(tg.default_agent.as_deref(), Some("assistant"));
        assert_eq!(tg.allowed_users, vec![350911908_i64, -1001234567890_i64]);

        let dc = cfg.channels.discord.expect("channels.discord missing");
        assert_eq!(dc.bot_token_env, "DISCORD_BOT_TOKEN");
        assert_eq!(dc.default_agent.as_deref(), Some("assistant"));

        let sl = cfg.channels.slack.expect("channels.slack missing");
        assert_eq!(sl.bot_token_env, "SLACK_BOT_TOKEN");
        assert_eq!(sl.app_token_env, "SLACK_APP_TOKEN");
        assert_eq!(sl.default_agent.as_deref(), Some("assistant"));
    }

    #[test]
    fn test_channel_overrides_defaults() {
        let ov = ChannelOverrides::default();
        assert_eq!(ov.dm_policy, DmPolicy::Respond);
        assert_eq!(ov.group_policy, GroupPolicy::MentionOnly);
        assert_eq!(ov.rate_limit_per_user, 0);
        assert!(!ov.threading);
        assert!(ov.output_format.is_none());
        assert!(ov.model.is_none());
    }

    #[test]
    fn test_fallback_config_serde_roundtrip() {
        let fb = FallbackProviderConfig {
            provider: "ollama".to_string(),
            model: "llama3.2:latest".to_string(),
            api_key_env: String::new(),
            base_url: None,
        };
        let json = serde_json::to_string(&fb).unwrap();
        let back: FallbackProviderConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(back.provider, "ollama");
        assert_eq!(back.model, "llama3.2:latest");
        assert!(back.api_key_env.is_empty());
        assert!(back.base_url.is_none());
    }

    #[test]
    fn test_fallback_config_default_empty() {
        let config = KernelConfig::default();
        assert!(config.fallback_providers.is_empty());
    }

    #[test]
    fn test_fallback_config_in_toml() {
        let toml_str = r#"
            [[fallback_providers]]
            provider = "ollama"
            model = "llama3.2:latest"

            [[fallback_providers]]
            provider = "deepseek"
            model = "deepseek-chat"
            api_key_env = "DEEPSEEK_API_KEY"
        "#;
        let config: KernelConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.fallback_providers.len(), 2);
        assert_eq!(config.fallback_providers[0].provider, "ollama");
        assert_eq!(config.fallback_providers[1].provider, "deepseek");
    }

    #[test]
    fn test_channel_overrides_serde() {
        let ov = ChannelOverrides {
            dm_policy: DmPolicy::Ignore,
            group_policy: GroupPolicy::CommandsOnly,
            rate_limit_per_user: 10,
            threading: true,
            output_format: Some(OutputFormat::TelegramHtml),
            ..Default::default()
        };
        let json = serde_json::to_string(&ov).unwrap();
        let back: ChannelOverrides = serde_json::from_str(&json).unwrap();
        assert_eq!(back.dm_policy, DmPolicy::Ignore);
        assert_eq!(back.group_policy, GroupPolicy::CommandsOnly);
        assert_eq!(back.rate_limit_per_user, 10);
        assert!(back.threading);
        assert_eq!(back.output_format, Some(OutputFormat::TelegramHtml));
    }

    #[test]
    fn test_clamp_bounds_zero_browser_timeout() {
        let mut config = KernelConfig::default();
        config.browser.timeout_secs = 0;
        config.clamp_bounds();
        assert_eq!(config.browser.timeout_secs, 30);
    }

    #[test]
    fn test_clamp_bounds_excessive_browser_sessions() {
        let mut config = KernelConfig::default();
        config.browser.max_sessions = 999;
        config.clamp_bounds();
        assert_eq!(config.browser.max_sessions, 100);
    }

    #[test]
    fn test_clamp_bounds_zero_fetch_bytes() {
        let mut config = KernelConfig::default();
        config.web.fetch.max_response_bytes = 0;
        config.clamp_bounds();
        assert_eq!(config.web.fetch.max_response_bytes, 5_000_000);
    }

    #[test]
    fn test_clamp_bounds_zero_fetch_timeout() {
        let mut config = KernelConfig::default();
        config.web.fetch.timeout_secs = 0;
        config.clamp_bounds();
        assert_eq!(config.web.fetch.timeout_secs, 30);
    }

    #[test]
    fn test_clamp_bounds_defaults_unchanged() {
        let mut config = KernelConfig::default();
        let browser_timeout = config.browser.timeout_secs;
        let browser_sessions = config.browser.max_sessions;
        let fetch_bytes = config.web.fetch.max_response_bytes;
        let fetch_timeout = config.web.fetch.timeout_secs;
        config.clamp_bounds();
        assert_eq!(config.browser.timeout_secs, browser_timeout);
        assert_eq!(config.browser.max_sessions, browser_sessions);
        assert_eq!(config.web.fetch.max_response_bytes, fetch_bytes);
        assert_eq!(config.web.fetch.timeout_secs, fetch_timeout);
    }
}
