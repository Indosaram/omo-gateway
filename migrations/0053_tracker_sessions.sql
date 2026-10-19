ALTER TABLE tracker_issues ADD COLUMN session_ref TEXT;
CREATE UNIQUE INDEX IF NOT EXISTS idx_tracker_issues_session_ref
    ON tracker_issues(session_ref) WHERE session_ref IS NOT NULL;
ALTER TABLE tracker_states ADD COLUMN session_state TEXT;
