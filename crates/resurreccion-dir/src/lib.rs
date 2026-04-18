//! Directory binding and path canonicalization for Resurreccion.
//!
//! Provides utilities to:
//! - Canonicalize filesystem paths
//! - Detect git repositories and extract metadata
//! - Compose stable binding keys from path and git context

use camino::Utf8PathBuf;
use resurreccion_core::BindingKey;

/// Metadata about a git repository.
#[derive(Debug, Clone)]
pub struct GitInfo {
    /// First remote URL (typically "origin"), or None if no remotes.
    pub remote: Option<String>,
    /// Basename of the git working directory.
    pub worktree_name: String,
}

/// Scope for binding key composition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Key scoped to canonical filesystem path only.
    PathScoped,
    /// Key scoped to git remote (or worktree name if no remote).
    RepoScoped,
}

/// Resolve a path to its canonical absolute UTF-8 path.
///
/// Resolves symlinks and returns an absolute, normalized path.
///
/// # Errors
///
/// Returns an error if:
/// - The path does not exist
/// - The path contains non-UTF-8 components
/// - Canonicalization fails (e.g., permission denied)
pub fn canonicalize(path: impl AsRef<std::path::Path>) -> anyhow::Result<Utf8PathBuf> {
    let canonical = std::fs::canonicalize(path)?;
    Utf8PathBuf::from_path_buf(canonical)
        .map_err(|p| anyhow::anyhow!("path is not valid UTF-8: {}", p.display()))
}

/// Detect if a path is inside a git repository and extract metadata.
///
/// # Errors
///
/// Returns an error if git discovery fails for reasons other than "not in a repo".
pub fn detect_git(path: &Utf8PathBuf) -> anyhow::Result<Option<GitInfo>> {
    match git2::Repository::discover(path.as_std_path()) {
        Ok(repo) => {
            let workdir_name = repo
                .workdir()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let remote = repo
                .find_remote("origin")
                .ok()
                .and_then(|r| r.url().map(std::string::ToString::to_string));
            Ok(Some(GitInfo {
                remote,
                worktree_name: workdir_name,
            }))
        }
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// Compose a stable binding key from path, optional git info, and scope.
///
/// The key is deterministic and stable across:
/// - Symlink resolution (both will have same canonical path)
/// - Machine restarts
/// - Repository clones (`RepoScoped` keys with remotes)
///
/// Two paths with the same canonical path but different scopes will produce different keys.
pub fn compose_binding_key(
    canonical: &Utf8PathBuf,
    git: Option<&GitInfo>,
    scope: Scope,
) -> BindingKey {
    let mut hasher = blake3::Hasher::new();

    // Mix the scope into the hash to ensure different scopes produce different keys.
    match scope {
        Scope::PathScoped => {
            hasher.update(b"path:");
            hasher.update(canonical.as_str().as_bytes());
        }
        Scope::RepoScoped => {
            hasher.update(b"repo:");
            if let Some(g) = git {
                if let Some(remote) = &g.remote {
                    hasher.update(remote.as_bytes());
                } else {
                    hasher.update(g.worktree_name.as_bytes());
                }
            } else {
                hasher.update(canonical.as_str().as_bytes());
            }
        }
    }

    BindingKey::from_bytes(*hasher.finalize().as_bytes())
}
