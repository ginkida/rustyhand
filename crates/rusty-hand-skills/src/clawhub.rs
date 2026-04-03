//! ClawHub marketplace client — search and install skills from clawhub.ai.
//!
//! ClawHub hosts 3,000+ community skills in both SKILL.md (prompt-only)
//! and package.json (Node.js) formats. This client downloads, converts,
//! and security-scans skills before installation.
//!
//! API reference: <https://clawhub.ai/api/v1/>
//! - Search: `GET /api/v1/search?q=...&limit=20`
//! - Browse: `GET /api/v1/skills?limit=20&sort=trending`
//! - Detail: `GET /api/v1/skills/{slug}`
//! - Download: `GET /api/v1/download?slug=...`
//! - File: `GET /api/v1/skills/{slug}/file?path=SKILL.md`

use crate::openclaw_compat;
use crate::verify::{SkillVerifier, SkillWarning, WarningSeverity};
use crate::SkillError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tracing::info;

// ---------------------------------------------------------------------------
// API response types (matching actual ClawHub v1 API — verified Feb 2026)
// ---------------------------------------------------------------------------

// -- Shared nested types ---------------------------------------------------

/// Stats nested inside browse entries and skill detail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubStats {
    #[serde(default)]
    pub comments: u64,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub installs_all_time: u64,
    #[serde(default)]
    pub installs_current: u64,
    #[serde(default)]
    pub stars: u64,
    #[serde(default)]
    pub versions: u64,
}

/// Version info nested inside browse entries and skill detail.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubVersionInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub changelog: String,
}

/// Owner info from the skill detail endpoint.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubOwner {
    #[serde(default)]
    pub handle: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub image: String,
}

// -- Browse: GET /api/v1/skills?limit=N&sort=trending ----------------------

/// A skill entry from the browse endpoint (`GET /api/v1/skills`).
///
/// Tags is a string→string map (e.g. `{"latest": "1.0.0"}`), not a list.
/// Timestamps are Unix milliseconds.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubBrowseEntry {
    pub slug: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summary: String,
    /// Version tags (e.g. `{"latest": "1.0.0"}`).
    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub stats: ClawHubStats,
    /// Unix ms timestamp.
    #[serde(default)]
    pub created_at: i64,
    /// Unix ms timestamp.
    #[serde(default)]
    pub updated_at: i64,
    #[serde(default)]
    pub latest_version: Option<ClawHubVersionInfo>,
}

/// Paginated response from the browse endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubBrowseResponse {
    pub items: Vec<ClawHubBrowseEntry>,
    #[serde(default)]
    pub next_cursor: Option<String>,
}

// -- Search: GET /api/v1/search?q=...&limit=N ------------------------------

/// A skill entry from the search endpoint (`GET /api/v1/search`).
///
/// Search results are much flatter than browse results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubSearchEntry {
    #[serde(default)]
    pub score: f64,
    pub slug: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub version: String,
    /// Unix ms timestamp.
    #[serde(default)]
    pub updated_at: i64,
}

/// Response from the search endpoint. Uses `results`, **not** `items`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubSearchResponse {
    pub results: Vec<ClawHubSearchEntry>,
}

// -- Detail: GET /api/v1/skills/{slug} -------------------------------------

/// The `skill` object nested inside the detail response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubSkillInfo {
    pub slug: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub stats: ClawHubStats,
    #[serde(default)]
    pub created_at: i64,
    #[serde(default)]
    pub updated_at: i64,
}

/// Full detail response from `GET /api/v1/skills/{slug}`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClawHubSkillDetail {
    pub skill: ClawHubSkillInfo,
    #[serde(default)]
    pub latest_version: Option<ClawHubVersionInfo>,
    #[serde(default)]
    pub owner: Option<ClawHubOwner>,
    /// Moderation status (null when clean).
    #[serde(default)]
    pub moderation: Option<serde_json::Value>,
}

// -- Sort enum -------------------------------------------------------------

/// Sort order for browsing skills.
#[derive(Debug, Clone, Copy)]
pub enum ClawHubSort {
    Trending,
    Updated,
    Downloads,
    Stars,
    Rating,
}

impl ClawHubSort {
    fn as_str(self) -> &'static str {
        match self {
            Self::Trending => "trending",
            Self::Updated => "updated",
            Self::Downloads => "downloads",
            Self::Stars => "stars",
            Self::Rating => "rating",
        }
    }
}

