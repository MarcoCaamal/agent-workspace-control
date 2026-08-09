# Tasks: AWC Schema, Identity, and Projects

## Review Workload Forecast

| Field | Value |
|-------|-------|
| Estimated changed lines | 700–900 |
| 400-line budget risk | High |
| Chained PRs recommended | Yes |
| Suggested split | PR 1 (primitives) → PR 2 (foundation) → PR 3 (CLI) |
| Delivery strategy | ask-on-risk |
| Chain strategy | feature-branch-chain |

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: feature-branch-chain
400-line budget risk: High

### Suggested Work Units (feature-branch-chain: PR #1 base = tracker `feature/awc-schema-identity-projects`; PR #2 base = PR #1 branch; PR #3 base = PR #2 branch)

| Unit | Goal | Likely PR | Focused test command | Runtime harness | Rollback boundary |
|------|------|-----------|----------------------|-----------------|-------------------|
| 1 | UUIDv7 identities, slug rules, hash/size primitives | PR 1 | `cargo test -p awc-core` | N/A — pure library module; no runnable behavior | Revert `Cargo.toml`, `domain.rs`, `hash.rs`, `error.rs` |
| 2 | Schema v2 migration, config defaults, path containment | PR 2 | `cargo test -p awc-core` | `cargo run -p awctl -- init` on temp dir; inspect ledger | Revert `sqlite.rs`, `config.rs`, `paths.rs`, `mod.rs`; restore pre-migration DB backup |
| 3 | Project add/list/show + CLI + integration tests | PR 3 | `cargo test -p awctl` | `cargo run -p awctl -- project add/show/list` on temp workspace; assert JSON + exits | Revert `application.rs`, `main.rs`, `cli.rs` |

## Phase 1: Identity and Hash Primitives (PR 1)

- [ ] 1.1 RED: `domain.rs` tests — UUIDv7 `ProjectId`/`ArtifactId`/`AuditEventId`; prefix resolve: 1 row selects, 0 → not-found, 2+ → ambiguous
- [ ] 1.2 RED: `hash.rs` tests — SHA-256 lower-case 64-hex + exact byte count
- [ ] 1.3 RED: slug tests — lowercase, non-alnum runs → single `-`, trimmed, empty rejected
- [ ] 1.4 GREEN: `crates/awc-core/Cargo.toml` — add `uuid` (v7, serde), `sha2`
- [ ] 1.5 GREEN: `domain.rs` — newtypes, `ContentFingerprint`, `derive_slug` rules
- [ ] 1.6 GREEN: create `infrastructure/hash.rs` — synchronous fingerprint over reader
- [ ] 1.7 GREEN: `error.rs` — add `project_not_found`, `ambiguous_project_id`, `slug_conflict`, `legacy_schema_data` (snake_case)
- [ ] 1.8 Gate: `cargo clippy -p awc-core -- -D warnings`; `cargo fmt --check`

## Phase 2: Foundation — Migration, Config, Paths (PR 2)

- [ ] 2.1 RED: `sqlite.rs` — empty v0.1 migrates to v2 (Project/Artifact/AuditEvent tables, ledger 2, `state.sqlite3` kept)
- [ ] 2.2 RED: populated v0.1 table → `LegacySchemaData`, no DDL, rows/ledger unchanged
- [ ] 2.3 RED: `config.rs` — v1 config without dir fields loads defaults, bytes unchanged
- [ ] 2.4 RED: `paths.rs` — create/repair 4 governed dirs; reject escaping target/symlink
- [ ] 2.5 GREEN: `sqlite.rs` — v2 migration transaction: row-count guard → FK-safe drop → create v2 → ledger 2
- [ ] 2.6 GREEN: `config.rs` — serde defaults `artifacts_dir`/`inbox_dir`/`tmp_dir`/`trash_dir`
- [ ] 2.7 GREEN: `paths.rs` — governed-dir repair with containment validation
- [ ] 2.8 GREEN: `infrastructure/mod.rs` — export `hash`; gate clippy/fmt

## Phase 3: Project Use Cases and CLI (PR 3)

- [ ] 3.1 RED: `application.rs` — `add_project` derives slug, persists, reports
- [ ] 3.2 RED: slug collision → `slug_conflict`, no insert
- [ ] 3.3 RED: `show_project` prefix resolve (not-found/ambiguous), `root_path` metadata only, no external write; `list_projects` deterministic
- [ ] 3.4 RED: `crates/awctl/tests/cli.rs` — JSON envelope `{schemaVersion:1, ok, data|error}`, exits 0/1/2/3
- [ ] 3.5 GREEN: `application.rs` — `add_project`/`list_projects`/`show_project`
- [ ] 3.6 GREEN: `main.rs` — `project add/list/show` parsing + human/JSON views
- [ ] 3.7 GREEN: `cli.rs` integration tests wired; gate clippy/fmt

## Phase 4: Verification

- [ ] 4.1 `cargo test --workspace` + `cargo check --workspace` + `cargo fmt --check` all green
- [ ] 4.2 Manual smoke: `awctl init` twice (repair), `project add/show/list` human+JSON — confirm dirs, exits, no external-root write
