fix: harden gateway across two full review cycles

Two whole-repository review cycles (src, web/src, migrations, scripts),
each fixed and re-reviewed until no confirmed P0/P1 remained.

Security
- Approval display redacts Authorization headers; hardline/dangerous
  detection covers path-qualified commands, sudo/env wrappers, escaped
  quotes around substitutions, kill -1 with redirects, and empty quote
  pairs; neutralizer strips invisible chars before sentinels.
- Cron script/ack_command go through deny globs and the approval gate,
  and the approval text names every stored shell field in full.
- Cron owner approval needs a one-time token that never reaches the
  agent (digest not mirrored, token redacted from stored output).
- Media uploads exclude secret files; dashboard blocks remote images.

Reliability
- Runtime ownership lock acquired in production; boot-time drain markers
  and cancel races no longer shut the gateway down.
- Replay dedup reruns failed turns; actor commands survive retirement;
  only busy sessions are marked resume_pending.
- Discord backfill cursor moves only past completed turns and snapshots
  cursors before replay; reactions target the thread first.
- Agent turns time out on inactivity; mid-turn status is delivered;
  cron briefings are consumed only after turn/start.
- Cron: monitor state committed after delivery, backoff retries run
  instead of being reported missed, stale remediations re-escalate,
  atomic dashboard job creation.
- Search documents follow session/message deletes (migration 0042).

Each behavior change has a regression test.
Gate: fmt, clippy -D warnings, cargo test (1536 passed), web test/tsc/build.
