# Contributing To AWC

AWC is an early Rust project with safety-sensitive filesystem and migration behavior. Keep changes focused, prove mutation boundaries with tests, and separate implemented behavior from roadmap design.

## Development Setup

Install a Rust toolchain that supports Rust 2024 edition (Rust 1.85 or newer), then clone and build the workspace:

```bash
git clone https://github.com/MarcoCaamal/agent-workspace-control.git
cd agent-workspace-control
cargo build --workspace
```

The workspace contains:

| Crate | Role |
|---|---|
| `awc-core` | Domain rules, use cases, configuration, paths, SQLite, and hashing |
| `awctl` | CLI parsing, rendering, and exit-code behavior |

Read [Architecture](docs/architecture.md) before changing path, configuration, migration, or persistence behavior. Canonical implemented requirements are under [`openspec/specs/`](openspec/specs/).

## Quality Checks

Run the complete local gate before opening a pull request:

```bash
cargo test --workspace
cargo check --workspace --all-targets
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

During development, target one crate or test when useful:

```bash
cargo test -p awc-core
cargo test -p awctl --test cli
```

Tests that exercise filesystem state must use isolated temporary directories and clean up after themselves. Add contract tests for CLI output, stderr/stdout routing, JSON shape, and exit codes when those surfaces change.

## Change Guidelines

- Preserve read-only behavior for `status`, `doctor --quick`, `project list`, and `project show`.
- Treat path containment, migration refusal, and byte-preserving configuration as public safety contracts.
- Keep CLI concerns in `awctl`; keep reusable policy and use cases in `awc-core`.
- Document only behavior that is implemented and tested. Label product-design features as roadmap work.
- Update tests and public documentation in the same change as behavior.

## Commits

Use Conventional Commits with a concise imperative subject:

```text
feat: add project filtering
fix: reject escaping artifact path
docs: clarify migration refusal
test: cover ambiguous project prefixes
```

Keep each commit a reviewable work unit. Include its tests and directly related documentation rather than splitting proof from implementation.

## Review Size

Aim for no more than roughly 400 changed lines per review when practical. Split larger work by coherent behavior or layer, not by arbitrary file boundaries. If changes depend on one another, use a clearly ordered series of pull requests so each review remains independently understandable.

Safety-critical migrations or filesystem operations should be smaller still: reviewers need to verify preconditions, mutation order, rollback behavior, and tests without reconstructing multiple features at once.

## Pull Requests

A pull request should:

- Explain the user-visible outcome and why it is needed.
- Identify mutation and compatibility risks, especially paths, schemas, JSON, and exit codes.
- State what is intentionally out of scope.
- Include focused tests for success, refusal, and no-mutation failure paths.
- Pass all commands in [Quality Checks](#quality-checks).
- Update public docs when commands or guarantees change.
- Avoid unrelated refactors and generated build output.

For roadmap context, consult the [artifact governance and adoption exploration](openspec/changes/awc-artifact-governance-adopt/exploration.md) and the broader [product design](docs/design/awc-product-design.md). Roadmap documents do not override current code and canonical specifications.
