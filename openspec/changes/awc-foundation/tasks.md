# Tasks: AWC Foundation

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | ~1,700–1,900 authored (generated Cargo.lock excluded) |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 → PR 2 → PR 3 → PR 4 → PR 5 → PR 6 → PR 7 |
| Delivery strategy | ask-on-risk |
| Chain strategy | pending |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

### Suggested Work Units

| Unit | Goal | Likely PR | Focused test command | Runtime harness | Rollback boundary |
|------|------|-----------|----------------------|-----------------|-------------------|
| 1 | Workspace + crate manifests + .gitignore | PR 1 | cargo check --workspace | cargo build --workspace | Remove Cargo.toml, Cargo.lock, crates/ |
| 2 | Domain, errors, TOML config | PR 2 | cargo test -p awc-core config | cargo test -p awc-core (temp TOML fixtures) | Revert error/domain/config/lib.rs |
| 3 | Discovery + symlink containment | PR 3 | cargo test -p awc-core paths | cargo test -p awc-core (real symlink fixtures) | Revert paths.rs + tests |
| 4 | SQLite migrations + repair | PR 4 | cargo test -p awc-core sqlite | cargo test -p awc-core (temp SQLite) | Revert sqlite.rs + tests |
| 5 | init/status/doctor_quick | PR 5 | cargo test -p awc-core | cargo test -p awc-core (temp workspace e2e) | Revert application.rs + tests |
| 6 | awctl CLI, renderers, exits | PR 6 | cargo test -p awctl | awctl init --json in temp dir; exits 0/1/2/3 | Revert crates/awctl |
| 7 | Testing refresh + hygiene | PR 7 | cargo test --workspace && cargo clippy --workspace | Full suite + .awc smoke test | Revert config.yaml testing block |

## Phase 1: Repository Bootstrap

- [x] 1.1 Create root `Cargo.toml` workspace (members `crates/awc-core`, `crates/awctl`)
- [x] 1.2 Create `crates/awc-core/Cargo.toml`: rusqlite bundled, serde, toml; no clap/Tokio
- [x] 1.3 Create `crates/awctl/Cargo.toml`: clap, serde_json, awc-core path dep
- [x] 1.4 Add `/target`, `.awc/` to `.gitignore`; compiling `lib.rs`/`main.rs` stubs
- [x] 1.5 Run `cargo check --workspace` (pins generated `Cargo.lock`)

## Phase 2: Core Contracts and Config

- [ ] 2.1 RED: unit tests — invalid TOML, unknown `schema_version`, byte-preserving round trip
- [ ] 2.2 Implement `error.rs`: `AwcError` variants per design contract
- [ ] 2.3 Implement `domain.rs`: Workspace, Config, CheckResult, Status, CommandResult, InitStatus
- [ ] 2.4 Implement `config.rs`: validate, preserve valid bytes, atomic write, reject unknown versions
- [ ] 2.5 RED: path tests — nearest ancestor, internal symlink accepted, escaping symlink rejected without target access
- [ ] 2.6 Implement `paths.rs`: canonical start, nearest-first ancestor walk, canonical containment check

## Phase 3: SQLite Migrations

- [ ] 3.1 RED: migration tests — `schema_migrations(version)` ledger, `projects`/`artifacts`/`audit_events`, rerun idempotent
- [ ] 3.2 Implement `sqlite.rs`: bundled rusqlite, transactional ordered migrations, schema health
- [ ] 3.3 Repair test: valid config + missing DB → state restored, config bytes unchanged

## Phase 4: Application Use Cases

- [ ] 4.1 Implement `application.rs` `init`: create dir, atomic config, open+migrate; remove only empty dir on config failure
- [ ] 4.2 Implement read-only `status` and `doctor_quick` (config/database/schema/path checks)
- [ ] 4.3 RED: integration — nested discovery, no-workspace without creating `.awc`, unchanged file metadata

## Phase 5: CLI Contracts

- [ ] 5.1 RED: CLI tests — exits 0/1/2/3; JSON envelope (`schemaVersion`+`ok`, exactly `data` or `error`); errors on stderr
- [ ] 5.2 Implement `main.rs`: clap (init, status, doctor --quick, --json), human/JSON renderers
- [ ] 5.3 Emit one newline-terminated JSON doc; clap errors → Usage, exit 2

## Phase 6: Delivery Hygiene

- [ ] 6.1 Refresh `openspec/config.yaml` testing block with detected commands (cargo test/clippy/fmt) post-bootstrap
- [ ] 6.2 Full `cargo test --workspace`, `cargo clippy --workspace -D warnings`, `cargo fmt --check`; fix findings

Threat matrix: all rows N/A — no RED tests.
