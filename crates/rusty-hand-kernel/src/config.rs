//! Configuration loading from `~/.rustyhand/config.toml` with defaults.
//!
//! Supports config includes: the `include` field specifies additional TOML files
//! to load and deep-merge before the root config (root overrides includes).

use rusty_hand_types::config::KernelConfig;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::info;

/// Maximum include nesting depth.
const MAX_INCLUDE_DEPTH: u32 = 10;

/// Load kernel configuration from a TOML file, with defaults.
///
/// If the config contains an `include` field, included files are loaded
/// and deep-merged first, then the root config overrides them.
pub fn load_config(path: Option<&Path>) -> KernelConfig {
    let config_path = path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(default_config_path);

    if config_path.exists() {
        // SECURITY: Check file permissions on Unix — config may contain api_key
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(&config_path) {
                let mode = metadata.permissions().mode();
                if mode & 0o077 != 0 {
                    tracing::warn!(
                        path = %config_path.display(),
                        mode = format!("{:o}", mode),
                        "Config file is readable by group/others — fixing to 0600"
                    );
                    let _ = std::fs::set_permissions(
                        &config_path,
                        std::fs::Permissions::from_mode(0o600),
                    );
                }
            }
        }

        match std::fs::read_to_string(&config_path) {
            Ok(contents) => match toml::from_str::<toml::Value>(&contents) {
                Ok(mut root_value) => {
                    // Process includes before deserializing
                    let config_dir = config_path
                        .parent()
                        .unwrap_or_else(|| Path::new("."))
                        .to_path_buf();
                    let mut visited = HashSet::new();
                    if let Ok(canonical) = std::fs::canonicalize(&config_path) {
                        visited.insert(canonical);
                    } else {
                        visited.insert(config_path.clone());
                    }

                    if let Err(e) =
                        resolve_config_includes(&mut root_value, &config_dir, &mut visited, 0)
                    {
                        tracing::warn!(
                            error = %e,
                            "Config include resolution failed, using root config only"
                        );
                    }

                    // Remove the `include` field before deserializing to avoid confusion
                    if let toml::Value::Table(ref mut tbl) = root_value {
                        tbl.remove("include");
                    }

                    match root_value.try_into::<KernelConfig>() {
                        Ok(config) => {
                            info!(path = %config_path.display(), "Loaded configuration");
                            return config;
                        }
                        Err(e) => {
                            tracing::warn!(
                                error = %e,
                                path = %config_path.display(),
                                "Failed to deserialize merged config, using defaults"
                            );
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        path = %config_path.display(),
                        "Failed to parse config, using defaults"
                    );
                }
            },
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    path = %config_path.display(),
                    "Failed to read config file, using defaults"
                );
            }
        }
    } else {
        info!(
            path = %config_path.display(),
            "Config file not found, using defaults"
        );
    }

    KernelConfig::default()
}

/// Resolve config includes by deep-merging included files into the root value.
///
/// Included files are loaded first and the root config overrides them.
/// Security: rejects absolute paths, `..` components, and circular references.
fn resolve_config_includes(
    root_value: &mut toml::Value,
    config_dir: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: u32,
) -> Result<(), String> {
    if depth > MAX_INCLUDE_DEPTH {
        return Err(format!(
            "Config include depth exceeded maximum of {MAX_INCLUDE_DEPTH}"
        ));
    }

    // Extract include list from the current value
    let includes = match root_value {
        toml::Value::Table(tbl) => {
            if let Some(toml::Value::Array(arr)) = tbl.get("include") {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect::<Vec<_>>()
            } else {
                return Ok(());
            }
        }
        _ => return Ok(()),
    };

    if includes.is_empty() {
        return Ok(());
    }

    // Merge each include (earlier includes are overridden by later ones,
    // and the root config overrides everything).
    let mut merged_base = toml::Value::Table(toml::map::Map::new());

    for include_path_str in &includes {
        // SECURITY: reject absolute paths
        let include_path = Path::new(include_path_str);
        if include_path.is_absolute() {
            return Err(format!(
                "Config include rejects absolute path: {include_path_str}"
            ));
        }
        // SECURITY: reject `..` components
        for component in include_path.components() {
            if let std::path::Component::ParentDir = component {
                return Err(format!(
                    "Config include rejects path traversal: {include_path_str}"
                ));
            }
        }

        let resolved = config_dir.join(include_path);
        // SECURITY: verify resolved path stays within config dir
        let canonical = std::fs::canonicalize(&resolved).map_err(|e| {
            format!(
                "Config include '{}' cannot be resolved: {e}",
                include_path_str
            )
        })?;
        let canonical_dir = std::fs::canonicalize(config_dir)
            .map_err(|e| format!("Config dir cannot be canonicalized: {e}"))?;
        if !canonical.starts_with(&canonical_dir) {
            return Err(format!(
                "Config include '{}' escapes config directory",
                include_path_str
            ));
        }

        // SECURITY: circular detection
        if !visited.insert(canonical.clone()) {
            return Err(format!(
                "Circular config include detected: {include_path_str}"
            ));
        }

        info!(include = %include_path_str, "Loading config include");

        let contents = std::fs::read_to_string(&canonical)
            .map_err(|e| format!("Failed to read config include '{}': {e}", include_path_str))?;
        let mut include_value: toml::Value = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config include '{}': {e}", include_path_str))?;

        // Recursively resolve includes in the included file
        let include_dir = canonical.parent().unwrap_or(config_dir).to_path_buf();
        resolve_config_includes(&mut include_value, &include_dir, visited, depth + 1)?;

        // Remove include field from the included file
        if let toml::Value::Table(ref mut tbl) = include_value {
            tbl.remove("include");
        }

        // Deep merge: include overrides the base built so far
        deep_merge_toml(&mut merged_base, &include_value);
    }

    // Now deep merge: root overrides the merged includes
    // Save root's current values (minus include), then merge root on top
    let root_without_include = {
        let mut v = root_value.clone();
        if let toml::Value::Table(ref mut tbl) = v {
            tbl.remove("include");
        }
        v
    };
    deep_merge_toml(&mut merged_base, &root_without_include);
    *root_value = merged_base;

    Ok(())
}

