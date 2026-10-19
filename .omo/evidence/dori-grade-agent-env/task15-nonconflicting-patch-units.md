# Task 15 nonconflicting patch units and deferred interfaces

SUPERSEDED patch-preparation snapshot. Use `task15-integrated-ready.md` for the selected contract. Do not apply Unit A's numeric owner conversion or Unit C's incomplete ownership schema below: exact actor string resolution and receipt-backed lane cwd removal/recreation supersede them. Unit B's required typed lifecycle seam remains a usable independent proposal after release. Auxiliary-only registration and invented unsupported-control behavior below are withdrawn, not current limitations.

2026-10-11. Evidence-only patch preparation, not an applied product patch. This document is the sole intentional write. No source/test/Cargo, Git, DB, network, model or service action occurred. Task 14 final confirmation at ledger 19/41 is supplied by the lead's latest message, not independently re-probed here. Worker c05 owns Task 13's active runner/CLI/mod/Cargo/tests integration; all hunks in those files are excluded. Preserve actor/backend durability and current generation fences.

W = `/Users/indo/code/project/omon-gateway-wt/dori-work`. Patch paths below are relative to W. Source contexts were read this turn. Units A/B can be applied independently by the lead after release; units C/D need the stated contract decisions. Nothing below constitutes schema authority, implementation success, compilation evidence or permission to run probes.

## Corrected owner configuration finding

The prior proposal found no owner-specific env field but missed existing actor policy. `src/multiplexer/actor.rs:968-978` reads DISCORD_ALLOWED_USERS, selects the first nonempty trimmed entry, falls back to `414011632306618368`, and compares that identity for owner DM behavior. `src/main.rs:105-113` separately parses all valid numeric entries as general admission. `src/inbox/sources/discord.rs:19` has the same compiled ID but no config derivation. Therefore first configured entry is an actual existing owner convention, not permission for every allowlisted/paired/role-authorized human. No actor/inbox refactor is proposed.

Recommended Task 15 policy: snapshot the first nonempty raw configured entry at startup; if absent/empty, use the existing compiled owner default; if present but invalid numeric ID, disable owner reopening or fail its config explicitly, never skip to a later authorized user. Still require ordinary channel/user admission and a matched lane bot binding. A dedicated owner env setting is unnecessary for this increment. The lead must accept reuse of this existing owner convention; it is not an independently documented security authority. No live env or .env was inspected/edited.

## Unit A: explicit configured owner field, no actor changes

Scope: `src/discord/commands.rs`, one assignment in `src/main.rs`. Retains PoiseData::new's inert default for fixtures. This reads the same raw setting as the actor rather than assuming parsed allowed_users.first() preserves invalid-entry semantics.

```diff
*** Begin Patch
*** Update File: src/discord/commands.rs
@@
     pub free_response_channels: Vec<u64>,
     pub allowed_users: Vec<u64>,
+    pub lane_owner_id: Option<u64>,
     pub allowed_roles: Vec<u64>,
     pub allow_all_users: bool,
     pub thread_sessions_per_user: bool,
@@
             free_response_channels: Vec::new(),
             allowed_users: Vec::new(),
+            lane_owner_id: None,
             allowed_roles: Vec::new(),
             allow_all_users: false,
             thread_sessions_per_user: true,
*** Update File: src/main.rs
@@
     poise_data.tool_registry = tools.clone();
     poise_data.free_response_channels = config.free_response_channels.clone();
     poise_data.allowed_users = config.allowed_users.clone();
+    poise_data.lane_owner_id = env::var("DISCORD_ALLOWED_USERS")
+        .ok()
+        .and_then(|value| {
+            value.split(',').map(str::trim).find(|id| !id.is_empty())
+                .map(ToString::to_string)
+        })
+        .unwrap_or_else(|| "414011632306618368".to_string())
+        .parse::<u64>()
+        .ok()
+        .filter(|id| *id != 0);
     poise_data.allowed_roles = config.allowed_roles.clone();
     poise_data.allow_all_users = config.allow_all_users;
     poise_data.thread_sessions_per_user = config.thread_sessions_per_user;
*** End Patch
```

