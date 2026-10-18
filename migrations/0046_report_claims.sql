CREATE TABLE report_claims (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_key TEXT NOT NULL,
    platform_message_id TEXT NOT NULL,
    sentence TEXT NOT NULL,
    argv_json TEXT NOT NULL,
    expect_regex TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'confirmed', 'corrected', 'unmeasurable', 'refused')),
    measured_output TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    checked_at DATETIME
);

CREATE INDEX idx_report_claims_state_created ON report_claims(state, created_at);
