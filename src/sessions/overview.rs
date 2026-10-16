use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::control::SessionControl;
use super::probe::{LiveSession, MachineSnapshot, PendingQuestion, Probe};
use super::registry::{CommandRunner, SessionList};

pub const DISCORD_LIMIT: usize = 1900;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub list: SessionList,
    pub snapshots: Vec<(String, Result<MachineSnapshot, String>)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Working,
    Question,
    Blocked,
    Idle,
}

pub fn state_of(s: &LiveSession) -> SessionState {
    if !s.questions.is_empty() {
        SessionState::Question
    } else if s.blocking_question {
        SessionState::Blocked
    } else if s.streaming || s.compacting {
        SessionState::Working
    } else {
        SessionState::Idle
    }
}

pub fn compact(value: &str, max: usize) -> String {
    value
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}

pub fn home_relative(folder: &str) -> String {
    for prefix in ["/Users/", "/home/"] {
        if let Some(rest) = folder.strip_prefix(prefix) {
            return match rest.split_once('/') {
                Some((_, suffix)) => format!("~/{suffix}"),
                None => "~".into(),
            };
        }
    }
    if folder == "/root" {
        return "~".into();
    }
    folder
        .strip_prefix("/root/")
        .map_or_else(|| folder.to_owned(), |rest| format!("~/{rest}"))
}

pub fn activity(value: Option<&str>, now: DateTime<Utc>) -> String {
    let Some(at) = value.and_then(|v| DateTime::parse_from_rfc3339(v).ok()) else {
        return "활동 미상 (KST)".into();
    };
    let seconds = now.signed_duration_since(at).num_seconds().max(0);
    let relative = if seconds < 60 {
        format!("{seconds}초 전")
    } else if seconds < 3600 {
        format!("{}분 전", seconds / 60)
    } else if seconds < 86400 {
        format!("{}시간 전", seconds / 3600)
    } else {
        format!("{}일 전", seconds / 86400)
    };
    format!("{relative} (KST)")
}

