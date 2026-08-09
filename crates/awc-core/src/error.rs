//! Central error type for AWC core.

use std::fmt;
use std::io;

use crate::domain::CONFIG_SCHEMA_VERSION;

/// Errors produced by AWC core. Messages are user-safe: they never embed
/// file contents or secrets.
#[derive(Debug)]
pub enum AwcError {
    /// Invalid CLI usage; maps to exit code 2 (clap contract).
    Usage(String),
    /// No workspace found in the directory or any ancestor.
    WorkspaceNotFound,
    /// `.awc` resolves outside its workspace root.
    UnsafeStatePath,
    /// Config exists but does not parse or violates the schema.
    InvalidConfig(String),
    /// Config declares a schema version this build cannot handle.
    UnsupportedConfigVersion(u32),
    /// Underlying filesystem error.
    Io(io::Error),
    /// Underlying SQLite error.
    Database(rusqlite::Error),
}

impl AwcError {
    /// Exit code contract: success = 0; Usage = 2; WorkspaceNotFound = 3;
    /// all other errors = 1.
    pub fn exit_code(&self) -> i32 {
        match self {
            AwcError::Usage(_) => 2,
            AwcError::WorkspaceNotFound => 3,
            _ => 1,
        }
    }
}

impl fmt::Display for AwcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AwcError::Usage(msg) => write!(f, "{msg}"),
            AwcError::WorkspaceNotFound => {
                write!(
                    f,
                    "no AWC workspace found in this directory or any ancestor"
                )
            }
            AwcError::UnsafeStatePath => {
                write!(f, "unsafe state path: `.awc` escapes its workspace root")
            }
            AwcError::InvalidConfig(msg) => write!(f, "invalid workspace config: {msg}"),
            AwcError::UnsupportedConfigVersion(v) => write!(
                f,
                "unsupported config schema_version {v}; supported version is {CONFIG_SCHEMA_VERSION}"
            ),
            AwcError::Io(err) => write!(f, "I/O error: {err}"),
            AwcError::Database(err) => write!(f, "database error: {err}"),
        }
    }
}

impl std::error::Error for AwcError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AwcError::Io(err) => Some(err),
            AwcError::Database(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for AwcError {
    fn from(err: io::Error) -> Self {
        AwcError::Io(err)
    }
}

impl From<rusqlite::Error> for AwcError {
    fn from(err: rusqlite::Error) -> Self {
        AwcError::Database(err)
    }
}
