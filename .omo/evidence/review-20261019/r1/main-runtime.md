# Lane main-runtime

I reviewed every hunk of `.omo/evidence/review-20261019/diffs/main-runtime.diff`, checking each one against the current files: `src/main.rs`, `src/entry.rs`, `src/dashboard.rs`, `src/dashboard_runtime.rs`, `src/usage.rs`, `src/lib.rs` and `src/security/tirith.rs`. That includes the uncommitted changes in `src/agent/omo_backend.rs` and `src/slack/egress.rs`. I found one P1 and two P2s. The rest of the diff holds up; what I checked and why it passes is listed after the findings.

### [P1] A single failed Slack `auth.test` at boot turns Slack off until the next restart
- Location: src/main.rs:813
- Trigger: The gateway starts with Slack configured, and the network or Slack API briefly fails during the one `auth.test` call. Typical cases are a launchd start right after login or wake, a DNS hiccup, or a Slack 5xx. `SlackApi::send` only retries on HTTP 429; a request error returns at once (src/slack/api.rs:405-407).
- Impact:
  - **Discord + Slack:** `slack_boot` is `None`, so `slack_egress`, `slack_runtime` and the Slack auth rules are never built (main.rs:1440, 1671-1712). Every Slack DM and mention gets no reply for the whole run, and nothing retries. Readiness shows `Failed("Slack auth.test failed at boot")`.
  - **Slack-only:** `PlatformRouter::for_transports(None, None)` fails and the process exits, so the outcome depends on the supervisor restarting it.
  - **Slack as home:** the same thing happens if only `open_im` fails (main.rs:833-844). `OMON_LANE_DEFAULT_LEAD` keeps the Discord default, so lane, cron-health, proposal and session-watch traffic for the owner goes to a Discord key for the whole run.
  - This is a regression in behaviour: the same diff added backoff retries for Discord's identity lookup (`fetch_current_user_with_retry`, main.rs:1086), but Slack still gets a single attempt.
- Evidence:
  ```rust
  let auth = match api.auth_test().await {
      Ok(auth) => auth,
      Err(error) => {
          warn!(%error, "Slack auth.test failed; Slack stays off for this run");
          return Ok(None);
      }
  };
  ```
- Fix: Retry `auth.test` with bounded backoff, the same way `fetch_current_user_with_retry` does. Retry on transport errors only and fail fast on `invalid_auth` or `not_authed`. Apply the same retry to `owner_dm_key` before giving up on the Slack lead.

### [P2] Dashboard "Allow Session" on a proposal-id approval is recorded as Allow Once
- Location: src/dashboard.rs:2458
- Trigger: Someone calls `POST /api/approvals/<numeric proposal id>/resolve` with `{"decision":"session"}` for a proposal that has no `platform_message_id`, so the guard matches it by proposal id.
- Impact: The session grant is quietly downgraded to Once, so the same action asks again later in that session. The UUID path keeps Session (`parse_custom_id` maps `"session"` to `ApprovalDecision::Session`, discord/approval.rs:862). The two paths behave differently for the same button.
- Evidence:
  ```rust
  "once" | "session" => omon_gateway::ApprovalDecision::Once,
  ```
- Fix: Map `"session"` to `omon_gateway::ApprovalDecision::Session`, matching `parse_custom_id`.

### [P2] `env::set_var` is called while the multi-threaded tokio runtime is running
- Location: src/main.rs:837
- Trigger: Slack is the home, `OMON_LANE_DEFAULT_LEAD` is unset, and `boot_slack` runs inside `#[tokio::main]` after `init_pool` has started.
- Impact: `setenv` is not thread-safe against concurrent `getenv` on macOS. I found no concurrent reader at this point in boot, so this is a latent soundness hazard, not an observed bug. The lead is also carried through process-global state that many modules read on their own (`default_lead()`, `actor.rs` `is_kibitzer_lead`).
- Evidence:
  ```rust
  if optional_env("OMON_LANE_DEFAULT_LEAD").is_none() {
      // Lanes, kibitzer, cron health, proposals and the actor read the lead here.
      env::set_var("OMON_LANE_DEFAULT_LEAD", key.storage_key());
  }
  ```
- Fix: Resolve the lead before the runtime starts and set it there, or pass the resolved lead explicitly instead of mutating the environment.

### Checked and not reported
- **Slack startup recovery:** `pending_recovery_actor` trusts `source='ingress'` rows. Slack ingress sets `author_id` from the Events API `user` (slack/ingress.rs:960). The actor indexes slack events (actor.rs:1356-1362). `SlackStartupAuthorization` mirrors the ignored/allowed channel rules.
- **Full-access default (`ApprovalPolicy::Never`):** it only affects `TerminalTool`/`CronTool` in the gateway `ToolRegistry`. I found no production path where the agent runs that registry (agent tools go through the omo app-server and the ledger in `omo_backend.rs`). Payments still ask (ledger.rs:318).
- **FDA alert:** it uses one marker keyed to the binary's identity and writes it only after a successful send. The Slack owner DM is chosen first.
- **Watcher stderr tail:** it is bounded (2 KiB) and handles UTF-8 boundaries. The reap timeout of 500ms cannot leak: the reader task ends at EOF once the child is killed.
- **Shutdown:** `flush_slack` runs on every shutdown arm.
- **Other dashboard changes:** `post_status` returns `Ok(None)` for web. The tracker router is nested before the fallback. `slack_config_view` exposes no tokens.
- **Uncommitted WIP:** removing the steer leaves no dangling variable, and `turn_steer_request` is still used at omo_backend.rs:2716. Slack egress now posts only the final reply. Partial chunks return early, a silent final posts nothing, and retries are deduplicated through `final_sent` and `completed`.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/main.rs - boot_slack makes one auth.test attempt (813) and calls set_var (837); the Discord retry is at 1086; the Slack-off wiring is at 1440 and 1671-1712
- /Users/indo/code/project/omon-gateway/src/slack/api.rs - send() retries only on 429 (405-411), so a transient boot failure is final
- /Users/indo/code/project/omon-gateway/src/dashboard.rs - resolve_approval maps "session" to Once (2458)
- /Users/indo/code/project/omon-gateway/src/discord/approval.rs - parse_custom_id maps "session" to Session (862) and resolve_proposal_id (784), for comparison
</files>
<answer>One P1: a transient Slack auth.test (or open_im) failure at boot disables Slack, or the Slack owner lead, for the whole run, with no retry. In a Slack-only setup it ends the process instead. Two P2s: the dashboard downgrades "session" to Once on proposal-id resolves, and set_var is called while the tokio runtime is running. Everything else in the diff and the WIP checked out.</answer>
<next_steps>Add bounded-backoff retries to boot_slack's auth_test and owner_dm_key, and change the "session" mapping in resolve_approval at dashboard.rs:2458.</next_steps>
</results>