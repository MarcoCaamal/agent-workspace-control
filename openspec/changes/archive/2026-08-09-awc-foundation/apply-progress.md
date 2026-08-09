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

# Apply Progress: AWC Foundation — Slice 2 (PR 2)

- **Work unit**: `slice-2-core-config` (runtime attempt ordinal 2)
- **Mode**: Standard (strict_tdd: false); behavior-first RED → GREEN followed for this unit
- **Delivery**: chained (user-resolved) → feature-branch-chain; current child branch `feature/awc-foundation-02-config`
- **Date**: 2026-08-09

## Tasks Completed (2.1–2.4)

| Task | Description | Status |
|------|-------------|--------|
| 2.1 | RED: unit tests — invalid TOML, unknown `schema_version`, byte-preserving round trip | [x] |
| 2.2 | `error.rs`: `AwcError` variants per design contract | [x] |
| 2.3 | `domain.rs`: Workspace, Config, CheckResult, Status, CommandResult, InitStatus | [x] |
| 2.4 | `config.rs`: validate, preserve valid bytes, atomic write, reject unknown versions | [x] |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awc-core/src/error.rs` | Created | `AwcError` (7 variants per design), safe Display, `source()`, `From<io::Error>`/`From<rusqlite::Error>`, `exit_code()` (Usage=2, WorkspaceNotFound=3, others=1) |
| `crates/awc-core/src/domain.rs` | Created | `CONFIG_SCHEMA_VERSION=1`, `DEFAULT_DATABASE_FILE`, `Config` (serde, deny-free version gate), `Workspace`, `CheckResult`, `Status`, `InitStatus`, `CommandResult{Init,Status}` (Doctor variant deferred to Phase 4 with QuickDoctor) |
| `crates/awc-core/src/infrastructure/mod.rs` | Created | `pub mod config;` — paths/sqlite modules land in slices 3–4 |
| `crates/awc-core/src/infrastructure/config.rs` | Created | `CONFIG_FILE_NAME`, `parse_config` (UTF-8 + TOML → InvalidConfig; version gate → UnsupportedConfigVersion), `default_config_bytes`, `write_config_atomic` (tmp + fsync + rename, tmp cleaned on failure), `load_or_create` (preserves valid bytes; creates default atomically) + 6 RED-first tests |
| `crates/awc-core/src/lib.rs` | Modified | Module wiring `domain`/`error`/`infrastructure`; root re-exports; `crate_name()` kept for awctl stub |
| `openspec/changes/awc-foundation/tasks.md` | Modified | Checkboxes 2.1–2.4 → `[x]` |
| `openspec/changes/awc-foundation/apply-progress.md` | Modified | Slice 2 section appended (this file) |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awc-core config` → `FAILED. 0 passed; 6 failed` (todo!() stubs). GREEN: same command → `ok. 6 passed; 0 failed` covering invalid TOML, missing fields, schema_version 0/2/99 rejection, comment-bearing byte preservation, atomic byte round trip, default creation idempotence |
| Runtime harness command/scenario and exact result | `cargo test -p awc-core` → `ok. 6 passed; 0 failed` (full crate, incl. 0 doc tests). `cargo fmt --check` → clean after `cargo fmt`. No external runtime boundary in this slice (pure library units); temp-dir fixtures exercise the real filesystem path incl. atomic rename |
| Rollback boundary | Delete `crates/awc-core/src/{error.rs,domain.rs,infrastructure/}`; restore `lib.rs` to 10-line stub; revert `tasks.md` checkboxes 2.1–2.4 and the Slice 2 section of `apply-progress.md`. Cargo.lock untouched (no dependency changes) |

Threat matrix: all rows N/A (design) — no threat-matrix RED tests.

## Changed-Line Estimate

- Authored: **342 code** (domain.rs 90, error.rs 84, config.rs 143, mod.rs 5, lib.rs 16 changed) + **4** tasks.md + ~45 apply-progress.md ≈ **~390** total
- Generated: Cargo.lock — untouched this slice
- Budget: 400 → risk: **OK** (slice is under budget)

