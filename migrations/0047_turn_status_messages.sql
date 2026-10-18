CREATE TABLE IF NOT EXISTS turn_status_messages (
    session_key TEXT NOT NULL,
    event_id TEXT NOT NULL,
    platform_message_id TEXT NOT NULL,
    started_at DATETIME NOT NULL,
    state TEXT NOT NULL DEFAULT 'running' CHECK (state IN ('running', 'done', 'failed')),
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (session_key, event_id)
);