/// Deep-merge two TOML values. `overlay` values override `base` values.
/// For tables, recursively merge. For everything else, overlay wins.
pub fn deep_merge_toml(base: &mut toml::Value, overlay: &toml::Value) {
    match (base, overlay) {
        (toml::Value::Table(base_tbl), toml::Value::Table(overlay_tbl)) => {
            for (key, overlay_val) in overlay_tbl {
                if let Some(base_val) = base_tbl.get_mut(key) {
                    deep_merge_toml(base_val, overlay_val);
                } else {
                    base_tbl.insert(key.clone(), overlay_val.clone());
                }
            }
        }
        (base, overlay) => {
            *base = overlay.clone();
        }
    }
}

/// Get the default config file path.
pub fn default_config_path() -> PathBuf {
    rusty_hand_types::config::rusty_hand_home_dir().join("config.toml")
}

/// Get the default RustyHand home directory.
pub fn rusty_hand_home() -> PathBuf {
    rusty_hand_types::config::rusty_hand_home_dir()
}

/// Ordered list of directories to search for agent manifests.
///
/// 1. `<home>/agents/` — user-customized templates (via `rustyhand init`
///    or hand-edited).
/// 2. `$RUSTY_HAND_AGENTS_DIR` — image-bundled templates. The official
///    Docker image sets this to `/opt/rustyhand/agents/` so a fresh
///    container with an empty `/data/agents/` still finds the bundled
///    `assistant`, `coordinator`, etc.
///
/// Without the env-var fallback, the channel bridge / template list
/// API / MCP `agent.list_templates` tool all came up empty on Docker
/// and the daemon replied "No agent assigned" to every Telegram
/// message.
pub fn agents_search_dirs_with_home(home_dir: &std::path::Path) -> Vec<PathBuf> {
    let env_dir = std::env::var("RUSTY_HAND_AGENTS_DIR")
        .ok()
        .filter(|s| !s.is_empty());
    agents_search_dirs_with_override(home_dir, env_dir.as_deref())
}

/// Same as `agents_search_dirs_with_home` but with the bundled-dir
/// override passed in explicitly. Lets unit tests exercise the
/// home-vs-bundle priority without mutating `RUSTY_HAND_AGENTS_DIR`,
/// which is data-race-unsafe under cargo's parallel test runner.
fn agents_search_dirs_with_override(
    home_dir: &std::path::Path,
    env_dir_override: Option<&str>,
) -> Vec<PathBuf> {
    let mut dirs = vec![home_dir.join("agents")];
    if let Some(env_dir) = env_dir_override.filter(|s| !s.is_empty()) {
        let p = PathBuf::from(env_dir);
        if !dirs.iter().any(|d| d == &p) {
            dirs.push(p);
        }
    }
    dirs
}

/// Same as `agents_search_dirs_with_home(&rusty_hand_home())`.
pub fn agents_search_dirs() -> Vec<PathBuf> {
    agents_search_dirs_with_home(&rusty_hand_home())
}

