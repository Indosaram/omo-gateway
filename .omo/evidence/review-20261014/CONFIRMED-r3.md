# CONFIRMED-r3: final delta re-review of the combined fix diff F1–F16 (review-20261014)

Workers: explore + mahoquot/claude-opus-5-5, read-only, no cargo. Reports are in r3/.

| Lane | Scope | Result |
|---|---|---|
| r3a-agent-runtime (st_01a13b3b) | omo_backend, omo_daemon, main, multiplexer/actor, discord/adapter, discord/commands (F1, F5, F6/F6b, F7/F7b, F8, F9, F10) | NO FINDINGS (0 P0/P1) |
| r3b-lanes-cron-sec (st_01a13b3c) | settle, runner, guard, scheduler, hardline, sessions/external, guard test (F2, F3, F4, F11, F14, F16) | 0 P0/P1, 1 P2. F16 VERIFIED |
| r3c-dashboard-deploy (st_01a13b3d) | dashboard, docker-compose, README, deploy.sh (F12, F13, F15) | NO FINDINGS (0 P0/P1) |

| r3d-weekday-step (st_01a13b3e) | scheduler.rs bare `n/step` weekday item now ends at 6 (F3c). Gate run1 failed on the `1/2` assertion, and this is the fix. | 0 P0/P1, 1 P2. Fix VERIFIED (8/8 assertions hand-traced) |

## Gate run1 finding
- `cron::scheduler::tests::five_field_weekdays_use_unix_numbering` failed: `1/2` gave "1,2,4,6", expected "2,4,6".
- This was the only failure across 78 test binaries.
- The test expectation was right (croniter's weekday range is 0-6), so the code was fixed (F3c) and re-reviewed in r3d.

| r3e-socket-bind (st_01a13b43) | cron/report.rs NativeServer::bind without the process-wide umask (F17, with the staging name shortened to .cs<8hex> for sun_path) | 0 P0/P1, 3 optional P2. F17 VERIFIED |

## Gate run2 finding (F17)
- `legacy::runner_tests::cron_script_timeout_reaps_descendants` (test_review_dashboard binary) failed with PermissionDenied at main.rs:2702 (`fs::create_dir_all` in a fresh tempdir). It was the only failure across 78 binaries. The same test passed in run1 (2 ok).
- Root cause (pre-existing, from commit 3a3adfa, outside F1–F16): `NativeServer::bind` (cron/report.rs) did `libc::umask(0o177)` around the bind. umask is process-wide, so a test thread creating directories at the same moment in the same binary (the bind test is at main.rs:1489) got mode 0600 dirs, then EACCES. In production the gateway can briefly create files and dirs with the wrong mode the same way.
- FIX F17: no umask. Bind inside a fresh 0700 staging dir next to the target, chmod the socket to 0600, rename it to the final path, remove the staging dir.
  - Lead probe on this Mac (python, same steps): the renamed listening socket still accepts connections, and the mode stays 0600.
  - The existing test_fix_r5_cron_exec asserts mode 0600 on the final path, so it still covers this.

## Lead falsification
- r3b P2 (on Linux the remote check uses st_ctime instead of birthtime): accepted as P2.
  - The failure mode is conservative: Done can be wrongly refused, never wrongly accepted, and no data is lost.
  - The remote target is used by 0 external lanes in the live DB.
  - Recorded only; not fixed, to keep the gate's code freeze.
- r3c note (a mid-request bind yields 404 wording): P2 wording only, and nothing is lost. Recorded.
- The prior r2 lanes r2a–r2i (CONFIRMED-r2.partial.md) each verified their fix. Every P2 that needed a code change was applied before r3, and r3 covered that final diff.

- r3d P2: a bare `7/step` is now passed untranslated, so it runs on Saturday instead of Sunday.
  - It is an obscure form that Vixie cron rejects outright, and no schedule in the repo uses it.
  - Recorded only; not fixed, to keep the code frozen for gate run2.

- r3e P2s, all recorded only (the code stays frozen for gate run3):
  - the staging dir has the umask-default mode before chmod 0700 (empty dir, DoS-only with a group-writable parent);
  - leftover .cs* dirs remain after a crash mid-bind;
  - two gateways starting at once race on the stale check and rename (pre-existing, same race as before).

## VERDICT
Final delta re-review round r3: 0 confirmed P0, 0 confirmed P1 across all 5 lanes (r3a, r3b, r3c, r3d, r3e).

## Integration gate (single gate on the final frozen code)
- gate/run3/SUMMARY.txt: fmt exit=0, clippy exit=0, test exit=0 (78/78 binaries `test result: ok`, 0 FAILED), web skipped (no web/src change). GATE-PASS.
- History:
  - run1 failed on fmt (a single formatting line) and on the F3 weekday test, which was a real bug, fixed as F3c.
  - run2 failed on the umask race, a real pre-existing bug, fixed as F17.
  - Both fixes were delta re-reviewed (r3d, r3e) before run3.
