# Release Notes - v0.9.17

## Summary

- Stabilized npm trusted publishing by hardening OIDC token handling in CI/release paths.
- Aligned migrated org/repo metadata across release workflow, extension manifests, and npm templates.
- Bumped release metadata to `v0.9.17`.

## Details

- `.github/workflows/release.yml` received a sequence of fixes to avoid stale `NODE_AUTH_TOKEN` context and to preserve trusted-publishing state.
- Metadata consistency updates were applied to:
  - `extension.toml`
  - `extensions/xsfire-camp/manifest.toml`
  - `npm/package.json`
  - `npm/template/package.json`
- Minor backend adapter metadata syncs were applied in:
  - `src/claude_code_agent.rs`
  - `src/gemini_agent.rs`
  - `src/multi_backend.rs`

## Verification

- `cargo test`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.17`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.17`
