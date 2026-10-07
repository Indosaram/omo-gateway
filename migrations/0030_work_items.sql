CREATE TABLE IF NOT EXISTS work_items (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    thread_id TEXT,
    delegate_pane TEXT,
    status TEXT NOT NULL DEFAULT 'in_progress',
    last_progress_at DATETIME NOT NULL,
    receipt_path TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_work_items_status_progress
    ON work_items(status, last_progress_at);
