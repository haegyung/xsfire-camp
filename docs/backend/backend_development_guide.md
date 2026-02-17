# Backend Development Guide

This document is a practical guide for adding or upgrading backend drivers in `xsfire-camp`.
It is intended for contributors who need clear implementation and verification criteria.

## Goals

- Keep ACP surface behavior stable.
- Preserve backend-specific strengths where possible.
- Make cross-backend continuity predictable through canonical logging.
- Avoid regressions in codex backend parity while expanding other backends.

## Current State

`--backend` values:

- `codex`: full ACP-oriented path (sessions, load/list, approvals, tool call updates, plan updates, richer streaming)
- `claude-code`: minimal one-shot CLI adapter (in-memory sessions, no load, no tool/approval bridge)
- `gemini`: minimal one-shot CLI adapter (in-memory sessions, no load, no tool/approval bridge)

See `docs/backend/backends.md` for high-level status.

## Runtime Path (Codex Reference Flow)

Use this sequence as the reference when implementing feature parity:

1. `new_session`
- ACP client sends `new_session`.
- `src/acp_agent.rs` delegates to backend driver.
- Driver creates backend session and registers session root (`cwd`) if filesystem operations are supported.

2. `prompt`
- ACP client sends `prompt`.
- Driver converts ACP prompt blocks into backend input.
- Backend executes turn and emits stream events.

3. `tool_call`
- Tool lifecycle updates are translated to ACP `SessionUpdate` events.
- File operations respect session root boundary (deny access outside root).

4. `approval`
- Risky actions are surfaced via ACP `RequestPermission`.
- User decision is mapped back to backend-specific continuation.

5. `log`
- Canonical event stream is appended (`acp`-level and backend-level signals).
- Sensitive token patterns are redacted before write.

## Driver Contract

All backends implement `BackendDriver` in `src/backend.rs`.
Minimum methods required for a usable backend:

- `auth_methods`
- `authenticate`
- `new_session`
- `list_sessions`
- `prompt`
- `cancel`

Parity-grade backend should additionally support:

- `load_session`
- `set_session_mode`
- `set_session_model`
- `set_session_config_option`

Rule: if unsupported, return explicit `invalid_params` with actionable message.

## Implementation Checklist (New Backend)

1. Add `BackendKind` mapping and CLI flag support.
2. Implement `BackendDriver` in a dedicated file under `src/`.
3. Wire driver selection in `src/lib.rs`.
4. Define backend auth strategy:
- pre-authenticated external CLI, or
- native login flow mapped into ACP `authenticate`.
5. Implement session lifecycle:
- stable session id format
- list behavior
- load behavior (if supported)
6. Implement prompt conversion from ACP blocks:
- text
- resource links
- embedded text resources
- fallback placeholders for unsupported media
7. Implement output streaming strategy:
- chunked updates if backend supports streaming
- otherwise one-shot chunk with clear limitations in docs
8. Integrate approval path when backend exposes actionable approvals.
9. Integrate canonical logging hooks for major state changes.
10. Add/update docs (`docs/backend/backends.md`, this guide, release notes if needed).

## Safety Requirements

- Enforce session-root boundary for backend file read/write hooks.
- Never log raw API keys; rely on redaction layer in `src/session_store.rs`.
- Keep logging best-effort (logging failure must not fail prompt execution).
- Preserve explicit error messages for unsupported operations.

## Canonical Logging Requirements

At minimum, log:

- prompt summary event
- backend submission/start marker
- agent message chunks
- tool call and status updates (if available)
- permission request and response
- plan updates (if available)

For non-parity backends, log what is available and document gaps explicitly.

## Testing Checklist

1. Unit tests:
- prompt argument parsing and expansion (`src/prompt_args.rs`)
- backend-specific helpers where pure logic exists

2. Behavior tests (manual or integration):
- `new_session` returns valid session id
- `list_sessions` includes created session
- `prompt` returns ACP-visible output
- unsupported APIs return explicit errors (not silent no-op)

3. Safety tests:
- session root boundary blocks out-of-root file access
- canonical log redaction masks token-like secrets

4. Regression checks:
- `cargo test`
- backend smoke run with expected env vars and local CLI availability

## Definition Of Done

A backend upgrade is done when:

1. Driver behavior is documented in `docs/backend/backends.md`.
2. Unsupported capabilities are explicit and intentional.
3. Required tests/checks pass.
4. No codex backend regression is introduced.
5. Canonical logging behavior is verified for the implemented event set.

## Suggested Next Targets

1. Add persistent session loading for `claude-code` and `gemini`.
2. Add streaming bridge (instead of single chunk output).
3. Add approval mediation where backend CLIs expose machine-readable prompts.
4. Normalize model/mode/config options across backends where semantics overlap.
