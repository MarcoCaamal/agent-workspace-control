//! Adopt plan model and persistence: deterministic workspace fingerprint and
//! plan documents under `.awc/runtime/adopt/` (design: Adopt plan).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::AwcError;

/// Name of the runtime adopt directory inside the workspace state dir.
pub const ADOPT_RUNTIME_DIR: &str = "runtime/adopt";

/// Deterministic workspace fingerprint: sorted walk of path + mtime + size
/// over non-governed, non-ignored files. Any file addition, removal, or
/// modification changes the digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceFingerprint {
    pub digest: String,
    pub entries: usize,
}

/// One explicit plan action (design: Adopt plan).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PlanAction {
    Register {
        rel_path: String,
        artifact_type: String,
    },
    MoveToInbox {
        rel_path: String,
    },
    Skip {
        rel_path: String,
    },
}

/// An adopt plan document: explicit actions plus the workspace fingerprint
/// the plan was created against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptPlan {
    pub id: String,
    pub fingerprint: WorkspaceFingerprint,
    pub actions: Vec<PlanAction>,
}

/// Computes the workspace fingerprint for `root`: a sorted walk of
/// non-governed, non-ignored files recording path + mtime + size, hashed
/// with SHA-256.
pub fn workspace_fingerprint(root: &Path) -> Result<WorkspaceFingerprint, AwcError> {
    let mut lines: Vec<String> = Vec::new();
    collect_fingerprint_lines(root, root, &mut lines)?;
    lines.sort();
    let mut hasher = Sha256::new();
    for line in &lines {
        hasher.update(line.as_bytes());
        hasher.update(b"\n");
    }
    let digest = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok(WorkspaceFingerprint {
        digest,
        entries: lines.len(),
    })
}

fn collect_fingerprint_lines(
    root: &Path,
    dir: &Path,
    out: &mut Vec<String>,
) -> Result<(), AwcError> {
    let entries = fs::read_dir(dir).map_err(AwcError::Io)?;
    let mut names: Vec<_> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name())
        .collect();
    names.sort();
    for name in names {
        let entry = dir.join(&name);
        let rel = entry
            .strip_prefix(root)
            .map_err(|_| AwcError::UnsafeStatePath)?;
        let first = rel
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("");
        if matches!(
            first,
            ".awc"
                | "artifacts"
                | "inbox"
                | "tmp"
                | "trash"
                | ".git"
                | "target"
                | "node_modules"
                | "dist"
                | ".venv"
        ) {
            continue;
        }
        let meta = fs::symlink_metadata(&entry).map_err(AwcError::Io)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_dir() {
            collect_fingerprint_lines(root, &entry, out)?;
        } else {
            out.push(format!(
                "{}|{}|{}",
                rel.display(),
                meta.modified()
                    .map_err(AwcError::Io)?
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or_default(),
                meta.len(),
            ));
        }
    }
    Ok(())
}

/// Resolves the adopt runtime directory under the workspace state dir,
/// creating it when absent.
fn runtime_dir(state_dir: &Path) -> Result<PathBuf, AwcError> {
    let dir = state_dir.join(ADOPT_RUNTIME_DIR);
    fs::create_dir_all(&dir).map_err(AwcError::Io)?;
    Ok(dir)
}

/// Persists a plan document as `<plan-id>.json` under the adopt runtime dir.
/// Existing plans are never edited in place: a regenerated plan gets a new id.
pub fn save_plan(state_dir: &Path, plan: &AdoptPlan) -> Result<(), AwcError> {
    let dir = runtime_dir(state_dir)?;
    let path = dir.join(format!("{}.json", plan.id));
    let bytes = serde_json::to_vec_pretty(plan)
        .map_err(|e| AwcError::InvalidConfig(format!("serialize adopt plan: {e}")))?;
    fs::write(path, bytes).map_err(AwcError::Io)?;
    Ok(())
}

/// Loads a plan document by id; a missing plan reports
/// [`AwcError::AdoptPlanNotFound`].
pub fn load_plan(state_dir: &Path, id: &str) -> Result<AdoptPlan, AwcError> {
    let dir = runtime_dir(state_dir)?;
    let path = dir.join(format!("{id}.json"));
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(AwcError::AdoptPlanNotFound(id.to_string()));
        }
        Err(err) => return Err(AwcError::Io(err)),
    };
    serde_json::from_slice(&bytes)
        .map_err(|e| AwcError::InvalidConfig(format!("parse adopt plan: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("awc-core-adopt-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn fingerprint_is_deterministic_and_changes_on_modification() {
        let root = temp_dir("fp");
        fs::write(root.join("a.md"), b"hello").unwrap();
        let first = workspace_fingerprint(&root).unwrap();
        let second = workspace_fingerprint(&root).unwrap();
        assert_eq!(first, second, "deterministic");
        fs::write(root.join("a.md"), b"hello world").unwrap();
        let changed = workspace_fingerprint(&root).unwrap();
        assert_ne!(first, changed, "modification changes the digest");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn fingerprint_excludes_governed_and_ignored() {
        let root = temp_dir("fp-excl");
        fs::create_dir_all(root.join(".awc")).unwrap();
        fs::create_dir_all(root.join("node_modules")).unwrap();
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::write(root.join("a.md"), b"a").unwrap();
        fs::write(root.join(".awc").join("state.sqlite3"), b"db").unwrap();
        fs::write(root.join("node_modules").join("x.js"), b"x").unwrap();
        fs::write(root.join("artifacts").join("f.txt"), b"f").unwrap();
        let fp = workspace_fingerprint(&root).unwrap();
        assert_eq!(fp.entries, 1, "only a.md counts");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn plan_round_trips_through_store() {
        let root = temp_dir("plan-store");
        let state = root.join(".awc");
        fs::create_dir_all(&state).unwrap();
        let plan = AdoptPlan {
            id: "plan-001".to_string(),
            fingerprint: WorkspaceFingerprint {
                digest: "abc".to_string(),
                entries: 2,
            },
            actions: vec![
                PlanAction::Register {
                    rel_path: "adopt-plan.md".to_string(),
                    artifact_type: "plan".to_string(),
                },
                PlanAction::MoveToInbox {
                    rel_path: "notes.md".to_string(),
                },
                PlanAction::Skip {
                    rel_path: ".env".to_string(),
                },
            ],
        };
        save_plan(&state, &plan).expect("save");
        let loaded = load_plan(&state, "plan-001").expect("load");
        assert_eq!(loaded, plan);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_plan_reports_not_found() {
        let root = temp_dir("plan-missing");
        let state = root.join(".awc");
        fs::create_dir_all(&state).unwrap();
        assert!(matches!(
            load_plan(&state, "nope").expect_err("missing"),
            AwcError::AdoptPlanNotFound(_)
        ));
        fs::remove_dir_all(&root).ok();
    }
}
