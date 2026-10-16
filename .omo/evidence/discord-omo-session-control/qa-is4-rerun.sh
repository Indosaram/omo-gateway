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
tmux kill-session -t omon-qa-tui 2>/dev/null; tmux new-session -d -s omon-qa-tui -c "$DIR" "omo --name omon-qa-tui"
sleep 15
ID=$(T 30 omo thread list --all-scope --json | python3 -c 'import json,sys;print(next(s["sessionId"] for s in json.load(sys.stdin) if "omon-qa-tui" in (s.get("cwd") or "")))')
echo "tui=$ID" | tee "$EV/tui-session.txt"
TARGET="local:${ID:0:13}"
run is4-send-long "$BIN" sessions send "$TARGET" --text "Without using any tools, write a very long 4000-word essay about the history of rivers directly in your reply."
for i in $(seq 1 45); do sleep 1; state "$ID" | grep -q '"streaming": true' && break; done
state "$ID" > "$EV/is4-before.json"
run is4-interrupt "$BIN" sessions interrupt "$TARGET"
for i in $(seq 1 15); do sleep 2; state "$ID" | grep -q '"streaming": false' && break; done
state "$ID" > "$EV/is4-after.json"
tmux capture-pane -p -t omon-qa-tui > "$EV/is4-pane.txt" 2>&1
{ tmux kill-session -t omon-qa-tui; echo "tmux kill rc=$?"; rm -rf "$DIR"; echo "rm dir rc=$?"; tmux has-session -t omon-qa-tui 2>/dev/null && echo still || echo "tmux gone"; } > "$EV/cleanup.txt" 2>&1
cat "$EV/cleanup.txt"
