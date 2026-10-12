# Lane Operator Guide

This guide describes the lane CLI and runtime represented by the current source tree. Exact CLI help acceptance remains pending a freshly built or otherwise version-matched `omo-gateway` binary. Features explicitly called planned below are plan targets, not verified behavior in this source snapshot.

The legacy herdr pane manager is not used by lane execution.

## Runtime model

A lane is a gateway-managed work record with its own key, brief, working directory, completion signals and lead session. The lane runner uses `OmoBackend` to run turns through the OMO app-server. It stores the OMO thread identifier in session state so later turns can continue that thread. The runner creates or resumes a lane session; it does not use a terminal pane manager.

The gateway runs the lane service when `OMON_LANES_ENABLED` is true or unset. In Discord-free mode, `OMON_DISCORD_ENABLED=false` requires the lane service to be enabled. Start the gateway with the same `DATABASE_URL` used by lane CLI commands. A lane key is 1 to 48 lowercase letters, digits, or hyphens. A start request needs a readable brief file, an existing working directory, and either a lead session key, `OMON_LANE_DEFAULT_LEAD`, or a matching session found from the Discord thread/channel.

The runner appends lane instructions to the brief. Workers report activity using `[REPORT] <key> | <kind> | <text>`, where the source currently recognizes `milestone`, `blocker`, `question`, and `done`. Blocker and question reports pause the lane; the lead can reply with `send`. A done report is not itself a verified completion claim. The worker must invoke `claim-done` with evidence. The CLI has no dedicated `kill` subcommand; disable lane service with `OMON_LANES_ENABLED=false` and restart the gateway to stop lane execution.

## CLI commands

The commands and flags below mirror `src/lanes/cli.rs`. Run `omo-gateway lane --help` and `omo-gateway lane <command> --help` on the deployed binary before relying on a changed version.

`omo-gateway lane help` prints lane help; `omo-gateway lane help <command>` prints help for the given subcommand.

### Start a lane

```sh
omo-gateway lane start task-42 \
  --title "Fix retry handling" \
  --brief /path/to/brief.md \
  --done 'command ["cargo","test","--test","test_retry"] stdout~"test result: ok"' \
  --cwd /path/to/repository \
  --lead 'discord:server:channel:user'
```

The required arguments are positional `key`, `--title`, `--brief`, and `--done`. Optional arguments are `--thread <channel-id>`, `--cwd <path>`, `--lead <session-key>`, and `--model auto|small|large` (default `auto`). `auto` asks the configured judge to size the work and treats a judge error as large work. A matching session for `--thread` can supply the lead; otherwise set `--lead` or `OMON_LANE_DEFAULT_LEAD`.

When resource checks hold the launch, the lane is stored as `queued` and the CLI returns exit code 4 with reasons. A successful start returns JSON containing the lane and reasons. Invalid input or command errors return JSON with `ok: false` and exit code 2.

### List lanes

```sh
omo-gateway lane list
omo-gateway lane list --status working
```

`--status` accepts a lane status. Without it, the command lists all records.

### Read a lane transcript

```sh
omo-gateway lane log task-42
omo-gateway lane log task-42 --last 40
```

`--last` limits the returned transcript entries.

### Send guidance to a lane

```sh
omo-gateway lane send task-42 --text "Use the existing retry helper and report the test result."
```

Messages can be queued while a lane is `working`, `blocked`, or `not_done`. Sending to a blocked lane moves it back to working and wakes the runner.

### Claim completion or object

```sh
omo-gateway lane claim-done task-42 --evidence "The required test command passed."
omo-gateway lane object task-42 --reason "The regression test still fails on the timeout case."
```

The claim records `done_claimed` and evidence; the objection records `not_done` and its reason. Both wake the runner. The source's claim handler queues an instruction for the worker to stay idle; the objection handler tells it to continue. Do not treat the claim response as proof that signals have been rechecked. The plan specifies a five-minute objection window and live signal settlement, but a periodic settle implementation is not present in the inspected runner source. Do not promise automatic verified completion or closure based on this guide alone.

