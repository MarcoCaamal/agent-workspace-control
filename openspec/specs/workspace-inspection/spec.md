# workspace-inspection Specification

## Requirements

### Requirement: Deterministic workspace status

The system MUST provide status using upward discovery and MUST report consistent state for unchanged inputs. Status MUST NOT mutate workspace state.

#### Scenario: Inspect a discovered workspace

- GIVEN a valid workspace and a nested working directory
- WHEN status runs from the nested directory
- THEN it reports workspace state
- AND no workspace files are changed

#### Scenario: Report no workspace

- GIVEN no ancestor of the working directory contains a valid workspace
- WHEN status runs
- THEN it reports workspace-not-found without creating `.awc`

### Requirement: Read-only quick diagnostics

The system MUST provide `doctor --quick` with config, database, schema, and path-integrity checks only. It MUST be read-only and deterministic for unchanged inputs.

#### Scenario: Run healthy quick diagnostics

- GIVEN a valid initialized workspace
- WHEN `doctor --quick` runs
- THEN it reports results for all four required checks
- AND it does not modify configuration or state

#### Scenario: Detect partial or unsafe state

- GIVEN a workspace with missing database state or an escaping `.awc` symlink
- WHEN `doctor --quick` runs
- THEN the corresponding check reports failure
- AND no repair is attempted
