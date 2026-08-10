//! Upward `.awc` discovery with canonical symlink containment.

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::domain::PathOwnership;
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
                let state = canonicalize_state_within(&root, &entry)?;
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

/// Canonicalizes `state` and verifies it stays within the canonical `root`
/// and is a directory, returning the canonical state path.
///
/// This is the single established containment check used by both discovery
/// and `init`: a `.awc` symlink is accepted only when its canonical target
/// remains inside the workspace root; an escaping, broken, or non-directory
/// state path fails with [`AwcError::UnsafeStatePath`] without use.
pub(crate) fn canonicalize_state_within(root: &Path, state: &Path) -> Result<PathBuf, AwcError> {
    let canonical_state = fs::canonicalize(state).map_err(AwcError::Io)?;
    if !canonical_state.starts_with(root) || !canonical_state.is_dir() {
        return Err(AwcError::UnsafeStatePath);
    }
    Ok(canonical_state)
}

/// Creates or repairs one governed directory (`artifacts/`, `inbox/`, `tmp/`,
/// `trash/`) inside the workspace root (design: Config and paths).
///
/// An existing entry is validated through the same canonical containment
/// check as `.awc`: a real directory or a contained symlink is accepted and
/// its canonical path returned; an escaping, broken, or non-directory entry
/// fails with [`AwcError::UnsafeStatePath`] and is never used, replaced, or
/// deleted. A missing entry is created through its canonical parent, so a
/// configured name with parent components can never write outside the root.
pub fn ensure_governed_dir(root: &Path, name: &str) -> Result<PathBuf, AwcError> {
    let root = fs::canonicalize(root).map_err(AwcError::Io)?;
    let entry = root.join(name);
    match fs::symlink_metadata(&entry) {
        Ok(_) => canonicalize_state_within(&root, &entry),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let parent = entry.parent().ok_or(AwcError::UnsafeStatePath)?;
            let leaf = entry.file_name().ok_or(AwcError::UnsafeStatePath)?;
            let parent = fs::canonicalize(parent).map_err(AwcError::Io)?;
            if !parent.starts_with(&root) {
                return Err(AwcError::UnsafeStatePath);
            }
            let target = parent.join(leaf);
            fs::create_dir_all(&target).map_err(AwcError::Io)?;
            Ok(target)
        }
        Err(err) => Err(AwcError::Io(err)),
    }
}

/// Classifies a workspace-relative path by its first component against the
/// fixed policy set (design: Path policy). Pure lexical, no fs access.
pub fn classify_path(rel: &Path) -> PathOwnership {
    let Some(Component::Normal(first)) = rel.components().next() else {
        return PathOwnership::Unmanaged;
    };
    match first.to_str().unwrap_or("") {
        ".awc" | "artifacts" | "inbox" | "tmp" | "trash" => PathOwnership::AwcManaged,
        "AGENTS.md" | "SOUL.md" | "MEMORY.md" | "memory" | "skills" => {
            PathOwnership::AgentRuntimeManaged
        }
        "docs" => PathOwnership::UserManaged,
        ".git" | "target" => PathOwnership::Ignored,
        _ => PathOwnership::Unmanaged,
    }
}

