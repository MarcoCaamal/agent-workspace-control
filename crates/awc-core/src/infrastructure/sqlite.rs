//! SQLite state backend: transactional, ordered migrations with a version ledger.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};
use uuid::Uuid;

use crate::domain::{Artifact, ArtifactId, ArtifactStatus, AuditEventId, Project, ProjectId};
use crate::error::AwcError;

/// Ledger table recording every applied migration version.
pub const MIGRATIONS_TABLE: &str = "schema_migrations";

/// Ordered migrations; `index + 1` is the version number. Schema only — no
/// lifecycle CRUD, APIs, or implicit records (design: Persistence/ledger).
/// Version 2 is guarded: it runs only when every v0.1 foundation table is
/// empty (see [`refuse_populated_legacy`]).
const MIGRATIONS: &[&str] = &[
    // v1: minimal placeholder projects/artifacts/audit_events with keys,
    // timestamps, and FKs. Replaced by v2 once proven empty.
    "CREATE TABLE projects (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL UNIQUE,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE artifacts (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        project_id INTEGER NOT NULL REFERENCES projects(id),
        name TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE audit_events (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        project_id INTEGER REFERENCES projects(id),
        event TEXT NOT NULL,
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );",
    // v2: complete Project/Artifact/AuditEvent metadata. Drop v1 tables in
    // FK-safe order (children first) and create v2 atomically (design:
    // Schema v2, Metadata before lifecycle).
    "DROP TABLE IF EXISTS audit_events;
    DROP TABLE IF EXISTS artifacts;
    DROP TABLE IF EXISTS projects;
    CREATE TABLE projects (
        id TEXT PRIMARY KEY,
        slug TEXT NOT NULL UNIQUE,
        name TEXT NOT NULL,
        root_path TEXT,
        status TEXT NOT NULL DEFAULT 'active',
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE artifacts (
        id TEXT PRIMARY KEY,
        project_id TEXT NOT NULL REFERENCES projects(id),
        artifact_type TEXT NOT NULL,
        title TEXT NOT NULL,
        path TEXT,
        status TEXT NOT NULL DEFAULT 'tracked',
        sha256 TEXT,
        size INTEGER,
        last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
        created_at TEXT NOT NULL DEFAULT (datetime('now'))
    );
    CREATE TABLE audit_events (
        id TEXT PRIMARY KEY,
        project_id TEXT REFERENCES projects(id),
        artifact_id TEXT REFERENCES artifacts(id),
        event_type TEXT NOT NULL,
        occurred_at TEXT NOT NULL DEFAULT (datetime('now'))
    );",
    // v3: additive lifecycle alignment. Guarded by [`migrate_v3`]: it runs
    // only when every legacy artifact row can be canonicalized (unknown
    // statuses, duplicate non-null paths, and duplicate non-empty
    // fingerprints refuse the migration). Canonicalizes `tracked` to
    // `active`, backfills `updated_at`/`original_path` from
    // `created_at`/`path`, then enforces uniqueness with partial indexes:
    // non-NULL `path` and `sha256 WHERE size > 0` (empty artifacts share
    // the empty fingerprint) (design: Migration/indexes).
    "UPDATE artifacts SET status = 'active' WHERE status = 'tracked';
    ALTER TABLE artifacts ADD COLUMN updated_at TEXT;
    ALTER TABLE artifacts ADD COLUMN original_path TEXT;
    UPDATE artifacts SET updated_at = created_at WHERE updated_at IS NULL;
    UPDATE artifacts SET original_path = path WHERE original_path IS NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_artifacts_path
        ON artifacts(path) WHERE path IS NOT NULL;
    CREATE UNIQUE INDEX IF NOT EXISTS ux_artifacts_fingerprint
        ON artifacts(sha256) WHERE size > 0;",
];

/// Opens an existing database strictly read-only; never creates the file.
/// Status/doctor use this so checking can never repair or recreate state.
pub fn open_readonly(path: &Path) -> Result<Connection, AwcError> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )?)
}

/// Opens (creating when absent) the database at `path` with foreign keys
/// enabled. Callers then run [`migrate`].
pub fn open(path: &Path) -> Result<Connection, AwcError> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

/// Opens an existing database read-write without creating it: a missing
/// state file is an error, never a silently created empty database.
pub fn open_readwrite(path: &Path) -> Result<Connection, AwcError> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE,
    )?)
}

const PROJECT_COLUMNS: &str = "id, slug, name, root_path, status";

const ARTIFACT_COLUMNS: &str = "id, project_id, artifact_type, title, path, original_path, status, sha256, size, last_seen_at, created_at, updated_at";

/// Parses the canonical hyphenated text form stored by v2 (rusqlite's
/// `uuid` feature reads raw 16-byte blobs, which v2 does not use).
fn parse_uuid(text: String) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(&text).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(err))
    })
}

/// Maps one v2 `projects` row to a typed project.
fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    let root_path: Option<String> = row.get(3)?;
    Ok(Project {
        id: ProjectId(parse_uuid(row.get(0)?)?),
        slug: row.get(1)?,
        name: row.get(2)?,
        root_path: root_path.map(PathBuf::from),
        status: row.get(4)?,
    })
}

