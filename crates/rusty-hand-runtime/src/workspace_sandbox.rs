//! Workspace filesystem sandboxing.
//!
//! Confines agent file operations to their workspace directory.
//! Prevents path traversal, symlink escapes, and access outside the sandbox.

use std::path::{Path, PathBuf};

/// Resolve a user-supplied path within a workspace sandbox.
///
/// - Rejects `..` components outright.
/// - Relative paths are joined with `workspace_root`.
/// - Absolute paths are checked against the workspace root after canonicalization.
/// - For new files: canonicalizes the parent directory and appends the filename.
/// - The final canonical path must start with the canonical workspace root.
pub fn resolve_sandbox_path(user_path: &str, workspace_root: &Path) -> Result<PathBuf, String> {
    let path = Path::new(user_path);

    // Reject any `..` components
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Path traversal denied: '..' components are forbidden".to_string());
        }
    }

    // Build the candidate path
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    // Canonicalize the workspace root
    let canon_root = workspace_root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve workspace root: {e}"))?;

    // Canonicalize the candidate (or its parent for new files)
    let canon_candidate = if candidate.exists() {
        candidate
            .canonicalize()
            .map_err(|e| format!("Failed to resolve path: {e}"))?
    } else {
        // For new files: canonicalize the parent and append the filename
        let parent = candidate
            .parent()
            .ok_or_else(|| "Invalid path: no parent directory".to_string())?;
        let filename = candidate
            .file_name()
            .ok_or_else(|| "Invalid path: no filename".to_string())?;
        let canon_parent = parent
            .canonicalize()
            .map_err(|e| format!("Failed to resolve parent directory: {e}"))?;
        canon_parent.join(filename)
    };

    // Verify the canonical path is inside the workspace
    if !canon_candidate.starts_with(&canon_root) {
        return Err(format!(
            "Access denied: path '{}' resolves outside workspace",
            user_path
        ));
    }

    Ok(canon_candidate)
}

/// Like [`resolve_sandbox_path`] but tolerates the target *and any number of its
/// leading ancestors* not yet existing (for `mkdir -p` / move / copy
/// destinations that create nested directories).
///
/// It finds the deepest *existing* ancestor, canonicalizes it (resolving any
/// symlinks along the real prefix), asserts that prefix is inside the sandbox,
/// then re-appends the non-existent tail. The tail components cannot be
/// symlinks — they don't exist yet — so no traversal can escape through them.
pub fn resolve_sandbox_path_allow_missing(
    user_path: &str,
    workspace_root: &Path,
) -> Result<PathBuf, String> {
    let path = Path::new(user_path);

    // Reject any `..` components (same as the strict resolver).
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err("Path traversal denied: '..' components are forbidden".to_string());
        }
    }

    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root.join(path)
    };

    let canon_root = workspace_root
        .canonicalize()
        .map_err(|e| format!("Failed to resolve workspace root: {e}"))?;

    // Walk up to the deepest ancestor that actually exists on disk.
    let mut existing = candidate.as_path();
    loop {
        if existing.exists() {
            break;
        }
        match existing.parent() {
            Some(parent) => existing = parent,
            None => return Err("Invalid path: no existing ancestor".to_string()),
        }
    }

    let canon_existing = existing
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {e}"))?;

    // The non-existent remainder after the deepest existing ancestor.
    let tail = candidate
        .strip_prefix(existing)
        .map_err(|_| "Invalid path: failed to derive non-existent tail".to_string())?;
    // When the whole path already exists the tail is empty; joining it would
    // append a trailing separator (`/foo/data.txt/`) and make the OS reject a
    // regular file with ENOTDIR. Return the canonical path unchanged instead.
    let resolved = if tail.as_os_str().is_empty() {
        canon_existing
    } else {
        canon_existing.join(tail)
    };

    if !resolved.starts_with(&canon_root) {
        return Err(format!(
            "Access denied: path '{}' resolves outside workspace",
            user_path
        ));
    }

    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_relative_path_inside_workspace() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("test.txt"), "hello").unwrap();

        let result = resolve_sandbox_path("data/test.txt", dir.path());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_absolute_path_inside_workspace() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("file.txt"), "ok").unwrap();
        let abs_path = dir.path().join("file.txt");

        let result = resolve_sandbox_path(abs_path.to_str().unwrap(), dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn test_absolute_path_outside_workspace_blocked() {
        let dir = TempDir::new().unwrap();
        let outside = std::env::temp_dir().join("outside_test.txt");
        std::fs::write(&outside, "nope").unwrap();

        let result = resolve_sandbox_path(outside.to_str().unwrap(), dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));

        let _ = std::fs::remove_file(&outside);
    }

    #[test]
    fn test_dotdot_component_blocked() {
        let dir = TempDir::new().unwrap();
        let result = resolve_sandbox_path("../../../etc/passwd", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path traversal denied"));
    }

    #[test]
    fn test_nonexistent_file_with_valid_parent() {
        let dir = TempDir::new().unwrap();
        let data_dir = dir.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();

        let result = resolve_sandbox_path("data/new_file.txt", dir.path());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));
        assert!(resolved.ends_with("new_file.txt"));
    }

    #[test]
    fn test_allow_missing_nested_dirs() {
        let dir = TempDir::new().unwrap();
        let result = resolve_sandbox_path_allow_missing("a/b/c/d", dir.path());
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.starts_with(dir.path().canonicalize().unwrap()));
        assert!(resolved.ends_with("a/b/c/d"));
    }

    #[test]
    fn test_allow_missing_existing_path() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("present.txt"), "ok").unwrap();
        let result = resolve_sandbox_path_allow_missing("present.txt", dir.path());
        assert!(result.is_ok());
        assert!(result.unwrap().ends_with("present.txt"));
    }

    #[test]
    fn test_allow_missing_dotdot_blocked() {
        let dir = TempDir::new().unwrap();
        let result = resolve_sandbox_path_allow_missing("../../etc/passwd", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Path traversal denied"));
    }

    #[test]
    fn test_allow_missing_absolute_outside_blocked() {
        let dir = TempDir::new().unwrap();
        let outside = std::env::temp_dir().join("rh_allow_missing_outside/new");
        let result = resolve_sandbox_path_allow_missing(outside.to_str().unwrap(), dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }

    #[cfg(unix)]
    #[test]
    fn test_allow_missing_symlink_prefix_escape_blocked() {
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        // A symlink inside the workspace that points outside it.
        let link_path = dir.path().join("escape");
        std::os::unix::fs::symlink(outside.path(), &link_path).unwrap();
        // Creating a new dir *under* the symlink must be rejected because the
        // canonical existing prefix (the symlink target) is outside the root.
        let result = resolve_sandbox_path_allow_missing("escape/new/dir", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlink_escape_blocked() {
        let dir = TempDir::new().unwrap();
        let outside = TempDir::new().unwrap();
        std::fs::write(outside.path().join("secret.txt"), "secret").unwrap();

        // Create a symlink inside the workspace pointing outside
        let link_path = dir.path().join("escape");
        std::os::unix::fs::symlink(outside.path(), &link_path).unwrap();

        let result = resolve_sandbox_path("escape/secret.txt", dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Access denied"));
    }
}
