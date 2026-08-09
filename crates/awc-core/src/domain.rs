//! Domain types for AWC workspaces: configuration and command results.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Supported configuration schema version. Bump only with an explicit
/// compatibility and migration decision (design: Config).
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Default SQLite state file name, relative to the `.awc` directory.
pub const DEFAULT_DATABASE_FILE: &str = "state.sqlite3";

/// Versioned workspace configuration (`schema_version = 1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub database_file: String,
}

impl Config {
    /// Default configuration for a freshly initialized workspace.
    pub fn default_config() -> Self {
        Config {
            schema_version: CONFIG_SCHEMA_VERSION,
            database_file: DEFAULT_DATABASE_FILE.to_string(),
        }
    }
}

/// A discovered workspace: its root directory and parsed configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
    pub config: Config,
}

/// Outcome of one read-only diagnostic check (config, database, schema, path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub name: &'static str,
    pub ok: bool,
    /// Human-readable detail; empty when the check passed.
    pub message: String,
}

impl CheckResult {
    pub fn ok(name: &'static str) -> Self {
        CheckResult {
            name,
            ok: true,
            message: String::new(),
        }
    }

    pub fn failed(name: &'static str, message: impl Into<String>) -> Self {
        CheckResult {
            name,
            ok: false,
            message: message.into(),
        }
    }
}

/// Read-only workspace status report (design: status reports root, config
/// version, and database/schema health).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub root: PathBuf,
    pub schema_version: u32,
    pub database_ok: bool,
    pub schema_ok: bool,
}

/// Result of a successful `init` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitStatus {
    pub root: PathBuf,
    pub schema_version: u32,
    pub database_ok: bool,
    pub schema_ok: bool,
}

/// Quick doctor report: the reported root plus one check per diagnostic, in
/// the fixed order path, config, database, schema (design: doctor --quick).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickDoctor {
    pub root: PathBuf,
    pub checks: Vec<CheckResult>,
}

/// Typed outcome of a core command, rendered by `awctl` (Phase 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Init(InitStatus),
    Status(Status),
    Doctor(QuickDoctor),
}
