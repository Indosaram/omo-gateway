# Task 15 integrated ready contract

2026-10-11, evidence only. This artifact supersedes the selected-contract portions of `task15-integration-ready.md` and `task15-nonconflicting-patch-units.md`; those files are explicitly historical. No product/test/Cargo edit or Git/DB/network/model/service/test action occurred. Worker c05 owns Task 13 runner/CLI/mod/Cargo/tests during the current compile freeze. No implementation or test pass is claimed. Task 14 is final-confirmed per the lead's ledger 19/41 update; actor/backend durability remains untouched.

W = `/Users/indo/code/project/omon-gateway-wt/dori-work`. Main authority is task 15 in `/Users/indo/code/project/omon-gateway/.omo/plans/dori-grade-agent-env.md`, with the lead's latest explicit decisions. File references reflect inspected contexts; fresh runner/CLI contexts must be read after c05 settles.

## Selected ownership and literal acceptance

Explicit authorized registration may include the lane's own **linked-worktree cwd**. The linked cwd is not the repository's main worktree. Require a surviving control repository outside every removal target, actual linked Git registration/common-repository identity, and a recorded existing local branch. Registration delegates removal and later same-branch recreation authority; no cwd containment, filesystem creation time, transcript or settlement selection implies ownership. Current start creates no worktree, so an ordinary external unowned cwd has zero capability and closes with a truthful skipped-removal receipt. That is valid behavior, but **does not satisfy the primary owned lane cwd removal QA**. Auxiliary-only removal cannot substitute for that QA.

Minimal command proposal after release: `lane register-worktree <key> --repo <surviving-control-repo> --path <existing-linked-worktree>`. Path equal to the lane's canonical cwd is allowed and recorded as cwd role; auxiliary linked paths may also be registered. Reject main worktree/control repo, ambiguous linkage, already delegated live path, detached HEAD and missing branch. Record existing branch, never create one. Local authorized CLI registration is a capability grant, not a general worktree manager. No automatic owner-path adoption or broad create/delete/list subsystem.

Close must remove the explicitly registered cwd using `git -C <surviving-repo> worktree remove <registered-path>` without --force and without branch deletion. Its preserved branch must remain present. Before removal persist previous cwd plus the exact ownership/branch identity in the close attempt. After observed success persist the removal receipt and closed lane's previous-cwd reference. The lane cannot start again until any required cwd restoration succeeds. Missing cwd alone is never permission to recreate an unowned path.

## Exact existing owner resolution, admission unchanged

Current authoritative source: `W/src/multiplexer/actor.rs:968-978` reads DISCORD_ALLOWED_USERS, takes the first nonempty trimmed **string**, defaults to `414011632306618368`, then compares key.user_id to that string. `src/main.rs:105-113` parses valid numeric entries separately for admission. First raw string selection must not be replaced with parsed allowed_users.first(), singleton enforcement, a new optional setting, numeric normalization, or silently skipping an invalid first entry. An explicit noncanonical/invalid value simply cannot equal the canonical Discord author string; the actor pattern does not select another user.

Reuse exactly this resolver in the Task 15 adapter. Minimal implementation can use the same small expression locally; if sharing is necessary, expose a helper in the existing actor module and replace only the expression at that site, preserving returned String and every actor durability/control branch. No dedicated DISCORD_LANE_OWNER_ID. No env-file modification. General admission still applies: channels/users/roles/pairing are checked as before, but none independently makes an admitted human the owner. Compare author.id.to_string() to the resolved owner string; unique lane thread and persisted bot binding must also match. Old unbound lane rows require evidenced binding, never a primary-bot guess.

Exact reusable expression (proposal, not applied):

```rust
std::env::var("DISCORD_ALLOWED_USERS")
    .ok()
    .and_then(|value| {
        value.split(',').map(str::trim).find(|id| !id.is_empty())
            .map(ToString::to_string)
    })
    .unwrap_or_else(|| "414011632306618368".to_string())
```

## Minimal durable records and interfaces

The schema is still a proposal requiring lead migration approval; no document grants DB authority. Recheck migration number (previous inventory ends at 0037) and duplicates before selecting 0038. No probe was executed.

