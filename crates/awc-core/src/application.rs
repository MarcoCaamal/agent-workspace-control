//! Application use cases: `init`, and read-only `status` / `doctor_quick`.

use std::fs;
use std::path::Path;

use uuid::Uuid;

use crate::domain::{
    AddProject, AdoptCandidate, Artifact, ArtifactId, ArtifactStatus, CheckResult, CommandResult,
    InitStatus, ProjectId, QuickDoctor, ScanCategory, Status, can_transition, derive_slug,
    resolve_artifact_id_prefix, resolve_id_prefix, validate_slug,
};
use crate::error::AwcError;
use crate::infrastructure::adopt::{self, AdoptPlan, PlanAction};
use crate::infrastructure::artifacts::{ArtifactFs, OsFs};
use crate::infrastructure::classify::{self, SuggestedAction};
use crate::infrastructure::config;
use crate::infrastructure::paths::{self, WORKSPACE_DIR_NAME};
use crate::infrastructure::sqlite;

/// Initializes the workspace at `start`: canonical root/state safety, create
/// `.awc`, atomic default config only when absent (valid bytes preserved),
/// open and migrate the database, then create or repair the four governed
/// directories (`artifacts/`, `inbox/`, `tmp/`, `trash/` as configured or
/// defaulted) with containment validation. Failure before the config commit
/// removes only an empty `.awc` created by this invocation; database and
/// governed-dir failures after it propagate untouched (later `init` resumes
/// recovery).
pub fn init(start: &Path) -> Result<CommandResult, AwcError> {
    let root = fs::canonicalize(start).map_err(AwcError::Io)?;
    let state_dir = root.join(WORKSPACE_DIR_NAME);

    let mut created_state_dir = false;
    let state_dir = match fs::symlink_metadata(&state_dir) {
        Ok(_) => {
            // Accept a real directory or a `.awc` symlink whose canonical
            // target stays inside the root; reject anything escaping or not
            // a directory (same check discovery uses). Every later write
            // uses this validated canonical path, so retargeting the
            // `.awc` symlink cannot redirect config/database writes
            // outside the workspace.
            paths::canonicalize_state_within(&root, &state_dir)?
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(&state_dir).map_err(AwcError::Io)?;
            created_state_dir = true;
            state_dir
        }
        Err(err) => return Err(AwcError::Io(err)),
    };

    let config = match config::load_or_create(&state_dir) {
        Ok(config) => config,
        Err(err) => {
            if created_state_dir {
                // `remove_dir` removes only an empty dir, so pre-existing
                // state is never touched.
                let _ = fs::remove_dir(&state_dir);
            }
            return Err(err);
        }
    };

    let mut conn = sqlite::open(&state_dir.join(&config.database_file))?;
    sqlite::migrate(&mut conn)?;
    let schema_ok = sqlite::schema_health(&conn)?;

    // Governed directories live at the workspace root; each is created when
    // missing and validated for containment when present (design: Config and
    // paths). An escaping entry fails the whole init without any use.
    for name in [
        &config.artifacts_dir,
        &config.inbox_dir,
        &config.tmp_dir,
        &config.trash_dir,
    ] {
        paths::ensure_governed_dir(&root, name)?;
    }

    Ok(CommandResult::Init(InitStatus {
        root,
        schema_version: config.schema_version,
        database_ok: true,
        schema_ok,
    }))
}

/// Reports the discovered workspace without mutating anything. The database
/// opens read-only, so a missing database reports unhealthy instead of
/// being recreated; invalid config is an error (status carries the version).
pub fn status(start: &Path) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let (database_ok, schema_ok) =
        match sqlite::open_readonly(&state_dir.join(&config.database_file)) {
            Ok(conn) => match sqlite::schema_health(&conn) {
                Ok(ok) => (true, ok),
                Err(_) => (true, false),
            },
            Err(_) => (false, false),
        };
    Ok(CommandResult::Status(Status {
        root,
        schema_version: config.schema_version,
        database_ok,
        schema_ok,
    }))
}

/// Runs the four read-only quick checks — path, config, database, schema —
/// in fixed order, never repairing. An unsafe path reports only the failed
/// path check without touching the target.
pub fn doctor_quick(start: &Path) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = match paths::discover_with_root(start) {
        Ok(found) => found,
        Err(AwcError::UnsafeStatePath) => {
            let root = fs::canonicalize(start).map_err(AwcError::Io)?;
            return Ok(CommandResult::Doctor(QuickDoctor {
                root,
                checks: vec![CheckResult::failed(
                    "path",
                    "`.awc` escapes its workspace root",
                )],
            }));
        }
        Err(err) => return Err(err),
    };
    let mut checks = vec![CheckResult::ok("path")];
    match config::load_readonly(&state_dir) {
        Ok(config) => {
            checks.push(CheckResult::ok("config"));
            match sqlite::open_readonly(&state_dir.join(&config.database_file)) {
                Ok(conn) => {
                    checks.push(CheckResult::ok("database"));
                    match sqlite::schema_health(&conn) {
                        Ok(true) => checks.push(CheckResult::ok("schema")),
                        Ok(false) => checks.push(CheckResult::failed(
                            "schema",
                            "schema is missing or incomplete",
                        )),
                        Err(err) => {
                            checks.push(CheckResult::failed("schema", err.to_string()));
                        }
                    }
                }
                Err(err) => {
                    checks.push(CheckResult::failed("database", err.to_string()));
                    checks.push(CheckResult::failed("schema", "database unavailable"));
                }
            }
        }
        Err(err) => {
            checks.push(CheckResult::failed("config", err.to_string()));
            checks.push(CheckResult::failed("database", "config unavailable"));
            checks.push(CheckResult::failed("schema", "config unavailable"));
        }
    }
    Ok(CommandResult::Doctor(QuickDoctor { root, checks }))
}

/// Persists a project: the slug derives from `name` unless an explicit slug
/// is supplied (both validated by the same rules), `root_path` is stored as
/// external metadata only, and the created project is reported.
pub fn add_project(start: &Path, input: AddProject) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let slug = match &input.slug {
        Some(slug) => {
            validate_slug(slug)?;
            slug.clone()
        }
        None => derive_slug(&input.name)?,
    };
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;
    Ok(CommandResult::ProjectAdded(sqlite::insert_project(
        &mut conn,
        &slug,
        &input.name,
        input.root_path.as_deref(),
    )?))
}

/// Lists all projects in deterministic slug order.
pub fn list_projects(start: &Path) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let conn = sqlite::open_readonly(&state_dir.join(&config.database_file))?;
    let mut projects = sqlite::select_projects_by_id_prefix(&conn, "")?;
    projects.sort_by(|a, b| a.slug.cmp(&b.slug));
    Ok(CommandResult::ProjectList(projects))
}

