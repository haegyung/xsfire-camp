# Release Notes - v0.9.16

## Summary

- Improved npm publish auth diagnostics and OIDC fallback handling in release workflow.
- Restructured `README.md` and added an explicit README improvement plan.
- Added registry unblock checklist and refreshed Zed extension metadata for release operations.

## Details

- `.github/workflows/release.yml` now includes clearer auth-path diagnostics and fallback handling.
- New planning docs were added:
  - `docs/plans/readme_improvement_plan.md`
  - `docs/plans/release_registry_unblock_checklist.md`
- Extension metadata alignment updates:
  - `extension.toml`
  - `extensions/xsfire-camp/manifest.toml`

## Verification

- `cargo test`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.16`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.16`
