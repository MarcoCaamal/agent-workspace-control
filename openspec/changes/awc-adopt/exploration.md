# Exploration: AWC Adopt (`awc-adopt`)

Change C of the v0.2 roadmap (design §55–59), after change A (`awc-schema-identity-projects`) and change B (`awc-artifact-lifecycle`) are complete, verified, and merged to `main` at `a9f1479`. Scope: `adopt scan/plan/apply` for brownfield workspace onboarding. Explicitly out of scope: purge, cleanup/retention, reconciliation (v0.5 doctor), MCP, runtime adapters, work items, secrets.

## Current State (after A + B)

- **CLI (`awctl`)**: `init`, `status`, `doctor --quick`, `project add/list/show`, `artifact create/show/list/archive/trash/restore/relink`. Clap 4, typed JSON envelope `{schemaVersion:1, ok, data|error}`, snake_case error codes, exits 0/1/2/3.
- **Core (`awc-core`)**: `domain.rs` (ArtifactStatus/Artifact/PathOwnership/ContentFingerprint, prefix rules, transition table), `error.rs` (stable artifact/path/lifecycle/migration/compensation codes), `application.rs` (7 artifact use cases with DB/filesystem compensation), `infrastructure/{sqlite,artifacts,hash,paths}.rs`.
- **Persistence**: schema v3 (canonical `active|archived|trashed`, `updated_at`/`original_path`, partial unique indexes on non-NULL `path` and `sha256 WHERE size > 0`), mandatory audit events, ledger-authoritative migrations.
- **Path policy**: fixed ownership classes (AwcManaged/AgentRuntimeManaged/UserManaged/Ignored/Unmanaged); lifecycle writes restricted to `artifacts/**` and `trash/**`; canonical containment with symlink rejection everywhere.
- **ArtifactFs**: injectable filesystem primitives (create_temp/rename/move_file/remove_file/exists) with `OsFs` and test `FailingFs`; compensating sequences documented in `docs/architecture.md`.
- **Fingerprint**: streaming SHA-256 + exact size (`hash::fingerprint_file`).
- **Governed dirs**: `artifacts/`, `inbox/`, `tmp/`, `trash/` created at init and validated for containment.
- **Tests**: 94 passing (82 core lib + 12 CLI integration); Clippy baseline 17 pre-existing errors on `origin/main`, zero new.

## Gaps for Adopt

| Feature | Gap |
|---|---|
| `adopt scan` | No workspace walk/classification exists. Nothing reads user files outside governed dirs today. |
| `adopt plan` | No plan model, no plan persistence under `.awc/runtime/adopt/`, no workspace fingerprint concept. |
| `adopt apply` | No plan execution, no per-action precondition re-check, no stale-plan rejection. |
| Classification | No deterministic classifier (location/name/extension/known signatures/frontmatter/config/known dirs per design §59). |
| Sensitive handling | No flag-and-skip rule; must never register/move sensitive candidates; no content scan until v0.5. |

## Design References (canonical product design)

- §55–58: `adopt scan` (read-only classification), `adopt plan` (produces an adopt plan), `adopt apply ADOPT-ID` (executes only explicit plan actions).
- §59: classification philosophy — deterministic signals only; a heuristic may produce a suggestion; never convert uncertainty into deletion.
- §60: `inbox/` = conserved file whose final classification is unknown (adopt should propose moving unknown candidates to inbox, never delete).
- §52 (runtime store pattern): plan JSON under `.awc/runtime/adopt/`.
- §53 (stale-plan preconditions): plan hash + workspace fingerprint + selected paths + expected metadata revalidated before apply; reject when stale.
- §95–97: compensation for file-moving actions.

## Compatibility Constraints (must preserve)

1. JSON schema v1 envelope; exactly one of `data`/`error`.
2. snake_case error codes; new codes follow (`stale_adopt_plan`, `adopt_plan_not_found`, ...).
3. Exit codes 0/1/2/3.
4. `state.sqlite3` filename.
5. Config schema v1 — no new config keys unless explicitly approved.
6. External project `root_path` metadata-only.
7. Artifact lifecycle rules unchanged (transition table, unique paths, duplicate non-empty fingerprint rejection, audit events, compensation).
8. Path policy unchanged: adopt must never write outside governed dirs; unknown candidates go to `inbox/` only; sensitive candidates flagged and skipped.

## Approaches