/// Maps one artifacts row to a typed artifact.
fn row_to_artifact(row: &rusqlite::Row<'_>) -> rusqlite::Result<Artifact> {
    let status_text: String = row.get(6)?;
    let status = ArtifactStatus::parse(&status_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            6,
            rusqlite::types::Type::Text,
            format!("unknown artifact status: {status_text}").into(),
        )
    })?;
    Ok(Artifact {
        id: ArtifactId(parse_uuid(row.get(0)?)?),
        project_id: ProjectId(parse_uuid(row.get(1)?)?),
        artifact_type: row.get(2)?,
        title: row.get(3)?,
        path: row.get::<_, Option<String>>(4)?.map(PathBuf::from),
        original_path: row.get::<_, Option<String>>(5)?.map(PathBuf::from),
        status,
        sha256: row.get(7)?,
        size: row.get::<_, Option<i64>>(8)?.map(|v| v as u64),
        last_seen_at: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

/// Inserts a project; a slug collision fails before any insert (design:
/// Create projects with deterministic slugs).
pub fn insert_project(
    conn: &mut Connection,
    slug: &str,
    name: &str,
    root_path: Option<&Path>,
) -> Result<Project, AwcError> {
    let tx = conn.transaction()?;
    let exists: i64 = tx.query_row(
        "SELECT COUNT(*) FROM projects WHERE slug = ?1",
        [slug],
        |row| row.get(0),
    )?;
    if exists > 0 {
        return Err(AwcError::SlugConflict(slug.to_string()));
    }
    let id = ProjectId::new();
    let project = tx.query_row(
        &format!(
            "INSERT INTO projects (id, slug, name, root_path) VALUES (?1, ?2, ?3, ?4) \
             RETURNING {PROJECT_COLUMNS}"
        ),
        rusqlite::params![
            id.0.to_string(),
            slug,
            name,
            root_path.map(|p| p.to_string_lossy().to_string())
        ],
        row_to_project,
    )?;
    tx.commit()?;
    Ok(project)
}

/// Candidate projects whose id starts with `prefix` (all rows when the
/// prefix is empty), in deterministic id order. The exact-one selection
/// rule lives in the application layer (design: Identity and lookup).
pub fn select_projects_by_id_prefix(
    conn: &Connection,
    prefix: &str,
) -> Result<Vec<Project>, AwcError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PROJECT_COLUMNS} FROM projects WHERE id LIKE ?1 || '%' ORDER BY id"
    ))?;
    let mut rows = stmt.query([prefix])?;
    let mut projects = Vec::new();
    while let Some(row) = rows.next()? {
        projects.push(row_to_project(row)?);
    }
    Ok(projects)
}

/// Applies pending migrations in version order, each inside its own
/// transaction, recording every applied version in the ledger. Rerunning is
/// idempotent; the ledger is authoritative. Version 2 first refuses the
/// migration when any v0.1 foundation table holds rows, leaving the database
/// untouched (design: Schema v2). Version 3 first refuses the migration when
/// legacy artifact rows cannot be canonicalized (design: Migration/indexes).
pub fn migrate(conn: &mut Connection) -> Result<(), AwcError> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {MIGRATIONS_TABLE} (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    ))?;
    for (index, sql) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        let applied: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {MIGRATIONS_TABLE} WHERE version = ?1"),
            [version],
            |row| row.get(0),
        )?;
        if applied > 0 {
            continue;
        }
        let tx = conn.transaction()?;
        match version {
            2 => refuse_populated_legacy(&tx)?,
            3 => migrate_v3(&tx)?,
            _ => {}
        }
        tx.execute_batch(sql)?;
        tx.execute(
            &format!("INSERT INTO {MIGRATIONS_TABLE} (version) VALUES (?1)"),
            [version],
        )?;
        tx.commit()?;
    }
    Ok(())
}

/// Refuses the v2 migration when any v0.1 foundation table holds rows. Runs
/// before any DDL inside the migration transaction, so rejection rolls back
/// without mutation: no schema change, no data change, ledger untouched.
fn refuse_populated_legacy(tx: &rusqlite::Transaction<'_>) -> Result<(), AwcError> {
    for table in ["projects", "artifacts", "audit_events"] {
        let rows: i64 = tx.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
            row.get(0)
        })?;
        if rows > 0 {
            return Err(AwcError::LegacySchemaData);
        }
    }
    Ok(())
}

