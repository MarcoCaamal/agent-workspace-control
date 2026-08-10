# Design: AWC Artifact Lifecycle

## Technical Approach

Extend Rust layers: lifecycle/policy rules in `awc-core`, SQLite/filesystem adapters, and Clap rendering in `awctl`. Retain config and JSON schema v1.

## Architecture Decisions

| Decision | Alternatives / tradeoff | Choice and rationale |
|---|---|---|
| Lifecycle model | Free-form strings; add Completed | `ArtifactStatus::{Active,Archived,Trashed}` with approved edges only. Archive updates DB only and remains reversible. |
| Migration/indexes | Rebuild table; application-only checks | Migration v3: `tracked`→`active`, backfill `updated_at`/`original_path` from `created_at`/`path`, then partial unique indexes on non-NULL `path` and `sha256 WHERE size > 0`. ALTER/indexes preserve v2 rows; duplicates or unknown status fail without rewriting ownership. |
| Cross-resource mutations | DB-first commit; filesystem-first commit | Hold one SQLite transaction while the filesystem primitive runs, write artifact plus audit in that transaction, then commit. On an error, roll back DB and compensate the filesystem. This is compensating consistency, **not cross-resource ACID**. |
| Artifact target | User-selected create path; derived stable path | `create` derives `artifacts/<artifact-id>` (new empty file); `relink --path` is the only explicit target. This removes create-path injection while preserving a strict, useful relink escape hatch. |
| Path policy | Configurable rules; string-prefix checks | Hard-coded `PathOwnership` plus lexical normalization, canonical parent/target containment, and `symlink_metadata` checks. Never follow a file symlink; reject any symlink or canonical escape. Existing contained directory validation remains the base primitive. |

## Data Flow

```text
Clap -> application -> domain validation/policy -> SQLite transaction
                         |                         + audit event
                         +-> fs temp/create | rename/move | fingerprint
```

`Artifact` has UUIDv7 id, `ProjectId`, type/title, current/original paths, status, fingerprint, and timestamps. Inputs are create, show, filtered list, transition id, and relink path. Prefixes resolve exactly-one; list orders `created_at DESC, id DESC`.

Create validates project and unowned target, creates a same-directory temp empty file, fingerprints it, inserts metadata/audit, atomically renames temp, then commits; failure removes temp/final and rolls back. Trash verifies active/source and collision-free `trash/<id>-<basename>`, moves, updates `path/status/updated_at`, writes audit, commits; commit failure moves back. Restore verifies trashed, original target free, reverses that sequence. Relink requires old current file absent; validates an existing, non-symlink, unowned `artifacts/` file, fingerprints it immediately before DB update, then updates path/fingerprint/size/last-seen/updated plus audit. Archive updates status/updated plus audit in one DB transaction.

Crash between filesystem mutation and commit can leave final/temp/moved residue; commands attempt compensation and never report success. Keep uncompensated residue for future reconciliation/doctor observability and report `AwcError::CompensationFailed`; no cleanup/purge is introduced.

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/awc-core/src/domain.rs` | Modify | Artifact/status/type/ownership models, views, transitions, prefix rule. |
| `crates/awc-core/src/error.rs` | Modify | Stable artifact/path/lifecycle/migration/compensation errors; retain 0/1/2/3 exits. |
| `crates/awc-core/src/application.rs` | Modify | Seven use cases and compensated sequences. |
| `crates/awc-core/src/infrastructure/{sqlite,paths,hash}.rs` | Modify | v3 migration, repository/audit operations, policy/filesystem primitives, file fingerprinting. |
| `crates/awctl/src/main.rs` | Modify | `artifact` parser tree, human/JSON artifact views, error-code mapping. |
| `crates/awc-core/src/infrastructure/artifacts.rs` | Create | Injectable filesystem operations for failure/compensation tests. |
| `crates/awctl/tests/cli.rs`, `docs/{usage,architecture}.md` | Modify | Contract coverage and user-facing lifecycle documentation. |

## Interfaces / Contracts

```rust
enum ArtifactStatus { Active, Archived, Trashed }
enum PathOwnership { AwcManaged, AgentRuntimeManaged, UserManaged, Ignored, Unmanaged }
// Artifact commands: create --project --title --type; show <id>; list [--project --type --status];
// archive|trash|restore <id>; relink <id> --path <relative-path>
```

Protected: `AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/**`, `skills/**`; ignored: `.git/**`, `target/**`; user-managed: `docs/**`; governed: `.awc/**`, `artifacts/**`, `inbox/**`, `tmp/**`, `trash/**`. Only unowned non-symlink `artifacts/**` targets are writable. New snake_case operational codes include `artifact_not_found`, `ambiguous_artifact_id`, `path_owned`, `protected_path`, `path_escape`, `artifact_status_conflict`, `restore_conflict`, and `duplicate_fingerprint`; JSON remains `{schemaVersion:1,ok,data|error}`.

## Testing Strategy

| Layer | What to test | Approach |
|---|---|---|
| Unit | transition table, ownership/canonical containment, errors, filters/order, hash | Core tests including protected/ignored/symlink/empty-fingerprint cases. |
| Integration | v3 upgrade, indexes, CRUD/audit coupling, create/trash/restore/relink compensation | Temp workspace SQLite tests with injectable failing filesystem primitives. |
| CLI | parser requirements, human/JSON v1 fields/errors/exits | Extend `crates/awctl/tests/cli.rs`; smoke each command manually (no configured E2E layer). |

## Threat Matrix

| Boundary | Applicability | Design response / RED tests |
|---|---|---|
| Documentation-like paths | N/A — paths are not executable classification | No execution boundary. |
| Git repository selection | N/A — no Git invocation | No subprocess. |
| Commit state | N/A — no VCS mutation | No subprocess. |
| Push state | N/A — no VCS mutation | No subprocess. |
| PR commands | N/A — no PR integration | No subprocess. |

## Migration / Rollout

`init` alone applies v3 transactionally and retains `state.sqlite3`/config v1. Back up before upgrade; on duplicate paths or invalid legacy status, fail unchanged and require operator remediation. No down migration; reverting code leaves additive data recoverable.

## Open Questions

- [ ] None; later task planning must honor the 400-line budget with 3–4 chained slices: domain/policy, persistence/filesystem, application, CLI/docs.
