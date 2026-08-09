# Delta for workspace-foundation

## ADDED Requirements

### Requirement: Defensive schema-v2 migration

The system MUST migrate new and empty v0.1 workspaces to schema version 2 with complete Project, Artifact, and AuditEvent metadata. It MUST retain the `state.sqlite3` filename. It MUST reject a migration when a v0.1 foundation table contains manually populated data and MUST leave that data unchanged.

#### Scenario: Migrate an empty v0.1 workspace

- GIVEN v0.1 foundation tables contain no user data
- WHEN initialization applies schema migration v2
- THEN schema v2 metadata is available in `state.sqlite3`

#### Scenario: Refuse populated legacy data

- GIVEN a v0.1 foundation table contains manually populated data
- WHEN initialization attempts migration v2
- THEN it fails without destructive conversion or data mutation

### Requirement: Backward-compatible governed-directory configuration

The system MUST retain configuration schema version 1 and MUST default omitted governed-directory fields. It MUST govern `artifacts/`, `inbox/`, `tmp/`, and `trash/` as configured or defaulted directories.

#### Scenario: Load a v1 configuration without directory fields

- GIVEN a valid v1 configuration omits governed-directory fields
- WHEN initialization loads it
- THEN it uses the default governed directories without changing the schema version

## MODIFIED Requirements

### Requirement: Safe workspace initialization and repair

The system MUST initialize a `.awc` workspace with versioned configuration, `state.sqlite3`, and the governed `artifacts/`, `inbox/`, `tmp/`, and `trash/` directories. Re-running initialization MUST repair missing state and governed directories, apply idempotent migrations, and preserve valid configuration bytes.

(Previously: Initialization created versioned configuration and state, repaired state, and applied migrations.)

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
