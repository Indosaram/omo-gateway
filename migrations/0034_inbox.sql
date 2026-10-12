CREATE TABLE IF NOT EXISTS inbox_items (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage')),
    source_ref TEXT NOT NULL,
    thread_ref TEXT,
    author TEXT,
    title TEXT,
    snippet TEXT NOT NULL,
    received_at DATETIME NOT NULL,
    link TEXT,
    raw_hash TEXT NOT NULL,
    collected_at DATETIME NOT NULL,
    UNIQUE(source, source_ref)
);

CREATE TABLE IF NOT EXISTS inbox_decisions (
    item_id TEXT PRIMARY KEY REFERENCES inbox_items(id),
    class TEXT NOT NULL CHECK(class IN ('owner','bot','fyi','ignore')),
    reason TEXT NOT NULL,
    due_at DATETIME,
    proposed_action TEXT,
    lane_key TEXT,
    status TEXT NOT NULL CHECK(status IN ('open','proposed','approved','done','dismissed')),
    decided_at DATETIME NOT NULL,
    model TEXT
);

CREATE TABLE IF NOT EXISTS inbox_cursors (
    source TEXT PRIMARY KEY,
    cursor TEXT NOT NULL,
    updated_at DATETIME NOT NULL,
    error TEXT
);

CREATE TABLE IF NOT EXISTS inbox_filter_stats (
    day TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage')),
    rule TEXT NOT NULL,
    count INTEGER NOT NULL CHECK(count >= 0),
    PRIMARY KEY(day, source, rule)
);
