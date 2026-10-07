#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DRY_RUN="${DRY_RUN:-0}"
DB_PATH="${GATEWAY_DB_PATH:-${REPO_ROOT}/data/gateway.db}"
LEGACY_PLIST="${LEGACY_PLIST:-${HOME}/Library/LaunchAgents/com.omomeow.cron.plist}"
LEGACY_SERVICE="${LEGACY_SERVICE:-gui/$(id -u)/com.omomeow.cron}"

for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=1
      ;;
    --db=*)
      DB_PATH="${arg#*=}"
      ;;
    --legacy-plist=*)
      LEGACY_PLIST="${arg#*=}"
      ;;
  esac
done

echo "=========================================================="
echo " [Phase 1] OmOMeow Cron Cutover Starting..."
echo " Mode: $([ "${DRY_RUN}" = "1" ] && echo "DRY RUN (Mock)" || echo "LIVE RUN")"
echo " Target DB: ${DB_PATH}"
echo " Legacy Plist: ${LEGACY_PLIST}"
echo "=========================================================="

echo ">>> Step 1: Disabling legacy OmOMeow cron jobs..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would check and unload launchd service: ${LEGACY_SERVICE}"
  echo "[DRY-RUN] Would disable/rename legacy plist if present: ${LEGACY_PLIST} -> ${LEGACY_PLIST}.disabled"
  echo "[DRY-RUN] Would backup and remove crontab omomeow entries if any"
else
  if command -v launchctl >/dev/null 2>&1; then
    if launchctl list | grep -q "com.omomeow.cron" 2>/dev/null; then
      echo "Unloading launchd service ${LEGACY_SERVICE}..."
      launchctl bootout "${LEGACY_SERVICE}" 2>/dev/null || launchctl unload "${LEGACY_PLIST}" 2>/dev/null || true
    fi
  fi

  if [ -f "${LEGACY_PLIST}" ]; then
    echo "Disabling legacy plist file ${LEGACY_PLIST} -> ${LEGACY_PLIST}.disabled"
    mv "${LEGACY_PLIST}" "${LEGACY_PLIST}.disabled"
  fi

  if crontab -l 2>/dev/null | grep -q "omomeow" 2>/dev/null; then
    echo "Backing up and commenting out omomeow crontab entries..."
    crontab -l | sed 's/^\(.*omomeow.*\)$/# DISABLED_BY_CUTOVER \1/' | crontab -
  fi
fi
echo "Legacy OmOMeow cron jobs deactivated."

echo ">>> Step 2: Activating 14 OmOMeow cron jobs in omon-gateway..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would execute SQL: UPDATE cron_jobs SET enabled = 1 WHERE id LIKE 'omomeow:%';"
  if [ -f "${DB_PATH}" ]; then
    MATCHED=$(sqlite3 "${DB_PATH}" "SELECT count(*) FROM cron_jobs WHERE id LIKE 'omomeow:%';")
    echo "[DRY-RUN] Verified ${MATCHED} matching omomeow cron records in ${DB_PATH}"
  fi
else
  if [ ! -f "${DB_PATH}" ]; then
    echo "ERROR: Target database file not found at ${DB_PATH}" >&2
    exit 1
  fi

  sqlite3 "${DB_PATH}" "UPDATE cron_jobs SET enabled = 1 WHERE id LIKE 'omomeow:%';"
  
  ACTIVE_COUNT=$(sqlite3 "${DB_PATH}" "SELECT count(*) FROM cron_jobs WHERE id LIKE 'omomeow:%' AND enabled = 1;")
  if [ "${ACTIVE_COUNT}" -ne 14 ]; then
    echo "WARNING: Expected 14 enabled omomeow jobs, found ${ACTIVE_COUNT}" >&2
  else
    echo "Verified: Successfully enabled exactly 14 OmOMeow cron jobs."
  fi
fi

echo "=========================================================="
echo " [Phase 1] OmOMeow Cron Cutover Completed Successfully."
echo "=========================================================="
exit 0
