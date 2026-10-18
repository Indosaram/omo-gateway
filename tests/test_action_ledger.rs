use std::path::PathBuf;

use omon_gateway::autonomy::{
    classify_command, classify_file_change, classify_json, decide_proposal, evaluate_pre_ask,
    finalize_ask_result, find_matching_precedent, get_proposal, requires_always_ask,
    revoke_precedent, upsert_precedent, AskResolution, Effect, PreAskInput, PreAskOutcome,
    PrecedentDecision, PrecedentScope, ProposalOrigin,
};
use omon_gateway::storage::init_pool;
use omon_gateway::ApprovalDecision;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/approval_frames")
}

#[test]
fn a_every_todo_0_fixture_maps_to_expected_effect() {
    let dir = fixtures_dir();
    let entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("fixtures dir must exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    assert!(
        entries.len() >= 9,
        "expected at least 9 fixture files, found {}",
        entries.len()
    );

    for entry in entries {
        let path = entry.path();
        let raw = std::fs::read_to_string(&path).expect("read fixture");
        let val: serde_json::Value = serde_json::from_str(&raw).expect("parse fixture json");
        let probe = val
            .get("probe")
            .and_then(|v| v.as_str())
            .expect("fixture must have probe field");
        let classified = classify_json(&val);
        let expected = match probe {
            "ls" | "rg" | "git_status" => Effect::Read,
            "git_commit" => Effect::VcsLocal,
            "git_push" => Effect::VcsRemote,
            "git_push_main" => Effect::VcsProtected,
            "gh_pr" => Effect::VcsRemote,
            "rm" => Effect::Delete,
            "edit" => {
                let loc = val.get("location").and_then(|v| v.as_str()).unwrap_or("");
                if loc.starts_with("/tmp") || loc.contains(".omon/workspace") {
                    Effect::Draft
                } else {
                    Effect::WriteLocal
                }
            }
            "eval_write" => Effect::Unknown,
            other => panic!("unexpected probe {other} in {}", path.display()),
        };
        assert_eq!(
            classified.effect,
            expected,
            "fixture {} (probe={probe}) classified as {:?}, expected {:?}",
            path.display(),
            classified.effect,
            expected
        );
    }
}

