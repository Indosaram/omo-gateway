CREATE TABLE lane_inbox (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    text TEXT NOT NULL,
    consumed_at DATETIME
);
CREATE INDEX idx_lane_inbox_pending ON lane_inbox(lane_key, consumed_at, id);
