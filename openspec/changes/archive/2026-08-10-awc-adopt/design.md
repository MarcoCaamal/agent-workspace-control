# Design: AWC Adopt

## Technical Approach

Extend Rust layers with a deterministic classifier, a read-only workspace
scanner, a persisted plan store under `.awc/runtime/adopt/`, and a per-action
apply executor that reuses the existing artifact lifecycle persistence and
`ArtifactFs` compensation. Retain config and JSON schema v1.

## Architecture Decisions

| Decision | Alternatives / tradeoff | Choice and rationale |
|---|---|---|
| Classifier | Content heuristics; NLP | Pure deterministic metadata-only function (location, name, extension, size). Unit-testable, stable, no content scan. |
| Scan | Walk everything; walk governed too | Walk non-governed, non-ignored files only, using the existing `classify_path` policy plus the adopt ignored set. Read-only. |
| Plan store | In-DB rows; single JSON file | JSON document under `.awc/runtime/adopt/<plan-id>.json` (design §52 pattern). Regeneration-only; explicit actions. |
| Workspace fingerprint | File count; mtime-only | Sorted walk of path + mtime + size over non-governed, non-ignored files. Deterministic and sensitive to any change. |
| Apply semantics | All-or-nothing; per-action | Per-action with immediate precondition re-check; failure skips that action, remaining actions continue, report applied/skipped. |
| Register-existing | Reuse `create_artifact`; new use case | New application use case `register_existing_artifact` that fingerprints the CURRENT bytes of an existing `artifacts/**` file, inserts metadata + audit, and reuses lifecycle invariants (path unowned, non-empty fingerprint unique). `create` keeps its new-empty-file semantics. |
| Move-to-inbox | Delete unknowns; leave in place | Move to `inbox/` via `ArtifactFs` with compensation; never delete, never convert uncertainty into destruction. |

## Data Flow

```text
adopt scan   -> walk -> classifier -> ScanReport (read-only)
adopt plan   -> ScanReport -> plan JSON + workspace fingerprint -> .awc/runtime/adopt/<plan-id>.json
adopt apply  -> load plan -> fingerprint recheck (stale_adopt_plan)
               -> per action: precondition recheck -> execute (register | move-to-inbox)
               -> applied/skipped report
```

## Module Impact

| File | Action | Description |
|---|---|---|
| `crates/awc-core/src/infrastructure/classify.rs` | Create | Pure deterministic classifier: category + suggested action. |
| `crates/awc-core/src/domain.rs` | Modify | AdoptCandidate/ScanCategory/PlanAction/AdoptPlan models + CommandResult variants. |
| `crates/awc-core/src/error.rs` | Modify | `StaleAdoptPlan`, `AdoptPlanNotFound`, `AdoptClassification` errors; exits unchanged. |
| `crates/awc-core/src/infrastructure/adopt.rs` | Create | Workspace walk + fingerprint + plan store (load/save/hash). |
| `crates/awc-core/src/application.rs` | Modify | `scan_adopt`, `plan_adopt`, `apply_adopt`, `register_existing_artifact` use cases. |
| `crates/awc-core/src/infrastructure/sqlite.rs` | Modify | `register_existing_artifact` persistence (insert with existing path + fingerprint). |
| `crates/awctl/src/main.rs` | Modify | `adopt` subcommand tree + human/JSON views + error mapping. |
| `crates/awctl/tests/cli.rs`, `docs/*` | Modify | Contract tests and docs. |

## Interfaces / Contracts

```rust
enum ScanCategory { KnownRuntime, ManagedCandidate, TemporaryCandidate, SensitiveCandidate, Unknown, Ignored }
struct AdoptCandidate { rel_path, category, suggested_type: Option<String>, suggested_action: Action }
enum Action { Register { artifact_type: String }, MoveToInbox, Skip }
struct AdoptPlan { id, fingerprint, actions: Vec<PlanAction> }
// adopt scan [--json]; adopt plan [--json]; adopt apply <plan-id> [--json]
```

New snake_case codes: `stale_adopt_plan`, `adopt_plan_not_found`. JSON stays
`{schemaVersion:1, ok, data|error}`; exits 0/1/2/3.

## Apply Execution

1. Load plan by id (`adopt_plan_not_found` if absent).
2. Recompute workspace fingerprint; mismatch → `stale_adopt_plan`, zero actions.
3. For each action in plan order:
   - Re-check preconditions: source exists, target unowned, fingerprint unchanged.
   - Register: `register_existing_artifact(project, path, artifact_type)` — fingerprint from current bytes, insert + audit in one transaction; file NOT moved (stays where it is, path becomes both path and original_path).
   - Move-to-inbox: `ArtifactFs.move_file` into `inbox/` with compensation (on failure, move back or remove partial; report failure).
   - Record applied/skipped; continue on failure.

## Testing Strategy

| Layer | What to test | Approach |
|---|---|---|
| Unit | classifier patterns (plan/review/report/temp/sensitive/runtime/ignored), fingerprint determinism | Pure function tests over in-memory paths; sorted-walk determinism tests |
| Integration | plan save/load/hash, stale rejection, per-action skip/continue, register-existing invariants, move-to-inbox compensation | Temp workspace SQLite + `FailingFs` injection |
| CLI | `adopt scan/plan/apply` human/JSON v1, error codes, exits | Extend `crates/awctl/tests/cli.rs` |

## Slice Boundary (for sdd-tasks)

1. **Classification + scan** — classify.rs, ScanReport, walk, scan use case + tests. ~350–450 lines.
2. **Plan + persistence** — models, fingerprint, plan store, plan use case + tests. ~300–400 lines.
3. **Apply + CLI** — register_existing_artifact, apply use case, compensation, adopt CLI + views + contract tests. ~400–500 lines.

## Migration / Rollout

No schema migration: plan store lives in `.awc/runtime/adopt/` (created on
first plan). Reverting code leaves plan JSON and adopted artifacts
recoverable; no down migration.

## Open Questions

- [ ] None blocking; task planning must honor the 400-line budget with the 3-slice chain and explicit `size:exception` requests if a cohesive slice exceeds it.
