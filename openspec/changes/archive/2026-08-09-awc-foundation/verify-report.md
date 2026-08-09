```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:e898d749ad151ed1f96b266d40d0c815a6aa739b66c6433f793dd8387c5c67c4
verdict: pass
blockers: 0
critical_findings: 0
requirements: 6/6
scenarios: 12/12
test_command: cargo test --workspace
test_exit_code: 0
test_output_hash: sha256:c2115d81c6b79dba5347dabeef273932f4fba48dfa2e6e4552dece6373f1317d
build_command: cargo check --workspace
build_exit_code: 0
build_output_hash: sha256:99f71393b9903dca098310bb446022a4bdee195ea9996f351a787195555210a6
```

## Verification Report

**Change**: awc-foundation
**Version**: N/A
**Mode**: Standard
**Review lineage**: `review-68bca93848f5cece-r1`
**Binding revision**: `sha256:40ed4b43eb6817f09fe07b4c432ace661b81326698c4cad4b27a81abf842952a`
**Review authority revision**: `sha256:5e344040e811cead56c3461a94a1f29a00e2434965e31793a5cb4eb277f429cb`
**Candidate tree**: `2f687f86d03e6f5556dd9eacda9a259e56c24af2` (matches the bound corrected review candidate)
**Runtime evidence**: attempt 9 completed and passed at `sha256:e898d749ad151ed1f96b266d40d0c815a6aa739b66c6433f793dd8387c5c67c4`

### Completeness

| Metric | Value |
|--------|-------|
| Requirements total / complete | 6 / 6 |
| Scenarios total / compliant | 12 / 12 |
| Tasks total | 23 |
| Tasks complete | 23 |
| Tasks incomplete | 0 |

### Build & Tests Execution

**Tests**: ✅ 36 passed / 0 failed / 0 ignored

```text
cargo test --workspace
Exit: 0
Output hash: sha256:c2115d81c6b79dba5347dabeef273932f4fba48dfa2e6e4552dece6373f1317d
30 awc-core tests and 6 awctl CLI integration tests passed; binary and doc-test targets contained 0 tests.
```

**Build/type-check**: ✅ Passed

```text
cargo check --workspace
Exit: 0
Output hash: sha256:99f71393b9903dca098310bb446022a4bdee195ea9996f351a787195555210a6
```

**Lint**: ✅ Passed

```text
cargo clippy --workspace -- -D warnings
Exit: 0
Output hash: sha256:99f71393b9903dca098310bb446022a4bdee195ea9996f351a787195555210a6
```

**Format**: ✅ Passed

```text
cargo fmt --check
Exit: 0
Output hash: sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
```

**Focused contained-symlink test**: ✅ Passed

```text
cargo test -p awc-core init_accepts_contained_state_symlink
Exit: 0
Output hash: sha256:01b7e636b0d64cbf3da2664f38e101d9e4980985c204a5c51557ece196feb0f9
```

**Focused escaping-symlink test**: ✅ Passed

```text
cargo test -p awc-core doctor_and_init_reject_escaping_state_symlink
Exit: 0
Output hash: sha256:3b9f5473d2f32fc277f15da873664155ea9ce7d51713bb1b778b55ac7169da2c
```

**Focused path-safety suite**: ✅ 8 passed

```text
cargo test -p awc-core paths
Exit: 0
Output hash: sha256:6946923dbebf34d1b1a1d889fa3420ca534ce63f7250eedfbe68307462ca30c8
```

**Manual CLI smoke harness**: ✅ Passed

```text
Contained `.awc` symlink: awctl init --json; nested awctl status --json; nested awctl doctor --quick --json
Escaping `.awc` symlink: awctl init --json
Exits: 0 / 0 / 0 / 1 (expected unsafe_state_path)
Harness exit: 0
Output hash: sha256:4cea43ebcb5b3d2dd5b4c6c5ff6176b161c468a99d5d6c32300d357d9d30f357
Config and database were written through the retained canonical contained target; the external escape marker stayed unchanged and no external state was created. Temporary workspaces were removed.
```

**Coverage**: ➖ Not available (no coverage command is configured)
**E2E layer**: ➖ Not configured; the CLI smoke run is recorded only as a manual harness.

### Spec Compliance Matrix

