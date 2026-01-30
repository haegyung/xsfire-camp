#!/usr/bin/env bash
set -euo pipefail

SETTINGS_PATH="${SETTINGS_PATH:-$HOME/.config/zed/settings.json}"
SETTINGS_DIR="$(dirname "$SETTINGS_PATH")"

if [[ $# -ge 1 ]]; then
  BACKUP_PATH="$1"
else
  BACKUP_PATH=$(ls -t "$SETTINGS_DIR"/settings.json.bak-* 2>/dev/null | head -n 1 || true)
fi

if [[ -z "${BACKUP_PATH}" || ! -f "$BACKUP_PATH" ]]; then
  echo "Backup file not found. Provide a backup path or create one first." >&2
  exit 1
fi

TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)
if [[ -f "$SETTINGS_PATH" ]]; then
  PRE_RESTORE="${SETTINGS_PATH}.pre-restore-${TIMESTAMP}"
  cp -p "$SETTINGS_PATH" "$PRE_RESTORE"
  echo "Current settings backed up: $PRE_RESTORE"
fi

cp -p "$BACKUP_PATH" "$SETTINGS_PATH"

echo "Restored settings from: $BACKUP_PATH"
