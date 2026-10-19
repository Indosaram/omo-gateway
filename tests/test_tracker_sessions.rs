use chrono::{DateTime, Utc};
use omon_gateway::sessions::overview::Overview;
use omon_gateway::sessions::probe::{LiveSession, MachineSnapshot, PendingQuestion, QuestionItem};
use omon_gateway::sessions::registry::{HostError, SessionList};
use omon_gateway::storage::Database;
use omon_gateway::tracker::seed::ensure_seeded;
use omon_gateway::tracker::sessions::{shape, sync_from_overview, PREVIEW_CHARS};
use sqlx::SqlitePool;

fn live(id: &str, updated_at: &str) -> LiveSession {
    LiveSession {
        machine: "local".into(),
        session_id: id.into(),
        routing_id: id.into(),
        name: Some(format!("name-{id}")),
        cwd: Some("/Users/indo/code".into()),
        updated_at: Some(updated_at.into()),
        ..LiveSession::default()
    }
}

fn fixture() -> Overview {
    let idle_old = live("idle-old", "2026-10-19T00:00:00Z");
    let idle_new = LiveSession {
        last_assistant: "가".repeat(PREVIEW_CHARS + 50),
        ..live("idle-new", "2026-10-19T02:00:00Z")
    };
    let working = LiveSession {
        streaming: true,
        message_count: 7,
        ..live("working", "2026-10-19T01:00:00Z")
    };
    let question = LiveSession {
        questions: vec![PendingQuestion {
            request_id: "req-1".into(),
            questions: vec![QuestionItem {
                id: "q1".into(),
                header: None,
                question: Some("진행할까요?".into()),
                options: vec!["네".into(), "아니오".into()],
            }],
        }],
        ..live("question", "2026-10-18T00:00:00Z")
    };
    Overview {
        list: SessionList {
            sessions: Vec::new(),
            errors: vec![HostError {
                machine: "omarchy".into(),
                message: "listing timed out".into(),
                code: 3,
            }],
            stale: Vec::new(),
            exit_code: 0,
        },
        snapshots: vec![
            (
                "local".into(),
                Ok(MachineSnapshot {
                    machine: "local".into(),
                    sessions: vec![idle_old, working, question, idle_new],
                    errors: Vec::new(),
                }),
            ),
            ("omarchy".into(), Err("probe timed out".into())),
        ],
    }
}

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-10-19T03:00:00Z")
        .unwrap()
        .with_timezone(&Utc)
}

#[test]
fn sessions_sort_by_state_then_recent_activity() {
    let board = shape(&fixture(), now());
    let ids: Vec<&str> = board["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["session_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["question", "working", "idle-new", "idle-old"]);
    let states: Vec<&str> = board["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["state"].as_str().unwrap())
        .collect();
    assert_eq!(states, ["question", "working", "idle", "idle"]);
    assert_eq!(board["sessions"][0]["questions"][0]["request_id"], "req-1");
    assert_eq!(board["sessions"][1]["message_count"], 7);
    assert_eq!(board["sessions"][1]["streaming"], true);
}

#[test]
fn counts_cover_every_state() {
    let board = shape(&fixture(), now());
    assert_eq!(
        board["counts"],
        serde_json::json!({"working": 1, "question": 1, "blocked": 0, "idle": 2, "total": 4})
    );
    assert_eq!(board["collected_at"], now().to_rfc3339());
}

#[test]
fn last_messages_truncate_on_char_boundary() {
    let board = shape(&fixture(), now());
    let text = board["sessions"][2]["last_assistant"].as_str().unwrap();
    assert_eq!(text.chars().count(), PREVIEW_CHARS);
    assert!(text.chars().all(|c| c == '가'));
}

#[test]
fn machine_errors_are_surfaced_once() {
    let board = shape(&fixture(), now());
    let machines = board["machines"].as_array().unwrap();
    assert_eq!(machines.len(), 2);
    assert_eq!(machines[0]["machine"], "local");
    assert_eq!(machines[0]["ok"], true);
    assert!(machines[0]["error"].is_null());
    assert_eq!(machines[0]["session_count"], 4);
    assert_eq!(machines[1]["machine"], "omarchy");
    assert_eq!(machines[1]["ok"], false);
    assert_eq!(machines[1]["error"], "probe timed out");
    assert_eq!(machines[1]["session_count"], 0);
}

struct Db {
    _dir: tempfile::TempDir,
    database: Database,
}

impl Db {
    async fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let url = format!("sqlite://{}", dir.path().join("tracker.db").display());
        let database = Database::connect(&url).await.unwrap();
        ensure_seeded(database.pool()).await.unwrap();
        Self {
            _dir: dir,
            database,
        }
    }

    fn pool(&self) -> &SqlitePool {
        self.database.pool()
    }

    /// (title, session_state) of the issue mirroring `reference`.
    async fn issue(&self, reference: &str) -> (String, String) {
        sqlx::query_as(
            "SELECT i.title, s.session_state FROM tracker_issues i
             JOIN tracker_states s ON s.id = i.state_id WHERE i.session_ref = ?",
        )
        .bind(reference)
        .fetch_one(self.pool())
        .await
        .unwrap()
    }

    async fn session_activity(&self, reference: &str) -> Vec<(Option<String>, Option<String>)> {
        sqlx::query_as(
            "SELECT a.from_value, a.to_value FROM tracker_activity a
             JOIN tracker_issues i ON i.id = a.issue_id
             WHERE i.session_ref = ? AND a.kind = 'session_state' ORDER BY a.id",
        )
        .bind(reference)
        .fetch_all(self.pool())
        .await
        .unwrap()
    }
}

