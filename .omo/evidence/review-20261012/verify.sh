#!/bin/bash
# usage: verify.sh <round-dir> lane...
D="$1"; shift; fail=0
for l in "$@"; do f="$D/$l.md"
 if [ ! -s "$f" ]; then echo "MISSING $l"; fail=1; continue; fi
 if grep -q '^NO FINDINGS$' "$f"; then echo "OK-EMPTY $l"; continue; fi
 n=$(grep -cE '^### \[P[012]\]' "$f"); loc=$(grep -cE '^- Location: [^ ]+:[0-9]+' "$f")
 if [ "$n" -eq 0 ] || [ "$loc" -lt "$n" ]; then echo "BADCONTRACT $l findings=$n locs=$loc"; fail=1; else echo "OK $l findings=$n P0=$(grep -c '^### \[P0\]' $f) P1=$(grep -c '^### \[P1\]' $f) P2=$(grep -c '^### \[P2\]' $f)"; fi
done; exit $fail
