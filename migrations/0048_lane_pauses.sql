-- Separate table: rebuilding lanes would have to carry its later columns, partial index and FK children.
CREATE TABLE lane_pauses (
    lane_key TEXT PRIMARY KEY REFERENCES lanes(key),
    paused_at DATETIME NOT NULL,
    paused_by TEXT NOT NULL,
    resumed_at DATETIME
);
