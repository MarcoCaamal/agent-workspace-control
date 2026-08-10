# Delta for artifact-lifecycle

## ADDED Requirements

### Requirement: Register an existing governed file as an artifact

The system MUST support registering an EXISTING file under `artifacts/**` as an artifact (used by `adopt apply`), in addition to `artifact create`'s new-empty-file semantics. Registration MUST require a target project, compute the fingerprint (SHA-256 + exact size) from the CURRENT bytes of the existing file, and store the file's current path as both `path` and `original_path` with status `active`. Registration MUST preserve all lifecycle invariants: the target path MUST be unowned, non-empty duplicate fingerprints MUST be rejected (empty files exempt), an audit event MUST be written, and any failure MUST leave no mutation.

#### Scenario: Register an existing governed file

- GIVEN an unowned existing file under `artifacts/**` and a target project
- WHEN the file is registered
- THEN an active artifact exists with fingerprint and size from the current bytes, path equal to original_path, and a matching audit event

#### Scenario: Registration honors ownership and fingerprint rules

- GIVEN the target path is owned by another artifact or the non-empty fingerprint duplicates an existing artifact
- WHEN registration is attempted
- THEN it fails without file or metadata mutation
