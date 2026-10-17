//! Ordered, embedded SQLite schema migrations. The only owner of versioning.
//! Never assign user_version based solely on CREATE TABLE IF NOT EXISTS.

use rusqlite::{
    Connection, OptionalExtension, TransactionBehavior,
    backup::{Backup, StepResult},
};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub const CURRENT_SCHEMA_VERSION: i64 = 3;

#[derive(Debug, thiserror::Error)]
pub enum StorageInitError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error(
        "database schema v{found} is newer than this Watcher supports (v{supported}); install a newer Watcher rather than downgrading the database"
    )]
    NewerSchema { found: i64, supported: i64 },
    #[error(
        "database has objects but schema version is 0 (unversioned); refusing to guess or overwrite its format. Make a backup and inspect the database manually"
    )]
    UnversionedDatabase,
    #[error("database schema verification failed: {0}; no schema changes were applied")]
    InvalidSchema(String),
    #[error("schema migration v{version} failed and was rolled back: {source}")]
    Migration {
        version: i64,
        #[source]
        source: rusqlite::Error,
    },
    #[error("could not make a consistent pre-migration backup at {}: {reason}; database was not upgraded", path.display())]
    BackupFailed { path: PathBuf, reason: String },
}

#[derive(Clone, Copy)]
struct Migration {
    version: i64,
    sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/001_initial.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/002_end_time_index.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("../migrations/003_browser_intervals.sql"),
    },
];

fn schema_version(conn: &Connection) -> Result<i64, StorageInitError> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

fn check_health(conn: &Connection) -> Result<(), StorageInitError> {
    // quick_check verifies database b-trees without requiring a full dump.
    // Used only for upgrades and pre-migration backups, not every startup.
    let status: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if status != "ok" {
        return Err(StorageInitError::InvalidSchema(
            "SQLite quick_check found damaged pages or indexes".into(),
        ));
    }
    Ok(())
}

fn verify_columns(conn: &Connection) -> Result<(), StorageInitError> {
    let mut stmt = conn.prepare("PRAGMA table_info('activity_intervals')")?;
    let columns = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let expected = [
        ("id", "INTEGER", 0, 1),
        ("start_ms", "INTEGER", 1, 0),
        ("end_ms", "INTEGER", 1, 0),
        ("app", "TEXT", 0, 0),
        ("state", "TEXT", 1, 0),
    ];
    if columns.len() != expected.len()
        || columns.iter().zip(expected).any(|(actual, expected)| {
            (actual.0.as_str(), actual.1.as_str(), actual.2, actual.3) != expected
        })
    {
        return Err(StorageInitError::InvalidSchema(
            "activity_intervals columns are missing, reordered or have unexpected types".into(),
        ));
    }

    let sql: Option<String> = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='activity_intervals'",
            [],
            |row| row.get(0),
        )
        .optional()?;
    let compact: String = sql
        .unwrap_or_default()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    if !compact.contains("check(end_ms>start_ms)")
        || !compact.contains("check(statein('active','idle','unknown','paused'))")
    {
        return Err(StorageInitError::InvalidSchema(
            "activity_intervals duration/state constraints are not the expected v1 constraints"
                .into(),
        ));
    }
    Ok(())
}

fn verify_index(
    conn: &Connection,
    index_name: &str,
    expected: &[&str],
) -> Result<(), StorageInitError> {
    // Index names are compile-time constants below, never user-provided SQL.
    let pragma = format!("PRAGMA index_info('{index_name}')");
    let mut stmt = conn.prepare(&pragma)?;
    let fields = stmt
        .query_map([], |row| row.get::<_, Option<String>>(2))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if fields.iter().map(|s| s.as_deref()).collect::<Vec<_>>()
        != expected.iter().map(|s| Some(*s)).collect::<Vec<_>>()
    {
        return Err(StorageInitError::InvalidSchema(format!(
            "required index {index_name} is missing or has unexpected columns"
        )));
    }
    Ok(())
}

