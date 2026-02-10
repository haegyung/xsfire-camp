# Backends (Roadmap)

This repository currently implements an ACP adapter around the Codex CLI (codex-rs).

## Goal

Support additional CLI-first coding agents (for example Claude Code and Gemini CLI) while preserving:

- ACP tool calls and streaming updates (Plan, ToolCall, Terminal)
- Approval / permission gating
- Session continuity across editor and CLI

## Non-goals (for now)

- Implementing a generic “chat-only” bridge that drops tool calls/approvals.
- Mixing session storage across vendors. Backend-specific session storage is acceptable.

## Proposed shape

- Add `--backend codex|claude-code|gemini` (default: `codex`).
- Factor an internal “backend driver” interface so each CLI backend can provide:
  - prompt submission
  - streamed events (tool calls, plan updates, terminal output)
  - approvals
  - session list/load/replay (if supported)

## Storage

Codex uses `CODEX_HOME`. For other backends, use a backend-specific home directory (for example `~/.acp/<backend>` or an env var like `<BACKEND>_HOME`) so sessions do not collide and behavior stays predictable.

In addition, this project can write a backend-agnostic canonical log under `ACP_HOME` (default:
`~/.acp`). This enables cross-backend continuity without forcing vendors to share one native session
format. See `docs/session_store.md`.