fn on(machine: &str, sessions: Vec<LiveSession>) -> (String, Result<MachineSnapshot, String>) {
    (
        machine.into(),
        Ok(MachineSnapshot {
            machine: machine.into(),
            sessions: sessions
                .into_iter()
                .map(|session| LiveSession {
                    machine: machine.into(),
                    ..session
                })
                .collect(),
            errors: Vec::new(),
        }),
    )
}

fn overview(snapshots: Vec<(String, Result<MachineSnapshot, String>)>) -> Overview {
    Overview {
        list: SessionList {
            sessions: Vec::new(),
            errors: Vec::new(),
            stale: Vec::new(),
            exit_code: 0,
        },
        snapshots,
    }
}

fn three_sessions() -> Vec<(String, Result<MachineSnapshot, String>)> {
    let working = LiveSession {
        streaming: true,
        ..live("working", "2026-10-19T01:00:00Z")
    };
    let transcript_named = LiveSession {
        name: Some("/Users/indo/.omo/sessions/abc.jsonl".into()),
        cwd: Some("/Users/indo/code/project/omon-gateway".into()),
        ..live("transcript", "2026-10-19T01:00:00Z")
    };
    let remote = LiveSession {
        blocking_question: true,
        ..live("remote", "2026-10-19T01:00:00Z")
    };
    vec![
        on("local", vec![working, transcript_named]),
        on("omarchy", vec![remote]),
    ]
}

#[tokio::test]
async fn sync_seeds_sessions_team_with_five_states() {
    let db = Db::new().await;
    sync_from_overview(db.pool(), &overview(Vec::new()), now())
        .await
        .unwrap();
    sync_from_overview(db.pool(), &overview(Vec::new()), now())
        .await
        .unwrap();
    let system: bool = sqlx::query_scalar("SELECT system FROM tracker_teams WHERE key = 'SES'")
        .fetch_one(db.pool())
        .await
        .unwrap();
    assert!(system);
    let states: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT s.name, s.type, s.session_state FROM tracker_states s
         JOIN tracker_teams t ON t.id = s.team_id WHERE t.key = 'SES' ORDER BY s.position",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    let expected = [
        ("Needs answer", "started", "question"),
        ("Blocked", "started", "blocked"),
        ("Working", "started", "working"),
        ("Idle", "unstarted", "idle"),
        ("Ended", "completed", "ended"),
    ];
    assert_eq!(states.len(), expected.len());
    for ((name, kind, session), (want_name, want_kind, want_session)) in states.iter().zip(expected)
    {
        assert_eq!(
            (name.as_str(), kind.as_str(), session.as_str()),
            (want_name, want_kind, want_session)
        );
    }
}