The adapter will compare author ID to lane_owner_id, never allowed_users membership alone. Invalid explicit owner yields None; the later admission path must log disabled owner reopening, not silently treat None as everyone. This patch adds config data only; it does not intercept or change current chat behavior. Rust formatting and type checks are deferred, not asserted. Check any new PoiseData struct-literal sites at integration; no tests are edited here.

## Unit B: required typed lifecycle seam, no successful defaults

Scope: append this interface/implementation immediately before DiscordThreads' declaration at `src/lanes/discord_threads.rs:336`. Existing inherent methods supply production operations, so the trait does not invent transport or reuse N's no-op PostSink defaults. Return types keep typed DiscordThreadError available to the service; using () for the archive result does not swallow errors.

```diff
*** Begin Patch
*** Update File: src/lanes/discord_threads.rs
@@
-#[derive(Clone)]
-pub struct DiscordThreads {
+#[async_trait::async_trait]
+pub trait LaneThreadLifecycle: Send + Sync {
+    async fn set_status(
+        &self,
+        thread: ChannelId,
+        status: ThreadStatus,
+    ) -> Result<String, DiscordThreadError>;
+    async fn archive(&self, thread: ChannelId) -> Result<(), DiscordThreadError>;
+    async fn unarchive(&self, thread: ChannelId) -> Result<(), DiscordThreadError>;
+}
+
+#[async_trait::async_trait]
+impl LaneThreadLifecycle for DiscordThreads {
+    async fn set_status(
+        &self,
+        thread: ChannelId,
+        status: ThreadStatus,
+    ) -> Result<String, DiscordThreadError> {
+        DiscordThreads::set_status(self, thread, status).await
+    }
+
+    async fn archive(&self, thread: ChannelId) -> Result<(), DiscordThreadError> {
+        DiscordThreads::archive(self, thread).await.map(|_| ())
+    }
+
+    async fn unarchive(&self, thread: ChannelId) -> Result<(), DiscordThreadError> {
+        DiscordThreads::unarchive(self, thread).await.map(|_| ())
+    }
+}
+
+#[derive(Clone)]
+pub struct DiscordThreads {
     http: Arc<Http>,
 }
*** End Patch
```

No change to posts.rs, milestone state or helper HTTP payloads. Thread-bound close with no lifecycle sink must fail visibly; a threadless headless lane records not-applicable without requiring a token. Runner injection is deferred to c05 settlement completion.

## Unit C: migration draft, subject to lead schema decision

Proposed single `migrations/0038_lane_lifecycle.sql`; W still lists 0037 as latest at this read. Recheck slot in both trees before applying. This is a candidate schema, not an applied migration or authority to open any DB. Keep bot binding in its own table to avoid changing every explicit Lane projection while c05 works.

```sql
CREATE UNIQUE INDEX idx_lanes_unique_thread
    ON lanes(thread_channel_id) WHERE thread_channel_id IS NOT NULL;

CREATE TABLE lane_thread_bindings (
    lane_key TEXT PRIMARY KEY REFERENCES lanes(key),
    bot_id TEXT NOT NULL,
    bound_at DATETIME NOT NULL
);

CREATE TABLE lane_close_intents (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    generation INTEGER NOT NULL,
    from_status TEXT NOT NULL,
    force INTEGER NOT NULL CHECK(force IN (0,1)),
    note TEXT,
    state TEXT NOT NULL CHECK(state IN ('pending','running','failed','completed')),
    phase TEXT NOT NULL CHECK(phase IN ('cancel_join','done_status','archive','worktrees','commit')),
    attempt INTEGER NOT NULL DEFAULT 1,
    error_kind TEXT,
    error_detail TEXT,
    created_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL,
    UNIQUE(lane_key,generation),
    CHECK(force = 0 OR (note IS NOT NULL AND note LIKE 'forced:%')),
    CHECK(force = 1 OR note IS NULL OR note NOT LIKE 'forced:%')
);

CREATE TABLE lane_owner_replies (
    platform_message_id TEXT PRIMARY KEY,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    bot_id TEXT NOT NULL,
    author_id TEXT NOT NULL,
    text TEXT NOT NULL,
    generation INTEGER NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending','applied','failed')),
    error_detail TEXT,
    reopen_history_id INTEGER REFERENCES lane_history(id),
    received_at DATETIME NOT NULL,
    updated_at DATETIME NOT NULL
);

CREATE TABLE lane_owned_worktrees (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    lane_key TEXT NOT NULL REFERENCES lanes(key),
    path TEXT NOT NULL,
    control_cwd TEXT NOT NULL,
    common_git_dir TEXT NOT NULL,
    registration_dir TEXT NOT NULL,
    registration_identity TEXT NOT NULL,
    registrar TEXT NOT NULL,
    registered_at DATETIME NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('registered','removing','removed')),
    close_intent_id INTEGER REFERENCES lane_close_intents(id)
);
CREATE UNIQUE INDEX idx_lane_owned_worktrees_live_path
    ON lane_owned_worktrees(path) WHERE state <> 'removed';
```

