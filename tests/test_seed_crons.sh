#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

TMP_DB="/tmp/test_crons_$$.db"
trap 'rm -f "${TMP_DB}"' EXIT

sqlite3 "${TMP_DB}" < "${REPO_ROOT}/migrations/0001_initial.sql"
sqlite3 "${TMP_DB}" < "${REPO_ROOT}/scripts/seed_omomeow_crons.sql"

TOTAL_ROWS=$(sqlite3 "${TMP_DB}" "SELECT COUNT(*) FROM cron_jobs;")
if [ "${TOTAL_ROWS}" -ne 14 ]; then
  echo "ERROR: expected 14 rows, got ${TOTAL_ROWS}" >&2
  exit 1
fi

ENABLED_ROWS=$(sqlite3 "${TMP_DB}" "SELECT COUNT(*) FROM cron_jobs WHERE enabled != 0;")
if [ "${ENABLED_ROWS}" -ne 0 ]; then
  echo "ERROR: expected 0 enabled rows, got ${ENABLED_ROWS}" >&2
  exit 1
fi

echo "SEED VERIFIED 14 JOBS DISABLED"
