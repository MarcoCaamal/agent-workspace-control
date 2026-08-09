//! AWC core library: local workspace state services.
//!
//! - [`domain`] — workspace configuration and command result types
//! - [`error`] — the shared `AwcError` with its exit-code contract
//! - [`infrastructure::config`] — versioned TOML config parsing and writing
//! - [`infrastructure::paths`] — upward discovery with symlink containment
//! - [`infrastructure::sqlite`] — transactional migrations with a version ledger
//!
//! Application use cases land in later work units.

pub mod domain;
pub mod error;
pub mod infrastructure;

pub use domain::{CheckResult, CommandResult, Config, InitStatus, Status, Workspace};
pub use error::AwcError;

/// Returns the crate name; lets the `awctl` stub prove the path dependency.
pub fn crate_name() -> &'static str {
    "awc-core"
}
