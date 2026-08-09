//! Domain types for AWC workspaces: configuration and command results.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AwcError;

/// Supported configuration schema version. Bump only with an explicit
/// compatibility and migration decision (design: Config).
pub const CONFIG_SCHEMA_VERSION: u32 = 1;

/// Default SQLite state file name, relative to the `.awc` directory.
pub const DEFAULT_DATABASE_FILE: &str = "state.sqlite3";

/// Typed UUIDv7 identity of a project. SQLite stores the canonical
/// hyphenated text form (design: Identity and lookup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(pub Uuid);

impl ProjectId {
    /// Generates a fresh UUIDv7 project identity.
    pub fn new() -> Self {
        ProjectId(Uuid::now_v7())
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed UUIDv7 identity of an artifact (design: Identity and lookup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ArtifactId(pub Uuid);

impl ArtifactId {
    /// Generates a fresh UUIDv7 artifact identity.
    pub fn new() -> Self {
        ArtifactId(Uuid::now_v7())
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed UUIDv7 identity of an audit event (design: Identity and lookup).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AuditEventId(pub Uuid);

impl AuditEventId {
    /// Generates a fresh UUIDv7 audit-event identity.
    pub fn new() -> Self {
        AuditEventId(Uuid::now_v7())
    }
}

impl Default for AuditEventId {
    fn default() -> Self {
        Self::new()
    }
}

/// Deterministic SHA-256 plus exact byte count over a reader, the future
/// reconciliation primitive for artifacts (design: hash.rs).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContentFingerprint {
    /// Lower-case 64-hex SHA-256 digest.
    pub sha256: String,
    /// Exact number of bytes hashed.
    pub size: u64,
}

/// Versioned workspace configuration (`schema_version = 1`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub database_file: String,
}

impl Config {
    /// Default configuration for a freshly initialized workspace.
    pub fn default_config() -> Self {
        Config {
            schema_version: CONFIG_SCHEMA_VERSION,
            database_file: DEFAULT_DATABASE_FILE.to_string(),
        }
    }
}

/// A discovered workspace: its root directory and parsed configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
    pub config: Config,
}

/// Derives a canonical slug from a project name: lowercases it, collapses
/// every run of non-alphanumeric characters into a single `-`, trims leading
/// and trailing `-`, and rejects an empty result (design: slug derivation).
pub fn derive_slug(name: &str) -> Result<String, AwcError> {
    let mut slug = String::new();
    let mut separated = true;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            if separated && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            separated = false;
        } else {
            separated = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        return Err(AwcError::InvalidSlug("slug must not be empty".into()));
    }
    Ok(slug)
}

/// Validates an explicit slug against the same canonical rules used by
/// derivation: non-empty, lowercase ASCII alphanumeric or `-`, no leading,
/// trailing, or consecutive `-`.
pub fn validate_slug(slug: &str) -> Result<(), AwcError> {
    if derive_slug(slug)? != slug {
        return Err(AwcError::InvalidSlug(
            "slug must be lowercase alphanumeric with single `-` separators".into(),
        ));
    }
    Ok(())
}

/// Resolves an ID prefix deterministically against the canonical hyphenated
/// text form: exactly one match selects it, zero matches report not found,
/// and two or more report ambiguity (design: Identity and lookup). This is
/// the pure rule; the persistence layer supplies the candidate rows.
pub fn resolve_id_prefix(prefix: &str, ids: &[Uuid]) -> Result<Uuid, AwcError> {
    let mut matches = ids
        .iter()
        .copied()
        .filter(|id| id.to_string().starts_with(prefix));
    let Some(first) = matches.next() else {
        return Err(AwcError::ProjectNotFound);
    };
    if matches.next().is_some() {
        return Err(AwcError::AmbiguousProjectId);
    }
    Ok(first)
}

/// Outcome of one read-only diagnostic check (config, database, schema, path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckResult {
    pub name: &'static str,
    pub ok: bool,
    /// Human-readable detail; empty when the check passed.
    pub message: String,
}

impl CheckResult {
    pub fn ok(name: &'static str) -> Self {
        CheckResult {
            name,
            ok: true,
            message: String::new(),
        }
    }

    pub fn failed(name: &'static str, message: impl Into<String>) -> Self {
        CheckResult {
            name,
            ok: false,
            message: message.into(),
        }
    }
}

