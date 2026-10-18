CREATE TABLE action_proposals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    origin TEXT NOT NULL CHECK (origin IN ('engine_approval', 'tool_approval', 'inbox', 'github', 'loop', 'yolo')),
    session_key TEXT NOT NULL,
    signature TEXT NOT NULL,
    effect TEXT NOT NULL,
    target TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    payload_json TEXT NOT NULL DEFAULT '{}',
    tainted INTEGER NOT NULL DEFAULT 0,
    state TEXT NOT NULL DEFAULT 'pending' CHECK (state IN ('pending', 'approved', 'declined', 'expired', 'auto_allowed', 'auto_denied')),
    precedent_id INTEGER REFERENCES action_precedents(id),
    owner_words TEXT,
    platform_message_id TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    decided_at DATETIME
);

CREATE INDEX idx_action_proposals_state_session ON action_proposals(state, session_key);
CREATE INDEX idx_action_proposals_effect_target ON action_proposals(effect, target, state);

CREATE TABLE action_precedents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    signature TEXT NOT NULL,
    effect TEXT NOT NULL,
    target_scope TEXT NOT NULL DEFAULT '',
    decision TEXT NOT NULL CHECK (decision IN ('allow', 'deny')),
    scope TEXT NOT NULL CHECK (scope IN ('exact', 'class')),
    approvals INTEGER NOT NULL DEFAULT 0,
    declines INTEGER NOT NULL DEFAULT 0,
    origin TEXT NOT NULL,
    owner_words TEXT,
    revoked_at DATETIME,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (signature, scope)
);

CREATE INDEX idx_action_precedents_class ON action_precedents(scope, effect, target_scope);

-- NULL = deliver now; a future value holds the event (quiet hours).
ALTER TABLE lane_events ADD COLUMN release_at DATETIME;

-- approval_allowlist stays in place, unused, for rollback.
INSERT OR IGNORE INTO action_precedents
    (signature, effect, target_scope, decision, scope, approvals, declines, origin, created_at, updated_at)
SELECT pattern_key, 'unknown', '', 'allow', 'exact', 1, 0, 'legacy_allowlist', created_at, created_at
FROM approval_allowlist;