1. **Layered 3-slice chain (recommended)** — mirror B's proven structure within one SDD pipeline:
   - PR 1 — Classification + scan: deterministic classifier (pure, unit-tested), read-only workspace walk producing a classified report. ~350–450 lines.
   - PR 2 — Plan model + persistence: plan JSON under `.awc/runtime/adopt/`, workspace fingerprint (path+mtime+size walk), plan load/save with hash. ~300–400 lines.
   - PR 3 — Apply: per-action precondition re-check, stale-plan rejection, compensation for moves (register as artifact via existing lifecycle use cases; move unknown to inbox), CLI subcommands and views. ~400–500 lines.
2. **Vertical slices** (scan+plan+apply per file class) — more churn on shared classifier/plan code; not recommended.
3. **Single PR** — impossible within the 400-line budget; not viable.

**Recommendation**: Approach 1, with sizes tuned by sdd-tasks to stay at/below 400 changed lines per slice; expect at least one `size:exception` decision (B's history shows cohesive slices can exceed the budget by 40–140 lines and the maintainer has accepted them when justified).

## Product Decisions Required Before Proposal

1. **Classification rules**: exact deterministic signals — filename/extension patterns for Plan (`*plan*.md`, `*plan*.txt`), CodeReview (`*review*.md`, `pr-*`), Report (`*report*`), temporary detection (`*.tmp`, `*.bak`, `~*`, `*~`, `result.json` as temporary?), known runtime files (AGENTS.md, SOUL.md, MEMORY.md, memory/**, skills/**), and sensitive handling (`.env*`, `*.pem`, `*secret*`, `*key*`, `.ssh/**`, credentials — flag + skip, never register/move; no content scan).
2. **Scan output**: pure report (no mutation) with categories: Known runtime, Managed candidates (register as artifact), Temporary candidates, Sensitive candidates, Unknown, Ignored. Suggested default: scan is read-only; each candidate gets a suggested action (register as `artifacts/` via existing create semantics vs move to `inbox/`).
3. **Plan semantics**: `adopt plan` persists the scan suggestions as an explicit plan under `.awc/runtime/adopt/<plan-id>.json`; each action is explicit (register as artifact type X / move to inbox / skip). Plan is editable only by regeneration (no interactive editing in v0.2).
4. **Workspace fingerprint**: definition for stale detection — walk of path + mtime + size for all non-governed, non-ignored files (deterministic, sorted). Rejection code `stale_adopt_plan` when the workspace changed after the plan was created.
5. **Apply semantics**: per-action with report of applied/skipped; each action re-checks its precondition immediately before executing (file still present, path still unowned, fingerprint unchanged). All-or-nothing is NOT recommended (a single failure should not block the whole adoption).
6. **Register semantics**: adopt apply that registers a candidate as an artifact uses the existing `create_artifact`-style lifecycle (project required? adopt needs a target project or a default). Move-to-inbox uses the existing governed path machinery with compensation.
7. **Known runtime files**: never touched by adopt (consistent with AgentRuntimeManaged policy); they are reported as recognized, never classified as garbage or moved.
8. **Ignored set**: reuse `.git/**`, `target/**` from the existing policy; add `node_modules/**`, `dist/**`, `.venv/**`? (needs product sign-off).

## Risks

- **Budget breach is likely** (~1,200–1,800 lines total across the change, per the umbrella exploration); the 3-slice chain is required, and at least one `size:exception` is probable.
- **Stale-plan drift**: without per-action precondition re-checks, apply can drift from scan results; the stale-plan invariant is non-negotiable.
- **Classifier false positives**: a deterministic heuristic can misclassify; never convert uncertainty into deletion (inbox is the conservative sink). Sensitive candidates must never be registered or moved.
- **Brownfield mutations**: adopt apply mutates user files; compensation and no-mutation-on-error are required for every move.
- **No content scan**: classification is metadata-only (location/name/extension/size); do not read file contents for classification in v0.2.

## Ready for Proposal

**No — not yet.** Exploration is complete and bounded, but `sdd-propose` requires the interactive question round: (1) confirm the classification rule set (esp. temporary and sensitive patterns), (2) decide apply semantics (per-action recommended), (3) approve the workspace-fingerprint definition and `stale_adopt_plan` rejection, (4) decide adopt's target-project requirement for registered artifacts, (5) approve the ignored-set extension. The orchestrator should present these to the user before launching proposal.
