CREATE TABLE session_watches (
    target TEXT PRIMARY KEY,
    added_at DATETIME NOT NULL,
    reason TEXT NOT NULL DEFAULT 'manual'
);

CREATE TABLE session_watch_notices (
    dedupe_key TEXT PRIMARY KEY,
    target TEXT NOT NULL,
    kind TEXT NOT NULL,
    sent_at DATETIME NOT NULL
);
