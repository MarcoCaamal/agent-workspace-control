## Exploration: AWC v0.2 Artifact Governance + Adopt

### Current State

v0.1 (`awc-foundation`, archived, verified 36/36 tests) ships a synchronous Rust workspace on branch `feature/awc-foundation-07-hygiene` (local feature-branch chain, no remote):

- **CLI (`awctl`)**: `init`, `status`, `doctor --quick` only. Clap 4, thin boundary; typed JSON envelope `{schemaVersion, ok, data|error}` (exactly one of data/error), one newline-terminated document, snake_case error codes (`workspace_not_found`, `invalid_config`, ...). Exit codes **0 success / 1 operational / 2 usage / 3 workspace-not-found**.
- **Core (`awc-core`)**: layered `domain.rs` (Config, Workspace, CheckResult, Status, CommandResult), `error.rs` (AwcError with exit-code contract), `infrastructure/{config,paths,sqlite}.rs`, `application.rs` (init/status/doctor_quick). Dependencies: rusqlite 0.32 bundled, serde, toml 0.8 — **no uuid, sha2, or thiserror yet** (design §24 lists them as initial deps; v0.1 hand-rolls Display).
- **Persistence**: `config.toml` (`schema_version = 1`, `database_file = "state.sqlite3"`), byte-preserved on valid content; SQLite migration ledger `schema_migrations(version INTEGER PRIMARY KEY)` with one v1 migration:
  - `projects(id INTEGER PK AUTOINCREMENT, name TEXT UNIQUE, created_at)`
  - `artifacts(id INTEGER PK AUTOINCREMENT, project_id INTEGER NOT NULL FK, name TEXT NOT NULL, created_at)`
  - `audit_events(id, project_id FK nullable, event, created_at)`
  - Schema only — **no lifecycle CRUD exists**, so tables are empty in practice.
- **Path safety**: upward discovery with canonical containment; `.awc` symlink accepted only when canonical target stays inside canonical root; atomic config writes (temp + rename + fsync); read-only status/doctor never repair.
- The canonical product design `docs/design/awc-product-design.md` is **untracked** (`?? docs/`).

**What must evolve for v0.2** (design §118): `project add/list/show`; `artifact create/show/list/archive/trash/restore/relink`; path ownership; protected paths; hash + size; `adopt scan/plan/apply`. Design references: entities §29-37 (Project/Artifact/ArtifactType/ArtifactStatus, identity, IDs, prefix resolution), lifecycle §47-49 (tmp, trash), adopt §55-58, classification §59, ownership §26-27, source of truth §28, policy §100, path safety §101-102, operations §72, exits §89, error model §90, SQLite/artifacts schema §91-92, compensation §95-97, config §99, invariants §116, roadmap §117-118.

### Affected Areas

- `crates/awc-core/src/domain.rs` — add Project/Artifact entities, ArtifactType/ArtifactStatus/PathOwnership enums, UUIDv7 newtypes (ProjectId/ArtifactId), ContentHash/ContentSize, prefix-resolution result types; extend CommandResult with project/artifact/adopt views.
- `crates/awc-core/src/error.rs` — new variants (NotFound, Conflict, PolicyViolation, AmbiguousId, UnsafeOperation, StalePlan...) and the exit-code map decision (see Compatibility Decisions).
- `crates/awc-core/src/infrastructure/sqlite.rs` — **migration v2 = table rebuild** for `projects` and `artifacts` (SQLite cannot alter PK type to TEXT/UUID or relax `project_id NOT NULL`); audit event vocabulary; schema_health table list update.
- `crates/awc-core/src/infrastructure/config.rs` + `domain::Config` — optional workspace/artifacts config keys (§99: dirs, require_project, embed_id) or a schema_version bump with defaults.
- New infra modules: `hash.rs` (sha2 SHA-256 + size), ownership/policy module (classification + protected paths), plan/runtime store (`.awc/runtime/`), filesystem ops for artifact create/trash/restore (atomic temp+rename).
- `crates/awc-core/src/application.rs` — project (add/list/show), artifact (create/show/list/archive/trash/restore/relink), adopt (scan/plan/apply) use cases + compensation helpers.
- `crates/awctl/src/main.rs` + `crates/awctl/tests/cli.rs` — new subcommands, views, exit-code/error-code contract tests.
- `openspec/specs/cli-contracts/spec.md` — MUST be MODIFIED via delta if exit-code map changes (breaking v0.1 contract).
- `Cargo.toml` (awc-core) — add `uuid` (v7 feature), `sha2`; optionally `thiserror`.
- Delivery note: canonical design doc is untracked — commit or explicitly version it before proposal.

### Approaches

**Split recommendation (item 3 of scope)** — v0.2 as ONE SDD change is untenable at the 400-line budget:

