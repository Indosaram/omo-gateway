#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKTREE_DIR="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
REPORT_FILE="${SCRIPT_DIR}/REPORT.md"

if [ ! -f "${REPORT_FILE}" ]; then
  echo "ERROR: REPORT.md not found at ${REPORT_FILE}" >&2
  exit 1
fi

echo "Checking evidence paths from ${REPORT_FILE}..."
echo "Worktree root: ${WORKTREE_DIR}"

PATHS=$(grep -oE '\`.omo/evidence/[^`]+\`' "${REPORT_FILE}" | tr -d '\`' | sort -u)

COUNT=0
MISSING=0

while IFS= read -r rel_path; do
  [ -z "${rel_path}" ] && continue
  COUNT=$((COUNT + 1))
  target="${WORKTREE_DIR}/${rel_path}"
  if test -e "${target}"; then
    echo "  [OK] ${rel_path}"
  else
    echo "  [MISSING] ${rel_path} -> ${target}" >&2
    MISSING=$((MISSING + 1))
  fi
done <<< "${PATHS}"

echo "Checked ${COUNT} paths: ${MISSING} missing."

if [ "${MISSING}" -gt 0 ]; then
  echo "FAIL: ${MISSING} evidence path(s) do not exist." >&2
  exit 1
fi

echo "PASS: All evidence paths exist."
exit 0
