---
slug: dori-parity-fixes
status: approved
intent: clear
review_required: true
plan_path: .omo/plans/dori-parity-fixes.md
plan_sha256: 2a3615d01b7be4f55150d364aec941e7acdeb55160894e0fa5633c80e7cc4f3b
review_round_id: round-3
review_round_limit: 5
pending-action: none (plan approved; owner starts /ulw-execute)
review:
  plan_reviewer:
    status: approved
    workspace_root: null
    runtime_home: null
    target: .omo/plans/dori-parity-fixes.md
    round_id: null
    plan_sha256: null
    launch_id: null
    session: null
    result: null
approach: fix the re-verified dori-mode contract gaps inside existing lanes modules (settle, storage/lanes pause, cli, runner, guard, signals, posts, tracker mirror) with focused tests in the existing test files; no new module, one additive migration at most
---

# Draft: dori-parity-fixes

## Affected user and ideal state
Users: (1) owner (Slack DM) who starts lanes through the lead bot and says wait/go; (2) the lead agent (bot persona) that reads lane events and CLI JSON; (3) lane agents that claim done; (4) tracker dashboard readers.
IS-1 | a lane is verified_done only when no commit made in it exists solely on this machine-local HEAD with nowhere to recover it, matching the footer promise "refuses commits on no remote" | footer runner.rs:2126 promises it
IS-2 | a done claim made while paused is settled (5-min window, signals, worktree checks) without needing a resume | reference scripts.md:82
IS-3 | `lane pause` exits 0 only after the runner confirmed the lane is stopped; otherwise exit 3 PAUSE_UNVERIFIED | reference pause.ts:36-46
IS-4 | a paused lane that still produces activity is reported to the lead once per stretch (working_while_paused) | reference pause.ts:70-85
IS-5 | tracker shows a paused lane as Paused, not Working | reference registry.ts:6 (paused is a status)
IS-6 | host guard alerts when active lanes exceed lanes_max | reference host-guard.ts:34
IS-7 | Done signal grammar accepts `Done =` prefix and `owner/repo/pull/N` / `issues/N`; rejects sha256 not 64 hex and empty json path | reference signals.ts:61,114,140
IS-8 | approval/lead fallback for a lane with no row goes to the runtime owner home (Slack DM), never the retired Discord DM | memory: Slack-only since 2026-10-18
IS-9 | a first-turn failure with a known startup cause (no API key, quota, rate limit, unknown provider, model not available) is reported at once as startup_error instead of after 3 retries | reference launch.ts:20,64-71
IS-10 | status text written to Slack has no emoji (owner decision pending) | reference writing.md:9
GAP-n == IS-n today unmet (evidence in Findings).

## Components (topology ledger)
C1 settle/worktree done check (IS-1) | C2 pause contract (IS-2..IS-5) | C3 guard lane-count alert (IS-6) | C4 signal grammar (IS-7) | C5 lead fallback (IS-8) | C6 startup error fast-fail (IS-9) | C7 status wording (IS-10)

## Open assumptions (announced defaults)
- object-done on a paused lane keeps the pause (reference: goal stays paused, so lane does no work) | reversible
- pause ack = runner inserts lane_history reason 'pause_acked' when it observes the pause (turn cancelled or idle); CLI waits up to 30s | reversible
- tracker Paused = new agent state row via additive migration 0055, mirror prefers paused flag over status | reversible
- COMPUTE_READY/BUSY not added: no consumer (heavy waits via wait_for_load_ok guard.rs:1070), would be noise against owner's event-only rule
- idle-lane reuse not added: reference CLI never calls idleLaneFor (only defined routing.ts:15); our persona block already instructs reuse (bot_profiles/blocks/lanes.md:4) = parity
- footer/scrub/dead-pane/sync-drift/2-tick report/`$(` refusal dropped: herdr screen-scrape or shell-transport artifacts
- test strategy: tests-after in existing test files; per owner memory, workers write only, one final build/test/clippy run

