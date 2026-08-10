# Exploration: AWC Artifact Lifecycle (`awc-artifact-lifecycle`)

Change B of the v0.2 roadmap (design §118), after change A (`awc-schema-identity-projects`, archived and verified 6/6 requirements, 12/12 scenarios, 68 tests). Scope: `artifact create/show/list/archive/trash/restore/relink`, path ownership, protected paths, DB/FS compensation. Explicitly out of scope: adopt (change C), reconciliation (v0.5 doctor), cleanup/tmp lifecycle/retention, MCP, runtime adapters, work items, secrets.

## Current State

The Rust workspace is fully built and green: `cargo test --workspace` = 68 passing (58 core + 10 CLI), clippy `-D warnings` and fmt configured. HEAD is `ef929da` on `docs/awc-schema-identity-projects-06-guides`; `main` locally still points at `231f5e2` (v0.1 foundation) — see Risks for a git-state discrepancy.

- **CLI (`awctl`)**: `init`, `status`, `doctor --quick`, `project add/list/show`. Global `--json`; envelope `{schemaVersion:1, ok, data|error}` (exactly one of data/error), one newline-terminated JSON doc, snake_case error codes, exit codes **0 success / 1 operational / 2 usage / 3 workspace-not-found**. Usage errors go to stderr (clap), application errors to stdout in JSON mode.
- **Domain (`domain.rs`, 372 lines)**: `ProjectId`/`ArtifactId`/`AuditEventId` UUIDv7 newtypes, `ContentFingerprint{sha256,size}`, `Config` (schema v1 + 4 serde-defaulted governed dirs), `derive_slug`/`validate_slug`, `resolve_id_prefix` (**project-typed** errors: `ProjectNotFound`/`AmbiguousProjectId`), `CommandResult` (Init/Status/Doctor/ProjectAdded/ProjectList/ProjectShown — **no artifact variants**), and the `Project` row type. **No `Artifact`, `ArtifactType`, `ArtifactStatus`, or `PathOwnership` types exist.**
- **Errors (`error.rs`, 112 lines)**: 12 variants (Usage, WorkspaceNotFound, UnsafeStatePath, InvalidConfig, UnsupportedConfigVersion, Io, Database, ProjectNotFound, AmbiguousProjectId, SlugConflict, LegacySchemaData, InvalidSlug). Exit map: Usage=2, WorkspaceNotFound=3, rest=1. **No artifact, path-policy, or lifecycle-conflict variants.**
- **SQLite v2 (`sqlite.rs`, 486 lines)**: ledger `schema_migrations` (versions 1, 2); populated-v1 refusal (`LegacySchemaData`) before DDL. `artifacts` table exists **schema-only**:
  - `id TEXT PK`, `project_id TEXT NOT NULL REFERENCES projects(id)`, `artifact_type TEXT NOT NULL`, `title TEXT NOT NULL`, `path TEXT` (nullable), `status TEXT NOT NULL DEFAULT 'tracked'`, `sha256 TEXT`, `size INTEGER`, `last_seen_at`, `created_at`.
  - **No `updated_at` column** (design §31 has it), **no UNIQUE on path**, **`project_id NOT NULL`** (design `require_project = false` is not expressible).
- **Persistence functions**: only `insert_project`, `select_projects_by_id_prefix`. **No artifact insert/select, no audit write** (`audit_events` table exists; no `insert_audit` anywhere).
- **Application (`application.rs`, 582 lines)**: init/status/doctor_quick/add_project/list_projects/show_project. **No artifact use cases, no compensation helpers** (design §95-97 pattern unimplemented).
- **Infrastructure**: `config.rs` (parse, atomic write, byte preservation), `paths.rs` (upward discovery, `canonicalize_state_within`, `ensure_governed_dir` — reusable for artifact path validation), `hash.rs` (SHA-256 + exact size over a reader — the fingerprint primitive for create/relink).
- **Docs**: `docs/usage.md` explicitly states artifact commands are not implemented; `docs/architecture.md` marks lifecycle as roadmap; canonical design `docs/design/awc-product-design.md` is committed and tracked.
- **OpenSpec**: canonical specs `cli-contracts`, `project-identity`, `workspace-foundation`, `workspace-inspection`. The umbrella exploration lives at `openspec/changes/awc-artifact-governance-adopt/exploration.md`.

