# Todo 36 - deploy and production e2e (2026-10-12)

Commit 1bbe7e2 (dori-work) + working-tree fixes found during deploy: src/judge/mod.rs (typesafe-judge under launchd PATH),
src/lanes/posts.rs (skip closed lanes; one lane's Discord failure no longer fails the lane service). These two are not yet committed.

| Check | Result | Evidence |
|---|---|---|
| Release build | cargo +1.98.1 build --release (stable 1.92 emits a proc-macro dylib dyld rejects) | release-build-198.log |
| Developer ID | TeamIdentifier=5DUM8WPB4C, sha256 49c75b55... | codesign-release.txt |
| Block markers in bot 1465631383862120451 prompt | begin=1 end=1 | apply-block.txt |
| Migrations | 31-33 checksums equal, 34-41 applied | ledger deploy-prep |
| e2e-1 | working -> done_claimed -> verified_done -> closed; done_claimed event delivered_at set | lane-history.json |
| e2e-3 (failure scenario) | false claim "proof.txt exists" -> not_done "file proof.txt: failed to read ... No such file" -> working; then forced close | lane-history.json |
| Discord | thread create x3, thread updates (archive/unarchive) | gateway-log-excerpt.txt |
| CLI on release binary | inbox list, sessions list, cron health, usage exit 0 | cli-*.json |
| jev in production | inbox_class calls 0.5-0.9 s, used_fallback=0 after PATH fix | judge-calls-release.json |
| herdr | pgrep -f "herdr server" exit 1 | ledger |
| Launchd env TYPESAFE_* | not in plist; loaded by code from ~/.zshenv (zero-config per owner rule) | src/judge/mod.rs |
| Owner-DM model | gateway default (plist OMON_DEFAULT_MODEL=mahoquot/nekos-claude-sonnet-5-5) overrides code default mahoquot/claude-opus-5-5; not changed (owner config) | gateway.log startup line |

## Incidents during deploy (own changes, all resolved in production)
1. First install was the final-verify DEBUG binary (113 MB); replaced by the release build.
2. Restart loop (~100 restarts, ~07:25-07:44Z): status refresh edited archived threads of closed QA lanes -> Discord 50083 -> lane service exit.
   Mitigated by clearing thread links of the QA rows; fixed in code (posts.rs) and deployed.
3. e2e-3 forced close hung in phase 0 (thread archive) and blocked the lane loop; QA row closed manually, gateway restarted, clean.

## Follow-ups (not fixed; need a code change + one build)
- Forced close of a lane whose turn was just cancelled hangs in thread_effect (phase 0) and stalls the lane service loop.
- Inbox triage collects OmOMeow's own "Running tool ..." Discord progress messages and jev classifies them owner_must_act; filter the bot's own messages at the Discord source.
- e2e lanes ran on the plist model; the plan's owner-DM opus check depends on the owner's plist choice.
