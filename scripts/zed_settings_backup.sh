#!/usr/bin/env bash
set -euo pipefail

SETTINGS_PATH="${SETTINGS_PATH:-$HOME/.config/zed/settings.json}"

if [[ ! -f "$SETTINGS_PATH" ]]; then
  echo "Zed settings not found: $SETTINGS_PATH" >&2
  exit 1
fi

TIMESTAMP=$(date -u +%Y%m%dT%H%M%SZ)

if [[ $# -ge 1 ]]; then
  OUT_PATH="$1"
else
  OUT_PATH="${SETTINGS_PATH}.bak-${TIMESTAMP}"
fi

cp -p "$SETTINGS_PATH" "$OUT_PATH"

echo "Backup created: $OUT_PATH"
