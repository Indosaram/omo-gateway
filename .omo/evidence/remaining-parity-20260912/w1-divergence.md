# Intentional and Not-Applicable Divergence Ledger

Audit source: `.omo/evidence/hermes-parity-20260905/matrix.md` (412 lines)  
Working repository: `/Users/indo/code/project/omon-gateway` (HEAD `1f75c65`)  
Reference: Installed Hermes checkout `/Users/indo/.hermes/hermes-agent` (HEAD `146f4ed`)  
Date: 2026-09-12  

---

## VERDICT

IMPLEMENTED (14 INTENTIONAL divergences and 10 NOT-APPLICABLE divergences classified and verified; 0 unclassified).

---

## INTENTIONAL DIVERGENCES (14 items)

| ID | Contract / Role | Current Rust Citation (Matrix Citation) | Hermes Citation |
|---|---|---|---|
| AP.AP17 | Canonical authorized roots for file/cwd operations; cwd is not an OS sandbox, args may name external files and executables use PATH; Hermes write approval defaults off | `src/tools/file.rs:49-60,237-259`, `src/tools/terminal.rs:143-184` (`src/tools/file.rs:49-60,237-259`, `src/tools/terminal.rs:146-184`) | `tools/approval.py:2862-2874,2910-2917` |
| AP.AP21 | Dedicated per-bot channel sessions, per-agent workspace roots, and 300s interactive turn timeout ceiling supersede Hermes shared/per-message lanes | `src/agent/omo_backend.rs:303-345,1179-1184` (`src/agent/omo_backend.rs:221-284,393-405`) | `gateway/session.py:863-881,904-990` |
| CFG.I01 | Final-only Discord responses, immediate persistent typing, and PNG tables supersede Hermes live preview and display configuration knobs | `src/discord/adapter.rs:2596-2601,2688-2690` (`src/discord/adapter.rs:1843-1846,1910-1912`, `omo_backend.rs`) | `gateway/display_config.py:34-74,182-241`, `gateway/config.py:691-784` |
| CFG.I02 | Permanent dedicated bot DM/channel/thread sessions and per-agent workspace/memory supersede automatic idle/daily reset, pruning, and participant splitting | `src/main.rs:676-685`, `src/models/session.rs:59-65` (`src/main.rs:1252-1258`) | `gateway/config.py:456-510,870-896` |
| CFG.I03 | Single multiplexed interactive daemon with per-agent state and 300s timeout ceiling; no restoration of one gateway per Hermes profile | `src/main.rs:660-705,1540-1544`, `src/agent/omo_backend.rs:1179-1184` (`src/main.rs:1237-1250,1322-1336`, `backend :393-398`) | `gateway/config.py:874-879`, `hermes_cli/profiles.py:949-987` |
| CR.C22 | Final-only Discord output and PNG tables; OMO agent backend; default-on local session transcript mirroring rather than Hermes opt-in | `src/cron/scheduler.rs:1725-1736`, `src/discord/adapter.rs:2596-2601` (`src/cron/scheduler.rs:1204-1227`, `src/main.rs:601-603`, `src/discord/adapter.rs:1910-1912`) | `cron/scheduler.py:1480-1519,624-654` |
| D.D28 | Suppression of interim streaming prose to eliminate live-edit flicker; final-only chunked messages via LiveEditThrottler | `src/discord/adapter.rs:2596-2601,2688-2690`, `src/discord/throttler.rs:210-225` (`src/discord/adapter.rs:1843,1910`, `src/discord/throttler.rs:166-189`) | `plugins/platforms/discord/adapter.py:3085-3289` |
| D.D29 | Multiplexed bot profiles, per-agent workspaces, and remote thread resumption replace independent Hermes agent intelligence and session rotation | `src/multiplexer/actor.rs:778-860`, `src/agent/omo_backend.rs:215-280,303-345` (`src/main.rs:1251-1260`, `src/multiplexer/actor.rs:498`, `src/agent/omo_backend.rs:158-209,224-287,396-410`) | `gateway/profile_routing.py:76-104`, `gateway/session.py:1449-1500,1786-1826` |
| D.D31 | Readiness probes and drain watchers handle gateway runtime lifecycle; cron fan-out/mirroring and multi-tool security map to their respective subsystem owners rather than generic Discord parity | `src/main.rs:847-857,872-880` (`src/main.rs:1397-1410,1418`) | `gateway/readiness.py:26-115`, `gateway/drain_control.py:189-227` |
| R.R20 | Discord adapter live preview intentionally differs: final-only output, persistent immediate typing, PNG tables; preview churn parity excluded | `src/discord/adapter.rs:2596-2601,2688-2690` (`src/discord/adapter.rs:1842-1846,1910-1912`) | `plugins/platforms/discord/adapter.py:3085-3289` |
| R.R21 | Provider interaction and session intelligence delegated entirely to OMO app-server; direct LLM provider reconstruction excluded in favor of permanent bot sessions and 300s interactive cap | `src/agent/omo_backend.rs:509-525,1179-1184` (`src/main.rs:1235-1255`, `src/agent/omo_backend.rs:349-364`) | `gateway/session.py:2705-2724` |
| S.S07 | Per-command environment maps for local subprocesses and per-agent cwd/memory roots in OMO; no replication of Python's global contextvars/session_context machinery in multiplexed daemon | `src/tools/terminal.rs:364-375`, `src/agent/agent_workspace.rs:40-80` (`src/tools/terminal.rs:291-305,339-386`, `src/agent/agent_workspace.rs:40-80`, `src/agent/omo_backend.rs:225-284`) | `gateway/session_context.py:156-217,260-310` |
| S.S09 | Actor GC evicts in-memory actor handles while preserving persistent SQLite session records; no automatic idle conversation wipes, daily expirations, or named-session rotations | `src/multiplexer/gc.rs:45-95` (`src/multiplexer/gc.rs:45-95`, `src/multiplexer/actor.rs:286-303`) | `gateway/session.py:1641-1711`, `gateway/turn_lease.py:216-275` |
| S.S10 | Discord adapter drops interim non-final prose; single interactive app-server daemon multiplexes per-bot workspaces without Hermes stream commentary | `src/discord/adapter.rs:2596-2601,2688-2690`, `src/agent/omo_config.rs:221-250` (`src/discord/adapter.rs:1843-1846,1910-1912`, `src/main.rs:1323-1338`, `src/agent/omo_config.rs:224-246`) | `gateway/stream_events.py:1-33,73-120` |

