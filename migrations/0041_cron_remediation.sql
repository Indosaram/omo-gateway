PRAGMA defer_foreign_keys=ON;
CREATE TABLE inbox_items_cron (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL CHECK(source IN ('mail','kakao','discord','imessage','cron')),
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
INSERT INTO inbox_items_cron SELECT * FROM inbox_items;
DROP TABLE inbox_items;
ALTER TABLE inbox_items_cron RENAME TO inbox_items;
CREATE TABLE cron_remediation_outcomes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    problem_id INTEGER NOT NULL REFERENCES cron_problems(id),
    job_id TEXT NOT NULL,
    message TEXT NOT NULL,
    created_at DATETIME NOT NULL,
    delivered_at DATETIME,
    UNIQUE(problem_id,message)
);