## Commands Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo test -p awc-core config` (RED, stubs) | Exit 1; `0 passed; 6 failed` — todo!() stubs prove tests fail first |
| `cargo test -p awc-core config` (GREEN) | Exit 0; `ok. 6 passed; 0 failed` |
| `cargo test -p awc-core` | Exit 0; `ok. 6 passed; 0 failed` |
| `cargo fmt` / `cargo fmt --check` | Format normalized; `--check` exit 0 |

## Remaining Work

- Tasks 2.5–6.2 pending (18 tasks). Next work unit: Unit 3 — Discovery + symlink containment (PR 3; `cargo test -p awc-core paths`).
- No commit/push/PR performed (lifecycle actions require parent receipt validation).

## Risks

- `CommandResult::Doctor(QuickDoctor)` deferred until Phase 4 (QuickDoctor not in task 2.3 type list) — design interface noted.
- `openspec/changes/awc-foundation/tasks.md` line 12 still says `Chain strategy: pending` while Engram records resolved `feature-branch-chain`; only checkboxes updated per dispatcher instruction.
- None blocking this slice.

# Apply Progress: AWC Foundation — Slice 3 (PR 3)

- **Work unit**: `slice-3-path-discovery` (runtime attempt ordinal 3)
- **Mode**: Standard (strict_tdd: false); behavior-first RED → GREEN followed for this unit
- **Delivery**: chained (user-resolved) → feature-branch-chain; current child branch `feature/awc-foundation-03-paths`
- **Date**: 2026-08-09

## Tasks Completed (2.5–2.6)

| Task | Description | Status |
|------|-------------|--------|
| 2.5 | RED: path tests — nearest ancestor, internal symlink accepted, escaping symlink rejected without target access | [x] |
| 2.6 | `paths.rs`: canonical start, nearest-first ancestor walk, canonical containment check | [x] |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awc-core/src/infrastructure/paths.rs` | Created | `WORKSPACE_DIR_NAME=".awc"`, `discover(start)`: canonicalize start, walk canonical ancestors nearest-first; `symlink_metadata` (no target read); canonicalize root+state; accept only `canonical_state.starts_with(canonical_root)` AND state is a dir; missing entry continues upward; escaping/broken/non-dir state fails (`UnsafeStatePath`) without target use; walk exhaustion → `WorkspaceNotFound` + 8 RED-first tests |
| `crates/awc-core/src/infrastructure/mod.rs` | Modified | `pub mod paths;` wiring (sqlite deferred note) |
| `crates/awc-core/src/lib.rs` | Modified | Doc: paths module listed as implemented; sqlite/application still later |
| `openspec/changes/awc-foundation/tasks.md` | Modified | Checkboxes 2.5–2.6 → `[x]` |
| `openspec/changes/awc-foundation/apply-progress.md` | Modified | Slice 3 section appended (this file) |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awc-core paths` → `FAILED. 0 passed; 8 failed` (todo!() stub). GREEN: same command → `ok. 8 passed; 0 failed` |
| Runtime harness command/scenario and exact result | `cargo test -p awc-core` → `ok. 14 passed; 0 failed` (8 paths + 6 config; 0 doc tests). `cargo fmt --check` → clean (exit 0). Real symlink fixtures under `std::env::temp_dir()` exercise actual kernel symlink resolution incl. escaping targets (marker byte-asserted untouched) |
| Rollback boundary | Delete `crates/awc-core/src/infrastructure/paths.rs`; revert `mod.rs`/`lib.rs` to slice-2 state; revert `tasks.md` checkboxes 2.5–2.6 and the Slice 3 section of `apply-progress.md`. Cargo.lock untouched (no dependency changes) |

Threat matrix: all rows N/A (design) — no threat-matrix RED tests.

## Invariant Coverage (8 tests)