/// Refuses the v3 migration when legacy artifact rows cannot be
/// canonicalized: an unknown status, duplicate non-null paths, or duplicate
/// non-empty fingerprints would make the lifecycle alignment unsafe. Runs
/// before any v3 DDL inside the migration transaction, so refusal rolls back
/// without mutation: no schema change, no data change, ledger untouched
/// (design: Migration/indexes).
fn migrate_v3(tx: &rusqlite::Transaction<'_>) -> Result<(), AwcError> {
    let mut stmt = tx.prepare("SELECT DISTINCT status FROM artifacts")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let status: String = row.get(0)?;
        if !matches!(
            status.as_str(),
            "tracked" | "active" | "archived" | "trashed"
        ) {
            return Err(AwcError::MigrationConflict(format!(
                "unknown artifact status {status:?}"
            )));
        }
    }
    let duplicate: i64 = tx.query_row(
        "SELECT COUNT(*) FROM (
            SELECT path FROM artifacts WHERE path IS NOT NULL GROUP BY path HAVING COUNT(*) > 1
         )",
        [],
        |row| row.get(0),
    )?;
    if duplicate > 0 {
        return Err(AwcError::MigrationConflict(
            "duplicate non-null artifact paths".into(),
        ));
    }
    let duplicate: i64 = tx.query_row(
        "SELECT COUNT(*) FROM (
            SELECT sha256 FROM artifacts WHERE size > 0 GROUP BY sha256 HAVING COUNT(*) > 1
         )",
        [],
        |row| row.get(0),
    )?;
    if duplicate > 0 {
        return Err(AwcError::MigrationConflict(
            "duplicate non-empty artifact fingerprints".into(),
        ));
    }
    Ok(())
}

/// Inserts an artifact with audit event in one transaction. A slug
/// collision on path or duplicate non-empty fingerprint fails before insert.
pub fn insert_artifact(conn: &mut Connection, artifact: &Artifact) -> Result<(), AwcError> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO artifacts (id, project_id, artifact_type, title, path, original_path, \
         status, sha256, size, last_seen_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            artifact.id.0.to_string(),
            artifact.project_id.0.to_string(),
            artifact.artifact_type,
            artifact.title,
            artifact
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact
                .original_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact.status.as_str(),
            artifact.sha256,
            artifact.size.map(|s| s as i64),
            artifact.last_seen_at,
            artifact.created_at,
            artifact.updated_at,
        ],
    )?;
    insert_audit_event_tx(
        &tx,
        artifact.project_id,
        Some(artifact.id),
        "artifact.created",
    )?;
    tx.commit()?;
    Ok(())
}

/// Registers an EXISTING governed file as an artifact (adopt apply): same
/// insert semantics as `insert_artifact` but the audit event records
/// `artifact.registered` and the path must already exist on disk.
pub fn register_artifact(conn: &mut Connection, artifact: &Artifact) -> Result<(), AwcError> {
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO artifacts (id, project_id, artifact_type, title, path, original_path, \
         status, sha256, size, last_seen_at, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
        rusqlite::params![
            artifact.id.0.to_string(),
            artifact.project_id.0.to_string(),
            artifact.artifact_type,
            artifact.title,
            artifact
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact
                .original_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact.status.as_str(),
            artifact.sha256,
            artifact.size.map(|s| s as i64),
            artifact.last_seen_at,
            artifact.created_at,
            artifact.updated_at,
        ],
    )?;
    insert_audit_event_tx(
        &tx,
        artifact.project_id,
        Some(artifact.id),
        "artifact.registered",
    )?;
    tx.commit()?;
    Ok(())
}

/// Updates an artifact's mutable fields and writes an audit event in one
/// transaction.
pub fn update_artifact(
    conn: &mut Connection,
    artifact: &Artifact,
    event_type: &str,
) -> Result<(), AwcError> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE artifacts SET path = ?2, original_path = ?3, status = ?4, \
         sha256 = ?5, size = ?6, last_seen_at = ?7, updated_at = ?8 WHERE id = ?1",
        rusqlite::params![
            artifact.id.0.to_string(),
            artifact
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact
                .original_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            artifact.status.as_str(),
            artifact.sha256,
            artifact.size.map(|s| s as i64),
            artifact.last_seen_at,
            artifact.updated_at,
        ],
    )?;
    insert_audit_event_tx(&tx, artifact.project_id, Some(artifact.id), event_type)?;
    tx.commit()?;
    Ok(())
}

/// Fetches one artifact by exact ID.
pub fn get_artifact(conn: &Connection, id: ArtifactId) -> Result<Artifact, AwcError> {
    conn.query_row(
        &format!("SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE id = ?1"),
        [id.0.to_string()],
        row_to_artifact,
    )
    .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => AwcError::ArtifactNotFound,
        other => AwcError::Database(other),
    })
}

/// Candidate artifacts whose id starts with `prefix`.
pub fn select_artifacts_by_id_prefix(
    conn: &Connection,
    prefix: &str,
) -> Result<Vec<Artifact>, AwcError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE id LIKE ?1 || '%' ORDER BY id"
    ))?;
    let mut rows = stmt.query([prefix])?;
    let mut artifacts = Vec::new();
    while let Some(row) = rows.next()? {
        artifacts.push(row_to_artifact(row)?);
    }
    Ok(artifacts)
}