---

## NOT-APPLICABLE DIVERGENCES (10 items)

| ID | Contract / Role | Current Rust Citation (Matrix Citation) | Hermes Citation |
|---|---|---|---|
| AP.AP20 | Hermes auxiliary smart approval reviewer and code execution elicitation are not applicable because OMO daemon owns agent tools and intelligence | `src/tools/browser.rs:124-127`, `src/agent/omo_backend.rs:509-515` (`tools/browser.rs:124-127`, `omo_backend.rs:347-359`) | `tools/approval.py:2562-2641,3635-3950` |
| CFG.N01 | Non-Discord platform credential/env bridges, HTTP webhook relays, and cross-platform egress excluded; gateway deployment is Discord-only | `src/cron/store.rs:580-590` (`src/cron/store.rs:248-251`) | `gateway/config.py:1803-2544` |
| CFG.N02 | Migration and service lifecycle scoped to local macOS launchd; Linux systemd service migration excluded | `src/migrate/gateway_down.rs:40-75` (`src/migrate/gateway_down.rs:74-87`) | `gateway/config.py:153-190,882-884` |
| CFG.N03 | Hermes xAI retirement rewrites, provider fallback engines, CLI skins, managed enterprise admin, and secret-manager fetching are engine capabilities outside gateway scope | `src/agent/omo_config.rs:16-29` (`src/agent/omo_config.rs:6-29`) | `hermes_cli/migrate.py:16-115`, `gateway/display_config.py:29-32` |
| CR.C23 | External NAS/scale-to-zero trigger mechanisms and non-Discord topic/voice delivery channels excluded | `src/cron/scheduler.rs:1807-1810` (`src/cron/store.rs:260-263`, `src/cron/scheduler.rs:1340-1342`) | `cron/scheduler_provider.py:1-19,122-160`, `gateway/delivery.py:449-548` |
| CR.C24 | Cron blueprint catalogs, suggestion generation, and script item classification intelligence are outside gateway runtime parity | `src/tools/cron.rs:55-61` (`src/tools/cron.rs:51-54`) | `cron/blueprint_catalog.py:1-21,661-713`, `cron/suggestions.py:223-244`, `cron/suggestion_catalog.py:124-154`, `cron/scripts/classify_items.py:147-190` |
| D.D30 | Opt-in Discord voice channel ambient/TTS mixing (`voice_mixer.py`) is excluded from text-only gateway runtime; gateway STT/voice notes are handled via file attachments | `src/voice/mod.rs:15-20` (`src/voice/mod.rs:14-19`) | `plugins/platforms/discord/voice_mixer.py:1-379` |
| R.R19 | Out-of-band transcript mirroring to non-Discord platforms excluded; Discord transcript mirroring is preserved | `src/mirror.rs:81-85`, `src/cron/scheduler.rs:1807-1810` (`src/mirror.rs:62-68`, `src/cron/scheduler.rs:1275-1279`) | `gateway/mirror.py:1-9` |
| S.S02 | Provider conversation history compaction, sanitization, and alternation delegated to OMO backend; gateway does not maintain direct LLM context | `src/agent/omo_backend.rs:509-515` (`src/main.rs:1234-1257`, `src/agent/omo_backend.rs:347-360`) | `gateway/session.py:2705-2724` |
| S.S08 | CLI and cross-platform session continuity/rebinding excluded; cron creates fresh remote threads; Discord cron transcript mirroring retained separately | `src/agent/omo_backend.rs:215-218` (`src/agent/omo_backend.rs:160-163,309-315`) | `gateway/session.py:2404-2471` |

