-- One row per action-loop item. `enqueued` only means the item was written to lane_events for
-- the lead; the agent moves it to `executed`, `verified` (with an external result id such as a
-- calendar event id) or `failed`. The item copy lets the loop re-queue it when the lead turn
-- produced no outcome.
CREATE TABLE action_loop_outcomes (
    item_key TEXT PRIMARY KEY,
    lane_event_id INTEGER,
    kind TEXT NOT NULL,
    source TEXT NOT NULL,
    line TEXT NOT NULL,
    link TEXT,
    status TEXT NOT NULL CHECK (status IN ('enqueued', 'executed', 'verified', 'failed')),
    external_id TEXT,
    detail TEXT,
    attempts INTEGER NOT NULL DEFAULT 1,
    enqueued_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);
CREATE INDEX idx_action_loop_outcomes_status ON action_loop_outcomes(status);
