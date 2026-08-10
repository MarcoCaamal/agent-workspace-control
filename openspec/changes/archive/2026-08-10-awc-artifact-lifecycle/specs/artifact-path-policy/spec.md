# artifact-path-policy Specification

## Purpose

Define non-configurable workspace ownership and path-safety boundaries for artifact operations.

## Requirements

### Requirement: Enforce contained artifact paths

The system MUST resolve paths against the workspace root, normalize them, verify canonical containment, and reject a symlink path or target that escapes the root. Artifact create and relink MUST target only unowned, non-symlink paths under `artifacts/`; external project root paths are metadata only and MUST NOT authorize writes.

#### Scenario: Accept a contained artifact target
- GIVEN an unowned non-symlink target under `artifacts/`
- WHEN create or relink validates it
- THEN it accepts the target as a governed artifact path

#### Scenario: Reject an escape
- GIVEN a path, symlink, or external project root that resolves outside the workspace
- WHEN an artifact command validates it
- THEN it rejects the request without a filesystem or metadata mutation

### Requirement: Apply fixed ownership and protected paths

The system MUST classify `.awc/**`, `artifacts/**`, `inbox/**`, `tmp/**`, and `trash/**` as governed; `AGENTS.md`, `SOUL.md`, `MEMORY.md`, `memory/**`, and `skills/**` as protected; `.git/**` and `target/**` as ignored; and `docs/**` as user-managed. Artifact commands MUST NOT write protected, ignored, user-managed, unmanaged, or another artifact's path.

#### Scenario: Reject a protected or non-artifact target
- GIVEN a target in a protected, ignored, user-managed, unmanaged, or non-artifacts governed path
- WHEN create or relink validates it
- THEN it rejects the target with a snake_case policy error

#### Scenario: Reject another artifact's path
- GIVEN a governed artifacts path belongs to a different artifact
- WHEN create or relink requests it
- THEN it fails without changing either artifact