/// Validates `rel` as an artifact write target under the canonical `root`:
/// lexical normalization (relative, no `..`), ownership policy (only
/// `artifacts/**` is writable), canonical containment of existing
/// components, and no symlink anywhere along the path. Escapes/symlinks →
/// [`AwcError::PathEscape`], protected paths → [`AwcError::ProtectedPath`],
/// other non-artifacts paths → [`AwcError::PathOwned`].
pub fn validate_artifact_target(root: &Path, rel: &str) -> Result<PathBuf, AwcError> {
    let rel = Path::new(rel);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(AwcError::PathEscape(rel.display().to_string()));
    }
    let parts: Vec<&OsStr> = rel
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| c.as_os_str())
        .collect();
    if parts.is_empty() {
        return Err(AwcError::PathEscape(rel.display().to_string()));
    }
    let rel: PathBuf = parts.iter().collect();
    let path = rel.display().to_string();

    match classify_path(&rel) {
        PathOwnership::AgentRuntimeManaged => return Err(AwcError::ProtectedPath(path)),
        PathOwnership::AwcManaged if parts[0] == OsStr::new("artifacts") => {}
        _ => return Err(AwcError::PathOwned(path)),
    }

    let root = fs::canonicalize(root).map_err(AwcError::Io)?;
    let components: Vec<Component<'_>> = rel.components().collect();
    let mut current = root.clone();
    for component in components.iter() {
        let candidate = current.join(component.as_os_str());
        match fs::symlink_metadata(&candidate) {
            Ok(meta) => {
                // Any symlink component — final or intermediate, contained or
                // escaping — is rejected; canonical targets are never followed.
                if meta.file_type().is_symlink() {
                    return Err(AwcError::PathEscape(path.clone()));
                }
                let canonical = fs::canonicalize(&candidate).map_err(AwcError::Io)?;
                if !canonical.starts_with(&root) {
                    return Err(AwcError::PathEscape(path.clone()));
                }
                current = canonical;
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => current = candidate,
            Err(err) => return Err(AwcError::Io(err)),
        }
    }
    Ok(current)
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

    #[test]
    fn governed_dirs_are_created_when_missing() {
        let root = temp_dir("gov-create");
        for name in ["artifacts", "inbox", "tmp", "trash"] {
            ensure_governed_dir(&root, name).expect("create governed dir");
            assert!(root.join(name).is_dir(), "missing {name}");
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn governed_dir_existing_is_kept_untouched() {
        let root = temp_dir("gov-existing");
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::write(root.join("artifacts").join("marker.txt"), b"keep").unwrap();
        ensure_governed_dir(&root, "artifacts").expect("existing dir accepted");
        assert_eq!(
            fs::read(root.join("artifacts").join("marker.txt")).unwrap(),
            b"keep"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn governed_dir_name_cannot_escape_via_parent_components() {
        let root = temp_dir("gov-dotdot");
        let pid = std::process::id();
        let outside = std::env::temp_dir().join(format!("awc-core-paths-{pid}-gov-dotdot-target"));
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).unwrap();
        let rel = format!("../awc-core-paths-{pid}-gov-dotdot-target/escape");

        let err = ensure_governed_dir(&root, &rel).unwrap_err();
        assert!(matches!(err, AwcError::UnsafeStatePath));
        assert!(
            !outside.join("escape").exists(),
            "no write outside the root"
        );
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    #[test]
    fn governed_dir_nested_name_creates_inside_existing_parent() {
        let root = temp_dir("gov-nested");
        fs::create_dir_all(root.join("data")).unwrap();
        let created = ensure_governed_dir(&root, "data/artifacts").expect("nested create");
        assert_eq!(created, canonical(&root.join("data").join("artifacts")));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn governed_dir_escaping_symlink_rejected_without_target_use() {
        let root = temp_dir("gov-escape");
        let outside = temp_dir("gov-escape-target");
        let marker = outside.join("marker.txt");
        fs::write(&marker, b"untouched").unwrap();
        symlink(&outside, &root.join("artifacts")).unwrap();

        assert!(matches!(
            ensure_governed_dir(&root, "artifacts").unwrap_err(),
            AwcError::UnsafeStatePath
        ));
        assert_eq!(fs::read(&marker).unwrap(), b"untouched");
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    fn expect_protected(root: &Path, path: &str) {
        let err = validate_artifact_target(root, path).unwrap_err();
        assert!(matches!(err, AwcError::ProtectedPath(_)), "{path}");
    }

    fn expect_owned(root: &Path, path: &str) {
        let err = validate_artifact_target(root, path).unwrap_err();
        assert!(matches!(err, AwcError::PathOwned(_)), "{path}");
    }

    fn expect_escape(root: &Path, path: &str) {
        let err = validate_artifact_target(root, path).unwrap_err();
        assert!(matches!(err, AwcError::PathEscape(_)), "{path:?}");
    }

    #[test]
    fn classify_assigns_fixed_ownership_classes() {
        for (path, expected) in [
            ("artifacts/a.txt", PathOwnership::AwcManaged),
            ("AGENTS.md", PathOwnership::AgentRuntimeManaged),
            ("memory/notes.md", PathOwnership::AgentRuntimeManaged),
            (".git/config", PathOwnership::Ignored),
            ("docs/readme.md", PathOwnership::UserManaged),
            ("src/main.rs", PathOwnership::Unmanaged),
        ] {
            assert_eq!(classify_path(Path::new(path)), expected, "{path}");
        }
    }

    #[test]
    fn artifact_target_enforces_containment_ownership_and_symlinks() {
        let root = temp_dir("target");
        let outside = temp_dir("target-outside");
        fs::create_dir_all(root.join("artifacts")).unwrap();
        assert_eq!(
            validate_artifact_target(&root, "artifacts/new.txt").unwrap(),
            canonical(&root).join("artifacts").join("new.txt")
        );
        assert!(validate_artifact_target(&root, "artifacts/deep/nested.txt").is_ok());
        for path in ["AGENTS.md", "memory/notes.md"] {
            expect_protected(&root, path);
        }
        for path in [
            "docs/readme.md",
            "src/main.rs",
            ".git/config",
            "target/app",
            ".awc/state.sqlite3",
            "inbox/x",
            "tmp/x",
            "trash/x",
        ] {
            expect_owned(&root, path);
        }
        for path in ["../up", "artifacts/../x", "/etc/passwd", ""] {
            expect_escape(&root, path);
        }
        // A symlinked `artifacts` dir escaping the root is rejected...
        fs::remove_dir_all(root.join("artifacts")).unwrap();
        symlink(&outside, root.join("artifacts")).unwrap();
        expect_escape(&root, "artifacts/x.txt");
        // ...as is a CONTAINED intermediate symlink (never followed, even
        // when its canonical target stays inside the root).
        fs::remove_dir_all(root.join("artifacts")).unwrap();
        fs::create_dir_all(root.join("artifacts").join("real")).unwrap();
        symlink(
            root.join("artifacts").join("real"),
            root.join("artifacts").join("link"),
        )
        .unwrap();
        expect_escape(&root, "artifacts/link/x.txt");
        // ...as is a dangling final-component symlink (never followed).
        fs::remove_dir_all(root.join("artifacts")).unwrap();
        fs::create_dir_all(root.join("artifacts")).unwrap();
        symlink(
            outside.join("gone"),
            root.join("artifacts").join("dangling"),
        )
        .unwrap();
        expect_escape(&root, "artifacts/dangling");
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }
}