Unique-thread preflight, deferred: on an authorized scratch/pre-migration DB, `SELECT thread_channel_id, COUNT(*) FROM lanes WHERE thread_channel_id IS NOT NULL GROUP BY thread_channel_id HAVING COUNT(*) > 1;`. Any existing duplicate blocks migration; report lane keys and choose explicit remediation with the lead, never delete or auto-select rows. No duplicate probe ran here. Null thread IDs remain valid.

Capability rows are granted only by an authorized local CLI registration/create producer. They are not backfilled from cwd or timestamp selection. Existing start creates no worktree, so ordinary external cwd starts with zero capability rows and close records skipped_no_owned_worktrees rather than asserting removal. Proposed auxiliary-only registration rejects main worktree, lane cwd, surviving control cwd and already delegated live path. Git registration identity must be defined before this table is applied; storing an arbitrary string alone is not deletion authority.

Lead decisions that gate C: accept new tables versus equivalent minimal representation; choose registration identity proof (canonical common-dir/back-pointer plus stable filesystem registration identity, fail closed on unsupported/mismatch); define authoritative local CLI registrar boundary; authorize migration slot and duplicate remediation. The text itself supplies none of those authorities. If capability registration is deferred, do not apply its table or claim removal acceptance.

## Unit D: storage and adapter exact insertion boundaries, deferred function bodies

`src/storage/lanes.rs` gets lifecycle helpers adjacent to get_lane_by_thread, without altering Lane fields/projections. Candidate signatures are contracts, not compiled code:

- request_close(pool,key,force,note) -> intent ID/status: transaction write-lock, capture MAX reopened history generation, verify normal eligibility, insert or return same pending intent; only explicit retry changes failed -> pending and increments attempt.
- record_owner_reply(pool,message_id,key,bot_id,author_id,text) -> new/duplicate receipt: unique identity and original generation transactionally; reject binding mismatch. Duplicate must return consumed even if current lane status is working.
- apply_owner_reply(transaction,receipt) -> reopen history ID: existing reopen reset/history/inbox semantics in the same transaction as applied receipt; closed only. Do not implement this while c05 owns runner::reopen callers. Later distinct pending replies from the same original generation insert once into current lane inbox without a second reopen history.
- capability_register(pool,validated_record): only the explicit authorized registration producer may call; no scanning/backfill helper. Validate Git linkage outside DB transaction, revalidate before removal; persist exact identity.

Adapter insertion: FullEvent::Message currently builds InboundFilterConfig at `src/discord/adapter.rs:895-914`, then calls message_to_inbound_with_config at `:915-919`. Interception belongs between those blocks and chat conversion. Extract/reuse the admission prefix at `:1556-1627` without chat mention/activity gating. Owner check is author == data.lane_owner_id; reject bot/webhook/self/system messages; preserve channel and configured admission checks. Resolve unique lane thread and persisted bot binding. Check existing receipt before status-based chat fallback so replay after reopening remains lane traffic. On valid closed/verified_done owner reply persist receipt and wake service, then return from handler; on lifecycle error retain receipt/visible failure and return, never call chat route/steer/debounce. Do not claim the existing chat ledger protects this earlier hook.

