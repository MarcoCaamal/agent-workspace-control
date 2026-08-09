# project-identity Specification

## Purpose

Define deterministic project identity and metadata without granting authority over external project roots.

## Requirements

### Requirement: Typed UUIDv7 identities and prefix resolution

The system MUST assign UUIDv7 identities to new projects and MUST expose project identity through a typed project identifier. It MUST resolve an ID prefix only when exactly one project matches; zero matches MUST report not found and multiple matches MUST report ambiguity. The system MUST provide SHA-256 hash and size metadata primitives for future artifact use.

#### Scenario: Resolve a unique project prefix

- GIVEN one project matches an ID prefix
- WHEN a project command resolves that prefix
- THEN it selects that project deterministically

#### Scenario: Reject an ambiguous or unknown prefix

- GIVEN a prefix matches zero or multiple projects
- WHEN a project command resolves the prefix
- THEN it fails without selecting a project

### Requirement: Create projects with deterministic slugs

The system MUST create a project from a name and derive its slug unless an explicit slug is supplied. It MUST reject a derived or explicit slug that collides with an existing project.

#### Scenario: Add a project with a derived slug

- GIVEN a workspace and an unused project name
- WHEN `project add` runs without a slug
- THEN it persists and reports the project with its derived slug

#### Scenario: Reject a slug collision

- GIVEN a project already uses a derived or explicit slug
- WHEN `project add` requests that slug
- THEN it fails without creating another project

### Requirement: Query project metadata safely

The system MUST provide deterministic `project list` and `project show` results in human-readable and JSON forms. A supplied `root_path` MUST be stored only as optional external context metadata and MUST NOT authorize managed writes outside the AWC workspace root. Project command failures in JSON mode MUST retain the existing schema-version-1 envelope, snake_case error code, and exit-code 0/1/2/3 contract.

#### Scenario: Show an externally rooted project

- GIVEN a project has an external `root_path`
- WHEN `project show` runs
- THEN it reports the path as metadata
- AND it performs no managed write at that path

#### Scenario: Return a stable JSON failure

- GIVEN a JSON project command cannot find its target
- WHEN the command fails
- THEN stdout contains the established failed JSON envelope with a snake_case error code
- AND it exits using the established failure contract
