#!/bin/bash
# one assertion per lane: report exists, starts with "# Lane", has findings or NO FINDINGS
cd "$(dirname "$0")/r1" || exit 2
fail=0
for l in discord-adapter-a discord-adapter-b discord-commands discord-auth main-runtime-a main-runtime-b dashboard multiplexer agent-backend agent-runtime cron-scheduler cron-store-exec tools-exec tools-context security lanes-runner lanes-signals lanes-surface sessions-judge storage-ledger inbox-migrate-voice web-deploy; do
  f="$l.md"
  if [ -s "$f" ] && head -1 "$f" | grep -q '^# Lane' && { grep -q '^### \[P[012]\]' "$f" || grep -qx 'NO FINDINGS' "$f"; }; then echo "PASS $l"; else echo "FAIL $l"; fail=1; fi
done
exit $fail
