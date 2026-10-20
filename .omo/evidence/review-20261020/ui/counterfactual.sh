#!/bin/bash
# Counterfactual pass: temporarily revert each fix, confirm the UI check fails, restore the file.
set -u
cd /Users/indo/code/project/omon-gateway
OUT=/tmp/omoverify
mkdir -p "$OUT/backup"

check() { # name, file
  local name="$1" file="$2"
  cp "$file" "$OUT/backup/$name.bak"
  local before
  before=$(md5 -q "$file")
  echo "$before" > "$OUT/backup/$name.md5"
  echo "== reverted $name (baseline $before)"
}

restore() { # name, file
  local name="$1" file="$2"
  cp "$OUT/backup/$name.bak" "$file"
  local now want
  now=$(md5 -q "$file"); want=$(cat "$OUT/backup/$name.md5")
  if [ "$now" = "$want" ]; then echo "== restored $name ($now)"; else echo "!! RESTORE MISMATCH $name $now != $want"; exit 9; fi
}

run() { # label
  ( cd "$OUT" && bun verify.ts 2>&1 | grep -E '^(PASS|FAIL|SUMMARY)' ) | sed "s/^/[$1] /"
}

# ---------------------------------------------------------------- D23
F=web/src/tracker/pages/SearchPage.tsx
check D23 "$F"
python3 - <<'EOF'
p='web/src/tracker/pages/SearchPage.tsx'
s=open(p).read()
s=s.replace("      setRemote(null)\n      setLoading(false)\n","      setRemote(null)\n",1)
open(p,'w').write(s)
EOF
run D23-reverted
restore D23 "$F"

# ---------------------------------------------------------------- D25
F=web/src/tracker/pages/ProjectPage.tsx
check D25 "$F"
python3 - <<'EOF'
p='web/src/tracker/pages/ProjectPage.tsx'
s=open(p).read()
s=s.replace("""    if (required && !draft.trim()) {
      setDraft(value)
      return
    }
""","",1)
open(p,'w').write(s)
EOF
run D25-reverted
restore D25 "$F"

# ---------------------------------------------------------------- D28
F=web/src/tracker/projects/cycles.tsx
check D28 "$F"
python3 - <<'EOF'
p='web/src/tracker/projects/cycles.tsx'
s=open(p).read()
block = """  // The dialog stays mounted while closed, so re-seed the default range each time it opens.
  const [wasOpen, setWasOpen] = useState(open)
  if (open !== wasOpen) {
    setWasOpen(open)
    if (open) {
      setStartsAt(toDateOnly(defaultStart))
      setEndsAt(toDateOnly(new Date(defaultStart.getTime() + (team.cycle_weeks || 2) * 7 * DAY)))
    }
  }

"""
assert block in s, 'D28 block not found'
s=s.replace(block,"",1)
open(p,'w').write(s)
EOF
run D28-reverted
restore D28 "$F"

# ---------------------------------------------------------------- D29
F=web/src/tracker/projects/Milestones.tsx
check D29 "$F"
python3 - <<'EOF'
p='web/src/tracker/projects/Milestones.tsx'
s=open(p).read()
assert 'if (!name.trim() || saving) return' in s
s=s.replace('if (!name.trim() || saving) return','if (!name.trim()) return',1)
open(p,'w').write(s)
EOF
run D29-reverted
restore D29 "$F"

echo "== final source hashes"
md5 -q web/src/tracker/pages/SearchPage.tsx web/src/tracker/pages/ProjectPage.tsx web/src/tracker/projects/cycles.tsx web/src/tracker/projects/Milestones.tsx
