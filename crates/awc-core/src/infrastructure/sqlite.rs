//! SQLite state backend: transactional, ordered migrations with a version ledger.

use std::path::Path;

use rusqlite::Connection;

use crate::error::AwcError;

/// Ledger table recording every applied migration version.
pub const MIGRATIONS_TABLE: &str = "schema_migrations";

/// Ordered migrations; `index + 1` is the version number. Schema only — no
/// lifecycle CRUD, APIs, or implicit records (design: Persistence/ledger).
const MIGRATIONS: &[&str] = &[
    // v1: minimal projects/artifacts/audit_events with keys, timestamps, FKs.
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
];

/// Opens (creating when absent) the database at `path` with foreign keys
/// enabled. Callers then run [`migrate`].
pub fn open(path: &Path) -> Result<Connection, AwcError> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA foreign_keys = ON;")?;
    Ok(conn)
}

/// Applies pending migrations in version order, each inside its own
/// transaction, recording every applied version in the ledger. Rerunning is
/// idempotent; the ledger is authoritative.
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
        tx.execute_batch(sql)?;
        tx.execute(
            &format!("INSERT INTO {MIGRATIONS_TABLE} (version) VALUES (?1)"),
            [version],
        )?;
        tx.commit()?;
    }
    Ok(())
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
        assert_eq!(ledger_versions(&conn), vec![1]);
        assert_eq!(schema_health(&conn).expect("health"), true);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let dir = temp_dir("fk");
        let conn = migrate_at(&dir);
        conn.execute("INSERT INTO projects (name) VALUES ('demo')", [])
            .expect("insert project");
        let err = conn
            .execute(
                "INSERT INTO artifacts (project_id, name) VALUES (999, 'orphan')",
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
}
