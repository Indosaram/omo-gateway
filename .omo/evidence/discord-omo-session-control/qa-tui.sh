#!/usr/bin/env bash
set -u
BIN="$1"; EV="$2"; mkdir -p "$EV"
T() { local secs="$1"; shift; perl -e 'alarm shift; exec @ARGV' "$secs" "$@"; }
run() { local name="$1"; shift; { echo "\$ $*"; T 90 "$@"; echo "rc=$?"; } >"$EV/$name.txt" 2>&1; tail -n 2 "$EV/$name.txt"; }
state() { "$BIN" sessions overview --json 2>/dev/null | python3 -c '
import json,sys
o=json.load(sys.stdin)
for m,r in o["snapshots"]:
    for s in (r.get("Ok") or {}).get("sessions",[]):
        if s["session_id"]==sys.argv[1]:
            print(json.dumps({"streaming":s["streaming"],"questions":s["questions"],"blocking":s["blocking_question"],"messages":s["message_count"]}))' "$1"; }

DIR=$(mktemp -d -t omon-qa-tui)
echo "$DIR" > "$EV/tui-dir.txt"
tmux new-session -d -s omon-qa-tui -c "$DIR" "omo --name omon-qa-tui"
sleep 15
ID=$(T 30 omo thread list --all-scope --json | python3 -c 'import json,sys;print(next(s["sessionId"] for s in json.load(sys.stdin) if "omon-qa-tui" in (s.get("cwd") or "")))')
echo "tui=$ID" | tee "$EV/tui-session.txt"
TARGET="local:${ID:0:13}"

run is4-send-long "$BIN" sessions send "$TARGET" --text "Run this bash command and wait for it: sleep 240. After it finishes reply DONE."
for i in $(seq 1 30); do sleep 2; state "$ID" | grep -q '"streaming": true' && break; done
state "$ID" > "$EV/is4-before.json"
run is4-interrupt "$BIN" sessions interrupt "$TARGET"
for i in $(seq 1 15); do sleep 2; state "$ID" | grep -q '"streaming": false' && break; done
state "$ID" > "$EV/is4-after.json"

run is5-ask "$BIN" sessions send "$TARGET" --text "Call the ask_user_question tool exactly once with waitForAnswer false: one question, header QA, question 'Pick one', options Alpha and Beta. Then end your turn without doing anything else."
for i in $(seq 1 60); do sleep 3; state "$ID" | grep -q '"request_id"' && break; done
state "$ID" > "$EV/is5-pending.json"
run is5-overview-question "$BIN" sessions overview
run is5-answer "$BIN" sessions answer "$TARGET" --text "Alpha"
for i in $(seq 1 40); do sleep 3; state "$ID" | grep -q '"questions": \[\]' && break; done
state "$ID" > "$EV/is5-after.json"
run is5-read-after "$BIN" sessions read "$TARGET" --limit 4

{
  echo "\$ cleanup"
  tmux kill-session -t omon-qa-tui 2>&1
  rm -rf "$DIR"
  tmux ls 2>&1 | grep -c omon-qa-tui
  T 30 omo thread list --all-scope --json | grep -c omon-qa-tui
} >"$EV/cleanup-tui.txt" 2>&1
tail -n 4 "$EV/cleanup-tui.txt"
