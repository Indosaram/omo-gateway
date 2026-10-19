<!-- task=st_01a1450e -->
st_01a1450e [completed] model mahoquot/claude-opus-5-5
I mapped the cron subsystem across all nine files in `src/cron/` plus the related migrations, the agent tool, and the `main.rs` wiring. Nothing was built or run.

<results>
<files>
- /Users/indo/code/project/omon-gateway/src/cron/scheduler.rs - poll loop, lease and claim logic, catch-up, backoff, delivery fan-out, schedule parsing
- /Users/indo/code/project/omon-gateway/src/cron/executor.rs - runs agent jobs: script first, optional skip, change monitor, prompt assembly, model run, process-tree kill
- /Users/indo/code/project/omon-gateway/src/cron/store.rs - `HermesJob` model, sync from the external Hermes job store, authority flag, notepads, run pruning
- /Users/indo/code/project/omon-gateway/src/cron/health.rs - sorts finished runs into problems, records missed runs, sends `cron_problem` events
- /Users/indo/code/project/omon-gateway/src/cron/remediate.rs - LLM diagnosis, fix lanes, verified reruns, owner approval with a one-time token
- /Users/indo/code/project/omon-gateway/src/cron/report.rs - `cron health|run-now|approve-rerun` commands, local Unix socket server, digest section
- /Users/indo/code/project/omon-gateway/src/cron/ack.rs - runs `ack_command` after delivery, with a time limit
- /Users/indo/code/project/omon-gateway/src/cron/guard.rs - blocks jobs that would restart or kill the gateway
- /Users/indo/code/project/omon-gateway/src/tools/cron.rs - agent-facing cron tool (add/update/list/pause/resume/trigger/runs/notepad)
- /Users/indo/code/project/omon-gateway/src/main.rs - wiring (lines 955-990 headless; 1251-1264 Discord with Hermes sync)
- /Users/indo/code/project/omon-gateway/migrations/{0001,0004,0013,0021,0022,0025,0026,0027,0035,0041}*.sql - cron tables
</files>

<answer>
## Capabilities

- **Polling scheduler on SQLite.** It checks every 1s and wakes early when a run finishes (`scheduler.rs:738-768`, default interval at `:620`). Each loop first syncs the Hermes store (`:756`) and then runs `run_due_jobs` (`:1236-1265`). If one job fails to claim, the other due jobs still run (`:1253-1262`).
- **Three schedule formats.** It accepts 5- or 6-field cron with Unix weekday numbering (`:2794-2847`), `interval:/@every/every N{ms,s,m,h,d}` (`:2849-2882`), and one-shot `once:<rfc3339>` with a 120s grace window (`:2635-2669`). Timezone-aware next-run times use IANA names via `chrono_tz` (`:2672`).
- **Leases so a job never runs twice at once.** Claims insert a `cron_runs` row with a 30-minute lease (`:23`), refreshed every 60s (`:1574-1590`, `:1638`). The insert refuses to start a second running row for the same job (`:1494-1507`). An expired lease is reclaimed only if the owning process is dead, or after a 120-minute safety net (`:27-61`, `:1313-1346`). A per-process set of active run ids handles reused process ids (`:63-85`).
- **Catch-up after downtime.** At most one missed occurrence is run, within `catch_up_hours` (default 6) (`:1386-1478`). Occurrences outside the window are recorded as "missed" problems (`:1424-1431`, `health.rs:210-239`). The same check runs at startup (`scheduler.rs:770-819`).
- **Failure handling.**
  - Backoff is exponential from 10s to 3600s (`:2884-2895`) and never earlier than the next scheduled time (`:2907-2922`).
  - A `retry_at` marker keeps a retry from being treated as a missed run (`:1969-1976`, `:2897`).
  - The repeat limit (`times`/`completed`) disables the job (`:275-321`, `:1365-1377`).
  - A job whose schedule can no longer be computed is disabled (`:1270-1292`).
  - Completion only writes if the job row is unchanged; if it was edited mid-run, the result is merged into the current row (`:2084`).
