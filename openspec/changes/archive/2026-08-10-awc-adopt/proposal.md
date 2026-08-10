# Proposal: awc-adopt

## Problem

AWC can govern workspaces it initializes, but existing (brownfield) workspaces
have no onboarding path. Users with agent workspaces full of plans, reviews,
reports, and loose files cannot adopt them into AWC governance without manual
`artifact create` calls, and there is no safe way to discover, classify, plan,
and apply adoption with certainty that nothing is lost or modified by accident.

## Goals

1. Provide `adopt scan` — a read-only classification report of a brownfield
   workspace using deterministic signals only (location, name, extension).
2. Provide `adopt plan` — persist an explicit per-candidate action plan under
   `.awc/runtime/adopt/<plan-id>.json`.
3. Provide `adopt apply` — execute plan actions one at a time, re-validating
   each precondition immediately before executing, reporting applied/skipped,
   and never reporting success for an action that was not executed.
4. Protect plans against workspace drift with a deterministic workspace
   fingerprint and `stale_adopt_plan` rejection.
5. Never delete, never guess destructively, and never touch sensitive or
   agent-runtime files.

## Non-Goals

Purge, cleanup/retention, reconciliation (v0.5 doctor), MCP, runtime adapters,
work items, secrets handling beyond flag-and-skip, and content scanning of
candidate files. Classification is metadata-only (path/name/extension/size).

## User-Approved Decisions

- **Classification rules (deterministic signals)**:
  - Plan candidates: `*plan*` in filename with `.md`/`.txt`.
  - CodeReview candidates: `*review*`, `pr-*` with `.md`/`.txt`.
  - Report candidates: `*report*` with `.md`/`.txt`.
  - Temporary candidates: `*.tmp`, `*.bak`, `~*`, `*~`.
  - Known runtime files: `AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/**`,
    `skills/**` — recognized, never touched.
  - Sensitive candidates: `.env*`, `*.pem`, `*secret*`, `*key*`,
    `.ssh/**`, credential-named files — flagged and skipped; never
    registered, moved, or read.
  - Unknown: everything else.
  - Ignored: `.git/**`, `target/**`, `node_modules/**`, `dist/**`, `.venv/**`.
- **adopt scan**: read-only; reports Known runtime / Managed candidates /
  Temporary candidates / Sensitive candidates / Unknown / Ignored. Suggested
  action per candidate: register as artifact (with target type) or move to
  inbox; sensitive and runtime candidates always skip.
- **adopt plan**: persists the scan suggestions as explicit actions under
  `.awc/runtime/adopt/<plan-id>.json`; regeneration-only, no interactive
  editing.
- **adopt apply**: per-action semantics. Each action re-checks its
  preconditions immediately before executing (file still present, target
  path unowned, fingerprint unchanged, plan not stale); reports
  applied/skipped; a single failure does not block remaining actions.
- **Stale-plan protection**: workspace fingerprint = deterministic sorted
  walk (path + mtime + size) over non-governed, non-ignored files. Apply
  revalidates the fingerprint and rejects with `stale_adopt_plan` when the
  workspace changed after the plan was created.
- **Mandatory project**: candidates registered as artifacts require a target
  project (`--project`), consistent with the artifact lifecycle's mandatory
  project ownership.
- **Register semantics (spec delta)**: adopt registers EXISTING files as
  artifacts, which differs from `artifact create`'s new-empty-file semantics.
  The fingerprint comes from the existing file. The proposal flags a delta to
  the artifact-lifecycle spec to cover registration of existing governed
  files (path already exists, fingerprint computed from current bytes, status
  active) without relaxing any lifecycle invariant.
- **Move-to-inbox**: unknown candidates proposed for `inbox/` are moved with
  compensation (no mutation on error), preserving the file bytes.

## Delivery Approach

Three chained slices within one SDD pipeline (feature-branch chain, mirroring
the proven structure of change B):

1. **Classification + scan** — pure deterministic classifier (unit-tested),
   read-only workspace walk, scan report. ~350–450 lines.
2. **Plan model + persistence** — plan JSON under `.awc/runtime/adopt/`,
   workspace fingerprint, load/save with hash. ~300–400 lines.
3. **Apply + CLI** — per-action precondition re-check, stale rejection,
   compensation, adopt subcommands with human/JSON v1 views. ~400–500 lines.

Expected total ~1,200–1,800 authored lines; at least one `size:exception` is
probable and will be requested explicitly per slice rather than assumed.

## Compatibility

Preserve JSON schema v1 (`schemaVersion`, exactly `data` xor `error`),
snake_case error codes (adding `stale_adopt_plan`, `adopt_plan_not_found`),
exit codes 0/1/2/3, `state.sqlite3`, config schema v1, metadata-only external
project roots, and all artifact lifecycle invariants (transition table,
unique paths, duplicate non-empty fingerprint rejection, audit events,
compensation).

## Dependencies

Depends on `awc-schema-identity-projects` (A) and `awc-artifact-lifecycle`
(B), both merged to `main` at `a9f1479`.