#[test]
fn b_at_least_twenty_native_command_samples_map_to_expected_effects() {
    let repo_cwd = Some("/Users/indo/code/project/omon-gateway");
    let ws_cwd = Some("~/.omon/workspace/agents/bot-1465631383862120451");

    let samples: Vec<(&str, Option<&str>, Effect)> = vec![
        ("ls -la src", repo_cwd, Effect::Read),
        ("rg -n 'ApprovalDecision' src", repo_cwd, Effect::Read),
        ("git status --short", repo_cwd, Effect::Read),
        ("git log --oneline -20", repo_cwd, Effect::Read),
        ("git diff HEAD~1", repo_cwd, Effect::Read),
        ("gh pr view 42 --json state", repo_cwd, Effect::Read),
        ("gh api notifications", repo_cwd, Effect::Read),
        (
            "sqlite3 -readonly omon_gateway.db 'SELECT count(*) FROM lanes'",
            repo_cwd,
            Effect::Read,
        ),
        ("touch /tmp/draft_note.md", repo_cwd, Effect::Draft),
        ("echo hello > /tmp/out.txt", repo_cwd, Effect::Draft),
        (
            "mkdir -p ~/.omon/workspace/agents/bot-1/scratch",
            ws_cwd,
            Effect::Draft,
        ),
        ("touch src/new_module.rs", repo_cwd, Effect::WriteLocal),
        (
            "sed -i '' 's/foo/bar/g' src/lib.rs",
            repo_cwd,
            Effect::WriteLocal,
        ),
        ("cargo fmt", repo_cwd, Effect::WriteLocal),
        ("git add src/lib.rs", repo_cwd, Effect::VcsLocal),
        (
            "git commit -m 'feat: add module'",
            repo_cwd,
            Effect::VcsLocal,
        ),
        ("git checkout -b lane/my-task", repo_cwd, Effect::VcsLocal),
        ("git push origin lane/my-task", repo_cwd, Effect::VcsRemote),
        (
            "gh pr create --title 'test' --body 'body'",
            repo_cwd,
            Effect::VcsRemote,
        ),
        (
            "gh pr comment 12 --body 'looks good'",
            repo_cwd,
            Effect::VcsRemote,
        ),
        ("git push origin main", repo_cwd, Effect::VcsProtected),
        (
            "git push --force origin lane/my-task",
            repo_cwd,
            Effect::VcsProtected,
        ),
        ("git merge lane/my-task", repo_cwd, Effect::VcsProtected),
        ("gh pr merge 12 --squash", repo_cwd, Effect::VcsProtected),
        (
            "imsg send --to +821012345678 --message 'hello'",
            repo_cwd,
            Effect::ExternalSend,
        ),
        (
            "zele mail send --to bob@example.com --subject hi",
            repo_cwd,
            Effect::ExternalSend,
        ),
        (
            "rm -rf /tmp/omon-approval-spike/f",
            repo_cwd,
            Effect::Delete,
        ),
        ("git worktree remove /tmp/wt", repo_cwd, Effect::Delete),
        (
            "stripe charges create --amount 1000",
            repo_cwd,
            Effect::Spend,
        ),
        (
            "xenoci build --script 'cargo test'",
            repo_cwd,
            Effect::Spend,
        ),
        (
            "custom-unknown-binary --do-something",
            repo_cwd,
            Effect::Unknown,
        ),
    ];

    assert!(samples.len() >= 20);
    for (cmd, cwd, expected) in samples {
        let got = classify_command(cmd, cwd);
        assert_eq!(
            got.effect, expected,
            "command `{cmd}` classified as {:?}, expected {:?}",
            got.effect, expected
        );
    }

    let edit_repo = classify_file_change("src/lib.rs", repo_cwd);
    assert_eq!(edit_repo.effect, Effect::WriteLocal);
    let edit_tmp = classify_file_change("/tmp/scratch.txt", repo_cwd);
    assert_eq!(edit_tmp.effect, Effect::Draft);
}

#[test]
fn c_signatures_are_stable_across_flag_order() {
    let cwd = Some("/Users/indo/code/project/omon-gateway");

    let push_a = classify_command("git push -u -v origin lane/feature", cwd);
    let push_b = classify_command("git push origin lane/feature -v -u", cwd);
    assert_eq!(push_a.signature, push_b.signature);
    assert_eq!(push_a.effect, Effect::VcsRemote);

    let rg_a = classify_command("rg -n -i --hidden pattern src", cwd);
    let rg_b = classify_command("rg --hidden -i -n pattern src", cwd);
    assert_eq!(rg_a.signature, rg_b.signature);
    assert_eq!(rg_a.effect, Effect::Read);

    let ls_a = classify_command("ls -l -a /tmp", cwd);
    let ls_b = classify_command("ls -a -l /tmp", cwd);
    assert_eq!(ls_a.signature, ls_b.signature);
    assert_eq!(ls_a.effect, Effect::Read);
}