/// Shows one project resolved by ID prefix: exactly one match selects it,
/// zero matches report not found, and two or more report ambiguity. The
/// persisted `root_path` is external context metadata only — nothing is
/// written there.
pub fn show_project(start: &Path, id_or_prefix: &str) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let conn = sqlite::open_readonly(&state_dir.join(&config.database_file))?;
    let candidates = sqlite::select_projects_by_id_prefix(&conn, id_or_prefix)?;
    let ids: Vec<Uuid> = candidates.iter().map(|p| p.id.0).collect();
    let id = resolve_id_prefix(id_or_prefix, &ids)?;
    let project = candidates
        .into_iter()
        .find(|p| p.id.0 == id)
        .ok_or(AwcError::ProjectNotFound)?;
    Ok(CommandResult::ProjectShown(project))
}

/// Creates a governed artifact: requires an existing project, title, and
/// type; derives the target `artifacts/<id>`; fingerprints the created
/// (initially empty) file; then commits metadata/audit and the file as one
/// compensated operation. On any failure the temporary or final file is
/// removed and no artifact row or audit event is committed.
pub fn create_artifact(
    start: &Path,
    project_id_or_prefix: &str,
    artifact_type: &str,
    title: &str,
) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;

    let project_candidates = sqlite::select_projects_by_id_prefix(&conn, project_id_or_prefix)?;
    let project_ids: Vec<Uuid> = project_candidates.iter().map(|p| p.id.0).collect();
    let project_id = resolve_id_prefix(project_id_or_prefix, &project_ids)?;
    let _project = project_candidates
        .into_iter()
        .find(|p| p.id.0 == project_id)
        .ok_or(AwcError::ProjectNotFound)?;

    let id = ArtifactId::new();
    let rel = format!("artifacts/{}", id.0);
    let target = paths::validate_artifact_target(&root, &rel)?;
    if sqlite::path_is_owned(&conn, &rel, None)? {
        return Err(AwcError::PathOwned(rel));
    }

    let fs = OsFs;
    let temp = fs.create_temp(&target)?;
    let fingerprint = crate::infrastructure::hash::fingerprint_file(&temp)?;
    if sqlite::fingerprint_is_duplicate(&conn, &fingerprint.sha256, fingerprint.size, None)? {
        let _ = fs.remove_file(&temp);
        return Err(AwcError::DuplicateFingerprint(fingerprint.sha256));
    }

    let now = chrono_now();
    let artifact = Artifact {
        id,
        project_id: ProjectId(project_id),
        artifact_type: artifact_type.to_string(),
        title: title.to_string(),
        path: Some(target.clone()),
        original_path: Some(target.clone()),
        status: ArtifactStatus::Active,
        sha256: Some(fingerprint.sha256),
        size: Some(fingerprint.size),
        last_seen_at: now.clone(),
        created_at: now.clone(),
        updated_at: now,
    };

    // Filesystem rename before the DB commit; on DB failure remove the final
    // file so no artifact row references a missing path.
    if let Err(err) = fs.rename(&temp, &target) {
        let _ = fs.remove_file(&temp);
        return Err(err);
    }
    if let Err(err) = sqlite::insert_artifact(&mut conn, &artifact) {
        let _ = fs.remove_file(&target);
        return Err(err);
    }
    Ok(CommandResult::ArtifactCreated(artifact))
}

/// Shows one artifact resolved by ID prefix (exactly one match selects it).
pub fn show_artifact(start: &Path, id_or_prefix: &str) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let conn = sqlite::open_readonly(&state_dir.join(&config.database_file))?;
    let artifact = resolve_one_artifact(&conn, id_or_prefix)?;
    Ok(CommandResult::ArtifactShown(artifact))
}

/// Lists artifacts with optional project/type/status filters, ordered by
/// `created_at DESC, id DESC`.
pub fn list_artifacts(
    start: &Path,
    project_id_or_prefix: Option<&str>,
    artifact_type: Option<&str>,
    status: Option<ArtifactStatus>,
) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let conn = sqlite::open_readonly(&state_dir.join(&config.database_file))?;
    let project_id = match project_id_or_prefix {
        Some(prefix) => {
            let candidates = sqlite::select_projects_by_id_prefix(&conn, prefix)?;
            let ids: Vec<Uuid> = candidates.iter().map(|p| p.id.0).collect();
            Some(ProjectId(resolve_id_prefix(prefix, &ids)?))
        }
        None => None,
    };
    Ok(CommandResult::ArtifactList(sqlite::list_artifacts(
        &conn,
        project_id,
        artifact_type,
        status,
    )?))
}

/// Archives an artifact: status-only transition active→archived plus audit,
/// all in one database transaction. No file moves.
pub fn archive_artifact(start: &Path, id_or_prefix: &str) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;
    let artifact = resolve_one_artifact(&conn, id_or_prefix)?;
    can_transition(artifact.status, ArtifactStatus::Archived)?;
    let mut updated = artifact.clone();
    updated.status = ArtifactStatus::Archived;
    updated.updated_at = chrono_now();
    sqlite::update_artifact(&mut conn, &updated, "artifact.archived")?;
    Ok(CommandResult::ArtifactShown(updated))
}

/// Trashes an artifact: physically moves the file into collision-safe
/// `trash/<id>-<basename>`, then updates metadata/status/audit. If the
/// database update fails after the move, the file is moved back.
pub fn trash_artifact(start: &Path, id_or_prefix: &str) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;
    let artifact = resolve_one_artifact(&conn, id_or_prefix)?;
    can_transition(artifact.status, ArtifactStatus::Trashed)?;
    let Some(current) = artifact.path.as_ref() else {
        return Err(AwcError::ArtifactStatusConflict(
            artifact.status,
            ArtifactStatus::Trashed,
        ));
    };
    let basename = current
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let trash_rel = format!("trash/{}-{basename}", artifact.id.0);
    let trash_target = paths::validate_artifact_target(&root, &trash_rel)?;
    if sqlite::path_is_owned(&conn, &trash_rel, None)? {
        return Err(AwcError::PathOwned(trash_rel));
    }

    let fs = OsFs;
    fs.move_file(current, &trash_target)?;
    let mut updated = artifact.clone();
    updated.status = ArtifactStatus::Trashed;
    updated.path = Some(trash_target.clone());
    updated.updated_at = chrono_now();
    if let Err(err) = sqlite::update_artifact(&mut conn, &updated, "artifact.trashed") {
        let _ = fs.move_file(&trash_target, current);
        return Err(err);
    }
    Ok(CommandResult::ArtifactShown(updated))
}

