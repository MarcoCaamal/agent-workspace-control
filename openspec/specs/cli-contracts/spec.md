# cli-contracts Specification

## Requirements

### Requirement: Stable command result contracts

The system MUST support human-readable and JSON results. Every JSON result MUST contain `schemaVersion` and `ok`, plus exactly one of `data` or `error`; its envelope shape and fields MUST remain stable for a schema version.

#### Scenario: Emit a successful JSON result

- GIVEN a command completes successfully with JSON output selected
- WHEN it emits its result
- THEN the envelope has `schemaVersion`, `ok: true`, and `data`
- AND it has no `error` field

#### Scenario: Emit a failed JSON result

- GIVEN a command fails with JSON output selected
- WHEN it emits its result
- THEN the envelope has `schemaVersion`, `ok: false`, and `error`
- AND it has no `data` field

### Requirement: Typed failures and exit codes

The system MUST use exit code 0 for success, 1 for operational failure, 2 for usage error, and 3 for workspace-not-found. It MUST report failures without state mutation unless initialization was invoked.

#### Scenario: Reject invalid command usage

- GIVEN a command invocation has invalid arguments
- WHEN the CLI validates the invocation
- THEN it reports a usage error and exits 2

#### Scenario: Return workspace-not-found

- GIVEN `status` or `doctor --quick` cannot discover a workspace
- WHEN the command runs
- THEN it reports workspace-not-found and exits 3
- AND it leaves the filesystem unchanged
