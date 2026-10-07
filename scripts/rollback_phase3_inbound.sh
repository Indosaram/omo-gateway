#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DRY_RUN="${DRY_RUN:-0}"
ENV_FILE="${ENV_FILE:-${REPO_ROOT}/.env}"
OMOMEOW_PLIST="${OMOMEOW_PLIST:-${HOME}/Library/LaunchAgents/com.omomeow.plist}"
OMOMEOW_SERVICE="${OMOMEOW_SERVICE:-gui/$(id -u)/com.omomeow}"
GATEWAY_SERVICE="${GATEWAY_SERVICE:-gui/$(id -u)/com.omon.gateway}"
GATEWAY_PLIST="${GATEWAY_PLIST:-${HOME}/Library/LaunchAgents/com.omon.gateway.plist}"

for arg in "$@"; do
  case "$arg" in
    --dry-run)
      DRY_RUN=1
      ;;
    --env=*)
      ENV_FILE="${arg#*=}"
      ;;
    --omomeow-plist=*)
      OMOMEOW_PLIST="${arg#*=}"
      ;;
    --gateway-plist=*)
      GATEWAY_PLIST="${arg#*=}"
      ;;
  esac
done

echo "=========================================================="
echo " [Phase 3 Rollback] OmOMeow Inbound Traffic Rollback Starting..."
echo " Mode: $([ "${DRY_RUN}" = "1" ] && echo "DRY RUN (Mock)" || echo "LIVE RUN")"
echo " Gateway Env File: ${ENV_FILE}"
echo " Legacy Service: ${OMOMEOW_SERVICE}"
echo "=========================================================="

echo ">>> Step 1: Stopping omon-gateway OmOMeow bot listener..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would remove or blank out OmOMeow token in ${ENV_FILE}"
  echo "[DRY-RUN] Would restart omon-gateway to release Discord Gateway websocket connection"
else
  if [ -f "${ENV_FILE}" ]; then
    echo "Reverting OmOMeow token in ${ENV_FILE}..."
    sed -i.bak "s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=|" "${ENV_FILE}" || true
    rm -f "${ENV_FILE}.bak"
  fi

  if command -v launchctl >/dev/null 2>&1 && [ -f "${GATEWAY_PLIST}" ]; then
    echo "Restarting omon-gateway without OmOMeow token..."
    launchctl kickstart -k "${GATEWAY_SERVICE}" 2>/dev/null || true
  else
    pkill -f "omon-gateway" 2>/dev/null || true
  fi
fi

echo ">>> Step 2: Restoring legacy OmOMeow service..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would restore plist: ${OMOMEOW_PLIST}.disabled -> ${OMOMEOW_PLIST}"
  echo "[DRY-RUN] Would bootstrap legacy daemon: ${OMOMEOW_SERVICE}"
else
  if [ -f "${OMOMEOW_PLIST}.disabled" ]; then
    echo "Restoring legacy daemon plist: ${OMOMEOW_PLIST}.disabled -> ${OMOMEOW_PLIST}"
    mv "${OMOMEOW_PLIST}.disabled" "${OMOMEOW_PLIST}"
  fi

  if [ -f "${OMOMEOW_PLIST}" ] && command -v launchctl >/dev/null 2>&1; then
    echo "Starting legacy OmOMeow service..."
    launchctl bootstrap "gui/$(id -u)" "${OMOMEOW_PLIST}" 2>/dev/null || launchctl load "${OMOMEOW_PLIST}" 2>/dev/null || true
  fi
fi

echo "=========================================================="
echo " [Phase 3 Rollback] Inbound Rollback Completed."
echo "=========================================================="
exit 0
