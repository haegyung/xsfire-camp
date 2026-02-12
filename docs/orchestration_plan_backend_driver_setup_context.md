# Work Orchestration Plan (R->P->M->W->A)

## Goal / Constraints / Done
- Goal: Implement (1) backend driver interface, (2) Plan-based setup wizard UX, and (3) context optimization/telemetry hardening, while keeping ACP surface stable and preserving slash-command parity.
- Constraints:
  - No external ACP API shape changes.
  - Keep existing slash-command behavior (including `test_slash_command_smoke_flow`).
  - Changes must be verifiable via commands and/or unit tests.
  - Parallelize implementation work where non-overlapping, but merge in a strictly ordered, sequence-dependent plan.
- Done criteria (Rubric):
  - Build/format/tests:
    - `cargo fmt --check` passes.
    - `cargo test` passes.
    - `node npm/testing/test-platform-detection.js` passes.
  - (1) Backend driver interface:
    - Driver trait exists and is used for backend selection (Codex is the default driver).
    - `--backend=codex` behavior unchanged.
    - Non-codex backends fail with a clear, actionable message (until implemented).
  - (2) Setup wizard UX:
    - `/setup` emits a Plan panel “wizard” (steps + completion states) in addition to minimal text.
    - Unit test asserts `/setup` produces a `SessionUpdate::Plan` with expected step labels.
  - (3) Context optimization/telemetry:
    - Plan updates log their `explanation` into canonical logs (when present).
    - Unit test asserts canonical log includes correlated events across: `acp.prompt` -> `acp.plan` -> `acp.request_permission` / `acp.request_permission_response` -> `acp.tool_call`.

## Plan
R0: Ship driver boundary + setup wizard plan UX + context telemetry hardening (depends_on: none)

P1: Baseline + Safety Gate (depends_on: R0)
M1.1: Establish green baseline on current HEAD (depends_on: P1)
W1.1.a: Run baseline checks (depends_on: M1.1)
A1.1.a.1: Run `cargo test` (depends_on: W1.1.a)
Definition: Confirm existing behavior is green before refactors.
Output: Test run result.
Done: `cargo test` exits 0.
Prerequisites: none

A1.1.a.2: Run `node npm/testing/test-platform-detection.js` (depends_on: W1.1.a)
Definition: Confirm npm wrapper tests are green before refactors.
Output: Node test run result.
Done: Script exits 0.
Prerequisites: none

P2: Backend Driver Interface (1) (depends_on: P1)
M2.1: Introduce backend driver trait and route agent through it (depends_on: P2)
W2.1.a: Add driver trait + ACP agent wrapper (depends_on: M2.1)
A2.1.a.1: Add `BackendDriver` trait and `AcpAgent` delegator (depends_on: W2.1.a)
Definition: Create a driver boundary without changing ACP request/response shapes.
Output: New modules + refactor wired in `src/lib.rs`.
Done: `cargo test` passes locally after wiring.
Prerequisites: A1.1.a.1

A2.1.a.2: Move Codex implementation to `CodexDriver` and implement `BackendDriver` (depends_on: W2.1.a)
Definition: Codex backend implements the driver interface and preserves behavior.
Output: Refactored Codex backend code.
Done: `cargo test` passes and `/sessions`, `/load`, slash commands still work in tests.
Prerequisites: A2.1.a.1

M2.2: Backend selection is explicit and stable (depends_on: M2.1)
W2.2.a: Wire `--backend` selection end-to-end (depends_on: M2.2)
A2.2.a.1: Pass backend kind into `run_main` and select driver (depends_on: W2.2.a)
Definition: Ensure backend selection is routed through the driver boundary.
Output: Updated `src/main.rs` + `src/lib.rs` wiring.
Done: `--backend=codex` runs; non-codex yields clear error on session creation.
Prerequisites: A2.1.a.2

P3: Setup Wizard UX (2) (depends_on: P2)
M3.1: `/setup` emits a Plan-based wizard (depends_on: P3)
W3.1.a: Implement Plan emission + completion heuristics (depends_on: M3.1)
A3.1.a.1: Update `/setup` handler to publish a plan (depends_on: W3.1.a)
Definition: Provide a multi-step wizard via `SessionUpdate::Plan`.
Output: `/setup` emits plan items with statuses.
Done: Unit test confirms Plan update emitted.
Prerequisites: A2.2.a.1

A3.1.a.2: Add unit test for `/setup` plan emission (depends_on: W3.1.a)
Definition: Lock UX behavior into tests to prevent regressions.
Output: New test in `src/thread.rs`.
Done: `cargo test` passes including the new test.
Prerequisites: A3.1.a.1

P4: Context Optimization/Telemetry Hardening (3) (depends_on: P3)
M4.1: Canonical log captures plan explanations and correlation path (depends_on: P4)
W4.1.a: Persist plan explanation + add correlation test (depends_on: M4.1)
A4.1.a.1: Include `explanation` in canonical `acp.plan` logs (depends_on: W4.1.a)
Definition: Ensure plan context is preserved for future replay/analysis.
Output: Updated canonical logging for plan updates.
Done: Unit test asserts explanation is present when provided.
Prerequisites: A3.1.a.2

A4.1.a.2: Add correlation test spanning prompt/plan/permission/tool-call logs (depends_on: W4.1.a)
Definition: Prove correlation continuity across key event types.
Output: New unit test writing to temp `ACP_HOME` and asserting event linkage.
Done: `cargo test` passes and test asserts the correlation invariants.
Prerequisites: A4.1.a.1

P5: Quality Gate + Documentation Update (depends_on: P4)
M5.1: Quality gates green + docs updated (depends_on: P5)
W5.1.a: Run format/tests and update docs (depends_on: M5.1)
A5.1.a.1: Run `cargo fmt --check` and `cargo test` (depends_on: W5.1.a)
Definition: Enforce repo coding style and regression coverage.
Output: Green formatting + test runs.
Done: Both commands exit 0.
Prerequisites: A4.1.a.2

A5.1.a.2: Update docs to reflect driver boundary and wizard UX (depends_on: W5.1.a)
Definition: Keep repo SoT aligned with shipped behavior.
Output: Updated `docs/backends.md` and/or `README.md` notes.
Done: Docs mention driver interface and `/setup` Plan wizard behavior.
Prerequisites: A5.1.a.1

## Roles
- Role split:
  - Driver refactor: backend trait + Codex driver wiring.
  - UX wizard: `/setup` plan emission + tests.
  - Telemetry hardening: canonical logging and correlation test.
- Feedback checkpoints:
  - After P2: `cargo test` must be green before starting P3 merge.
  - After P3: `/setup` test must be green before starting P4 merge.
  - After P4: correlation test must be green before final quality gate.

## Context Snapshot
- Project: `xsfire-camp`
- Branch: `main`
- Active files: `src/lib.rs`, `src/main.rs`, `src/codex_agent.rs`, `src/thread.rs`, `docs/orchestration_plan_backend_driver_setup_context.md`
- Mode: local dev, tests-first, no external API changes
- State: baseline tests currently pass; backend flag exists but only codex supported

## Risks / Next Step
- Risks:
  - Driver refactor could subtly break session lifecycle or capability reporting.
  - Async trait objects / Rc vs Arc mismatches could cause compile/runtime issues.
  - Tests that mutate env (`ACP_HOME`) can be flaky without serialization.
  - “Parallel” changes landing out of order can increase integration risk.
- Next step:
  - Execute P1 then start P2 refactor with tight compile/test loops.
