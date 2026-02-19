# Release Notes — v0.9.13

## Summary

- Added multi-backend session routing (`codex`, `claude-code`, `gemini`) under `--backend=multi`.
- Added backend switching from the same thread via `/backend ...` and session config option `backend`.
- Expanded Claude/Gemini session controls: `model` config option, `/status`, `/model`, `/help`, `/reset`.
- Enabled canonical session logging by default for Claude/Gemini backends (same `ACP_HOME` flow used by Codex).

## Details

- Multi backend driver:
  - New `src/multi_backend.rs` routes ACP requests per session and preserves active backend state.
  - Auth dispatch is method-id based:
    - Codex: `chatgpt`, `codex-api-key`, `openai-api-key`
    - Claude: `claude-cli`
    - Gemini: `gemini-cli`
- Config/UI behavior:
  - `backend` option is injected in multi mode and updates immediately.
  - Active backend config options are preserved when switching backend in config UI.
- Logging:
  - Claude/Gemini now initialize `SessionStore` and append canonical events (`acp.prompt`, `acp.agent_message_chunk`) by default when `ACP_HOME` is resolvable.

## Verification

- `cargo test` passes.
- `scripts/build_and_install.sh` completes and updates local binary.

## Release

- Tag: `v0.9.13`
- GitHub Release: `https://github.com/haegyung/xsfire-camp/releases/tag/v0.9.13`