#[tokio::test]
async fn d_migration_copies_legacy_rows_and_release_at_exists() {
    let pool = init_pool("sqlite::memory:").await.unwrap();

    sqlx::query(
        "INSERT INTO approval_allowlist (pattern_key, created_at)
         VALUES ('terminal:git status:*', '2026-10-01 00:00:00')",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT OR IGNORE INTO action_precedents
            (signature, effect, target_scope, decision, scope, approvals, declines, origin, created_at, updated_at)
         SELECT pattern_key, 'unknown', '', 'allow', 'exact', 1, 0, 'legacy_allowlist', created_at, created_at
         FROM approval_allowlist",
    )
    .execute(&pool)
    .await
    .unwrap();

    let row: (String, String, String, String) = sqlx::query_as(
        "SELECT signature, decision, scope, origin FROM action_precedents WHERE signature = 'terminal:git status:*'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.0, "terminal:git status:*");
    assert_eq!(row.1, "allow");
    assert_eq!(row.2, "exact");
    assert_eq!(row.3, "legacy_allowlist");

    sqlx::query(
        "INSERT INTO lane_events (lane_key, lead_session_key, kind, body, created_at, release_at)
         VALUES (NULL, 'slack:dm:U1', 'action_loop', 'test hold', CURRENT_TIMESTAMP, '2026-10-18 23:00:00')",
    )
    .execute(&pool)
    .await
    .unwrap();

    let release_at: Option<String> =
        sqlx::query_scalar("SELECT release_at FROM lane_events WHERE kind = 'action_loop' LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(release_at.as_deref(), Some("2026-10-18 23:00:00"));
}

#[tokio::test]
async fn e_fixed_owner_rules_hold() {
    let pool = init_pool("sqlite::memory:").await.unwrap();

    assert!(requires_always_ask(Effect::Spend, false));
    assert!(requires_always_ask(Effect::Spend, true));
    assert!(!Effect::Spend.allows_any_precedent());
    assert!(!Effect::Spend.allows_class_precedent());
    assert!(!Effect::Delete.allows_class_precedent());
    assert!(!Effect::VcsProtected.allows_class_precedent());

    assert!(requires_always_ask(Effect::ExternalSend, true));
    assert!(requires_always_ask(Effect::Delete, true));
    assert!(requires_always_ask(Effect::VcsProtected, true));
    assert!(!requires_always_ask(Effect::Read, true));
    assert!(!requires_always_ask(Effect::VcsRemote, false));

    let spend_id = upsert_precedent(
        &pool,
        "spend:stripe spend:/repo",
        Effect::Spend,
        "/repo",
        PrecedentDecision::Allow,
        PrecedentScope::Exact,
        "owner",
        None,
    )
    .await
    .unwrap();
    assert_eq!(spend_id, None, "spend must never create a precedent");

    let delete_class_id = upsert_precedent(
        &pool,
        "delete:rm:/tmp",
        Effect::Delete,
        "/tmp",
        PrecedentDecision::Allow,
        PrecedentScope::Class,
        "owner",
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        delete_class_id, None,
        "delete must never create a class precedent"
    );

    let _ext_class = sqlx::query(
        "INSERT INTO action_precedents (signature, effect, target_scope, decision, scope, approvals, declines, origin)
         VALUES ('external_send:class:bob@example.com', 'external_send', 'bob@example.com', 'allow', 'class', 1, 0, 'test')",
    )
    .execute(&pool)
    .await
    .unwrap();
    let tainted_class_match = find_matching_precedent(
        &pool,
        "external_send:zele mail send:bob@example.com",
        Effect::ExternalSend,
        "bob@example.com",
        true,
    )
    .await
    .unwrap();
    assert!(
        tainted_class_match.is_none(),
        "tainted external_send must not match a class precedent"
    );

    let exact_id = upsert_precedent(
        &pool,
        "external_send:zele mail send:bob@example.com",
        Effect::ExternalSend,
        "bob@example.com",
        PrecedentDecision::Allow,
        PrecedentScope::Exact,
        "owner",
        Some("밥에게는 답장 보내도 돼"),
    )
    .await
    .unwrap()
    .expect("exact external_send precedent is allowed");

    let exact_match = find_matching_precedent(
        &pool,
        "external_send:zele mail send:bob@example.com",
        Effect::ExternalSend,
        "bob@example.com",
        true,
    )
    .await
    .unwrap();
    assert_eq!(exact_match.map(|p| p.id), Some(exact_id));
}

fn daytime_utc() -> chrono::DateTime<chrono::Utc> {
    use chrono::TimeZone;
    chrono_tz::Asia::Seoul
        .with_ymd_and_hms(2026, 10, 18, 14, 0, 0)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

#[tokio::test]
async fn todo8_a_and_b_first_request_asks_and_after_always_same_signature_is_auto_allowed() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let cls = classify_command(
        "git push origin lane/feat-1",
        Some("/Users/indo/code/project/mhc"),
    );

    let first = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: "slack:T1:C1::U1",
            lead_session_key: "slack:T1:C1::U1",
            classification: &cls,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap();
    let PreAskOutcome::Ask { proposal_id } = first else {
        panic!("expected Ask on first occurrence, got {first:?}");
    };

    let res = finalize_ask_result(
        &pool,
        proposal_id,
        &cls,
        "engine_approval",
        "slack:T1:C1::U1",
        Ok(ApprovalDecision::Always),
    )
    .await
    .unwrap();
    assert_eq!(res, AskResolution::Allow);

    let second = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: "slack:T1:C1::U1",
            lead_session_key: "slack:T1:C1::U1",
            classification: &cls,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap();
    let PreAskOutcome::Allow {
        proposal_id: Some(p2_id),
        precedent_id: Some(prec_id),
        notice: Some(notice),
    } = second
    else {
        panic!("expected AutoAllowed on second occurrence, got {second:?}");
    };
    assert!(notice.contains(&format!("선례 #{prec_id}로 자동 진행:")));
    let p2 = get_proposal(&pool, p2_id).await.unwrap().unwrap();
    assert_eq!(p2.state, "auto_allowed");
    assert_eq!(p2.precedent_id, Some(prec_id));
}

#[tokio::test]
async fn todo8_c_and_d_third_once_emits_one_candidate_and_class_decide_auto_allows_same_effect_and_repo(
) {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let repo = "/Users/indo/code/project/mhc";
    let lead = "slack:T1:C1::U1";

    let mut last_proposal_id = 0;
    for i in 1..=4 {
        let cls = classify_command(&format!("git commit -m 'step {i}'"), Some(repo));
        let PreAskOutcome::Ask { proposal_id } = evaluate_pre_ask(
            &pool,
            PreAskInput {
                origin: ProposalOrigin::EngineApproval,
                session_key: lead,
                lead_session_key: lead,
                classification: &cls,
                payload_json: "{}",
                tainted: false,
                yolo: false,
                always_mode: false,
                now: daytime_utc(),
            },
        )
        .await
        .unwrap() else {
            panic!("expected Ask for Once #{i}");
        };
        last_proposal_id = proposal_id;
        finalize_ask_result(
            &pool,
            proposal_id,
            &cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Once),
        )
        .await
        .unwrap();
    }

    let candidates: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'precedent_candidate'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        candidates, 1,
        "3rd Once must emit exactly 1 precedent_candidate"
    );

    decide_proposal(
        &pool,
        None,
        last_proposal_id,
        "allow-always",
        PrecedentScope::Class,
        Some("로컬 커밋/브랜치는 물어보지 마"),
        Some(lead),
    )
    .await
    .unwrap();

    let diff_cmd = classify_command("git checkout -b lane/new-branch", Some(repo));
    assert_eq!(diff_cmd.effect, Effect::VcsLocal);
    let outcome = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: lead,
            lead_session_key: lead,
            classification: &diff_cmd,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(outcome, PreAskOutcome::Allow { precedent_id: Some(_), .. }),
        "different command with same effect and repo must auto-allow via class precedent: {outcome:?}"
    );
}