/// Restores an artifact to active: a trashed artifact's file is moved back
/// to its unoccupied original path, while an archived artifact needs no file
/// move (archive is status-only). A DB failure after a move moves the file
/// back to trash.
pub fn restore_artifact(start: &Path, id_or_prefix: &str) -> Result<CommandResult, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;
    let artifact = resolve_one_artifact(&conn, id_or_prefix)?;
    can_transition(artifact.status, ArtifactStatus::Active)?;
    let Some(original) = artifact.original_path.clone() else {
        return Err(AwcError::RestoreConflict(
            "artifact has no original path".into(),
        ));
    };

    // Archived artifacts never moved: only the status changes.
    if artifact.status == ArtifactStatus::Archived {
        let mut updated = artifact.clone();
        updated.status = ArtifactStatus::Active;
        updated.updated_at = chrono_now();
        sqlite::update_artifact(&mut conn, &updated, "artifact.restored")?;
        return Ok(CommandResult::ArtifactShown(updated));
    }

    if sqlite::path_is_owned(&conn, &original.to_string_lossy(), Some(artifact.id))? {
        return Err(AwcError::RestoreConflict(
            "original path is occupied".into(),
        ));
    }
    if original.exists() {
        return Err(AwcError::RestoreConflict(
            "original path exists on disk".into(),
        ));
    }
    let Some(current) = artifact.path.as_ref() else {
        return Err(AwcError::RestoreConflict(
            "artifact has no current file".into(),
        ));
    };

    let fs = OsFs;
    fs.move_file(current, &original)?;
    let mut updated = artifact.clone();
    updated.status = ArtifactStatus::Active;
    updated.path = Some(original.clone());
    updated.updated_at = chrono_now();
    if let Err(err) = sqlite::update_artifact(&mut conn, &updated, "artifact.restored") {
        let _ = fs.move_file(&original, current);
        return Err(err);
    }
    Ok(CommandResult::ArtifactShown(updated))
}

/// Relinks an artifact to a new `artifacts/` path: requires the old current
/// file absent, an unowned non-symlink target, then fingerprints the target
/// immediately before the metadata/audit update.
pub fn relink_artifact(
    start: &Path,
    id_or_prefix: &str,
    new_path: &str,
) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;
    let artifact = resolve_one_artifact(&conn, id_or_prefix)?;
    if let Some(current) = artifact.path.as_ref()
        && current.exists()
    {
        return Err(AwcError::RestoreConflict(
            "current file still exists; remove it before relinking".into(),
        ));
    }
    let target = paths::validate_artifact_target(&root, new_path)?;
    if sqlite::path_is_owned(&conn, new_path, Some(artifact.id))? {
        return Err(AwcError::PathOwned(new_path.to_string()));
    }
    let fingerprint = crate::infrastructure::hash::fingerprint_file(&target)?;
    if sqlite::fingerprint_is_duplicate(
        &conn,
        &fingerprint.sha256,
        fingerprint.size,
        Some(artifact.id),
    )? {
        return Err(AwcError::DuplicateFingerprint(fingerprint.sha256));
    }
    let mut updated = artifact.clone();
    updated.path = Some(target.clone());
    updated.sha256 = Some(fingerprint.sha256);
    updated.size = Some(fingerprint.size);
    updated.last_seen_at = chrono_now();
    updated.updated_at = chrono_now();
    sqlite::update_artifact(&mut conn, &updated, "artifact.relinked")?;
    Ok(CommandResult::ArtifactShown(updated))
}

/// Runs a read-only adopt scan: walks non-governed, non-ignored files under
/// the workspace root, classifies each with deterministic metadata-only
/// signals, and returns the ordered candidate report. No file is created,
/// moved, registered, deleted, or modified.
pub fn scan_adopt(start: &Path) -> Result<CommandResult, AwcError> {
    let (root, _) = paths::discover_with_root(start)?;
    let mut candidates = Vec::new();
    walk_adopt_dir(&root, &root, &mut candidates)?;
    candidates.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    Ok(CommandResult::AdoptScan(candidates))
}

/// Walks `dir` (canonical) under the canonical `root`, skipping governed
/// directories, ignored trees, and the `.awc` state dir, classifying every
/// other file.
fn walk_adopt_dir(root: &Path, dir: &Path, out: &mut Vec<AdoptCandidate>) -> Result<(), AwcError> {
    let entries = std::fs::read_dir(dir).map_err(AwcError::Io)?;
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
        // Skip the state dir and governed dirs entirely.
        if matches!(first, ".awc" | "artifacts" | "inbox" | "tmp" | "trash") {
            continue;
        }
        // Skip ignored trees (existing policy + extended adopt set).
        if matches!(first, ".git" | "target" | "node_modules" | "dist" | ".venv") {
            continue;
        }
        let meta = std::fs::symlink_metadata(&entry).map_err(AwcError::Io)?;
        if meta.file_type().is_symlink() {
            continue; // never follow symlinks during scan
        }
        if meta.is_dir() {
            walk_adopt_dir(root, &entry, out)?;
            continue;
        }
        let (category, action) = classify::classify(rel);
        let suggested_type = match (category, action) {
            (ScanCategory::ManagedCandidate, SuggestedAction::Register) => {
                Some(infer_artifact_type(rel))
            }
            _ => None,
        };
        out.push(AdoptCandidate {
            rel_path: rel.display().to_string(),
            category,
            suggested_type,
        });
    }
    Ok(())
}

/// Infers a concrete artifact type from the classified filename.
fn infer_artifact_type(rel: &std::path::Path) -> String {
    let name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    if name.contains("plan") {
        "plan".to_string()
    } else if name.contains("review") || name.starts_with("pr-") {
        "code_review".to_string()
    } else {
        "report".to_string()
    }
}

/// Creates an adopt plan from a scan: persists explicit per-candidate
/// actions plus the current workspace fingerprint under
/// `.awc/runtime/adopt/<plan-id>.json`. Regeneration-only.
pub fn plan_adopt(start: &Path) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let fingerprint = adopt::workspace_fingerprint(&root)?;
    let CommandResult::AdoptScan(candidates) = scan_adopt(&root)? else {
        return Err(AwcError::Usage("adopt plan requires a scan".into()));
    };
    let actions: Vec<PlanAction> = candidates
        .iter()
        .filter(|c| c.category != ScanCategory::KnownRuntime)
        .map(|c| match &c.suggested_type {
            Some(t) => PlanAction::Register {
                rel_path: c.rel_path.clone(),
                artifact_type: t.clone(),
            },
            None => match c.category {
                ScanCategory::SensitiveCandidate | ScanCategory::TemporaryCandidate => {
                    PlanAction::Skip {
                        rel_path: c.rel_path.clone(),
                    }
                }
                _ => PlanAction::MoveToInbox {
                    rel_path: c.rel_path.clone(),
                },
            },
        })
        .collect();
    let plan = AdoptPlan {
        id: format!("adopt-{}", chrono_now().replace([' ', ':'], "-")),
        fingerprint,
        actions,
    };
    adopt::save_plan(&state_dir, &plan)?;
    Ok(CommandResult::AdoptPlanCreated {
        plan_id: plan.id,
        actions: plan.actions.len(),
    })
}

