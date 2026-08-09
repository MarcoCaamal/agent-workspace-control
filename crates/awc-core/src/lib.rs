//! AWC core library: local workspace state services.
//!
//! Domain, application, and infrastructure modules land in later work units
//! (config/domain in PR 2, discovery in PR 3, SQLite in PR 4). This stub
//! establishes the package identity so the workspace graph builds end to end.

/// Returns the crate name; lets the `awctl` stub prove the path dependency.
pub fn crate_name() -> &'static str {
    "awc-core"
}
