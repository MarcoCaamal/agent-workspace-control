# Apply Progress: AWC Foundation — Slice 1 (PR 1)

- **Change**: awc-foundation
- **Work unit**: `slice-1-workspace-bootstrap` (runtime attempt ordinal 1)
- **Mode**: Standard (strict_tdd: false — pre-bootstrap project, no test runner yet)
- **Delivery**: chained delivery (user-resolved) → feature-branch-chain; branch `feature/awc-foundation`; approved slice = work unit 1 only
- **Date**: 2026-08-08

## Tasks Completed (1.1–1.5)

| Task | Description | Status |
|------|-------------|--------|
| 1.1 | Root `Cargo.toml` workspace, members `crates/awc-core`, `crates/awctl` | [x] |
| 1.2 | `crates/awc-core/Cargo.toml`: rusqlite (bundled), serde, toml; no clap/Tokio | [x] |
| 1.3 | `crates/awctl/Cargo.toml`: clap, serde_json, awc-core path dep | [x] |
| 1.4 | `.gitignore` (`/target`, `.awc/`); compiling `lib.rs`/`main.rs` stubs | [x] |
| 1.5 | `cargo check --workspace` (pins generated `Cargo.lock`) | [x] |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `Cargo.toml` | Created | Workspace manifest: members `crates/awc-core`, `crates/awctl`; resolver 3; workspace package defaults (version 0.1.0, edition 2024, MIT) |
| `Cargo.lock` | Created (generated) | Pinned crate graph from `cargo check --workspace`; 13,096 bytes |
| `crates/awc-core/Cargo.toml` | Created | Deps: rusqlite 0.32 (bundled), serde 1 (derive), toml 0.8 — no clap, no Tokio |
| `crates/awc-core/src/lib.rs` | Created | Minimal compiling library stub with `crate_name()` proving package identity |
| `crates/awctl/Cargo.toml` | Created | Deps: awc-core (path), clap 4 (derive), serde_json 1 |
| `crates/awctl/src/main.rs` | Created | Minimal compiling binary stub printing `awctl stub: linked awc-core` |
| `.gitignore` | Created | `/target`, `.awc/` |
| `openspec/changes/awc-foundation/tasks.md` | Modified | Checkboxes 1.1–1.5 → `[x]` |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | `cargo check --workspace` → exit 0, `Finished dev profile in 12.43s`; checked awc-core v0.1.0 and awctl v0.1.0 (plus deps: rusqlite 0.32.1, clap 4.6.6, toml 0.8.23, serde 1.0.229) |
| Runtime harness command/scenario and exact result | `cargo build --workspace` → exit 0, `Finished dev profile in 5.26s`; then `./target/debug/awctl` → stdout `awctl stub: linked awc-core` (proves path dep links and binary runs) |
| Rollback boundary | Remove only new files: `Cargo.toml`, `Cargo.lock`, `crates/`, `.gitignore`; revert `tasks.md` checkboxes. No pre-existing project files were modified |

Threat matrix: all rows N/A (design), no RED tests required for this slice.

## Changed-Line Estimate

- Authored lines: **51** (Cargo.toml 8, awc-core manifest 11, lib.rs 10, awctl manifest 11, main.rs 9, .gitignore 2)
- Generated: Cargo.lock (excluded from authored count per 400-line rule)
- Budget: 400 → risk: **Low** for this slice

## Commands Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo check --workspace` | Exit 0; both crates checked; Cargo.lock pinned |
| `cargo build --workspace` | Exit 0; debug binaries built |
| `./target/debug/awctl` | Exit 0; printed `awctl stub: linked awc-core` |

## Detected Test/Build Commands (post-bootstrap, for later refresh)

- Test: `cargo test --workspace` (unit/integration per crate: `cargo test -p awc-core`, `cargo test -p awctl`)
- Lint: `cargo clippy --workspace -D warnings`
- Format: `cargo fmt --check`
- Build: `cargo build --workspace`
- NOTE: `openspec/config.yaml` testing block NOT rewritten — that is task 6.1 (PR 7), out of slice scope.

## Remaining Work

- Tasks 2.1–6.2 pending (17 tasks). Next work unit: Unit 2 — Domain, errors, TOML config (PR 2; `cargo test -p awc-core config`).
- No commit/push/PR performed (lifecycle actions require parent receipt validation).

## Risks

- rusqlite 0.32.1 pins libsqlite3-sys 0.30.1 (bundled) — verified compiles on this host.
- Edition 2024 (resolver 3) — verified compatible with toolchain rustc/cargo 1.92.0.
- None blocking this slice.