### Close or reopen a lane

```sh
omo-gateway lane close task-42
omo-gateway lane close task-42 --force --note "forced: owner cancelled the work"
omo-gateway lane reopen task-42 --text "Resume with the latest review feedback."
omo-gateway lane reopen task-42
```

Normal close follows the status transition rules and is permitted from `verified_done`. `--force` requires `--note` beginning with `forced:`. The current forced-close path updates lane status/history; do not assume it also archives a Discord thread or removes worktrees. The inspected source has no `close_failed` status. Reopen requires an eligible lane and resumes it as working; the optional `--text` is sent as its first prompt.

### Check launch capacity

```sh
omo-gateway lane can-launch
```

The response reports `can_launch` and hold reasons. Exit code 0 means no hold; exit code 4 means a hold. The current check considers free memory, free disk and active lane count. High load by itself is not a launch hold.

### Run a heavy command

```sh
omo-gateway lane heavy tests -- cargo test --test test_retry
```

The label is followed by `--` and the command as separate argv items. The command waits for a heavy slot and for load below the configured release threshold. The response includes the command exit code, stdout and stderr. The current default is one heavy slot.

## Completion signal grammar

Pass one or more semicolon-separated signals to `--done`. Signal kinds and options supported by the current parser are:

```text
merged owner/repo#123
closed owner/repo#456
published package@1.2.3
command ["program","arg"] [stdout~"regular expression"]
file path [sha256=<hex>] [json:.field="value"]
url https://example.test/health [status=200] [body~"regular expression"]
```

For example, multiple signals can be written as:

```sh
--done 'file out/result.json json:.status="ok"; command ["cargo","test","--test","test_retry"]'
```

Commands are JSON argv arrays, not shell command strings. The parser rejects empty signals and uncheckable trivial commands such as `true`. File paths are checked relative to the lane working directory unless absolute or home-relative. URL checks use HTTP GET; the default expected status is 200. Set every signal to concrete evidence that can be checked again, not a prose-only promise.

The source includes checkers for merged pull requests, closed issues, published packages, commands, files and URLs. Their runtime results depend on the required local tools, credentials and network access. A declaration or successful `claim-done` response is not a substitute for observing each check's result.

## Statuses and current lifecycle

The status values defined in source are:

| Status | Meaning in the current implementation |
| --- | --- |
| `queued` | Stored, but launch was held by the capacity check. |
| `working` | Runner may execute or continue turns. |
| `blocked` | Waiting for lead input after a blocker or question. |
| `done_claimed` | Worker claimed completion; verification and settlement must not be assumed complete. |
| `not_done` | A lead objection was recorded, or work is resuming after an objection. |
| `verified_done` | Verification succeeded according to a transition. Automatic settlement is not established in the inspected runner. |
| `closed` | Lane record has been closed. Thread archival and worktree cleanup are not established by the current close command. |
| `failed` | Runner recorded a terminal failure, such as repeated turn errors or a turn cap. |

The five-minute settle window, automatic queue retry, `GuardWatcher` service wiring, Discord thread status lifecycle and automatic close cleanup appear in the plan. They are not verified as running behavior in this source snapshot. In particular, `close_failed` is a planned troubleshooting case, not a current status value.

## Environment and guard settings

Defaults below are read from the inspected source. Invalid positive-integer lane limits or turn settings can make the lane service fail to start.