#[tokio::test]
async fn todo8_e_and_f_deny_auto_declines_with_owner_words_and_revoke_returns_to_asking() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let repo = "/Users/indo/code/project/mhc";
    let lead = "slack:T1:C1::U1";
    let cls = classify_command("gh pr create --title t --body b", Some(repo));

    let PreAskOutcome::Ask { proposal_id } = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: lead,
            lead_session_key: lead,
            classification: &cls,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap() else {
        panic!("expected Ask");
    };

    decide_proposal(
        &pool,
        None,
        proposal_id,
        "deny",
        PrecedentScope::Exact,
        Some("PR은 내가 직접 올릴게"),
        Some(lead),
    )
    .await
    .unwrap();

    let denied = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: lead,
            lead_session_key: lead,
            classification: &cls,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap();
    let PreAskOutcome::Deny {
        proposal_id: d_id,
        precedent_id: Some(prec_id),
        reason,
        ..
    } = denied
    else {
        panic!("expected AutoDenied, got {denied:?}");
    };
    assert_eq!(
        reason,
        format!("owner declined this before (#{prec_id}): PR은 내가 직접 올릴게")
    );
    let d_row = get_proposal(&pool, d_id).await.unwrap().unwrap();
    assert_eq!(d_row.state, "auto_denied");

    assert!(revoke_precedent(&pool, prec_id).await.unwrap());

    let after_revoke = evaluate_pre_ask(
        &pool,
        PreAskInput {
            origin: ProposalOrigin::EngineApproval,
            session_key: lead,
            lead_session_key: lead,
            classification: &cls,
            payload_json: "{}",
            tainted: false,
            yolo: false,
            always_mode: false,
            now: daytime_utc(),
        },
    )
    .await
    .unwrap();
    assert!(
        matches!(after_revoke, PreAskOutcome::Ask { .. }),
        "after revoke, request must return to asking"
    );
}

