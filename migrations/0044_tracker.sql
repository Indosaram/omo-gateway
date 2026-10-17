CREATE TABLE IF NOT EXISTS tracker_users (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    email TEXT,
    avatar_color TEXT NOT NULL,
    kind TEXT NOT NULL CHECK(kind IN ('human', 'agent')),
    model TEXT,
    is_me INTEGER NOT NULL DEFAULT 0 CHECK(is_me IN (0, 1)),
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_teams (
    id TEXT PRIMARY KEY,
    key TEXT UNIQUE NOT NULL CHECK(key GLOB '[A-Z]*' AND length(key) BETWEEN 1 AND 5),
    name TEXT NOT NULL,
    icon TEXT NOT NULL,
    color TEXT NOT NULL,
    issue_seq INTEGER NOT NULL DEFAULT 0,
    system INTEGER NOT NULL DEFAULT 0 CHECK(system IN (0, 1)),
    cycles_enabled INTEGER NOT NULL DEFAULT 0 CHECK(cycles_enabled IN (0, 1)),
    cycle_weeks INTEGER NOT NULL DEFAULT 2,
    triage_enabled INTEGER NOT NULL DEFAULT 0 CHECK(triage_enabled IN (0, 1)),
    created_at TEXT NOT NULL,
    sort_order REAL NOT NULL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS tracker_states (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES tracker_teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('triage', 'backlog', 'unstarted', 'started', 'completed', 'canceled')),
    color TEXT NOT NULL,
    position REAL NOT NULL DEFAULT 0.0,
    lane_status TEXT CHECK(lane_status IS NULL OR lane_status IN ('queued', 'working', 'blocked', 'done_claimed', 'not_done', 'verified_done', 'closed', 'failed')),
    description TEXT,
    UNIQUE(team_id, lane_status)
);

CREATE TABLE IF NOT EXISTS tracker_labels (
    id TEXT PRIMARY KEY,
    team_id TEXT REFERENCES tracker_teams(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    parent_id TEXT REFERENCES tracker_labels(id) ON DELETE SET NULL,
    description TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    summary TEXT NOT NULL DEFAULT '',
    icon TEXT NOT NULL DEFAULT 'box',
    color TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('backlog', 'planned', 'started', 'paused', 'completed', 'canceled')),
    priority INTEGER NOT NULL DEFAULT 0 CHECK(priority BETWEEN 0 AND 4),
    lead_id TEXT REFERENCES tracker_users(id) ON DELETE SET NULL,
    start_date TEXT,
    target_date TEXT,
    sort_order REAL NOT NULL DEFAULT 0.0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived_at TEXT
);

CREATE TABLE IF NOT EXISTS tracker_project_teams (
    project_id TEXT NOT NULL REFERENCES tracker_projects(id) ON DELETE CASCADE,
    team_id TEXT NOT NULL REFERENCES tracker_teams(id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, team_id)
);

CREATE TABLE IF NOT EXISTS tracker_project_members (
    project_id TEXT NOT NULL REFERENCES tracker_projects(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, user_id)
);

CREATE TABLE IF NOT EXISTS tracker_project_updates (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES tracker_projects(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    health TEXT NOT NULL CHECK(health IN ('on_track', 'at_risk', 'off_track')),
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_milestones (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES tracker_projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    target_date TEXT,
    sort_order REAL NOT NULL DEFAULT 0.0
);

CREATE TABLE IF NOT EXISTS tracker_cycles (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES tracker_teams(id) ON DELETE CASCADE,
    number INTEGER NOT NULL,
    name TEXT,
    description TEXT,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    completed_at TEXT,
    UNIQUE(team_id, number)
);

CREATE TABLE IF NOT EXISTS tracker_issues (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL REFERENCES tracker_teams(id) ON DELETE RESTRICT,
    number INTEGER NOT NULL,
    identifier TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    state_id TEXT NOT NULL REFERENCES tracker_states(id) ON DELETE RESTRICT,
    priority INTEGER NOT NULL DEFAULT 0 CHECK(priority BETWEEN 0 AND 4),
    estimate INTEGER,
    assignee_id TEXT REFERENCES tracker_users(id) ON DELETE SET NULL,
    creator_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE RESTRICT,
    project_id TEXT REFERENCES tracker_projects(id) ON DELETE SET NULL,
    milestone_id TEXT REFERENCES tracker_milestones(id) ON DELETE SET NULL,
    cycle_id TEXT REFERENCES tracker_cycles(id) ON DELETE SET NULL,
    parent_id TEXT REFERENCES tracker_issues(id) ON DELETE SET NULL,
    due_date TEXT,
    sort_order REAL NOT NULL DEFAULT 0.0,
    lane_key TEXT UNIQUE REFERENCES lanes(key) ON DELETE SET NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT,
    canceled_at TEXT,
    archived_at TEXT,
    snoozed_until TEXT
);

CREATE TABLE IF NOT EXISTS tracker_issue_labels (
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    label_id TEXT NOT NULL REFERENCES tracker_labels(id) ON DELETE CASCADE,
    PRIMARY KEY (issue_id, label_id)
);

CREATE TABLE IF NOT EXISTS tracker_subscribers (
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    PRIMARY KEY (issue_id, user_id)
);

CREATE TABLE IF NOT EXISTS tracker_comments (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    body TEXT NOT NULL,
    parent_id TEXT REFERENCES tracker_comments(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    edited INTEGER NOT NULL DEFAULT 0 CHECK(edited IN (0, 1)),
    resolved_at TEXT
);

CREATE TABLE IF NOT EXISTS tracker_reactions (
    comment_id TEXT NOT NULL REFERENCES tracker_comments(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    PRIMARY KEY (comment_id, user_id, emoji)
);

CREATE TABLE IF NOT EXISTS tracker_relations (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    related_issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    type TEXT NOT NULL CHECK(type IN ('blocks', 'related', 'duplicate')),
    created_at TEXT NOT NULL,
    UNIQUE(issue_id, related_issue_id, type)
);

CREATE TABLE IF NOT EXISTS tracker_links (
    id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_activity (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    issue_id TEXT NOT NULL REFERENCES tracker_issues(id) ON DELETE CASCADE,
    actor_id TEXT REFERENCES tracker_users(id) ON DELETE SET NULL,
    kind TEXT NOT NULL,
    from_value TEXT,
    to_value TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_views (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    icon TEXT NOT NULL DEFAULT 'layers',
    color TEXT,
    owner_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    team_id TEXT REFERENCES tracker_teams(id) ON DELETE CASCADE,
    filters TEXT NOT NULL DEFAULT '{}',
    display TEXT NOT NULL DEFAULT '{}',
    shared INTEGER NOT NULL DEFAULT 1 CHECK(shared IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracker_favorites (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK(kind IN ('issue', 'project', 'view', 'cycle', 'team', 'label')),
    target_id TEXT NOT NULL,
    sort_order REAL NOT NULL DEFAULT 0.0,
    UNIQUE(user_id, kind, target_id)
);

CREATE TABLE IF NOT EXISTS tracker_notifications (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES tracker_users(id) ON DELETE CASCADE,
    issue_id TEXT REFERENCES tracker_issues(id) ON DELETE CASCADE,
    project_id TEXT REFERENCES tracker_projects(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    actor_id TEXT REFERENCES tracker_users(id) ON DELETE SET NULL,
    body TEXT NOT NULL,
    created_at TEXT NOT NULL,
    read_at TEXT,
    snoozed_until TEXT,
    archived_at TEXT
);

CREATE TABLE IF NOT EXISTS tracker_meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_tracker_issues_team ON tracker_issues(team_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_state ON tracker_issues(state_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_assignee ON tracker_issues(assignee_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_project ON tracker_issues(project_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_cycle ON tracker_issues(cycle_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_parent ON tracker_issues(parent_id);
CREATE INDEX IF NOT EXISTS idx_tracker_issues_lane_key ON tracker_issues(lane_key);
CREATE INDEX IF NOT EXISTS idx_tracker_activity_issue ON tracker_activity(issue_id);
CREATE INDEX IF NOT EXISTS idx_tracker_comments_issue ON tracker_comments(issue_id);
CREATE INDEX IF NOT EXISTS idx_tracker_notifications_user_created ON tracker_notifications(user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_tracker_states_team ON tracker_states(team_id);
CREATE INDEX IF NOT EXISTS idx_tracker_labels_team ON tracker_labels(team_id);
CREATE INDEX IF NOT EXISTS idx_tracker_relations_related ON tracker_relations(related_issue_id);
