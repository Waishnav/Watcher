//! SQLite interval store. A single process owns writes; readers go through IPC.
#![forbid(unsafe_code)]

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use watcher_core::{ActivityState, Interval};

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        let schema_version: i64 =
            conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if schema_version > 1 {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "database schema {schema_version} is newer than supported version 1"
            )));
        }
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS activity_intervals (
                id INTEGER PRIMARY KEY,
                start_ms INTEGER NOT NULL,
                end_ms INTEGER NOT NULL CHECK(end_ms > start_ms),
                app TEXT,
                state TEXT NOT NULL CHECK(state IN ('active','idle','unknown','paused'))
             );
             CREATE INDEX IF NOT EXISTS intervals_by_time ON activity_intervals(start_ms,end_ms);
             PRAGMA user_version = 1;",
        )?;
        Ok(Self { conn })
    }

    pub fn last_end(&self) -> rusqlite::Result<Option<i64>> {
        self.conn
            .query_row("SELECT MAX(end_ms) FROM activity_intervals", [], |row| {
                row.get(0)
            })
    }

    pub fn append(&mut self, interval: &Interval) -> rusqlite::Result<()> {
        if interval.end_ms <= interval.start_ms {
            return Ok(());
        }
        let previous = self
            .conn
            .query_row(
                "SELECT id, end_ms, app, state FROM activity_intervals ORDER BY id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                },
            )
            .optional()?;
        if let Some((id, end, app, state)) = previous {
            if end == interval.start_ms && app == interval.app && state == interval.state.as_str() {
                self.conn.execute(
                    "UPDATE activity_intervals SET end_ms=? WHERE id=?",
                    params![interval.end_ms, id],
                )?;
                return Ok(());
            }
            // No overlap in a single desktop session even after clock adjustments.
            if end > interval.start_ms {
                return Err(rusqlite::Error::InvalidParameterName(
                    "overlapping interval".into(),
                ));
            }
        }
        self.conn.execute(
            "INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES (?,?,?,?)",
            params![
                interval.start_ms,
                interval.end_ms,
                interval.app,
                interval.state.as_str()
            ],
        )?;
        Ok(())
    }

    /// Replace a provisional recent activity span when an idle signal arrives
    /// after the configured timeout. This removes every overlapping state from
    /// this session (not only the most recent focused application).
    pub fn replace_recent_with_idle(
        &mut self,
        since_ms: i64,
        until_ms: i64,
    ) -> rusqlite::Result<()> {
        if since_ms >= until_ms {
            return Ok(());
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM activity_intervals WHERE start_ms >= ?",
            [since_ms],
        )?;
        tx.execute(
            "UPDATE activity_intervals SET end_ms=? WHERE start_ms < ? AND end_ms > ?",
            params![since_ms, since_ms, since_ms],
        )?;
        tx.execute(
            "INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES(?,?,NULL,'idle')",
            params![since_ms, until_ms],
        )?;
        tx.commit()
    }

    pub fn query(&self, from_ms: i64, to_ms: i64, limit: usize) -> rusqlite::Result<Vec<Interval>> {
        let mut stmt = self.conn.prepare(
            "SELECT start_ms, end_ms, app, state FROM activity_intervals
             WHERE end_ms > ? AND start_ms < ? ORDER BY start_ms LIMIT ?",
        )?;
        let rows = stmt.query_map(
            params![from_ms, to_ms, i64::try_from(limit).unwrap_or(i64::MAX)],
            |row| {
                let state: String = row.get(3)?;
                let state = match state.as_str() {
                    "active" => ActivityState::Active,
                    "idle" => ActivityState::Idle,
                    "paused" => ActivityState::Paused,
                    _ => ActivityState::Unknown,
                };
                Ok(Interval {
                    start_ms: row.get(0)?,
                    end_ms: row.get(1)?,
                    app: row.get(2)?,
                    state,
                })
            },
        )?;
        rows.collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idempotent_adjacent_chunks_and_idle_rewind() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store
            .append(&Interval {
                start_ms: 1000,
                end_ms: 2000,
                app: Some("A".into()),
                state: ActivityState::Active,
            })
            .unwrap();
        store
            .append(&Interval {
                start_ms: 2000,
                end_ms: 4000,
                app: Some("A".into()),
                state: ActivityState::Active,
            })
            .unwrap();
        store
            .append(&Interval {
                start_ms: 4000,
                end_ms: 5000,
                app: Some("B".into()),
                state: ActivityState::Active,
            })
            .unwrap();
        assert_eq!(store.query(0, 10_000, 100).unwrap().len(), 2);
        store.replace_recent_with_idle(3000, 6000).unwrap();
        let data = store.query(0, 10_000, 100).unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!((data[0].start_ms, data[0].end_ms), (1000, 3000));
        assert_eq!(
            (data[1].start_ms, data[1].end_ms, data[1].state),
            (3000, 6000, ActivityState::Idle)
        );
    }

    #[test]
    fn reopen_and_gap_are_durable() {
        let path = std::env::temp_dir().join(format!(
            "watcher-storage-test-{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        {
            let mut store = Store::open(&path).unwrap();
            store
                .append(&Interval {
                    start_ms: 1000,
                    end_ms: 2000,
                    app: Some("A".into()),
                    state: ActivityState::Active,
                })
                .unwrap();
        }
        {
            let mut store = Store::open(&path).unwrap();
            assert_eq!(store.last_end().unwrap(), Some(2000));
            store
                .append(&Interval {
                    start_ms: 2000,
                    end_ms: 10000,
                    app: None,
                    state: ActivityState::Unknown,
                })
                .unwrap();
            assert_eq!(store.query(0, 11000, 100).unwrap().len(), 2);
        }
        let _ = std::fs::remove_file(path);
    }
}