## Gaps for the Seven Commands

| Command | Gap |
|---|---|
| `artifact create` | No use case, no file creation, no governed-path derivation, no policy check, no fingerprint capture, no audit. Design §132: AWC creates the path and returns ART-ID; agent writes content afterwards. |
| `artifact show` / `list` | No `Artifact` row type, no sqlite select, no CommandResult variants, no JSON/human views. Prefix resolution is project-only today. |
| `artifact archive` | No status transition to `archived`, no optional physical move to an archive subtree (design §25 `artifacts/*/archive/`), no audit. |
| `artifact trash` / `restore` | No move to `trash/` (design §49 ACTIVE→TRASH→retention→PURGE; purge out of scope), no collision-safe trash naming, no restore-to-original-path logic, no compensation. |
| `artifact relink` | No path update, no re-hash/re-size, no ownership-conflict check on the target, no audit (`artifact.relinked`). |

Cross-cutting gaps: `ArtifactType`/`ArtifactStatus` enums, `PathOwnership` + policy module, audit event vocabulary + write path, `updated_at` maintenance, artifact prefix resolution, `[artifacts]` config keys (design §99: `require_project`, `embed_id`), and the DB default `status = 'tracked'` which is not in the design's status set (Active/Completed/Archived/Trashed, §33).

## Path Ownership and Protected-Path Policy

Design §26-27, §100-102. Nothing is implemented. Change B adds a rules module (hardcoded Rust rules per §100, no policy language):

- `enum PathOwnership { AwcManaged, AgentRuntimeManaged, UserManaged, Ignored, Unmanaged }`.
- `AwcManaged`: `.awc/**`, `artifacts/**`, `inbox/**`, `tmp/**`, `trash/**` — the only paths artifact commands may write.
- `AgentRuntimeManaged` (design §27: AWC knows, does not control — never touched): `AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/**`, `skills/**`. Note this list is OpenClaw-flavored while AWC is agent-agnostic (§5.5) — explicit product decision needed on the v0.2 set.
- `Ignored`: `.git/**`, `target/**`. `UserManaged`: `docs/**`. Everything else: `Unmanaged`.
- Path-safety pipeline (design §101): resolve relative to root → normalize → canonical containment → symlink validation (default: do not follow external symlink targets, §102) → ownership/policy. The existing `paths::canonicalize_state_within` + `ensure_governed_dir` machinery is the containment foundation to reuse; artifact paths additionally need per-artifact ownership classification and `PolicyViolation` variants (`RootArtifactForbidden`, `ProtectedPath`, `PathEscapesWorkspace`, `UnsafeDelete`).

## DB/Filesystem Transaction and Compensation Boundaries

Design §95-97 + invariant 7 (no silent mutation failure). There is no joint ACID transaction between SQLite and the filesystem; the established pattern must be applied per command:

- **create**: determine path → policy validate → write temp in the target directory → begin DB tx → insert artifact metadata → atomic rename temp→final → commit → append audit event. On error: rollback DB, remove temp. Hashing happens once over the file during create (or on a provided content buffer).
- **trash**: move file into `trash/` with a collision-safe name (open decision: flat dir with timestamp/ID prefix vs preserved relative path) → DB status update. If the DB write fails after a successful move, compensate by moving the file back; if the move fails, the DB never changed.
- **restore**: verify the original path is free (else conflict) → move back → status update; compensate on failure.
- **archive**: status update; physical move to an archive subtree only if the product decision requires it.
- **relink**: validate target ownership → update `path` (and re-hash if the target file exists) → audit.
- Crash residue (`metadata-without-file`, `file-without-metadata`, temp residue) is full-doctor territory (v0.5); v0.2 must clean its own temps on error paths and leave pre/post states consistent so later doctor can detect drift.

## Compatibility Constraints (settled by change A — do not reopen)

