# Release Notes - v0.9.19

## Summary

- Added runtime diagnostics aimed at investigating Zed-side memory spike reports.
- Removed npm distribution channel and shifted release guidance to binary/registry-first flows.
- Added fallback npm token path after OIDC publish failures for release resilience.

## Details

- Runtime diagnostics and related session telemetry updates were implemented in:
  - `src/thread.rs`
  - `src/lib.rs`
  - `src/acp_agent.rs`
- Release/operations documentation was updated:
  - `README.md`
  - `docs/README.md`
  - `docs/guides/github_registry_release_runbook.md`
  - `docs/guides/npm_publish_recovery.md`
  - `docs/quality/qa_checklist.md`
  - `docs/quality/verification_guidance.md`
- CI/release channel adjustments include:
  - removal of `.github/workflows/release.yml`
  - fallback handling commits included before tagging (`f068df9`)

## Verification

- `cargo test`
- `node npm/testing/test-platform-detection.js`

## Release

- Tag: `v0.9.19`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.19`
