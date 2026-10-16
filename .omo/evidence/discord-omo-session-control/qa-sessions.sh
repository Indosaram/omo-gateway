#!/usr/bin/env bash
# Real-surface QA for plan discord-omo-session-control. Usage: qa-sessions.sh <omo-gateway binary> <evidence dir>
set -u
BIN="$1"; EV="$2"; mkdir -p "$EV"
SCRATCH=$(mktemp -d -t omon-qa-sessions); echo "$SCRATCH" > "$EV/scratch-dir.txt"
export DATABASE_URL="sqlite://$SCRATCH/qa.db?mode=rwc"
T() { local secs="$1"; shift; perl -e 'alarm shift; exec @ARGV' "$secs" "$@"; }
run() { local name="$1"; shift; { echo "\$ $*"; T 60 "$@"; echo "rc=$?"; } >"$EV/$name.txt" 2>&1; tail -n 3 "$EV/$name.txt"; }

LOCAL_QA_DIR=$(mktemp -d -t omon-qa-local)
OPEN=$(cd "$LOCAL_QA_DIR" && T 60 omo host session open --cwd "$LOCAL_QA_DIR" --name omon-qa-local --json)
echo "$OPEN" > "$EV/local-open.json"
LOCAL_ID=$(printf '%s' "$OPEN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["durableSessionId"])')
LOCAL_RID=$(printf '%s' "$OPEN" | python3 -c 'import json,sys;print(json.load(sys.stdin)["sessionId"])')
T 40 ssh -o BatchMode=yes -o ConnectTimeout=5 omarchy 'd=$(mktemp -d /tmp/omon-qa-remote.XXXXXX); tmux new-session -d -s omon-qa-remote -c "$d" "omo --name omon-qa-remote"; sleep 15' 2>/dev/null
REMOTE_ID=$(T 30 ssh -o BatchMode=yes -o ConnectTimeout=5 omarchy 'omo thread list --all-scope --json' 2>/dev/null | python3 -c 'import json,sys;print(next(s["sessionId"] for s in json.load(sys.stdin) if "omon-qa-remote" in (s.get("cwd") or "")))')
REMOTE_RID="$REMOTE_ID"
echo "remote tmux session omon-qa-remote id=$REMOTE_ID" > "$EV/remote-open.json"
echo "local=$LOCAL_ID($LOCAL_RID) remote=$REMOTE_ID($REMOTE_RID)" | tee "$EV/qa-sessions.txt"

run is1-overview "$BIN" sessions overview
run is1-overview-json "$BIN" sessions overview --json
OTHER=$(T 30 omo thread list --all-scope --json | python3 -c 'import json,sys,os;print(next(s["sessionId"] for s in json.load(sys.stdin) if s.get("cwd")!=os.getcwd() and s.get("surface")=="tui"))')
run is2-read-other-workspace "$BIN" sessions read "local:${OTHER:0:13}" --limit 2
run is2-read-remote "$BIN" sessions read "omarchy:$REMOTE_ID" --limit 1
run is7-ambiguous-prefix "$BIN" sessions read "local:01a1" --limit 1
run is3-send-local-multiline "$BIN" sessions send "local:$LOCAL_ID" --text $'Reply with exactly the two words QA-LOCAL-OK.\nSecond line is part of the same message.'
run is3-send-remote-multiline "$BIN" sessions send "omarchy:$REMOTE_ID" --text $'Reply with exactly the two words QA-REMOTE-OK.\nSecond line is part of the same message.'
T 120 omo host session wait "$LOCAL_RID" --until idle --timeout 110000 --json >"$EV/is3-local-wait.json" 2>&1
for i in $(seq 1 40); do sleep 3; "$BIN" sessions overview --json 2>/dev/null | python3 -c 'import json,sys;o=json.load(sys.stdin);s=[x for m,r in o["snapshots"] if m=="omarchy" and "Ok" in r for x in r["Ok"]["sessions"] if x["session_id"]==sys.argv[1]];sys.exit(0 if s and not s[0]["streaming"] and s[0]["message_count"]>0 else 1)' "$REMOTE_ID" && break; done; echo "remote idle after $i polls" >"$EV/is3-remote-wait.json"
run is3-read-local-after "$BIN" sessions read "local:$LOCAL_ID" --limit 4
run is3-read-remote-after "$BIN" sessions read "omarchy:$REMOTE_ID" --limit 4

run is4-send-long "$BIN" sessions send "local:$LOCAL_ID" --text "Run the bash command: sleep 300 ; then say done."
sleep 8
run is4-interrupt "$BIN" sessions interrupt "local:$LOCAL_ID"
sleep 5
run is4-overview-after "$BIN" sessions overview --json

run is5-ask "$BIN" sessions send "local:$LOCAL_ID" --text "Call the ask_user_question tool once with waitForAnswer false, one question header 'QA', question 'Pick one', options 'Alpha' and 'Beta'. Then end your turn."
T 120 omo host session wait "$LOCAL_RID" --until idle --timeout 110000 --json >"$EV/is5-wait.json" 2>&1
run is5-overview-question "$BIN" sessions overview
run is6-watch-tick-1 "$BIN" sessions watch-tick --once
run is6-watch-tick-2 "$BIN" sessions watch-tick --once
run is5-answer "$BIN" sessions answer "local:$LOCAL_ID" --text "Alpha"
T 120 omo host session wait "$LOCAL_RID" --until idle --timeout 110000 --json >"$EV/is5-wait-after.json" 2>&1
run is5-read-after-answer "$BIN" sessions read "local:$LOCAL_ID" --limit 6

run is10-adopt "$BIN" sessions adopt "omarchy:${REMOTE_ID:0:13}" --goal "QA adopt probe" --done "file /tmp/omon-qa-adopt-nonexistent" --lead '7:discord|-|19:1536981942963150899|-|18:414011632306618368'
run is10-db-row sqlite3 "$SCRATCH/qa.db" "select l.key, l.kind, l.external_target, e.cursor from lanes l join external_sessions e on e.lane_key=l.key"
run is10-release "$BIN" sessions release "omarchy:${REMOTE_ID:0:13}"
run is10-db-after sqlite3 "$SCRATCH/qa.db" "select l.status, l.external_target, e.released_at is not null from lanes l join external_sessions e on e.lane_key=l.key"
run is11-unreachable env OMON_SESSION_HOSTS=omarchy,nonexistent-qa-host.invalid "$BIN" sessions overview

{
  echo "\$ cleanup"
  (cd "$LOCAL_QA_DIR" && T 30 omo host session close "$LOCAL_RID" --json)
  T 30 ssh -o BatchMode=yes -o ConnectTimeout=5 omarchy 'tmux kill-session -t omon-qa-remote; rm -rf /tmp/omon-qa-remote.*; tmux ls 2>&1 | grep -c omon-qa-remote' 2>/dev/null
  rm -rf "$LOCAL_QA_DIR" "$SCRATCH"
  T 30 omo thread list --all-scope --json | grep -c omon-qa-local
  T 30 ssh -o BatchMode=yes -o ConnectTimeout=5 omarchy 'omo thread list --all-scope --json' 2>/dev/null | grep -c omon-qa-remote
} >"$EV/cleanup.txt" 2>&1
tail -n 6 "$EV/cleanup.txt"
