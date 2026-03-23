# Release Notes - v0.9.15

## Summary

- Hardened thread bridge flow and session command orchestration in `src/thread.rs`.
- Improved runtime handling across setup/monitor control paths in active sessions.
- Bumped release metadata to `v0.9.15` across Cargo and npm package manifests.

## Details

- Main implementation changes are concentrated in `src/thread.rs` to stabilize bridge behavior under ACP session flows.
- Version metadata was updated in:
  - `Cargo.toml`
  - `Cargo.lock`
  - `npm/package.json`

## Verification

- `cargo test`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.15`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.15`