// -- Backward compat aliases -----------------------------------------------

/// Alias kept for code that still references the old name.
pub type ClawHubListResponse = ClawHubBrowseResponse;
/// Alias kept for code that still references the old name.
pub type ClawHubSearchResults = ClawHubSearchResponse;
/// Alias kept for code that still references the old name.
pub type ClawHubEntry = ClawHubBrowseEntry;

/// Result of installing a skill from ClawHub.
#[derive(Debug, Clone)]
pub struct ClawHubInstallResult {
    /// Installed skill name.
    pub skill_name: String,
    /// Installed version.
    pub version: String,
    /// The skill slug on ClawHub.
    pub slug: String,
    /// Security warnings from the scan pipeline.
    pub warnings: Vec<SkillWarning>,
    /// Tool name translations applied (OpenClaw → RustyHand).
    pub tool_translations: Vec<(String, String)>,
    /// Whether this is a prompt-only skill.
    pub is_prompt_only: bool,
}

/// Client for the ClawHub marketplace (clawhub.ai).
pub struct ClawHubClient {
    /// Base URL for the ClawHub API.
    base_url: String,
    /// HTTP client.
    client: reqwest::Client,
    /// Local cache directory for downloaded skills.
    _cache_dir: PathBuf,
}

impl ClawHubClient {
    /// Create a new ClawHub client with default settings.
    ///
    /// Uses the official ClawHub API at `https://clawhub.ai/api/v1`.
    pub fn new(cache_dir: PathBuf) -> Self {
        Self::with_url("https://clawhub.ai/api/v1", cache_dir)
    }

