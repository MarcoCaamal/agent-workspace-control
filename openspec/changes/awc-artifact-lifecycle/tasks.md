# Tasks: AWC Artifact Lifecycle

## Review Workload Forecast

Estimated changed lines: ~1,600 total (A 380, B 420 near limit, C 400, D 390, E 150)

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

Chain strategy pending: orchestrator/user decision required before apply.

### Work Units

| Unit | Goal | Focused test | Harness | Rollback |
|------|------|--------------|---------|----------|
| 1 | Domain/policy/migration | cargo test -p awc-core | N/A: pure logic, in-mem SQLite | Revert domain/error/paths + v3 DDL |
| 2 | Persistence/fs compensation | cargo test -p awc-core | Temp workspace, fs injection | Revert artifacts.rs + repo ops |
| 3 | Use cases | cargo test -p awc-core | Temp workspace integration | Revert application.rs use cases |
| 4 | CLI/contracts | cargo test -p awctl | Manual smoke, 7 commands | Revert main.rs + cli.rs tests |
| 5 | Docs/verification | cargo test --workspace; clippy; fmt --check | Docs walkthrough | Revert docs/*.md |

## Phase 1: Domain/Policy/Migration

- [x] 1.1 RED: transition table in `domain.rs`: legal edges, reject Completed/archived→trashed
- [x] 1.2 GREEN: `ArtifactStatus`/`Artifact`/`PathOwnership`/views/prefix rule in `domain.rs`
- [x] 1.3 RED: ownership/containment in `paths.rs`: protected/ignored/user-managed/symlink/escape
- [x] 1.4 GREEN: `PathOwnership` classification + canonical containment in `paths.rs`
- [ ] 1.5 RED: v3 migration in `sqlite.rs`: tracked→active, backfill, duplicate-path fail (Slice 1B)
- [ ] 1.6 GREEN: migration v3 + partial unique indexes (path; sha256>0) in `sqlite.rs` (Slice 1B)
- [x] 1.7 Add errors (`artifact_not_found`…`duplicate_fingerprint`, `CompensationFailed`) to `error.rs`, exits 0/1/2/3

## Phase 2: Persistence/Filesystem Compensation

- [ ] 2.1 RED: repo CRUD + audit coupling (temp-workspace SQLite)
- [ ] 2.2 GREEN: artifact repo + audit ops in `sqlite.rs`
- [ ] 2.3 Create `infrastructure/artifacts.rs`: injectable `ArtifactFs` (temp-create, rename, trash-move, move-back)
- [ ] 2.4 RED: compensation: fs failure leaves DB unchanged, temp cleaned, `CompensationFailed`
- [ ] 2.5 Add file fingerprint helper in `hash.rs` (size+sha256; shared empty fingerprint)

## Phase 3: Application Use Cases

- [ ] 3.1 RED→GREEN: `create_artifact`: derive `artifacts/<id>`, empty file, fingerprint, reject occupied/duplicate
- [ ] 3.2 RED→GREEN: `show_artifact`/`list_artifacts`: filters, `created_at DESC, id DESC`
- [ ] 3.3 RED→GREEN: `archive_artifact`: status-only + audit
- [ ] 3.4 RED→GREEN: `trash_artifact`: collision-safe `trash/<id>-<basename>`, move, audit
- [ ] 3.5 RED→GREEN: `restore_artifact`: original path free, reverse sequence
- [ ] 3.6 RED→GREEN: `relink_artifact`: old absent, unowned target, refreshed fingerprint
- [ ] 3.7 Verify deferred caps (adopt/purge/reconciliation/MCP) rejected, no mutation

## Phase 4: CLI/Contracts

- [ ] 4.1 RED: contracts in `crates/awctl/tests/cli.rs`: v1 JSON, snake_case errors, exits 0/1/2/3
- [ ] 4.2 GREEN: artifact subcommands in `awctl/src/main.rs` (create/show/list/archive/trash/restore/relink)
- [ ] 4.3 Add human/JSON v1 views (original_path, last_seen_at) + error mapping in `main.rs`
- [ ] 4.4 RED→GREEN: prefix resolution: unknown → `artifact_not_found`, ambiguous → `ambiguous_artifact_id`

## Phase 5: Docs/Verification

- [ ] 5.1 Update `docs/usage.md`: commands, transitions, policy, compensation
- [ ] 5.2 Update `docs/architecture.md`: v3 migration, compensating consistency
- [ ] 5.3 Gate: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`
- [ ] 5.4 Manual smoke: 7 commands in temp workspace; record results