---

## EXPLICIT EXCLUSION BOUNDARIES

Extracted verbatim in scope and intent from `### Acceptance boundaries` and `### Latest-pin policy and delivery boundaries` of `matrix.md`:

1. **Interim Discord Prose & Live-Preview Streaming**: No restoration of interim Discord prose preview streaming or live-edit status chatter. Responses remain strictly final-only (`completed_stream_content`), decorated with immediate persistent typing indicators and lossless PNG table conversion.
2. **Automatic Session Expiry & Conversation Resets**: No restoration of automatic session expiry, daily expiration routines, or idle conversation resets. Conversations remain permanent dedicated sessions keyed by bot and channel/thread.
3. **Per-User Guild Conversation Splitting**: No restoration of per-user session isolation within shared guild channels or threads. Dedicated per-bot channel/thread lanes take precedence over Hermes per-user defaults.
4. **Hermes Agent Intelligence Porting**: No porting of Hermes agent intelligence, direct provider conversation sanitization/compression, provider alternation, inference routing/fallbacks, smart approval reviewers, or code execution elicitation. Agent tools, model execution, and reasoning belong entirely to the OMO app-server daemon.
5. **Non-Discord Platforms & Transports**: No restoration of non-Discord platform transports (Telegram, Slack, HTTP relay/webhook listeners, external NAS/scale-to-zero triggers, cross-platform CLI continuity). The gateway deployment is strictly Discord-only.
6. **Linux Fleet Infrastructure**: No restoration of Linux service migration or fleet infrastructure. Lifecycle management and process supervision are strictly scoped to macOS `launchd` and local process controls.
7. **Voice Channel (VC) Ambient & TTS Mixing**: Voice chat ambient listening and TTS audio mixing (`voice_mixer.py`) are deliberately excluded from this final-only text gateway deployment. Native Discord voice note downloads and inbound STT are retained as Discord attachment operations.
8. **Auxiliary Catalogs & Proposal Intelligence**: Blueprint catalogs, cron suggestion generators, and automated script classification heuristics are outside assigned gateway runtime parity.
9. **Pairing Admin CRUD Extensions**: Admin pairing listing and interactive revocation endpoints mentioned in audit discussions are not required defects; pairing is handled via explicit operator approval and persisted tokens.
10. **Extended fnmatch Bracket Syntax**: Terminal security filters support `*` and `?` glob wildcards; full POSIX/fnmatch character bracket classes are not required parity.
11. **Agent Turn Replay / Resubmission on Delivery Retry**: Transport-level retries must strictly distinguish unsent, definitively failed, and ambiguous output. Retries apply to delivery only: no agent resubmission, no repeated `turn/start`, no additional daemon invocation, and no conversation resets.
12. **Multiple Gateways per Hermes Profile**: Interactive work is consolidated into a single multiplexed gateway daemon supporting per-agent workspaces; no separate gateway process per Hermes profile.

---

## EVIDENCE

