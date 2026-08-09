```yaml
schema: gentle-ai.verify-result/v1
evidence_revision: sha256:d019fc26034401c6f32175130d128ed5c09a837952746a9747ef9eb01b0936e3
verdict: pass
blockers: 0
critical_findings: 0
requirements: 6/6
scenarios: 12/12
test_command: cargo test --workspace
test_exit_code: 0
test_output_hash: sha256:b356076e8ef73e7e86b6ca56d02140d1e10c01e0ed71cf323b53f6cd884267cf
build_command: cargo check --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings
build_exit_code: 0
build_output_hash: sha256:bffca379cc6b6d36132a4ac4fbe9f63bde05978fffd1a67ea72f7fa25560f346
```

## Verification Report

**Change**: `awc-schema-identity-projects`  
**Version**: N/A  
**Mode**: Standard (`strict_tdd=false`)  
**Artifact mode**: Hybrid (OpenSpec + Engram)

### Evidence Identity

The evidence revision hashes this exact implementation preimage:

```text
HEAD=a766622dc0c547c314124b732cd4be4c356edd3d
TREE=ba9677e258238bad650ea64c5755d6e0f22840d9
```

The working tree was clean before verification. Verification did not modify production code, tasks, commits, branches, reviews, or pull requests.

### Completeness

| Metric | Value |
|---|---:|
| Requirements total / verified | 6 / 6 |
| Scenarios total / compliant | 12 / 12 |
| Tasks total | 25 |
| Tasks complete | 25 |
| Tasks incomplete | 0 |

All task checkboxes were independently counted from `tasks.md`. Source inspection confirmed the implementation described by the completed task groups in `domain.rs`, `hash.rs`, `error.rs`, `sqlite.rs`, `config.rs`, `paths.rs`, `application.rs`, `main.rs`, and `cli.rs`.

### Build & Tests Execution

**Tests**: ✅ 68 passed, 0 failed, 0 ignored

```text
$ cargo test --workspace
awc-core: 58 passed; 0 failed
awctl CLI integration: 10 passed; 0 failed
doc tests: 0 passed; 0 failed
exit code: 0
combined stdout/stderr sha256: b356076e8ef73e7e86b6ca56d02140d1e10c01e0ed71cf323b53f6cd884267cf
```

**Build / static gates**: ✅ Passed

```text
$ cargo check --workspace && cargo fmt --check && cargo clippy --workspace -- -D warnings
exit code: 0
combined stdout/stderr sha256: bffca379cc6b6d36132a4ac4fbe9f63bde05978fffd1a67ea72f7fa25560f346
```

**Coverage**: ➖ No coverage command or threshold is configured; scenario compliance is established by passing named runtime tests plus the independent CLI harness below.

### Independent Runtime Harness

A separate temporary-workspace harness invoked `target/debug/awctl` rather than reusing apply-phase claims. It initialized twice, removed and repaired `tmp/`, compared config hashes, verified all four governed directories, added derived and explicit slugs, exercised full-ID/unique-prefix/ambiguous-prefix/not-found behavior, listed and showed projects in human and JSON modes, checked exits 0/1/2/3, and used an existing external root with a marker hash to prove no write and no external `.awc` creation.

```text
{"schemaVersion":1,"ok":true,"data":{"root":"/tmp/opencode/awc-verify-runtime.Xi1yZ7","schemaVersion":1,"databaseOk":true,"schemaOk":true}}
{"schemaVersion":1,"ok":true,"data":{"root":"/tmp/opencode/awc-verify-runtime.Xi1yZ7","schemaVersion":1,"databaseOk":true,"schemaOk":true}}
RUNTIME_OK init_repair=true config_unchanged=true governed_dirs=4 uuidv7_ids=019fe83e-d0b5-7631-9765-658b009b9298,019fe83e-d0b6-7b90-9e2b-2ebaea571c4b derived_slug=alpha-core explicit_slug=beta-service prefix_unique=true prefix_ambiguous_exit=1 collision_exit=1 not_found_exit=1 usage_exit=2 workspace_exit=3 external_root_unchanged=true
RUNTIME_EXIT=0
RUNTIME_HASH=sha256:320a8017a0ad6fd93f4ac7b9ef0e2ecf7ed0cbb4bd91edc2c436d263f1e29952
```

### Spec Compliance Matrix