- Nearest valid ancestor wins (`nearest_ancestor_wins`)
- Missing `.awc` continues upward (`continues_upward_when_ancestor_missing`)
- Internal symlink accepted, returns canonical target (`internal_symlink_returns_canonical_target`)
- Escaping symlink rejected without target use — marker file byte-asserted untouched (`escaping_symlink_rejected_without_target_use`)
- Existing escaping `.awc` fails instead of skipping to an outer valid workspace (`escaping_symlink_fails_instead_of_skipping_to_outer`)
- No workspace → `WorkspaceNotFound`, `.awc` never created (`no_workspace_returns_not_found_without_creating_state`)
- Canonical start: discovery via a symlinked parent resolves to the real workspace (`canonical_start_via_symlinked_parent`)
- Non-directory `.awc` file → `UnsafeStatePath` (`plain_file_named_awc_is_rejected`)

Note: non-directory `.awc` maps to `UnsafeStatePath` (same fail-don't-skip path as escaping); error variant's Display text is unchanged.

## Changed-Line Estimate

- Authored: **242** (paths.rs 175, mod.rs 3, lib.rs 5, tasks.md 4, apply-progress.md ≈55)
- Generated: Cargo.lock — untouched this slice
- Budget: 400 → risk: **OK** (slice is under budget; target ≤330 met)

## Commands Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo test -p awc-core paths` (RED, stub) | Exit 1; `0 passed; 8 failed` — todo!() stub proves tests fail first |
| `cargo test -p awc-core paths` (GREEN) | Exit 0; `ok. 8 passed; 0 failed` |
| `cargo test -p awc-core` | Exit 0; `ok. 14 passed; 0 failed` (6 config + 8 paths) |
| `cargo fmt --check` | Exit 0; clean (no fmt pass needed — files written formatted) |

## Remaining Work

- Tasks 3.1–6.2 pending (16 tasks). Next work unit: Unit 4 — SQLite migrations + repair (PR 4; `cargo test -p awc-core sqlite`).
- No commit/push/PR performed (lifecycle actions require parent receipt validation).

## Risks

- Tests gated `#[cfg(all(test, unix))]` — symlink fixtures need a Unix filesystem; Linux is Tier 1 per design, macOS also covered.
- `discover` returns the canonical state dir only; config parsing inside the workspace remains Phase 4's composition boundary (discovery never creates or reads config bytes).
- None blocking this slice.

# Apply Progress: AWC Foundation — Slice 4 (PR 4)

- **Work unit**: `slice-4-sqlite-migrations` (runtime attempt ordinal 4)
- **Mode**: Standard (strict_tdd: false); behavior-first RED → GREEN followed for this unit
- **Delivery**: chained (user-resolved) → feature-branch-chain; current child branch `feature/awc-foundation-04-sqlite`
- **Date**: 2026-08-09

## Tasks Completed (3.1–3.3)

| Task | Description | Status |
|------|-------------|--------|
| 3.1 | RED: migration tests — `schema_migrations(version)` ledger, `projects`/`artifacts`/`audit_events`, rerun idempotent | [x] |
| 3.2 | `sqlite.rs`: bundled rusqlite, transactional ordered migrations, schema health | [x] |
| 3.3 | Repair test: valid config + missing DB → state restored, config bytes unchanged | [x] |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awc-core/src/infrastructure/sqlite.rs` | Created | `MIGRATIONS_TABLE="schema_migrations"`, ordered `MIGRATIONS` (v1: projects/artifacts/audit_events with keys, timestamps, FKs), `open` (create-if-absent + `PRAGMA foreign_keys=ON`), `migrate` (ledger first, per-migration transactions, skip applied versions), `schema_health` (tables + ledger-authoritative version count) + 8 RED-first tests |
| `crates/awc-core/src/infrastructure/mod.rs` | Modified | `pub mod sqlite;` wiring; doc note removed |
| `crates/awc-core/src/lib.rs` | Modified | Doc: `infrastructure::sqlite` listed; only application use cases remain later |
| `openspec/changes/awc-foundation/tasks.md` | Modified | Checkboxes 3.1–3.3 → `[x]` |
| `openspec/changes/awc-foundation/apply-progress.md` | Modified | Slice 4 section appended (this file) |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awc-core sqlite` → `FAILED. 0 passed; 8 failed` (todo!() stubs). GREEN: same command → `ok. 8 passed; 0 failed` |
| Runtime harness command/scenario and exact result | `cargo test -p awc-core` → `ok. 22 passed; 0 failed` (6 config + 8 paths + 8 sqlite; 0 doc tests). `cargo fmt` then `cargo fmt --check` → clean (exit 0). Real SQLite files in `std::env::temp_dir()` temp dirs exercise actual open/create/migrate/FK enforcement incl. drop-DB repair rerun |
| Rollback boundary | Delete `crates/awc-core/src/infrastructure/sqlite.rs`; revert `mod.rs`/`lib.rs` wiring; revert `tasks.md` checkboxes 3.1–3.3 and the Slice 4 section of `apply-progress.md`. Cargo.lock untouched (no dependency changes) |

Threat matrix: all rows N/A (design) — no threat-matrix RED tests.

## Invariant Coverage (8 tests)

- Ledger + projects/artifacts/audit_events all created (`migrations_create_ledger_and_tables`)
- Every version recorded in order 1..N (`migrations_record_every_version_in_order`)
- Rerun idempotent: single ledger row, health still true (`migrate_rerun_is_idempotent`)
- Foreign keys enforced: orphan artifact insert fails with FOREIGN KEY error (`foreign_keys_are_enforced`)
- Health true after migrate (`schema_health_ok_after_migrate`); false on empty DB and on dropped table (`schema_health_false_when_state_missing`)
- Ledger authoritative: recorded version not re-applied after table drop (`ledger_is_authoritative_over_table_existence`)
- Repair: missing DB recreated+migrated, config bytes byte-identical before/after (`repair_recreates_missing_db_and_preserves_config_bytes`)

## Changed-Line Estimate

- Authored: **~311** (sqlite.rs 244, mod.rs 1, lib.rs 2, tasks.md 4, apply-progress.md ≈60)
- Generated: Cargo.lock — untouched this slice
- Budget: 400 → risk: **OK** (slice under budget; target ≤330 met)

## Commands Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo test -p awc-core sqlite` (RED, stubs) | Exit 1; `0 passed; 8 failed` — todo!() stub proves tests fail first |
| `cargo test -p awc-core sqlite` (GREEN) | Exit 0; `ok. 8 passed; 0 failed` |
| `cargo test -p awc-core` | Exit 0; `ok. 22 passed; 0 failed` (6 config + 8 paths + 8 sqlite) |
| `cargo fmt` / `cargo fmt --check` | Format normalized; `--check` exit 0 |

## Remaining Work

- Tasks 4.1–6.2 pending (13 tasks). Next work unit: Unit 5 — init/status/doctor_quick (PR 5; `cargo test -p awc-core`).
- No commit/push/PR performed (lifecycle actions require parent receipt validation).

## Risks

- `migrate` takes `&mut Connection` (`transaction()` over `unchecked_transaction()`); schema_health stays `&Connection` for read-only status/doctor use.
- Repair scope is module-level (open+migrate); full init composition (dir/config/db ordering, empty-dir removal) is Phase 4 task 4.1.
- `schema_health` treats a missing ledger row as unhealthy (ledger authoritative) — documented, tested.
- None blocking this slice.
# Apply Progress: AWC Foundation — Slice 5 (PR 5)

- **Work unit**: `slice-5-application-use-cases` (attempt ordinal 5); date 2026-08-09
- **Mode**: Standard (strict_tdd: false); behavior-first RED → GREEN
- **Delivery**: feature-branch-chain; child branch `feature/awc-foundation-05-application`

## Tasks Completed (4.1–4.3)

| Task | Status |
|------|--------|
| 4.1 `init`: canonical root/state safety, create `.awc`, atomic default config only when absent, open+migrate DB, remove only empty state dir created in this invocation on pre-config failure | [x] |
| 4.2 Read-only `status` + `doctor_quick` composing discovery/config/sqlite; checks path/config/database/schema; no repair | [x] |
| 4.3 RED integration: nested discovery, no-workspace without `.awc` creation, partial repair, unchanged config bytes/metadata, unhealthy missing-DB/unsafe-path | [x] |

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `application.rs` | Created | `init` (containment, `created_state_dir` guard + `remove_dir`, open+migrate), `status` (read-only open, unhealthy booleans), `doctor_quick` (unsafe path → failed path check only; check chain) + 7 RED-first tests |
| `domain.rs` | Modified | `QuickDoctor { root, checks }`; `CommandResult::Doctor(QuickDoctor)` |
| `config.rs`, `paths.rs`, `sqlite.rs`, `lib.rs` | Modified | `load_readonly` (parse, never create); `discover_with_root` (root+state); `open_readonly` (read-only open); module wiring + re-export |
| `tasks.md`, `apply-progress.md` | Modified | Checkboxes 4.1–4.3 `[x]`; Slice 5 section appended (slices 1–5) |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awc-core` → `FAILED. 22 passed; 8 failed` (todo!() stubs). GREEN: `cargo test -p awc-core application` → `ok. 7 passed; 0 failed`; full `cargo test -p awc-core` → `ok. 29 passed; 0 failed` |
| Runtime harness command/scenario and exact result | Temp-dir E2E: init → config.toml + state.sqlite3 migrated; status/doctor from nested dir report same canonical root; DB deleted → doctor reports database/schema failed AND file stays absent (read-only open cannot recreate); status `database_ok=false`; config bytes + mtime byte-identical after status/doctor; re-init repairs DB, config bytes unchanged; escaping symlink → doctor path check failed, init `UnsafeStatePath`, target untouched |
| Rollback boundary | Delete `application.rs`; revert domain/config/paths/sqlite/lib.rs + tasks.md checkboxes + Slice 5 section. Cargo.lock untouched. Threat matrix: all rows N/A (design) — no RED tests |

## Changed-Line Estimate

- Authored: **~398** (application.rs 299, domain 10, config 13, paths 8, sqlite 11, lib 6, tasks.md 6, apply-progress ≈45)
- Budget: 400 hard → **OK** (398); target ≤330 not met (~68 over) — all seven 4.3 evidence scenarios kept, none dropped; precedent slice 2 (~390) landed between target and hard budget

## Remaining Work

- Tasks 5.1–6.2 pending (5). Next: Unit 6 — awctl CLI (PR 6; `cargo test -p awctl`). No commit/push/PR performed (lifecycle actions require parent receipt validation).

## Risks

- `status` maps DB errors to `database_ok`/`schema_ok` booleans (design); `doctor_quick` carries detail strings. Missing/invalid config: hard `status` error, failed `doctor` check.
- Empty-dir cleanup branch (pre-config failure) verified by inspection — deterministic trigger needs privilege games; "never remove pre-existing state" IS behavior-tested (`init_rejects_invalid_config_and_keeps_existing_state`).
- Escaping-symlink tests `#[cfg(all(test, unix))]` (Linux Tier 1). None blocking.
# Apply Progress: AWC Foundation — Slice 6 (PR 6)

- **Work unit**: `slice-6-awctl-cli` (attempt ordinal 6); date 2026-08-09; Standard mode (strict_tdd: false), behavior-first RED → GREEN; feature-branch-chain, child branch `feature/awc-foundation-06-cli`

## Tasks Completed (5.1–5.3)
- [x] 5.1 RED CLI tests — exits 0/1/2/3; JSON envelope (`schemaVersion`+`ok`, exactly `data` xor `error`); errors on stderr; one newline-terminated doc
- [x] 5.2 `main.rs`: clap (init, status, doctor --quick required, global --json), human/typed-JSON renderers; dispatch to awc-core only
- [x] 5.3 One newline-terminated JSON doc; clap usage errors → stderr exit 2; core `exit_code()` mapping preserved (3 ws-not-found, 1 operational)

## Files Changed
| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awctl/tests/cli.rs` | Created | 6 RED-first contract tests via `CARGO_BIN_EXE_awctl`: envelope both polarities, exits 0/1/2/3, exactly-one-newline stdout, stderr discipline, `.awc` non-creation on ws-not-found, deterministic check order |
| `crates/awctl/src/main.rs` | Replaced stub | Synchronous clap boundary; `parts`/`ws`/`check_view` mappers; typed camelCase views (`WorkspaceView`, `CheckView`, `DataView`, `ErrorView`, `JsonDoc`); `error_code` per variant; human renderers; `render_error` (JSON doc stdout, human stderr) |
| `crates/awctl/Cargo.toml`, `Cargo.lock` | Modified / Generated | `serde` derive dep; `serde_json` dev-dep; Cargo.lock +1 (awctl gains `serde`, no new packages; generated, excluded from authored count) |
| `tasks.md`, `apply-progress.md` | Modified | Checkboxes 5.1–5.3 `[x]`; Slice 6 section appended (slices 1–6 cumulative) |

## Work Unit Evidence
| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awctl` → `FAILED. 0 passed; 6 failed` (stub fails all contract tests). GREEN: `cargo test -p awctl` → `ok. 6 passed; 0 failed` |
| Runtime harness command/scenario and exact result | Smoke: temp dir — `init --json` exit 0 single JSON doc stderr empty; `status --json`/`doctor --quick --json` from nested dir exit 0, checks ordered path/config/database/schema, `message` omitted when ok; `status` human outside workspace exit 3 stdout empty stderr `awctl: no AWC workspace…`; `status --json` exit 3 error envelope and `.awc` never created; `bogus`/`doctor`/`status --nope` exit 2 usage on stderr; invalid config → exit 1 `invalid_config` |
| Rollback boundary | Delete `crates/awctl/tests/`; revert `main.rs` to 9-line stub, `Cargo.toml` deps, Cargo.lock line, tasks.md checkboxes, Slice 6 progress section. awc-core untouched |

Threat matrix: all rows N/A (design) — no threat-matrix RED tests.

## Changed-Line Estimate
- Authored: **~395** (main.rs 198, tests/cli.rs 146, Cargo.toml 4, tasks.md 6, apply-progress ≈41); Cargo.lock +1 generated, excluded. Budget: 400 hard → **OK** (5-line margin); target ≤330 not met (~65 over) — precedent slices 2 (~390) and 5 (398); scope fully delivered, nothing dropped

## Commands Run with Outcomes
| Command | Outcome |
|---------|---------|
| `cargo test -p awctl` (RED, stub) | Exit 1; `0 passed; 6 failed` |
| `cargo test -p awctl` (GREEN) | Exit 0; `ok. 6 passed; 0 failed` |
| `cargo test --workspace` | Exit 0; 35 passed (29 core + 6 CLI) |
| `cargo fmt` / `cargo fmt --check` | Clean (exit 0) |

## Remaining Work / Risks
- Tasks 6.1–6.2 pending (2). Next: Unit 7 — testing refresh + hygiene (PR 7). No commit/push/PR performed (lifecycle actions require parent receipt validation).
- Pre-existing awc-core unused-import warnings (`CONFIG_SCHEMA_VERSION`, `CONFIG_FILE_NAME` in application.rs, used only in tests) — clippy `-D warnings` (task 6.2) must address; NOT touched in this slice (out of scope).
- Doctor JSON `message` passes core's detail string verbatim (may embed the state DB path) — core-authored, matches doctor's detail-string design; awctl renders as-is. `awctl doctor` without `--quick` is a clap usage error exit 2 (contract-compatible).
# Apply Progress: AWC Foundation — Slice 7 (PR 7)

- **Work unit**: `slice-7-delivery-hygiene` (attempt ordinal 8; previous attempts made zero changes, tree stayed clean); date 2026-08-09; Standard mode (strict_tdd: false)
- **Delivery**: feature-branch-chain; child branch `feature/awc-foundation-07-hygiene` (base `feature/awc-foundation-06-cli`)

## Tasks Completed (6.1–6.2) — ALL TASKS NOW 22/22

- [x] 6.1 Refresh `openspec/config.yaml` testing block with detected commands (cargo test/clippy/fmt) post-bootstrap
- [x] 6.2 Full `cargo test --workspace`, `cargo clippy --workspace -D warnings`, `cargo fmt --check`; fix findings

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awc-core/src/application.rs` | Modified | Moved test-only constants into `#[cfg(test)] mod tests`: removed `CONFIG_SCHEMA_VERSION` from the `domain` import group and `CONFIG_FILE_NAME` from the `config` import; added `use crate::domain::CONFIG_SCHEMA_VERSION;` and `use crate::infrastructure::config::CONFIG_FILE_NAME;` inside the test module. Zero behavior change — both constants are referenced only in tests |
| `openspec/config.yaml` | Modified | Refreshed stale pre-bootstrap facts: `context` (workspace exists, layered architecture, Git work tree on child branch), `testing` (manifest `Cargo.toml`; runner available `cargo test --workspace`; unit `cargo test -p awc-core`; integration `cargo test -p awctl`; e2e NOT claimed; coverage NOT claimed), `quality` (clippy/rustfmt/rustc configured with exact commands), `rules.apply.test_command` and `rules.verify.test_command/build_command` populated; guidelines forbid invented coverage/E2E claims |
| `tasks.md`, `apply-progress.md` | Modified | Checkboxes 6.1–6.2 `[x]` (22/22); Slice 7 section appended (slices 1–7 cumulative) |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | `cargo clippy --workspace -- -D warnings` → RED: exit 101, `error: unused import: CONFIG_SCHEMA_VERSION` + `error: unused import: CONFIG_FILE_NAME` (only findings). GREEN after import fix: exit 0, `Finished dev profile` — zero warnings |
| Runtime harness command/scenario and exact result | Temp-workspace CLI smoke: `init --json` exit 0 single JSON doc stderr empty; `status --json` exit 0; `doctor --quick --json` exit 0 with checks ordered path/config/database/schema; `.awc` contains `config.toml` + `state.sqlite3`; temp dir removed afterward (no `.awc` runtime state committed) |
| Rollback boundary | Revert `application.rs` import move (2-line change in tests module), `openspec/config.yaml` testing/context/rules blocks, `tasks.md` checkboxes 6.1–6.2, and this Slice 7 section. Cargo.lock untouched. No unrelated work removed |

## Gates Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo test --workspace` | Exit 0; 35 passed (29 awc-core + 6 awctl integration; 0 doc tests) |
| `cargo clippy --workspace -- -D warnings` | Exit 0 (was 101 pre-fix on the two test-only imports) |
| `cargo fmt --check` | Exit 0; clean |
| `cargo check --workspace` | Exit 0; no warnings (was 2 unused-import warnings pre-fix) |
| Temp-workspace smoke (`init/status/doctor --quick --json`) | All exit 0; single JSON doc per command; `.awc` seeded correctly |

## Changed-Line Estimate

- Authored: **~90** (application.rs ±3, config.yaml ~42, tasks.md 2, apply-progress.md ≈45)
- Generated: Cargo.lock — untouched this slice
- Budget: 400 hard → **OK** (slice far under budget; final slice of the chain)

## Remaining Work / Status

- **All 22/22 tasks complete.** Next phase: **verify** (sdd-verify), NOT archive — archive only after verification passes.
- No commit/push/PR performed (lifecycle actions require parent receipt validation).
- Engram refreshed in the same run: `sdd/awc-foundation/tasks` (#331), `sdd/awc-foundation/apply-progress` (#333), `sdd/agent-workspace-control/testing-capabilities` (#320).

# Apply Progress: AWC Foundation — Slice 8 (Post-Verification Adjustment)

- **Work unit**: `symlink-init-alignment` (runtime attempt ordinal 9 — maintainer-approved post-verification adjustment; not a new PR slice)
- **Mode**: Standard (strict_tdd: false); behavior-first RED → GREEN for this bug fix
- **Delivery**: ask-on-risk resolved → feature-branch-chain; adjustment applied on the current child branch `feature/awc-foundation-07-hygiene`; no commit/push/PR performed
- **Date**: 2026-08-09

## Adjustment Task (7.1)

- [x] 7.1 `symlink-init-alignment`: `init` accepts a `.awc` symlink whose canonical target remains contained within the workspace root; escaping symlinks still rejected before use; regression tests prove both polarities

## Diagnosis

`init` validated an existing `.awc` with `symlink_metadata(...).is_dir()`, which is always false for a symlink itself — so init rejected even a contained `.awc` symlink, while `discover_with_root` (which canonicalizes and checks the target) accepted it. Fix: extracted the established canonical containment check into `paths::canonicalize_state_within(root, state)` (canonicalize → `starts_with(root)` → `is_dir()` → `UnsafeStatePath`) and used it from both `discover_with_root` and `init`. No weaker path checks were added; escaping rejection and no-write safety are preserved (escaping-symlink marker byte-asserted untouched).

## Files Changed

| File | Action | What Was Done |
|------|--------|---------------|
| `crates/awc-core/src/infrastructure/paths.rs` | Modified | Extracted `canonicalize_state_within(root, state)` — the single canonical containment check — and used it inside `discover_with_root` (behavior unchanged; 8 existing tests green) |
| `crates/awc-core/src/application.rs` | Modified | `init` existing-state branch now calls `paths::canonicalize_state_within(&root, &state_dir)` instead of rejecting on `symlink_metadata(...).is_dir()`; added RED-first regression test `init_accepts_contained_state_symlink` (init + status + doctor over a contained symlink; config/db land at the canonical target) |
| `openspec/changes/awc-foundation/tasks.md` | Modified | Phase 7 adjustment task 7.1 → `[x]` |
| `openspec/changes/awc-foundation/apply-progress.md` | Modified | Slice 8 section appended (slices 1–8 cumulative; prior slice history untouched) |
| `openspec/changes/awc-foundation/verify-report.md` | Untouched | Stale after this adjustment — verification must be re-run before archive; not hand-edited as if final verification passed |

## Work Unit Evidence

| Evidence | Required value |
|---|---|
| Focused test command and exact result | RED: `cargo test -p awc-core init_accepts_contained_state_symlink` → `FAILED. 0 passed; 1 failed` (init rejected the contained symlink). GREEN: same command → `ok. 1 passed; 0 failed`; `cargo test -p awc-core paths` → `ok. 8 passed; 0 failed`; `cargo test -p awc-core application` → `ok. 8 passed; 0 failed` |
| Runtime harness command/scenario and exact result | Real CLI smoke (kernel symlink resolution): contained `.awc` → `awctl init --json` exit 0, `ok:true`, `config.toml` + `state.sqlite3` created at the canonical target; escaping `.awc` → exit 1, JSON `{"code":"unsafe_state_path",...}`, target marker byte-untouched and no state written |
| Rollback boundary | Revert `application.rs` (new test + init branch) and `paths.rs` (helper + `discover_with_root` use) — 2 files; revert tasks.md task 7.1 checkbox and this Slice 8 section. No other code touched; Cargo.lock untouched |

## Changed-Line Estimate

- Authored: **~150** (paths.rs ±20, application.rs 52 changed incl. 32-line regression test, tasks.md ~6, apply-progress.md ~50)
- Generated: Cargo.lock — untouched
- Budget: 400 hard → **OK** (well under; single focused adjustment)

## Gates Run with Outcomes

| Command | Outcome |
|---------|---------|
| `cargo test -p awc-core init_accepts_contained_state_symlink` (RED, before fix) | Exit 1; `0 passed; 1 failed` |
| `cargo test -p awc-core init_accepts_contained_state_symlink` (GREEN, after fix) | Exit 0; `1 passed; 0 failed` |
| `cargo test --workspace` | Exit 0; **36 passed** (30 awc-core + 6 awctl integration; 0 doc tests) |
| `cargo clippy --workspace -- -D warnings` | Exit 0; zero warnings |
| `cargo fmt --check` | Exit 0; clean |
| `cargo check --workspace` | Exit 0; no warnings |

## Remaining Work / Status

- **22/22 original tasks complete + adjustment task 7.1 complete.** Verification is NOT current: `verify-report.md` predates this adjustment; re-run sdd-verify before archive.
- No commit/push/PR performed (lifecycle actions require parent receipt validation).
- Native attempt: attempt ordinal 9 active (`symlink-init-alignment`), objective generation 8, begin revision `sha256:6be2d4f9c533e3ebe05c3c94225fb2043a5e92e979a9e93b346eff13f7ae4075`; implementation + evidence complete, attempt finishing left to the parent orchestrator.
