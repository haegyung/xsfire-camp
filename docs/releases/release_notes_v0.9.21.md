# Release Notes - v0.9.21

## Summary

- Added a Zed-specific setup plan progress summary row for clearer wizard progress visibility.
- Preserved non-Zed behavior to avoid changing plan UX for other ACP clients.
- Added focused tests for Zed/non-Zed summary behavior and synchronized release metadata.

## Details

- `src/thread.rs` now:
  - detects Zed clients via `client_info`
  - computes setup-plan status counts
  - prepends a progress summary entry to `SessionUpdate::Plan` for Zed only
- The summary entry includes:
  - progress bar
  - completion percentage
  - completed/in-progress/pending counts
- New tests:
  - `thread::tests::test_setup_emits_zed_plan_progress_summary`
  - `thread::tests::test_setup_does_not_emit_plan_progress_summary_for_non_zed_client`

## Verification

- `cargo test`
- `cargo test thread::tests::test_setup_emits_zed_plan_progress_summary`
- `cargo test thread::tests::test_setup_does_not_emit_plan_progress_summary_for_non_zed_client`

## Release

- Tag: `v0.9.21`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.21`
