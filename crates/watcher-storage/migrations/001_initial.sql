-- Historical Watcher v1 database format. Must match the originally shipped v1.
CREATE TABLE activity_intervals (
    id INTEGER PRIMARY KEY,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL CHECK(end_ms > start_ms),
    app TEXT,
    state TEXT NOT NULL CHECK(state IN ('active','idle','unknown','paused'))
);
CREATE INDEX intervals_by_time ON activity_intervals(start_ms,end_ms);
