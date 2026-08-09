//! Upward `.awc` discovery with canonical symlink containment.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::AwcError;

/// Name of the workspace state directory searched on each ancestor.
pub const WORKSPACE_DIR_NAME: &str = ".awc";

/// Finds the nearest workspace state directory for `start`.
///
/// The start path is canonicalized, then each ancestor directory is checked
/// nearest-first for [`WORKSPACE_DIR_NAME`]. A missing entry continues upward;
/// an entry whose canonical target escapes its containing canonical directory,
/// or is not a directory, fails with [`AwcError::UnsafeStatePath`] instead of
/// being skipped. The returned path is the canonical state directory. No
/// target file contents are read or modified; escaping targets are rejected
/// before any use.
pub fn discover(start: &Path) -> Result<PathBuf, AwcError> {
    Ok(discover_with_root(start)?.1)
}

/// Like [`discover`], also returning the canonical workspace root directory
/// (the canonical ancestor containing `.awc`) next to the canonical state dir.
pub fn discover_with_root(start: &Path) -> Result<(PathBuf, PathBuf), AwcError> {
    let mut dir = fs::canonicalize(start).map_err(AwcError::Io)?;
    loop {
        let entry = dir.join(WORKSPACE_DIR_NAME);
        match fs::symlink_metadata(&entry) {
            Ok(_) => {
                let root = fs::canonicalize(&dir).map_err(AwcError::Io)?;
                let state = fs::canonicalize(&entry).map_err(AwcError::Io)?;
                if !state.starts_with(&root) || !state.is_dir() {
                    return Err(AwcError::UnsafeStatePath);
                }
                return Ok((root, state));
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(AwcError::Io(err)),
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return Err(AwcError::WorkspaceNotFound),
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("awc-core-paths-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn make_workspace(dir: &Path) -> PathBuf {
        let state = dir.join(WORKSPACE_DIR_NAME);
        fs::create_dir_all(&state).expect("create .awc");
        state
    }

    fn canonical(path: &Path) -> PathBuf {
        fs::canonicalize(path).expect("canonicalize")
    }

    #[test]
    fn nearest_ancestor_wins() {
        let root = temp_dir("nearest");
        make_workspace(&root);
        let inner = make_workspace(&root.join("a"));
        let nested = root.join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(discover(&nested).unwrap(), canonical(&inner));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn continues_upward_when_ancestor_missing() {
        let root = temp_dir("upward");
        let top = make_workspace(&root);
        let nested = root.join("x").join("y");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(discover(&nested).unwrap(), canonical(&top));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn internal_symlink_returns_canonical_target() {
        let root = temp_dir("internal-link");
        let target = root.join("state").join("awc");
        fs::create_dir_all(&target).unwrap();
        symlink(&target, &root.join(WORKSPACE_DIR_NAME)).unwrap();
        let nested = root.join("deep").join("dir");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(discover(&nested).unwrap(), canonical(&target));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn escaping_symlink_rejected_without_target_use() {
        let root = temp_dir("escape");
        let outside = temp_dir("escape-target");
        let marker = outside.join("marker.txt");
        fs::write(&marker, b"untouched").unwrap();
        symlink(&outside, &root.join(WORKSPACE_DIR_NAME)).unwrap();

        let err = discover(&root).unwrap_err();
        assert!(matches!(err, AwcError::UnsafeStatePath));
        assert_eq!(fs::read(&marker).unwrap(), b"untouched");
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    #[test]
    fn escaping_symlink_fails_instead_of_skipping_to_outer() {
        let root = temp_dir("skip");
        make_workspace(&root);
        let mid = root.join("mid");
        fs::create_dir_all(&mid).unwrap();
        let outside = temp_dir("skip-target");
        symlink(&outside, &mid.join(WORKSPACE_DIR_NAME)).unwrap();
        let nested = mid.join("deep");
        fs::create_dir_all(&nested).unwrap();

        assert!(matches!(
            discover(&nested).unwrap_err(),
            AwcError::UnsafeStatePath
        ));
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    #[test]
    fn no_workspace_returns_not_found_without_creating_state() {
        let dir = temp_dir("none");
        fs::create_dir_all(dir.join("a").join("b")).unwrap();

        assert!(matches!(
            discover(&dir.join("a").join("b")).unwrap_err(),
            AwcError::WorkspaceNotFound
        ));
        assert!(!dir.join(WORKSPACE_DIR_NAME).exists());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn canonical_start_via_symlinked_parent() {
        let root = temp_dir("canonical-start");
        let real = root.join("real");
        fs::create_dir_all(&real).unwrap();
        let state = make_workspace(&real);
        let link = root.join("link");
        symlink(&real, &link).unwrap();
        let nested = link.join("sub");
        fs::create_dir_all(&nested).unwrap();

        assert_eq!(discover(&nested).unwrap(), canonical(&state));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn plain_file_named_awc_is_rejected() {
        let root = temp_dir("file-awc");
        fs::write(root.join(WORKSPACE_DIR_NAME), b"not a dir").unwrap();

        assert!(matches!(
            discover(&root).unwrap_err(),
            AwcError::UnsafeStatePath
        ));
        fs::remove_dir_all(&root).ok();
    }
}
