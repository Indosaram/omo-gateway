# Guard-only repair provenance

Running monitor: mon_G5ESGVGSM8FQRMZE; session bash_15; batch PID 39537 recorded in guard-repair-20261011T144644Z-gates.started. Source-settled SHA-256: 11e1f9e8a098463edceab0beccb87edadbc869bc70e006640ed0510f25be08a7. Scoped rustfmt exit 0. Only tests/test_lane_reopen.rs differs from the prior repair source manifest.

Authorized source path: dori-work/tests/test_lane_reopen.rs. Before SHA-256: 623e1ade66f7c91164d44d4ef2f9d0e2ab0a73c8a8d118c67260ce3108897b0a, equal to canonical and prior repair manifests. No readers or cargo/rustfmt/clippy processes observed before edit.

Change: enclose each `seen` guard and every assertion using it in a lexical block, ending before subsequent database awaits. All assertions remain; no race synchronization or old-turn steering changes. No dori-next file copied or edited; active race investigation belongs to st_01a12b2d.

Read current test, prior scoped guard receipts next-posts-20261011T142217Z-results.md and next-clippy-20261011T142121Z-diagnostics.md, plus prior repair diagnostics. Rust LSP: no diagnostics. Shell harness syntax checked before execution; Bash LSP unavailable in this session.

Source manifest policy: rehash every path in repair-20261011T144053Z-post-manifest.sha256. Separately hash task-7.log and every prior repair receipt/harness before and after. No existing receipt is overwritten. New raw logs and status are under guard-repair-20261011T144644Z.

Run policy: scoped rustfmt once; then one strict all-target/all-feature clippy on existing target-w4 and one focused test_lane_reopen. No full/state/inbox/shell replay. Two build jobs. Monitor deadline 1800000 ms. Durable noclobber started marker records batch PID; restore must attach/read status, never relaunch. Focused test TMPDIR is a uniquely owned scratch directory; record remaining entries and remove only if empty with rmdir. No live database/service/Discord/git actions.
