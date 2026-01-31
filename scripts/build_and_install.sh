#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
INSTALL_PATH="${INSTALL_PATH:-$HOME/.local/bin/theprometheus-codex-acp}"

CARGO_HOME_VALUE="${CARGO_HOME:-}"
CARGO_TARGET_DIR_VALUE="${CARGO_TARGET_DIR:-}"

mkdir -p "$(dirname "$INSTALL_PATH")"

ENV_PREFIX=()
if [[ -n "$CARGO_HOME_VALUE" ]]; then
  ENV_PREFIX+=("CARGO_HOME=$CARGO_HOME_VALUE")
fi
if [[ -n "$CARGO_TARGET_DIR_VALUE" ]]; then
  ENV_PREFIX+=("CARGO_TARGET_DIR=$CARGO_TARGET_DIR_VALUE")
fi

(
  cd "$ROOT_DIR"
  if [[ ${#ENV_PREFIX[@]} -gt 0 ]]; then
    "${ENV_PREFIX[@]}" cargo build --release
  else
    cargo build --release
  fi
)

BIN_PATH="$ROOT_DIR/target/release/codex-acp"
if [[ -n "$CARGO_TARGET_DIR_VALUE" ]]; then
  BIN_PATH="$CARGO_TARGET_DIR_VALUE/release/codex-acp"
fi

if [[ ! -f "$BIN_PATH" ]]; then
  echo "Build output not found: $BIN_PATH" >&2
  exit 1
fi

cp -f "$BIN_PATH" "$INSTALL_PATH"
chmod +x "$INSTALL_PATH"

echo "Installed: $INSTALL_PATH"
