CREATE TABLE IF NOT EXISTS cron_problems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    run_id TEXT,
    signature TEXT NOT NULL,
    detail TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('open','remediating','fixed','needs_owner','ignored')),
    lane_key TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    UNIQUE(job_id, kind, signature)
);
