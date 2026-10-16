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
            print(json.dumps({"machine":m,"streaming":s["streaming"],"messages":s["message_count"]}))' "$1"; }
RD=$(ssh omarchy 'mktemp -d -t omon-qa-ri.XXXXXX' 2>/dev/null)
echo "$RD" > "$EV/remote-dir.txt"
ssh omarchy "tmux kill-session -t omon-qa-ri 2>/dev/null; tmux new-session -d -s omon-qa-ri -c $RD 'bash -lc \"omo --name omon-qa-ri\"'" 2>/dev/null
sleep 20
ID=$("$BIN" sessions overview --json 2>/dev/null | python3 -c '
import json,sys
o=json.load(sys.stdin)
for m,r in o["snapshots"]:
    for s in (r.get("Ok") or {}).get("sessions",[]):
        if m=="omarchy" and "omon-qa-ri" in json.dumps(s): print(s["session_id"]); break')
echo "remote=$ID" | tee "$EV/remote-session.txt"
TARGET="omarchy:${ID:0:13}"
run ri-send-long "$BIN" sessions send "$TARGET" --text "Without using any tools, write a very long 4000-word essay about the history of rivers directly in your reply."
for i in $(seq 1 30); do sleep 1; state "$ID" | grep -q '"streaming": true' && break; done
state "$ID" > "$EV/ri-before.json"
run ri-interrupt "$BIN" sessions interrupt "$TARGET"
sleep 4
state "$ID" > "$EV/ri-after.json"
ssh omarchy 'tmux capture-pane -p -t omon-qa-ri' > "$EV/ri-pane.txt" 2>/dev/null
{ ssh omarchy "tmux kill-session -t omon-qa-ri; echo tmux kill rc=\$?; rm -rf $RD; echo rm rc=\$?; tmux has-session -t omon-qa-ri 2>/dev/null && echo still || echo tmux gone"; } > "$EV/cleanup.txt" 2>/dev/null
cat "$EV/cleanup.txt"
