-- Representative on-disk database produced by the original Phase 1 Store::open.
-- Keep this fixture frozen: migration tests must not generate v1 using v2 code.
CREATE TABLE IF NOT EXISTS activity_intervals (
    id INTEGER PRIMARY KEY,
    start_ms INTEGER NOT NULL,
    end_ms INTEGER NOT NULL CHECK(end_ms > start_ms),
    app TEXT,
    state TEXT NOT NULL CHECK(state IN ('active','idle','unknown','paused'))
);
CREATE INDEX IF NOT EXISTS intervals_by_time
ON activity_intervals(start_ms, end_ms);
PRAGMA user_version = 1;
INSERT INTO activity_intervals(start_ms,end_ms,app,state) VALUES
  (1000, 2500, 'firefox', 'active'),
  (2500, 3000, NULL, 'idle'),
  (3000, 4500, 'terminal', 'active'),
  (4500, 5000, NULL, 'unknown'),
  (5000, 6000, NULL, 'paused');