/// Registers an EXISTING governed file under `artifacts/**` as an artifact:
/// fingerprints the current bytes, requires an unowned path and a mandatory
/// target project, and writes metadata + audit in one transaction. The file
/// is not moved: its current path becomes both `path` and `original_path`.
pub fn register_existing_artifact(
    start: &Path,
    project_id_or_prefix: &str,
    rel_path: &str,
    artifact_type: &str,
    title: &str,
) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let mut conn = sqlite::open_readwrite(&state_dir.join(&config.database_file))?;

    let project_candidates = sqlite::select_projects_by_id_prefix(&conn, project_id_or_prefix)?;
    let project_ids: Vec<Uuid> = project_candidates.iter().map(|p| p.id.0).collect();
    let project_id = resolve_id_prefix(project_id_or_prefix, &project_ids)?;
    let _project = project_candidates
        .into_iter()
        .find(|p| p.id.0 == project_id)
        .ok_or(AwcError::ProjectNotFound)?;

    let target = paths::validate_artifact_target(&root, rel_path)?;
    let target_str = target.to_string_lossy().to_string();
    if sqlite::path_is_owned(&conn, &target_str, None)? {
        return Err(AwcError::PathOwned(rel_path.to_string()));
    }
    let fingerprint = crate::infrastructure::hash::fingerprint_file(&target)?;
    if sqlite::fingerprint_is_duplicate(&conn, &fingerprint.sha256, fingerprint.size, None)? {
        return Err(AwcError::DuplicateFingerprint(fingerprint.sha256));
    }
    let now = chrono_now();
    let artifact = Artifact {
        id: ArtifactId::new(),
        project_id: ProjectId(project_id),
        artifact_type: artifact_type.to_string(),
        title: title.to_string(),
        path: Some(target.clone()),
        original_path: Some(target.clone()),
        status: ArtifactStatus::Active,
        sha256: Some(fingerprint.sha256),
        size: Some(fingerprint.size),
        last_seen_at: now.clone(),
        created_at: now.clone(),
        updated_at: now,
    };
    sqlite::register_artifact(&mut conn, &artifact)?;
    Ok(CommandResult::ArtifactShown(artifact))
}

/// Applies an adopt plan per action: revalidates the workspace fingerprint
/// (stale → `StaleAdoptPlan`, zero actions), then for each action re-checks
/// its preconditions immediately before executing. Register actions call
/// `register_existing_artifact`; move-to-inbox actions move the file with
/// compensation. Reports applied/skipped; a failure does not block remaining
/// actions. Uses the workspace's single project by default.
pub fn apply_adopt(start: &Path, plan_id: &str) -> Result<CommandResult, AwcError> {
    apply_adopt_with_project(start, plan_id, None)
}

/// Like [`apply_adopt`] but with an explicit target project for registered
/// artifacts.
pub fn apply_adopt_with_project(
    start: &Path,
    plan_id: &str,
    project_id_or_prefix: Option<&str>,
) -> Result<CommandResult, AwcError> {
    let (root, state_dir) = paths::discover_with_root(start)?;
    let plan = adopt::load_plan(&state_dir, plan_id)?;
    let fresh = adopt::workspace_fingerprint(&root)?;
    if fresh.digest != plan.fingerprint.digest {
        return Err(AwcError::StaleAdoptPlan(
            "workspace changed since the plan was created".into(),
        ));
    }
    let project = match project_id_or_prefix {
        Some(pid) => pid.to_string(),
        None => default_project(start)?,
    };
    let mut applied = 0;
    let mut skipped = 0;
    for action in &plan.actions {
        match action {
            PlanAction::Register {
                rel_path,
                artifact_type,
            } => {
                let source = root.join(rel_path);
                if !source.exists() {
                    skipped += 1;
                    continue;
                }
                // Move the candidate under artifacts/ (the only writable
                // lifecycle target), then register; on registration failure
                // move the file back (compensation).
                let basename = source
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let target_rel = format!("artifacts/{basename}");
                let target = match paths::validate_artifact_target(&root, &target_rel) {
                    Ok(t) if !t.exists() => t,
                    _ => {
                        skipped += 1;
                        continue;
                    }
                };
                let fs = OsFs;
                if fs.move_file(&source, &target).is_err() {
                    skipped += 1;
                    continue;
                }
                match register_existing_artifact(
                    start,
                    &project,
                    &target_rel,
                    artifact_type,
                    rel_path,
                ) {
                    Ok(_) => applied += 1,
                    Err(_) => {
                        let _ = fs.move_file(&target, &source);
                        skipped += 1;
                    }
                }
            }
            PlanAction::MoveToInbox { rel_path } => {
                let source = root.join(rel_path);
                if !source.exists() {
                    skipped += 1;
                    continue;
                }
                let inbox = paths::ensure_governed_dir(&root, "inbox")?;
                let target = inbox.join(
                    source
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                );
                if target.exists() {
                    skipped += 1;
                    continue;
                }
                let fs = OsFs;
                match fs.move_file(&source, &target) {
                    Ok(()) => applied += 1,
                    Err(_) => skipped += 1,
                }
            }
            PlanAction::Skip { .. } => skipped += 1,
        }
    }
    Ok(CommandResult::AdoptApplied {
        plan_id: plan_id.to_string(),
        applied,
        skipped,
    })
}

/// Resolves the single project of the workspace for adopt registration, or
/// fails with a clear usage error when none or several exist.
fn default_project(start: &Path) -> Result<String, AwcError> {
    let (_, state_dir) = paths::discover_with_root(start)?;
    let config = config::load_readonly(&state_dir)?;
    let conn = sqlite::open_readonly(&state_dir.join(&config.database_file))?;
    let projects = sqlite::select_projects_by_id_prefix(&conn, "")?;
    match projects.len() {
        1 => Ok(projects[0].id.0.to_string()),
        0 => Err(AwcError::Usage(
            "adopt apply requires a project; run `awctl project add` first".into(),
        )),
        _ => Err(AwcError::Usage(
            "adopt apply requires exactly one project, or pass --project".into(),
        )),
    }
}

/// Resolves exactly one artifact by ID prefix against the persistence layer.
fn resolve_one_artifact(
    conn: &rusqlite::Connection,
    id_or_prefix: &str,
) -> Result<Artifact, AwcError> {
    let candidates = sqlite::select_artifacts_by_id_prefix(conn, id_or_prefix)?;
    let ids: Vec<Uuid> = candidates.iter().map(|a| a.id.0).collect();
    let id = resolve_artifact_id_prefix(id_or_prefix, &ids)?;
    candidates
        .into_iter()
        .find(|a| a.id.0 == id)
        .ok_or(AwcError::ArtifactNotFound)
}

/// Current UTC timestamp in the same text form the schema uses.
fn chrono_now() -> String {
    // Reuse the database clock contract: ISO-8601 UTC without external deps.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // SQLite datetime('now') format approximation: YYYY-MM-DD HH:MM:SS UTC.
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}",
        hour = rem / 3600,
        minute = (rem % 3600) / 60,
        second = rem % 60,
    )
}

