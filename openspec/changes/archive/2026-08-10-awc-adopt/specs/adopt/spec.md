# Delta for adopt

## ADDED Requirements

### Requirement: Scan and classify a brownfield workspace read-only

The system MUST provide `adopt scan` that walks non-governed, non-ignored files of the workspace and classifies each candidate using deterministic metadata-only signals (location, name, extension, size). Classification MUST be read-only: no file is created, moved, registered, deleted, or modified. The report MUST assign every candidate exactly one of: KnownRuntime, ManagedCandidate (with a suggested artifact type), TemporaryCandidate, SensitiveCandidate, Unknown, or Ignored. Known runtime files (`AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/**`, `skills/**`) MUST be recognized and never proposed for mutation. Sensitive candidates (`.env*`, `*.pem`, `*secret*`, `*key*`, `.ssh/**`, credential-named files) MUST be flagged and skipped; they MUST never be registered, moved, or read. Temporary candidates (`*.tmp`, `*.bak`, `~*`, `*~`) MUST be reported without proposed registration. Ignored paths (`node_modules/**`, `dist/**`, `.venv/**` in addition to `.git/**` and `target/**`) MUST be excluded from classification.

#### Scenario: Scan classifies known patterns

- GIVEN a workspace containing `adopt-plan.md`, `review-pr-13.md`, `q3-report.md`, `AGENTS.md`, `.env`, `backup.tmp`, and `notes.md`
- WHEN `adopt scan` runs
- THEN it reports Plan, CodeReview, and Report candidates with suggested actions, recognizes `AGENTS.md` as known runtime, flags `.env` as sensitive (skip), reports `backup.tmp` as temporary, and lists `notes.md` as unknown — with zero mutation

#### Scenario: Scan never touches files

- GIVEN a brownfield workspace with arbitrary files
- WHEN `adopt scan` runs
- THEN the bytes, mtimes, and paths of every file are unchanged

#### Scenario: Ignored paths are excluded

- GIVEN a workspace with `node_modules/`, `dist/`, `.venv/`, `.git/`, and `target/` trees
- WHEN `adopt scan` runs
- THEN no candidate inside those trees is reported

### Requirement: Persist an explicit adopt plan with a workspace fingerprint

The system MUST provide `adopt plan` that persists the scan suggestions as an explicit plan document under `.awc/runtime/adopt/<plan-id>.json`. Each plan action MUST be explicit: register-as-artifact with target type and target project, or move-to-inbox, or skip. The plan MUST record a deterministic workspace fingerprint computed as a sorted walk of path + mtime + size over non-governed, non-ignored files, plus a plan identity/hash. Plan creation MUST be regeneration-only: existing plans are never edited in place.

#### Scenario: Plan persists scan suggestions

- GIVEN a completed `adopt scan` report
- WHEN `adopt plan` runs
- THEN a plan document exists under `.awc/runtime/adopt/` containing explicit actions and the workspace fingerprint

#### Scenario: Plan identity is deterministic

- GIVEN the same workspace state
- WHEN `adopt plan` runs twice
- THEN both plans carry the same workspace fingerprint and identical suggestions

### Requirement: Apply an adopt plan per action with stale protection

The system MUST provide `adopt apply <plan-id>` that executes plan actions one at a time. Before executing each action, it MUST re-validate the action's preconditions: the plan is not stale, the source file still exists, the target path is unowned, and the fingerprint is unchanged. A stale plan (workspace fingerprint differs from the recorded one) MUST be rejected with `stale_adopt_plan` before any action executes. Each executed action MUST report applied or skipped; a failure of one action MUST NOT block the remaining actions. Register-as-artifact actions MUST reuse artifact lifecycle registration (existing-file registration per the artifact-lifecycle delta) with a mandatory target project. Move-to-inbox actions MUST move the file into `inbox/` with compensation: on error, no mutation remains and failure is reported.

#### Scenario: Apply registers a candidate artifact

- GIVEN a plan with a register-as-artifact action for `adopt-plan.md` and a target project
- WHEN `adopt apply <plan-id>` runs and preconditions hold
- THEN the file is registered as an active artifact under `artifacts/` with fingerprint from the current bytes, an audit event is written, and the action reports applied

#### Scenario: Apply rejects a stale plan

- GIVEN a plan created for a workspace whose fingerprint has since changed (a file added, removed, or modified)
- WHEN `adopt apply <plan-id>` runs
- THEN it fails with `stale_adopt_plan` and executes no action

#### Scenario: Apply re-checks per-action preconditions

- GIVEN a plan with two actions where the first action's source file was deleted after planning
- WHEN `adopt apply <plan-id>` runs
- THEN the first action reports skipped (precondition failed) and the second action still executes if its preconditions hold

#### Scenario: Move-to-inbox compensates on error

- GIVEN a plan with a move-to-inbox action whose target move fails
- WHEN `adopt apply <plan-id>` runs
- THEN no file mutation remains, the action reports failure, and remaining actions continue

### Requirement: Expose adopt results in compatible CLI forms

The system MUST render `adopt scan`, `adopt plan`, and `adopt apply` results in human and JSON schema-v1 forms. JSON MUST use the established envelope (`schemaVersion:1`, exactly `data` or `error`). Errors MUST use snake_case codes (`stale_adopt_plan`, `adopt_plan_not_found`, ...) and exit codes 0/1/2/3.

#### Scenario: JSON adopt output

- GIVEN a workspace with classified candidates
- WHEN `adopt scan --json` runs
- THEN it emits one newline-terminated JSON document with `schemaVersion:1`, `ok:true`, and categorized data

#### Scenario: Missing plan is an operational error

- GIVEN no plan with the requested id exists
- WHEN `adopt apply <missing-id> --json` runs
- THEN it emits the failed envelope with `adopt_plan_not_found` and exit 1

### Requirement: Exclude deferred capabilities

The system MUST NOT implement purge, cleanup/retention, reconciliation, MCP, runtime adapters, work items, or secrets handling beyond flag-and-skip as part of adopt.

#### Scenario: Deferred capability is not exposed

- GIVEN the adopt command surface
- WHEN a deferred capability is requested
- THEN the request is rejected without any workspace mutation
