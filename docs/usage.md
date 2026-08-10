# Using `awctl`

`awctl` operates on the nearest AWC workspace found in the current directory or an ancestor. Human output is the default; add the global `--json` flag to any command for one newline-terminated JSON document.

## Quick Path

```bash
mkdir my-workspace
cd my-workspace
awctl init
awctl status
awctl doctor --quick
awctl project add --name "Example Project"
awctl project list
```

## Commands

| Command | Purpose | Mutates state |
|---|---|---|
| `awctl init` | Initialize or repair the current directory as a workspace | Yes |
| `awctl status` | Report configuration, database, and schema health | No |
| `awctl doctor --quick` | Run path, config, database, and schema checks | No |
| `awctl project add` | Store project metadata | Yes, database only |
| `awctl project list` | List projects in slug order | No |
| `awctl project show` | Show a project by full ID or unique prefix | No |
| `awctl artifact create` | Create a new empty governed artifact | Yes, database + file |
| `awctl artifact show` | Show an artifact by full ID or unique prefix | No |
| `awctl artifact list` | List artifacts with filters | No |
| `awctl artifact archive` | Archive an artifact (status only) | Yes, database only |
| `awctl artifact trash` | Move an artifact into governed trash | Yes, database + file |
| `awctl artifact restore` | Restore a trashed artifact to its original path | Yes, database + file |
| `awctl artifact relink` | Point an artifact at a new artifacts/ path | Yes, database only |

### Initialize

Run initialization at the directory that should be the workspace root:

```bash
awctl init
awctl init --json
```

`init` creates `.awc/config.toml`, `.awc/state.sqlite3`, and the configured governed directories when absent. Re-running it applies pending migrations and repairs a missing database or governed directory while preserving valid configuration bytes.

`init` does not search upward. It initializes the current directory. Other commands discover the nearest workspace upward.

### Inspect Status

```bash
awctl status
awctl status --json
```

Status reports the canonical workspace root, configuration schema version, database health, and schema health. A missing or unhealthy database is reported as `database: unhealthy` and `schema: unhealthy`; status does not recreate it.

### Run Quick Diagnostics

The `--quick` flag is required:

```bash
awctl doctor --quick
awctl doctor --quick --json
```

Checks run in this fixed order:

| Check | Verifies |
|---|---|
| `path` | `.awc` resolves to a directory inside the workspace root |
| `config` | `config.toml` parses and uses a supported schema version |
| `database` | The configured SQLite database opens read-only |
| `schema` | Expected tables and all migration ledger entries exist |

Quick diagnostics report failures but do not repair them. An unsafe escaping `.awc` path produces a failed path check without using the target.

### Add A Project

Only `--name` is required:

```bash
awctl project add --name "Payments API"
```

The default slug is derived as `payments-api`. To choose it explicitly:

```bash
awctl project add \
  --name "Payments API" \
  --slug payments-service
```

Explicit slugs must contain lowercase ASCII letters, digits, and single hyphen separators. Leading, trailing, repeated hyphens and duplicate slugs are rejected.

An optional external path can be stored as context:

```bash
awctl project add \
  --name "Payments API" \
  --root-path /srv/repos/payments
```

`--root-path` is metadata only. The path may be absent, and AWC does not create or write to it.

### List Projects

```bash
awctl project list
awctl project list --json
```

Results are deterministic and sorted by slug.

### Show A Project

Use the full UUID returned by `project add`, or a prefix that matches exactly one project:

```bash
awctl project show 019c4f86-1234-7abc-8def-0123456789ab
awctl project show 019c4f86
awctl project show 019c4f86 --json
```

No match is an operational error. A prefix matching multiple projects is rejected as ambiguous.

### Manage Artifacts

Artifacts are governed files that live under `artifacts/` and are tracked with
status, fingerprint (SHA-256 + size), and audit events.

```bash
# Create an empty artifact for a project (project id or unique prefix).
awctl artifact create --project 019c4f86 --type doc --title "Design notes"
awctl artifact create --project 019c4f86 --type report --title "Q3 review" --json

# Show, list, and filter.
awctl artifact show 019c4f86-1234-7abc-8def-0123456789ab
awctl artifact list
awctl artifact list --status active
awctl artifact list --type doc --json

# Lifecycle: the legal transitions are
#   active -> archived | trashed, trashed -> active, archived -> active.
awctl artifact archive 019c4f86-1234-7abc-8def-0123456789ab
awctl artifact trash 019c4f86-1234-7abc-8def-0123456789ab
awctl artifact restore 019c4f86-1234-7abc-8def-0123456789ab

# Relink: old file must be gone; target must be an unowned artifacts/ path.
awctl artifact relink 019c4f86-1234-7abc-8def-0123456789ab --path artifacts/notes-v2.md
```

Behavior notes:

- `create` always creates a new empty file at `artifacts/<artifact-id>`.
- `archive` changes metadata only; `trash` physically moves the file to a
  collision-safe `trash/<id>-<basename>` name; `restore` moves it back to the
  original path and fails if that path is occupied.
- `relink` refuses while the old file still exists, rejects an occupied
  target, and refreshes the stored SHA-256 and size from the new file.
- Empty artifacts share the empty fingerprint and are always allowed; a
  duplicate non-empty fingerprint is rejected.
- Every mutation writes an audit event in the same database transaction, and
  file/database failures are compensated (the command restores the prior
  state where possible or reports `compensation_failed`).

### Adopt An Existing Workspace

`adopt` helps onboard a brownfield workspace that was not initialized with
AWC. It never deletes and never guesses destructively.