| Requirement | Scenario | Passing runtime evidence | Result |
|-------------|----------|--------------------------|--------|
| Stable command result contracts | Emit a successful JSON result | `crates/awctl/tests/cli.rs > init_json_exits_0_with_data_and_silent_stderr`; `human_init_then_status_and_doctor_json_from_nested_dir` | ✅ COMPLIANT |
| Stable command result contracts | Emit a failed JSON result | `crates/awctl/tests/cli.rs > workspace_not_found_exits_3_with_json_error_and_no_mutation`; `operational_failure_exits_1_with_json_error` | ✅ COMPLIANT |
| Typed failures and exit codes | Reject invalid command usage | `crates/awctl/tests/cli.rs > usage_errors_exit_2_with_stderr_only` | ✅ COMPLIANT |
| Typed failures and exit codes | Return workspace-not-found | `crates/awctl/tests/cli.rs > workspace_not_found_exits_3_with_json_error_and_no_mutation`; `crates/awc-core/src/application.rs > read_only_commands_error_without_creating_awc` | ✅ COMPLIANT |
| Safe workspace initialization and repair | Initialize a new workspace | `crates/awc-core/src/application.rs > init_then_status_and_doctor_from_nested_dir`; `crates/awc-core/src/infrastructure/sqlite.rs > migrations_create_ledger_and_tables` | ✅ COMPLIANT |
| Safe workspace initialization and repair | Repair partial state without configuration loss | `crates/awc-core/src/application.rs > reinit_repairs_missing_db_and_preserves_config_bytes` | ✅ COMPLIANT |
| Upward workspace discovery and path containment | Discover from a nested directory | `crates/awc-core/src/infrastructure/paths.rs > nearest_ancestor_wins`; `crates/awc-core/src/application.rs > init_then_status_and_doctor_from_nested_dir` | ✅ COMPLIANT |
| Upward workspace discovery and path containment | Refuse an escaping state symlink | `crates/awc-core/src/infrastructure/paths.rs > escaping_symlink_rejected_without_target_use`; `crates/awc-core/src/application.rs > doctor_and_init_reject_escaping_state_symlink` | ✅ COMPLIANT |
| Deterministic workspace status | Inspect a discovered workspace | `crates/awc-core/src/application.rs > read_only_commands_preserve_config_bytes_and_metadata`; `init_then_status_and_doctor_from_nested_dir` | ✅ COMPLIANT |
| Deterministic workspace status | Report no workspace | `crates/awc-core/src/application.rs > read_only_commands_error_without_creating_awc`; `crates/awctl/tests/cli.rs > workspace_not_found_exits_3_with_json_error_and_no_mutation` | ✅ COMPLIANT |
| Read-only quick diagnostics | Run healthy quick diagnostics | `crates/awc-core/src/application.rs > init_then_status_and_doctor_from_nested_dir`; `read_only_commands_preserve_config_bytes_and_metadata` | ✅ COMPLIANT |
| Read-only quick diagnostics | Detect partial or unsafe state | `crates/awc-core/src/application.rs > doctor_reports_unhealthy_db_without_creating_it`; `doctor_and_init_reject_escaping_state_symlink` | ✅ COMPLIANT |

**Compliance summary**: 12/12 scenarios compliant; 6/6 requirements complete.

### Correctness (Static Evidence)

| Requirement | Status | Notes |
|------------|--------|-------|
| Stable command result contracts | ✅ Implemented | Typed declaration-order JSON views emit `schemaVersion`, `ok`, and exactly one of `data` or `error`. |
| Typed failures and exit codes | ✅ Implemented | Core errors map operational/usage/not-found failures to exits 1/2/3; success returns 0; read-only failures do not initialize state. |
| Safe workspace initialization and repair | ✅ Implemented | Initialization creates versioned TOML and transactional SQLite migrations, preserves valid config bytes, repairs missing database state, and uses the retained canonical state path for writes. |
| Upward discovery and path containment | ✅ Implemented | Shared canonical containment accepts contained `.awc` symlinks and rejects escaping targets before config/database use. |
| Deterministic workspace status | ✅ Implemented | Status uses read-only config/database access and reports root, config version, and database/schema health. |
| Read-only quick diagnostics | ✅ Implemented | Quick diagnostics execute path/config/database/schema checks without creating or repairing database state. |

### Coherence (Design)

| Decision | Followed? | Notes |
|----------|-----------|-------|
| Synchronous `awc-core` plus thin `awctl` | ✅ Yes | Core has no clap/Tokio dependency; CLI owns parsing and rendering. |
| Transactional ordered migration ledger | ✅ Yes | `schema_migrations(version INTEGER PRIMARY KEY)` records ordered migrations. |
| Versioned TOML with byte preservation | ✅ Yes | Valid existing bytes are parsed without rewrite; unsupported versions fail. |
| Typed deterministic JSON | ✅ Yes | Struct-based rendering preserves the declared envelope order and one-document output. |
| Read-only status and quick doctor | ✅ Yes | SQLite is opened with `SQLITE_OPEN_READ_ONLY`; tests prove missing state is not recreated. |
| Canonical `.awc` containment and contained-symlink initialization | ✅ Yes | Discovery and `init` share `canonicalize_state_within`; `init` retains the returned canonical `PathBuf` and uses it for config/database writes, closing the reviewed validation-to-write TOCTOU gap while preserving escape rejection. |

### Issues Found

**CRITICAL**: None.

**WARNING**: None.

**SUGGESTION**: None.

### Verdict

**PASS**

All 23 tasks are complete, all 6 requirements and 12 scenarios have passing runtime coverage, all 36 workspace tests pass, and check/clippy/fmt plus focused path-safety gates pass. The prior contained-symlink warning is resolved, and current source/runtime evidence proves the bounded TOCTOU correction retains and writes through the validated canonical state path.
