# Design: AWC Foundation

Implement a synchronous Rust workspace: `awc-core` owns local state; `awctl` parses and renders. It satisfies the three delta specs while excluding lifecycle CRUD, MCP, runtimes, and process integration.

## Technical Approach

`init` creates/repairs and migrates; `status` and `doctor --quick` share discovery but never repair. `awc-core` has no clap, Tokio, or SDK/runtime dependency.

## Architecture Decisions

| Decision | Options / trade-off | Choice and rationale |
|---|---|---|
| Boundary | traits / concrete code | Concrete core functions; add a trait only for a second implementation. |
| Layout | one crate / core + CLI | `awc-core` domain/application/infrastructure plus thin `awctl`, isolating CLI/runtime dependencies. |
| Persistence | ad-hoc SQL / ledger | Transactional, ordered migrations recorded in `schema_migrations(version INTEGER PRIMARY KEY)`. |
| Config | rewrite / versioned TOML | `schema_version = 1`, `database_file = "state.sqlite3"`; validate but preserve valid bytes; reject unknown versions. |
| JSON | maps / typed structs | Declaration-order serde structs: `schemaVersion`, `ok`, then exactly `data` or `error`; no timestamps or maps. |

## Modules and Data Flow

`domain.rs` defines `Workspace`, `Config`, `CheckResult`, `Status`, and `AwcError`; `application.rs` exposes `init`, `status`, and `doctor_quick`; `infrastructure/{paths,config,sqlite}.rs` implements containment, TOML, and rusqlite. `awctl/main.rs` uses clap synchronously and renders results.

```text
CLI args -> awctl parser -> awc-core application -> paths/config/sqlite
                         <- typed result/error  <- filesystem/SQLite
                         -> deterministic renderer -> stdout/stderr + exit
```

Discovery canonicalizes the start, walks canonical ancestors nearest-first, and for each `.awc` uses `symlink_metadata`, canonicalizes root/state, then accepts only `canonical_state.starts_with(canonical_root)`. Missing state continues; invalid/escaping state fails without use. `init` validates existing state or creates under the canonical target root. Linux is Tier 1.

## Initialization, Recovery, and Read-only Behavior

Create directory, atomically write new config, open SQLite, then migrate transactionally. On config failure remove only an empty directory created in this invocation. After config commit, DB/migration failure remains detectable; do not overwrite or roll back valid config. Later `init` recreates DB/resumes migrations. Invalid config, unknown versions, and unsafe paths never repair.

Migrations create the ledger then minimal `projects`, `artifacts`, and `audit_events` tables with keys/timestamps/foreign keys. Schema only: **no lifecycle CRUD, APIs, or implicit records**. Status reports root, config version, and DB/schema health; quick doctor returns config/database/schema/path checks. Both are read-only.

## Interfaces / Contracts

```rust
pub enum AwcError { Usage(String), WorkspaceNotFound, UnsafeStatePath, InvalidConfig(String), UnsupportedConfigVersion(u32), Io(std::io::Error), Database(rusqlite::Error) }
pub enum CommandResult { Init(InitStatus), Status(Status), Doctor(QuickDoctor) }
// exit: success=0; Usage=2; WorkspaceNotFound=3; all other AwcError=1
```

Success is `{ "schemaVersion": 1, "ok": true, "data": ... }`; failure replaces `data` with `{ "code", "message" }`. JSON is one newline-terminated document; human errors use stderr. Clap maps to `Usage`; core never imports clap.

## File Changes

| File | Action | Description |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | Create | Workspace and pinned crate graph. |
| `crates/awc-core/Cargo.toml`, `src/{lib,domain,application,error}.rs` | Create | Core contracts and use cases. |
| `crates/awc-core/src/infrastructure/{paths,config,sqlite}.rs` | Create | Local adapters and migrations. |
| `crates/awctl/Cargo.toml`, `src/main.rs` | Create | clap boundary, renderers, exits. |
| `crates/{awc-core,awctl}/tests/` | Create | Core and CLI contracts. |
| `.gitignore`, `openspec/config.yaml` | Modify | Ignore runtime state; refresh detected Cargo test commands after bootstrap. |

## Testing Strategy

| Layer | What to test | Approach |
|---|---|---|
| Unit | config, error/exit, JSON shape | Exact JSON and temp inputs. |
| Integration | init/repair/migrations/discovery | Temp filesystem/SQLite; unchanged config bytes and expected tables. |
| Linux path safety | nearest, internal, escaping symlink | Escaping discovery/init fails without target access. |
| CLI | 0/1/2/3 and read-only commands | Invoke binary; assert output/envelope and unchanged metadata. |

## Threat Matrix

| Boundary | Applicability | Design response / RED tests |
|---|---|---|
| Documentation-like paths | N/A — no executable classification | None. |
| Git repository selection | N/A — no Git behavior | None. |
| Commit state | N/A — no Git behavior | None. |
| Push state | N/A — no network/VCS behavior | None. |
| PR commands | N/A — no PR/process behavior | None. |

## Migration / Rollout

No external migration: versions start at 1. Before adoption remove binaries/manifests; after it preserve config/database for future recovery. Git initialization and PR slicing are delivery prerequisites/risks, not approved work; ask on 400-line risk before apply.

## Open Questions

None blocking. Future config/schema versions require explicit compatibility and migration decisions.