    /// Create a ClawHub client with a custom API URL.
    pub fn with_url(base_url: &str, cache_dir: PathBuf) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            _cache_dir: cache_dir,
        }
    }

    /// GET with retry on 429 (rate-limit). Up to 3 attempts with exponential
    /// back-off (1s, 2s, 4s). Respects `Retry-After` header when present.
    async fn get_with_retry(&self, url: &str) -> Result<reqwest::Response, SkillError> {
        let max_retries: u32 = 3;
        for attempt in 0..=max_retries {
            let response = self
                .client
                .get(url)
                .header("User-Agent", "RustyHand/0.1")
                .send()
                .await
                .map_err(|e| SkillError::Network(format!("Request failed: {e}")))?;

            if response.status() != reqwest::StatusCode::TOO_MANY_REQUESTS
                || attempt == max_retries
            {
                return Ok(response);
            }

            // Respect Retry-After header, fall back to exponential back-off
            let wait = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1 << attempt);
            let wait = wait.min(10); // cap at 10s

            info!(attempt, wait_secs = wait, url, "ClawHub 429, retrying");
            tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
        }
        Err(SkillError::Network(format!(
            "ClawHub request failed after {max_retries} retries: {url}"
        )))
    }

    /// Search for skills on ClawHub using vector/semantic search.
    ///
    /// Uses `GET /api/v1/search?q=...&limit=...`.
    /// Returns `ClawHubSearchResponse` whose root key is `results` (not `items`).
    pub async fn search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<ClawHubSearchResponse, SkillError> {
        let url = format!(
            "{}/search?q={}&limit={}",
            self.base_url,
            urlencoded(query),
            limit.min(50)
        );

        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(SkillError::Network(format!(
                "ClawHub API returned {}",
                response.status()
            )));
        }

        let results: ClawHubSearchResponse = response
            .json()
            .await
            .map_err(|e| SkillError::Network(format!("Failed to parse ClawHub response: {e}")))?;

        Ok(results)
    }

    /// Browse skills by sort order (trending, downloads, stars, etc.).
    ///
    /// Uses `GET /api/v1/skills?limit=...&sort=...`.
    pub async fn browse(
        &self,
        sort: ClawHubSort,
        limit: u32,
        cursor: Option<&str>,
    ) -> Result<ClawHubBrowseResponse, SkillError> {
        let mut url = format!(
            "{}/skills?limit={}&sort={}",
            self.base_url,
            limit.min(50),
            sort.as_str()
        );

        if let Some(c) = cursor {
            url.push_str(&format!("&cursor={}", urlencoded(c)));
        }

        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(SkillError::Network(format!(
                "ClawHub browse returned {}",
                response.status()
            )));
        }

        let results: ClawHubBrowseResponse = response
            .json()
            .await
            .map_err(|e| SkillError::Network(format!("Failed to parse ClawHub browse: {e}")))?;

        Ok(results)
    }

    /// Get detailed info about a specific skill.
    ///
    /// Uses `GET /api/v1/skills/{slug}`.
    /// Response is `{ skill: {...}, latestVersion: {...}, owner: {...}, moderation: null }`.
    pub async fn get_skill(&self, slug: &str) -> Result<ClawHubSkillDetail, SkillError> {
        let url = format!("{}/skills/{}", self.base_url, urlencoded(slug));

        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(SkillError::Network(format!(
                "ClawHub detail returned {}",
                response.status()
            )));
        }

        let detail: ClawHubSkillDetail = response
            .json()
            .await
            .map_err(|e| SkillError::Network(format!("Failed to parse ClawHub detail: {e}")))?;

        Ok(detail)
    }

    /// Helper: extract the version string from a browse entry.
    pub fn entry_version(entry: &ClawHubBrowseEntry) -> &str {
        entry
            .latest_version
            .as_ref()
            .map(|v| v.version.as_str())
            .or_else(|| entry.tags.get("latest").map(|s| s.as_str()))
            .unwrap_or("")
    }

    /// Fetch a specific file from a skill (e.g., SKILL.md, README).
    ///
    /// Uses `GET /api/v1/skills/{slug}/file?path=SKILL.md`.
    pub async fn get_file(&self, slug: &str, path: &str) -> Result<String, SkillError> {
        let url = format!(
            "{}/skills/{}/file?path={}",
            self.base_url,
            urlencoded(slug),
            urlencoded(path)
        );

        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(SkillError::Network(format!(
                "ClawHub file returned {}",
                response.status()
            )));
        }

        let text = response
            .text()
            .await
            .map_err(|e| SkillError::Network(format!("Failed to read ClawHub file: {e}")))?;

        Ok(text)
    }

    /// Install a skill from ClawHub into the target directory.
    ///
    /// Security pipeline:
    /// 1. Download skill zip and compute SHA256
    /// 2. Detect format (SKILL.md vs package.json)
    /// 3. Convert to RustyHand manifest
    /// 4. Run manifest security scan
    /// 5. If prompt-only: run prompt injection scan
    /// 6. Check binary dependencies
    /// 7. Write skill.toml with `verified: false`
    pub async fn install(
        &self,
        slug: &str,
        target_dir: &Path,
    ) -> Result<ClawHubInstallResult, SkillError> {
        // Use /api/v1/download?slug=... endpoint
        let url = format!("{}/download?slug={}", self.base_url, urlencoded(slug));

        info!(slug, "Downloading skill from ClawHub");

        let response = self.get_with_retry(&url).await?;

        if !response.status().is_success() {
            return Err(SkillError::Network(format!(
                "ClawHub download returned {}",
                response.status()
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| SkillError::Network(format!("Failed to read download: {e}")))?;

        // Step 1: SHA256 of downloaded content
        let sha256 = {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            hex::encode(hasher.finalize())
        };
        info!(slug, sha256 = %sha256, "Downloaded skill");

        // Create skill directory
        let skill_dir = target_dir.join(slug);
        std::fs::create_dir_all(&skill_dir)?;

        // Extract: ClawHub delivers as zip bundles. For now, detect content type
        // and write accordingly. A full zip extraction would use the `zip` crate.
        let content_str = String::from_utf8_lossy(&bytes);
        let is_skillmd = content_str.trim_start().starts_with("---");

        if is_skillmd {
            std::fs::write(skill_dir.join("SKILL.md"), &*bytes)?;
        } else {
            // Try to detect if it's a zip by magic bytes (PK\x03\x04)
            if bytes.len() >= 4 && bytes[0] == 0x50 && bytes[1] == 0x4b {
                // Extract ZIP contents into skill directory
                extract_zip(&bytes, &skill_dir)?;
                // If the ZIP didn't contain a SKILL.md, fetch it separately
                if !skill_dir.join("SKILL.md").exists() {
                    if let Ok(content) = self.get_file(slug, "SKILL.md").await {
                        std::fs::write(skill_dir.join("SKILL.md"), &content)?;
                    }
                }
            } else {
                std::fs::write(skill_dir.join("package.json"), &*bytes)?;
            }
        }

        // Step 2-3: Detect format and convert
        let mut all_warnings = Vec::new();
        let mut tool_translations = Vec::new();
        let mut is_prompt_only = false;

        let manifest = if is_skillmd || openclaw_compat::detect_skillmd(&skill_dir) {
            let converted = openclaw_compat::convert_skillmd(&skill_dir)?;
            tool_translations = converted.tool_translations;
            is_prompt_only =
                converted.manifest.runtime.runtime_type == crate::SkillRuntime::PromptOnly;

            // Step 5: Prompt injection scan
            let prompt_warnings = SkillVerifier::scan_prompt_content(&converted.prompt_context);
            if prompt_warnings
                .iter()
                .any(|w| w.severity == WarningSeverity::Critical)
            {
                // Block installation of skills with critical prompt injection
                let critical_msgs: Vec<_> = prompt_warnings
                    .iter()
                    .filter(|w| w.severity == WarningSeverity::Critical)
                    .map(|w| w.message.clone())
                    .collect();

                // Clean up skill directory on blocked install
                let _ = std::fs::remove_dir_all(&skill_dir);

                return Err(SkillError::SecurityBlocked(format!(
                    "Skill blocked due to prompt injection: {}",
                    critical_msgs.join("; ")
                )));
            }
            all_warnings.extend(prompt_warnings);

            // Write prompt context
            openclaw_compat::write_prompt_context(&skill_dir, &converted.prompt_context)?;

            // Step 6: Binary dependency check
            for bin in &converted.required_bins {
                if which_check(bin).is_none() {
                    all_warnings.push(SkillWarning {
                        severity: WarningSeverity::Warning,
                        message: format!("Required binary not found: {bin}"),
                    });
                }
            }

            converted.manifest
        } else if openclaw_compat::detect_openclaw_skill(&skill_dir) {
            openclaw_compat::convert_openclaw_skill(&skill_dir)?
        } else {
            return Err(SkillError::InvalidManifest(
                "Downloaded content is not a recognized skill format".to_string(),
            ));
        };

        // Step 4: Manifest security scan
        let manifest_warnings = SkillVerifier::security_scan(&manifest);
        all_warnings.extend(manifest_warnings);

        // Step 7: Write skill.toml
        openclaw_compat::write_rusty_hand_manifest(&skill_dir, &manifest)?;

        let result = ClawHubInstallResult {
            skill_name: manifest.skill.name.clone(),
            version: manifest.skill.version.clone(),
            slug: slug.to_string(),
            warnings: all_warnings,
            tool_translations,
            is_prompt_only,
        };

        info!(
            slug,
            skill_name = %result.skill_name,
            warnings = result.warnings.len(),
            "Installed skill from ClawHub"
        );

        Ok(result)
    }

    /// Check if a ClawHub skill is already installed locally.
    pub fn is_installed(&self, slug: &str, skills_dir: &Path) -> bool {
        let skill_dir = skills_dir.join(slug);
        skill_dir.join("skill.toml").exists()
    }
}

/// Extract a ZIP archive from in-memory bytes into `target_dir`.
///
/// Security: uses `enclosed_name()` to prevent path-traversal attacks.
/// Skips macOS resource-fork junk (`__MACOSX/`, `.DS_Store`).
/// Sets the executable bit on `.py`/`.sh`/`.js` files on Unix.
fn extract_zip(data: &[u8], target_dir: &Path) -> Result<(), SkillError> {
    let reader = Cursor::new(data);
    let mut archive = zip::ZipArchive::new(reader)
        .map_err(|e| SkillError::InvalidManifest(format!("Invalid ZIP archive: {e}")))?;

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| SkillError::InvalidManifest(format!("ZIP entry error: {e}")))?;

        // Prevent path traversal
        let Some(relative) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            continue;
        };

        let name_str = relative.to_string_lossy();

        // Skip macOS resource-fork junk
        if name_str.starts_with("__MACOSX") || name_str.ends_with(".DS_Store") {
            continue;
        }

        let out_path = target_dir.join(&relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&out_path)?;
            std::io::copy(&mut entry, &mut outfile)?;

            // Set executable bit on script files (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(ext) = out_path.extension().and_then(|e| e.to_str()) {
                    if matches!(ext, "py" | "sh" | "js") {
                        let _ =
                            std::fs::set_permissions(&out_path, std::fs::Permissions::from_mode(0o755));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Minimal URL-encoding for query parameters.
fn urlencoded(s: &str) -> String {
    s.replace(' ', "+")
        .replace('&', "%26")
        .replace('=', "%3D")
        .replace('?', "%3F")
        .replace('#', "%23")
        .replace('/', "%2F")
}

/// Check if a binary is available on PATH.
fn which_check(name: &str) -> Option<PathBuf> {
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("where").arg(name).output()
    } else {
        std::process::Command::new("which").arg(name).output()
    };

    match result {
        Ok(output) if output.status.success() => {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let first_line = path_str.lines().next()?;
            Some(PathBuf::from(first_line.trim()))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browse_entry_serde_real_format() {
        // Matches actual ClawHub browse API response (verified Feb 2026)
        let json = r#"{
            "slug": "sonoscli",
            "displayName": "Sonoscli",
            "summary": "Control Sonos speakers.",
            "tags": {"latest": "1.0.0"},
            "stats": {
                "comments": 1,
                "downloads": 19736,
                "installsAllTime": 455,
                "installsCurrent": 437,
                "stars": 15,
                "versions": 1
            },
            "createdAt": 1767545381030,
            "updatedAt": 1771777535889,
            "latestVersion": {
                "version": "1.0.0",
                "createdAt": 1767545381030,
                "changelog": ""
            }
        }"#;

        let entry: ClawHubBrowseEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.slug, "sonoscli");
        assert_eq!(entry.display_name, "Sonoscli");
        assert_eq!(entry.stats.downloads, 19736);
        assert_eq!(entry.stats.stars, 15);
        assert_eq!(entry.tags.get("latest").unwrap(), "1.0.0");
        assert_eq!(entry.latest_version.as_ref().unwrap().version, "1.0.0");
        assert_eq!(entry.updated_at, 1771777535889);
    }

    #[test]
    fn test_browse_response_serde() {
        let json = r#"{
            "items": [{
                "slug": "test",
                "displayName": "Test",
                "summary": "A test",
                "tags": {},
                "stats": {"downloads": 100, "stars": 5},
                "createdAt": 0,
                "updatedAt": 0
            }],
            "nextCursor": null
        }"#;

        let resp: ClawHubBrowseResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].slug, "test");
        assert_eq!(resp.items[0].stats.downloads, 100);
        assert!(resp.next_cursor.is_none());
    }

    #[test]
    fn test_search_entry_serde_real_format() {
        // Matches actual ClawHub search API response (verified Feb 2026)
        let json = r#"{
            "score": 3.7110556674218,
            "slug": "github",
            "displayName": "Github",
            "summary": "Interact with GitHub using the gh CLI.",
            "version": "1.0.0",
            "updatedAt": 1771777539580
        }"#;

        let entry: ClawHubSearchEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.slug, "github");
        assert_eq!(entry.display_name, "Github");
        assert!(entry.score > 3.0);
        assert_eq!(entry.version, "1.0.0");
        assert_eq!(entry.updated_at, 1771777539580);
    }

    #[test]
    fn test_search_response_serde() {
        // Search uses "results" not "items"
        let json = r#"{
            "results": [{
                "score": 3.5,
                "slug": "test",
                "displayName": "Test",
                "summary": "A test",
                "version": "0.1.0",
                "updatedAt": 0
            }]
        }"#;

        let resp: ClawHubSearchResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.results.len(), 1);
        assert_eq!(resp.results[0].slug, "test");
    }

    #[test]
    fn test_skill_detail_serde_real_format() {
        // Matches actual ClawHub detail API response (verified Feb 2026)
        let json = r##"{
            "skill": {
                "slug": "github",
                "displayName": "Github",
                "summary": "Interact with GitHub using the gh CLI.",
                "tags": {"latest": "1.0.0"},
                "stats": {
                    "comments": 3,
                    "downloads": 23790,
                    "installsAllTime": 428,
                    "installsCurrent": 417,
                    "stars": 67,
                    "versions": 1
                },
                "createdAt": 1767545344344,
                "updatedAt": 1771777539580
            },
            "latestVersion": {
                "version": "1.0.0",
                "createdAt": 1767545344344,
                "changelog": ""
            },
            "owner": {
                "handle": "steipete",
                "userId": "kn70pywhg0fyz996kpa8xj89s57yhv26",
                "displayName": "Peter Steinberger",
                "image": "https://avatars.githubusercontent.com/u/58493?v=4"
            },
            "moderation": null
        }"##;

        let detail: ClawHubSkillDetail = serde_json::from_str(json).unwrap();
        assert_eq!(detail.skill.slug, "github");
        assert_eq!(detail.skill.display_name, "Github");
        assert_eq!(detail.skill.stats.downloads, 23790);
        assert_eq!(detail.skill.stats.stars, 67);
        assert_eq!(detail.latest_version.as_ref().unwrap().version, "1.0.0");
        assert_eq!(detail.owner.as_ref().unwrap().handle, "steipete");
        assert!(detail.moderation.is_none());
    }

    #[test]
    fn test_clawhub_install_result() {
        let result = ClawHubInstallResult {
            skill_name: "test-skill".to_string(),
            version: "1.0.0".to_string(),
            slug: "test-skill".to_string(),
            warnings: vec![],
            tool_translations: vec![("Read".to_string(), "file_read".to_string())],
            is_prompt_only: true,
        };

        assert_eq!(result.skill_name, "test-skill");
        assert!(result.is_prompt_only);
        assert_eq!(result.tool_translations.len(), 1);
    }

    #[test]
    fn test_urlencoded() {
        assert_eq!(urlencoded("hello world"), "hello+world");
        assert_eq!(urlencoded("a&b=c"), "a%26b%3Dc");
        assert_eq!(urlencoded("path/to#frag"), "path%2Fto%23frag");
    }

    #[test]
    fn test_clawhub_sort_str() {
        assert_eq!(ClawHubSort::Trending.as_str(), "trending");
        assert_eq!(ClawHubSort::Downloads.as_str(), "downloads");
        assert_eq!(ClawHubSort::Stars.as_str(), "stars");
    }

    #[test]
    fn test_clawhub_client_url() {
        let client = ClawHubClient::new(PathBuf::from("/tmp/cache"));
        assert_eq!(client.base_url, "https://clawhub.ai/api/v1");
    }

    #[test]
    fn test_extract_zip_basic() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();

        // Build an in-memory ZIP with SKILL.md and scripts/run.py
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("SKILL.md", opts).unwrap();
            zip.write_all(b"---\nname: test\n---\nbody").unwrap();
            zip.start_file("scripts/run.py", opts).unwrap();
            zip.write_all(b"print('hello')").unwrap();
            zip.finish().unwrap();
        }

        extract_zip(buf.get_ref(), dir.path()).unwrap();

        assert!(dir.path().join("SKILL.md").exists());
        assert!(dir.path().join("scripts/run.py").exists());
        let content = std::fs::read_to_string(dir.path().join("scripts/run.py")).unwrap();
        assert_eq!(content, "print('hello')");

        // On Unix, verify executable bit
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::metadata(dir.path().join("scripts/run.py"))
                .unwrap()
                .permissions();
            assert_ne!(perms.mode() & 0o111, 0, "script should be executable");
        }
    }

    #[test]
    fn test_extract_zip_skips_macos_junk() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();

        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file("SKILL.md", opts).unwrap();
            zip.write_all(b"---\nname: t\n---\nb").unwrap();
            zip.start_file("__MACOSX/._SKILL.md", opts).unwrap();
            zip.write_all(b"junk").unwrap();
            zip.start_file(".DS_Store", opts).unwrap();
            zip.write_all(b"junk2").unwrap();
            zip.finish().unwrap();
        }

        extract_zip(buf.get_ref(), dir.path()).unwrap();

        assert!(dir.path().join("SKILL.md").exists());
        assert!(!dir.path().join("__MACOSX").exists());
        assert!(!dir.path().join(".DS_Store").exists());
    }

    #[test]
    fn test_entry_version_helper() {
        let entry = ClawHubBrowseEntry {
            slug: "test".to_string(),
            display_name: "Test".to_string(),
            summary: String::new(),
            tags: [("latest".to_string(), "2.0.0".to_string())]
                .into_iter()
                .collect(),
            stats: ClawHubStats::default(),
            created_at: 0,
            updated_at: 0,
            latest_version: Some(ClawHubVersionInfo {
                version: "2.0.0".to_string(),
                created_at: 0,
                changelog: String::new(),
            }),
        };
        assert_eq!(ClawHubClient::entry_version(&entry), "2.0.0");
    }
}
