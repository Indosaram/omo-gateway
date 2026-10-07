# DoneClaim: Discord button parity

## Changed files
- `src/discord/buttons.rs`

## Changes
- Added a 500-entry LRU of successfully acknowledged Discord interaction IDs; duplicates are ignored before action/config processing.
- On acknowledgement failure, post the requested channel reply, release in-flight state, and do not execute the action.
- Added per-namespace `nonterminal_actions` config support; successful actions get the done label while the clicked button remains enabled when configured.

## Verification
- `cargo test --lib discord::buttons::tests -- --nocapture` — exit 0; 12 passed, 0 failed, 0 ignored.
- `cargo check` — exit 0.
- `cargo clippy --all-targets --all-features -- -D warnings` — fails on existing out-of-scope `match_like_matches_macro` in `src/agent/omo_backend.rs:961`.
- `cargo fmt --all -- --check` — fails on existing out-of-scope formatting diff in `src/agent/omo_backend.rs`; `rustfmt --edition 2024 --check src/discord/buttons.rs` passes.
- `cargo test --test test_discord_adapter test_approval_buttons_and_parse_custom_id` — failed to compile because of unrelated concurrent sibling changes; orchestrator confirms the button unit suite is the accepted target.
- Parsed-config manual QA: `config-manual-qa.txt`; fixture JSON deserializes with `nonterminal_actions: ["preview"]` and the unit assertion passes.

## ULTRAQA
- `misleading_success_output`: checked; test output confirms 12 named button tests ran and passed.
- `flaky tests`: checked; config is written to a `tempfile` fixture, no real `~/.omon/button-actions.json` access.
- `hung commands`: checked; cargo completed, no command remains running.

## Cleanup receipt
- No changes made to the real `~/.omon/button-actions.json`.
- No temporary config fixture remains; tempfile fixture is removed by RAII.
- Unrelated sibling-worker changes in the worktree were not staged.

## Risks
- No HTTP-level test induces a Discord acknowledgement transport failure; the test covers the no-execution state contract while the handler path posts the reply and exits before spawning work.
- Workspace fmt/clippy/integration gates remain blocked by unrelated concurrent changes outside this task's scope.
