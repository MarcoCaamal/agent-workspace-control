```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:9424b435be70740373570109577402c252fe1eb11a4e24d16f2463c3c2e67e9a
verdict: pass
blockers: 0
critical_findings: 0
requirements: 8/8
scenarios: 17/17
test_command: cargo test --workspace
test_exit_code: 0
test_output_hash: sha256:3c2a893bdb93c8e244dbd52a261124c906d7a842a7d90bd3645aba70433e30f0
build_command: cargo check --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings
build_exit_code: 0
build_output_hash: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

## Verification Report

**Change**: `awc-artifact-lifecycle`
**Version**: N/A

### Summary

Verdict **PASS** — all 27 tasks complete, all 8 requirements and 17 scenarios
covered by passing tests, all compatibility constraints preserved.

### Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 94 passed, 0 failed (82 awc-core lib + 12 awctl integration) |
| `cargo check --workspace` | Finished |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 17 pre-existing origin/main baseline errors; 0 new from this change |
| `git diff --check` | Clean |
| Runtime smoke (temp workspace) | init → project add → artifact create/show/list/archive/restore(archived→active)/trash/restore(trashed→active)/relink-guard — all passed |

### Requirements and Scenarios

- **artifact-lifecycle** (5 requirements, 9 scenarios): create/identify governed artifacts; query metadata; lifecycle/relink transitions; mutation/audit/compensation coupling; deferred capabilities excluded. All implemented and covered by unit + CLI integration tests.
- **artifact-path-policy** (2 requirements, 4 scenarios): fixed ownership classes; artifact write targets restricted to `artifacts/**` and `trash/**`; symlink/escape/ownership rejection. All implemented and tested.
- **workspace-foundation** (1 requirement, 4 scenarios): schema-v3 additive migration with canonical status alignment, backfill, partial unique indexes, and no-mutation refusal on duplicate/invalid legacy rows. All implemented and tested.

### Compatibility Constraints

- JSON schema v1 envelope (`schemaVersion`, exactly `data` xor `error`): preserved.
- Exit codes 0/1/2/3: preserved.
- snake_case error codes: preserved and extended (`artifact_not_found`, `ambiguous_artifact_id`, `path_owned`, `protected_path`, `path_escape`, `artifact_status_conflict`, `restore_conflict`, `duplicate_fingerprint`, `compensation_failed`, `migration_conflict`).
- `state.sqlite3`: retained.
- Config schema v1: retained; no config keys added.
- External project `root_path`: metadata only; no managed writes.

### Deferred Capabilities (not implemented)

adopt, purge, retention, reconciliation, MCP, runtime adapters, work items, secrets.

### Notes

- Clippy baseline: 17 errors are identical to `origin/main`; this change adds zero new Clippy errors after fixing three introduced during Phases 2/3 (collapsible if, `Error::other`, missing `Default`).
- DB/filesystem mutations use compensating consistency, never cross-resource ACID; crash residue stays observable (documented in `docs/architecture.md`).
