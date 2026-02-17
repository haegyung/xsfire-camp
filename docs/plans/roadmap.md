# Roadmap

This document describes where this project is heading.

## North Star

Keep **work context continuous** even if you switch:

- editors (Zed, VS Code, ...)
- threads
- LLM backends (Codex, Claude Code, Gemini CLI, ...)

…while preserving each backend's unique capabilities (tool calls, approvals, and file edits), not
flattening them into “chat only”.

## Now (Shipped)

- Codex CLI as an ACP agent (stdio) with session continuity via shared `CODEX_HOME`.
- Compatibility with community VS Code ACP extensions that invoke agents as `<command> acp` or with
  `--acp` (no-ops).
- Global canonical session store under `ACP_HOME` (default `~/.acp`) that records a structured
  timeline (prompt summary, tool calls, plan, approvals). See `docs/backend/session_store.md`.

## Next (Near Term)

- Backend driver interface
  - Factor an internal “driver” trait so each backend can implement:
    - prompt submission
    - streamed events (tool calls, plan, terminal)
    - approvals / permission gating
    - session list/load/replay (if supported)
  - Keep the ACP surface stable while swapping drivers.
- VS Code support hardening
  - Collect and document behavior differences across community “VSCode ACP” extensions (agent
    invocation, env propagation, cwd behavior, streaming UI quirks).
  - Provide a reference configuration and smoke checklist.
- Canonical log schema tightening
  - Versioned event types and minimal required fields.
  - Better correlation IDs across: prompt -> tool calls -> approvals -> file changes.

## Later (Medium Term)

- Add real backends (CLI-first)
  - Claude Code (stream-json output, externalized permission prompts)
  - Gemini CLI (stream-json event model, approval modes)
- “Translator” quality improvements
  - Map backend-specific tool/approval models into ACP events without losing key information.
  - Ensure canonical logs remain useful even when backends differ in granularity.
- Security and compliance
  - Expand redaction beyond basic token patterns.
  - Policy knobs for excluding certain event types or payload fields from canonical logs.

## Non-Goals (For Now)

- Forcing different vendors to share one native session store format.
- Building a generic “chat only” bridge that drops tool calls/approvals.