```bash
# Classify files read-only (plan/review/report patterns, temporary,
# sensitive, runtime, unknown).
awctl adopt scan
awctl adopt scan --json

# Persist an explicit adoption plan with a workspace fingerprint.
awctl adopt plan
awctl adopt plan --json

# Apply the plan per action (stale plans are rejected).
awctl adopt apply adopt-2026-08-10-12-00-00
awctl adopt apply adopt-2026-08-10-12-00-00 --project 019c4f86 --json
```

Behavior notes:

- `scan` is strictly read-only: it classifies candidates with deterministic
  signals (filename/extension). Sensitive files (`.env*`, `*.pem`, `*key*`,
  `.ssh/**`) are flagged and never touched; runtime files (`AGENTS.md`,
  `memory/**`) are recognized and never moved.
- `plan` stores explicit actions under `.awc/runtime/adopt/` together with a
  workspace fingerprint (sorted path + mtime + size walk).
- `apply` re-checks the fingerprint first: if the workspace changed since the
  plan was created, it fails with `stale_adopt_plan` and executes nothing.
  Each action then re-checks its own preconditions; a failing action is
  reported as skipped and the remaining actions continue.
- Register actions move the candidate into `artifacts/` and register it as an
  active artifact with a mandatory project; unknown candidates are moved to
  `inbox/`; nothing is ever deleted.

## JSON Mode

The global `--json` flag may appear before or after subcommands:

```bash
awctl --json status
awctl project list --json
```

Every application-level result is exactly one JSON document followed by one newline. Success uses `data`:

```json
{"schemaVersion":1,"ok":true,"data":{"projects":[]}}
```

Failure uses `error`:

```json
{"schemaVersion":1,"ok":false,"error":{"code":"workspace_not_found","message":"no AWC workspace found in this directory or any ancestor"}}
```

Successful JSON writes no stderr. Application-level JSON errors are written to stdout for machine consumption. Argument parsing errors are produced by Clap on stderr and do not use the JSON envelope because parsing never reaches application rendering.

Project objects use this shape:

```json
{
  "id": "019c4f86-1234-7abc-8def-0123456789ab",
  "slug": "payments-api",
  "name": "Payments API",
  "status": "active",
  "rootPath": "/srv/repos/payments"
}
```

`rootPath` is omitted when it is not set.

## Exit Codes

| Code | Meaning | Examples |
|---:|---|---|
| `0` | Command completed | Initialization, query, or diagnostic report produced |
| `1` | Operational failure | Invalid config, unsafe path, database error, slug conflict, unknown/ambiguous project or artifact, illegal lifecycle transition, restore conflict, duplicate fingerprint |
| `2` | CLI usage error | Unknown command, missing `doctor --quick`, invalid argument |
| `3` | Workspace not found | No `.awc` exists in the current directory or any ancestor |

Health is data, not a separate process status: `status` may exit `0` while reporting an unhealthy database, and `doctor --quick` may exit `0` with one or more failed checks. Use the returned fields or checks when automating health decisions.

```bash
output=$(awctl status --json) || {
  code=$?
  printf 'awctl failed with exit code %s\n' "$code" >&2
  exit "$code"
}

printf '%s\n' "$output"
```

## Workspace Configuration

The configuration directory is `.awc/` at the workspace root. The default configuration is:

```toml
schema_version = 1
database_file = "state.sqlite3"
artifacts_dir = "artifacts"
inbox_dir = "inbox"
tmp_dir = "tmp"
trash_dir = "trash"
```

The four directory values are relative to the workspace root and may name nested paths whose parents already exist. Their canonical locations must remain inside the root. A valid v1 file may omit the four directory fields; AWC uses the defaults without rewriting the file.

There is currently no user-global configuration directory or environment-variable override.

The governed directories are only established boundaries in the current release. Artifact classification, retention, trash transitions, cleanup, and adoption commands are not implemented yet.

## Migration Behavior

Database migrations run only during `awctl init`:

- A new or empty legacy database advances through all migrations.
- Every migration is transactional and recorded in `.awc/state.sqlite3`.
- Re-running `init` skips ledger versions already recorded.
- A populated v0.1 foundation table causes migration refusal with no schema, data, or ledger change.
- `status` and `doctor --quick` only report incomplete schema; they never migrate it.

Before retrying a refused legacy migration, preserve the database and inspect the legacy rows. AWC intentionally provides no automatic conversion because the old schema lacks enough information for lossless mapping.

## Troubleshooting

| Symptom | Cause | Action |
|---|---|---|
| `no AWC workspace found` | No `.awc` in the current directory or ancestors | Change to the intended root and run `awctl init` |
| `database: unhealthy` | Database is missing or cannot be opened read-only | Run `awctl init` to recreate a missing database; inspect permissions for other failures |
| `schema: unhealthy` | Tables or migration ledger entries are incomplete | Run `awctl init`; if it refuses legacy data, back up and inspect that data |
| `unsafe state path` | `.awc` or a governed path escapes the root or is not a usable directory | Replace it with a real directory or a symlink whose target remains inside the root |
| `invalid workspace config` | Missing, malformed, or incomplete TOML | Correct `.awc/config.toml`; AWC will not overwrite an existing invalid file |
| `unsupported config schema_version` | The file targets another configuration contract | Use a compatible `awctl` version or migrate the config explicitly |
| `slug conflict` | Another project already uses the slug | Supply a different canonical `--slug` |
| `ambiguous project id` | The supplied UUID prefix matches multiple projects | Retry with a longer prefix or full ID |

For implementation boundaries and invariants, see [Architecture](architecture.md).
