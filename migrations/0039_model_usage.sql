CREATE TABLE model_usage (
    id TEXT PRIMARY KEY,
    session_key TEXT NOT NULL,
    purpose TEXT NOT NULL,
    model TEXT NOT NULL,
    input_tokens INTEGER NOT NULL CHECK(input_tokens >= 0),
    output_tokens INTEGER NOT NULL CHECK(output_tokens >= 0),
    cached_tokens INTEGER NOT NULL CHECK(cached_tokens >= 0),
    created_at DATETIME NOT NULL
);
CREATE INDEX model_usage_created_at ON model_usage(created_at);
CREATE TABLE lane_usage_context (
    lane_key TEXT PRIMARY KEY REFERENCES lanes(key),
    start_commit TEXT,
    purpose TEXT NOT NULL CHECK(purpose IN ('lane_small', 'lane_large'))
);
