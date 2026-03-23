# Release Notes - v0.9.22

## Summary

- Stabilized `thread` setup-related tests by removing environment-dependent visibility behavior from shared test setup.
- Backfilled missing historical release notes (`v0.9.15` through `v0.9.21`) and repaired release-note indices in project docs.
- Bumped release metadata to `v0.9.22`.

## Details

- `src/thread.rs` now includes a test-only `SessionClient::with_client_and_visibility(...)` constructor.
  - Existing runtime path still uses `UiVisibilityMode::from_env()`.
  - Shared test setup now pins `UiVisibilityMode::Full` to avoid ambient env leakage.
- Documentation updates:
  - Added `docs/releases/release_notes_v0.9.15.md` through `docs/releases/release_notes_v0.9.21.md`.
  - Updated release-note links in `README.md` and `docs/README.md`.
  - Removed dead links from `docs/README.md` quality section.
- Version metadata updates:
  - `Cargo.toml`
  - `Cargo.lock`
  - `npm/package.json`

## Verification

- `cargo fmt --check`
- `cargo test`
- `node npm/testing/test-platform-detection.js`
- `perl -ne 'while(/\\[[^\\]]+\\]\\(([^)]+)\\)/g){...}' README.md docs/README.md` (markdown local-link existence check)

## Release

- Tag: `v0.9.22`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.22`