No adapter patch calling nonexistent helpers is provided: that would be a stub, not an apply-ready compiling unit. C's decisions and storage implementation must settle first. Attachment-only text and /stop remain explicit lead choices; unsupported lane controls should remain consumed/visible rather than falling through chat. Normal non-lane traffic retains current chat conversion. Old thread binding cannot be guessed from primary bot alone; explicit binding backfill requires evidence.

## Deferred runner/CLI interfaces after c05 source settles

No runner/CLI/mod/Cargo hunks in this artifact. Lead takes back fresh contexts after c05 completion; do not paste prior N file versions.

1. Add lifecycle transport injection independently of PostSink, retaining W conditional Discord startup. Extend active bookkeeping with per-lane token and joined task identity. Pending/running lifecycle intent suppresses new working/not_done turns. Cancellation-and-join completes before any thread or Git effect; preserve CAPTURE_GENERATION, outcome/inbox CAS and joined service shutdown.
2. Attach to c05's newly verified callback and recover already verified_done rows at startup after socket ownership. Settler checks/reservation remain Task 13; close never rechecks arbitrary signals.
3. Service serializes close/reply per lane; CAS pending -> running; next-phase receipt after success. Order cancel_join -> Done status -> archive -> registered auxiliary removals -> generation/status-guarded closed/history commit. Ordinary failure retains verified_done; forced failure retains captured original status. Emit one scrubbed close_failed per attempt, preserving typed Discord code/status/Git reason. Force is only state permission, never Git --force or branch deletion.
4. Retry failed attempt only by explicit control request. Startup reconciles interrupted running phases: target-state thread edit is idempotent under same binding; removing reconciles original registration/path identity. Matching original permits non-force retry; absent original registration and path permits removed receipt; replacement/mismatch/ambiguous state fails without deletion. An SQL CAS alone cannot guarantee external atomicity.
5. Closed reply: while DB stays closed restore Working/unarchive, then apply reopen/inbox/receipt in one transaction, wake new turn. Verified_done reply first completes safe close. Failure retains exact reply and no chat obligation. Manual CLI reopen must use same service boundary; existing direct verified_done reopen rejection stays.
6. CLI Close returns pending intent ID, not closed success. Minimal optional register-worktree command explicitly grants auxiliary ownership and records verified linkage. No worktree manager, creation automation, branch cleanup or deletion inferred from settlement's selection. Current external cwd survives and remains reopenable.

## Exact choices still requiring lead resolution

| Choice | Proposed default | Consequence if not selected |
| --- | --- | --- |
| Owner convention | Reuse actor first nonempty configured ID/default with invalid explicit ID disabled; also require existing admission | Do not install owner interception until an explicit policy is chosen; never equate all authorized humans with owner |
| Capability registration | Explicit authorized auxiliary-only CLI delegation with fail-closed Git/registration identity | Empty-capability close valid with truthful skips, owned removal acceptance remains deferred |
| Migration/bindings | Unique thread + separate binding table; explicit evidence for legacy binding backfill | Duplicate/unbound threads cannot auto-reopen; no silent data rewriting |
| Forced cleanup failure | Original state + failed intent; force never verifies | No false verified_done or closed receipt |
| Multiple distinct pending replies | First creates reopened generation, others inbox once | Must not drop later text or create repeated reopen histories |
| /stop and attachments | Visible unsupported lane-control result until defined | No chat fallback; full control behavior remains a declared limitation |
| close_failed dispatch | Preserve existing non-critical judged admission | Do not claim guaranteed lead wake unless policy and focused test explicitly change |

Future probes are release-only: owner config fixtures, duplicate migration fixture, ordered cancellation/typed transport fake, synthetic explicitly registered clean/dirty worktree and branch-preservation checks, replacement-identity restart, duplicate/no-chat adapter calls, persistent dispatcher reconstruction. No new receipts or test passes are asserted. Existing helper loopback pass remains neighboring historical evidence; external Discord QA is forbidden for this agent track. Use one coordinated parent batch after Task 13 integration, not a competing build.

Stop condition met for preparation: A/B have exact nonconflicting source-context patches; C is a gated SQL draft; D has exact hook boundaries and required storage semantics; runner/CLI interfaces are deferred to c05 completion with explicit remaining decisions. Product Task 15 is not implemented or confirmed.