/// Lists artifacts with optional filters, ordered by created_at DESC, id DESC.
pub fn list_artifacts(
    conn: &Connection,
    project_id: Option<ProjectId>,
    artifact_type: Option<&str>,
    status: Option<ArtifactStatus>,
) -> Result<Vec<Artifact>, AwcError> {
    let mut sql = format!("SELECT {ARTIFACT_COLUMNS} FROM artifacts WHERE 1=1");
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;
    if let Some(pid) = project_id {
        sql.push_str(&format!(" AND project_id = ?{idx}"));
        params.push(Box::new(pid.0.to_string()));
        idx += 1;
    }
    if let Some(t) = artifact_type {
        sql.push_str(&format!(" AND artifact_type = ?{idx}"));
        params.push(Box::new(t.to_string()));
        idx += 1;
    }
    if let Some(s) = status {
        sql.push_str(&format!(" AND status = ?{idx}"));
        params.push(Box::new(s.as_str().to_string()));
    }
    sql.push_str(" ORDER BY created_at DESC, id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let mut rows = stmt.query(refs.as_slice())?;
    let mut artifacts = Vec::new();
    while let Some(row) = rows.next()? {
        artifacts.push(row_to_artifact(row)?);
    }
    Ok(artifacts)
}

/// Checks whether a non-null path is already registered to a different artifact.
pub fn path_is_owned(
    conn: &Connection,
    path: &str,
    exclude_id: Option<ArtifactId>,
) -> Result<bool, AwcError> {
    let sql = if exclude_id.is_some() {
        "SELECT COUNT(*) FROM artifacts WHERE path = ?1 AND id != ?2"
    } else {
        "SELECT COUNT(*) FROM artifacts WHERE path = ?1"
    };
    let count: i64 = match exclude_id {
        Some(id) => conn.query_row(sql, rusqlite::params![path, id.0.to_string()], |row| {
            row.get(0)
        })?,
        None => conn.query_row(sql, [path], |row| row.get(0))?,
    };
    Ok(count > 0)
}

/// Checks whether a non-empty fingerprint already belongs to a different artifact.
pub fn fingerprint_is_duplicate(
    conn: &Connection,
    sha256: &str,
    size: u64,
    exclude_id: Option<ArtifactId>,
) -> Result<bool, AwcError> {
    if size == 0 {
        return Ok(false);
    }
    let sql = if exclude_id.is_some() {
        "SELECT COUNT(*) FROM artifacts WHERE sha256 = ?1 AND size > 0 AND id != ?2"
    } else {
        "SELECT COUNT(*) FROM artifacts WHERE sha256 = ?1 AND size > 0"
    };
    let count: i64 = match exclude_id {
        Some(id) => conn.query_row(sql, rusqlite::params![sha256, id.0.to_string()], |row| {
            row.get(0)
        })?,
        None => conn.query_row(sql, [sha256], |row| row.get(0))?,
    };
    Ok(count > 0)
}

/// Inserts an audit event inside an existing transaction.
fn insert_audit_event_tx(
    tx: &rusqlite::Transaction<'_>,
    project_id: ProjectId,
    artifact_id: Option<ArtifactId>,
    event_type: &str,
) -> Result<(), AwcError> {
    tx.execute(
        "INSERT INTO audit_events (id, project_id, artifact_id, event_type) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            AuditEventId::new().0.to_string(),
            project_id.0.to_string(),
            artifact_id.map(|a| a.0.to_string()),
            event_type,
        ],
    )?;
    Ok(())
}

/// Counts audit events for one artifact, used by tests to verify coupling.
#[cfg(test)]
pub fn count_audit_events(conn: &Connection, artifact_id: ArtifactId) -> Result<i64, AwcError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM audit_events WHERE artifact_id = ?1",
        [artifact_id.0.to_string()],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Schema health: all expected tables exist and every migration version is
