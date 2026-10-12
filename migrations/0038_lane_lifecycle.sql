CREATE UNIQUE INDEX lane_unique_thread ON lanes(thread_channel_id)
    WHERE thread_channel_id IS NOT NULL;
CREATE TABLE lane_thread_bindings (
    lane_key TEXT PRIMARY KEY REFERENCES lanes(key),
    bot_id TEXT NOT NULL,
    bound_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE lane_close_intents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    generation INTEGER NOT NULL,
    original_status TEXT NOT NULL,
    force INTEGER NOT NULL,
    note TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending','running','failed','completed')),
    phase INTEGER NOT NULL DEFAULT 0,
    attempt INTEGER NOT NULL DEFAULT 1,
    previous_cwd TEXT NOT NULL,
    removal TEXT,
    error TEXT,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(lane_key, generation)
);
CREATE TABLE lane_owned_worktrees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    path TEXT NOT NULL UNIQUE,
    role TEXT NOT NULL CHECK(role IN ('cwd','auxiliary')),
    repo TEXT NOT NULL,
    common_dir TEXT NOT NULL,
    git_dir TEXT NOT NULL,
    identity TEXT NOT NULL,
    branch TEXT NOT NULL,
    registrar TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'registered'
        CHECK(state IN ('registered','removing','removed','restoring')),
    removal_intent INTEGER REFERENCES lane_close_intents(id),
    removal_identity TEXT,
    removal_receipt TEXT,
    restoration_receipt TEXT,
    registered_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE TABLE lane_owner_replies (
    message_id TEXT PRIMARY KEY,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    bot_id TEXT,
    author_id TEXT NOT NULL,
    text TEXT,
    generation INTEGER NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending' CHECK(state IN ('pending','applied','failed')),
    reopen_history_id INTEGER REFERENCES lane_history(id),
    error TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