#[tokio::test]
async fn todo8_g_spend_never_auto_allows_and_emits_no_candidate() {
    let pool = init_pool("sqlite::memory:").await.unwrap();
    let lead = "slack:T1:C1::U1";
    let cls = classify_command("stripe charges create --amount 5000", Some("/repo"));
    assert_eq!(cls.effect, Effect::Spend);

    for _ in 0..3 {
        let PreAskOutcome::Ask { proposal_id } = evaluate_pre_ask(
            &pool,
            PreAskInput {
                origin: ProposalOrigin::EngineApproval,
                session_key: lead,
                lead_session_key: lead,
                classification: &cls,
                payload_json: "{}",
                tainted: false,
                yolo: false,
                always_mode: false,
                now: daytime_utc(),
            },
        )
        .await
        .unwrap() else {
            panic!("spend must always ask");
        };
        finalize_ask_result(
            &pool,
            proposal_id,
            &cls,
            "engine_approval",
            lead,
            Ok(ApprovalDecision::Always),
        )
        .await
        .unwrap();
    }

    let candidates: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM lane_events WHERE kind = 'precedent_candidate'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(candidates, 0, "spend must never emit precedent_candidate");
}

#[test]
fn f_eval_cell_shell_commands_are_classified_from_the_code() {
    let push = "const r=await Bun.$`cd /tmp/repo && git push origin lane/x 2>&1`.nothrow();print(r.exitCode)";
    let c = omon_gateway::autonomy::classify_eval_code(push, None).expect("push is a side effect");
    assert_eq!(c.effect, Effect::VcsRemote);

    let main = "await $`git push origin main`";
    assert_eq!(
        omon_gateway::autonomy::classify_eval_code(main, None)
            .unwrap()
            .effect,
        Effect::VcsProtected
    );

    let bash_tool = r#"await tool.bash({ command: "rm -rf /tmp/x" })"#;
    assert_eq!(
        omon_gateway::autonomy::classify_eval_code(bash_tool, None)
            .unwrap()
            .effect,
        Effect::Delete
    );

    let reads = "const s=await Bun.$`git status && ls -la`.text(); print(s)";
    assert!(omon_gateway::autonomy::classify_eval_code(reads, None).is_none());
    assert!(omon_gateway::autonomy::classify_eval_code("print(1+1)", None).is_none());
}
