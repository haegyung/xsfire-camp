#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="$ROOT_DIR/logs/manual_verification"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
REPORT_PATH="$REPORT_DIR/setup_monitor_${TIMESTAMP}.md"

usage() {
  cat <<'EOF'
Usage: scripts/manual_verification_setup_monitor.sh [--skip-gates]

Runs a deterministic preflight for setup/task-monitoring changes and creates a
manual verification checklist report under logs/manual_verification/.

Options:
  --skip-gates   Skip automated gates (cargo fmt --check, cargo test, node test)
EOF
}

SKIP_GATES="false"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-gates)
      SKIP_GATES="true"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

mkdir -p "$REPORT_DIR"

STATUS_FMT="not_run"
STATUS_TEST="not_run"
STATUS_NODE="not_run"

if [[ "$SKIP_GATES" != "true" ]]; then
  if (cd "$ROOT_DIR" && cargo fmt --check); then
    STATUS_FMT="pass"
  else
    STATUS_FMT="fail"
  fi

  if (cd "$ROOT_DIR" && cargo test); then
    STATUS_TEST="pass"
  else
    STATUS_TEST="fail"
  fi

  if (cd "$ROOT_DIR" && node npm/testing/test-platform-detection.js); then
    STATUS_NODE="pass"
  else
    STATUS_NODE="fail"
  fi
fi

cat > "$REPORT_PATH" <<EOF
# Setup/Monitor Manual Verification Report

- Generated at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")
- Repository: $ROOT_DIR

## Automated preflight

- cargo fmt --check: $STATUS_FMT
- cargo test: $STATUS_TEST
- node npm/testing/test-platform-detection.js: $STATUS_NODE

## Manual scenario checklist

Run these in ACP client (e.g., Zed Agent Panel) using the same workspace.

1. Run \`/setup\` and confirm setup wizard text + Plan panel appears.
2. Ask ACP to output a markdown file link to a known local file in this workspace, click the link, and confirm it opens the intended absolute path without a \`-50\` error.
3. Run \`/status\` then check Plan step \`Verify: run /status, /monitor, and /vector\` changes from pending to in_progress.
4. While the task is still running, confirm ACP shows live plan progress in at least one visible surface:
   - Zed: Plan panel rows update immediately.
   - Non-Zed ACP client: agent text includes \`Plan update: ...\`, \`Current: ...\`, and optional \`Note: ...\`.
5. Run \`/monitor\` and confirm output includes:
   - \`Task monitoring: orchestration=..., monitor=..., vector_checks=...\`
   - \`Task queue: ...\`
6. Run \`/vector\` and confirm the same Plan verify step becomes completed.
7. Finish a prompt that triggers at least one tool call or exec step, wait for the final \`completed\` message, and confirm ACP leaves processing state promptly instead of spinning indefinitely.
8. Open Config Options and change one option among:
   - Model / Reasoning Effort / Approval Preset
   - Task Orchestration / Task Monitoring / Progress Vector Checks
   Confirm Plan progress updates immediately.
9. Set \`Task Orchestration\` to \`sequential\`, start one task, then send another prompt.
   Confirm sequential wait guidance appears instead of submitting a parallel task.
10. Inspect logs:
   - \`logs/codex_chats/.../*.md\` contains Plan/ToolCall/RequestPermission traces.
   - Optional: \`ACP_HOME/sessions/<id>/canonical.jsonl\` contains \`acp.plan\` updates.

## Result summary

- Automated preflight overall: $( [[ "$STATUS_FMT" == "pass" && "$STATUS_TEST" == "pass" && "$STATUS_NODE" == "pass" ]] && echo "pass" || { [[ "$SKIP_GATES" == "true" ]] && echo "skipped"; [[ "$SKIP_GATES" != "true" ]] && echo "check-failures"; } )
- Manual checks: pending (fill during execution)
EOF

echo "Generated report: $REPORT_PATH"
if [[ "$SKIP_GATES" != "true" ]]; then
  if [[ "$STATUS_FMT" != "pass" || "$STATUS_TEST" != "pass" || "$STATUS_NODE" != "pass" ]]; then
    echo "One or more preflight checks failed. See report: $REPORT_PATH" >&2
    exit 2
  fi
fi

exit 0
