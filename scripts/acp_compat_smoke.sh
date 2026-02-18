#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPORT_DIR="$ROOT_DIR/logs/smoke"
TIMESTAMP="$(date +"%Y%m%d_%H%M%S")"
REPORT_PATH="$REPORT_DIR/acp_compat_smoke_${TIMESTAMP}.md"

usage() {
  cat <<'EOF'
Usage: scripts/acp_compat_smoke.sh [--skip-tests]

Runs ACP compatibility smoke checks and writes a markdown report under logs/smoke/.

Options:
  --skip-tests   Run static checks only (skip cargo test commands)
  -h, --help     Show this help message
EOF
}

SKIP_TESTS="false"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-tests)
      SKIP_TESTS="true"
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

STATIC_FAILURES=0
TEST_FAILURES=0
declare -a STATIC_LINES
declare -a TEST_LINES
declare -a DETAIL_LINES

run_rg_present() {
  local label="$1"
  local pattern="$2"
  local file="$3"
  if (cd "$ROOT_DIR" && rg -n --no-messages -- "$pattern" "$file" >/dev/null); then
    STATIC_LINES+=("- ${label}: pass")
  else
    STATIC_LINES+=("- ${label}: fail")
    DETAIL_LINES+=("${label}: expected pattern '${pattern}' in ${file}")
    STATIC_FAILURES=1
  fi
}

run_rg_absent() {
  local label="$1"
  local pattern="$2"
  local file="$3"
  if (cd "$ROOT_DIR" && rg -n --no-messages -- "$pattern" "$file" >/dev/null); then
    STATIC_LINES+=("- ${label}: fail")
    DETAIL_LINES+=("${label}: unexpected pattern '${pattern}' found in ${file}")
    STATIC_FAILURES=1
  else
    STATIC_LINES+=("- ${label}: pass")
  fi
}

run_test_command() {
  local label="$1"
  local command="$2"
  if (cd "$ROOT_DIR" && bash -lc "$command"); then
    TEST_LINES+=("- ${label}: pass")
  else
    TEST_LINES+=("- ${label}: fail")
    DETAIL_LINES+=("${label}: command failed -> ${command}")
    TEST_FAILURES=1
  fi
}

# ACP initialize/capability contract checks.
run_rg_present "initialize forces protocol v1" \
  "ProtocolVersion::V1" \
  "src/acp_agent.rs"
run_rg_present "prompt capabilities advertise embedded context + image" \
  "PromptCapabilities::new\\(\\)\\.embedded_context\\(true\\)\\.image\\(true\\)" \
  "src/acp_agent.rs"
run_rg_present "MCP capabilities advertise HTTP transport" \
  "McpCapabilities::new\\(\\)\\.http\\(true\\)" \
  "src/acp_agent.rs"
run_rg_absent "MCP SSE is not advertised as enabled" \
  "\\.sse\\(true\\)" \
  "src/acp_agent.rs"
run_rg_present "session list capability is advertised" \
  "SessionCapabilities::new\\(\\)\\.list\\(SessionListCapabilities::new\\(\\)\\)" \
  "src/acp_agent.rs"
run_rg_present "load_session is gated to codex backend" \
  "backend_kind\\(\\) == BackendKind::Codex" \
  "src/acp_agent.rs"

# Event stream and progress/plan pathways.
run_rg_present "plan updates are emitted to ACP notifications" \
  "client\\.update_plan\\(plan, explanation\\)\\.await;" \
  "src/thread.rs"
run_rg_present "tool call updates are emitted" \
  "SessionUpdate::ToolCallUpdate" \
  "src/thread.rs"
run_rg_present "permission request is canonically logged" \
  "\"acp.request_permission\"" \
  "src/thread.rs"
run_rg_present "permission response is canonically logged" \
  "\"acp.request_permission_response\"" \
  "src/thread.rs"

# Backend-specific compatibility expectations.
run_rg_present "claude load_session unsupported contract" \
  "load_session is not supported for --backend=claude-code yet" \
  "src/claude_code_agent.rs"
run_rg_present "claude set_session_model unsupported contract" \
  "set_session_model is not supported for --backend=claude-code yet" \
  "src/claude_code_agent.rs"
run_rg_present "claude set_session_config_option unsupported contract" \
  "set_session_config_option is not supported for --backend=claude-code yet" \
  "src/claude_code_agent.rs"
run_rg_present "gemini load_session unsupported contract" \
  "load_session is not supported for --backend=gemini yet" \
  "src/gemini_agent.rs"
run_rg_present "gemini set_session_model unsupported contract" \
  "set_session_model is not supported for --backend=gemini yet" \
  "src/gemini_agent.rs"
run_rg_present "gemini set_session_config_option unsupported contract" \
  "set_session_config_option is not supported for --backend=gemini yet" \
  "src/gemini_agent.rs"

if [[ "$SKIP_TESTS" == "true" ]]; then
  TEST_LINES+=("- cargo tests: skipped (--skip-tests)")
else
  run_test_command "cargo test -q thread::tests::" \
    "cargo test -q thread::tests::"
  run_test_command \
    "cargo test -q session_store::tests::writes_canonical_log_and_redacts_secrets" \
    "cargo test -q session_store::tests::writes_canonical_log_and_redacts_secrets"
fi

if [[ "$SKIP_TESTS" == "true" ]]; then
  if [[ $STATIC_FAILURES -eq 0 ]]; then
    OVERALL_STATUS="pass (static-only)"
  else
    OVERALL_STATUS="check-failures"
  fi
else
  if [[ $STATIC_FAILURES -eq 0 && $TEST_FAILURES -eq 0 ]]; then
    OVERALL_STATUS="pass"
  else
    OVERALL_STATUS="check-failures"
  fi
fi

{
  echo "# ACP Compatibility Smoke Report"
  echo
  echo "- Generated at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo "- Repository: $ROOT_DIR"
  echo "- Script: scripts/acp_compat_smoke.sh"
  echo "- Skip tests: $SKIP_TESTS"
  echo
  echo "## Static checks"
  for line in "${STATIC_LINES[@]}"; do
    echo "$line"
  done
  echo
  echo "## Test checks"
  for line in "${TEST_LINES[@]}"; do
    echo "$line"
  done
  if [[ ${#DETAIL_LINES[@]} -gt 0 ]]; then
    echo
    echo "## Failure details"
    for detail in "${DETAIL_LINES[@]}"; do
      echo "- $detail"
    done
  fi
  echo
  echo "## Result summary"
  echo "- Overall: $OVERALL_STATUS"
} > "$REPORT_PATH"

echo "Generated report: $REPORT_PATH"

if [[ "$OVERALL_STATUS" != "pass" && "$OVERALL_STATUS" != "pass (static-only)" ]]; then
  echo "ACP smoke check failed. See report: $REPORT_PATH" >&2
  exit 2
fi

exit 0
