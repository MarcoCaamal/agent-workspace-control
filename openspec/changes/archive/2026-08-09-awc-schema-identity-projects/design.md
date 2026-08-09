# Design: AWC Schema, Identity, and Projects

## Technical Approach

Extend the existing synchronous layered core: typed domain values and application use cases coordinate concrete SQLite, TOML, filesystem, and hashing modules; `awctl` remains parsing/rendering only. This implements the `project-identity` and `workspace-foundation` deltas while deferring artifact lifecycle, ownership enforcement, and adoption.

## Architecture Decisions

| Decision | Options / trade-off | Decision and rationale |
|---|---|---|
| Identity and lookup | Raw strings or typed UUIDv7 newtypes | Add `ProjectId`, `ArtifactId`, and `AuditEventId` newtypes around `Uuid`; generate `Uuid::now_v7()`. SQLite stores canonical hyphenated UUID text. `project show` resolves a supplied prefix with `id LIKE prefix%` and accepts exactly one row; zero/many become typed not-found/ambiguous errors. The resolver is at the application/persistence boundary, never in CLI rendering. |
| Schema v2 | In-place alteration or destructive rebuild | Add a special v2 migration transaction. Before any DDL, count rows in all v1 foundation tables; any row returns `LegacySchemaData` and rolls back untouched. Otherwise drop v1 tables in FK-safe order, create complete v2 tables, and record ledger version 2 atomically. This avoids fabricated UUIDs and partial schemas. |
| Metadata before lifecycle | Minimal project table or future-ready concrete tables | Create full Project, Artifact, and AuditEvent columns now: UUID IDs, timestamps, project slug/name/root/status; artifact project/type/title/path/status/hash/size/last-seen; audit IDs, optional project/artifact references, event type, timestamp. Enforce slug uniqueness and FKs; do not add CRUD, lifecycle transitions, or implicit audit writes. |
| Boundaries | Providers/traits, async, or concrete synchronous modules | Keep concrete functions and `rusqlite::Connection`; add no provider traits or runtime. This follows the current architecture and preserves a small, testable v0.2 foundation. |
| Config and paths | Rewrite old config or default in memory | Keep schema 1. Add serde-defaulted `artifacts_dir`, `inbox_dir`, `tmp_dir`, and `trash_dir`; existing valid bytes are parsed but never rewritten. `init` creates/repairs all configured/defaulted directories through canonical containment validation and rejects a target or symlink escaping the workspace root. |

## Data Flow

```
awctl project add/list/show
  -> application::{add_project,list_projects,show_project}
  -> sqlite (slug conflict / prefix resolution / rows)
  -> typed CommandResult -> existing human or JSON envelope

awctl init -> config parse/defaults -> paths contained directories
           -> sqlite v1/v2 migration -> InitStatus
```

Slug derivation lowercases a name, converts non-alphanumeric runs to one `-`, trims `-`, and rejects an empty result; `--slug` bypasses derivation but is validated by the same slug rules. Both paths reject collisions. `root_path` is persisted and returned as external metadata only; these commands never write it.

`hash.rs` exposes a synchronous SHA-256-plus-size fingerprint over a reader/file: stream bytes, return lower-case 64-hex SHA-256 and the exact byte count. It performs no filesystem mutation and establishes future reconciliation semantics.

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/awc-core/Cargo.toml` | Modify | Add `uuid` (`v7`, `serde`) and `sha2`. |
| `crates/awc-core/src/{domain,application,error,lib}.rs` | Modify | Domain newtypes/models/results, project use cases, stable typed errors, exports. |
| `crates/awc-core/src/infrastructure/{sqlite,config,paths,mod}.rs` | Modify | v2 migration, config defaults, governed-directory containment, module export. |
| `crates/awc-core/src/infrastructure/hash.rs` | Create | Deterministic hash/size primitive. |
| `crates/awctl/src/main.rs` | Modify | Thin `project add/list/show` parsing and project result views. |
| `crates/awctl/tests/cli.rs` | Modify | Project JSON/human and exit-contract integration tests. |

## Interfaces / Contracts

```rust
pub struct ProjectId(pub Uuid);
pub struct ContentFingerprint { pub sha256: String, pub size: u64 }
pub struct AddProject { pub name: String, pub slug: Option<String>, pub root_path: Option<PathBuf> }
```

Existing JSON remains `{schemaVersion: 1, ok, data|error}` and exits remain 0/1/2/3. New operational error codes are snake_case (including `project_not_found`, `ambiguous_project_id`, `slug_conflict`, and `legacy_schema_data`).

## Testing Strategy

| Layer | What to Test | Approach |
|---|---|---|
| Unit | UUID/prefix/slug/hash/config defaults and contained directory repair | Module tests, including escaping symlinks and byte-identical existing config. |
| Integration | SQLite v2 shape, idempotency, rollback on populated v1 tables | Temporary databases; assert rows/schema/ledger unchanged after rejection. |
| CLI | Add/list/show human and JSON output, collisions, not-found, exits | Extend `CARGO_BIN_EXE_awctl` tests; assert one JSON document and no external-root write. |

## Threat Matrix

N/A — no routing, shell, subprocess, VCS/PR automation, executable-file classification, or process-integration boundary. Clap subcommand parsing dispatches in-process only; no shell is invoked.

## Migration / Rollout

Release order: first core schema/config/path/hash support, then CLI commands and tests. `init` migrates new and empty v1 databases to v2; populated v1 databases fail before DDL and require backup/manual resolution. Roll back binaries safely before migration; after v2, restore a pre-migration backup or retain v2 for forward recovery—never down-convert UUID identities. Review-budget risk is medium: migration, core, and CLI should be separate reviewable units under 400 changed lines.

## Open Questions

None.