1. **JSON schema v1 envelope** (`schemaVersion:1`, exactly one of `data`/`error`) — locked by `cli-contracts` and reaffirmed by `project-identity`.
2. **snake_case error codes** — new codes must follow (`artifact_not_found`, `ambiguous_artifact_id`, `path_owned`, `protected_path`, `path_escape`, `artifact_status_conflict`, ...).
3. **Exit codes 0/1/2/3** — change A explicitly kept the v0.1 contract (`project-identity` spec: "exit-code 0/1/2/3 contract"). The design §89 richer map was **not** adopted; adopting it now would be a breaking delta and is not recommended.
4. **`state.sqlite3`** filename retained.
5. **Config schema v1** retained with serde-defaulted optional keys only; byte-preservation invariant (valid bytes never rewritten). Any `[artifacts]` keys must be optional under v1.
6. **External project `root_path` is metadata-only** — artifact commands must never write outside the workspace root, even for a project with a `root_path`.
7. **Migration ledger is authoritative** — adding a v3 migration extends `schema_health` automatically (`MIGRATIONS.len()`); v2's populated-table refusal pattern applies to any future destructive rebuild, not to additive v3.

## Approaches

1. **Layered 3–4 PR chain (recommended)** — mirror change A's proven structure within one SDD pipeline:
   - PR 1 — Domain + policy: `Artifact`/`ArtifactType`/`ArtifactStatus`/`PathOwnership`/policy module, artifact error variants, generic or artifact-typed prefix resolution, audit vocabulary. Pure library + unit tests.
   - PR 2 — Persistence + filesystem: artifact CRUD in sqlite, audit insert, `updated_at` maintenance (v3 migration if decided), collision-safe trash/restore moves and create temp+rename with compensation, fingerprint wiring.
   - PR 3 — Application use cases: `create_artifact`/`list_artifacts`/`show_artifact`/`archive_artifact`/`trash_artifact`/`restore_artifact`/`relink_artifact` + CommandResult variants.
   - PR 4 — CLI + integration: `artifact` subcommand tree, JSON/human views, `cli.rs` contract tests, `docs/usage.md` update.
   - Pros: each PR is independently testable (`cargo test -p awc-core` then `-p awctl`), matches the existing diff-size discipline from change A, clean layering, tests land with code.
   - Cons: artifact commands become user-visible only in PR 4; slightly more pipeline ceremony.
   - Effort: High (total ~1,900–2,700 authored lines).

2. **Vertical command slices** — PR 1: `create/show/list`; PR 2: `archive/trash/restore`; PR 3: `relink` + policy hardening.
   - Pros: user-visible value earlier; each slice is a complete command story.
   - Cons: every slice crosses domain/persistence/application/CLI at once (scaffolding duplicated, policy and compensation defined piecemeal); harder to keep each diff coherent and under budget; more churn on shared files.
   - Effort: High.

3. **Single PR** — impossible within the 400-line review budget; not viable.

**Recommendation**: Approach 1. It reuses the exact structure that made change A reviewable (primitives → foundation → use cases → CLI), keeps each PR at/below the 400-line budget only if slices are sized by sdd-tasks, and defers none of the compensation or policy logic.

### Review Workload Forecast

- Estimated authored size: **~1,900–2,700 lines** (domain/policy ≈ 700–900; persistence/fs ≈ 600–800; application ≈ 400–550; CLI + views + integration + docs ≈ 600–900).
- **3–4 chained PRs** (feature-branch-chain, child targets previous slice); `400-line budget risk: High`; `Chained PRs recommended: Yes`; delivery strategy stays `ask-on-risk`.
- Forecast guard lines for sdd-tasks: `Decision needed before apply: Yes`, `Chained PRs recommended: Yes`, `400-line budget risk: High`.

## Product / Business Decisions Required Before Proposal

The orchestrator must get user answers to these before launching `sdd-propose` (each has a recommended default):

