# Watcher: local SQLite migration contract

**Implemented in the Rust rewrite:** automatic migration of historical schema v1 to schema **v2**, plus creation of v2 databases from scratch. The migration code is embedded in the daemon; no standalone migration executable, cloud service, or installer script is needed.

## Authoritative code

- `crates/watcher-storage/src/migrations.rs` — schema version, validation, backup, upgrade transaction.
- `crates/watcher-storage/migrations/001_initial.sql` — frozen, originally shipped schema v1.
- `crates/watcher-storage/migrations/002_end_time_index.sql` — additive performance index in v2, with no data rewrite.
- `crates/watcher-storage/tests/fixtures/v1.sql` — historical v1 fixture independent of the latest schema creation code.
- `crates/watcher-storage/tests/schema_upgrade.rs` — on-disk upgrade and failure-mode tests.

## Current v2 schema

```sql
CREATE TABLE activity_intervals (
    id INTEGER PRIMARY KEY,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL CHECK(end_ms > start_ms),
    app TEXT,
    state TEXT NOT NULL CHECK(state IN ('active','idle','unknown','paused'))
);
CREATE INDEX intervals_by_time ON activity_intervals(start_ms, end_ms);
CREATE INDEX intervals_by_end ON activity_intervals(end_ms);
PRAGMA user_version = 2;
```

Source-of-truth activity intervals are unchanged. All times are UTC Unix milliseconds. There are no daily aggregates and no cloud-dependent schema version.

## Startup sequence

1. Acquire the per-profile single-instance daemon lock. The database is opened **before any IPC listener binds or collector starts**.
2. Read SQLite's `PRAGMA user_version`. If its value is newer than the binary supports, stop with an explicit incompatibility error; never downgrade. For v0, require an **empty** schema; refuse nonempty unversioned databases instead of guessing.
3. Validate the expected v1/v2 table columns, duration/state constraints, and indexes. Before an actual upgrade, check SQLite integrity with `PRAGMA quick_check`.
4. On an existing v1 database, write a consistent online backup using SQLite's native backup interface, **including committed changes still in the WAL**. A fresh empty v0 database does not need a pre-migration snapshot.
5. Run all missing numbered SQL migrations and increment `user_version` inside **one immediate SQLite transaction**. Verify final schema and quick-check **before committing**. Any failure rolls back the entire sequence.
6. Enable WAL and then start the daemon's IPC and OS collectors. Subsequent starts at the same schema version are idempotent and create no additional backup.

Backups remain **on the user's own device** beside the database, with unique names:

```text
watcher.sqlite.pre-v1-to-v2-<timestamp-ms>-<pid>-<counter>.bak
```

Existing backups are never overwritten or auto-deleted. On Unix, database and backup files are mode `0600`, and the daemon's user data directory is mode `0700`. On Windows, files are kept within the user's `%LOCALAPPDATA%` profile. Backup size can approach the original database size; insufficient disk space fails the upgrade **before schema modification**.

## Failure and recovery

On migration or validation failure, `watcherd` aborts startup, reports a human-readable error, and **does not start tracking**. The existing database is not silently reset.

For manual recovery, first stop the daemon, preserve the database **and any `-wal`/`-shm` sidecar files** in a separate location, and choose the appropriate pre-upgrade `.bak` snapshot. Use SQLite's restore mechanism while Watcher is stopped, rather than copying only the main database during an active WAL session. Verify `PRAGMA integrity_check` and `PRAGMA user_version` before restarting. Restoring an old v1 snapshot into a new v2 binary will automatically rerun the same migration; a v1 binary must never write a v2 database.

## How to add v3 and beyond

1. Add an immutable `migrations/003_description.sql`, append it to the embedded ordered migration list, and increment `CURRENT_SCHEMA_VERSION`. Never edit already-released SQL migrations.
2. Extend `verify_structure` with the new expected schema. Prefer additive changes; for destructive table rebuilds, copy, check, and rename within the same transaction.
3. Retain historical fixtures. Test every supported upgrade chain, e.g. v1→v3 and v2→v3, plus fresh initialization, backups, idempotence, rollback when SQL succeeds but the final schema is wrong, and failure cases.
4. Include actual local SQLite WAL backups and integrity checks in tests. Never directly copy a live WAL database file as a backup.
5. On the next Watcher update, the new daemon applies migrations automatically at startup. Users don't have to run SQL.

## Verification performed

- Frozen historical v1 fixture upgraded to v2 **with another connection still holding uncheckpointed WAL changes**; all five existing intervals survived and the standalone backup remained readable at v1 with five rows.
- Fresh v0 initialization, repeated v2 startup without extra backup, Unix private file permissions.
- Explicit refusal of unversioned nonempty, partial/mislabeled v1/v2, and newer database versions without changing their schemas.
- A failing second migration and a syntactically successful but structurally incomplete migration both rolled back fully.
- Unavailable backup location fails without changing source v1.
- A real `watcherd --dev` startup using a disposable historical v1 file upgraded to v2, preserved five historical rows, and exposed working `watcher --dev status`; SQLite integrity check returned `ok`.

No CI workflow changes or production activity database changes were made for this work.
