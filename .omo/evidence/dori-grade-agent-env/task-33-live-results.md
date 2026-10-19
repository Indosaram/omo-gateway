# Live JevCli replay

Command: `zsh -c 'source ~/.zshenv; CARGO_TARGET_DIR=$PWD/target-w33 CARGO_BUILD_JOBS=1 CARGO_INCREMENTAL=0 cargo test --test test_judge test_live_12_samples_replay -- --ignored --nocapture'`.

Exit 0. One ignored live test explicitly executed and passed in 9.18s. All 12 expected results matched through JevCli, not merely direct CLI probes.

| Sample | Purpose | Result | Milliseconds |
| --- | --- | --- | --- |
| 1 | inbox_class | owner_must_act | 2021 |
| 2 | inbox_class | fyi | 652 |
| 3 | lane_size | large | 946 |
| 4 | lane_size | small | 627 |
| 5 | event_wake | wake | 675 |
| 6 | kibitzer_wake | wake | 666 |
| 7 | report_intent | finished | 684 |
| 8 | report_intent | progress | 571 |
| 9 | cron_cause | needs_owner_login_or_quota | 639 |
| 10 | owner_intent | wake | 548 |
| 11 | operational failure probability | 0.880 | 596 |
| 12 | routine success failure probability | 0.020 | 549 |

Outage routing probe: `cargo test --test test_judge test_dead_url_fallback_probe -- --ignored --nocapture` passed, exit 0, in 0.26s. It uses FakeJudgeFallback and proves one fallback invocation for the dead endpoint; live small-model inference is not established by this probe.

Full-suite final result remains pending.
