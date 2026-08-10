```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:d20fdc4c90f62092c358727ee7fa8f9b20dabcea703ac327f2077b45a0a58d48
verdict: pass
blockers: 0
critical_findings: 0
requirements: 6/6
scenarios: 14/14
test_command: cargo test --workspace
test_exit_code: 0
test_output_hash: sha256:86f26d29cfadef0a3528e4b2d46dce79497d4cafd0569df63bb73b2a2c2025a8
build_command: cargo check --workspace && cargo fmt --all -- --check
build_exit_code: 0
build_output_hash: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

## Verification Report

**Change**: `awc-adopt`
**Version**: N/A

### Summary

Verdict **PASS** — all 16 tasks complete, all 6 requirements and 14 scenarios
covered by passing tests, all compatibility constraints preserved.

### Gates

| Gate | Result |
|---|---|
| `cargo test --workspace` | 111 passed, 0 failed (98 awc-core lib + 13 awctl integration) |
| `cargo check --workspace` | Finished |
| `cargo fmt --all -- --check` | Clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 17 pre-existing origin/main baseline; 0 new |
| `git diff --check` | Clean |
| Runtime smoke (temp brownfield) | scan (4 categories) → plan → apply (3 applied, 1 skipped) → stale rejection — all passed |

### Requirements and Scenarios

- **adopt** (5 requirements, 12 scenarios): scan classification read-only;
  plan persistence with workspace fingerprint; per-action apply with stale
  protection; CLI compatibility; deferred capabilities excluded.
- **artifact-lifecycle delta** (1 requirement, 2 scenarios): register an
  existing governed file as an artifact (fingerprint from current bytes,
  path = original_path, status active, unowned + unique non-empty
  fingerprint, audit `artifact.registered`).

### Compatibility Constraints

- JSON schema v1 envelope preserved.
- Exit codes 0/1/2/3 preserved.
- snake_case codes extended: `stale_adopt_plan`, `adopt_plan_not_found`.
- `state.sqlite3`, config schema v1, metadata-only roots preserved.
- Artifact lifecycle invariants unchanged; register-existing is additive.

### Deferred Capabilities (not implemented)

purge, cleanup/retention, reconciliation, MCP, runtime adapters, work items,
secrets beyond flag-and-skip, content scanning.

### Notes

- Clippy baseline: 17 errors identical to `origin/main`; this change adds
  zero new Clippy errors (three introduced during slice implementation were
  fixed in Phase 4).
- Adopt apply moves candidates into `artifacts/` before registration (path
  policy restricts writes to `artifacts/**` and `trash/**`); unknown
  candidates move to `inbox/`; nothing is ever deleted.
