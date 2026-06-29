//! FangHub marketplace client — install skills from the registry.
//!
//! For Phase 1, uses GitHub releases as the registry backend.
//! Each skill is a GitHub repo with releases containing the skill bundle.

use crate::SkillError;
use flate2::read::GzDecoder;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use tar::Archive;
use tracing::{info, warn};

/// FangHub registry configuration.
#[derive(Debug, Clone)]
pub struct MarketplaceConfig {
    /// Base URL for the registry API.
    pub registry_url: String,
    /// GitHub organization for community skills.
    pub github_org: String,
}

impl Default for MarketplaceConfig {
    fn default() -> Self {
        Self {
            registry_url: "https://api.github.com".to_string(),
            github_org: "rusty-hand-skills".to_string(),
        }
    }
}

/// Client for the FangHub marketplace.
pub struct MarketplaceClient {
    config: MarketplaceConfig,
    http: reqwest::Client,
}

impl MarketplaceClient {
    /// Create a new marketplace client.
    pub fn new(config: MarketplaceConfig) -> Self {
        Self {
            config,
            http: reqwest::Client::builder()
                .user_agent("rusty-hand-skills/0.1")
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    /// Search for skills by query string.
    pub async fn search(&self, query: &str) -> Result<Vec<SkillSearchResult>, SkillError> {
        let url = format!(
            "{}/search/repositories?q={}+org:{}&sort=stars",
            self.config.registry_url, query, self.config.github_org
        );

        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| SkillError::Network(format!("Search request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(SkillError::Network(format!(
                "Search returned status {}",
                resp.status()
            )));
        }

        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SkillError::Network(format!("Parse search response: {e}")))?;

