# Next Cycle Execution Plan (2026-02-11)

## Purpose
- Product purpose: keep ACP work context continuous across tools and backends.
- Task purpose: lock the next implementation cycle into one sequence-dependent path with testable outputs.
- Prototype contribution: preserve ACP surface while preparing internal backend-driver extraction.

## Constraints
- No external ACP API shape changes.
- Slash-command behavior parity must stay intact.
- All outputs must be directly verifiable by command or file diff.

## Done Criteria
- One selected near-term priority with rationale.
- Three acceptance criteria, all testable.
- Three-task implementation breakdown with first execution command.

## R->P->M->W->A

### R1
- Objective: start next implementation cycle in a single, unambiguous path.
- depends_on: `none`

### P1 Release Evidence Lock (completed)
- depends_on: `R1`
- Entry: release verification already run.
- Exit: evidence anchored in repo docs.

#### M1 Evidence documented (completed)
- depends_on: `P1`

##### W1 Release doc update (completed)
- depends_on: `M1`

###### A1 Write verification summary (completed)
- depends_on: `none`
- output: `docs/release_plan_v0.9.4.md` verification section
- done_check: `cargo test` and node test results listed
- owner: `Codex`

###### A2 Add traceability refs (completed)
- depends_on: `A1`
- output: commit/tag references in `docs/release_plan_v0.9.4.md`
- done_check: commit `afc3190`, tags `v0.9.4`, `v0.9.5` explicitly listed
- owner: `Codex`

### P2 Next Scope Selection (completed)
- depends_on: `P1`
- Entry: release evidence locked.
- Exit: one priority + acceptance criteria fixed.

#### M2 Near-term item selected (completed)
- depends_on: `P2`

##### W2 Scope and quality bar defined (completed)
- depends_on: `M2`

###### A3 Select one priority (completed)
- depends_on: `A2`
- output: selected item name
- done_check: selected item appears exactly once with rationale
- owner: `Codex`
- selected: `Backend driver interface`
- rationale: unlocks multi-backend expansion while keeping ACP surface stable.

###### A4 Write acceptance criteria (completed)
- depends_on: `A3`
- output: acceptance criteria list
- done_check: 3 criteria, each command/test verifiable
- owner: `Codex`
- criteria:
  1. Internal driver trait added for prompt submission and streamed event forwarding without ACP type changes.
  2. Slash-command parity preserved; `cargo test` remains green including `thread::tests::test_slash_command_smoke_flow`.
  3. At least one abstraction-level test confirms event correlation continuity (`prompt -> tool/plan/approval`).

### P3 Execution Readiness (completed)
- depends_on: `P2`
- Entry: priority and criteria fixed.
- Exit: first coding task can start without extra clarification.

#### M3 First implementation slice defined (completed)
- depends_on: `P3`

##### W3 Task breakdown and kickoff (completed)
- depends_on: `M3`

###### A5 Create three-task breakdown (completed)
- depends_on: `A4`
- output: 3-task worklist
- done_check: each task has deliverable + verification
- owner: `Codex`
- tasks:
  1. Extract driver boundary from current backend orchestration in `src/codex_agent.rs`.
     - verification: compile succeeds and ACP request/response types unchanged.
  2. Isolate event translation/mapping path to driver-owned layer.
     - verification: unit tests assert correlation IDs and event category continuity.
  3. Wire default Codex driver end-to-end and run regressions.
     - verification: `cargo test` passes including smoke flow.

###### A6 Fix kickoff command (completed)
- depends_on: `A5`
- output: start instruction
- done_check: first files + first command specified
- owner: `Codex`
- kickoff:
  - first files: `src/codex_agent.rs`, `src/thread.rs` (and `src/lib.rs` only if module wiring needed)
  - first command: `cargo test`

## Context Snapshot
- branch: `main`
- head: `afc3190` (pushed)
- release tags: `v0.9.4` (`43bbe0e`), `v0.9.5` (`2cf1842`)
- source references: `docs/roadmap.md`, `docs/release_plan_v0.9.4.md`, `src/codex_agent.rs`, `src/thread.rs`