Current-tree citations and 1-3 line source quotations verifying each intentional and not-applicable divergence:

### INTENTIONAL Divergences

- **AP.AP17** (`src/tools/file.rs:49-51` and `src/tools/terminal.rs:155-157`):
  ```rust
  // src/tools/file.rs:49-51
  pub fn is_authorized(&self, canonical_path: &Path) -> bool {
      if let Ok(root) = self.canonical_root() {
          if canonical_path.starts_with(&root) {
  ```
  ```rust
  // src/tools/terminal.rs:155-157
  let path = canonical(&target)?;
  if !self.is_authorized(&path) || !path.is_dir() {
      return Err(OmonError::ToolExecution("working directory escapes tool root".into()));
  ```

- **AP.AP21** (`src/agent/omo_backend.rs:303-306` and `src/agent/omo_backend.rs:1179-1183`):
  ```rust
  // src/agent/omo_backend.rs:303-306
  let workspace = if self.config.per_agent_workspace {
      if let Some(root) = &self.config.workspace_root {
          let slug = agent_workspace_slug(
  ```
  ```rust
  // src/agent/omo_backend.rs:1179-1183
  let effective_total_timeout = if is_cron_session {
      self.config.total_timeout
  } else {
      self.config.total_timeout.min(Duration::from_secs(300))
  };
  ```

- **CFG.I01** (`src/discord/adapter.rs:2597-2600` and `src/discord/adapter.rs:2688-2690`):
  ```rust
  // src/discord/adapter.rs:2597-2600
  let Some(content) = completed_stream_content(&chunk.content, chunk.is_final) else {
      self.keep_typing(&session).await;
      return Ok(());
  };
  ```
  ```rust
  // src/discord/adapter.rs:2688-2690
  fn completed_stream_content(content: &str, is_final: bool) -> Option<&str> {
      is_final.then_some(content)
  }
  ```

- **CFG.I02** (`src/main.rs:676-678`):
  ```rust
  // src/main.rs:676-678
  let profile_router = ProfileRouter::new(config.profile_routes.clone());
  let multiplexer = SessionMultiplexer::with_profile_router(
      pool.clone(),
  ```

- **CFG.I03** (`src/main.rs:1540-1544`):
  ```rust
  // src/main.rs:1540-1544
  assert_eq!(
      cron.appserver_url, interactive.appserver_url,
      "Cron and interactive must share one daemon endpoint"
  );
  ```

- **CR.C22** (`src/cron/scheduler.rs:1725-1729`):
  ```rust
  // src/cron/scheduler.rs:1725-1729
  let attach_to_session = job
      .payload()
      .ok()
      .and_then(|p| p.get("attach_to_session").and_then(Value::as_bool))
      .unwrap_or(true);
  ```

- **D.D28** (`src/discord/throttler.rs:210-213`):
  ```rust
  // src/discord/throttler.rs:210-213
  let chunks = bound_split_messages(
      chunk_markdown(content, DISCORD_MESSAGE_LIMIT),
      MAX_SPLIT_CHUNKS,
  );
  ```

- **D.D29** (`src/multiplexer/actor.rs:811-813`):
  ```rust
  // src/multiplexer/actor.rs:811-813
  if let Some(router) = profile_router {
      if let Some(target) = router.route(key) {
          apply_route_target(context, target);
  ```

- **D.D31** (`src/main.rs:847-850` and `src/main.rs:872-875`):
  ```rust
  // src/main.rs:847-850
  let readiness = omon_gateway::collect_runtime_readiness(
      &pool,
      &config.workspace_root,
      &config.default_model,
  ```
  ```rust
  // src/main.rs:872-875
  let drain_watcher = omon_gateway::DrainWatcher::new(
      config.workspace_root.clone(),
      std::time::Duration::from_secs(3),
  );
  ```

- **R.R20** (`src/discord/adapter.rs:2597-2600`):
  ```rust
  // src/discord/adapter.rs:2597-2600
  let Some(content) = completed_stream_content(&chunk.content, chunk.is_final) else {
      self.keep_typing(&session).await;
      return Ok(());
  };
  ```

- **R.R21** (`src/agent/omo_backend.rs:509-511`):
  ```rust
  // src/agent/omo_backend.rs:509-511
  ws.send(turn_start_request(&thread_id, &user_prompt, model))
      .await
      .map_err(|e| OmonError::Llm(format!("failed to send turn/start: {e}")))?;
  ```

