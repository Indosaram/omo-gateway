# Task14 strict-clippy wrapper repair

Source settled for parent release. Product edit is limited to `src/multiplexer/actor.rs`: gate the existing `SessionActor::run` convenience wrapper with `#[cfg(test)]`. Production uses `run_with_shutdown`; eight legitimate unit-test callers still use `run`. Keeping the test-only wrapper avoids changing their existing result/termination behavior. No dead-code allowance, lifecycle redesign, or other product edits.

LSP references at the run and run_with_shutdown declarations returned no references. A read-only source call-site search found eight actor.run() callers in actor.rs unit tests and the production router.run_with_shutdown call in router.rs. The wrapper is therefore test-only, not removable without updating legitimate tests.

Actor-only rustfmt `--edition 2021 --check --config skip_children=true` exited 0. Final actor LSP returned no diagnostics. Final SHA-256:

```text
4f980c4856826f7aa0e7f5811577ab4cb19766ab0790b2a199092207fa471be6  src/multiplexer/actor.rs
```

User-reported combined173003Z results: headless4, dispatch10, inbox19 with4 ignored, events25 with1 ignored, post2, thread1 passed. Its strict-clippy exit101 reported only the unused SessionActor::run wrapper. These are prior parent-batch results, not a rerun after this edit. No tests, build, or clippy were run for this repair. Stable196 manifest966ee6e8... and previous batch artifacts were not edited. Parent owns the next released verification.