- `lane_close_intents`: unique lane/reopen-generation, captured original status, force/note, pending/running/failed/completed, next unfinished phase, attempt, typed error detail, previous_cwd, optional cwd_capability_id, timestamps. Normal verified_done eligibility and explicit forced: boundary remain; accepted intent is not closed success.
- `lane_owner_replies`: unique Discord message ID across bot clients, lane/bot/author, exact ordinary text, captured generation, pending/applied/failed, reopen-history ID and error/timestamps. Applied receipt, reopened history and first-prompt inbox entry commit together.
- `lane_thread_bindings`: lane FK, explicit bot ID, bound timestamp. Partial unique index for non-null lanes.thread_channel_id. Existing duplicates block migration; report identities and request lead remediation, never pick/delete one.
- `lane_owned_worktrees`: lane FK, canonical registered path and role cwd/auxiliary, surviving control repo, common Git dir, linked registration path and identity proof, **existing branch ref**, registrar, registered time, registered/removing/removed/restoring states, close intent/removal receipt reference. Path ownership includes explicit delegation; it is not proved by writing a row with an arbitrary identity string.
- `lane_cwd_restorations`: only if a separate record is needed for atomic restoration fencing; prefer recording restoration attempt/receipt on the existing cwd capability instead. Retain prior verified removal receipt and old registration identity when replacing the current registration identity after recreation. Do not overwrite proof of removal with new linkage.

Minimal deferred APIs: request_close(...)->intent; record_owner_reply(...)->new/duplicate consumed receipt; register_owned_worktree(...validated linkage/existing branch/caller authority...)->capability; restore_registered_cwd(...completed removal receipt...)->observed restored cwd; apply_reopen_receipt(...)->history/inbox/applied atomically. Exact storage representation remains lead-approved; no dummy implementations are supplied.

## Close, failure and restart order

Service claims intent by CAS under per-lane serialization and suppresses new turns while pending/running. Cancel the actual run_cancelable token and join backend/task completion before external effects. Then real typed Done rename, archive, owned removals including registered cwd, final generation/status-guarded closed/history/previous-cwd receipt commit. Preserve W conditional Discord startup and all current capture/outcome/inbox generation fences. Required lifecycle seam has no successful no-op defaults; a thread-bound lane without transport fails visibly, while threadless headless effects are not-applicable.

Tracked or untracked dirty non-force removal refusal retains verified_done for ordinary close, records failed intent and one scrubbed close_failed event per attempt. Forced failure preserves captured original status, never fabricates verified_done. Preserve branches, useful Git stderr and typed Discord status/code. **close_failed retains current Jev admission policy**, not a silent critical-kind change. No periodic destructive retries: explicit retry or interrupted-operation startup reconciliation only.

Startup after socket ownership reconstructs persisted phases and already verified_done close requests, without rerunning Task 13 signals. Removal uncertainty: matching original linkage allows non-force retry; absence of both original registration/path can reconcile removal only for a previously authorized removing attempt; replacement, mismatch or ambiguous absence fails without deletion. Thread target-state operations may reconcile/repeat idempotently under the same binding/generation. These receipts cover external uncertainty; a SQL CAS alone does not make Git or Discord atomic.

## Registered cwd recreation before reopen

For closed lane with missing previous registered cwd, require its completed, verified removal receipt and preserved existing branch in the same surviving repository. Confirm path is absent, branch exists and is not checked out elsewhere; do not bypass Git's checks. Recreate only with argv `git -C <control-repo> worktree add <previous-cwd> <existing-branch>`. No -b/-B, branch creation, --force, alternate path, inferred ownership or fallback branch. Revalidate resulting linked registration and branch, persist restoration receipt/new identity under the same lane intent/generation fence. Missing branch, occupied path, foreign/replacement linkage or missing removal proof is a visible restoration failure; preserve closed and pending reply, never start the lane or route its reply to chat.

If cwd still exists, do not recreate it: validate it matches the registered expected linkage, or for unowned external cwd use existing cwd validity contract without granting cleanup authority. An interrupted restoring attempt cannot treat arbitrary later path presence as success. Persist the intended path/branch and any observed created linkage; if the creation-to-receipt gap is ambiguous, fail closed for explicit reconciliation instead of adopting a replacement. Preserve branch movement by its legitimate history; never reset branch contents merely to match old commits.

