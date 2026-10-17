use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use watcher_core::{ActivityState, Interval};
use watcher_storage::{CURRENT_SCHEMA_VERSION, StorageInitError, Store};

fn backups(dir: &Path) -> Vec<PathBuf> {
    let mut results: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|d| d.unwrap().path())
        .filter(|path| {
            path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n.starts_with("watcher.sqlite.pre-v1-to-v3-") && n.ends_with(".bak")
            })
        })
        .collect();
    results.sort();
    results
}

fn version(conn: &Connection) -> i64 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap()
}

#[test]
fn blank_database_is_created_at_v2_without_a_backup() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("watcher.sqlite");
    let store = Store::open(&path).unwrap();
    assert!(store.query(0, 1000, 10).unwrap().is_empty());
    drop(store);
    let conn = Connection::open(&path).unwrap();
    assert_eq!(version(&conn), CURRENT_SCHEMA_VERSION);
    assert!(backups(dir.path()).is_empty());
    let n: i64 = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='index' AND name='intervals_by_end'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn upgrades_an_actual_v1_fixture_preserving_uncheckpointed_wal_rows() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("watcher.sqlite");
    let fixture = Connection::open(&path).unwrap();
    fixture.pragma_update(None, "journal_mode", "WAL").unwrap();
    fixture
        .pragma_update(None, "wal_autocheckpoint", 0)
        .unwrap();
    fixture
        .execute_batch(include_str!("fixtures/v1.sql"))
        .unwrap();
    assert_eq!(version(&fixture), 1);

    // Keep this original connection open: the latest changes may exist only
    // in watcher.sqlite-wal. A normal filesystem copy would lose data.
    let mut store = Store::open(&path).unwrap();
    let rows = store.query(0, 7000, 100).unwrap();
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].app.as_deref(), Some("firefox"));
    assert_eq!(rows[1].state, ActivityState::Idle);
    assert_eq!(rows[4].state, ActivityState::Paused);
    store
        .append(&Interval {
            start_ms: 6000,
            end_ms: 7000,
            app: Some("new-editor".into()),
            state: ActivityState::Active,
        })
        .unwrap();
    drop(store);

    let backup_files = backups(dir.path());
    assert_eq!(backup_files.len(), 1, "exactly one pre-upgrade backup");
    let snapshot = Connection::open(&backup_files[0]).unwrap();
    assert_eq!(version(&snapshot), 1);
    assert_eq!(
        snapshot
            .query_row("SELECT count(*) FROM activity_intervals", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        5
    );
    assert_eq!(
        snapshot
            .query_row(
                "SELECT app FROM activity_intervals WHERE start_ms=1000",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        "firefox"
    );
    assert_eq!(
        snapshot
            .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    let after = Connection::open(&path).unwrap();
    assert_eq!(version(&after), 3);
    assert_eq!(
        after
            .query_row("SELECT count(*) FROM activity_intervals", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        6
    );
    drop(after);

    // Reopening the upgraded DB must be idempotent and not create new backups.
    let reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.query(0, 10_000, 100).unwrap().len(), 6);
    assert_eq!(backups(dir.path()).len(), 1);
    drop(reopened);
    drop(snapshot);
    drop(fixture);
}

#[test]
fn refuses_unversioned_tables_without_changing_or_backing_up_data() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("watcher.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(
        "CREATE TABLE personal_data (secret TEXT); INSERT INTO personal_data VALUES('keep');",
    )
    .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open(&path),
        Err(StorageInitError::UnversionedDatabase)
    ));
    let conn = Connection::open(&path).unwrap();
    assert_eq!(version(&conn), 0);
    assert_eq!(
        conn.query_row("SELECT secret FROM personal_data", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "keep"
    );
    assert!(backups(dir.path()).is_empty());
}

#[test]
fn refuses_newer_versions_without_downgrading() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("watcher.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA user_version = 99; CREATE TABLE sentinel (value INTEGER);")
        .unwrap();
    drop(conn);
    assert!(matches!(
        Store::open(&path),
        Err(StorageInitError::NewerSchema {
            found: 99,
            supported: 3
        })
    ));
    let conn = Connection::open(&path).unwrap();
    assert_eq!(version(&conn), 99);
    assert!(backups(dir.path()).is_empty());
}

#[test]
fn refuses_partial_database_falsely_labeled_as_v1_or_v2() {
    for schema_version in [1, 2] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("watcher.sqlite");
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(&format!(
            "CREATE TABLE activity_intervals(id INTEGER PRIMARY KEY, start_ms INTEGER); PRAGMA user_version={schema_version}"
        )).unwrap();
        drop(conn);
        assert!(matches!(
            Store::open(&path),
            Err(StorageInitError::InvalidSchema(_))
        ));
        let conn = Connection::open(&path).unwrap();
        assert_eq!(version(&conn), schema_version);
        assert!(backups(dir.path()).is_empty());
    }
}

#[cfg(unix)]
#[test]
fn backup_is_private_to_the_local_user() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("watcher.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("fixtures/v1.sql")).unwrap();
    drop(conn);
    Store::open(&path).unwrap();
    let mode = fs::metadata(&backups(dir.path())[0])
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let db_mode = fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(db_mode, 0o600);
}

#[test]
fn browser_replay_is_idempotent_and_never_inflates_desktop_time() {
    use watcher_ipc::BrowserInterval;
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("watcher.sqlite");
    let mut store = Store::open(&path).unwrap();
    let batch = vec![BrowserInterval {
        source_id: "browser-a".into(),
        seq: 1,
        start_ms: 1_000,
        end_ms: 10_000,
        domain: "example.org".into(),
    }];
    store.import_browser(&batch, 15_000).unwrap();
    store.import_browser(&batch, 15_000).unwrap();
    assert_eq!(
        store.browser_report(0, 20_000).unwrap()[0].duration_ms,
        9_000
    );
    // Browser records are never inserted into OS activity_intervals.
    assert!(store.query(0, 20_000, 100).unwrap().is_empty());
    let invalid = vec![BrowserInterval {
        seq: 2,
        domain: "example.org/path?private=1".into(),
        ..batch[0].clone()
    }];
    assert!(store.import_browser(&invalid, 15_000).is_err());
    assert_eq!(store.browser_report(0, 20_000).unwrap().len(), 1);
    let conflict = vec![BrowserInterval {
        domain: "elsewhere.example".into(),
        ..batch[0].clone()
    }];
    assert!(store.import_browser(&conflict, 15_000).is_err());
    let overlap = vec![BrowserInterval {
        seq: 2,
        start_ms: 5_000,
        end_ms: 11_000,
        ..batch[0].clone()
    }];
    assert!(store.import_browser(&overlap, 15_000).is_err());
}

#[test]
fn upgrades_version_two_to_three_without_losing_device_intervals() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("watcher.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("fixtures/v1.sql")).unwrap();
    conn.execute_batch(include_str!("../migrations/002_end_time_index.sql"))
        .unwrap();
    conn.pragma_update(None, "user_version", 2).unwrap();
    drop(conn);
    let db = Store::open(&path).unwrap();
    assert_eq!(db.query(0, 10_000, 100).unwrap().len(), 5);
    drop(db);
    let conn = Connection::open(&path).unwrap();
    assert_eq!(version(&conn), 3);
    let copies = fs::read_dir(temp.path())
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().contains("pre-v2-to-v3"))
        .count();
    assert_eq!(copies, 1);
}