#[tokio::test]
async fn sync_mirrors_sessions_as_issues() {
    let db = Db::new().await;
    sync_from_overview(db.pool(), &overview(three_sessions()), now())
        .await
        .unwrap();
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM tracker_issues WHERE session_ref IS NOT NULL")
            .fetch_one(db.pool())
            .await
            .unwrap();
    assert_eq!(count, 3);
    assert_eq!(
        db.issue("local:working").await,
        ("name-working".into(), "working".into())
    );
    assert_eq!(
        db.issue("local:transcript").await,
        ("omon-gateway".into(), "idle".into())
    );
    assert_eq!(
        db.issue("omarchy:remote").await,
        ("name-remote".into(), "blocked".into())
    );
    let identifiers: Vec<String> = sqlx::query_scalar(
        "SELECT identifier FROM tracker_issues WHERE session_ref IS NOT NULL ORDER BY number",
    )
    .fetch_all(db.pool())
    .await
    .unwrap();
    assert_eq!(identifiers, ["SES-1", "SES-2", "SES-3"]);
    let assignee_model: String = sqlx::query_scalar(
        "SELECT u.model FROM tracker_issues i JOIN tracker_users u ON u.id = i.assignee_id
         WHERE i.session_ref = 'omarchy:remote'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(assignee_model, "omo@omarchy");
}

#[tokio::test]
async fn vanished_sessions_end_only_on_machines_that_answered() {
    let db = Db::new().await;
    sync_from_overview(db.pool(), &overview(three_sessions()), now())
        .await
        .unwrap();

    let working = LiveSession {
        streaming: true,
        ..live("working", "2026-10-19T02:00:00Z")
    };
    let second = overview(vec![
        on("local", vec![working]),
        ("omarchy".into(), Err("probe timed out".into())),
    ]);
    sync_from_overview(db.pool(), &second, now()).await.unwrap();

    assert_eq!(db.issue("local:transcript").await.1, "ended");
    assert_eq!(db.issue("local:working").await.1, "working");
    assert_eq!(db.issue("omarchy:remote").await.1, "blocked");
    let completed: Option<String> = sqlx::query_scalar(
        "SELECT completed_at FROM tracker_issues WHERE session_ref = 'local:transcript'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert!(completed.is_some());

    // The session comes back: the issue returns to its live state.
    sync_from_overview(db.pool(), &overview(three_sessions()), now())
        .await
        .unwrap();
    assert_eq!(db.issue("local:transcript").await.1, "idle");
}

#[tokio::test]
async fn state_change_writes_one_activity_row() {
    let db = Db::new().await;
    sync_from_overview(db.pool(), &overview(three_sessions()), now())
        .await
        .unwrap();
    let before = db.session_activity("local:working").await;
    assert_eq!(before, [(None, Some("working".to_string()))]);

    // Unchanged state: no new row.
    sync_from_overview(db.pool(), &overview(three_sessions()), now())
        .await
        .unwrap();
    assert_eq!(db.session_activity("local:working").await.len(), 1);

    let asking = LiveSession {
        questions: vec![PendingQuestion {
            request_id: "req-9".into(),
            questions: Vec::new(),
        }],
        ..live("working", "2026-10-19T02:00:00Z")
    };
    let mut snapshots = three_sessions();
    snapshots[0] = on(
        "local",
        vec![
            asking,
            LiveSession {
                name: Some("/Users/indo/.omo/sessions/abc.jsonl".into()),
                cwd: Some("/Users/indo/code/project/omon-gateway".into()),
                ..live("transcript", "2026-10-19T01:00:00Z")
            },
        ],
    );
    sync_from_overview(db.pool(), &overview(snapshots), now())
        .await
        .unwrap();
    let after = db.session_activity("local:working").await;
    assert_eq!(after.len(), 2);
    assert_eq!(
        after[1],
        (Some("working".to_string()), Some("question".to_string()))
    );
    let notified: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM tracker_notifications n JOIN tracker_issues i ON i.id = n.issue_id
         WHERE i.session_ref = 'local:working' AND n.kind = 'session_question'",
    )
    .fetch_one(db.pool())
    .await
    .unwrap();
    assert_eq!(notified, 1);
}
