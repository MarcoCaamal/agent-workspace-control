//! Application use cases: `init`, and read-only `status` / `doctor_quick`.

use std::fs;
use std::path::Path;

use crate::domain::{CheckResult, CommandResult, InitStatus, QuickDoctor, Status};
use crate::error::AwcError;
use crate::infrastructure::config;
use crate::infrastructure::paths::{self, WORKSPACE_DIR_NAME};
use crate::infrastructure::sqlite;

/// Initializes the workspace at `start`: canonical root/state safety, create
/// `.awc`, atomic default config only when absent (valid bytes preserved),
/// then open and migrate the database. Failure before the config commit
/// removes only an empty `.awc` created by this invocation; database
/// failures after it propagate untouched (later `init` resumes recovery).
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::CONFIG_SCHEMA_VERSION;
    use crate::infrastructure::config::CONFIG_FILE_NAME;

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
}
