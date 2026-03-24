# Release Notes - v0.9.23

## Summary

- Added a new `Progress signals` panel to `/monitor` output for live bottleneck, repetition, and stall visibility.
- Wired active open-tool runtime tracking into monitoring so long-running tool calls can be attributed to a specific submission.
- Added regression coverage for repetition/stall heuristics and the new bottleneck snapshot rendering.

## Details

- `src/thread.rs` now includes:
  - monitor thresholds for slow bottlenecks, repetitive loops, and no-progress stalls
  - `FlowVectorState` progress-tracking fields (`last_plan_update_at`, `last_progress_at`, stalled update streak)
  - dynamic signal renderers:
    - `render_repeat_signal`
    - `render_stall_signal`
    - `render_progress_signals_snapshot`
  - longest-running open-tool-call lookup across active submissions
- `/monitor` output now renders the new `Progress signals` section before recent actions.
- Added/updated tests:
  - `thread::tests::test_monitor_command`
  - `thread::tests::test_flow_vector_repeat_and_stall_signals_show_stagnation`
  - `thread::tests::test_progress_signals_snapshot_reports_long_running_open_tool_call`

## Verification

- `cargo test -- --nocapture`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.23`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.23`

## Post-Release Verification Snapshot (2026-03-24T02:37:29Z)

- Product release workflow:
  - Run `23447088552` (`Release`, branch `v0.9.23`) completed with `success`.
  - URL: `https://github.com/theprometheusxyz/xsfire-camp/actions/runs/23447088552`
- ACP registry PR:
  - PR `#93` head is updated to commit `52cf9d6` (`v0.9.23` entry), and state remains `OPEN` with `mergeStateStatus=BLOCKED`.
  - URL: `https://github.com/agentclientprotocol/registry/pull/93`
- ACP registry checks:
  - `gh pr checks 93 --repo agentclientprotocol/registry` currently reports: `no checks reported on the 'add-xsfire-camp-agent' branch`.
  - Latest `Build Registry` run `23470404308` is `action_required`: `https://github.com/agentclientprotocol/registry/actions/runs/23470404308`
  - Maintainer re-run request comment: `https://github.com/agentclientprotocol/registry/pull/93#issuecomment-4114976120`
- Release asset integrity:
  - All 8 release archives matched expected SHA256 values.
  - Archive integrity checks passed for all 8 assets (`tar -tzf` / `unzip -tqq`).
