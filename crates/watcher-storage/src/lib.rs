//! SQLite interval store. A single process owns writes; readers go through IPC.
#![forbid(unsafe_code)]

use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use watcher_core::{ActivityState, Interval};

mod migrations;
pub use migrations::{CURRENT_SCHEMA_VERSION, StorageInitError};

pub struct Store {
    conn: Connection,
}

impl Store {
    /// A provisional interval and retroactive idle correction must be atomic.
    pub fn apply(
        &mut self,
        interval: Option<&Interval>,
        idle_since_ms: Option<i64>,
        until_ms: i64,
    ) -> rusqlite::Result<()> {
        self.conn.execute_batch("SAVEPOINT watcher_observation")?;
        let result = (|| {
            if let Some(interval) = interval {
                self.append(interval)?;
            }
            if let Some(since) = idle_since_ms {
                self.replace_recent_with_idle(since, until_ms)?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = self.conn.execute_batch("ROLLBACK TO watcher_observation");
        }
        self.conn.execute_batch("RELEASE watcher_observation")?;
        result
    }

    /// On daemon startup, upgrade an existing database in place. All required
    /// upgrades are transactional and backed up using SQLite's online backup
    /// API before the first schema change.
    pub fn open(path: &Path) -> Result<Self, StorageInitError> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(2))?;
        migrations::ensure_current(&mut conn, path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
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
                "SELECT id, end_ms, app, state FROM activity_intervals ORDER BY end_ms DESC, id DESC LIMIT 1",
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

    /// Replace only the provisional *active* portions, preserving pauses,
    /// unknown coverage and earlier idle events. A late idle notification must
    /// never corrupt other state or a disconnected collector's history.
    pub fn replace_recent_with_idle(
        &mut self,
        since_ms: i64,
        until_ms: i64,
    ) -> rusqlite::Result<()> {
        if since_ms >= until_ms {
            return Ok(());
        }
        let tx = self.conn.savepoint()?;
        let segments: Vec<(i64, i64, i64, Option<String>)> = {
            let mut stmt = tx.prepare(
                "SELECT id,start_ms,end_ms,app FROM activity_intervals
                 WHERE state='active' AND end_ms > ? AND start_ms < ? ORDER BY start_ms",
            )?;
            stmt.query_map(params![since_ms, until_ms], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<rusqlite::Result<_>>()?
        };
        for (id, start, end, app) in segments {
            tx.execute("DELETE FROM activity_intervals WHERE id=?", [id])?;
            let left = start.max(since_ms);
            let right = end.min(until_ms);
            if start < left {
                tx.execute(
                    "INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES(?,?,?,'active')",
                    params![start,left,app],
                )?;
            }
            tx.execute(
                "INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES(?,?,NULL,'idle')",
                params![left, right],
            )?;
            if right < end {
                tx.execute(
                    "INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES(?,?,?,'active')",
                    params![right,end,app],
                )?;
            }
        }
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
        assert_eq!(data.len(), 3);
        assert_eq!((data[0].start_ms, data[0].end_ms), (1000, 3000));
        assert_eq!(
            (data[1].start_ms, data[1].end_ms, data[1].state),
            (3000, 4000, ActivityState::Idle)
        );
        assert_eq!(
            (data[2].start_ms, data[2].end_ms, data[2].state),
            (4000, 5000, ActivityState::Idle)
        );
    }

    #[test]
    fn late_idle_preserves_paused_and_unknown_segments() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        for (start_ms, end_ms, state) in [
            (0, 1000, ActivityState::Active),
            (1000, 2000, ActivityState::Paused),
            (2000, 3000, ActivityState::Unknown),
            (3000, 4000, ActivityState::Active),
        ] {
            db.append(&Interval {
                start_ms,
                end_ms,
                app: Some("A".into()),
                state,
            })
            .unwrap();
        }
        db.replace_recent_with_idle(500, 4000).unwrap();
        let rows = db.query(0, 5000, 100).unwrap();
        assert_eq!(
            rows.iter()
                .filter(|r| r.state == ActivityState::Paused)
                .count(),
            1
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r.state == ActivityState::Unknown)
                .count(),
            1
        );
        assert_eq!(
            rows.iter()
                .filter(|r| r.state == ActivityState::Idle)
                .map(|r| r.end_ms - r.start_ms)
                .sum::<i64>(),
            1500
        );
    }

    #[test]
    fn atomic_transition_rolls_back_on_conflict() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        db.append(&Interval {
            start_ms: 10,
            end_ms: 20,
            app: Some("A".into()),
            state: ActivityState::Active,
        })
        .unwrap();
        let conflict = Interval {
            start_ms: 15,
            end_ms: 30,
            app: Some("B".into()),
            state: ActivityState::Active,
        };
        assert!(db.apply(Some(&conflict), Some(10), 30).is_err());
        let existing = db.query(0, 100, 10).unwrap();
        assert_eq!(existing.len(), 1);
        assert_eq!(existing[0].state, ActivityState::Active);
        assert_eq!(existing[0].end_ms, 20);
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
