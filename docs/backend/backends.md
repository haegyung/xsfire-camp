# Backends

This repository implements an ACP adapter around the Codex CLI (codex-rs).
It also includes a backend driver boundary so we can add other CLI-first agents later without changing the ACP surface.

Detailed implementation guidance:

- `docs/backend/backend_development_guide.md`

## CLI Flag

- `--backend=codex|claude-code|gemini` (default: `codex`)

Backend status:

- `codex`: full ACP parity via codex-rs (sessions, approvals, tool calls, plan updates)
- `claude-code`: minimal driver via the `claude` CLI (`claude --print --cwd <cwd> <prompt>`)
- `gemini`: minimal driver via the `gemini` CLI (`gemini --output-format text --approval-mode <mode> --prompt <prompt>`)

The minimal drivers currently:

- keep sessions in-memory (no `load_session` yet)
- stream the response as a single ACP `AgentMessageChunk`
- do not bridge tool calls/approvals yet (they rely on the CLI being non-interactive)

Env overrides:

- `XSFIRE_CLAUDE_BIN` / `XSFIRE_CLAUDE_ARGS`
- `XSFIRE_GEMINI_BIN` / `XSFIRE_GEMINI_ARGS` / `XSFIRE_GEMINI_APPROVAL_MODE` (default: `plan`)

## Architecture

- `src/backend.rs`: `BackendKind` + `BackendDriver` trait (ACP method surface)
- `src/acp_agent.rs`: `AcpAgent` implements `agent_client_protocol::Agent` and delegates to a `BackendDriver`
- `src/codex_agent.rs`: `CodexDriver` (Codex CLI implementation)
- `src/claude_code_agent.rs`: `ClaudeCodeDriver` (Claude Code CLI, minimal)
- `src/gemini_agent.rs`: `GeminiCliDriver` (Gemini CLI, minimal)
- `src/cli_common.rs`: prompt formatting + common notification helpers

This keeps the ACP request/response shapes stable while allowing internal backend selection.

## Storage

Codex uses `CODEX_HOME`. For other backends, use a backend-specific home directory (for example `~/.acp/<backend>` or an env var like `<BACKEND>_HOME`) so sessions do not collide and behavior stays predictable.

In addition, this project can write a backend-agnostic canonical log under `ACP_HOME` (default: `~/.acp`). This enables cross-backend continuity without forcing vendors to share one native session format. See `docs/backend/session_store.md`.

## Session Monitoring Defaults (codex backend)

For codex-backed sessions, task monitoring defaults are:

- `Task Orchestration`: `parallel`
- `Task Monitoring`: `auto` (optionally `on` or `off`)
- `Progress Vector Checks`: `on`

These are exposed as session config options and can be changed at runtime.
When `/setup` has been invoked, setup plan progress is refreshed immediately on config changes and on `/status`, `/monitor`, `/vector`.
