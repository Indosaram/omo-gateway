# QA2 inbox source settlement

## Retained state and diagnosis

Read-only SQLite inspection of `/private/tmp/omon-task24-qa2-K6WyJH/qa.db` found 166 inbox items (all iMessage), all 166 without decisions. `judge_calls` contains two `inbox_class/failed/used_fallback=0` rows and two `inbox_class/capped:owner_must_act/used_fallback=1` rows. Primary attempts took 20 and 22 ms, ruling out the five-second timeout on these two attempts. No writer usage or lanes exists in the retained receipt.

The retained `task-24-qa2.log` records triage rejection at 00:19:07.881 after collection and at 00:21:07.872 after deadline. Thus Triage::serve DID run, collector progress DID reach it, the full batch DID attempt classification, and the two-minute deadline DID fire. Current headless main.rs starts the collector at line 699; spawn_enabled_inbox_collector wires Some(progress_tx) and triage.serve. No main.rs patch is required or created.

Zero fallback cap does not cap successful primary jev calls: judge/mod.rs first executes jev under its independent primary cap, and only takes the zero-capped fallback after primary failure. Rejecting that conservative choice in inbox triage is intentional; it cannot be treated as a real classification. The failure is primary judge readiness in this harness, not all primary classifications being rejected by a zero cap. Retained error details do not distinguish missing credentials, subprocess failure, or invalid judge output; no unsupported exact upstream cause is claimed.

QA2's environment has a private HOME and only copies TYPESAFE_API_KEY, TYPESAFE_URL and TYPESAFE_MODEL if already exported by its parent. Those three variables are absent in this diagnostic session. The executable exists at /Users/indo/.cargo/bin/typesafe-judge. judge/mod.rs loads missing credentials from `$HOME/.zshenv`, which the private HOME does not supply. No real model or judge was invoked in this investigation. A rerun must supply the owner's existing judge configuration read-only and preflight its availability without recording secrets.

Mail `No accounts registered` is a harness issue. qa2-common.mjs launches the gateway/watch helper with private HOME, hiding existing zele accounts. The mail helper must pass the owner's real HOME read-only for mail list/watch; do not copy accounts, log in, write owner configuration, or replace the entire gateway HOME. This source-only task records that required helper change without modifying helpers.

## Product changes

- src/inbox/mod.rs skips whitespace-empty references/snippets, counts drops under empty_source_ref_or_snippet, and commits valid rows plus the returned cursor. Wrong-source rows and actual SQL failures still fail and roll back. An empty Kakao attachment row no longer discards the whole source batch.
- src/inbox/set.rs routes explicit local approval of an existing owner proposal through proposal execution, instead of only changing decision metadata. Items without a native proposal retain the existing status-only behavior.
- src/inbox/proposals.rs shares the existing snapshot-validated, committed-before-effects CAS between Discord approval and local approve_item. Local evidence is `cli:inbox-proposal-N`, never a fabricated Discord message. GatewayActions::from_env supplies the existing lane/direct actions without requiring Discord ingress. A stored trusted action route is required; raw proposed_action is not parsed as a command. Duplicate/restarted/concurrent execution remains prevented by proposal state.

Todo 26 at main checkout plan lines 445-452 explicitly prescribes owner approval -> inbox set approved -> lead execution and records lane key/outcome. The old metadata-only set and Discord-only proposal CAS did not fulfill this. Retained `/private/tmp/omon-task26-qa2-7Glkdp/qa.db` confirms proposal 1 pending, approval_message_id NULL, decision approved, lane_key NULL, zero lanes. The fix connects the local path for fresh pending proposals; retained databases were not repaired or written. An existing manually status-approved snapshot is still not automatically replayed, preserving the prior stale-snapshot guard.

## Verification

Requested command (run from the worktree, no model invocation):

```sh
RUSTC_WRAPPER= CARGO_TARGET_DIR=$PWD/target-w4 cargo test -j4 --offline --test test_inbox_triage --test test_inbox_store --test test_inbox_cli_digest --test test_inbox_proposals
```

Added tests cover a mixed valid/empty Kakao batch with cursor advancement; successful real JevCli adapter calls to a local fake executable with fallback cap zero; and local set approval producing exactly one lane linked through inbox_decisions, with the item's source link asserted in the brief by the action fixture, followed by repeated approval with no second lane.

LSP diagnostics initially reported zero errors for src/inbox and the changed test files; a later refresh for test_inbox_store.rs timed out. Targeted rustfmt check passed before Cargo began. The first requested warm-target suite passed CLI/digest (9) and proposals (10), then the new store test failed because its supposedly valid Kakao fixture lacked DirectChat room metadata and was legitimately prefiltered. The fixture was corrected; product filters were not weakened. Cargo stopped before triage in that run. The full requested command then passed with exit 0: CLI/digest 9, proposals 10, store 11, triage 9; total 39 passed, zero failed. The final targeted rustfmt check also passed. Both executions used target-w4 and waited for its shared build-directory lock; no competing build or other process was changed. No live DB writes, services, real models, env edits, git operations, or forbidden source edits were performed. Strict clippy and live QA were not run; this task authorized the four focused test targets and expressly forbade services/models.
