<!-- task=st_01a14518 -->
st_01a14518 [completed] model mahoquot/claude-sonnet-5-5
Ask: read amplifying.ai's Dots study, five openai/codex issues and the hands-on comments on HN 49896604 for real behavior, failure modes, refusals and stalls. Wanted: a sourced report.
For you: all seven sources were read, but the HN thread only through keyword searches, so coverage is partial. Now: report complete. Next: none.

**Routes used**
- **amplifying.ai:** direct curl returned the HTML, truncated at 50KB. The full text came from `/research/openai-dots-choose.md`, which `llms.txt` advertises. No Cloudflare challenge appeared.
- **GitHub issues:** `gh issue view --comments`, all five fully read.
- **HN:** `hn.algolia.com/api/v1/items/49896604` came back truncated at 325KB. I then ran Algolia comment searches for tried, stuck, sandbox, approval, refused, auto-review and "I used".
- **Not used:** r.jina.ai returned 429 (rate limit). Wayback and old.reddit were not needed.

## Findings

- **Dots building apps, benchmark (SECONDARY, third-party, 8 runs).** All 8 builds used Cloudflare Workers, D1 and ChatGPT sign-in through "Sites". First delivery took 13 to 29 minutes. [https://amplifying.ai/research/openai-dots-choose] (curl .md export)
- **Scheduling stalls (SECONDARY).** Five of 8 runs configured a schedule, and all five used a separate ChatGPT task. Run 3 shipped with no schedule. Runs 5 and 7 left their collector connection pending or unconfigured. Only Run 4 showed new timestamps while the browser was closed. Runs 5 and 7 said so in the page UI. [https://amplifying.ai/research/openai-dots-choose] (curl .md)
- **Approval gates mid-build (SECONDARY).** Runs 1, 6 and 8 paused to ask for an app connection or for permission to set a private server secret before background collection could be authenticated. [same URL]
- **Overlap control and failure handling in generated code (SECONDARY).** Runs 1, 2 and 4 added a collection lock or lease, and runs 2 and 4 added a "due" timing gate. The fetch helper in Run 1 used a host allowlist, `redirect:"manual"`, an 18s timeout and an 8MB cap. Local checks found that failures retained old data and last-success times. [same URL]
- **Blocked sources.** Mistral returned 403 and 401 from the hosted runtime, Bedrock had no usable feed, and OpenRouter returned 503 (Run 8 kept stale data). Gaps were shown on the page rather than as "healthy". [same URL]
- **Study limitation (SECONDARY).** Dots' "tested X" steps are self-reported. The authors say "unverified" repeatedly. [same URL]
- **#51757 (PRIMARY, OPEN, Windows).** The error was `The invoking thread is not attached to an active Aeon`, with `UNKNOWN` as the error code.
  - It failed `list_environments`, `send_message`, `create`, `list_threads` and `get_orbit_profile`.
  - Reads of existing tasks still worked, and the already-running task kept pushing code.
  - Reboot (name, then Reboot) restored discovery and task creation.
  - Local exec then failed with `helper_unknown_error: setup refresh had errors`.
  - A rollback to 26.930.7945.0 fixed exec, but a later update broke it again.
  - The bot flagged duplicates #51742, #51743, #51746, #51730 and #51740. [https://github.com/openai/codex/issues/51757] (gh)
- **#50115 (PRIMARY).** The Dot's own cloud computer showed "Unavailable" while chat still worked. The UI gave no reason, retry or statement about whether state was preserved. A Windows commenter saw `failed to connect native pipe: Connection refused (os error 111)` and `Target.setDiscoverTargets: Target closed`. [https://github.com/openai/codex/issues/50115] (gh)
- **#50012 (PRIMARY).** The GUI terminal and the agent shell see different `/home/agent` contents on one instance, with the same hostname and UID.
  - The agent shell mounts `/home/agent` as a read-only tmpfs and the workspace as a writable overlay.
  - A CLI login completed in the GUI was invisible to the agent, which failed with `mkdir /home/agent/.doppler: read-only file system`.
  - A second user reproduced it.
  - Config state was also lost when the hostname later changed, though the workspace survived. [https://github.com/openai/codex/issues/50012] (gh)
- **#51213 (PRIMARY).** The Dot vanished from the desktop sidebar and the cloud-chat composer was disabled, while the Dot kept sending messages. Logs showed `Workspace routing is unavailable`, status 432, on `/accounts/check`, `/settings/user` and `/wham/usage`, plus 404 on `/tbo/primary`. One commenter saw `authorized=false` for a connected computer. Reboot plus app Repair restored the entry, but exec then failed again with the setup-refresh error. Sender identity also degraded: the label showed an old task title. [https://github.com/openai/codex/issues/51213] (gh)
- **#51540 (PRIMARY, refusal pattern).** The notice reads: "Your dot is on a break. Congrats you are in the very top users and have hit our abuse prevention limit. Check back in a bit!"
  - It gave no threshold, meter, reset time or task-state information.
  - Durations observed: about 1h30, about 2h10, and a report of "blocked for 3 hours".
  - A separate error, "You've hit the preview limit. Try again at Nov 13th, 2026 10:06 AM", hit delegated sub-runs about 30 days out with no timezone. The main chat stayed up while four sub-runs were blocked.
  - Work meter showed 31% used at the time of the first pause.
  - A user log of 26 `host.response_headers` events showed HTTP 200 and no 429, so the limit is not visible as a transport error.
  - Docs say Dot conversations do not count toward ChatGPT limits, while tasks started in Codex or Work do. [https://github.com/openai/codex/issues/51540] (gh)
- **HN, stalls (PRIMARY first-hand, single user).** joshstrange: the cloud machine died several times mid-task. There was no preview-link support, and even with "unrestricted" internet it could not reach a tunnel on a non-standard port. Via laptop it went silent all night, then showed a typing indicator with no message. [https://news.ycombinator.com/item?id=49920558] (Algolia)
- **HN, security design (SECONDARY, opinion).**
  - rkagerer notes that auto-review (a model gating actions) is the boundary and calls it unreliable. [https://news.ycombinator.com/item?id=49906176]
  - drejt proposes gating by action class: reads free, mutating writes ask, and a new outbound host or unused credential always blocks. [https://news.ycombinator.com/item?id=49904634]
  - eddythompson80 argues credentials should be injected outside the sandbox, with domain, URL and method allowlists. [https://news.ycombinator.com/item?id=49905655]
  - mmulqueen runs per-project VMs with tight egress and mounts. [https://news.ycombinator.com/item?id=49910556]
- **HN, always-on behavior.** mike_hearn runs two self-built Codex "employees" with Maildir wakeups, timers and per-agent UNIX accounts. Output waits for his approval. [https://news.ycombinator.com/item?id=49907695]
- **HN, cost (not Dots).** bilalq saw a $400 token burn from Claude Tag waking with empty prompt caches on an idle channel. [https://news.ycombinator.com/item?id=49899355]

## Design patterns worth copying (Rust gateway)

1. **Separate the agent from its environment.** A Dot's conversation outlives its compute. Model `Agent{id, state}` apart from `Env{id, kind, status}`. `EnvStatus` should be an explicit enum: `Starting | Ready | Unavailable{reason, retry_at, state_preserved: Option<bool>}`. Never show a bare "Unavailable" (#50115).
2. **One filesystem view per agent.** Run the GUI and agent shell in a single mount namespace, or document the split. If HOME is a read-only tmpfs, fail with a typed `HomeReadOnly` error that names the writable path (#50012).
3. **Typed binding errors with a reboot transition.** #51757 is a conversation-to-session association going stale. Keep `Binding{conv_id, session_id, epoch}`. Give a stale epoch an error variant `StaleBinding` with a built-in `rebind` transition (their "Reboot"), not an opaque `UNKNOWN`.
4. **Ledger-style quota, never silent.** Use a token bucket or ledger per scope: `chat | delegated_run | background`. A refusal returns `Refused{scope, limit, used, reset_at: DateTime<Utc>, tz, affects: [TaskId]}`. Warn at 80%. Add a `Paused` task state that survives the pause and resumes, with a handover summary of completed and pending steps (#51540).
5. **Task state machine.** `Queued → Running → AwaitingApproval → Running → Done | Failed | Paused(reason)`. Persist heartbeats. Surface "Running with no output for N min" as `Stalled`, then retry or notify (HN stall).
6. **Single-flight job lease.** The pattern in their runs 1, 2 and 4: a `leases(name, holder, expires_at)` row plus a "due" check, with compare-and-set acquire. Run scheduling in-process with a timer rather than an external pinger that needs a connected-app step. Their reliance on a separate task was the main cause of silent no-schedule deliveries (runs 3, 5 and 7).
7. **Stale-not-healthy data model.** `Freshness{last_success, last_attempt, last_error}`. Unknown or stale never renders as OK. Keep old data on failure. Show coverage gaps.
8. **Outbound policy.** Allowlist by host, no redirects (`redirect::Policy::none`), timeout, and a body-size cap, as in the Run 1 `sourceText` code. A new host or credential always requires approval.
9. **Approval by action class.**
   - Read: allow.
   - Mutate or shell: ask.
   - New egress host or first-use secret: always ask, and expire the approval.
   - Do not rely on a model judge as the sole gate. Use it as a secondary signal on top of deterministic rules.
10. **Secrets injected at the egress proxy.** The agent never sees tokens. The proxy attaches them per host, method and path.
11. **Proactive background work is read-only.** Dots restrict background "proactive research" to read-only tools. Tag every task `origin: background | user` and enforce capability by origin.
12. **Idle budget.** Cap tokens per idle wake and per channel per day to avoid cold-cache burn.
13. **Sender identity.** Include a stable `agent_name` on every outbound message, not the last task title (#51213).

## CLAIMS

| CLAIM | RISK | SOURCES | PRIMARY |
|---|---|---|---|
| Dots hit opaque "abuse prevention" pauses of 1.5 to 3h or more with no meter or reset | High: unpredictable availability | #51540 | Yes (user reports, no OpenAI staff reply) |
| A "preview limit" blocks sub-runs until about 30 days out while chat continues | High | #51540 comment | Yes (one user) |
| Active-Aeon binding errors break environment discovery and task creation, but reads still work | High | #51757, 5 listed duplicates | Yes |
| After reboot or rollback, local exec still fails with `setup refresh had errors` | Medium | #51757, #51213 | Yes |
| The GUI and agent shell have different HOME views | Medium | #50012 | Yes, 2 users |
| The cloud computer goes "Unavailable" with no recovery action | High | #50115 | Yes, 2 users |
| The Dot disappears from the UI on routing error 432 while it keeps running | Medium | #51213 | Yes |
| Dots default to Cloudflare Workers, D1 and ChatGPT sign-in in 8/8 builds | Low | amplifying.ai | No (third party) |
| Scheduled background scraping was verified in only 1 of 8 runs | Medium | amplifying.ai | No |
| HN: a Dot stalled silently overnight and could not reach a non-standard-port tunnel | Medium | HN 49920558 | Yes, but n=1 |
| Auto-review is the security boundary and is unreliable | Medium | HN 49906176 | No (opinion) |

## EXPAND

- Read the remaining HN subthreads by walking the item tree. Only keyword searches were run, so hands-on reports using other words are missing.
- Read the follow-ups cited in #51757 and #51213: #51730, #51740, #51742, #51778 and PR #51822. They hold more evidence on the exec setup-refresh failure and the sandbox file-lock bug.
- Read learn.chatgpt.com docs pages: `dots/computers-and-apps`, `sandboxing/auto-review`, and the OpenAI "how we build safety, security and privacy into Dots" article.
- Check the evidence repo linked from #51540: github.com/COOLak/chatgpt-dot-abuse-prevention-limit.