Only after cwd is usable, restore Working/unarchive while DB lane remains closed, then transactionally perform existing closed -> working reset/history, exact first prompt and applied receipt. verified_done owner reply first completes safe close and its removal receipts. Duplicate Discord messages remain consumed after reopening, with zero chat route/steer/debounce. Later distinct pending ordinary texts for the original generation enter the lane inbox once without another reopen history. Manual CLI reopen uses the same cwd/lifecycle boundary after c05 integration; direct verified_done reopening stays illegal.

## Ordinary owner text and existing controls

Ordinary owner text in closed/verified_done lane threads is the required new behavior. Intercept after existing admission, before chat context/reactions/steer/debounce; do not require ordinary mention/activity gating to reopen a matched archived lane thread. Preserve exact owner text and durable identity.

Attachments and /stop retain existing admission, hydration/control behavior; no invented unsupported-control subsystem or new attachment manager. Keep /stop on its existing immediate control path rather than treating it as a reopen prompt. Do not consume attachment-only traffic as empty ordinary reopening text. The adapter integration must preserve the existing control branch and attachment path ordering; any specific lane-control redirection needs an explicit narrow lead decision, not a silent chat fallback for a receipted ordinary owner reply. The exactly-once/no-chat acceptance here applies to the required ordinary-text reopen path.

## Allowed paths and deferred patch units

Current writes are only these evidence artifacts. No implementation while Task 13 gate is frozen.

- After release, independent typed lifecycle seam in `src/lanes/discord_threads.rs` remains Unit B of the historical patch artifact, with required status/archive/unarchive methods delegating to existing helper. No N sink defaults or posts.rs changes.
- `src/discord/adapter.rs`: narrow admission reuse, exact owner resolver, ordinary-text receipt interception, existing controls preserved. No extra owner config/PoiseData field/main assignment from old Unit A. Optional shared resolver extraction may touch only `src/multiplexer/actor.rs` owner expression; durability code is out of scope.
- `src/storage/lanes.rs`, one approved lifecycle migration: close/reply/binding/capability/removal/restoration receipts and uniqueness. Do not apply historical Unit C unchanged: it lacks branch/cwd restoration proof.
- `src/lanes/runner.rs`, `src/lanes/cli.rs`: **deferred until c05 source and gate settle**. Fresh contexts only. Service cancellation/join/close/recreate/reopen ordering; minimal explicit registration command and truthful pending response. Keep settler owned by Task 13 and avoid whole-file N replacement.
- Existing runner/reopen/dispatch and focused close/adapter tests after release; no tests/Cargo/mod edits now. No worktree manager, env edit, new model config, actor/backend rewrite, branch management or product Git action.

## Required future proof and remaining lead decisions

Primary synthetic literal QA: authorize/register the actual lane linked-worktree cwd and existing branch; settle to verified_done; close; verify that cwd is gone from path and Git worktree list, branch remains, Done/archive and closed receipt are real; ordinary owner reply recreates the same path on the same existing branch, preserves session continuity, and reaches one first prompt with zero chat routing. Auxiliary-only removal and external-cwd skip are separate cases, not substitutes.

Also prove dirty removal failure/verified_done retention, missing branch/occupied path/unproved absence denial, restart at removal/restoration receipt gaps, duplicate reply/no chat, unique-thread migration rejection and undelivered-event reconstruction. Subscription-before-trigger and bounded exact signals, no sleeps. Existing historical helper loopback receipts are neighboring evidence only; no new pass is claimed. No external Discord QA by this agent track; future synthetic Git/DB/loopback checks require parent release and one coordinated batch.

Selected now: actor-exact owner resolution; explicit owned lane cwd removal/recreation on preserved branch; external unowned skip; ordinary owner text; existing control behavior; current Jev close_failed policy. Remaining decisions are mechanical but explicit: schema/migration slot, stable registration identity proof and ambiguous restoration reconciliation, local authorized registrar boundary, evidenced legacy bot binding/duplicate remediation, and exact service/storage integration after c05 gate. This is integrated-ready evidence, not an implemented or confirmed Task 15.