## Findings (cited - path:lines)
- settle.rs:618-624 skips unpushed when `git remote` empty; same rule runner.rs:584-595 keep_reason; test fixture tests/test_lane_done.rs:88-140 uses a no-remote repo (passing tests rely on the skip)
- settle.rs:52-56 skips paused done_claimed lanes (added 8f324b63); change_claim_at cli.rs:676-760 never touches lane_pauses
- pause_lane storage/lanes.rs:234-260 returns true after inserting row; CLI cli.rs:186-195 exits 0; runner cancels at notify runner.rs:1840 and at boundary runner.rs:1667-1673; outcome 'paused' runner.rs:1924
- no detection of activity after pause: activity.rs:118-160 capture ignores pauses
- tracker map tracker/lanes.rs:72-84 has no paused; seed tracker/seed.rs:44-75
- guard check_at guard.rs:610-676 load/mem/disk only; lanes_max guard.rs:40,68 used only in can_launch
- signals.rs:179 no prefix strip; :304,:343 '#' only; :675 no length check; :684-688 accepts json path '.'
- runner.rs:933 and agent/omo_backend.rs:1915 use kibitzer::DEFAULT_LEAD (Discord DM) instead of runtime_config::default_lead() (runtime_config.rs:31, set to Slack owner DM main.rs:875)
- runner.rs:1689-1693 fail("dead") only after 3 errors; fail() runner.rs:2104-2120 emits lane_events kind
- emoji: posts.rs:390 "⏳ ...", slack_threads.rs:20 "✅ lane closed", starter "🔄 lane" slack_threads.rs:514 test
- Rejected claims: resume quiet clock (runner.rs:1784 resets per turn), eyes removal (slack/egress.rs:32,600), unmentioned thread replies (slack/ingress.rs:142), heavy slot flock

## Decisions (with rationale)
(see assumptions; owner decisions pending below)

## Scope IN
C1-C7 as above.

## Scope OUT (Must NOT have)
- no LaneStatus::Paused enum / lanes table rebuild (autonomy-two-axis.md:302 decision kept)
- no herdr, no Discord work, no senpi engine patch, no .env edits, no COMPUTE_* events, no idle reuse code

## Open questions
Q1 RESOLVED (owner): no-remote repo with commits -> refuse as 'unpushed <path> (no remote)'; applies to settle worktree_reasons and runner keep_reason; fixtures get a bare remote
Q2 RESOLVED (owner): keep emoji; C7/IS-10 dropped from scope

## Approval gate
status: approved (owner chose Approve 2026-10-20); plan written .omo/plans/dori-parity-fixes.md; next: plan-consultant then plan-reviewer rounds
<!-- When exploration is exhausted and unknowns are answered, set status: awaiting-approval. -->
<!-- That durable record is the loop guard: on a later turn read it and resume at the gate instead of re-running exploration. -->


## Consultant pass (st_01a15bb3, 2026-10-20)
- Applied: A1-A6 refs, B1-B5 contradictions, C1 (service-pass ack), C2 (ack on paused_now), C3 (external NOT_PAUSABLE, default), C4 (rev-list --not --remotes, unborn HEAD), C5 (test g -> test_lane_done), C6 (new-issue path, closed-lane 409), C7 (prod no-remote cwd audit), C8 (lead guidance), C9 (permanent fail / transient report, default), C10, C11 (strip prefix on store), D1-D8.
- Open owner questions asked (non-blocking, recommended defaults written into plan): Q3 startup transient handling, Q4 reach vs exceed, Q5 external pause.

## Review rounds
- round 1: plan-reviewer st_01a15bc4, plan_sha256 1b0f4e85f0a83e7aab905bc328475b172231309ef8254c9497039447075779db, verdict OKAY with 2 non-blocking notes (todo 3/5 cli.rs ordering; Paused refusal location) -> both applied, plan changed
- round 2: plan-reviewer st_01a15bc6, plan_sha256 6f630e58c711a6b948bc0a249ebe8276123170df66d9f5b9a59e0a78d8f489c8, verdict OKAY with 2 non-blocking notes (todo 3 also after todo 1; test j SQL pause row; sync_inner Paused lookup) -> applied, plan changed
- round 3: plan-reviewer st_01a15bc8, plan_sha256 2a3615d01b7be4f55150d364aec941e7acdeb55160894e0fa5633c80e7cc4f3b, verdict OKAY (unconditional; 2 informational notes, no plan change required) - FINAL APPROVAL RECEIPT