fn verify_structure(conn: &Connection, version: i64) -> Result<(), StorageInitError> {
    verify_columns(conn)?;
    verify_index(conn, "intervals_by_time", &["start_ms", "end_ms"])?;
    if version >= 2 {
        verify_index(conn, "intervals_by_end", &["end_ms"])?;
    }
    if version >= 3 {
        let names: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='browser_intervals'",
            [],
            |row| row.get(0),
        )?;
        if names != 1 {
            return Err(StorageInitError::InvalidSchema(
                "browser_intervals missing".into(),
            ));
        }
        verify_index(conn, "browser_by_time", &["start_ms", "end_ms"])?;
        let mut columns = conn.prepare("PRAGMA table_info('browser_intervals')")?;
        let actual = columns
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let expected = [
            ("source_id", "TEXT", 1, 1),
            ("seq", "INTEGER", 1, 2),
            ("start_ms", "INTEGER", 1, 0),
            ("end_ms", "INTEGER", 1, 0),
            ("domain", "TEXT", 1, 0),
        ];
        if actual.len() != expected.len()
            || actual
                .iter()
                .zip(expected)
                .any(|(a, e)| (a.0.as_str(), a.1.as_str(), a.2, a.3) != e)
        {
            return Err(StorageInitError::InvalidSchema(
                "browser_intervals columns differ from v3".into(),
            ));
        }
        let sql: String = conn.query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='browser_intervals'",
            [],
            |row| row.get(0),
        )?;
        let compact: String = sql
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_lowercase();
        if !compact.contains("check(seq>0)")
            || !compact.contains("check(end_ms>start_ms)")
            || !compact.contains("withoutrowid")
        {
            return Err(StorageInitError::InvalidSchema(
                "browser_intervals constraints differ from v3".into(),
            ));
        }
    }
    Ok(())
}

