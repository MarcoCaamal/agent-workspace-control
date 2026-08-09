//! `awctl` — AWC command-line boundary.
//!
//! Command parsing, JSON/human renderers, and exit codes land in later work
//! units (CLI contracts, PR 6). This stub exists so the workspace builds and
//! proves the `awc-core` path dependency links.

fn main() {
    println!("awctl stub: linked {}", awc_core::crate_name());
}
