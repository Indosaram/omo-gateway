# B12-1 corrected source receipt

Read the full authoritative main task12 pre-review. Its single blocker is fixed
in src/lanes/cli.rs; tests/test_lane_cli.rs gains behavioral coverage. No state
machine broadening, todo13 settle work, runner, entry, Cargo or main edits.

Object of a blocked lane selects the existing blocked -> working transition.
The transaction acquires the SQLite writer, records the latest reopened-history
generation, conditionally updates that generation/status, clears the idle latch,
records objection/history, and queues the exact supplied owner text. It commits
before the existing CLI wake call. No redundant lead wake event is inserted for
an answer to an existing question. The CLI JSON reports the resulting status.

DoneClaimed object retains NotDone, objection event and existing lead guidance.
Other states still fail through the unchanged allowed-transition table and exit
2 without queued input. The new tests cover every other registry status, the
claimed path, and a captured question -> actual CLI object -> runner next prompt
equal to owner text -> claim. The runner wait is bounded; no sleeps are used.

Source-settled only: current-tree compilation, tests and manual QA remain UNRUN
for the coordinated batch. Rustfmt parsing/output exited 0 for both changed
files. LSP timed out. No live DB, Discord, service, configuration, schema or git
commit operations occurred. No process remains running.

The audit's other capture/transport observations were read precisely: its
receipt/uncertainty assessment and cumulative-stream findings are nonblockers;
no indefinite exactly-once guarantee or bonus transport work was introduced.
