CREATE TABLE IF NOT EXISTS source_health (
    source TEXT PRIMARY KEY,
    state TEXT NOT NULL,
    detail TEXT NOT NULL,
    since DATETIME NOT NULL,
    notified_on TEXT
);
