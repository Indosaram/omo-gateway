# Todo 14 - integration gate and deploy

## Gate (run once, HEAD aa49764 working tree)
- `cargo fmt --check`: exit 0
- `cargo clippy --all-targets -- -D warnings`: clean
- Full `cargo test` (CARGO_TARGET_DIR=WT/target): 1809 passed, 1 failed
  - failure: `r_dash_001::dash_001_chat_protocol_ingress_and_contract` (tests/test_review_dashboard.rs:807) - pre-existing, also failing before this branch (see task-14-final-test.txt at 4e9c65c)
  - log: task-14-final-test2.txt
- Focused suites inside that run: test_action_ledger 18/18, test_approval_wait 8/8, test_action_loop 7/7, test_context_pack 7/7, test_inbox_ledger 6/6, test_github_source 3/3, test_lane_control 6/6, test_lane_worktree_auto 4/4, test_report_claims (in run), test_omo_backend 54/54 (+1 ignored live)

## Commits on branch autonomy-two-axis (no push to main)
- 8f324b6 main feature, cb388f0, 58a5fa3, 9041b6e, 8e888ae, 22b5e24, 4e9c65c, aa49764 (review blockers)

## Deploy
- scripts/deploy.sh only; log task-14-deploy-final6.log

## Final gate re-run (--no-fail-fast, all 86 binaries)
passed=1888 failed=1 ignored=20 (task-14-final-test3.txt). Only failure: pre-existing r_dash_001 (tests/test_review_dashboard.rs:807). test_status_message 3/3 ok.

## Deploy receipt
rev=aa49764 dirty_paths=2 source=/Users/indo/code/project/omon-gateway-wt/autonomy deployed_at=20261019-005907
TeamIdentifier=5DUM8WPB4C
    "OMON_ACTION_LOOP_ENABLED" => "true"
    "OMON_APPROVAL_LEDGER_ENABLED" => "true"
    "OMON_GITHUB_SOURCE_ENABLED" => "true"
aa49764 fix(autonomy): close review blockers in approval ledger
4e9c65c fix(autonomy): scope compound eval command pieces to their cd directory
22b5e24 fix(autonomy): keep the cd target when classifying eval shell commands
8e888ae fix(autonomy): classify eval approval frames by their code instead of auto-allowing
9041b6e fix(autonomy): set the workspace permission preset before resuming a thread
58a5fa3 fix(autonomy): baseline the action loop backlog on first enable and cap listed items
cb388f0 fix(autonomy): route ledger events to a real session key and reload changed launchd env
8f324b6 feat(autonomy): learned-autonomy gateway on the two-axis plan
f7e6c41 chore: snapshot deployed slack-channel working tree as the autonomy base
?? .omo/evidence/autonomy-two-axis-code-review.md
?? .omo/evidence/autonomy-two-axis/
