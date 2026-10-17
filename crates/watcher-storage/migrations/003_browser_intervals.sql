-- Browser-observed intervals are separate from OS-verified active time.
-- Never add these durations to device active totals directly.
CREATE TABLE browser_intervals (
  source_id TEXT NOT NULL,
  seq INTEGER NOT NULL CHECK(seq > 0),
  start_ms INTEGER NOT NULL,
  end_ms INTEGER NOT NULL CHECK(end_ms > start_ms),
  domain TEXT NOT NULL,
  PRIMARY KEY (source_id, seq)
) WITHOUT ROWID;
CREATE INDEX browser_by_time ON browser_intervals(start_ms, end_ms);
