# Architecture

AWC is a synchronous Rust workspace with a thin CLI boundary and a reusable core. The current architecture separates command rendering, application use cases, domain rules, and filesystem/SQLite infrastructure so future interfaces can reuse policy without duplicating it.

## Layered View

```text
CLI input
   │
   ▼
awctl: parse arguments, select output mode, map errors to exit codes
   │
   ▼
application: orchestrate init, inspection, and project use cases
   │
   ├── domain: typed data, identity, slug, and prefix rules
   ├── error: shared failures and exit-code mapping
   │
   ▼
infrastructure: canonical paths, TOML configuration, SQLite, hashing
```

Dependencies point inward: `awctl` depends on `awc-core`; the core does not depend on Clap, an async runtime, an agent SDK, or a specific agent host.

## Crates And Modules

| Location | Responsibility |
|---|---|
| `crates/awctl` | CLI parsing, human/JSON views, stdout/stderr behavior, process exit codes |
| `awc-core/src/application.rs` | Use-case orchestration and read/write intent |
| `awc-core/src/domain.rs` | Configuration, results, project model, UUIDv7 newtypes, slug and ID-prefix rules |
| `awc-core/src/error.rs` | User-safe errors and the `0/1/2/3` exit-code contract |
| `awc-core/src/infrastructure/paths.rs` | Upward discovery and canonical containment checks |
| `awc-core/src/infrastructure/config.rs` | TOML parsing, defaults, validation, and atomic creation |
| `awc-core/src/infrastructure/sqlite.rs` | Connection modes, migrations, schema health, and project persistence |
| `awc-core/src/infrastructure/hash.rs` | Streaming SHA-256 and exact byte-count primitive for future artifact reconciliation |

The core is synchronous. There is currently no MCP crate, server runtime, agent adapter, or background process.

## Workspace Model

AWC discovers the nearest `.awc` entry by walking from the canonical current directory toward its ancestors. The ancestor containing that entry is the workspace root.

```text
workspace root/
├── .awc/                 # Internal AWC state
│   ├── config.toml       # Versioned, user-readable configuration
│   └── state.sqlite3     # Metadata and migration ledger
├── artifacts/            # Governed directory; lifecycle is roadmap work
├── inbox/                # Governed directory; classification is roadmap work
├── tmp/                  # Governed directory; retention is roadmap work
└── trash/                # Governed directory; purge is roadmap work
```

Directory names are configurable. A missing directory field in a valid v1 configuration receives its default in memory without rewriting the file.

## Configuration And Data

Three independent version numbers exist:

| Contract | Current value | Purpose |
|---|---:|---|
| `config.toml` `schema_version` | 1 | Validates the configuration shape |
| SQLite `schema_migrations` | versions 1 and 2 | Records applied database migrations |
| JSON `schemaVersion` | 1 | Versions the public CLI output envelope |

The default configuration is:

```toml
schema_version = 1
database_file = "state.sqlite3"
artifacts_dir = "artifacts"
inbox_dir = "inbox"
tmp_dir = "tmp"
trash_dir = "trash"
```

SQLite is the source of truth for metadata. The current schema contains:

| Table | Current role |
|---|---|
| `schema_migrations` | Authoritative ledger of applied migration versions |
| `projects` | UUIDv7 identity, unique slug, name, optional external root path, status, timestamp |
| `artifacts` | Metadata foundation for future artifact commands; no public CRUD yet |
| `audit_events` | Metadata foundation for future audit behavior; no public commands yet |

Project slugs are lowercase ASCII alphanumeric strings separated by single hyphens. Derived slugs lowercase names, collapse non-alphanumeric runs, and reject an empty result. Project lists are sorted by slug. `project show` accepts a full canonical UUID or a prefix only when exactly one project matches.

## Migration Safety

`awctl init` is the only current command that creates a database and applies migrations.

- Migrations run in numeric order and each migration uses its own transaction.
- The migration ledger is authoritative; recorded migrations are not reapplied automatically.
- Re-running `init` is idempotent for a healthy workspace and can recreate a missing database.
- The schema-v2 migration rebuilds the original foundation tables only when all legacy tables are empty.
- If any legacy foundation table contains rows, migration stops before DDL and leaves the rows, schema, and ledger unchanged.
- Existing valid `config.toml` bytes, including comments and omitted defaulted fields, are preserved.

This refusal is deliberate: AWC has no safe, general conversion for manually populated legacy rows.

## Managed-Write Boundaries

| Operation | Writes allowed |
|---|---|
| `init` | `.awc/config.toml` when absent, `.awc/state.sqlite3`, and configured governed directories inside the workspace root |
| `project add` | A new row in the existing workspace database |
| `status` | None; configuration and database are opened read-only |
| `doctor --quick` | None; failures are reported, never repaired |
| `project list` / `project show` | None; database is opened read-only |

The optional project `root_path` is stored and displayed as external context. It is not canonicalized, created, inspected, or treated as a managed root by project commands.

## Key Invariants

1. AWC never uses a `.awc` or governed-directory target that canonically escapes the workspace root.
2. Read-only commands do not create, repair, migrate, or rewrite workspace state.
3. Existing valid configuration bytes are not normalized or reserialized.
4. A migration ledger entry is committed with its migration, never before it.
5. Populated legacy foundation tables are preserved rather than guessed at or destroyed.
6. Project slug collisions fail before insertion.
7. An ID prefix resolves only when exactly one project matches.
8. External project roots are metadata, not write authority.
9. JSON success contains `data`; JSON failure contains `error`; neither contains both.

## Roadmap Boundaries

The schema and hash primitive prepare for artifact governance, but they do not implement it. Artifact lifecycle, adoption, cleanup, reconciliation, full diagnostics, audit behavior, MCP, and agent adapters remain roadmap work. See the [roadmap exploration](../openspec/changes/awc-artifact-governance-adopt/exploration.md) and [product design](design/awc-product-design.md).

Canonical implemented requirements live under [`openspec/specs/`](../openspec/specs/).
