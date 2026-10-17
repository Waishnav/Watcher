-- Speeds up lookups for the recent portion of a large activity history.
-- No interval data is rewritten; existing v1 rows remain unchanged.
CREATE INDEX intervals_by_end ON activity_intervals(end_ms);
