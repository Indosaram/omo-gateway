# Commit and deploy (review-20261014)
- The user asked to "commit and deploy". The other session's inbox changes were included per the user's answer ("include everything, commit and deploy").
- Gate on the full tree before the commit: run4 (clippy 0, test 0, fmt 1 on one line of tests/test_inbox_triage.rs), then rustfmt, then run5 with fmt 0, clippy 0, test 0 (78/78), GATE-PASS. No file changed after run5 started.
- Commit: e57b472 "fix: close confirmed defects from a full review round" (28 files; .omo/, .senpi/, docs/research/ excluded). Not pushed.
- Deploy: scripts/deploy.sh exit 0 (log: deploy.log).
  - The backup is at ~/.omon/deploy-backups/omo-gateway.20261015-065835.
  - The binary is signed with the Developer ID (TeamIdentifier=5DUM8WPB4C).
  - Verification passed: gateway pid 41310, Discord ready, app-server 42032 (ppid 41310), readyz 200.
  - The F15 string is present in the deployed binary.
