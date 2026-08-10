# Delta for workspace-foundation

## MODIFIED Requirements

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

#### Scenario: Apply the additive lifecycle migration

- GIVEN a schema-v2 workspace with legacy artifact statuses or paths
- WHEN initialization migrates it
- THEN legacy `tracked` status becomes `active`, required timestamps exist, and paths are unique

#### Scenario: Reject conflicting migrated paths

- GIVEN legacy artifact rows contain duplicate non-null paths
- WHEN the additive migration runs
- THEN it fails without silently discarding or rewriting artifact ownership
