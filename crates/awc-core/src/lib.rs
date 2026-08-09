//! AWC core library: local workspace state services.
//!
//! - [`domain`] — workspace configuration and command result types
//! - [`error`] — the shared `AwcError` with its exit-code contract
//! - [`infrastructure::config`] — versioned TOML config parsing and writing
//! - [`infrastructure::paths`] — upward discovery with symlink containment
//! - [`infrastructure::sqlite`] — transactional migrations with a version ledger
//! - [`application`] — `init`, `status`, and `doctor_quick` use cases

pub mod application;
pub mod domain;
pub mod error;
pub mod infrastructure;

pub use domain::{
    ArtifactId, AuditEventId, CheckResult, CommandResult, Config, ContentFingerprint, InitStatus,
    ProjectId, QuickDoctor, Status, Workspace,
};
pub use error::AwcError;

/// Returns the crate name; lets the `awctl` stub prove the path dependency.
pub fn crate_name() -> &'static str {
    "awc-core"
}
