# Release Notes - v0.9.14

## Summary

- Added a rubric-driven default execution protocol baseline for all `xsfire-camp` use cases.
- Exposed the protocol directly in setup Plan UI so iteration gates remain visible until completion.
- Extended `/skills` with options, filtering, usage/examples output, and stronger test coverage.

## Details

- Setup Plan/UI protocol baseline:
  - Added explicit setup guidance for:
    `Goal -> Rubric (Must/Should + evidence) -> Research -> Plan -> Implement -> Verify -> Score`.
  - Added a loop gate plan step that remains tied to verification progress and `Must=100%` completion.
  - Updated setup-related tests to assert the new protocol/loop-gate steps.
- `/skills` command improvements:
  - Added option hint to command metadata:
    `--enabled`, `--disabled`, `--scope <scope>`, `--reload`, `<keyword>`.
  - Added parser + filter handling for those options.
  - Added usage/examples output, including invalid-option guidance.
  - Added tests:
    - `test_skills_with_reload_option`
    - `test_skills_with_enabled_filter_option`
    - `test_skills_with_invalid_option_returns_usage_without_submit`
- Documentation updates:
  - `README.md`
  - `docs/backend/policies.md`
  - `docs/reference/event_handling.md`

## Verification

- `cargo fmt --check` passes.
- `cargo test` passes (44 tests, 0 failures).
- `cargo test test_skills -- --nocapture` passes.
- `cargo test test_setup -- --nocapture` passes.

## Release

- Tag: `v0.9.14`
- GitHub Release: `https://github.com/haegyung/xsfire-camp/releases/tag/v0.9.14`
