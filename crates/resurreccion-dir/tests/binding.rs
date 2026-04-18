//! Tests for directory binding key composition.

use camino::Utf8PathBuf;
use resurreccion_dir::{canonicalize, compose_binding_key, detect_git, Scope};
use std::os::unix::fs as unix_fs;
use tempfile::TempDir;

#[test]
fn test_canonicalize_returns_utf8_path() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let path = tempdir.path();

    let result = canonicalize(path).expect("canonicalize should succeed");

    assert!(result.is_absolute());
    assert_eq!(result.as_str(), path.canonicalize().unwrap().to_string_lossy());
}

#[test]
fn test_canonicalize_resolves_symlinks() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let tempdir_path = Utf8PathBuf::from_path_buf(tempdir.path().to_path_buf())
        .expect("tempdir should be valid UTF-8");

    let symlink_path = tempdir_path
        .parent()
        .unwrap()
        .join(format!("symlink_target_{}", std::process::id()));
    unix_fs::symlink(&tempdir_path, &symlink_path).expect("failed to create symlink");

    let canonical_direct = canonicalize(&tempdir_path).expect("canonicalize direct should succeed");
    let canonical_symlink =
        canonicalize(&symlink_path).expect("canonicalize symlink should succeed");

    assert_eq!(canonical_direct, canonical_symlink);
}

#[test]
fn test_distinct_paths_produce_distinct_keys() {
    let tempdir1 = TempDir::new().expect("failed to create tempdir");
    let tempdir2 = TempDir::new().expect("failed to create tempdir");

    let path1 =
        canonicalize(tempdir1.path()).expect("canonicalize tempdir1 should succeed");
    let path2 =
        canonicalize(tempdir2.path()).expect("canonicalize tempdir2 should succeed");

    let key1 = compose_binding_key(&path1, None, Scope::PathScoped);
    let key2 = compose_binding_key(&path2, None, Scope::PathScoped);

    assert_ne!(key1, key2);
}

#[test]
fn test_same_path_produces_same_key() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let path = canonicalize(tempdir.path()).expect("canonicalize should succeed");

    let key1 = compose_binding_key(&path, None, Scope::PathScoped);
    let key2 = compose_binding_key(&path, None, Scope::PathScoped);

    assert_eq!(key1, key2);
}

#[test]
fn test_path_scoped_and_repo_scoped_differ() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let path = canonicalize(tempdir.path()).expect("canonicalize should succeed");

    let key_path = compose_binding_key(&path, None, Scope::PathScoped);
    let key_repo = compose_binding_key(&path, None, Scope::RepoScoped);

    assert_ne!(key_path, key_repo);
}

#[test]
fn test_detect_git_returns_none_outside_repo() {
    let tempdir = TempDir::new().expect("failed to create tempdir");
    let path = canonicalize(tempdir.path()).expect("canonicalize should succeed");

    let result = detect_git(&path).expect("detect_git should not error");

    assert!(result.is_none());
}
