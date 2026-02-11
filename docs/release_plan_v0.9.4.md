# v0.9.4 Release Plan

Scope: reinforce ACP slash command parity and make local repo artifacts less noisy, without changing external behavior.

## Status (2026-02-11)
- Completed.
- Verification run:
  - `cargo test` -> `20 passed; 0 failed` (includes `thread::tests::test_slash_command_smoke_flow`)
  - `node npm/testing/test-platform-detection.js` -> all platform detection tests passed
- Repo hygiene check:
  - `.gitignore` includes `logs` and `.DS_Store`
  - no tracked files under `logs/`
  - no tracked `.DS_Store` files
- Traceability:
  - status update commit: `afc3190` (`chore: update v0.9.4 release plan status`)
  - release tags: `v0.9.4` -> `43bbe0e`, `v0.9.5` -> `2cf1842`

## Goals
- Add a scenario-based smoke test that chains common slash commands in one session.
- Keep local artifacts (`.DS_Store`, `logs/`) out of the repo history and reduce accidental noise during development.

## Work Items
- Tests:
  - Add `test_slash_command_smoke_flow` to `src/thread.rs` to validate a basic `/init` -> prompt -> `/review` -> `/compact` flow.
- Repo hygiene:
  - Remove stray `.DS_Store` files inside the repo.
  - Ensure `logs/` stays ignored and contains only local/dev artifacts.

## Non-Goals
- No public API changes.
- No behavior changes to command semantics; only coverage and hygiene.
