PRAGMA defer_foreign_keys=ON;
CREATE TEMP TABLE inbox_items_hold AS SELECT * FROM inbox_items;
DROP TABLE inbox_items;
CREATE TABLE inbox_items (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage','cron','slack','github')),
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

CREATE TABLE inbox_filter_stats_github (
    day TEXT NOT NULL,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage','cron','slack','github')),
    rule TEXT NOT NULL,
    count INTEGER NOT NULL CHECK(count >= 0),
    PRIMARY KEY(day, source, rule)
);
INSERT INTO inbox_filter_stats_github SELECT * FROM inbox_filter_stats;
DROP TABLE inbox_filter_stats;
ALTER TABLE inbox_filter_stats_github RENAME TO inbox_filter_stats;
