# Tasks: AWC Adopt

## Review Workload Forecast

Estimated changed lines: ~1,300–1,600 total (A 380–450, B 300–400, C 420–500, D 120–180)

Decision needed before apply: Yes
Chained PRs recommended: Yes
Chain strategy: pending
400-line budget risk: High

Chain strategy pending: orchestrator/user decision required before apply (feature-branch-chain recommended, mirroring change B).

### Work Units

| Unit | Goal | Focused test | Harness | Rollback |
|------|------|--------------|---------|----------|
| 1 | Classifier + scan | cargo test -p awc-core | Pure fn + temp workspace | Revert classify.rs + scan use case |
| 2 | Plan + persistence | cargo test -p awc-core | Temp workspace `.awc/runtime/` | Revert adopt.rs plan store |
| 3 | Apply + CLI | cargo test --workspace | Temp workspace, FailingFs | Revert application.rs + main.rs |
| 4 | Docs + final gates | cargo test --workspace; clippy; fmt | Manual smoke scan/plan/apply | Revert docs/*.md |

## Phase 1: Classification and Scan

- [x] 1.1 RED: classifier unit tests in `infrastructure/classify.rs`: plan/review/report patterns, temporary detection, sensitive flag+skip, runtime recognition, ignored exclusion, unknown
- [x] 1.2 GREEN: pure `classify(path, size) -> (ScanCategory, Option<SuggestedAction>)` in `infrastructure/classify.rs`
- [x] 1.3 RED: `scan_adopt` walk in `application.rs`: read-only over non-governed non-ignored files; zero mutation asserted
- [x] 1.4 GREEN: ScanReport model + walk + scan use case (no fs writes)

## Phase 2: Plan and Persistence

- [x] 2.1 RED: workspace fingerprint determinism tests (sorted path+mtime+size walk; change detection)
- [x] 2.2 GREEN: fingerprint walk + `AdoptPlan` model in `infrastructure/adopt.rs`
- [x] 2.3 RED: plan save/load/hash under `.awc/runtime/adopt/<plan-id>.json`; missing plan → `adopt_plan_not_found`
- [x] 2.4 GREEN: `plan_adopt` use case: persist explicit actions + fingerprint; regeneration-only

## Phase 3: Apply and CLI

- [x] 3.1 RED: `apply_adopt` stale rejection: fingerprint mismatch → `stale_adopt_plan`, zero actions
- [x] 3.2 RED: per-action precondition re-check: source missing → skipped, remaining actions continue
- [x] 3.3 GREEN: `register_existing_artifact` use case + sqlite persistence: fingerprint from current bytes, path=original_path, audit, unowned + unique non-empty fingerprint invariants
- [x] 3.4 GREEN: move-to-inbox action with ArtifactFs compensation (failure leaves no mutation)
- [x] 3.5 RED: CLI contracts in `crates/awctl/tests/cli.rs`: `adopt scan/plan/apply` JSON v1, snake_case errors, exits 0/1/2/3
- [x] 3.6 GREEN: `adopt` subcommand tree + human/JSON v1 views in `main.rs`

## Phase 4: Docs and Final Gates

- [ ] 4.1 Update `docs/usage.md`: adopt scan/plan/apply, classification, stale plans
- [ ] 4.2 Update `docs/architecture.md`: adopt flow, fingerprint, per-action apply
- [ ] 4.3 Gate: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` (17 pre-existing baseline), `cargo fmt --check`
- [ ] 4.4 Manual smoke: scan → plan → apply in temp brownfield workspace; record results