1. **One change, ~9–12 chained PRs** — keep the umbrella name `awc-artifact-governance-adopt`, run one pipeline, chain every subsystem.
   - Pros: single roadmap item, one spec/design cycle, one archive entry.
   - Cons: spec/design artifacts span 3 subsystems and become un-reviewable; estimated authored size **~4,000–6,000 lines** (schema rebuild + identity ≈ 1,100–1,800; lifecycle ≈ 1,600–2,400; adopt ≈ 1,200–1,800; plus CLI/rendering and tests) → guaranteed budget breach, ~10 PRs, verify spans everything; ask-on-risk forces repeated checkpoints.
   - Effort: High.

2. **Three ordered autonomous SDD changes (recommended)**:
   - **A. `awc-schema-identity-projects`** — migration v2 (UUIDv7, full Project/Artifact schema, nullable project_id, hash/size/status columns), identity deps, prefix resolution, hash+size infra, config evolution, `project add/list/show`. ~1,100–1,800 lines → 2–3 chained PRs.
   - **B. `awc-artifact-lifecycle`** — `artifact create/show/list/archive/trash/restore/relink`, path ownership + protected paths, DB/FS compensation. Depends on A. ~1,600–2,400 lines → 3–5 chained PRs.
   - **C. `awc-adopt`** — `adopt scan/plan/apply`, `.awc/runtime/` plan store, stale-plan preconditions. Depends on A+B. ~1,200–1,800 lines → 3–4 chained PRs.
   - Pros: each change has its own proposal/spec/design/verify and a reviewable diff; dependency order is clean (A→B→C); rollback per change; the existing change folder name becomes a roadmap tracker only.
   - Cons: 3× pipeline overhead; inter-change coordination (B and C state their dependency on A's spec).
   - Effort: High (total same, but bounded per review).

3. **Split inside one pipeline run** (option 1 with stricter PR discipline) — same risk as option 1, worse spec ergonomics; not recommended.

**Recommendation**: **Approach 2** — split into 3 ordered autonomous changes; if the user insists on the single umbrella name, sdd-tasks must plan ~10 chained PRs under explicit ask-on-risk acceptance.

### Compatibility Decisions Required (item 4 — must be settled before proposal)

1. **Exit codes**: current `0/1/2/3` vs design §89 `0/2/3/4/5/6/7/8/9` (design has no `1`, renumbers 3 = CONFIGURATION_ERROR, adds 4–9). Adopting the design map **breaks the shipped v0.1 cli-contracts spec** → requires a MODIFIED delta. Recommend adopting the design map as the stable v0.2 contract (pre-1.0, sole consumer is the test suite) — needs user sign-off.
2. **Error codes**: keep v0.1 snake_case (`workspace_not_found`) or move to design §87's SCREAMING_SNAKE (`ARTIFACT_NOT_FOUND`)? Separate decision from exit numbers; recommend aligning with design in the same delta.
3. **state.sqlite3 vs state.db**: design §25 layout shows `.awc/state.db`; v0.1 ships `state.sqlite3` in code + config. Since `database_file` is stored per-workspace in config, recommend **keeping `state.sqlite3`** (design is schematic); no rename churn.
4. **Schema evolution**: v0.1 `INTEGER AUTOINCREMENT` IDs + `name` vs design UUIDv7 newtypes + `slug/title/path/artifact_type/status/content_hash/content_size/last_seen_at/updated_at` + `project_id` nullable (`require_project = false`) + `Project.root_path/status`. Requires migration v2 **table rebuild** (defensive copy-transform even though tables are empty in practice). Also decide audit event vocabulary and whether `ProjectStatus`/`ArtifactStatus` include `Completed` in v0.2 (no complete command exists until v0.6 work items).
5. **Config schema**: §99 adds `[workspace]` dirs, `[cleanup]`, `[artifacts]`, `[doctor]`, `[git]`. Keep `schema_version = 1` with optional serde-default keys (backward compatible, byte-preservation preserved) vs bump to 2 with a migration. Recommend optional keys under v1 unless a hard break appears.
6. **Dependencies**: add `uuid` (v7) + `sha2`; `thiserror` optional (v0.1 hand-rolls Display — can keep).

### Product Questions for the Proposal Round (item 5 — business rules/scope, not mechanics)

1. **`artifact create`**: create a file on disk at a governed path (which type/path derivation? frontmatter? `embed_id = false`), or register an existing path?
2. **`artifact trash`/`restore`**: relocate the physical file to `trash/` (layout §25) or status-only? On restore, back to original path or inbox? (No purge in v0.2 — retention is later.)
3. **`artifact archive`**: move to `artifacts/archive/` + status Archived? Reversible in v0.2 (no `unarchive` command listed)?
4. **Lifecycle transition table**: which explicit transitions are legal in v0.2 (Active→Archived, Active→Trashed, Trashed→Active, Archived→Trashed?, Archived→Active?); is `Completed` reachable?
5. **`artifact relink`**: manual explicit (ID + new path) only in v0.2 (auto-reconciliation is v0.5 doctor). Re-hash on relink? Refuse when the target path is already owned, or when the artifact's current file still exists?
6. **Protected/ownership set**: which paths are protected in v0.2 — minimal (`.awc`, `artifacts/`, `inbox/`, `tmp/`, `trash/` as AwcManaged; `AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/`, `skills/` as AgentRuntimeManaged per §26)? Hardcoded Rust rules (§100) vs config?
7. **Workspace dirs**: does v0.2 create `artifacts/`, `inbox/`, `tmp/`, `trash/` at init, or lazily on first use?
8. **Adopt classification rules**: exact deterministic signals (§59) — filename/extension patterns for Plan/CodeReview/Report types, temporary detection, known runtime files, and **sensitive handling** (flag + skip, never register/move — no content scan until v0.5).
9. **Adopt plan preconditions**: persist plan JSON under `.awc/runtime/adopt/` (§52 pattern)? Workspace fingerprint definition for v0.2 (walk of path+mtime+size vs file count) and rejection code (`STALE_ADOPT_PLAN`?) when the workspace changed.
10. **Adopt apply semantics**: all-or-nothing vs per-action with report of applied/skipped; per-action precondition re-check before executing.
11. **`project add/list/show`**: slug required or derived? `root_path` optional (project ≠ git repo, §30)? Keyed by slug or prefix ID? Is there a "current project" in v0.2, or is project purely a label?

### Safety Invariants (item 6 — carry into specs/design)

- **No path escape**: extend the v0.1 canonical containment pipeline (§101: resolve → normalize → containment → symlink validation → ownership/policy) to all artifact/adopt paths; default do-not-follow external symlink targets (§102).
- **Preserve on uncertainty**: never convert uncertainty into deletion; inbox = conserved file with unknown classification (§60); ambiguous hash matches are reported, never auto-fixed (§35, §37).
- **Protected/runtime-managed paths**: `AgentRuntimeManaged` = AWC knows but does not control content/lifecycle (§27); never auto-touched, never classified as garbage.
- **Explicit lifecycle transitions**: no implicit status flips; every mutation failure surfaces (invariant 7).
- **Ambiguous hash matches never auto-relink**: relink requires an explicit artifact ID; auto-reconciliation deferred to v0.5 doctor (§64, §115).
- **Adopt scan/plan/apply with stale-plan preconditions**: plan hash + workspace fingerprint + selected paths + expected metadata revalidated before apply; reject when stale (§53), each plan action executed only when its preconditions still hold.
- **Compensation** (§95-97): write temp → DB tx → insert metadata → atomic rename → commit; on error rollback + remove temp; residue remains detectable (full doctor is v0.5, but v0.2 must clean its own temp on error paths).

### Recommendation

Split v0.2 into **three ordered autonomous changes** (A: schema+identity+projects, B: artifact lifecycle+ownership, C: adopt), each with its own SDD pipeline and internally chained PRs to stay at/below 400 changed lines per review. Settle the six compatibility decisions (exit codes, error codes, state filename, schema rebuild, config evolution, deps) and the product questions above in the interactive proposal round before writing specs. If the user prefers one umbrella change, that is acceptable only as an explicit size/chain exception under ask-on-risk (~10 PRs).

### Risks

- **Budget breach is certain** as one change (~4,000–6,000 lines); split is required, not optional, to protect the 400-line review budget.
- **Breaking the v0.1 cli-contracts spec** (exit-code renumbering) is a deliberate contract break — needs explicit sign-off and a MODIFIED delta with migration note.
- **Migration v2 rebuild** of projects/artifacts: low data risk (tables empty today) but must be written defensively per preserve-on-uncertainty; do not drop or destroy on unknown states.
- **Hash/content identity semantics**: hashing adds I/O cost; must be deterministic and stable for later v0.5 reconciliation — define once in the spec.
- **Adopt is brownfield and mutable**: without stale-plan guards and per-action preconditions, adopt apply can drift from scan results; the stale-plan invariant is non-negotiable.
- **Canonical design doc is untracked** (`?? docs/`); version it before proposal so the design reference is reviewable.
- **Agent-runtime path list is OpenClaw-flavored** (§26 examples) while AWC is agent-agnostic (§5.5) — the v0.2 protected set needs an explicit product decision.

### Ready for Proposal

**No — not yet.** Exploration is complete, but proposal requires the interactive question round: (1) confirm the 3-change split vs single umbrella change (ask-on-risk, ~10 PRs); (2) decide the exit-code/error-code contract break; (3) answer the 11 product questions, especially artifact create/trash/archive semantics, the protected-path set, and adopt stale-plan fingerprint; (4) approve the schema v2 rebuild and config evolution approach. The orchestrator should present the six compatibility decisions and the 11 product questions to the user before launching sdd-propose.