fn collapse_newlines(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn format_question_summary(question: &PendingQuestion) -> String {
    if let Some(first) = question.questions.first() {
        let text = first
            .question
            .as_deref()
            .or(first.header.as_deref())
            .unwrap_or("질문 대기 중");
        let collapsed = collapse_newlines(text);
        if first.options.is_empty() {
            compact(&collapsed, 90)
        } else {
            let opts = first
                .options
                .iter()
                .map(|opt| format!("[{}]", collapse_newlines(opt)))
                .collect::<Vec<_>>()
                .join(" ");
            compact(&format!("{collapsed} {opts}"), 90)
        }
    } else {
        "질문 대기 중".into()
    }
}

/// Session ids share a time-ordered prefix, so 13 characters are needed to tell apart
/// sessions started minutes apart; the result is a valid unique-prefix target.
pub fn short_id(id: &str) -> &str {
    id.char_indices().nth(13).map_or(id, |(end, _)| &id[..end])
}

pub fn format_overview(o: &Overview, now: DateTime<Utc>, max_chars: usize) -> String {
    let mut machines = Vec::<String>::new();
    let mut add_machine = |m: &str| {
        if !machines.iter().any(|seen| seen == m) {
            machines.push(m.to_owned());
        }
    };

    for (m, _) in &o.snapshots {
        add_machine(m);
    }
    for s in &o.list.sessions {
        add_machine(&s.machine);
    }
    for e in &o.list.errors {
        add_machine(&e.machine);
    }
    for s in &o.list.stale {
        add_machine(&s.machine);
    }

    if machines.is_empty() {
        machines.push("local".to_owned());
    }

    let snapshot_map: BTreeMap<&str, &Result<MachineSnapshot, String>> = o
        .snapshots
        .iter()
        .map(|(m, res)| (m.as_str(), res))
        .collect();

    let mut lines = Vec::new();

    for machine in &machines {
        let m_str = machine.as_str();

        let snap_res = snapshot_map.get(m_str);

        if let Some(Err(err_msg)) = snap_res {
            lines.push(format!(
                "⚠️ {}: {}",
                compact(m_str, 32),
                compact(err_msg, 70)
            ));
            continue;
        }

        let snap_sessions = snap_res.and_then(|r| r.as_ref().ok()).map(|s| &s.sessions);

        let listing_sessions: Vec<_> = o
            .list
            .sessions
            .iter()
            .filter(|s| s.machine == m_str)
            .collect();

        let stale_suffix = o
            .list
            .stale
            .iter()
            .find(|item| item.machine == m_str && item.stale)
            .map(|item| format!(" · stale {}초 전", item.age_seconds))
            .unwrap_or_default();

        let mut working_items = Vec::new();
        let mut question_items = Vec::new();
        let mut blocked_items = Vec::new();
        let mut idle_items = Vec::new();

        let mut matched_snapshot_indices = std::collections::BTreeSet::new();

        if let Some(snaps) = snap_sessions {
            for (snap_idx, live) in snaps.iter().enumerate() {
                matched_snapshot_indices.insert(snap_idx);
                let state = state_of(live);

                let matched_listing = listing_sessions.iter().find(|ls| {
                    ls.session_id == live.session_id
                        || ls.raw.get("durableSessionId").and_then(|v| v.as_str())
                            == Some(&live.session_id)
                });

                let activity_str = activity(
                    live.updated_at.as_deref().or_else(|| {
                        matched_listing
                            .and_then(|ls| ls.updated_at.as_deref().or(ls.last_activity.as_deref()))
                    }),
                    now,
                );

                let id8 = short_id(&live.session_id);

                let name = live
                    .name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
                    .or_else(|| matched_listing.and_then(|ls| ls.name.as_deref()))
                    .or_else(|| {
                        Some(live.last_user.as_str()).filter(|text| !text.trim().is_empty())
                    })
                    .unwrap_or(&live.session_id);

                let cwd = live
                    .cwd
                    .as_deref()
                    .or_else(|| matched_listing.and_then(|ls| ls.cwd.as_deref()))
                    .unwrap_or("?");

                let cwd_rel = home_relative(cwd);

                match state {
                    SessionState::Working => {
                        let header_line = format!(
                            "🟢 {} · {} · {} · `{}:{}`",
                            compact(name, 32),
                            compact(&cwd_rel, 36),
                            activity_str,
                            m_str,
                            id8
                        );
                        let sub_line = format!(
                            "  ↳ {}",
                            compact(&collapse_newlines(&live.last_assistant), 90)
                        );
                        working_items.push((header_line, sub_line));
                    }
                    SessionState::Question => {
                        let header_line = format!(
                            "❓ {} · {} · {} · `{}:{}`",
                            compact(name, 32),
                            compact(&cwd_rel, 36),
                            activity_str,
                            m_str,
                            id8
                        );
                        let q_summary = live
                            .questions
                            .first()
                            .map(format_question_summary)
                            .unwrap_or_else(|| "질문 대기 중".into());
                        let sub_line = format!("  ↳ {q_summary}");
                        question_items.push((header_line, sub_line));
                    }
                    SessionState::Blocked => {
                        let header_line = format!(
                            "⛔ {} · {} · {} · `{}:{}`",
                            compact(name, 32),
                            compact(&cwd_rel, 36),
                            activity_str,
                            m_str,
                            id8
                        );
                        let sub_line = "  ↳ 터미널에서 답변 필요 (차단형 질문)".to_string();
                        blocked_items.push((header_line, sub_line));
                    }
                    SessionState::Idle => {
                        let idle_str = format!("⚪ {} · `{}:{}`", compact(name, 28), m_str, id8);
                        idle_items.push(idle_str);
                    }
                }
            }
        }

        for ls in &listing_sessions {
            let already_included = snap_sessions.is_some_and(|snaps| {
                snaps.iter().any(|live| {
                    live.session_id == ls.session_id
                        || ls.raw.get("durableSessionId").and_then(|v| v.as_str())
                            == Some(&live.session_id)
                })
            });

            if !already_included {
                let id8 = short_id(&ls.session_id);
                let name = ls.name.as_deref().unwrap_or(&ls.session_id);
                idle_items.push(format!("⚪ {} · `{}:{}`", compact(name, 28), m_str, id8));
            }
        }

        let working_cnt = working_items.len();
        let question_cnt = question_items.len() + blocked_items.len();
        let idle_cnt = idle_items.len();

        let header = format!(
            "🖥️ {} — 작업 중 {} · 답변 대기 {} · 대기 {}{}",
            compact(m_str, 32),
            working_cnt,
            question_cnt,
            idle_cnt,
            stale_suffix
        );
        lines.push(header);

        for (hl, sl) in working_items {
            lines.push(hl);
            lines.push(sl);
        }
        for (hl, sl) in question_items {
            lines.push(hl);
            lines.push(sl);
        }
        for (hl, sl) in blocked_items {
            lines.push(hl);
            lines.push(sl);
        }

        if !idle_items.is_empty() {
            if idle_items.len() <= 8 {
                for item in idle_items {
                    lines.push(item);
                }
            } else {
                for item in idle_items.iter().take(8) {
                    lines.push(item.clone());
                }
                let overflow = idle_items.len() - 8;
                lines.push(format!("… 외 {overflow}개 대기"));
            }
        }
    }

    for err in &o.list.errors {
        if !snapshot_map.contains_key(err.machine.as_str()) {
            lines.push(format!(
                "⚠️ {}: {}",
                compact(&err.machine, 32),
                compact(&err.message, 70)
            ));
        }
    }

    if lines.is_empty() {
        return "실행 중인 세션 없음".into();
    }

    apply_max_chars(lines, max_chars)
}

fn apply_max_chars(lines: Vec<String>, max_chars: usize) -> String {
    let ellipsis = "… (일부 생략)";
    let full = lines.join("\n");
    if full.chars().count() <= max_chars {
        return full;
    }

    let mut out_lines = Vec::new();
    let mut current_chars = 0;
    let ellipsis_chars = ellipsis.chars().count() + 1;

    for line in lines {
        let line_chars = line.chars().count() + if out_lines.is_empty() { 0 } else { 1 };
        if current_chars + line_chars + ellipsis_chars > max_chars {
            break;
        }
        current_chars += line_chars;
        out_lines.push(line);
    }

    out_lines.push(ellipsis.to_string());
    out_lines.join("\n")
}

pub fn format_detail(
    target: &str,
    s: &LiveSession,
    transcript_tail: &[String],
    max_chars: usize,
) -> String {
    let state = state_of(s);
    let state_str = match state {
        SessionState::Working => "작업 중 (Streaming/Compacting)",
        SessionState::Question => "답변 대기 중",
        SessionState::Blocked => "터미널 답변 대기 중 (차단형)",
        SessionState::Idle => "대기 중 (Idle)",
    };

    let name = s.name.as_deref().unwrap_or(&s.session_id);
    let cwd_rel = home_relative(s.cwd.as_deref().unwrap_or("?"));

    let mut lines = Vec::new();
    lines.push(format!("📌 세션 상세: `{target}` ({})", compact(name, 40)));
    lines.push(format!("상태: {state_str}"));
    lines.push(format!("경로: {cwd_rel}"));
    lines.push(format!(
        "메시지 수: {} (대기 중: {})",
        s.message_count, s.pending_messages
    ));

    if let Some(err) = &s.messages_error {
        lines.push(format!("⚠️ 메시지 조회 경고: {}", compact(err, 80)));
    }

    if !s.questions.is_empty() {
        lines.push("❓ 대기 중인 질문:".into());
        for q in &s.questions {
            for item in &q.questions {
                let q_text = item
                    .question
                    .as_deref()
                    .or(item.header.as_deref())
                    .unwrap_or("?");
                lines.push(format!("  - [{}] {}", item.id, collapse_newlines(q_text)));
                if !item.options.is_empty() {
                    let opts = item
                        .options
                        .iter()
                        .map(|opt| format!("[{}]", collapse_newlines(opt)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    lines.push(format!("    선택지: {opts}"));
                }
            }
        }
    }

    if !transcript_tail.is_empty() {
        lines.push("📜 최근 대화 내용:".into());
        for entry in transcript_tail {
            for l in entry.lines() {
                lines.push(format!("  {l}"));
            }
        }
    } else {
        if !s.last_user.is_empty() {
            lines.push(format!(
                "👤 최근 사용자: {}",
                compact(&collapse_newlines(&s.last_user), 120)
            ));
        }
        if !s.last_assistant.is_empty() {
            lines.push(format!(
                "🤖 최근 어시스턴트: {}",
                compact(&collapse_newlines(&s.last_assistant), 120)
            ));
        }
    }

    apply_max_chars(lines, max_chars)
}

pub async fn collect<R: CommandRunner + 'static>(
    control: &SessionControl<R>,
    text_limit: usize,
) -> Overview {
    let list = control.registry().list_all().await;

    let mut machines = Vec::new();
    machines.push("local".to_string());
    for s in &list.sessions {
        if !machines.contains(&s.machine) {
            machines.push(s.machine.clone());
        }
    }
    for e in &list.errors {
        if !machines.contains(&e.machine) {
            machines.push(e.machine.clone());
        }
    }

    let probe = Probe::new(control.runner());
    let snap_results = probe.snapshot_all(&machines, text_limit).await;

    let snapshots = snap_results
        .into_iter()
        .map(|(m, res)| (m, res.map_err(|err| err.to_string())))
        .collect();

    Overview { list, snapshots }
}

#[cfg(test)]
mod tests {
    use super::super::probe::QuestionItem;
    use super::super::registry::Session;
    use super::*;

    #[test]
    fn state_of_detects_all_states() {
        let mut s = LiveSession::default();
        assert_eq!(state_of(&s), SessionState::Idle);

        s.streaming = true;
        assert_eq!(state_of(&s), SessionState::Working);
        s.streaming = false;

        s.compacting = true;
        assert_eq!(state_of(&s), SessionState::Working);
        s.compacting = false;

        s.blocking_question = true;
        assert_eq!(state_of(&s), SessionState::Blocked);

        s.questions.push(PendingQuestion {
            request_id: "req1".into(),
            questions: vec![QuestionItem {
                id: "q1".into(),
                header: None,
                question: Some("진행할까요?".into()),
                options: vec!["네".into(), "아니오".into()],
            }],
        });
        assert_eq!(state_of(&s), SessionState::Question);
    }

    #[test]
    fn format_overview_grouping_and_korean_counts() {
        let now = DateTime::parse_from_rfc3339("2026-10-16T12:00:00+09:00")
            .unwrap()
            .with_timezone(&Utc);

        let list = SessionList {
            sessions: vec![
                Session {
                    session_id: "sess-11112222".into(),
                    name: Some("worker-1".into()),
                    cwd: Some("/home/indo/code/proj".into()),
                    status: Some("working".into()),
                    alive: None,
                    surface: None,
                    updated_at: Some("2026-10-16T11:58:00+09:00".into()),
                    controls: vec![],
                    machine: "local".into(),
                    last_activity: None,
                    raw: serde_json::json!({}),
                },
                Session {
                    session_id: "sess-33334444".into(),
                    name: Some("idle-worker".into()),
                    cwd: Some("/home/indo/other".into()),
                    status: Some("idle".into()),
                    alive: None,
                    surface: None,
                    updated_at: None,
                    controls: vec![],
                    machine: "local".into(),
                    last_activity: None,
                    raw: serde_json::json!({}),
                },
            ],
            errors: vec![],
            stale: vec![],
            exit_code: 0,
        };

        let snapshots = vec![(
            "local".into(),
            Ok(MachineSnapshot {
                machine: "local".into(),
                sessions: vec![LiveSession {
                    machine: "local".into(),
                    session_id: "sess-11112222".into(),
                    routing_id: "sess-11112222".into(),
                    endpoint_kind: "tui".into(),
                    name: Some("worker-1".into()),
                    cwd: Some("/home/indo/code/proj".into()),
                    streaming: true,
                    compacting: false,
                    blocking_question: false,
                    pending_messages: 0,
                    message_count: 5,
                    last_user: "작업해줘".into(),
                    last_assistant: "작업을 진행하고\n있습니다.".into(),
                    questions: vec![],
                    messages_error: None,
                    updated_at: None,
                }],
                errors: vec![],
            }),
        )];

        let overview = Overview { list, snapshots };
        let formatted = format_overview(&overview, now, 1900);

        assert!(formatted.contains("🖥️ local — 작업 중 1 · 답변 대기 0 · 대기 1"));
        assert!(
            formatted.contains("🟢 worker-1 · ~/code/proj · 2분 전 (KST) · `local:sess-11112222`")
        );
        assert!(formatted.contains("  ↳ 작업을 진행하고 있습니다."));
        assert!(formatted.contains("⚪ idle-worker · `local:sess-33334444`"));
    }

    #[test]
    fn short_id_distinguishes_sessions_sharing_a_time_prefix() {
        let a = "01a1428d-5e53-7f52-974a-0f2bcb7e9722";
        let b = "01a1428d-6683-7c11-8f0e-1d2c3b4a5968";
        assert_eq!(&a[..8], &b[..8]);
        assert_ne!(short_id(a), short_id(b));
        assert!(a.starts_with(short_id(a)));
        assert_eq!(short_id("sess-1"), "sess-1");
    }

    #[test]
    fn format_overview_question_rendering_with_options() {
        let now = Utc::now();
        let list = SessionList {
            sessions: vec![],
            errors: vec![],
            stale: vec![],
            exit_code: 0,
        };
        let snapshots = vec![(
            "local".into(),
            Ok(MachineSnapshot {
                machine: "local".into(),
                sessions: vec![LiveSession {
                    machine: "local".into(),
                    session_id: "sess-question1".into(),
                    routing_id: "sess-question1".into(),
                    endpoint_kind: "tui".into(),
                    name: Some("q-bot".into()),
                    cwd: Some("/Users/indo/repo".into()),
                    streaming: false,
                    compacting: false,
                    blocking_question: false,
                    pending_messages: 0,
                    message_count: 2,
                    last_user: "".into(),
                    last_assistant: "".into(),
                    questions: vec![PendingQuestion {
                        request_id: "req-1".into(),
                        questions: vec![QuestionItem {
                            id: "q1".into(),
                            header: Some("배포 승인".into()),
                            question: Some("배포를 진행할까요?".into()),
                            options: vec!["승인".into(), "거절".into()],
                        }],
                    }],
                    messages_error: None,
                    updated_at: None,
                }],
                errors: vec![],
            }),
        )];

        let overview = Overview { list, snapshots };
        let formatted = format_overview(&overview, now, 1900);

        assert!(formatted.contains("❓ q-bot · ~/repo · 활동 미상 (KST) · `local:sess-question`"));
        assert!(formatted.contains("  ↳ 배포를 진행할까요? [승인] [거절]"));
    }

    #[test]
    fn format_overview_idle_overflow_and_unreachable_host() {
        let now = Utc::now();
        let mut sessions = Vec::new();
        for i in 1..=12 {
            sessions.push(LiveSession {
                machine: "local".into(),
                session_id: format!("sess-idle-{i:04}"),
                routing_id: format!("sess-idle-{i:04}"),
                endpoint_kind: "tui".into(),
                name: Some(format!("idle-{i}")),
                cwd: Some("/home/indo".into()),
                streaming: false,
                compacting: false,
                blocking_question: false,
                pending_messages: 0,
                message_count: 0,
                last_user: "".into(),
                last_assistant: "".into(),
                questions: vec![],
                messages_error: None,
                updated_at: None,
            });
        }

        let list = SessionList {
            sessions: vec![],
            errors: vec![],
            stale: vec![],
            exit_code: 0,
        };
        let snapshots = vec![
            (
                "local".into(),
                Ok(MachineSnapshot {
                    machine: "local".into(),
                    sessions,
                    errors: vec![],
                }),
            ),
            (
                "remote-down".into(),
                Err("ssh: connect to host remote-down port 22: Connection refused".into()),
            ),
        ];

        let overview = Overview { list, snapshots };
        let formatted = format_overview(&overview, now, 1900);

        assert!(formatted.contains("🖥️ local — 작업 중 0 · 답변 대기 0 · 대기 12"));
        assert!(formatted.contains("… 외 4개 대기"));
        assert!(formatted.contains(
            "⚠️ remote-down: ssh: connect to host remote-down port 22: Connection refused"
        ));
    }

    #[test]
    fn format_overview_max_chars_respected_for_200_sessions() {
        let now = Utc::now();
        let mut live_sessions = Vec::new();
        for i in 0..200 {
            live_sessions.push(LiveSession {
                machine: "local".into(),
                session_id: format!("session-uuid-number-{i:04}-abcd-efgh"),
                routing_id: format!("session-uuid-number-{i:04}-abcd-efgh"),
                endpoint_kind: "tui".into(),
                name: Some(format!("worker-session-{i}")),
                cwd: Some(format!("/home/indo/projects/repo-{i}")),
                streaming: i % 2 == 0,
                compacting: false,
                blocking_question: false,
                pending_messages: 0,
                message_count: 10,
                last_user: "명령".into(),
                last_assistant: format!("작업 결과 내용 긴 텍스트 {i} 반복합니다."),
                questions: vec![],
                messages_error: None,
                updated_at: None,
            });
        }

        let list = SessionList {
            sessions: vec![],
            errors: vec![],
            stale: vec![],
            exit_code: 0,
        };
        let snapshots = vec![(
            "local".into(),
            Ok(MachineSnapshot {
                machine: "local".into(),
                sessions: live_sessions,
                errors: vec![],
            }),
        )];

        let overview = Overview { list, snapshots };
        let max_chars = 1900;
        let formatted = format_overview(&overview, now, max_chars);

        assert!(formatted.chars().count() <= max_chars);
        assert!(formatted.contains("… (일부 생략)"));
    }

    #[test]
    fn format_detail_renders_state_and_tail() {
        let live = LiveSession {
            machine: "local".into(),
            session_id: "0123456789abcdef".into(),
            routing_id: "0123456789abcdef".into(),
            endpoint_kind: "tui".into(),
            name: Some("test-session".into()),
            cwd: Some("/home/indo/work".into()),
            streaming: true,
            compacting: false,
            blocking_question: false,
            pending_messages: 1,
            message_count: 15,
            last_user: "코드 작성해줘".into(),
            last_assistant: "완료했습니다.".into(),
            questions: vec![],
            messages_error: None,
            updated_at: None,
        };

        let tail = vec![
            "User: 안녕하세요".to_string(),
            "Assistant: 무엇을 도와드릴까요?".to_string(),
        ];
        let detail = format_detail("local:01234567", &live, &tail, 1900);
        assert!(detail.contains("📌 세션 상세: `local:01234567` (test-session)"));
        assert!(detail.contains("상태: 작업 중 (Streaming/Compacting)"));
        assert!(detail.contains("경로: ~/work"));
        assert!(detail.contains("User: 안녕하세요"));
        assert!(detail.contains("Assistant: 무엇을 도와드릴까요?"));
    }
}
