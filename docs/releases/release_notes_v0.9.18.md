# Release Notes - v0.9.18

## Summary

- Aligned npm publish scope with the current GitHub organization.
- Stabilized `test_init` behavior and related CI checks.
- Bumped release metadata and synchronized extension/npm artifacts to `v0.9.18`.

## Details

- Scope and package metadata adjustments were applied in:
  - `npm/bin/xsfire-camp.js`
  - `npm/publish/update-base-package.sh`
  - `npm/testing/validate.sh`
  - `npm/package.json`
- Extension metadata was updated in:
  - `extension.toml`
  - `extensions/xsfire-camp/manifest.toml`
- `src/thread.rs` received targeted test/init stability adjustments.

## Verification

- `cargo test`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.18`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.18`
