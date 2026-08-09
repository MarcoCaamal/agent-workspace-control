# workspace-foundation Specification

## Requirements

### Requirement: Safe workspace initialization and repair

The system MUST initialize a `.awc` workspace with versioned configuration and state. Re-running initialization MUST repair missing state and apply idempotent migrations without overwriting valid configuration.

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