/// Called before a daemon starts collectors. On failure Store::open aborts.
pub(super) fn ensure_current(conn: &mut Connection, path: &Path) -> Result<(), StorageInitError> {
    let version = schema_version(conn)?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(StorageInitError::NewerSchema {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if version == 0 {
        let existing: i64 = conn.query_row(
            "SELECT count(*) FROM sqlite_master WHERE type IN ('table','index','view','trigger') AND name NOT LIKE 'sqlite_%'",
            [],
            |row| row.get(0),
        )?;
        if existing != 0 {
            return Err(StorageInitError::UnversionedDatabase);
        }
    } else {
        verify_structure(conn, version)?;
    }
    if version == CURRENT_SCHEMA_VERSION {
        secure_existing_database(path)?;
        return Ok(());
    }

    // For pre-existing data, validate first and make a SQLite-consistent copy,
    // including uncheckpointed WAL pages. Fresh, empty v0 databases don't need
    // backups. Every upgrade retains the backup until a user removes it.
    if version > 0 {
        check_health(conn)?;
        if path != Path::new(":memory:") {
            create_backup(conn, path, version)?;
        }
    }
    apply_migrations(conn, version, MIGRATIONS)?;
    verify_structure(conn, CURRENT_SCHEMA_VERSION)?;
    secure_existing_database(path)?;
    Ok(())
}

fn apply_migrations(
    conn: &mut Connection,
    starting_version: i64,
    migrations: &[Migration],
) -> Result<(), StorageInitError> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let mut applied = starting_version;
    for migration in migrations.iter().filter(|m| m.version > starting_version) {
        if migration.version != applied + 1 {
            return Err(StorageInitError::InvalidSchema(format!(
                "non-contiguous migration sequence at v{}",
                migration.version
            )));
        }
        tx.execute_batch(migration.sql)
            .map_err(|source| StorageInitError::Migration {
                version: migration.version,
                source,
            })?;
        tx.pragma_update(None, "user_version", migration.version)
            .map_err(|source| StorageInitError::Migration {
                version: migration.version,
                source,
            })?;
        applied = migration.version;
    }
    if applied != CURRENT_SCHEMA_VERSION {
        return Err(StorageInitError::InvalidSchema(format!(
            "migration chain ended at v{applied}, expected v{CURRENT_SCHEMA_VERSION}"
        )));
    }
    // Validate while still holding the migration transaction. Even a
    // syntactically successful but incomplete migration must roll back.
    verify_structure(&tx, applied)?;
    check_health(&tx)?;
    tx.commit()?;
    Ok(())
}

fn secure_existing_database(path: &Path) -> Result<(), StorageInitError> {
    if path != Path::new(":memory:") {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
    }
    Ok(())
}

fn create_backup(
    source: &Connection,
    db_path: &Path,
    from_version: i64,
) -> Result<PathBuf, StorageInitError> {
    let filename = db_path
        .file_name()
        .ok_or_else(|| StorageInitError::InvalidSchema("invalid database file name".into()))?
        .to_string_lossy();
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let mut destination_path = None;
    for attempt in 0..100 {
        let path = db_path.with_file_name(format!(
            "{filename}.pre-v{from_version}-to-v{CURRENT_SCHEMA_VERSION}-{millis}-{}-{attempt}.bak",
            std::process::id()
        ));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Err(err) = file.set_permissions(fs::Permissions::from_mode(0o600)) {
                        drop(file);
                        let _ = fs::remove_file(&path);
                        return Err(StorageInitError::BackupFailed {
                            path,
                            reason: err.to_string(),
                        });
                    }
                }
                drop(file);
                destination_path = Some(path);
                break;
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(err) => {
                return Err(StorageInitError::BackupFailed {
                    path,
                    reason: err.to_string(),
                });
            }
        }
    }
    let path = destination_path.ok_or_else(|| StorageInitError::BackupFailed {
        path: db_path.to_owned(),
        reason: "could not reserve a unique backup filename".into(),
    })?;
    let result = (|| -> Result<(), StorageInitError> {
        let mut target = Connection::open(&path)?;
        {
            let backup = Backup::new(source, &mut target)?;
            let mut last_progress = Instant::now();
            loop {
                match backup.step(256)? {
                    StepResult::Done => break,
                    StepResult::More => last_progress = Instant::now(),
                    StepResult::Busy | StepResult::Locked => {
                        if last_progress.elapsed() > Duration::from_secs(10) {
                            return Err(StorageInitError::InvalidSchema(
                                "database remained locked during backup for 10 seconds".into(),
                            ));
                        }
                        thread::sleep(Duration::from_millis(10));
                    }
                    _ => {
                        return Err(StorageInitError::InvalidSchema(
                            "SQLite backup returned an unsupported status".into(),
                        ));
                    }
                }
            }
        }
        if schema_version(&target)? != from_version {
            return Err(StorageInitError::InvalidSchema(
                "backup schema version does not match the original".into(),
            ));
        }
        check_health(&target)?;
        Ok(())
    })();
    if let Err(err) = result {
        let _ = fs::remove_file(&path);
        return Err(StorageInitError::BackupFailed {
            path,
            reason: err.to_string(),
        });
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_embedded_migrations_are_contiguous() {
        assert_eq!(
            MIGRATIONS.iter().map(|m| m.version).collect::<Vec<_>>(),
            (1..=CURRENT_SCHEMA_VERSION).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_failed_later_migration_rolls_back_every_prior_change() {
        let mut conn = Connection::open_in_memory().unwrap();
        let bad = [
            Migration {
                version: 1,
                sql: "CREATE TABLE sample (id INTEGER PRIMARY KEY); INSERT INTO sample VALUES(1);",
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE change_me (x INTEGER); INSERT INTO nonexistent VALUES(42);",
            },
        ];
        assert!(matches!(
            apply_migrations(&mut conn, 0, &bad),
            Err(StorageInitError::Migration { version: 2, .. })
        ));
        assert_eq!(schema_version(&conn).unwrap(), 0);
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name IN ('sample','change_me')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn a_syntactically_valid_but_incomplete_migration_also_rolls_back() {
        let mut conn = Connection::open_in_memory().unwrap();
        let wrong = [
            Migration {
                version: 1,
                sql: MIGRATIONS[0].sql,
            },
            Migration {
                version: 2,
                sql: "CREATE TABLE unrelated (id INTEGER PRIMARY KEY);",
            },
        ];
        assert!(matches!(
            apply_migrations(&mut conn, 0, &wrong),
            Err(StorageInitError::InvalidSchema(_))
        ));
        assert_eq!(schema_version(&conn).unwrap(), 0);
        let remaining: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE name='activity_intervals'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[test]
    fn a_failed_backup_does_not_modify_a_valid_v1_database() {
        let temp = tempfile::tempdir().unwrap();
        let impossible = temp.path().join("absent-folder").join("watcher.sqlite");
        let source = Connection::open_in_memory().unwrap();
        source.execute_batch(MIGRATIONS[0].sql).unwrap();
        source.pragma_update(None, "user_version", 1).unwrap();
        assert!(matches!(
            create_backup(&source, &impossible, 1),
            Err(StorageInitError::BackupFailed { .. })
        ));
        assert_eq!(schema_version(&source).unwrap(), 1);
        verify_structure(&source, 1).unwrap();
    }
}
