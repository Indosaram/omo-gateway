CREATE TABLE IF NOT EXISTS lanes (
    key TEXT PRIMARY KEY CHECK(length(key) BETWEEN 1 AND 48 AND key NOT GLOB '*[^a-z0-9-]*'),
    title TEXT NOT NULL,
    lead_session_key TEXT NOT NULL,
    thread_channel_id TEXT,
    parent_channel_id TEXT,
    status_message_id TEXT,
    cwd TEXT NOT NULL,
    brief_path TEXT NOT NULL,
    session_key TEXT NOT NULL,
    omo_thread_id TEXT,
    model TEXT,
    status TEXT NOT NULL CHECK(status IN ('queued','working','blocked','done_claimed','not_done','verified_done','closed','failed')),
    done_signals TEXT NOT NULL,
    claim_evidence TEXT,
    claim_at DATETIME,
    objection TEXT,
    turns_run INTEGER NOT NULL DEFAULT 0,
    last_turn_outcome TEXT,
    last_report TEXT,
    last_activity_at DATETIME NOT NULL,
    stale_posted_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS lane_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    at DATETIME NOT NULL,
    from_status TEXT,
    to_status TEXT NOT NULL,
    reason TEXT
);

CREATE TABLE IF NOT EXISTS lane_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT,
    lead_session_key TEXT NOT NULL,
    kind TEXT NOT NULL,
    body TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    delivered_at DATETIME
);

CREATE TABLE IF NOT EXISTS lane_transcript (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL,
    at DATETIME NOT NULL,
    role TEXT NOT NULL,
    text TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_lanes_status
    ON lanes(status);

CREATE INDEX IF NOT EXISTS idx_lane_events_delivered_lead
    ON lane_events(delivered_at, lead_session_key);

CREATE INDEX IF NOT EXISTS idx_lane_transcript_lane_id
    ON lane_transcript(lane_key, id);