- **Job types** (`executor.rs`):
  - native payload: a script, then a prompt or `brief_file` (`:441-643`);
  - Hermes job: `script`, `no_agent`, and skip-the-agent when the script prints `wakeAgent:false` or nothing (`:53-95`, `scheduler.rs:199-231`);
  - change monitor via `monitor_script` or `monitor_url`, which skips the agent when the SHA-256 hash is unchanged (`executor.rs:96-178`);
  - plain shell jobs (`scheduler.rs:491-569`).
- **Prompt assembly.** The prompt is built from the cron hint plus workspace instructions (`executor.rs:198-210`), the latest output of earlier jobs listed in `context_from`, capped at 8000 chars (`:212-226`, `scheduler.rs:135-197`), notepads (`executor.rs:228-238`), skills and skill bundles (`:240-252`, `:668-866`), and the script output. The assembled prompt is scanned for prompt injection (`:259-265`).
- **Per-job overrides.** A job can set model, provider and base_url (`:285-303`).
- **Process safety.**
  - Scripts get a timeout, run in their own process group, and the whole process tree is killed on timeout (`:465-535`, `:1144-1260`).
  - The lifecycle guard rejects restart/kill of the gateway in prompts, scripts and ack commands (`guard.rs:5-37`, `store.rs:395-451`).
  - Working directories are limited to authorized roots (`executor.rs:1081-1117`).
- **Delivery.**
  - Fans out to several Discord channels or threads (`scheduler.rs:2535-2629`, `store.rs:622-807`).
  - Every send is written to the delivery ledger as an obligation (`scheduler.rs:2412-2434`).
  - The delivered message is copied into the matching session transcript, with owner tokens redacted (`:2440-2459`, `:2462-2533`).
  - Empty output or `[SILENT]`/`NO_REPLY` suppresses delivery (`:233-273`).
  - `ack_command` runs only after a successful delivery (`:2255-2265`, `ack.rs:66-89`).
  - Failures go to `failure_deliver` channels, and repeat alerts for an incident the owner acknowledged are suppressed (`scheduler.rs:2280-2347`, `:644-673`).
