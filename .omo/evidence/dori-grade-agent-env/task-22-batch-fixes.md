# Shared fixes after batch 20261011T102415Z

Status: SOURCE-SETTLED; compilation and runtime verification pending the next coordinated batch.

- Fixed the steered Discord callback to use `crate::inbox::watchers` after the metadata index commit.
- Supplied the optional inbox sender (`None`) to all 12 existing actor test loader calls without changing their assertions.
- Removed the unused watcher import and unnecessary receiver mutability.
- Cloned the pool and producer sender before the static startup closure. Queued one Startup event before exposing the sender; the same consumer collects all sources first and then handles queued source-specific events serially. Removed concurrent initial collection and the extra initial Kakao pass.
- Kept progress logging in the owned JoinSet. Shutdown observes the current watch value, joins children, and retains the five-second outer abort fallback with kill_on_drop children. Producer sends can be cancelled by shutdown.
- Verified `zele mail watch --help` for zele 0.8.0: `--interval [interval]` is supported (default 15). Production mail watch now passes `--interval 60`.
- Repaired inbox tests: imported watchers, used existing Config error, borrowed the formatted snippet, separated ordinary GitHub notification from review escalation, and asserted empty/unadvanced cursor plus recorded error and no items for invalid source data. No tests removed or skipped.
- Applied formatting only to owned main/entry/actor/router/watchers/adapter/inbox-store/profile-import files using rustfmt stdout and apply_patch. Rustfmt parsed those inputs; LSP diagnostics timed out. No Cargo or runtime checks during repairs.

Other owners' sources/**, cron, lanes, omo_backend, and test_lane_events paths were not edited in this repair unit. The known backend.rs:61 formatting baseline remains untouched. Earlier batch logs are preserved. No services, environment files, external messages, live DB writes, or commits.