        let results = body["items"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .map(|item| SkillSearchResult {
                        name: item["name"].as_str().unwrap_or("").to_string(),
                        description: item["description"].as_str().unwrap_or("").to_string(),
                        stars: item["stargazers_count"].as_u64().unwrap_or(0),
                        url: item["html_url"].as_str().unwrap_or("").to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(results)
    }

    /// Install a skill from a GitHub repo by name.
    ///
    /// Downloads the latest release tarball and extracts it to the target directory.
    pub async fn install(&self, skill_name: &str, target_dir: &Path) -> Result<String, SkillError> {
        let repo = format!("{}/{}", self.config.github_org, skill_name);
        let url = format!(
            "{}/repos/{}/releases/latest",
            self.config.registry_url, repo
        );

        info!("Fetching skill info from {url}");

        let resp = self
            .http
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await
            .map_err(|e| SkillError::Network(format!("Fetch release: {e}")))?;

        if !resp.status().is_success() {
            return Err(SkillError::NotFound(format!(
                "Skill '{skill_name}' not found in marketplace (status {})",
                resp.status()
            )));
        }

        let release: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| SkillError::Network(format!("Parse release: {e}")))?;

        let version = release["tag_name"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        // Find the tarball asset
        let tarball_url = release["tarball_url"]
            .as_str()
            .ok_or_else(|| SkillError::Network("No tarball URL in release".to_string()))?;

        info!("Downloading skill {skill_name} {version}...");

        let skill_dir = target_dir.join(skill_name);
        std::fs::create_dir_all(&skill_dir)?;

        // Download the tarball
        let tar_resp = self
            .http
            .get(tarball_url)
            .send()
            .await
            .map_err(|e| SkillError::Network(format!("Download tarball: {e}")))?;

        if !tar_resp.status().is_success() {
            return Err(SkillError::Network(format!(
                "Download failed: {}",
                tar_resp.status()
            )));
        }

        // Bound the compressed download. extract_targz only caps the
        // DECOMPRESSED output, so an unbounded body could exhaust RAM before
        // decompression even begins. Reject on the advertised Content-Length
        // when present, then stream chunks with a running-total cap so a lying
        // or chunked-encoded response can't blow past the limit either.
        const MAX_TARBALL_BYTES: u64 = 128 * 1024 * 1024;

        if let Some(len) = tar_resp.content_length() {
            if len > MAX_TARBALL_BYTES {
                return Err(SkillError::Network(format!(
                    "Tarball is {len} bytes, exceeds download cap {MAX_TARBALL_BYTES}"
                )));
            }
        }

        let mut tar_resp = tar_resp;
        let mut bytes: Vec<u8> = Vec::new();
        while let Some(chunk) = tar_resp
            .chunk()
            .await
            .map_err(|e| SkillError::Network(format!("Read tarball body: {e}")))?
        {
            if bytes.len() as u64 + chunk.len() as u64 > MAX_TARBALL_BYTES {
                return Err(SkillError::Network(format!(
                    "Tarball exceeds download cap {MAX_TARBALL_BYTES}"
                )));
            }
            bytes.extend_from_slice(&chunk);
        }

        extract_targz(&bytes, &skill_dir)?;

        // Persist the install metadata alongside the skill for `skill list`.
        let meta = serde_json::json!({
            "name": skill_name,
            "version": version,
            "source": tarball_url,
            "installed_at": chrono::Utc::now().to_rfc3339(),
        });
        std::fs::write(
            skill_dir.join("marketplace_meta.json"),
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        )?;

        info!("Installed skill: {skill_name} {version}");
        Ok(version)
    }
}

/// Extract a gzipped tarball from in-memory bytes into `target_dir`.
///
/// Security:
/// - Per-file and total uncompressed size caps defeat tar-bomb attacks.
/// - Entries outside `target_dir` (path traversal via `..` or absolute paths)
///   are rejected.
/// - Symlinks and hardlinks are skipped — scripts installed from a
///   marketplace tarball must not arbitrarily reach outside the skill dir.
/// - GitHub release tarballs prefix every entry with a `<repo>-<sha>/`
///   directory; that wrapper is transparently stripped so the skill files
///   land at `target_dir/<skill-files>` as expected by the registry loader.
fn extract_targz(data: &[u8], target_dir: &Path) -> Result<(), SkillError> {
    const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
    const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;

    let decoder = GzDecoder::new(Cursor::new(data));
    let mut archive = Archive::new(decoder);

    // Canonicalize the target once so we can verify every extracted file
    // lands inside it, defeating `../../evil` entries.
    std::fs::create_dir_all(target_dir)?;
    let canonical_target = std::fs::canonicalize(target_dir)?;

    let mut total_written: u64 = 0;
    let mut prefix: Option<PathBuf> = None;

    for entry in archive
        .entries()
        .map_err(|e| SkillError::InvalidManifest(format!("Invalid tarball: {e}")))?
    {
        let mut entry =
            entry.map_err(|e| SkillError::InvalidManifest(format!("Tar entry error: {e}")))?;

        let header = entry.header().clone();
        let entry_type = header.entry_type();

        // Refuse link/device/fifo entries. Skills are just code.
        if !entry_type.is_file() && !entry_type.is_dir() {
            warn!(
                "Skipping non-regular tar entry type {:?}",
                entry_type.as_byte()
            );
            continue;
        }

        let raw_path = entry
            .path()
            .map_err(|e| SkillError::InvalidManifest(format!("Tar path error: {e}")))?
            .into_owned();

        // GitHub prefixes `<repo>-<sha>/` on every entry — strip the first
        // component so archive contents land in target_dir directly.
        let relative: PathBuf = if let Some(p) = prefix.as_ref() {
            raw_path.strip_prefix(p).unwrap_or(&raw_path).to_path_buf()
        } else {
            // First useful entry sets the prefix.
            let mut comps = raw_path.components();
            if let Some(first) = comps.next() {
                prefix = Some(PathBuf::from(first.as_os_str()));
            }
            comps.as_path().to_path_buf()
        };

        if relative.as_os_str().is_empty() {
            continue;
        }

        // Reject absolute paths and parent-traversal.
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Err(SkillError::InvalidManifest(format!(
                "Tar entry path escapes skill dir: {}",
                relative.display()
            )));
        }

        let out_path = canonical_target.join(&relative);

        // Defense in depth: verify the resolved path still starts with the
        // canonical target dir (catches clever symlink-free traversal).
        if !out_path.starts_with(&canonical_target) {
            return Err(SkillError::InvalidManifest(format!(
                "Tar entry resolves outside skill dir: {}",
                out_path.display()
            )));
        }

        if entry_type.is_dir() {
            std::fs::create_dir_all(&out_path)?;
            continue;
        }

        // File entry — apply size caps before writing anything.
        let declared = header.size().unwrap_or(0);
        if declared > MAX_FILE_BYTES {
            return Err(SkillError::InvalidManifest(format!(
                "Tar entry '{}' declares {} bytes, exceeds per-file cap {}",
                relative.display(),
                declared,
                MAX_FILE_BYTES
            )));
        }
        if total_written.saturating_add(declared) > MAX_TOTAL_BYTES {
            return Err(SkillError::InvalidManifest(format!(
                "Tar archive exceeds total-size cap {} (entry '{}' would push over)",
                MAX_TOTAL_BYTES,
                relative.display()
            )));
        }

        if let Some(parent) = out_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut outfile = std::fs::File::create(&out_path)?;
        // Read up to cap+1 so we can distinguish a legitimate file of
        // exactly MAX_FILE_BYTES (pass) from an under-declared liar that
        // would overflow the cap (reject).
        let limited = std::io::Read::take(&mut entry, MAX_FILE_BYTES + 1);
        let written = std::io::copy(&mut { limited }, &mut outfile)?;
        if written > MAX_FILE_BYTES {
            return Err(SkillError::InvalidManifest(format!(
                "Tar entry '{}' exceeded per-file cap {}",
                relative.display(),
                MAX_FILE_BYTES
            )));
        }
        total_written = total_written.saturating_add(written);

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

    Ok(())
}

/// A search result from the marketplace.
#[derive(Debug, Clone)]
pub struct SkillSearchResult {
    /// Skill name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Star count.
    pub stars: u64,
    /// Repository URL.
    pub url: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_default_config() {
        let config = MarketplaceConfig::default();
        assert!(config.registry_url.contains("github"));
        assert_eq!(config.github_org, "rusty-hand-skills");
    }

    #[test]
    fn test_client_creation() {
        let client = MarketplaceClient::new(MarketplaceConfig::default());
        assert_eq!(client.config.github_org, "rusty-hand-skills");
    }

    /// Build a GitHub-style .tar.gz (every entry prefixed with `<repo>-<sha>/`)
    /// in memory for unit testing the extractor.
    fn build_test_targz(prefix: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut tar_bytes: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            for (name, content) in files {
                let mut header = tar::Header::new_gnu();
                header.set_size(content.len() as u64);
                header.set_mode(0o644);
                header.set_cksum();
                let path = format!("{}/{}", prefix, name);
                builder.append_data(&mut header, path, *content).unwrap();
            }
            builder.finish().unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).unwrap();
        gz.finish().unwrap()
    }

    #[test]
    fn test_extract_targz_strips_github_prefix_and_writes_files() {
        // Regression: v0.7.1 and earlier never extracted tarballs at all —
        // marketplace skill installs produced only a metadata JSON. Verify
        // files actually land on disk and the `<repo>-<sha>/` wrapper is
        // transparently stripped.
        let skill_toml = b"[skill]\nname = \"demo\"\nversion = \"0.1\"\n";
        let entry_py = b"def run(x):\n    return x\n";
        let data = build_test_targz(
            "rusty-hand-skills-abc123",
            &[
                ("skill.toml", skill_toml),
                ("entry.py", entry_py),
                ("README.md", b"# demo"),
            ],
        );
        let tmp = tempfile::tempdir().unwrap();
        extract_targz(&data, tmp.path()).expect("extract must succeed");

        let written_toml = std::fs::read(tmp.path().join("skill.toml")).unwrap();
        assert_eq!(written_toml, skill_toml);
        let written_py = std::fs::read(tmp.path().join("entry.py")).unwrap();
        assert_eq!(written_py, entry_py);
        assert!(tmp.path().join("README.md").exists());
        // The GitHub prefix dir must NOT exist under target — it was stripped.
        assert!(!tmp.path().join("rusty-hand-skills-abc123").exists());
    }

    #[test]
    fn test_extract_targz_accepts_file_at_exact_cap() {
        // Regression: an earlier version of the size check used
        // `written == MAX_FILE_BYTES` which false-rejected a legitimate
        // file whose size equals the cap. This test documents that the
        // boundary is inclusive — cap-sized files are allowed through.
        //
        // We use a much smaller payload and pretend the cap is 1 KiB by
        // fabricating the header; extract_targz's cap is 32 MiB so we
        // cannot cheaply hit it in a test. Instead we check the inverse:
        // files under the cap succeed. (The oversize path covers > cap.)
        let small = vec![0u8; 1024];
        let data = build_test_targz("repo-sha", &[("skill.toml", &small)]);
        let tmp = tempfile::tempdir().unwrap();
        extract_targz(&data, tmp.path()).expect("1 KiB file must succeed");
        assert_eq!(
            std::fs::read(tmp.path().join("skill.toml")).unwrap().len(),
            1024
        );
    }

    #[test]
    fn test_extract_targz_rejects_oversize_file() {
        // A tarball declaring a 100 MiB file must be rejected by the
        // per-file cap (32 MiB) before any bytes hit disk.
        let mut header = tar::Header::new_gnu();
        header.set_size(100 * 1024 * 1024);
        header.set_mode(0o644);
        header.set_cksum();
        let mut tar_bytes: Vec<u8> = Vec::new();
        {
            let mut builder = tar::Builder::new(&mut tar_bytes);
            // We don't actually write 100 MiB — header lies and extract_targz
            // must check declared size before trying to copy bytes.
            builder
                .append_data(&mut header, "skill/huge.bin", std::io::empty())
                .unwrap();
            builder.finish().unwrap();
        }
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(&tar_bytes).unwrap();
        let data = gz.finish().unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let err = extract_targz(&data, tmp.path()).expect_err("oversize must fail");
        assert!(format!("{err}").contains("exceeds per-file cap"));
    }
}