/// Read-only workspace status report (design: status reports root, config
/// version, and database/schema health).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub root: PathBuf,
    pub schema_version: u32,
    pub database_ok: bool,
    pub schema_ok: bool,
}

/// Result of a successful `init` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitStatus {
    pub root: PathBuf,
    pub schema_version: u32,
    pub database_ok: bool,
    pub schema_ok: bool,
}

/// Quick doctor report: the reported root plus one check per diagnostic, in
/// the fixed order path, config, database, schema (design: doctor --quick).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickDoctor {
    pub root: PathBuf,
    pub checks: Vec<CheckResult>,
}

/// Typed outcome of a core command, rendered by `awctl` (Phase 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Init(InitStatus),
    Status(Status),
    Doctor(QuickDoctor),
}

#[cfg(test)]
mod tests {
    use super::*;

    use uuid::{Uuid, Variant, Version};

    fn assert_uuidv7(uuid: Uuid) {
        assert_eq!(uuid.get_version(), Some(Version::SortRand));
        assert_eq!(uuid.get_variant(), Variant::RFC4122);
        assert_eq!(uuid.to_string().len(), 36, "canonical hyphenated form");
    }

    #[test]
    fn uuidv7_newtypes_carry_version_and_variant() {
        assert_uuidv7(ProjectId::new().0);
        assert_uuidv7(ArtifactId::new().0);
        assert_uuidv7(AuditEventId::new().0);
    }

    #[test]
    fn prefix_resolve_selects_the_single_matching_id() {
        let a = Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
        let b = Uuid::from_u128(0xaaaa_bbbb_cccc_dddd_eeee_ffff_0000_1111);
        let resolved = resolve_id_prefix("11112222", &[a, b]).expect("unique prefix");
        assert_eq!(resolved, a);
        // Prefixes may be split on the canonical hyphenated form as stored.
        let resolved = resolve_id_prefix("11112222-3333", &[a, b]).expect("unique prefix");
        assert_eq!(resolved, a);
    }

    #[test]
    fn prefix_resolve_zero_matches_reports_not_found() {
        let a = Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
        let err = resolve_id_prefix("9999", &[a]).unwrap_err();
        assert!(matches!(err, AwcError::ProjectNotFound));
    }

    #[test]
    fn prefix_resolve_multiple_matches_reports_ambiguity() {
        let a = Uuid::from_u128(0x1111_2222_3333_4444_5555_6666_7777_8888);
        let b = Uuid::from_u128(0x1111_aaaa_bbbb_cccc_dddd_eeee_ffff_0000);
        let err = resolve_id_prefix("1111", &[a, b]).unwrap_err();
        assert!(matches!(err, AwcError::AmbiguousProjectId));
    }

    #[test]
    fn derive_slug_lowercases_and_collapses_runs() {
        assert_eq!(derive_slug("My Cool  Project!").unwrap(), "my-cool-project");
        assert_eq!(derive_slug("ProjectX").unwrap(), "projectx");
    }

    #[test]
    fn derive_slug_trims_leading_and_trailing_dashes() {
        assert_eq!(derive_slug("!!hello!!").unwrap(), "hello");
        assert_eq!(derive_slug("---alpha---").unwrap(), "alpha");
    }

    #[test]
    fn derive_slug_rejects_empty_result() {
        assert!(matches!(derive_slug("!!!"), Err(AwcError::InvalidSlug(_))));
        assert!(matches!(derive_slug(""), Err(AwcError::InvalidSlug(_))));
    }

    #[test]
    fn validate_slug_accepts_only_canonical_form() {
        assert!(validate_slug("my-project").is_ok());
        assert!(validate_slug("abc123").is_ok());
        assert!(matches!(
            validate_slug("My-Project"),
            Err(AwcError::InvalidSlug(_))
        ));
        assert!(matches!(
            validate_slug("my--project"),
            Err(AwcError::InvalidSlug(_))
        ));
        assert!(matches!(
            validate_slug("-my"),
            Err(AwcError::InvalidSlug(_))
        ));
        assert!(matches!(
            validate_slug("my-"),
            Err(AwcError::InvalidSlug(_))
        ));
        assert!(matches!(validate_slug(""), Err(AwcError::InvalidSlug(_))));
    }

    #[test]
    fn derived_slug_of_canonical_name_round_trips() {
        assert_eq!(derive_slug("my-project").unwrap(), "my-project");
        assert!(validate_slug(&derive_slug("Some Project").unwrap()).is_ok());
    }
}