| Requirement | Scenario | Passing runtime evidence | Result |
|---|---|---|---|
| Typed UUIDv7 identities and prefix resolution | Resolve a unique project prefix | `application::tests::show_project_resolves_prefixes_and_list_is_deterministic`; `project_show_resolves_prefix_and_rejects_not_found_or_ambiguous`; independent harness | ✅ COMPLIANT |
| Typed UUIDv7 identities and prefix resolution | Reject an ambiguous or unknown prefix | `domain::tests::prefix_resolve_zero_matches_reports_not_found`; `domain::tests::prefix_resolve_multiple_matches_reports_ambiguity`; CLI integration test; independent harness | ✅ COMPLIANT |
| Create projects with deterministic slugs | Add a project with a derived slug | `application::tests::add_project_derives_slug_persists_and_reports`; `project_add_derives_slug_json_and_human`; independent harness | ✅ COMPLIANT |
| Create projects with deterministic slugs | Reject a slug collision | `application::tests::add_project_rejects_derived_and_explicit_slug_collisions`; `project_add_slug_conflict_exits_1_without_insert`; independent harness | ✅ COMPLIANT |
| Query project metadata safely | Show an externally rooted project | `project_list_is_deterministic_and_external_root_is_metadata_only`; independent existing-root marker harness | ✅ COMPLIANT |
| Query project metadata safely | Return a stable JSON failure | `project_show_resolves_prefix_and_rejects_not_found_or_ambiguous`; `operational_failure_exits_1_with_json_error`; independent harness | ✅ COMPLIANT |
| Defensive schema-v2 migration | Migrate an empty v0.1 workspace | `infrastructure::sqlite::tests::empty_v1_database_migrates_to_v2`; `migrate_v2_creates_full_metadata_schema_and_records_ledger` | ✅ COMPLIANT |
| Defensive schema-v2 migration | Refuse populated legacy data | `infrastructure::sqlite::tests::populated_v1_table_rejects_v2_without_mutation` | ✅ COMPLIANT |
| Backward-compatible governed-directory configuration | Load a v1 configuration without directory fields | `infrastructure::config::tests::v1_config_without_dir_fields_loads_defaults_and_preserves_bytes` | ✅ COMPLIANT |
| Safe workspace initialization and repair | Initialize a new workspace | `application::tests::init_then_status_and_doctor_from_nested_dir`; `init_json_exits_0_with_data_and_silent_stderr`; independent harness | ✅ COMPLIANT |
| Safe workspace initialization and repair | Repair partial state without configuration loss | `application::tests::reinit_repairs_missing_db_and_preserves_config_bytes`; `infrastructure::sqlite::tests::repair_recreates_missing_db_and_preserves_config_bytes` | ✅ COMPLIANT |
| Safe workspace initialization and repair | Repair missing governed directories | `application::tests::init_creates_and_repairs_governed_dirs_preserving_config`; independent harness | ✅ COMPLIANT |

**Compliance summary**: 12/12 scenarios compliant.

### Correctness (Static Evidence)

| Requirement | Status | Notes |
|---|---|---|
| Typed UUIDv7 identities and prefix resolution | ✅ Implemented | Typed `ProjectId`, `ArtifactId`, and `AuditEventId` wrap `Uuid`; generation uses `Uuid::now_v7`; prefix selection accepts exactly one canonical textual match; streaming SHA-256 returns lower-case 64-hex plus exact size. |
| Create projects with deterministic slugs | ✅ Implemented | Derivation lowercases ASCII alphanumerics, collapses separator runs, trims, rejects empty values, validates explicit slugs, and enforces unique persistence with no insert on conflict. |
| Query project metadata safely | ✅ Implemented | List order is deterministic by slug; show resolves full IDs and prefixes; human and schema-v1 JSON renderers are present; `root_path` is only persisted/read and is never passed to a write API. Error codes remain snake_case. |
| Defensive schema-v2 migration | ✅ Implemented | The v2 transaction checks all three v1 tables before DDL, drops in FK-safe order only when empty, creates complete metadata tables, and records ledger v2 atomically. `state.sqlite3` remains the default filename. |
| Backward-compatible governed-directory configuration | ✅ Implemented | Config schema remains 1; all four directory fields use serde defaults; existing config is parsed without serialization or rewrite. |
| Safe workspace initialization and repair | ✅ Implemented | Init creates/repairs state and four governed directories, reruns migrations idempotently, preserves valid config bytes, rejects parent traversal and escaping governed/state symlinks, and avoids using external targets. |

### Contract Checks

| Contract | Evidence | Result |
|---|---|---|
| UUIDv7 typed IDs | Version/variant unit test passed; independent generated IDs have UUIDv7 version nibble | ✅ |
| SHA-256 + exact size | Known-vector, empty, multi-read, and reader-error tests passed | ✅ |
| Derived and explicit slug behavior | Unit, application, CLI, and independent runtime checks passed | ✅ |
| Deterministic unique prefix | Pure resolver, application, CLI, and independent runtime checks passed | ✅ |
| Migration no mutation | Guard precedes DDL in the v2 transaction; populated-v1 test proves rows, schema, and ledger unchanged | ✅ |
| Governed-dir repair and containment | Repair, parent traversal, escaping symlink, and contained nested-path tests passed | ✅ |
| Human and JSON project CLI | Add/list/show integration tests and independent harness passed | ✅ |
| Exit codes 0/1/2/3 | CLI tests and independent harness exercised all four codes | ✅ |
| External root metadata no-write | CLI test plus existing-directory marker hash and absence of external `.awc` passed | ✅ |

### Coherence (Design)

| Decision | Followed? | Notes |
|---|---|---|
| Typed UUIDv7 IDs and application-boundary prefix resolution | ✅ Yes | Newtypes and UUIDv7 generation are in the domain; SQLite supplies candidates; application performs exact-one resolution. |
| Transactional guarded schema-v2 rebuild | ✅ Yes | Populated legacy rows are checked before DDL in the migration transaction; ledger insertion and DDL commit together. |
| Future-ready metadata without lifecycle CRUD | ✅ Yes | Full Project/Artifact/AuditEvent schema exists; only project add/list/show behavior was added. |
| Concrete synchronous layered core | ✅ Yes | No async runtime or provider abstractions were introduced; CLI remains parsing/rendering around application calls. |
| Schema-v1 defaults and contained governed paths | ✅ Yes | Existing bytes remain unchanged; configured/default directories are containment-checked before use. |
| Thin CLI with stable envelope and exits | ✅ Yes | CLI dispatches to core use cases and renders the established schema-version-1 envelope and error mappings. |

### Issues Found

**CRITICAL**: None.  
**WARNING**: None.  
**SUGGESTION**: None.

### Verdict

**PASS**

All 6 requirements, all 12 scenarios, and all 25 tasks are verified against current source and passing runtime evidence. Workspace tests, check, formatting, Clippy, migration safety, path containment/repair, identity/slug/prefix behavior, CLI forms, exit contracts, and external-root no-write behavior passed independently.
