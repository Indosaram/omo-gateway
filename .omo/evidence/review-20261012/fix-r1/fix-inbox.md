# fix-inbox (review-20261012 / fix-r1)

Files edited: `src/inbox/watchers.rs` and `src/inbox/sources/mod.rs` only. No caller files were touched. On save, the editor hook ran rustfmt on `src/inbox/sources/mod.rs` and changed only test-module formatting.

## Defect 1 [P1]: `on_gateway_message` and the progress send block the caller

- Status: **FIXED**
- Changes in `src/inbox/watchers.rs`:
  - `on_gateway_message`: replaced `sender.send(..).await` with `try_send`. `Full` now returns `Ok(())`: the channel already holds pending work and `GatewayMessage` is idempotent. `Closed(e)` is mapped to `SendError(e)`. The signature and return type are unchanged, so callers in `multiplexer/actor.rs` and `discord/adapter.rs` compile without edits.
  - `run_event_watch_with_shutdown`: the progress send `tx.send(results).await` became `tx.try_send(results)`. When the consumer lags, the result is dropped and the event drain keeps going.
- Tests are in the new `#[cfg(test)] mod nonblocking_tests` in `watchers.rs`:
  - `on_gateway_message_returns_promptly_when_channel_full`: fills a capacity-1 channel with no receiver activity, then calls the function under a 5s `tokio::time::timeout` and expects `Ok(Ok(()))`.
  - `event_drain_does_not_block_on_full_progress_channel`: pre-fills a capacity-1 progress channel, queues 2 events, drops the event sender, and requires `run_event_watch` to return within 5s. It uses a lazy in-memory pool and no sources, so `collect_all` does no I/O.
  - Neither test sleeps. Each timeout is only a bound on a hang: when the code is fixed, the test finishes immediately.

## Defect 2 [P1]: `run_json` does not kill hung children on timeout

- Status: **FIXED**
- Change in `src/inbox/sources/mod.rs`: `run_json` adds `.kill_on_drop(true)` to the `Command`. When `collect_all`'s 60s `tokio::time::timeout` drops the future, the zele/imsg child is now killed.
- Test: `dropped_run_json_future_kills_hung_child`, in the new `#[cfg(test)] mod run_json_tests` in `sources/mod.rs`. It has to live there because `run_json` is `pub(super)`.
  - The test runs `sh -c 'echo $$ > <fifo>; exec sleep 300'` through `run_json`. A blocking FIFO read gets the child's pid, so there is no sleep and no race.
  - It then drops the future, which is exactly what a timeout does, and calls `waitpid(pid)` with a 10s bound. The child is our direct child, so `waitpid` is valid.
  - Before the fix, the child stays alive and `waitpid` never returns, so the bound fails. After the fix, the child is killed and reaped immediately.
  - If the assertion fails, the test SIGKILLs the child so no orphan is left behind.

## RED (unfixed code, tests only)

Command: `cargo test --lib inbox::`

```
test inbox::watchers::nonblocking_tests::on_gateway_message_returns_promptly_when_channel_full ... FAILED
test inbox::watchers::nonblocking_tests::event_drain_does_not_block_on_full_progress_channel ... FAILED
test inbox::sources::run_json_tests::dropped_run_json_future_kills_hung_child ... FAILED
thread '...on_gateway_message_returns_promptly_when_channel_full' panicked at src/inbox/watchers.rs:245:9:
thread '...event_drain_does_not_block_on_full_progress_channel' panicked at src/inbox/watchers.rs:269:9:
thread '...dropped_run_json_future_kills_hung_child' panicked at src/inbox/sources/mod.rs:128:9:
test result: FAILED. 0 passed; 3 failed; 0 ignored; 0 measured; 530 filtered out; finished in 10.02s
```

## GREEN (after fixes)

Command: `cargo test --lib inbox::`

```
test inbox::watchers::nonblocking_tests::on_gateway_message_returns_promptly_when_channel_full ... ok
test inbox::watchers::nonblocking_tests::event_drain_does_not_block_on_full_progress_channel ... ok
test inbox::sources::run_json_tests::dropped_run_json_future_kills_hung_child ... ok
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 531 filtered out; finished in 0.01s
```

Note: the first GREEN attempts failed to compile because other workers had edits in progress in their own files (`src/discord/adapter.rs` E0308, `src/migrate/config_import.rs` E0425/E0308). Neither error was in an inbox file. Once those files settled, the same command was re-run in the foreground and passed. Not run (left to the lead): fmt/clippy across the whole repo and the full test suite.
