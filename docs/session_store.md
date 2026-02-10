# Global Session Store (Canonical Log)

This project keeps **backend-native session storage** as-is (for example, Codex uses `CODEX_HOME`),
and additionally writes a **backend-agnostic canonical work log** so you can continue work even if:

- you switch editors (Zed, VS Code, etc.)
- you switch threads
- you switch LLM backends in the future (Codex, Claude Code, Gemini CLI, ...)

The canonical log is designed to capture "what happened" (prompt summary, tool calls, plan updates,
approvals) without forcing different vendors to share the same native session format.

## Location

By default the canonical store lives at:

- `~/.acp/`

Override with:

- `ACP_HOME=/path/to/acp-home`

## Layout

- `~/.acp/index.json`
  - Maps a backend session key to a global session id.
  - Example key: `codex:<session_id>`
- `~/.acp/sessions/<global_session_id>/state.json`
  - Minimal session metadata snapshot (backend, ids, cwd, created time).
- `~/.acp/sessions/<global_session_id>/canonical.jsonl`
  - Append-only JSON Lines of canonical events.

## What Gets Logged

Best-effort (logging must not break the agent):

- `acp.prompt`: prompt summary (text blocks, resource links, embedded context refs, image/audio counts)
- `backend.codex.submit`: submission id + coarse op kind (user_input, review, compact, ...)
- `acp.agent_message_chunk`, `acp.agent_thought_chunk`
- `acp.tool_call`, `acp.tool_call_update`
- `acp.plan`
- `acp.request_permission`, `acp.request_permission_response`

## Embedded Context Logging

By default, the canonical store **does not duplicate embedded file contents** (it logs only the URI
and length). This reduces the risk of re-storing secrets in multiple places.

If you explicitly want embedded text resources to be included:

- `ACP_LOG_EMBEDDED_CONTEXT=1`

To limit log size for prompt summaries:

- `ACP_LOG_MAX_TEXT_CHARS=16384` (default: 16384)

## Why A Global Store Helps (Even If Each Backend Has Its Own Logs)

Think of it as a **translator layer**:

- Each backend (Codex, Claude Code, Gemini CLI) keeps its own native logs and features.
- The adapter translates those events into a canonical, searchable timeline.

This is the piece that lets you preserve the full work context across models and tools, without
pretending all vendors share the same session database.

