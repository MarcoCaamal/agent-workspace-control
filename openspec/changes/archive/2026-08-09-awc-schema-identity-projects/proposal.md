# Proposal: AWC Schema, Identity, and Projects

## Intent

Establish identity and metadata foundations before AWC governs file lifecycle. Safely replace v0.1 placeholder tables, add deterministic project commands, and preserve shipped contracts.

## Scope

### In Scope
- UUIDv7 newtypes, deterministic unique-prefix resolution, and SHA-256 hash/size primitives.
- Defensive SQLite migration v2 for full Project/Artifact metadata; reject populated v0.1 tables rather than invent identities.
- `project add/list/show`; derive slug from name, allow `--slug`, and reject collisions. Optional external `root_path` is context only, never managed-write authority.
- Keep config schema v1 with optional defaulted governed-directory settings; `init` creates or repairs `artifacts/`, `inbox/`, `tmp/`, and `trash/`.
- Preserve `state.sqlite3`, exit codes 0/1/2/3, and snake_case JSON error codes.

### Out of Scope
- Artifact CRUD/lifecycle/relink, path-ownership enforcement, and adopt scan/plan/apply.
- WorkItems, MCP, cleanup, security, secret references, and integrations.

## Capabilities

### New Capabilities
- `project-identity`: UUIDv7 identity, prefix resolution, project metadata, slug rules, and project add/list/show behavior.

### Modified Capabilities
- `workspace-foundation`: Initialization provisions governed directories and defaulted v1 config; migration v2 establishes full metadata while refusing populated v0.1 tables.

## Approach

Add typed identity and hashing primitives to `awc-core`; rebuild foundation tables transactionally only after proving replacement is safe. Extend config with backward-compatible defaults and expose project use cases through `awctl`.

## Affected Areas

| Area | Impact | Description |
|------|--------|-------------|
| `crates/awc-core/src/{domain,application,error}.rs` | Modified | Identity, projects, results, errors |
| `crates/awc-core/src/infrastructure/{sqlite,config,hash}.rs` | Modified/New | Migration, defaults, hashing |
| `crates/awctl/src/main.rs` | Modified | Project commands and rendering |
| `crates/awc-core/Cargo.toml` | Modified | UUIDv7 and SHA-256 dependencies |

## Risks

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| Destructive schema conversion | Med | Transactional rebuild; reject populated legacy tables without mutation |
| External roots imply ownership | Med | Treat `root_path` strictly as metadata |
| Compatibility drift | Low | Preserve config, database filename, exits, and error-code casing |

## Rollback Plan

Revert binaries and config additions while preserving `state.sqlite3`. After migration v2, restore a pre-migration backup or retain v2 for forward recovery—never down-convert UUID identities.

## Dependencies

- Builds on shipped v0.1 `workspace-foundation` and `cli-contracts`.
- `awc-artifact-lifecycle` depends on this change; `awc-adopt` depends on both.

## Success Criteria

- [ ] New workspaces and empty v0.1 workspaces reach schema v2; populated v0.1 tables are rejected with zero data loss.
- [ ] Project add/list/show produce deterministic human/JSON results; derived and explicit slug collisions fail.
- [ ] Re-running `init` repairs all four governed directories without changing valid config bytes.
- [ ] Existing `state.sqlite3`, exit 0/1/2/3, and snake_case error contracts remain unchanged.
