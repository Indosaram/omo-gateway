PRAGMA defer_foreign_keys=ON;
CREATE TEMP TABLE inbox_items_hold AS SELECT * FROM inbox_items;
DROP TABLE inbox_items;
CREATE TABLE inbox_items (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage','cron','slack')),
    source_ref TEXT NOT NULL,
    thread_ref TEXT,
    author TEXT,
    title TEXT,
    snippet TEXT NOT NULL,
    received_at DATETIME NOT NULL,
    link TEXT,
    raw_hash TEXT NOT NULL,
    collected_at DATETIME NOT NULL,
    UNIQUE(source,source_ref)
);
INSERT INTO inbox_items SELECT * FROM inbox_items_hold;
DROP TABLE inbox_items_hold;

CREATE TABLE inbox_filter_stats_slack (
    day TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage','cron','slack')),
    rule TEXT NOT NULL,
    count INTEGER NOT NULL CHECK(count >= 0),
    PRIMARY KEY(day, source, rule)
);
INSERT INTO inbox_filter_stats_slack SELECT * FROM inbox_filter_stats;
DROP TABLE inbox_filter_stats;
ALTER TABLE inbox_filter_stats_slack RENAME TO inbox_filter_stats;

CREATE TABLE IF NOT EXISTS slack_cursors (
    bot_id TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    thread_ts TEXT NOT NULL DEFAULT '',
    last_ts TEXT NOT NULL,
    updated_at DATETIME NOT NULL,
    PRIMARY KEY(bot_id, channel_id, thread_ts)
);

CREATE TABLE IF NOT EXISTS slack_dead_targets (
    channel_id TEXT PRIMARY KEY,
    reason TEXT NOT NULL,
    marked_at DATETIME NOT NULL,
    probed_at DATETIME
);

CREATE TABLE IF NOT EXISTS slack_paired_users (
    user_id TEXT PRIMARY KEY,
    paired_at DATETIME NOT NULL,
    operator_id TEXT NOT NULL
);
