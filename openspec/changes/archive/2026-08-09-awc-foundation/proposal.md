# Proposal: AWC Foundation

## Intent

Establish a trustworthy local control plane for persistent AI-agent workspaces. Prove initialization, persistence, discovery, inspection, diagnostics, machine output, and path safety without adding lifecycle management.

## Scope

### In Scope
- Synchronous Rust workspace with `awc-core` and `awctl`.
- `awctl init`, `status`, and a four-check, read-only `doctor --quick` covering config, database, schema, and path integrity.
- Upward `.awc/` discovery; versioned TOML config; SQLite migrations; minimal Project, Artifact, and AuditEvent schema foundations.
- Stable errors and exits: 0 success, 1 operational failure, 2 usage error, 3 workspace not found.
- Versioned JSON envelope containing `schemaVersion`, `ok`, and exactly one of `data` or `error`.
- Canonical-root validation and rejection of escaping `.awc` symlinks.

### Out of Scope
- MCP, Tokio, lifecycle CRUD, WorkItem, cleanup, adoption, reconciliation, secret references, integrations, and full doctor.
- Artifact APIs, cross-platform paths, sandboxing, or I/O interception.

## Capabilities

### New Capabilities
- `workspace-foundation`: Safe initialization, repair, discovery, config, migrations, and foundational records.
- `workspace-inspection`: Deterministic status and limited read-only quick diagnostics.
- `cli-contracts`: Stable human/JSON results, versioning, typed errors, and exit behavior.

### Modified Capabilities
None.

## Approach

Build deterministic filesystem and SQLite operations in `awc-core`; keep `awctl` as a thin synchronous boundary. `init` repairs missing/partial state, reruns idempotent migrations, and preserves valid configuration. `status` and `doctor --quick` discover upward without mutation.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `Cargo.toml`, `crates/awc-core/` | New | Workspace and core state services |
| `crates/awctl/` | New | CLI commands and output contracts |
| `.awc/` runtime state | New | Versioned config and SQLite database |
| `.gitignore`, `openspec/config.yaml` | Modified | Delivery hygiene and post-bootstrap test detection |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Scope creep | Med | Enforce explicit non-goals |
| Contract or migration drift | Med | Version and test persisted/output contracts |
| Unsafe path handling | Med | Canonicalize and test symlink boundaries |
| 400-line review budget exceeded | High | Ask before apply; no chain or exception is approved yet |
| No Git work tree | High | Initialize Git before review delivery |

## Rollback Plan

Before external adoption, remove the new crates/manifests and generated `.awc/` state. After adoption, revert binaries while preserving user configuration and database files for forward recovery.

## Dependencies

- Rust/Cargo toolchain and bundled SQLite support.
- Git initialization is a delivery prerequisite, not a product capability.

## Success Criteria

- [ ] Init, repair, upward discovery, status, and quick diagnostics satisfy their invariants end to end.
- [ ] JSON, errors, exits, migrations, and path-safety behavior are versioned and regression-tested.