/// Converts days since epoch to a civil (year, month, day) date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + if m <= 2 { 1 } else { 0 }, m as u32, d as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{CONFIG_SCHEMA_VERSION, Project};
    use crate::infrastructure::config::CONFIG_FILE_NAME;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("awc-core-app-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn init_then_status_and_doctor_from_nested_dir() {
        let root = temp_dir("nested");
        let CommandResult::Init(init) = init(&root).expect("init") else {
            panic!("expected Init result");
        };
        let canonical = fs::canonicalize(&root).unwrap();
        assert_eq!(init.root, canonical);
        assert_eq!(init.schema_version, CONFIG_SCHEMA_VERSION);
        assert!(init.database_ok && init.schema_ok);
        let state = root.join(WORKSPACE_DIR_NAME);
        assert!(state.join(CONFIG_FILE_NAME).exists() && state.join("state.sqlite3").exists());

        let nested = root.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        let CommandResult::Status(status) = status(&nested).expect("status") else {
            panic!("expected Status result");
        };
        assert_eq!(status.root, canonical);
        assert!(status.database_ok && status.schema_ok);

        let CommandResult::Doctor(doctor) = doctor_quick(&nested).expect("doctor") else {
            panic!("expected Doctor result");
        };
        assert_eq!(doctor.checks.iter().filter(|c| c.ok).count(), 4);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn read_only_commands_error_without_creating_awc() {
        let dir = temp_dir("nows");
        fs::create_dir_all(dir.join("sub")).unwrap();
        let err = status(&dir.join("sub")).unwrap_err();
        assert!(matches!(err, AwcError::WorkspaceNotFound));
        let err = doctor_quick(&dir.join("sub")).unwrap_err();
        assert!(matches!(err, AwcError::WorkspaceNotFound));
        assert!(
            !dir.join(WORKSPACE_DIR_NAME).exists(),
            "read-only commands must never create .awc"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reinit_repairs_missing_db_and_preserves_config_bytes() {
        let root = temp_dir("repair");
        init(&root).expect("first init");
        let state = fs::canonicalize(root.join(WORKSPACE_DIR_NAME)).unwrap();
        let config_path = state.join(CONFIG_FILE_NAME);
        let bytes = fs::read(&config_path).unwrap();
        fs::remove_file(state.join("state.sqlite3")).unwrap();

        let CommandResult::Init(init) = init(&root).expect("second init") else {
            panic!("expected Init result");
        };
        assert!(init.database_ok && init.schema_ok);
        assert!(state.join("state.sqlite3").exists());
        assert_eq!(fs::read(&config_path).unwrap(), bytes);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn read_only_commands_preserve_config_bytes_and_metadata() {
        let root = temp_dir("meta");
        init(&root).expect("init");
        let state = fs::canonicalize(root.join(WORKSPACE_DIR_NAME)).unwrap();
        let config_path = state.join(CONFIG_FILE_NAME);
        let db_path = state.join("state.sqlite3");
        let config_before = fs::read(&config_path).unwrap();
        let db_before = fs::read(&db_path).unwrap();
        let config_meta = fs::metadata(&config_path).unwrap().modified().unwrap();

        status(&root).expect("status");
        doctor_quick(&root).expect("doctor");

        assert_eq!(fs::read(&config_path).unwrap(), config_before);
        assert_eq!(fs::read(&db_path).unwrap(), db_before);
        assert_eq!(
            fs::metadata(&config_path).unwrap().modified().unwrap(),
            config_meta
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn doctor_reports_unhealthy_db_without_creating_it() {
        let root = temp_dir("unhealthy");
        init(&root).expect("init");
        let state = fs::canonicalize(root.join(WORKSPACE_DIR_NAME)).unwrap();
        fs::remove_file(state.join("state.sqlite3")).unwrap();

        let CommandResult::Doctor(doctor) = doctor_quick(&root).expect("doctor") else {
            panic!("expected Doctor result");
        };
        let check = |name| doctor.checks.iter().find(|c| c.name == name).unwrap();
        assert!(check("path").ok && check("config").ok);
        assert!(!check("database").ok);
        assert!(!check("schema").ok);

        let CommandResult::Status(status) = status(&root).expect("status") else {
            panic!("expected Status result");
        };
        assert!(!status.database_ok && !status.schema_ok);
        assert!(
            !state.join("state.sqlite3").exists(),
            "read-only commands must not recreate a missing database"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn init_rejects_invalid_config_and_keeps_existing_state() {
        let root = temp_dir("invalid");
        let state = root.join(WORKSPACE_DIR_NAME);
        fs::create_dir_all(&state).unwrap();
        fs::write(state.join(CONFIG_FILE_NAME), b"schema_version = [bad").unwrap();
        let marker = state.join("keep.txt");
        fs::write(&marker, b"x").unwrap();

        assert!(matches!(
            init(&root).unwrap_err(),
            AwcError::InvalidConfig(_)
        ));
        assert!(marker.exists(), "pre-existing state must never be removed");
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(all(test, unix))]
    #[test]
    fn doctor_and_init_reject_escaping_state_symlink() {
        let root = temp_dir("escape");
        let outside = temp_dir("escape-target");
        let marker = outside.join("marker.txt");
        fs::write(&marker, b"untouched").unwrap();
        std::os::unix::fs::symlink(&outside, &root.join(WORKSPACE_DIR_NAME)).unwrap();

        let CommandResult::Doctor(doctor) = doctor_quick(&root).expect("doctor reports") else {
            panic!("expected Doctor result");
        };
        let path_check = doctor.checks.iter().find(|c| c.name == "path").unwrap();
        assert!(!path_check.ok);

        assert!(matches!(
            init(&root).unwrap_err(),
            AwcError::UnsafeStatePath
        ));
        assert_eq!(fs::read(&marker).unwrap(), b"untouched");
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    #[cfg(all(test, unix))]
    #[test]
    fn init_accepts_contained_state_symlink() {
        let root = temp_dir("init-contained-link");
        let target = root.join("state").join("awc");
        fs::create_dir_all(&target).unwrap();
        std::os::unix::fs::symlink(&target, &root.join(WORKSPACE_DIR_NAME)).unwrap();

        let CommandResult::Init(init) = init(&root).expect("init accepts a contained symlink")
        else {
            panic!("expected Init result");
        };
        let canonical = fs::canonicalize(&root).unwrap();
        assert_eq!(init.root, canonical);
        assert!(init.database_ok && init.schema_ok);
        // Config and database land at the canonical target, not beside the link.
        assert!(target.join(CONFIG_FILE_NAME).exists());
        assert!(target.join("state.sqlite3").exists());
        // The `.awc` entry itself stays a symlink: init wrote through the
        // validated canonical target, never replacing the link.
        let meta = fs::symlink_metadata(&root.join(WORKSPACE_DIR_NAME)).unwrap();
        assert!(meta.file_type().is_symlink());

        // Read-only commands agree on the symlinked state.
        let CommandResult::Status(status) = status(&root).expect("status") else {
            panic!("expected Status result");
        };
        assert!(status.database_ok && status.schema_ok);
        let CommandResult::Doctor(doctor) = doctor_quick(&root).expect("doctor") else {
            panic!("expected Doctor result");
        };
        assert_eq!(doctor.checks.iter().filter(|c| c.ok).count(), 4);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn init_creates_and_repairs_governed_dirs_preserving_config() {
        let root = temp_dir("gov-dirs");
        init(&root).expect("init");
        for name in ["artifacts", "inbox", "tmp", "trash"] {
            assert!(root.join(name).is_dir(), "missing governed dir {name}");
        }
        let state = fs::canonicalize(root.join(WORKSPACE_DIR_NAME)).unwrap();
        let config_bytes = fs::read(state.join(CONFIG_FILE_NAME)).unwrap();
        for name in ["artifacts", "tmp"] {
            fs::remove_dir_all(root.join(name)).unwrap();
        }

        init(&root).expect("re-init repairs governed dirs");
        for name in ["artifacts", "inbox", "tmp", "trash"] {
            assert!(root.join(name).is_dir(), "missing governed dir {name}");
        }
        assert_eq!(
            fs::read(state.join(CONFIG_FILE_NAME)).unwrap(),
            config_bytes,
            "valid config bytes must be unchanged"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[cfg(all(test, unix))]
    #[test]
    fn init_rejects_escaping_governed_symlink() {
        let root = temp_dir("gov-escape-init");
        let outside = temp_dir("gov-escape-init-target");
        let marker = outside.join("marker.txt");
        fs::write(&marker, b"untouched").unwrap();
        std::os::unix::fs::symlink(&outside, &root.join("artifacts")).unwrap();

        assert!(matches!(
            init(&root).unwrap_err(),
            AwcError::UnsafeStatePath
        ));
        assert_eq!(fs::read(&marker).unwrap(), b"untouched");
        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside).ok();
    }

    fn init_at(root: &Path) {
        init(root).expect("init");
    }

    fn add(root: &Path, name: &str) -> Project {
        match add_project(
            root,
            AddProject {
                name: name.into(),
                slug: None,
                root_path: None,
            },
        ) {
            Ok(CommandResult::ProjectAdded(p)) => p,
            _ => panic!("add_project failed for {name}"),
        }
    }

    #[test]
    fn add_project_derives_slug_persists_and_reports() {
        let root = temp_dir("proj-add");
        init_at(&root);
        let CommandResult::ProjectAdded(p) = add_project(
            &root,
            AddProject {
                name: "My Cool  Project!".into(),
                slug: None,
                root_path: None,
            },
        )
        .expect("add") else {
            panic!("expected ProjectAdded");
        };
        assert_eq!(p.slug, "my-cool-project");
        assert_eq!(p.name, "My Cool  Project!");
        assert_eq!(p.status, "active");
        assert_eq!(p.id.0.to_string().len(), 36);
        assert!(p.root_path.is_none());

        // An explicit slug bypasses derivation but follows the same rules;
        // the external root_path is metadata only and is never written.
        let CommandResult::ProjectAdded(p) = add_project(
            &root,
            AddProject {
                name: "Weird".into(),
                slug: Some("explicit-slug".into()),
                root_path: Some(PathBuf::from("/does/not/exist")),
            },
        )
        .expect("explicit slug") else {
            panic!("expected ProjectAdded");
        };
        assert_eq!(p.slug, "explicit-slug");
        assert_eq!(p.root_path, Some(PathBuf::from("/does/not/exist")));
        assert!(
            !p.root_path.unwrap().exists(),
            "root_path is metadata only; never written"
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn add_project_rejects_derived_and_explicit_slug_collisions() {
        let root = temp_dir("proj-collide");
        init_at(&root);
        add(&root, "Alpha");
        for input in [
            AddProject {
                name: "alpha".into(),
                slug: None,
                root_path: None,
            },
            AddProject {
                name: "Beta".into(),
                slug: Some("alpha".into()),
                root_path: None,
            },
        ] {
            let err = add_project(&root, input).expect_err("collision must fail");
            assert!(matches!(err, AwcError::SlugConflict(_)));
        }
        let CommandResult::ProjectList(list) = list_projects(&root).expect("list") else {
            panic!("expected ProjectList");
        };
        assert_eq!(list.len(), 1, "no insert on collision");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn show_project_resolves_prefixes_and_list_is_deterministic() {
        let root = temp_dir("proj-show");
        init_at(&root);
        let a = add(&root, "Zulu");
        let b = add(&root, "Alpha");
        let id_a = a.id.0.to_string();
        let id_b = b.id.0.to_string();
        // A prefix matching exactly one project: the maximal common prefix
        // plus the next character of `a` (which differs from `b`'s).
        let common: String = id_a
            .chars()
            .zip(id_b.chars())
            .take_while(|(x, y)| x == y)
            .map(|(x, _)| x)
            .collect();
        let unique_a = format!("{}{}", common, &id_a[common.len()..common.len() + 1]);

        let CommandResult::ProjectShown(p) = show_project(&root, &id_a).expect("full id") else {
            panic!("expected ProjectShown");
        };
        assert_eq!(p.id, a.id);
        let CommandResult::ProjectShown(p) = show_project(&root, &unique_a).expect("unique prefix")
        else {
            panic!("expected ProjectShown");
        };
        assert_eq!(p.id, a.id);

        assert!(matches!(
            show_project(&root, "ffffffff").expect_err("no match"),
            AwcError::ProjectNotFound
        ));
        assert!(matches!(
            show_project(&root, &common).expect_err("two matches"),
            AwcError::AmbiguousProjectId
        ));

        let CommandResult::ProjectList(list) = list_projects(&root).expect("list") else {
            panic!("expected ProjectList");
        };
        let slugs: Vec<&str> = list.iter().map(|p| p.slug.as_str()).collect();
        assert_eq!(slugs, ["alpha", "zulu"], "deterministic slug order");
        fs::remove_dir_all(&root).ok();
    }

    fn seeded_workspace(name: &str) -> (std::path::PathBuf, String) {
        let root = temp_dir(name);
        init(&root).expect("init");
        let CommandResult::ProjectAdded(p) = add_project(
            &root,
            AddProject {
                name: "Demo".into(),
                slug: None,
                root_path: None,
            },
        )
        .expect("add project") else {
            panic!("expected ProjectAdded");
        };
        (root, p.id.0.to_string())
    }

    #[test]
    fn create_artifact_makes_file_and_metadata() {
        let (root, project_id) = seeded_workspace("use-create");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "My doc").expect("create")
        else {
            panic!("expected ArtifactCreated");
        };
        assert_eq!(a.status, ArtifactStatus::Active);
        assert!(a.path.as_ref().unwrap().exists(), "file must exist");
        assert!(a.path.as_ref().unwrap().starts_with(root.join("artifacts")));
        assert_eq!(a.size, Some(0), "created file is empty");
        let CommandResult::ArtifactShown(found) =
            show_artifact(&root, &a.id.0.to_string()).expect("show")
        else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(found.id, a.id);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn create_artifact_rejects_unknown_project_and_allows_empty_duplicates() {
        let (root, project_id) = seeded_workspace("use-create-reject");
        assert!(matches!(
            create_artifact(&root, "ffffffff", "doc", "x").expect_err("no project"),
            AwcError::ProjectNotFound
        ));
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "one").expect("create one")
        else {
            panic!("expected ArtifactCreated");
        };
        // Empty artifacts share the empty fingerprint: multiple are allowed
        // (approved product decision), so a second create succeeds.
        let CommandResult::ArtifactCreated(b) =
            create_artifact(&root, &project_id, "doc", "two").expect("create two")
        else {
            panic!("expected ArtifactCreated");
        };
        assert_ne!(a.id, b.id);
        assert!(a.path.as_ref().unwrap().exists());
        assert!(b.path.as_ref().unwrap().exists());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn archive_trash_restore_cycle_moves_files_and_statuses() {
        let (root, project_id) = seeded_workspace("use-cycle");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "cycle").expect("create")
        else {
            panic!("expected ArtifactCreated");
        };
        let id = a.id.0.to_string();
        let original = a.path.unwrap().clone();

        let CommandResult::ArtifactShown(trashed) = trash_artifact(&root, &id).expect("trash")
        else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(trashed.status, ArtifactStatus::Trashed);
        assert!(
            trashed
                .path
                .as_ref()
                .unwrap()
                .starts_with(root.join("trash"))
        );
        assert!(!original.exists(), "file moved out");

        let CommandResult::ArtifactShown(restored) = restore_artifact(&root, &id).expect("restore")
        else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(restored.status, ArtifactStatus::Active);
        assert_eq!(restored.path, Some(original.clone()));
        assert!(original.exists(), "file moved back");

        // Archive is status-only: path unchanged, file still present.
        let CommandResult::ArtifactShown(archived) = archive_artifact(&root, &id).expect("archive")
        else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(archived.status, ArtifactStatus::Archived);
        assert_eq!(
            archived.path,
            Some(original.clone()),
            "archive is status-only"
        );
        assert!(original.exists(), "archive never moves the file");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn trash_moves_to_collision_safe_name_and_restore_conflicts() {
        let (root, project_id) = seeded_workspace("use-trash-name");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "a").expect("create")
        else {
            panic!("expected ArtifactCreated");
        };
        let id = a.id.0.to_string();
        let original = a.path.unwrap().clone();
        trash_artifact(&root, &id).expect("trash");
        // Occupy the original path with another file, then restore must fail.
        fs::write(&original, b"new owner").expect("occupy");
        assert!(matches!(
            restore_artifact(&root, &id).expect_err("restore conflict"),
            AwcError::RestoreConflict(_)
        ));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn relink_requires_absent_old_file_and_refreshes_fingerprint() {
        let (root, project_id) = seeded_workspace("use-relink");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "relink").expect("create")
        else {
            panic!("expected ArtifactCreated");
        };
        let id = a.id.0.to_string();
        // Old file still exists -> relink refused.
        assert!(matches!(
            relink_artifact(&root, &id, "artifacts/new.txt").expect_err("old exists"),
            AwcError::RestoreConflict(_)
        ));
        // Remove old file and create the new target, then relink.
        let old = a.path.unwrap();
        fs::remove_file(&old).expect("remove old");
        let target = root.join("artifacts").join("new.txt");
        fs::write(&target, b"new content").expect("write new");
        let CommandResult::ArtifactShown(relinked) =
            relink_artifact(&root, &id, "artifacts/new.txt").expect("relink")
        else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(relinked.path, Some(target.clone()));
        assert_eq!(relinked.size, Some(11), "new content size");
        assert!(relinked.sha256.is_some());
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn list_artifacts_filters_by_status_and_orders_newest_first() {
        let (root, project_id) = seeded_workspace("use-list");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "first").expect("create a")
        else {
            panic!("expected ArtifactCreated");
        };
        let CommandResult::ArtifactCreated(b) =
            create_artifact(&root, &project_id, "doc", "second").expect("create b")
        else {
            panic!("expected ArtifactCreated");
        };
        let CommandResult::ArtifactList(all) =
            list_artifacts(&root, None, None, None).expect("all")
        else {
            panic!("expected ArtifactList");
        };
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, b.id, "newest first");
        archive_artifact(&root, &a.id.0.to_string()).expect("archive a");
        let CommandResult::ArtifactList(archived) =
            list_artifacts(&root, None, None, Some(ArtifactStatus::Archived)).expect("archived")
        else {
            panic!("expected ArtifactList");
        };
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].id, a.id);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn illegal_transition_from_archived_to_trashed_is_rejected() {
        let (root, project_id) = seeded_workspace("use-illegal");
        let CommandResult::ArtifactCreated(a) =
            create_artifact(&root, &project_id, "doc", "x").expect("create")
        else {
            panic!("expected ArtifactCreated");
        };
        let id = a.id.0.to_string();
        archive_artifact(&root, &id).expect("archive");
        assert!(matches!(
            trash_artifact(&root, &id).expect_err("archived->trashed illegal"),
            AwcError::ArtifactStatusConflict(_, _)
        ));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adopt_scan_classifies_workspace_read_only() {
        let root = temp_dir("adopt-scan");
        init(&root).expect("init");
        // Brownfield files outside governed dirs.
        fs::write(root.join("adopt-plan.md"), b"# Plan").expect("plan");
        fs::write(root.join("review-pr-13.md"), b"# Review").expect("review");
        fs::write(root.join("q3-report.md"), b"# Report").expect("report");
        fs::write(root.join("AGENTS.md"), b"# Agent").expect("agents");
        fs::write(root.join(".env"), b"SECRET=1").expect("env");
        fs::write(root.join("backup.tmp"), b"tmp").expect("tmp");
        fs::write(root.join("notes.md"), b"notes").expect("notes");
        fs::create_dir_all(root.join("node_modules")).expect("nm");
        fs::write(root.join("node_modules").join("x.js"), b"x").expect("nm file");

        let CommandResult::AdoptScan(candidates) = scan_adopt(&root).expect("scan") else {
            panic!("expected AdoptScan");
        };
        let by_path: std::collections::HashMap<&str, &ScanCategory> = candidates
            .iter()
            .map(|c| (c.rel_path.as_str(), &c.category))
            .collect();
        assert_eq!(
            by_path.get("adopt-plan.md"),
            Some(&&ScanCategory::ManagedCandidate)
        );
        assert_eq!(
            by_path.get("review-pr-13.md"),
            Some(&&ScanCategory::ManagedCandidate)
        );
        assert_eq!(
            by_path.get("q3-report.md"),
            Some(&&ScanCategory::ManagedCandidate)
        );
        assert_eq!(by_path.get("AGENTS.md"), Some(&&ScanCategory::KnownRuntime));
        assert_eq!(
            by_path.get(".env"),
            Some(&&ScanCategory::SensitiveCandidate)
        );
        assert_eq!(
            by_path.get("backup.tmp"),
            Some(&&ScanCategory::TemporaryCandidate)
        );
        assert_eq!(by_path.get("notes.md"), Some(&&ScanCategory::Unknown));
        assert!(
            !by_path.contains_key("node_modules/x.js"),
            "ignored trees are excluded"
        );
        // Zero mutation: every file still exists with original bytes.
        assert_eq!(fs::read(root.join("AGENTS.md")).unwrap(), b"# Agent");
        assert_eq!(fs::read(root.join(".env")).unwrap(), b"SECRET=1");
        assert_eq!(fs::read(root.join("notes.md")).unwrap(), b"notes");
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adopt_plan_persists_explicit_actions_and_fingerprint() {
        let root = temp_dir("adopt-plan");
        init(&root).expect("init");
        fs::write(root.join("adopt-plan.md"), b"# Plan").expect("plan");
        fs::write(root.join("review-pr-1.md"), b"# Review").expect("review");
        fs::write(root.join("notes.md"), b"notes").expect("notes");
        fs::write(root.join(".env"), b"SECRET=1").expect("env");

        let CommandResult::AdoptPlanCreated { plan_id, actions } = plan_adopt(&root).expect("plan")
        else {
            panic!("expected AdoptPlanCreated");
        };
        assert_eq!(actions, 4, "register x2 + move-to-inbox x1 + skip x1");

        let (_, state_dir) = paths::discover_with_root(&root).expect("discover");
        let plan = adopt::load_plan(&state_dir, &plan_id).expect("load");
        assert_eq!(plan.actions.len(), 4);
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            PlanAction::Register { rel_path, artifact_type }
                if rel_path == "adopt-plan.md" && artifact_type == "plan"
        )));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            PlanAction::Register { rel_path, artifact_type }
                if rel_path == "review-pr-1.md" && artifact_type == "code_review"
        )));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            PlanAction::Skip { rel_path } if rel_path == ".env"
        )));
        assert!(plan.actions.iter().any(|a| matches!(
            a,
            PlanAction::MoveToInbox { rel_path } if rel_path == "notes.md"
        )));
        // Fingerprint matches a fresh computation.
        let fresh = adopt::workspace_fingerprint(&root).expect("fingerprint");
        assert_eq!(plan.fingerprint, fresh);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn register_existing_artifact_registers_current_bytes() {
        let (root, project_id) = seeded_workspace("adopt-register");
        let target = root.join("artifacts").join("existing.md");
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::write(&target, b"hello adopt").expect("write");
        let CommandResult::ArtifactShown(a) = register_existing_artifact(
            &root,
            &project_id,
            "artifacts/existing.md",
            "plan",
            "existing",
        )
        .expect("register") else {
            panic!("expected ArtifactShown");
        };
        assert_eq!(a.size, Some(11));
        assert_eq!(a.status, ArtifactStatus::Active);
        assert!(a.path.as_ref().unwrap().exists());
        // Duplicate registration of the same path fails.
        assert!(matches!(
            register_existing_artifact(&root, &project_id, "artifacts/existing.md", "plan", "x")
                .expect_err("owned"),
            AwcError::PathOwned(_)
        ));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adopt_apply_rejects_stale_plan_without_actions() {
        let root = temp_dir("adopt-stale");
        init(&root).expect("init");
        fs::write(root.join("adopt-plan.md"), b"# Plan").expect("plan");
        let CommandResult::AdoptPlanCreated { plan_id, .. } = plan_adopt(&root).expect("plan")
        else {
            panic!("expected plan");
        };
        // Mutate the workspace after planning.
        fs::write(root.join("adopt-plan.md"), b"# CHANGED").expect("change");
        assert!(matches!(
            apply_adopt(&root, &plan_id).expect_err("stale"),
            AwcError::StaleAdoptPlan(_)
        ));
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adopt_apply_skips_missing_source_and_continues() {
        let root = temp_dir("adopt-skip");
        init(&root).expect("init");
        let CommandResult::ProjectAdded(p) = add_project(
            &root,
            AddProject {
                name: "D".to_string(),
                slug: None,
                root_path: None,
            },
        )
        .expect("proj") else {
            panic!("expected project");
        };
        fs::write(root.join("review-a.md"), b"# A").expect("a");
        fs::write(root.join("review-b.md"), b"# B").expect("b");
        let CommandResult::AdoptPlanCreated { plan_id, .. } = plan_adopt(&root).expect("plan")
        else {
            panic!("expected plan");
        };
        // Occupy artifacts/review-a.md WITHOUT touching the source (the
        // fingerprint excludes artifacts/, so the plan stays valid): the
        // apply action for review-a.md must skip (target occupied) while
        // review-b.md still applies.
        fs::create_dir_all(root.join("artifacts")).unwrap();
        fs::write(root.join("artifacts").join("review-a.md"), b"occupied").unwrap();
        let CommandResult::AdoptApplied {
            applied, skipped, ..
        } = apply_adopt_with_project(&root, &plan_id, Some(&p.id.0.to_string())).expect("apply")
        else {
            panic!("expected AdoptApplied");
        };
        assert_eq!(applied, 1, "review-b registers");
        assert_eq!(skipped, 1, "review-a target occupied is skipped");
        // review-b is now registered: its file moved to artifacts/.
        assert!(!root.join("review-b.md").exists(), "moved into artifacts");
        assert!(root.join("artifacts").join("review-b.md").exists());
        let (_, state_dir) = paths::discover_with_root(&root).expect("discover");
        let config = config::load_readonly(&state_dir).unwrap();
        let conn = sqlite::open_readonly(&state_dir.join(&config.database_file)).unwrap();
        let artifacts = sqlite::list_artifacts(&conn, None, None, None).expect("list");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(
            artifacts[0].path.as_deref(),
            Some(root.join("artifacts").join("review-b.md").as_path())
        );
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn adopt_apply_moves_unknown_to_inbox() {
        let root = temp_dir("adopt-inbox");
        init(&root).expect("init");
        add_project(
            &root,
            AddProject {
                name: "D".to_string(),
                slug: None,
                root_path: None,
            },
        )
        .expect("proj");
        fs::write(root.join("notes.md"), b"notes").expect("notes");
        let CommandResult::AdoptPlanCreated { plan_id, .. } = plan_adopt(&root).expect("plan")
        else {
            panic!("expected plan");
        };
        let CommandResult::AdoptApplied { applied, .. } =
            apply_adopt(&root, &plan_id).expect("apply")
        else {
            panic!("expected apply");
        };
        assert_eq!(applied, 1);
        assert!(!root.join("notes.md").exists(), "moved out");
        assert!(root.join("inbox").join("notes.md").exists(), "in inbox");
        fs::remove_dir_all(&root).ok();
    }
}
