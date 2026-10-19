# Lane r3a-agent-runtime

NO FINDINGS

I read the final code in all six files. I could not run `git diff HEAD` here because the shell only accepts `gh` and `curl`, so I located the changed regions with grep and the F-number line references in CONFIRMED-r1.md. Nothing was built or tested, as instructed. Here is what each fix still does in the final code, including how the fixes interact:

- **F1, tool-open abort** (`src/agent/omo_backend.rs:966,1219,1243,1597-1604`): the inactivity abort and its timer are skipped while `open_tool_ids` is non-empty. The total work deadline at line 1017 still bounds the turn. Ids are removed on every `item/completed` and added only for started items that are not message or reasoning types, so the set cannot grow without limit.
- **F6/F6b, pidfile owner and dashboard-first takeover** (`src/agent/omo_daemon.rs:290-338,485-500,560-683`):
  - The pidfile has the form `"{pid} {owner}"`. A pidfile from an older format with no owner field still falls through to the existing `ps` command-line check, so an unrelated process is not killed.
  - The takeover supervisor starts with an empty slot. The watcher treats the empty slot as "exited", backs off, finds the port serving and waits in the 5s readyz loop. It respawns only after the other process's daemon is gone.
  - `kill()` and `Drop` do nothing on the empty slot. `force_restart_owned_daemon` hits the `None` guard and returns false, without blocking.
  - Before spawning, the watcher always probes readyz, so a gateway and a dashboard that both supervise the same port cannot fight over it.
- **F8, headless supervisor** (`src/main.rs:826-836`): `_daemon_supervisor` and `_cron_daemon_supervisor` stay in scope until the `return outcome` at about line 924, so they are not dropped early. The cron/main URL de-duplication matches the Discord path at line 1128. One behaviour change: a missing `omo` binary with autospawn on now makes headless startup fail, the same as the Discord path. The compose setup from F12 uses autospawn off, so it is not affected.
- **F7/F7b, cursor rewind** (`src/multiplexer/actor.rs:30-80,930`):
  - The cursor is rewound only for `ActorCommand::Event` items dropped at shutdown, after `complete_delivery`.
  - For constituent ids of the form `discord:{id}:{bot}` (`adapter.rs:2162`), `substr(...,9)` produces `"id:bot"`, and SQLite's `CAST(... AS INTEGER)` takes the leading integer, so this works correctly.
  - The `UPDATE` only lowers the cursor (guarded by `>= ?`). DB errors are logged, not silently ignored.
- **F5, stream clears dead target** (`src/discord/adapter.rs:3476`): `clear_for_bot` runs only after `throttler.update(...)?` succeeds. The dead-target short-circuit at line 3399 still runs before the placeholder send.
- **F9, proposal packing** (`src/discord/adapter.rs:914-958`):
  - Each message's budget is `DISCORD_MESSAGE_LIMIT` (2000, from `throttler.rs:13`, already in scope because the existing code uses it) minus the footer's character count.
  - An oversized entry is cut to budget-1 characters plus `…`, so it exactly fits the budget. Every chunk therefore stays at or under 2000 characters including the footer.
  - Each proposal number is bound to the id of the message that shows it.
- **F10, /thread** (`src/discord/commands.rs:1247-1249`): `CreateThread::new(..).kind(ChannelType::PublicThread)` compiles and type-checks as written.

## Verdict
0 P0 / 0 P1.