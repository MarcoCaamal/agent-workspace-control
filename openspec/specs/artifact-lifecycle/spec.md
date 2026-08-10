# artifact-lifecycle Specification

## Purpose

Define governed artifact creation, lifecycle, queries, integrity, and observable CLI results.

## Requirements

### Requirement: Create and identify governed artifacts

The system MUST require a project, title, and type; create a new file only below `artifacts/`; assign a unique artifact ID and path; and record project, type, title, path, status, SHA-256, size, original path, created/updated/last-seen timestamps. It MUST reject an occupied or artifact-owned path and a duplicate non-empty fingerprint; multiple empty artifacts MAY share the empty fingerprint.

#### Scenario: Create a unique artifact
- GIVEN an existing project and an unoccupied permitted artifacts path
- WHEN `artifact create` supplies project, title, and type
- THEN it creates and reports an active artifact with complete metadata

#### Scenario: Reject invalid or duplicate creation
- GIVEN required input is missing, the path is occupied, or non-empty content matches another artifact
- WHEN `artifact create` runs
- THEN it fails without an artifact or file mutation

### Requirement: Query artifact metadata

The system MUST provide `artifact show` and `artifact list` in human and schema-version-1 JSON forms. Results MUST include ID, project, type, title, path, status, SHA-256, size, original path, created_at, updated_at, and last_seen_at; list MUST filter by project, type, and status and order by `created_at` descending deterministically. Errors MUST use snake_case codes and exit codes 0/1/2/3.

#### Scenario: List filtered artifacts
- GIVEN artifacts spanning projects, types, and statuses
- WHEN `artifact list` uses filters
- THEN it returns only matching complete records in deterministic newest-first order

#### Scenario: Show an unknown artifact
- GIVEN no artifact matches the requested ID prefix
- WHEN `artifact show --json` runs
- THEN it emits the established failed JSON envelope and operational exit

### Requirement: Enforce lifecycle and relink transitions

The system MUST permit only active→archived, active→trashed, archived→active, and trashed→active. Archive MUST change status only. Trash MUST physically move the file into collision-safe governed trash; restore MUST return it to its unoccupied original path. Relink MUST be manual, require the old file absent and an unowned `artifacts/` target, and refresh SHA-256 and size. Completed and archived→trashed MUST be rejected.

#### Scenario: Apply legal lifecycle transitions
- GIVEN an artifact in each legal source status
- WHEN its matching archive, trash, restore, or relink command runs
- THEN its status and physical location match the defined transition

#### Scenario: Reject a lifecycle or restore conflict
- GIVEN an illegal source status or an occupied original restore path
- WHEN the transition command runs
- THEN it fails without changing artifact metadata or files

### Requirement: Couple mutations, audit, and compensation

Each successful create, archive, trash, restore, or relink MUST create its corresponding audit event. DB and filesystem mutations MUST succeed as one compensated operation: failure MUST restore the pre-command state where possible, clean command-created temporary files, and report failure; the system MUST NOT silently report partial success.

#### Scenario: Record a successful mutation
- GIVEN a valid artifact mutation request
- WHEN the mutation completes
- THEN its metadata/filesystem result and audit event are both durable

#### Scenario: Compensate a failed mutation
- GIVEN a DB or filesystem step fails during a file-changing mutation
- WHEN the command handles the error
- THEN it restores its prior state or reports the uncompensated failure without success

### Requirement: Exclude deferred artifact capabilities

The system MUST NOT implement adopt, purge, retention, reconciliation, MCP, runtime adapters, work items, or secrets as part of artifact lifecycle.

#### Scenario: Request a deferred capability
- GIVEN a lifecycle command invocation requests a deferred capability
- WHEN validation runs
- THEN it rejects the request without lifecycle mutation