- **S.S07** (`src/tools/terminal.rs:364-367`):
  ```rust
  // src/tools/terminal.rs:364-367
  if let Some(session) = session {
      for (key, value) in build_session_environment(session) {
          command.env(key, value);
      }
  ```

- **S.S09** (`src/multiplexer/gc.rs:70-74`):
  ```rust
  // src/multiplexer/gc.rs:70-74
  Ok(Ok(true)) => {
      multiplexer.remove_handle(&key, &handle);
      handle.mark_finished();
      evicted += 1;
  }
  ```

- **S.S10** (`src/discord/adapter.rs:2688-2690`):
  ```rust
  // src/discord/adapter.rs:2688-2690
  fn completed_stream_content(content: &str, is_final: bool) -> Option<&str> {
      is_final.then_some(content)
  }
  ```

---

### NOT-APPLICABLE Divergences

- **AP.AP20** (`src/tools/browser.rs:124-126`):
  ```rust
  // src/tools/browser.rs:124-126
  "eval" | "screenshot" => Err(OmonError::ToolExecution(format!(
      "browser action `{action}` requires a CDP WebSocket session and is not configured"
  ))),
  ```

- **CFG.N01** (`src/cron/store.rs:583-586`):
  ```rust
  // src/cron/store.rs:583-586
  if origin.platform.eq_ignore_ascii_case("discord")
      && !origin.chat_id.is_empty()
  {
  ```

- **CFG.N02** (`src/migrate/gateway_down.rs:43-45`):
  ```rust
  // src/migrate/gateway_down.rs:43-45
  let plist = home
      .join("Library/LaunchAgents")
      .join("ai.hermes.gateway.plist");
  ```

- **CFG.N03** (`src/agent/omo_config.rs:16-20`):
  ```rust
  // src/agent/omo_config.rs:16-20
  Some(v)
      if v.eq_ignore_ascii_case("llm")
          || v.eq_ignore_ascii_case("hermes")
          || v.eq_ignore_ascii_case("direct") =>
  {
  ```

- **CR.C23** (`src/cron/scheduler.rs:1807-1809`):
  ```rust
  // src/cron/scheduler.rs:1807-1809
  if destination.platform.eq_ignore_ascii_case("discord") {
      return job.discord_destinations();
  }
  ```

- **CR.C24** (`src/tools/cron.rs:56-60`):
  ```rust
  // src/tools/cron.rs:56-60
  if auth == "hermes_mirror" || auth == "hermes_synced" {
      return Err(OmonError::ToolExecution(format!(
          "cannot modify imported job '{id}' (read-only)"
      )));
  }
  ```

- **D.D30** (`src/voice/mod.rs:15-19`):
  ```rust
  // src/voice/mod.rs:15-19
  pub struct SongbirdAudioEventListener {
      channel_id: u64,
      sender: mpsc::Sender<AudioFrame>,
      sequence: AtomicU64,
  }
  ```

- **R.R19** (`src/mirror.rs:81-83`):
  ```rust
  // src/mirror.rs:81-83
  pub async fn find_session_by_origin(
      pool: &SqlitePool,
      platform: &str,
  ```

- **S.S02** (`src/agent/omo_backend.rs:509-511`):
  ```rust
  // src/agent/omo_backend.rs:509-511
  ws.send(turn_start_request(&thread_id, &user_prompt, model))
      .await
      .map_err(|e| OmonError::Llm(format!("failed to send turn/start: {e}")))?;
  ```

- **S.S08** (`src/agent/omo_backend.rs:215-218`):
  ```rust
  // src/agent/omo_backend.rs:215-218
  if is_cron {
      session.state.metadata.remove("omo_thread_id");
      self.thread_ids.lock().remove(&storage_key);
  }
  ```

---

## RESIDUAL

None. All 14 INTENTIONAL and 10 NOT-APPLICABLE divergence items reflect permanent system invariants, architectural decisions (such as final-only Discord responses, single-daemon multiplexing, and OMO daemon delegation), or out-of-scope non-Discord / Linux features. None of these items represent remaining defect backlog work.

---

## CLAIM

CLOSED: All 14 INTENTIONAL and 10 NOT-APPLICABLE divergence rows plus explicit exclusion boundaries fully extracted and verified against current tree.