- **Health and self-repair.**
  - `on_run_finished` sorts each run into stuck, delivery_failed, failed, empty_success (output didn't match the expected regex) or missed (`health.rs:50-179`).
  - Problems are de-duplicated with an error signature that strips timestamps, pids and ids (`:19-48`).
  - Each new problem sends a `cron_problem` event to the lead session (`:150-160`).
  - `Remediator` asks an LLM judge to pick one of three causes (`remediate.rs:299-302`). It then reruns and verifies once (up to `MAX_RERUNS=3`), opens a fix lane for script bugs, or escalates to the owner with an inbox item and a SHA-256-hashed one-time token (`:351-395`, `:503-529`).
  - Jobs with external effects (`external_effect` or the hardcoded `PUBLISHING_JOBS`) always need owner approval (`:17-21`, `:214-217`).
  - Problems stuck in `remediating` for an hour are swept and escalated (`:398-423`).
- **Operator interface.** `omo-gateway cron health|run-now|approve-rerun` (`report.rs:28-49`), health report (`:74-128`), digest section (`:207`), and a 0600 Unix socket at `~/.omon/cron-run-now.sock` (`:241-395`). The agent tool covers list/add/update/pause/resume/trigger/runs/notepad (`tools/cron.rs:331-962`).

## Data model

| Table | Key fields | Where |
|---|---|---|
| `cron_jobs` | id, session_key, expression, payload_json (holds most job state), enabled, next_run_at, authority (`hermes_mirror`/`cutover_pending`/`omon_owned`) | 0001:39, 0022 |
| `cron_runs` | run_id, job_id, claim_token, lease_expires_at, status, attempt, error, owner_pid | 0004, 0013 |
| `cron_outputs` | (profile, job_id, run_id) → output | 0021 |
| `cron_monitor_states` | job_id, last_hash, last_snapshot | 0025 |
| `cron_incidents` | job_id, error_signature, acknowledged | 0026 |
| `cron_notepads` | profile, job_id, key, value (16 KiB per value, 64 KiB per job) | 0027, `store.rs:1139` |
| `cron_problems` | kind, signature, status (open/remediating/fixed/needs_owner/ignored), lane_key, detail JSON | 0035 |
| `cron_remediation_outcomes`; `inbox_items` (source `cron`), `inbox_decisions` | owner escalation | 0041, `remediate.rs:376-379` |
| `cron_cutover_receipts*` | while one is pending, all scheduling pauses | 0023, `scheduler.rs:602` |

Rust types: `CronJob` (`scheduler.rs:345`), `CronJobSpec` (`:323`), `CronRunOutcome` (`:449`), `CronTaskExecutor` trait (`:466`), `HermesJob`/`HermesOrigin`/`HermesSchedule` (`store.rs:284/213/145`), `Problem`/`Detail`/`RunReceipt` (`remediate.rs:23/33/81`).

## Limits and gaps for an always-on autonomous agent platform

1. **Delivery only reaches Discord.** `deliver` parses `chat_id` as a Discord u64 (`scheduler.rs:2357-2362`). Other platform prefixes are skipped (`:2611-2613`). The executor's session is `discord` or `local` (`executor.rs:267-287`). Extension point: replace `deliver` with an `OutboundDispatcher` keyed on `destination.platform`, and extend `delivery_destinations`.
2. **The headless runtime has no dispatcher.** `main.rs:955` uses `CronScheduler::new`, so `deliver` only records ledger obligations (`scheduler.rs:2414-2425`). Extension point: `main.rs:955`.
3. **Triggers are time-only.** Jobs start on a time poll (`scheduler.rs:1238-1244`). Change monitors poll too (`executor.rs:96-178`). There is no webhook, email or event trigger. Extension point: an event table that `run_due_jobs`/`claim_job(require_due=false)` (`:1295`) can consume, or the `wake` Notify (`:586`).
4. **Single-node, local execution.** Scripts run with `sh -c` on the host (`executor.rs:470-485`). Reclaim relies on PID liveness (`scheduler.rs:41-61`), and the store is SQLite. There is no isolated per-user "cloud computer". Extension point: an executor that runs in a VM or container behind the `CronTaskExecutor` trait (`:466`).
5. **Approvals are coarse.** Approval exists only for remediation reruns and publishing jobs (`remediate.rs:214-217`, `:446-451`), and the publishing list is hardcoded (`:17-21`). There is no per-action approval gate inside an agent turn. Extension point: `publishing()` and `needs_owner` (`:351`) as a general approval queue.
6. **Memory is limited.** It is notepads (64 KiB per job) plus the previous output of jobs named in `context_from` (`executor.rs:212-238`). There is no link to the `memories` table (0001:53). Extension point: prompt assembly at `executor.rs:198-258`.
7. **No proactive planning.** Jobs are created by the user or agent through `tools/cron.rs:626`. No component decides on its own to create jobs. Self-repair stops at a three-way classifier (`remediate.rs:299-302`).
8. **Job fields are loosely typed.** Most job state lives in untyped `payload_json` keys (`last_status`, `retry_at`, `repeat`) that are rewritten on every run (`scheduler.rs:1701-1722`). There are no per-job concurrency, cost or token budgets. The lease is a fixed 30 minutes.
9. **Hardcoded operator details.** KST is hardcoded in health output (`report.rs:111-114`). The quota check depends on `mahoquot 127.0.0.1:18801` (`remediate.rs:313-318`).
10. **Failure classification is fragile.** A failure counts as a delivery error only if the message contains "deliver" or "discord" (`scheduler.rs:1912-1918`).
</answer>

<next_steps>
Expansion leads:
- `src/tools/cron.rs:626-787` defines the job-creation interface the agent can use.
- The Discord-only parsing in `src/cron/store.rs:622-807` (`discord_destinations`) has to change for multi-channel delivery.
- `src/lanes/dispatch.rs:55` and `src/lanes/settle.rs:102` show how remediation connects to the lanes system.
- `src/migrate/cron_cutover.rs` explains how authority moves from the Hermes mirror to omon ownership.
- `crate::DeliveryLedgerService` and `crate::security::scan_assembled_cron_prompt` are outside cron but cron depends on them.
</next_steps>
</results>