| Variable | Current source behavior |
| --- | --- |
| `OMON_LANES_ENABLED` | Enables lane runner; defaults to `true`. Set `false` to disable it. |
| `OMON_LANE_TURN_SECS` | Maximum duration for one lane turn; defaults to 3600 seconds. Must be positive. |
| `OMON_LANE_MAX_TURNS` | Maximum turns before a lane fails; runner default is 24. If set, must be a positive integer. |
| `OMON_LANE_DEFAULT_LEAD` | Lead session key fallback when no matching session is found and `--lead` is absent. |
| `OMON_LANE_KEYWORDS` | Replaces the runner's default lane prompt keywords, `ulw set goal and work`. |
| `OMON_LANES_MAX` | Maximum concurrent active lanes; defaults to 6. |
| `OMON_GUARD_LOAD_ALERT` | Guard alert load average; defaults to 150. |
| `OMON_GUARD_LOAD_OK` | Load threshold used by heavy command execution; defaults to 80. |
| `OMON_GUARD_MEM_FREE_MIN_PCT` | Minimum free memory percentage for launching; defaults to 20. |
| `OMON_GUARD_DISK_FREE_MIN_GB` | Minimum free disk space for launching; defaults to 50 GB. |
| `OMON_HEAVY_SLOTS` | Number of cross-process heavy slots; defaults to 1. |
| `OMON_KIBITZER_ENABLED` | Planned event-driven kibitzer switch from the plan. No current lane service consumer was found in the inspected source, so changing it is not verified to affect runtime. |

`DATABASE_URL` selects the gateway database. Lane CLI and daemon operations must target the same database. When a worker invokes a lane command, the runner supplies its executable and database path; preserve those values rather than pointing at a different gateway instance.

## Watching work

Use `lane list` to inspect recorded status and `lane log <key>` to inspect the persisted lane transcript. The lane service also records runner activity and errors in gateway logs. A `[REPORT]` milestone is activity, not evidence that a done signal passed.

The plan calls for a dashboard lane view, but no lane-specific dashboard route or view was found in the inspected source. Use the CLI and gateway logs until a matching binary and dashboard implementation are confirmed. This guide does not claim live dashboard status, event delivery to the lead, or Discord thread archival as verified behavior.

## Troubleshooting

### Daemon is down

Check the supervisor's gateway process and gateway logs first. Confirm `DATABASE_URL` points to the intended database and that the gateway can open it. For a Discord-free lane service, verify `OMON_DISCORD_ENABLED=false` and `OMON_LANES_ENABLED=true`; otherwise startup reports that Discord-free mode requires lane service. Restart the gateway through its existing service manager after correcting the underlying startup error.

### Lane turn errors

Inspect `omo-gateway lane log <key>` and the gateway log for the turn error. Check that the OMO app-server is available, the lane working directory and brief remain readable, and the configured turn duration is positive. The runner marks the lane failed after three consecutive turn errors; it also enforces its turn cap. Fix the cause before reopening or starting replacement work.

### Claim appears stuck

Inspect the lane status and transcript with `lane list` and `lane log`. The current source can record `done_claimed`, but the inspected runner does not implement the planned five-minute settle loop. Do not wait for automatic settlement as an established behavior. Read the declared signals and verify them manually; if work remains, use `lane object <key> --reason "..."` to return it to work.

### Close or cleanup failure

The current CLI has no `close_failed` state and does not establish automatic worktree cleanup or thread archival. If a normal close is rejected, check that the lane reached `verified_done` and review the state transition error. Forced close requires an explicit `forced:` note and should be reserved for an owner-directed override. Check worktree and thread cleanup separately; do not infer they occurred from `status: closed`.

### Launch is queued

Run `omo-gateway lane can-launch` against the same database and inspect its reasons. Review free memory, disk headroom and active lane count. Adjust guard thresholds only when the host's actual capacity supports the new values. The current CLI does not establish automatic queued-lane retries.

## Rollback

To stop lane execution while keeping the gateway available, set `OMON_LANES_ENABLED=false` in the gateway's service environment and restart it through its service manager. In Discord-free mode, this leaves no lane service and startup exits with an error, so use the gateway's regular Discord-enabled configuration or stop the whole service as appropriate.

The plan names `launchctl kickstart` as its rollback example. Use the installed gateway LaunchAgent label, not a guessed label:

```sh
launchctl kickstart -k "gui/$(id -u)/<installed-gateway-label>"
```

Set `OMON_LANES_ENABLED=false` in that service's environment before restarting it. Do not delete lane rows, briefs, threads or worktrees as part of a feature rollback. Re-enable lanes by restoring or removing the override and restarting the gateway. In Discord-free mode, disabling the only lane service makes gateway startup fail, so retain another configured gateway service or stop the service through its supervisor.