1. **Create semantics** — create a new governed file (design §132) and return ART-ID + path, with optional initial content (`--content` or `--file`)? Register an existing file, or both? Recommended: create-new-file only; `--type` with a type-derived default subdirectory under `artifacts/` (plans, reviews, research, reports, decisions, handoffs, documentation, other); `--title` required; `--project` optional at CLI but DB currently forces NOT NULL (see 7).
2. **Lifecycle transitions** — v0.2 legal set: Active→Archived, Active→Trashed, Trashed→Active (restore), Archived→Active? Is Archived→Trashed allowed? `Completed` unreachable until v0.6 work items. Recommended: allow exactly the four above; no Completed.
3. **Archive/trash/restore paths** — archive = status-only or physical move to `artifacts/*/archive/`? trash = physical move into `trash/` (recommended, design §49) with what collision naming? restore target = original path when free, else conflict error (recommended) vs inbox? purge/retention explicitly out of v0.2.
4. **Relink rules** — manual explicit only (auto-reconciliation is v0.5). Refuse when the target path is owned by another artifact; refuse (or require `--force`) when the current file still exists at the old path? Re-hash on relink (recommended)? Target allowed anywhere in the workspace or only under `artifacts/`?
5. **Duplicate handling** — same content hash as an existing artifact: allowed (a hash match is not necessarily a move, design §35) or warned? Target path already exists on disk at create: refuse with `path_owned` (recommended). Add a UNIQUE constraint on `path` via v3 or enforce at the application layer?
6. **Protected-path scope** — confirm the §26 set (AwcManaged = `.awc` + 4 governed dirs; AgentRuntimeManaged = AGENTS.md, SOUL.md, MEMORY.md, memory/**, skills/**; Ignored = .git/**, target/**; UserManaged = docs/**) and that artifact create only writes under `artifacts/`. Hardcoded Rust rules (recommended) vs configurable.
7. **`project_id NOT NULL`** — keep require-project in v0.2 (no migration) or add a v3 migration making `project_id` nullable to honor `require_project = false` (§99)? Recommended: keep NOT NULL in v0.2, defer nullable to a later change with a v3 migration.
8. **Status naming** — DB default `'tracked'` vs design's `'active'`; migrate via v3 or accept `tracked` as the v0.2 active state? Recommended: align to `active`/`archived`/`trashed` text via v3 if a migration already exists for other reasons; otherwise document `tracked` as active.
9. **Audit** — which events in v0.2 (`artifact.created`, `.archived`, `.trashed`, `.restored`, `.relinked`) and are they mandatory? Recommended: mandatory writes in the same transaction/compensation unit as the mutation.
10. **`artifact show/list` view** — fields (id, projectId, type, title, path, status, sha256, size, lastSeenAt, createdAt, updatedAt?) and list filters (`--project`, `--type`, `--status`) + ordering (createdAt desc recommended).

## Risks

- **Git state discrepancy**: the orchestrator states change A merged to `main` at `ffe03ea9...`; that commit is **not present in the local clone** (`main` = `231f5e2`, v0.1 foundation; HEAD = `ef929da` on the docs branch, which contains all change A commits). Before proposal/apply, confirm the intended base branch — change B must branch from a point containing change A's code (HEAD qualifies).
- **Schema-v2 shape debt**: `artifacts` lacks `updated_at` and path uniqueness, defaults `status='tracked'`, and forces `project_id NOT NULL`; lifecycle work may require an additive v3 migration — decide scope early (products 7/8).
- **Compensation correctness**: crash between FS move and DB commit leaves residue; v0.2 must clean its own temps and never silently ignore a mutation failure (invariant 7).
- **Policy enforcement surface**: without an explicit protected set, create/relink could touch `AGENTS.md`/`memory/` or escape the root; the §101 pipeline must be applied to every artifact path, with symlink targets not followed.
- **Budget**: ~1,900–2,700 lines forecast guarantees a 400-line breach unless chained PRs are planned and each slice sized by sdd-tasks; ask-on-risk applies.
- **Duplicate content/path semantics** unresolved could create registry inconsistencies (two artifacts, one path) that v0.5 reconciliation must later repair — decide before spec.

## Ready for Proposal

**No — not yet.** Exploration is complete and bounded, but `sdd-propose` requires the interactive question round first: the ten product decisions above (especially create semantics, lifecycle transitions, archive/trash/restore paths, relink rules, duplicate handling, and protected-path scope), plus confirmation of the git base-branch state. The orchestrator should present the decisions with the recommended defaults and the git discrepancy to the user before launching proposal.
