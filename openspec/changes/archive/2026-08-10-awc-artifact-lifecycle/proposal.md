# Proposal: AWC Artifact Lifecycle

## Intent

Make governed artifacts safe and usable through deterministic lifecycle commands, strict path ownership, durable metadata, and auditable DB/filesystem mutations while preserving shipped compatibility contracts.

## Scope

### In Scope
- Define artifact metadata, `active`/`archived`/`trashed`, active→archived/trashed and archived/trashed→active transitions, unique paths, and duplicate non-empty fingerprint rejection; allow multiple empty artifacts.
- Implement project-, title-, and type-required `artifact create/show/list/archive/trash/restore/relink`; create only under `artifacts/`, expose complete metadata and project/type/status filters, and order lists by `created_at` descending.
- Enforce fixed Rust path policy: governed AWC directories, protected agent-runtime paths, ignored `.git/**` and `target/**`, and metadata-only external project roots.
- Add additive migration support, persistence, fingerprinting, mandatory transactional audit events, filesystem compensation, human/JSON views, tests, and documentation.

### Out of Scope
- Adopt; cleanup, retention, or purge; reconciliation; MCP; runtime adapters; work items; secrets.
- Nullable project ownership, Completed status, or Archived→Trashed.

## Capabilities

### New Capabilities
- `artifact-lifecycle`: Artifact metadata, creation, queries, transitions, strict relink, fingerprint uniqueness, audit, and compensated filesystem behavior.
- `artifact-path-policy`: Ownership classification, protected paths, containment, and governed write boundaries.

### Modified Capabilities
- `workspace-foundation`: Additive migration aligns artifact statuses and supports lifecycle-required timestamps and path uniqueness without changing `state.sqlite3` or config schema v1.

## Approach

Deliver layered domain/policy, persistence/filesystem, application, and CLI slices. Archive changes status only; trash moves to a collision-safe governed path; restore returns to the original unoccupied path; relink requires the old file absent, an unowned `artifacts/` target, and refreshed hash/size. Keep each mutation and audit event in one transaction/compensation unit.

## Affected Areas

| Area | Impact | Description |
|---|---|---|
| `crates/awc-core/src/{domain,error,application}.rs` | Modified | Contracts and use cases |
| `crates/awc-core/src/infrastructure/{sqlite,paths,hash}.rs` | Modified | Migration, policy, persistence, compensation |
| `crates/awctl/src/main.rs`, `crates/awctl/tests/cli.rs` | Modified | Commands, views, contract tests |
| `docs/{usage,architecture}.md` | Modified | Lifecycle documentation |

## Risks

| Risk | Likelihood | Mitigation |
|---|---|---|
| DB/filesystem divergence | High | Atomic operations, compensation, mandatory audit |
| Path escape or protected mutation | High | Normalize, contain, validate symlinks, classify policy |
| Review overload | High | Ask before apply; plan chained slices under 400 lines |

## Rollback Plan

Revert command and policy code; retain additive schema data for forward recovery. Restore files through recorded original paths and audit evidence; never destructive-down-migrate.

## Dependencies

- Requires archived `awc-schema-identity-projects` foundations: schema ledger, typed IDs/fingerprints, project ownership, governed directories, and stable CLI contracts.

## Success Criteria

- [ ] All seven commands enforce lifecycle, ownership, fingerprint, audit, and compensation rules.
- [ ] Human/JSON outputs preserve schema v1, snake_case errors, and exits 0/1/2/3.
- [ ] Migration preserves existing workspaces, `state.sqlite3`, config v1, and mandatory project ownership.
