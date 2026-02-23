# xsfire-camp

ACP(Agent Client Protocol) 클라이언트에서 Codex CLI를 실행형 에이전트로 연결하는 브리지입니다.

`xsfire-camp` lets ACP-compatible clients (for example Zed) run Codex CLI as an execution-first agent session.

- ACP: https://agentclientprotocol.com/
- Codex: https://github.com/openai/codex
- Project purpose: keep session continuity across IDE and CLI with structured logs and approval flow

## Quick Start (60 sec)

1. Build
```bash
cargo build --release
```

2. Run
```bash
OPENAI_API_KEY=... CODEX_HOME="$HOME/.codex" target/release/xsfire-camp
```

3. Optional: multi-backend mode
```bash
target/release/xsfire-camp --backend=multi
```

4. Verify
```bash
cargo test
node npm/testing/test-platform-detection.js
```

## Who This Is For

- ACP client users who want stable Codex behavior independent of client adapter changes
- Teams that need traceable tool/plan/approval logs for review and safety
- Operators who want to preserve context across terminal and IDE sessions

## Prerequisites

| Item | Required | Notes |
| --- | --- | --- |
| Rust toolchain | Yes (build from source) | For `cargo build --release` |
| ACP client (example: Zed) | Yes | Must support stdio ACP agent |
| Auth (`OPENAI_API_KEY` or `CODEX_API_KEY`) | Yes (Codex backend) | Depends on backend/auth route |
| `CODEX_HOME` | Recommended | Session/thread continuity root |
| `ACP_HOME` | Optional | Canonical ACP log root (default `~/.acp`) |

## Run Modes

```bash
target/release/xsfire-camp --backend=codex
target/release/xsfire-camp --backend=claude-code
target/release/xsfire-camp --backend=gemini
target/release/xsfire-camp --backend=multi
```

Notes:
- `claude-code` and `gemini` backends require their CLIs to be installed and authenticated.
- In `multi` mode, switch backend in-thread: `/backend codex|claude-code|gemini`.
- Backend-specific overrides:
  - `XSFIRE_CLAUDE_BIN`, `XSFIRE_CLAUDE_ARGS`
  - `XSFIRE_GEMINI_BIN`, `XSFIRE_GEMINI_ARGS`, `XSFIRE_GEMINI_APPROVAL_MODE`

## Common Commands Snapshot

| Category | Commands |
| --- | --- |
| Core | `/setup`, `/review`, `/review-branch`, `/review-commit`, `/compact`, `/undo`, `/init`, `/status` |
| Session | `/sessions`, `/load` |
| Integrations | `/mcp`, `/skills` |
| Monitoring | `/monitor`, `/monitor retro`, `/vector`, `/experimental` |
| UX | `/new-window` |

## Client Integration

### Zed custom agent registration

`settings.json` example:

```json
{
  "agent_servers": {
    "xsfire-camp": {
      "type": "custom",
      "command": "/absolute/path/to/xsfire-camp",
      "env": {
        "CODEX_HOME": "/Users/you/.codex"
      }
    }
  }
}
```

### VS Code notes

This repository does not ship a VS Code ACP extension. Use a VS Code ACP client extension that can run a stdio custom agent.

Compatibility note:

```bash
xsfire-camp acp
```

If the extension resolves agents from PATH, expose command via npm:

```bash
npm i -g @haegyung/xsfire-camp
```

## npm Package

```bash
npx @haegyung/xsfire-camp
```

Package:
- Base: `@haegyung/xsfire-camp`
- Platform optional dependencies:
  - `@haegyung/xsfire-camp-darwin-arm64`
  - `@haegyung/xsfire-camp-darwin-x64`
  - `@haegyung/xsfire-camp-linux-arm64`
  - `@haegyung/xsfire-camp-linux-x64`
  - `@haegyung/xsfire-camp-win32-arm64`
  - `@haegyung/xsfire-camp-win32-x64`

## Troubleshooting (Top 5)

1. Auth error on startup
- Check `OPENAI_API_KEY` or `CODEX_API_KEY` is set for Codex backend.

2. Sessions not shared between CLI and ACP client
- Ensure both run with the same `CODEX_HOME`.

3. Backend switch fails
- Confirm target backend CLI (`claude`/`gemini`) is installed and authenticated.

4. npm package not found
- Check latest release workflow and npm publish auth (`NPM_TOKEN` or trusted publishing).

5. Zed community extension not visible yet
- Registry PR may still be open or waiting for maintainer merge queue.

## Docs Index

### Architecture and backend
- `docs/backend/backends.md`
- `docs/backend/session_store.md`
- `docs/backend/policies.md`
- `docs/reference/acp_standard_spec.md`
- `docs/reference/event_handling.md`
- `docs/reference/codex_home_overview.md`

### Planning and roadmap
- `docs/plans/roadmap.md`
- `docs/plans/next_cycle_execution_plan.md`
- `docs/plans/orchestration_plan_backend_driver_setup_context.md`

### Quality and release
- `docs/quality/verification_guidance.md`
- `docs/quality/qa_checklist.md`
- `docs/releases/release_notes_v0.9.8.md`
- `docs/releases/release_notes_v0.9.10.md`
- `docs/releases/release_notes_v0.9.11.md`
- `docs/releases/release_notes_v0.9.12.md`
- `docs/releases/release_notes_v0.9.13.md`
- `docs/releases/release_notes_v0.9.14.md`

### Zed extension
- `docs/zed/zed_extension_pr_template.md`
- `docs/zed/extensions_toml_sample.md`
- `docs/zed/install_shared_settings.md`

## README Update Checklist (for each release)

1. Version strings and examples align with the current release.
2. Quick Start command flags still match binary behavior.
3. Verification commands are still valid.
4. `docs/` links are alive and accurate.
5. npm and Zed registry status text is current.
6. New release notes link is added.

## English Summary

`xsfire-camp` is an ACP bridge around Codex CLI.
It focuses on:
- execution-first sessions instead of plain chat,
- continuity across IDE and terminal via shared `CODEX_HOME`,
- traceable tool/plan/approval updates,
- explicit control gates for risky operations.

If you are new, start from **Quick Start**, then jump to **Docs Index**.

## License

CC BY-NC-SA 4.0. See `LICENSE`.