/// LLM providers still in the v0.7.x catalog. Manifests referencing
/// any other provider (e.g. groq, gemini, openai) won't spawn — `create_driver`
/// returns `Unknown provider`.
const SUPPORTED_PROVIDERS: &[&str] = &[
    "anthropic",
    "kimi",
    "deepseek",
    "minimax",
    "zhipu",
    "openrouter",
    "ollama",
];

/// Validate that a manifest at `path` parses and points at a supported
/// provider. Returns `Ok(())` on success, `Err(reason)` on failure.
fn validate_manifest_for_spawn(path: &std::path::Path) -> Result<(), String> {
    let contents = std::fs::read_to_string(path).map_err(|e| format!("read failed: {e}"))?;
    let manifest: rusty_hand_types::agent::AgentManifest =
        toml::from_str(&contents).map_err(|e| format!("toml parse failed: {e}"))?;
    let provider = manifest.model.provider.as_str();
    if !SUPPORTED_PROVIDERS.contains(&provider) {
        return Err(format!(
            "manifest declares removed provider '{provider}'; supported: {SUPPORTED_PROVIDERS:?}"
        ));
    }
    Ok(())
}

/// Find the first **valid** `<dir>/<name>/agent.toml` across the agents
/// search dirs.
///
/// "Valid" = parses as `AgentManifest` and uses a supported provider.
/// If a home-dir copy is invalid (typically a stale manifest persisted
/// before v0.7.0 stripped most providers), we log a warning and fall
/// through to the next dir — typically the image-bundled
/// `/opt/rustyhand/agents/` which has been kept current. Without this
/// fallthrough a stale `/data/agents/assistant/agent.toml` from an old
/// `rustyhand init` would silently mask every image upgrade.
pub fn resolve_agent_manifest_with_home(home_dir: &std::path::Path, name: &str) -> Option<PathBuf> {
    resolve_agent_manifest_in(&agents_search_dirs_with_home(home_dir), name)
}

/// Test seam for `resolve_agent_manifest_with_home`: take an explicit
/// list of search dirs instead of reading the env var. Lets unit tests
/// exercise the home-vs-bundle priority and the stale-manifest
/// fallthrough without mutating `RUSTY_HAND_AGENTS_DIR` (which is
/// data-race-unsafe under cargo's parallel test runner).
fn resolve_agent_manifest_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = dirs
        .iter()
        .map(|d| d.join(name).join("agent.toml"))
        .filter(|p| p.exists())
        .collect();

    let total = candidates.len();
    for (idx, path) in candidates.iter().enumerate() {
        match validate_manifest_for_spawn(path) {
            Ok(()) => return Some(path.clone()),
            Err(reason) => {
                if idx + 1 < total {
                    tracing::warn!(
                        manifest = %path.display(),
                        next = %candidates[idx + 1].display(),
                        reason = %reason,
                        "Stale agent manifest, falling through to next search dir"
                    );
                } else {
                    tracing::warn!(
                        manifest = %path.display(),
                        reason = %reason,
                        "Last available manifest is invalid; nothing to fall back to"
                    );
                }
            }
        }
    }
    None
}

