use chrono::{DateTime, Utc};
use omon_gateway::sessions::overview::Overview;
use omon_gateway::sessions::probe::{LiveSession, MachineSnapshot, PendingQuestion, QuestionItem};
use omon_gateway::sessions::registry::{HostError, SessionList};
use omon_gateway::tracker::sessions::{shape, PREVIEW_CHARS};

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
