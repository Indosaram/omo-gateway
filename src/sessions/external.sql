ALTER TABLE lanes ADD COLUMN kind TEXT NOT NULL DEFAULT 'managed' CHECK(kind IN ('managed','external'));
ALTER TABLE lanes ADD COLUMN external_target TEXT;
CREATE UNIQUE INDEX idx_lanes_external_target ON lanes(external_target) WHERE external_target IS NOT NULL;
CREATE TABLE external_sessions (
    lane_key TEXT PRIMARY KEY REFERENCES lanes(key),
    goal TEXT NOT NULL,
    send_key TEXT NOT NULL,
    sent_at DATETIME,
    cursor TEXT,
    released_at DATETIME
);
CREATE TABLE external_session_items (
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    item_id TEXT NOT NULL,
    PRIMARY KEY(lane_key, item_id)
);
