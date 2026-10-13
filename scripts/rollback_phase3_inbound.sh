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
  EXTRACTED_TOKEN=""
  if [ -f "${OMOMEOW_ENV_SRC}" ]; then
    EXTRACTED_TOKEN=$(python3 -c "import json; data=json.load(open('${OMOMEOW_ENV_SRC}')); print(data.get('token', data.get('discord_token', '')))" 2>/dev/null || echo "")
  fi
  if [ -f "${ENV_FILE}" ] && [ -n "${EXTRACTED_TOKEN}" ]; then
    ENV_BACKUP="${ENV_FILE}.backup.$(date +%Y%m%dT%H%M%S)"
    cp -p "${ENV_FILE}" "${ENV_BACKUP}"
    echo "Backed up ${ENV_FILE} to ${ENV_BACKUP}"
    echo "Removing only the OmOMeow token from ${ENV_FILE}..."
    python3 - "${ENV_FILE}" "${EXTRACTED_TOKEN}" <<'PY'
import os, sys, tempfile
path, token = sys.argv[1], sys.argv[2]
mode = "remove"
lines = open(path).read().split("\n") if os.path.exists(path) else []
out, found = [], False
for line in lines:
    if line.startswith("DISCORD_BOT_TOKENS="):
        found = True
        tokens = [t for t in line.split("=", 1)[1].split(",") if t.strip()]
        if mode == "add" and token not in tokens:
            tokens.append(token)
        if mode == "remove":
            tokens = [t for t in tokens if t != token]
        line = "DISCORD_BOT_TOKENS=" + ",".join(tokens)
    out.append(line)
if mode == "add" and not found:
    if out and out[-1] == "":
        out.pop()
    out.append("DISCORD_BOT_TOKENS=" + token)
    out.append("")
fd, tmp = tempfile.mkstemp(dir=os.path.dirname(os.path.abspath(path)))
with os.fdopen(fd, "w") as handle:
    handle.write("\n".join(out))
os.chmod(tmp, os.stat(path).st_mode & 0o777 if os.path.exists(path) else 0o600)
os.replace(tmp, path)
PY
  else
    echo "WARNING: OmOMeow token or ${ENV_FILE} unavailable; ${ENV_FILE} left unchanged" >&2
  fi

  if command -v launchctl >/dev/null 2>&1 && [ -f "${GATEWAY_PLIST}" ]; then
    echo "Restarting omon-gateway without OmOMeow token..."
    launchctl kickstart -k "${GATEWAY_SERVICE}" 2>/dev/null || true
  else
    pkill -x "omo-gateway" 2>/dev/null || true
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