/// recorded in the ledger (ledger is authoritative).
pub fn schema_health(conn: &Connection) -> Result<bool, AwcError> {
    for table in [MIGRATIONS_TABLE, "projects", "artifacts", "audit_events"] {
        let found: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [table],
            |row| row.get(0),
        )?;
        if found == 0 {
            return Ok(false);
        }
    }
    let versions: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM {MIGRATIONS_TABLE}"),
        [],
        |row| row.get(0),
    )?;
    Ok(versions as usize == MIGRATIONS.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("awc-core-sqlite-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn migrate_at(dir: &std::path::Path) -> Connection {
        let mut conn = open(&dir.join("state.sqlite3")).expect("open db");
        migrate(&mut conn).expect("migrate");
        conn
    }

    fn table_count(conn: &Connection, name: &str) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            [name],
            |row| row.get(0),
        )
        .expect("query sqlite_master")
    }

    fn ledger_versions(conn: &Connection) -> Vec<i64> {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT version FROM {MIGRATIONS_TABLE} ORDER BY version"
            ))
            .expect("prepare ledger query");
        stmt.query_map([], |row| row.get(0))
            .expect("query ledger")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect versions")
    }

    #[test]
    fn migrations_create_ledger_and_tables() {
        let dir = temp_dir("create");
        let conn = migrate_at(&dir);
        for table in [MIGRATIONS_TABLE, "projects", "artifacts", "audit_events"] {
            assert_eq!(table_count(&conn, table), 1, "missing table {table}");
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrations_record_every_version_in_order() {
        let dir = temp_dir("order");
        let conn = migrate_at(&dir);
        let expected: Vec<i64> = (1..=MIGRATIONS.len() as i64).collect();
        assert_eq!(ledger_versions(&conn), expected);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_rerun_is_idempotent() {
        let dir = temp_dir("rerun");
        let mut conn = open(&dir.join("state.sqlite3")).expect("open db");
        migrate(&mut conn).expect("first migrate");
        migrate(&mut conn).expect("second migrate");
        assert_eq!(ledger_versions(&conn), vec![1, 2, 3]);
        assert_eq!(schema_health(&conn).expect("health"), true);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let dir = temp_dir("fk");
        let conn = migrate_at(&dir);
        conn.execute(
            "INSERT INTO projects (id, slug, name) VALUES ('11111111-1111-7111-8111-111111111111', 'demo', 'Demo')",
            [],
        )
        .expect("insert project");
        let err = conn
            .execute(
                "INSERT INTO artifacts (id, project_id, artifact_type, title) VALUES ('22222222-2222-7222-8222-222222222222', '99999999-9999-7999-8999-999999999999', 'doc', 'orphan')",
                [],
            )
            .expect_err("orphan artifact must violate the foreign key");
        assert!(
            err.to_string().contains("FOREIGN KEY"),
            "unexpected error: {err}"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn schema_health_ok_after_migrate() {
        let dir = temp_dir("healthy");
        let conn = migrate_at(&dir);
        assert_eq!(schema_health(&conn).expect("health"), true);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn schema_health_false_when_state_missing() {
        let dir = temp_dir("unhealthy");
        let conn = open(&dir.join("state.sqlite3")).expect("open db");
        assert_eq!(schema_health(&conn).expect("health"), false);
        drop(conn);
        let conn = migrate_at(&dir);
        conn.execute_batch("DROP TABLE artifacts")
            .expect("drop table");
        assert_eq!(schema_health(&conn).expect("health"), false);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ledger_is_authoritative_over_table_existence() {
        let dir = temp_dir("ledger");
        let mut conn = migrate_at(&dir);
        conn.execute_batch("DROP TABLE projects")
            .expect("drop table");
        migrate(&mut conn).expect("rerun after drop");
        assert_eq!(
            table_count(&conn, "projects"),
            0,
            "recorded version must not re-apply"
        );
        assert_eq!(schema_health(&conn).expect("health"), false);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn repair_recreates_missing_db_and_preserves_config_bytes() {
        let dir = temp_dir("repair");
        let config_path = dir.join("config.toml");
        let config_bytes = crate::infrastructure::config::default_config_bytes();
        fs::write(&config_path, &config_bytes).expect("write config");

        // First init: open + migrate; valid bytes stay untouched.
        {
            let conn = migrate_at(&dir);
            assert_eq!(schema_health(&conn).expect("health"), true);
        }
        assert_eq!(fs::read(&config_path).expect("read config"), config_bytes);

        // Missing DB (partial state): recreate + migrate; bytes still unchanged.
        fs::remove_file(dir.join("state.sqlite3")).expect("remove db");
        {
            let conn = migrate_at(&dir);
            assert_eq!(schema_health(&conn).expect("health"), true);
        }
        assert_eq!(fs::read(&config_path).expect("read config"), config_bytes);
        fs::remove_dir_all(&dir).ok();
    }

    /// Simulates a v0.1 workspace database: the ledger records version 1 and
    /// the v1 foundation tables exist. `with_rows` also inserts one v1
    /// project row so the schema-v2 guard has data to refuse.
    fn v1_db(dir: &std::path::Path, with_rows: bool) -> Connection {
        let conn = open(&dir.join("state.sqlite3")).expect("open db");
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {MIGRATIONS_TABLE} (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );"
        ))
        .expect("create ledger");
        conn.execute_batch(MIGRATIONS[0]).expect("apply v1 schema");
        conn.execute(
            &format!("INSERT INTO {MIGRATIONS_TABLE} (version) VALUES (1)"),
            [],
        )
        .expect("record v1 in ledger");
        if with_rows {
            conn.execute("INSERT INTO projects (name) VALUES ('legacy')", [])
                .expect("populate a v1 project row");
        }
        conn
    }

    fn columns(conn: &Connection, table: &str) -> String {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({table})"))
            .expect("prepare pragma");
        stmt.query_map([], |row| row.get::<_, String>(1))
            .expect("query pragma")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect columns")
            .join(",")
    }

    /// Simulates a schema-v2 workspace database: the ledger records versions
    /// 1 and 2 and the v2 schema is applied, so only v3 remains pending.
    fn v2_db(dir: &std::path::Path) -> Connection {
        let conn = open(&dir.join("state.sqlite3")).expect("open db");
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS {MIGRATIONS_TABLE} (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );"
        ))
        .expect("create ledger");
        conn.execute_batch(MIGRATIONS[0]).expect("apply v1 schema");
        conn.execute_batch(MIGRATIONS[1]).expect("apply v2 schema");
        conn.execute(
            &format!("INSERT INTO {MIGRATIONS_TABLE} (version) VALUES (1), (2)"),
            [],
        )
        .expect("record v1 and v2 in ledger");
        conn
    }

    const V2_PROJECT_ID: &str = "11111111-1111-7111-8111-111111111111";

    /// Seeds one v2 artifact row (and its project) so v3 has legacy data to
    /// align or refuse. `created_at` is fixed so backfill assertions are
    /// deterministic.
    fn seed_v2_artifact(
        conn: &Connection,
        id: &str,
        path: Option<&str>,
        status: &str,
        sha256: Option<&str>,
        size: Option<i64>,
    ) {
        conn.execute(
            "INSERT OR IGNORE INTO projects (id, slug, name) \
             VALUES ('11111111-1111-7111-8111-111111111111', 'demo', 'Demo')",
            [],
        )
        .expect("seed project");
        conn.execute(
            "INSERT INTO artifacts (id, project_id, artifact_type, title, path, status, \
             sha256, size, created_at, last_seen_at) \
             VALUES (?1, ?2, 'doc', ?1, ?3, ?4, ?5, ?6, '2026-01-01T00:00:00Z', \
             '2026-01-01T00:00:00Z')",
            rusqlite::params![id, V2_PROJECT_ID, path, status, sha256, size],
        )
        .expect("seed artifact");
    }

    #[test]
    fn migrate_v3_aligns_tracked_status_and_backfills_timestamps() {
        let dir = temp_dir("v3-align");
        let mut conn = v2_db(&dir);
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000001",
            Some("artifacts/a.txt"),
            "tracked",
            None,
            None,
        );
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000002",
            None,
            "archived",
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
            Some(0),
        );
        migrate(&mut conn).expect("v3 aligns legacy rows");
        assert_eq!(ledger_versions(&conn), vec![1, 2, 3]);
        assert!(schema_health(&conn).expect("health"));
        let (status, updated_at, original_path): (String, String, Option<String>) = conn
            .query_row(
                "SELECT status, updated_at, original_path FROM artifacts WHERE id = ?1",
                ["aaaaaaaa-0000-0000-0000-000000000001"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("read migrated row");
        assert_eq!(status, "active", "tracked must canonicalize to active");
        assert_eq!(
            updated_at, "2026-01-01T00:00:00Z",
            "updated_at backfilled from created_at"
        );
        assert_eq!(
            original_path.as_deref(),
            Some("artifacts/a.txt"),
            "original_path backfilled from path"
        );
        let (status, original_path): (String, Option<String>) = conn
            .query_row(
                "SELECT status, original_path FROM artifacts WHERE id = ?1",
                ["aaaaaaaa-0000-0000-0000-000000000002"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read second migrated row");
        assert_eq!(status, "archived", "canonical statuses are preserved");
        assert_eq!(original_path, None, "NULL path stays NULL");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_v3_rejects_duplicate_paths_without_mutation() {
        let dir = temp_dir("v3-dup-path");
        let mut conn = v2_db(&dir);
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000001",
            Some("artifacts/a.txt"),
            "tracked",
            None,
            None,
        );
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000002",
            Some("artifacts/a.txt"),
            "tracked",
            None,
            None,
        );
        let err = migrate(&mut conn).expect_err("duplicate non-null paths must refuse v3");
        assert!(matches!(err, AwcError::MigrationConflict(_)));
        assert_eq!(
            ledger_versions(&conn),
            vec![1, 2],
            "v3 must not be recorded"
        );
        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM artifacts", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(rows, 2, "legacy rows must be unchanged");
        assert!(
            !columns(&conn, "artifacts").contains("updated_at"),
            "no v3 DDL may run"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_v3_rejects_unknown_status_without_mutation() {
        let dir = temp_dir("v3-unknown");
        let mut conn = v2_db(&dir);
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000001",
            Some("artifacts/a.txt"),
            "weird",
            None,
            None,
        );
        let err = migrate(&mut conn).expect_err("unknown status must refuse v3");
        assert!(matches!(err, AwcError::MigrationConflict(_)));
        assert_eq!(ledger_versions(&conn), vec![1, 2]);
        assert!(
            !columns(&conn, "artifacts").contains("updated_at"),
            "no v3 DDL may run"
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_v3_rejects_duplicate_non_empty_fingerprints_without_mutation() {
        let dir = temp_dir("v3-dup-hash");
        let mut conn = v2_db(&dir);
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000001",
            Some("artifacts/a.txt"),
            "tracked",
            Some("abc"),
            Some(5),
        );
        seed_v2_artifact(
            &conn,
            "aaaaaaaa-0000-0000-0000-000000000002",
            Some("artifacts/b.txt"),
            "tracked",
            Some("abc"),
            Some(5),
        );
        let err = migrate(&mut conn).expect_err("duplicate non-empty fingerprints must refuse v3");
        assert!(matches!(err, AwcError::MigrationConflict(_)));
        assert_eq!(ledger_versions(&conn), vec![1, 2]);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_v3_creates_lifecycle_indexes_and_enforces_them() {
        let dir = temp_dir("v3-indexes");
        let conn = migrate_at(&dir);
        let path_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'ux_artifacts_path'",
                [],
                |row| row.get(0),
            )
            .expect("path uniqueness index");
        assert!(path_sql.contains("path IS NOT NULL"), "{path_sql}");
        let fingerprint_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type = 'index' AND name = 'ux_artifacts_fingerprint'",
                [],
                |row| row.get(0),
            )
            .expect("fingerprint uniqueness index");
        assert!(fingerprint_sql.contains("size > 0"), "{fingerprint_sql}");
        conn.execute(
            "INSERT INTO projects (id, slug, name) \
             VALUES ('11111111-1111-7111-8111-111111111111', 'demo', 'Demo')",
            [],
        )
        .expect("insert project");
        let mut seq = 0_u32;
        let mut insert = |path: Option<&str>, sha: Option<&str>, size: Option<i64>| {
            seq += 1;
            conn.execute(
                "INSERT INTO artifacts (id, project_id, artifact_type, title, path, status, \
                 sha256, size) VALUES (?1, ?2, 'doc', ?1, ?3, 'active', ?4, ?5)",
                rusqlite::params![
                    format!("aaaaaaaa-0000-0000-0000-0000000000{seq:02}"),
                    V2_PROJECT_ID,
                    path,
                    sha,
                    size
                ],
            )
        };
        insert(Some("artifacts/one.txt"), Some("1111"), Some(1)).expect("first insert");
        let err = insert(Some("artifacts/one.txt"), Some("2222"), Some(2))
            .expect_err("duplicate path must be rejected");
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        insert(Some("artifacts/two.txt"), Some("3333"), Some(3)).expect("second insert");
        let err = insert(Some("artifacts/three.txt"), Some("3333"), Some(4))
            .expect_err("duplicate non-empty fingerprint must be rejected");
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        insert(Some("artifacts/empty-a.txt"), Some("empty"), Some(0)).expect("empty artifact");
        insert(Some("artifacts/empty-b.txt"), Some("empty"), Some(0))
            .expect("second empty artifact shares the empty fingerprint");
        insert(None, None, None).expect("NULL path is not unique-constrained");
        insert(None, None, None).expect("second NULL path row");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn migrate_v2_creates_full_metadata_schema_and_records_ledger() {
        let dir = temp_dir("v2-shape");
        let conn = migrate_at(&dir);
        assert_eq!(ledger_versions(&conn), vec![1, 2, 3]);
        assert!(columns(&conn, "projects").contains("id,slug,name,root_path,status,created_at"));
        assert!(columns(&conn, "artifacts").contains(
            "id,project_id,artifact_type,title,path,status,sha256,size,last_seen_at,\
                 created_at,updated_at,original_path"
        ));
        assert!(
            columns(&conn, "audit_events")
                .contains("id,project_id,artifact_id,event_type,occurred_at")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_v1_database_migrates_through_latest() {
        let dir = temp_dir("legacy-empty");
        let mut conn = v1_db(&dir, false);
        migrate(&mut conn).expect("empty v0.1 workspace migrates through v3");
        assert_eq!(ledger_versions(&conn), vec![1, 2, 3]);
        assert_eq!(schema_health(&conn).expect("health"), true);
        assert!(columns(&conn, "projects").contains("slug"));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn populated_v1_table_rejects_v2_without_mutation() {
        let dir = temp_dir("legacy-reject");
        let mut conn = v1_db(&dir, true);
        let err = migrate(&mut conn).expect_err("populated v0.1 data must refuse migration");
        assert!(matches!(err, AwcError::LegacySchemaData));

        let rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(rows, 1, "populated rows must be unchanged");
        assert_eq!(ledger_versions(&conn), vec![1], "ledger must be unchanged");
        assert_eq!(
            schema_health(&conn).expect("health"),
            false,
            "v2 must not be recorded"
        );
        assert!(
            !columns(&conn, "projects").contains("slug"),
            "no v2 DDL may run"
        );
        fs::remove_dir_all(&dir).ok();
    }

    fn make_artifact(id: &str, project_id: ProjectId, path: Option<&str>) -> Artifact {
        Artifact {
            id: ArtifactId(Uuid::parse_str(id).unwrap()),
            project_id,
            artifact_type: "doc".into(),
            title: "test".into(),
            path: path.map(PathBuf::from),
            original_path: path.map(PathBuf::from),
            status: ArtifactStatus::Active,
            sha256: None,
            size: None,
            last_seen_at: "2026-01-01T00:00:00Z".into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
        }
    }

    #[test]
    fn insert_artifact_writes_row_and_audit() {
        let dir = temp_dir("crud-insert");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let artifact = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        insert_artifact(&mut conn, &artifact).expect("insert artifact");
        let loaded = get_artifact(&conn, artifact.id).expect("get artifact");
        assert_eq!(loaded.title, "test");
        assert_eq!(loaded.status, ArtifactStatus::Active);
        assert_eq!(count_audit_events(&conn, artifact.id).expect("count"), 1);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn insert_artifact_rejects_duplicate_path() {
        let dir = temp_dir("crud-dup-path");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let first = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        insert_artifact(&mut conn, &first).expect("insert first");
        let second = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000002",
            project.id,
            Some("artifacts/a.txt"),
        );
        let err = insert_artifact(&mut conn, &second).expect_err("duplicate path");
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn insert_artifact_rejects_duplicate_non_empty_fingerprint() {
        let dir = temp_dir("crud-dup-hash");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let mut first = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        first.sha256 = Some("abc".into());
        first.size = Some(5);
        insert_artifact(&mut conn, &first).expect("insert first");
        let mut second = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000002",
            project.id,
            Some("artifacts/b.txt"),
        );
        second.sha256 = Some("abc".into());
        second.size = Some(5);
        let err = insert_artifact(&mut conn, &second).expect_err("duplicate fingerprint");
        assert!(err.to_string().contains("UNIQUE"), "{err}");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn insert_artifact_allows_shared_empty_fingerprint() {
        let dir = temp_dir("crud-empty-hash");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let mut first = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        first.sha256 = Some("empty".into());
        first.size = Some(0);
        insert_artifact(&mut conn, &first).expect("insert first");
        let mut second = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000002",
            project.id,
            Some("artifacts/b.txt"),
        );
        second.sha256 = Some("empty".into());
        second.size = Some(0);
        insert_artifact(&mut conn, &second).expect("insert second with same empty fingerprint");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn update_artifact_changes_status_and_audits() {
        let dir = temp_dir("crud-update");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let artifact = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        insert_artifact(&mut conn, &artifact).expect("insert");
        let mut updated = artifact.clone();
        updated.status = ArtifactStatus::Archived;
        updated.updated_at = "2026-01-02T00:00:00Z".into();
        update_artifact(&mut conn, &updated, "artifact.archived").expect("update");
        let loaded = get_artifact(&conn, artifact.id).expect("get");
        assert_eq!(loaded.status, ArtifactStatus::Archived);
        assert_eq!(count_audit_events(&conn, artifact.id).expect("count"), 2);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn list_artifacts_filters_and_orders() {
        let dir = temp_dir("crud-list");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let mut a = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        a.created_at = "2026-01-01T00:00:00Z".into();
        a.status = ArtifactStatus::Active;
        a.artifact_type = "doc".into();
        insert_artifact(&mut conn, &a).expect("insert a");
        let mut b = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000002",
            project.id,
            Some("artifacts/b.txt"),
        );
        b.created_at = "2026-01-02T00:00:00Z".into();
        b.status = ArtifactStatus::Archived;
        b.artifact_type = "report".into();
        insert_artifact(&mut conn, &b).expect("insert b");
        let all = list_artifacts(&conn, None, None, None).expect("list all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, b.id, "newest first");
        let filtered = list_artifacts(&conn, None, Some("doc"), None).expect("filter type");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, a.id);
        let filtered = list_artifacts(&conn, None, None, Some(ArtifactStatus::Archived))
            .expect("filter status");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, b.id);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn path_and_fingerprint_ownership_checks() {
        let dir = temp_dir("crud-ownership");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let mut artifact = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        artifact.sha256 = Some("abc".into());
        artifact.size = Some(5);
        insert_artifact(&mut conn, &artifact).expect("insert");
        assert!(path_is_owned(&conn, "artifacts/a.txt", None).expect("path owned"));
        assert!(!path_is_owned(&conn, "artifacts/b.txt", None).expect("path free"));
        assert!(!path_is_owned(&conn, "artifacts/a.txt", Some(artifact.id)).expect("exclude self"));
        assert!(fingerprint_is_duplicate(&conn, "abc", 5, None).expect("fingerprint dup"));
        assert!(
            !fingerprint_is_duplicate(&conn, "abc", 0, None).expect("empty fingerprint exempt")
        );
        assert!(
            !fingerprint_is_duplicate(&conn, "abc", 5, Some(artifact.id)).expect("exclude self")
        );
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn select_artifacts_by_prefix() {
        let dir = temp_dir("crud-prefix");
        let mut conn = migrate_at(&dir);
        let project = insert_project(&mut conn, "demo", "Demo", None).expect("insert project");
        let artifact = make_artifact(
            "aaaaaaaa-0000-0000-0000-000000000001",
            project.id,
            Some("artifacts/a.txt"),
        );
        insert_artifact(&mut conn, &artifact).expect("insert");
        let found = select_artifacts_by_id_prefix(&conn, "aaaaaaaa").expect("prefix");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, artifact.id);
        let none = select_artifacts_by_id_prefix(&conn, "bbbbbbbb").expect("no match");
        assert!(none.is_empty());
        fs::remove_dir_all(&dir).ok();
    }
}
