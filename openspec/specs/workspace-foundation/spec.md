# workspace-foundation Specification

## Requirements

### Requirement: Safe workspace initialization and repair

The system MUST initialize a `.awc` workspace with versioned configuration, `state.sqlite3`, and the governed `artifacts/`, `inbox/`, `tmp/`, and `trash/` directories. Re-running initialization MUST repair missing state and governed directories, apply idempotent migrations, and preserve valid configuration bytes.

#### Scenario: Initialize a new workspace

- GIVEN a directory without `.awc`
- WHEN initialization runs in that directory
- THEN versioned configuration and state exist
- AND Project, Artifact, and AuditEvent schema is available

#### Scenario: Repair partial state without configuration loss

- GIVEN a workspace with valid configuration and missing database state
- WHEN initialization runs again
- THEN missing state is restored and migrations complete
- AND the valid configuration content is unchanged

#### Scenario: Repair missing governed directories

- GIVEN a valid workspace is missing one or more governed directories
- WHEN initialization runs again
- THEN all four governed directories exist
- AND valid configuration bytes are unchanged

### Requirement: Upward workspace discovery and path containment

The system MUST discover a workspace by searching current and ancestor directories for `.awc`. It MUST reject `.awc` symlinks whose canonical target escapes the workspace root.

#### Scenario: Discover from a nested directory

- GIVEN a valid workspace and a descendant working directory
- WHEN workspace discovery runs from the descendant
- THEN it selects the nearest ancestor workspace

#### Scenario: Refuse an escaping state symlink

- GIVEN `.awc` is a symlink outside its workspace root
- WHEN initialization or discovery validates the workspace
- THEN it fails without using the symlink target

### Requirement: Defensive schema-v2 migration

The system MUST migrate new and empty v0.1 workspaces to schema version 2 with complete Project, Artifact, and AuditEvent metadata. It MUST retain the `state.sqlite3` filename. It MUST reject a migration when a v0.1 foundation table contains manually populated data and MUST leave that data unchanged. It MUST apply an additive lifecycle migration that canonicalizes existing artifact statuses to `active`, `archived`, or `trashed`, adds lifecycle-required timestamps, and enforces unique artifact paths without changing config schema version 1 or project ownership requirements.

(Previously: schema v2 established metadata but did not define lifecycle status canonicalization, timestamps, or unique paths.)

#### Scenario: Migrate an empty v0.1 workspace

- GIVEN v0.1 foundation tables contain no user data
- WHEN initialization applies schema migration v2
- THEN schema v2 metadata is available in `state.sqlite3`

#### Scenario: Refuse populated legacy data

- GIVEN a v0.1 foundation table contains manually populated data
- WHEN initialization attempts migration v2
- THEN it fails without destructive conversion or data mutation

#### Scenario: Canonicalize legacy artifact lifecycle state

- GIVEN a schema-v2 workspace whose artifact rows use the legacy `tracked` status
- WHEN initialization applies the lifecycle migration
- THEN those artifacts become `active` and lifecycle timestamps are populated

#### Scenario: Refuse unsafe lifecycle alignment

- GIVEN a schema-v2 workspace with duplicate non-NULL artifact paths or duplicate non-empty fingerprints
- WHEN initialization applies the lifecycle migration
- THEN it fails without schema, row, or ledger mutation

#### Scenario: Enforce unique artifact paths after migration

- GIVEN the lifecycle migration has been applied
- WHEN an artifact insert uses a path already owned by another artifact
- THEN the insert fails without mutation

### Requirement: Backward-compatible governed-directory configuration

The system MUST retain configuration schema version 1 and MUST default omitted governed-directory fields. It MUST govern `artifacts/`, `inbox/`, `tmp/`, and `trash/` as configured or defaulted directories.

#### Scenario: Load a v1 configuration without directory fields

- GIVEN a valid v1 configuration omits governed-directory fields
- WHEN initialization loads it
- THEN it uses the default governed directories without changing the schema version
