## Exploration: awc-foundation

### Current State

`agent-workspace-control` is a greenfield foundation. The directory contains only SDD bootstrap scaffolding (`.atl/skill-registry.md`, `openspec/config.yaml`, empty `openspec/specs/` and `openspec/changes/archive/`). There is no `Cargo.toml`, no source, no test configuration, and **no Git work tree**.

Host tooling verified: rustc 1.92.0, cargo 1.92.0, rustfmt 1.8.0, clippy 0.1.92. SQLite CLI 3.53.4 present (not needed; rusqlite bundles). Current crate versions on crates.io: rusqlite 0.40.2, toml 1.1.4, clap 4.6.6, serde 1.0.229, thiserror 2.0.20. Strict TDD is disabled in config because no project test runner exists yet; `apply` rules require re-detecting testing capabilities after the workspace is created.

Product boundary: AWC is a local, deterministic, filesystem-governance **control plane** for persistent AI-agent workspaces. It is not a sandbox and does not intercept I/O; it records and governs workspace state (project identity, artifacts, audit events) under a `.awc/` state directory. v0.1 is a synchronous, trustworthy vertical slice: `awc-core` (lib) + `awctl` (bin).

### Affected Areas

- `Cargo.toml` (workspace root) — new; workspace manifest, `resolver = "2"`, members `crates/awc-core`, `crates/awctl`.
- `crates/awc-core/src/lib.rs` — new; library surface: config, state, discovery, migrations, audit, path safety.
- `crates/awctl/src/main.rs` — new; clap CLI, command dispatch, JSON/human envelope rendering, exit-code mapping.
- `.gitignore` + `git init` — new; **required for any review/delivery machinery** (repo is not a Git work tree).
- `openspec/config.yaml` — testing capabilities must be re-detected after workspace init (`test_command: cargo test --workspace`).
- No existing code is modified — this change creates everything.

### Approaches

1. **Full candidate scope as listed** — Cargo workspace, `awc-core` + `awctl`, `init`/`status`/`doctor --quick`, discovery + `.awc/`, versioned TOML config, SQLite migrations, Project/Artifact/AuditEvent foundations, JSON envelope + exit codes, path safety.
   - Pros: single coherent foundation; matches user intent; no mid-slice gaps.
   - Cons: realistic authored size ≈ 900–1400 lines (incl. tests) → **breaches the 400-line budget by 2–3×**; forces chained PRs or an explicit size exception.
   - Effort: Medium.

2. **Smallest coherent E2E slice (recommended)** — workspace + `awc-core` (config/state/discovery/migrations/audit/path-safety) + `awctl init`/`status` + envelope/exit codes; `doctor --quick` reduced to a 4-check read-only pass; Artifact exists as an empty migrated table (no lifecycle API); repair = `init` is idempotent and re-runs migrations. No CRUD beyond `init`.
   - Pros: end-to-end trust path (init → persisted state → status reads it back → JSON contract) provable in one slice; every phase of the SDD pipeline exercises real code; holds the line against artifact-lifecycle creep.
   - Cons: `doctor --quick` and Artifact table are thinner than the full description; still likely ~600–900 authored lines → likely chained into 2 PRs.
   - Effort: Low-Medium.

3. **Bare-bones slice** — workspace + `init` only (drop `status`, `doctor`, audit table).
   - Pros: smallest possible; near-certainly within budget.
   - Cons: no read-back path → cannot prove persistence correctness or the JSON envelope against a real read; weak acceptance story; proposal/spec value drops sharply.
   - Effort: Low.

### Key Ambiguities (must be resolved before/during proposal)

- **Git initialization**: repo is not a Git work tree. Recommend `git init` + `.gitignore` as part of this change; without it, PR-based review is impossible. Flag to user.
- **Workspace discovery semantics**: recommend walk-up from CWD looking for `.awc/` (git-style), bounded at filesystem root; no env override or `--workspace` flag in v0.1 (scope control). Explicitly define "not a workspace" behavior (distinct exit code).
- **`init` idempotency / repair invariant**: recommend `init` on an existing or partially-initialized workspace is a no-op repair (idempotent migrations, config rewritten from defaults only if missing) → this is the slice's repairability story; `doctor --quick` detects, `init` repairs. No separate `repair` command.
- **Migration mechanism**: use SQLite `PRAGMA user_version` (integer, monotonic) rather than a schema_migrations table — simplest deterministic option for v0.1; record decision in design.
- **Envelope contract**: `{"ok": bool, "exit_code": int, "data"|"error": {...}}`; `--json` flag for machine-readable output (default human); **document the envelope shape in the delta spec** so it is a stable contract, not an accident. Exit-code map: 0 ok, 1 operational failure, 2 usage error, 3 not-a-workspace (or similar) — propose exact map in proposal.
- **"Deterministic" definition**: determinism applies to output (stable key order, sorted rows, no wall-clock timestamps in status output; audit events carry timestamps but are ordered by id). Absolute paths only where semantically required.
- **Path safety scope**: canonicalize workspace root; `.awc/` must be a real directory inside the root — refuse a `.awc` symlink that escapes the workspace; Linux-only semantics in this slice (note Windows as future work).
- **Artifact table scope**: schema exists + migration, zero lifecycle commands. "Foundation" must not silently grow into artifact CRUD.

### Recommendation

**Approach 2 — smallest coherent E2E slice**, with `doctor --quick` as a 4-check read-only pass and Artifact as an empty migrated table. It is the smallest slice that proves the whole trust path end-to-end (init → persisted SQLite+TOML state → status read-back → JSON envelope → stable exit codes → path safety) while keeping review risk manageable. Approach 1 is the same product but over-scoped for a first PR; Approach 3 proves too little.

Because the authored size still likely exceeds 400 lines, `sdd-tasks` MUST plan chained PRs (e.g. PR1: workspace + `awc-core` config/state/migrations + `init`; PR2: `status` + envelope/exit codes + audit read; PR3: `doctor --quick` + path-safety checks + tests/polish) or the user must explicitly accept a size exception under `ask-on-risk`. Recommend the chained plan.

### Risks

- **Review-budget breach is near-certain** (est. 600–900 authored lines vs 400 budget) → plan chained PRs up front; do not discover this in apply.
- **No Git work tree** blocks all review/delivery machinery → git init must be an explicit early deliverable; needs user sign-off if they want the repo managed elsewhere.
- **Scope creep into artifact lifecycle / reconciliation / secret refs** — explicitly out of slice; the Artifact table's empty-CRUD status must be stated in the spec so later slices are additive.
- **Envelope contract drift** — without a documented envelope + exit-code map in the delta spec, `awctl` output becomes an informal contract; fix by specifying it in `sdd-spec`.
- **rusqlite bundled build time / version drift** — minor; pin versions at proposal time.
- **Testing bootstrap**: no runner today; `cargo test --workspace` + `assert_cmd`/`predicates`/`tempfile` for CLI integration tests must be established in this slice, then re-detected in `openspec/config.yaml` per apply rules.

### Ready for Proposal

**Yes** — exploration is complete and the slice boundary is defensible. The orchestrator should tell the user: (1) the repo is not a Git work tree and this change must include `git init`; (2) expected authored size will exceed the 400-line budget and chained PRs are recommended (ask-on-risk); (3) `doctor --quick` and Artifact lifecycle are intentionally thin in v0.1; (4) envelope + exit-code mapping will be codified in the delta spec.