/// Same as `resolve_agent_manifest_with_home(&rusty_hand_home(), name)`.
pub fn resolve_agent_manifest(name: &str) -> Option<PathBuf> {
    resolve_agent_manifest_with_home(&rusty_hand_home(), name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Regression: agents-search must include `$RUSTY_HAND_AGENTS_DIR`.
    /// Before this fallback the channel bridge / template list / MCP
    /// `agent.list_templates` tool all came up empty on Docker (where
    /// the bundled manifests live in `/opt/rustyhand/agents/` but
    /// `home_dir/agents/` is empty on a fresh volume) — every Telegram
    /// message replied "No agent assigned".
    ///
    /// All four cases are folded into one test because they mutate the
    /// process-wide env var and the parallel runner would otherwise race
    /// them.
    /// Build a minimum valid AgentManifest with the given provider, so
    /// resolve_agent_manifest_with_home can validate it.
    fn manifest_toml(name: &str, provider: &str) -> String {
        format!(
            r#"name = "{name}"
version = "0.1.0"
description = "test"
author = "test"
module = "builtin:chat"

[model]
provider = "{provider}"
model = "test-model"
max_tokens = 8192
temperature = 0.5
system_prompt = "test"

[capabilities]
"#
        )
    }

    #[test]
    fn agents_search_dirs_falls_back_to_env_var() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let bundle = tmp.path().join("bundle");
        std::fs::create_dir_all(home.join("agents")).unwrap();
        std::fs::create_dir_all(bundle.join("assistant")).unwrap();
        std::fs::write(
            bundle.join("assistant/agent.toml"),
            manifest_toml("assistant", "anthropic"),
        )
        .unwrap();

        // The tests use the explicit-override helpers
        // (`agents_search_dirs_with_override`,
        // `resolve_agent_manifest_in`) so we never mutate
        // `RUSTY_HAND_AGENTS_DIR` — `std::env::set_var` /
        // `remove_var` are data-race-unsafe under cargo's parallel
        // test runner (Rust 2024 edition marks them `unsafe` for that
        // reason). Race-free fixtures here mean CI doesn't flake when
        // unrelated tests in the same binary inspect process env.
        let bundle_str = bundle.to_string_lossy().to_string();
        let with_bundle = || agents_search_dirs_with_override(&home, Some(bundle_str.as_str()));
        let no_bundle = || agents_search_dirs_with_override(&home, None);
        let empty_bundle = || agents_search_dirs_with_override(&home, Some(""));

        // 1. Env var set, home empty → bundle manifest resolves (Docker).
        let dirs = with_bundle();
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0], home.join("agents"));
        assert_eq!(dirs[1], bundle);
        let env_only = resolve_agent_manifest_in(&dirs, "assistant");
        assert_eq!(
            env_only.as_deref(),
            Some(bundle.join("assistant/agent.toml").as_path()),
            "RUSTY_HAND_AGENTS_DIR fallback should resolve missing manifests"
        );

        // 2. User-customized home wins over bundle dir.
        std::fs::create_dir_all(home.join("agents/assistant")).unwrap();
        std::fs::write(
            home.join("agents/assistant/agent.toml"),
            manifest_toml("assistant", "anthropic"),
        )
        .unwrap();
        let home_wins = resolve_agent_manifest_in(&with_bundle(), "assistant");
        assert_eq!(
            home_wins.as_deref(),
            Some(home.join("agents/assistant/agent.toml").as_path()),
            "user-customized home_dir manifest must win over the bundle dir"
        );

        // 3. Unknown name still returns None even with both dirs available.
        assert!(resolve_agent_manifest_in(&with_bundle(), "ghost").is_none());

        // 4. Env var unset → only home dir is searched.
        let dirs = no_bundle();
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], home.join("agents"));
        assert!(resolve_agent_manifest_in(&dirs, "ghost").is_none());

        // 5. Empty env var is treated as unset.
        let dirs = empty_bundle();
        assert_eq!(dirs.len(), 1, "empty env var must be ignored");

        // 6. Stale home-dir manifest (v0.7.0-removed provider) must
        //    fall through to the bundled copy. This is the real-world
        //    chain that broke users on v0.7.9: a /data/agents/<name>/
        //    persisted by an old `rustyhand init` masks the fixed image
        //    copy; without fallthrough every spawn errors with
        //    "Unknown provider 'groq'" and the channel reply is "No
        //    agent assigned" forever.
        std::fs::write(
            home.join("agents/assistant/agent.toml"),
            manifest_toml("assistant", "groq"),
        )
        .unwrap();
        let stale_skipped = resolve_agent_manifest_in(&with_bundle(), "assistant");
        assert_eq!(
            stale_skipped.as_deref(),
            Some(bundle.join("assistant/agent.toml").as_path()),
            "stale home manifest must fall through to the bundled copy"
        );
    }

    #[test]
    fn test_load_config_defaults() {
        let config = load_config(None);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_load_config_missing_file() {
        let config = load_config(Some(Path::new("/nonexistent/config.toml")));
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_deep_merge_simple() {
        let mut base: toml::Value = toml::from_str(
            r#"
            log_level = "debug"
            api_listen = "0.0.0.0:4200"
        "#,
        )
        .unwrap();
        let overlay: toml::Value = toml::from_str(
            r#"
            log_level = "info"
            network_enabled = true
        "#,
        )
        .unwrap();
        deep_merge_toml(&mut base, &overlay);
        assert_eq!(base["log_level"].as_str(), Some("info"));
        assert_eq!(base["api_listen"].as_str(), Some("0.0.0.0:4200"));
        assert_eq!(base["network_enabled"].as_bool(), Some(true));
    }

    #[test]
    fn test_deep_merge_nested_tables() {
        let mut base: toml::Value = toml::from_str(
            r#"
            [memory]
            decay_rate = 0.1
            consolidation_threshold = 10000
        "#,
        )
        .unwrap();
        let overlay: toml::Value = toml::from_str(
            r#"
            [memory]
            decay_rate = 0.5
        "#,
        )
        .unwrap();
        deep_merge_toml(&mut base, &overlay);
        let mem = base["memory"].as_table().unwrap();
        assert_eq!(mem["decay_rate"].as_float(), Some(0.5));
        assert_eq!(mem["consolidation_threshold"].as_integer(), Some(10000));
    }

    #[test]
    fn test_basic_include() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base.toml");
        let root_path = dir.path().join("config.toml");

        // Base config
        let mut f = std::fs::File::create(&base_path).unwrap();
        writeln!(f, "log_level = \"debug\"").unwrap();
        writeln!(f, "api_listen = \"0.0.0.0:9999\"").unwrap();
        drop(f);

        // Root config (includes base, overrides log_level)
        let mut f = std::fs::File::create(&root_path).unwrap();
        writeln!(f, "include = [\"base.toml\"]").unwrap();
        writeln!(f, "log_level = \"warn\"").unwrap();
        drop(f);

        let config = load_config(Some(&root_path));
        assert_eq!(config.log_level, "warn"); // root overrides
        assert_eq!(config.api_listen, "0.0.0.0:9999"); // from base
    }

    #[test]
    fn test_nested_include() {
        let dir = tempfile::tempdir().unwrap();
        let grandchild = dir.path().join("grandchild.toml");
        let child = dir.path().join("child.toml");
        let root = dir.path().join("config.toml");

        let mut f = std::fs::File::create(&grandchild).unwrap();
        writeln!(f, "log_level = \"trace\"").unwrap();
        drop(f);

        let mut f = std::fs::File::create(&child).unwrap();
        writeln!(f, "include = [\"grandchild.toml\"]").unwrap();
        writeln!(f, "log_level = \"debug\"").unwrap();
        drop(f);

        let mut f = std::fs::File::create(&root).unwrap();
        writeln!(f, "include = [\"child.toml\"]").unwrap();
        writeln!(f, "log_level = \"info\"").unwrap();
        drop(f);

        let config = load_config(Some(&root));
        assert_eq!(config.log_level, "info"); // root wins
    }

    #[test]
    fn test_circular_include_detected() {
        let dir = tempfile::tempdir().unwrap();
        let a_path = dir.path().join("a.toml");
        let b_path = dir.path().join("b.toml");

        let mut f = std::fs::File::create(&a_path).unwrap();
        writeln!(f, "include = [\"b.toml\"]").unwrap();
        writeln!(f, "log_level = \"info\"").unwrap();
        drop(f);

        let mut f = std::fs::File::create(&b_path).unwrap();
        writeln!(f, "include = [\"a.toml\"]").unwrap();
        drop(f);

        // Should not panic — circular detection triggers, falls back gracefully
        let config = load_config(Some(&a_path));
        // Falls back to defaults due to the circular error
        assert!(!config.log_level.is_empty());
    }

    #[test]
    fn test_path_traversal_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("config.toml");

        let mut f = std::fs::File::create(&root).unwrap();
        writeln!(f, "include = [\"../etc/passwd\"]").unwrap();
        drop(f);

        // Should not panic — path traversal triggers error, falls back
        let config = load_config(Some(&root));
        assert_eq!(config.log_level, "info"); // defaults
    }

    #[test]
    fn test_max_depth_exceeded() {
        let dir = tempfile::tempdir().unwrap();

        // Create a chain of 12 files (exceeds MAX_INCLUDE_DEPTH=10)
        for i in (0..12).rev() {
            let name = format!("level{i}.toml");
            let path = dir.path().join(&name);
            let mut f = std::fs::File::create(&path).unwrap();
            if i < 11 {
                let next = format!("level{}.toml", i + 1);
                writeln!(f, "include = [\"{next}\"]").unwrap();
            }
            writeln!(f, "log_level = \"level{i}\"").unwrap();
            drop(f);
        }

        let root = dir.path().join("level0.toml");
        let config = load_config(Some(&root));
        // Falls back due to depth limit — but should not panic
        assert!(!config.log_level.is_empty());
    }

    #[test]
    fn test_absolute_path_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("config.toml");

        let mut f = std::fs::File::create(&root).unwrap();
        writeln!(f, "include = [\"/etc/shadow\"]").unwrap();
        drop(f);

        let config = load_config(Some(&root));
        assert_eq!(config.log_level, "info"); // defaults
    }

    #[test]
    fn test_no_includes_works() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("config.toml");

        let mut f = std::fs::File::create(&root).unwrap();
        writeln!(f, "log_level = \"trace\"").unwrap();
        drop(f);

        let config = load_config(Some(&root));
        assert_eq!(config.log_level, "trace");
    }
}
