#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DRY_RUN="${DRY_RUN:-0}"
ENV_FILE="${ENV_FILE:-${REPO_ROOT}/.env}"
OMOMEOW_ENV_SRC="${OMOMEOW_ENV_SRC:-${HOME}/.config/agent-messenger/discordbot-credentials.json}"
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
echo " [Phase 3] OmOMeow Inbound Traffic Cutover Starting..."
echo " Mode: $([ "${DRY_RUN}" = "1" ] && echo "DRY RUN (Mock)" || echo "LIVE RUN")"
echo " Gateway Env File: ${ENV_FILE}"
echo " Legacy Service: ${OMOMEOW_SERVICE}"
echo " Gateway Service: ${GATEWAY_SERVICE}"
echo "=========================================================="

echo ">>> Step 1: Terminating legacy OmOMeow daemon process..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would unload and bootout legacy daemon: ${OMOMEOW_SERVICE}"
  echo "[DRY-RUN] Would disable plist: ${OMOMEOW_PLIST} -> ${OMOMEOW_PLIST}.disabled"
  echo "[DRY-RUN] Would pkill -f 'omomeow' if process remains"
else
  if command -v launchctl >/dev/null 2>&1; then
    if launchctl list | grep -q "com.omomeow" 2>/dev/null; then
      echo "Unloading launchd service ${OMOMEOW_SERVICE}..."
      launchctl bootout "${OMOMEOW_SERVICE}" 2>/dev/null || launchctl unload "${OMOMEOW_PLIST}" 2>/dev/null || true
    fi
  fi

  if [ -f "${OMOMEOW_PLIST}" ]; then
    echo "Disabling legacy daemon plist: ${OMOMEOW_PLIST} -> ${OMOMEOW_PLIST}.disabled"
    mv "${OMOMEOW_PLIST}" "${OMOMEOW_PLIST}.disabled"
  fi

  if pgrep -f "omomeow" >/dev/null 2>&1; then
    echo "Sending SIGTERM to existing omomeow processes..."
    pkill -f "omomeow" 2>/dev/null || true
    sleep 1
    if pgrep -f "omomeow" >/dev/null 2>&1; then
      echo "Sending SIGKILL to remaining omomeow processes..."
      pkill -9 -f "omomeow" 2>/dev/null || true
    fi
  fi
fi
echo "Legacy OmOMeow daemon stopped."

echo ">>> Step 2: Applying OmOMeow Production Bot Token to omon-gateway..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would verify OmOMeow credentials and inject DISCORD_BOT_TOKENS into ${ENV_FILE}"
else
  if [ ! -f "${ENV_FILE}" ]; then
    if [ -f "${REPO_ROOT}/.env.omomeow.example" ]; then
      echo "Creating ${ENV_FILE} from .env.omomeow.example..."
      cp "${REPO_ROOT}/.env.omomeow.example" "${ENV_FILE}"
    fi
  fi

  if [ -f "${OMOMEOW_ENV_SRC}" ]; then
    echo "Extracting production token from ${OMOMEOW_ENV_SRC}..."
    EXTRACTED_TOKEN=$(python3 -c "import json; data=json.load(open('${OMOMEOW_ENV_SRC}')); print(data.get('token', data.get('discord_token', '')))" 2>/dev/null || echo "")
    if [ -n "${EXTRACTED_TOKEN}" ]; then
      sed -i.bak "s|^DISCORD_BOT_TOKENS=.*|DISCORD_BOT_TOKENS=${EXTRACTED_TOKEN}|" "${ENV_FILE}" || true
      rm -f "${ENV_FILE}.bak"
      echo "Injected production token into ${ENV_FILE}."
    fi
  fi
fi

echo ">>> Step 3: Restarting omon-gateway daemon..."
if [ "${DRY_RUN}" = "1" ]; then
  echo "[DRY-RUN] Would restart omon-gateway via launchctl or cargo run"
else
  if command -v launchctl >/dev/null 2>&1 && [ -f "${GATEWAY_PLIST}" ]; then
    echo "Restarting omon-gateway via launchd..."
    launchctl kickstart -k "${GATEWAY_SERVICE}" 2>/dev/null || {
      launchctl unload "${GATEWAY_PLIST}" 2>/dev/null || true
      launchctl load "${GATEWAY_PLIST}" 2>/dev/null || true
    }
  else
    echo "Launchd plist not registered; restarting via pkill if running..."
    pkill -f "omon-gateway" 2>/dev/null || true
  fi
fi

echo "=========================================================="
echo " [Phase 3] Inbound Cutover Completed Successfully."
echo "=========================================================="
exit 0
