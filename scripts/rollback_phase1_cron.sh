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
echo " [Phase 1 Rollback] OmOMeow Cron Rollback Starting..."
echo " Mode: $([ "${DRY_RUN}" = "1" ] && echo "DRY RUN (Mock)" || echo "LIVE RUN")"
echo " Target DB: ${DB_PATH}"
echo " Legacy Plist: ${LEGACY_PLIST}"
echo "=========================================================="

echo ">>> Step 1: Disabling 14 OmOMeow cron jobs in omon-gateway..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would execute SQL: UPDATE cron_jobs SET enabled = 0 WHERE id LIKE 'omomeow:%';"
  if [ -f "${DB_PATH}" ]; then
    MATCHED=$(sqlite3 "${DB_PATH}" "SELECT count(*) FROM cron_jobs WHERE id LIKE 'omomeow:%';")
    echo "[DRY-RUN] Verified ${MATCHED} matching omomeow cron records in ${DB_PATH}"
  fi
else
  if [ ! -f "${DB_PATH}" ]; then
    echo "ERROR: Target database file not found at ${DB_PATH}" >&2
    exit 1
  fi

  sqlite3 "${DB_PATH}" "UPDATE cron_jobs SET enabled = 0 WHERE id LIKE 'omomeow:%';"
  
  ACTIVE_COUNT=$(sqlite3 "${DB_PATH}" "SELECT count(*) FROM cron_jobs WHERE id LIKE 'omomeow:%' AND enabled = 1;")
  echo "Verified: ${ACTIVE_COUNT} active omomeow jobs remaining in omon-gateway (expected 0)."
fi

echo ">>> Step 2: Restoring legacy OmOMeow cron jobs..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would restore plist file if disabled: ${LEGACY_PLIST}.disabled -> ${LEGACY_PLIST}"
  echo "[DRY-RUN] Would bootstrap/load launchd service: ${LEGACY_SERVICE}"
  echo "[DRY-RUN] Would uncomment crontab omomeow entries if any"
else
  if [ -f "${LEGACY_PLIST}.disabled" ]; then
    echo "Restoring legacy plist ${LEGACY_PLIST}.disabled -> ${LEGACY_PLIST}"
    mv "${LEGACY_PLIST}.disabled" "${LEGACY_PLIST}"
  fi

  if [ -f "${LEGACY_PLIST}" ] && command -v launchctl >/dev/null 2>&1; then
    echo "Loading launchd service ${LEGACY_SERVICE}..."
    launchctl bootstrap "gui/$(id -u)" "${LEGACY_PLIST}" 2>/dev/null || launchctl load "${LEGACY_PLIST}" 2>/dev/null || true
  fi

  if crontab -l 2>/dev/null | grep -q "DISABLED_BY_CUTOVER" 2>/dev/null; then
    echo "Restoring crontab entries..."
    crontab -l | sed 's/^# DISABLED_BY_CUTOVER //' | crontab -
  fi
fi

echo "=========================================================="
echo " [Phase 1 Rollback] OmOMeow Cron Rollback Completed."
echo "=========================================================="
exit 0
