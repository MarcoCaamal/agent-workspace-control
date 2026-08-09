# Agent Workspace Control

Agent Workspace Control (AWC) is a local-first control plane for persistent AI-agent workspaces. Its `awctl` CLI establishes a safe workspace boundary, stores local metadata in SQLite, and provides deterministic human-readable and JSON output.

The current release is an early foundation. It initializes and inspects workspaces and records project metadata; artifact lifecycle and workspace adoption are roadmap features.

## Current Capabilities

| Area | Available now |
|---|---|
| Workspace setup | Initialize or repair `.awc` state and governed directories |
| Discovery | Find the nearest workspace from the current directory or an ancestor |
| Inspection | Read-only `status` and `doctor --quick` checks |
| Projects | Add, list, and show project metadata with UUIDv7 identities |
| Automation | Stable, newline-terminated JSON envelopes and documented exit codes |
| Persistence | Versioned TOML configuration and transactional SQLite migrations |

Not yet implemented: artifact create/archive/trash/restore, adoption, cleanup, reconciliation, full diagnostics, MCP, portable skills, and runtime adapters. See [Roadmap](#roadmap).

## Safety Model

AWC is designed to preserve state and reject uncertain writes:

- `status`, `doctor --quick`, `project list`, and `project show` are read-only.
- `init` writes only workspace state under `.awc/` and configured governed directories inside the workspace root.
- `.awc` and governed-directory symlinks are accepted only when their canonical targets remain inside the workspace root.
- Existing valid configuration bytes are preserved. New configuration is written through a same-directory temporary file and atomic rename.
- SQLite migrations run in ordered transactions. Migration from populated legacy foundation tables is refused without changing the data.
- A project's `root_path` is metadata only. It does not grant AWC permission to write to that path.

AWC does not sandbox an agent or prevent arbitrary filesystem access by other processes. Current guarantees apply to writes performed by `awctl` itself.

## Build And Install

AWC requires a Rust toolchain that supports Rust 2024 edition (Rust 1.85 or newer).

```bash
git clone https://github.com/MarcoCaamal/agent-workspace-control.git
cd agent-workspace-control
cargo build --release
```

The binary is written to `target/release/awctl`. To install it with Cargo from the checkout:

```bash
cargo install --path crates/awctl
```

## Quick Start

Run `init` from the directory that should become the workspace root:

```bash
mkdir my-agent-workspace
cd my-agent-workspace
awctl init
awctl doctor --quick
awctl project add --name "My Project"
awctl project list
```

Initialization creates this default layout:

```text
my-agent-workspace/
├── .awc/
│   ├── config.toml
│   └── state.sqlite3
├── artifacts/
├── inbox/
├── tmp/
└── trash/
```

Commands can run from any descendant directory; AWC searches upward and uses the nearest `.awc` workspace.

## CLI Examples

```bash
# Inspect the nearest workspace without changing it
awctl status
awctl doctor --quick

# Store an external repository path as metadata only
awctl project add \
  --name "Payments API" \
  --slug payments-api \
  --root-path /srv/repos/payments

# Resolve a project by full UUID or a unique UUID prefix
awctl project show 019c4f86

# Request one machine-readable JSON document
awctl project list --json
```

See the [usage guide](docs/usage.md) for command details, JSON shapes, exit codes, configuration, migrations, and troubleshooting.

## Status

AWC is pre-1.0 and under active development. The implemented surface is intentionally small:

- Workspace foundation and inspection are implemented and tested.
- Project identity and metadata commands are implemented and tested.
- Artifact and audit tables exist as metadata foundations, but no public artifact or audit commands exist yet.
- Linux is the primary development platform. Cross-platform support has not yet been established by release automation.

## Roadmap

Future work is described in the [artifact governance and adoption exploration](openspec/changes/awc-artifact-governance-adopt/exploration.md). The broader direction is captured in the [product design](docs/design/awc-product-design.md); it is aspirational and should not be read as a list of current commands.

Planned areas include artifact lifecycle, safe workspace adoption, hygiene and reconciliation, MCP integration, and agent-runtime integrations.

## Documentation

- [Usage](docs/usage.md)
- [Architecture](docs/architecture.md)
- [Canonical specifications](openspec/specs/)
- [Contributing](CONTRIBUTING.md)

## Contributing

Development setup, quality checks, commit conventions, and pull request expectations are in [CONTRIBUTING.md](CONTRIBUTING.md).
