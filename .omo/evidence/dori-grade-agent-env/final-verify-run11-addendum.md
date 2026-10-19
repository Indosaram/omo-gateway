# Final verification run 11 addendum

- final-verify.json: fmt 0, clippy 0, build 0, source_cmp 0; 1144 tests passed; baseline comparison 1 because of one new failure:
  test_multiplexer::terminal_outcomes_release_typing (typing release not observed).
- Cause: the test waited with 100 bare yield_now() calls; host load was 18.5 during the run. It passed in run 8 and run 9 and 6/6 standalone.
  Not a product change: src/multiplexer is identical between run 9 and run 11.
- Fix (test only): the capturing dispatcher signals a tokio Notify on every dispatch and the test awaits that signal with a 5 s bound.
  clippy -D warnings clean; full test_multiplexer 19/19 passed 3 times in a row.
- The release binary of run 11 is unaffected (tests are not compiled into it): /Users/indo/code/project/omon-gateway-wt/dori-work/target-w4/release-bins/5d1e25a4af283dde87a525a75f635cefe8ee44e1be7ea31e5df5fec0838067b6/omo-gateway
  sha256 5d1e25a4af283dde87a525a75f635cefe8ee44e1be7ea31e5df5fec0838067b6
- All other failures equal the todo-2 baseline (cron_boundary_parity 1, cron_executor_parity 1, migrate 2, omo_backend 3, wiring_e2e 1).
