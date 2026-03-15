# npm Publish Recovery Guide (Historical)

Status: historical incident record only. `xsfire-camp` no longer uses npm as an active distribution channel.

## Why This File Still Exists
- It records the investigation history for failed npm publishing after the GitHub org move (`haegyung` -> `theprometheusxyz`).
- It is preserved so earlier issue comments, workflow runs, and release notes still have a stable reference.
- The current release path is GitHub release binaries plus ACP registry binary distribution.

## Final State When npm Distribution Was Retired
- Real release reruns and an isolated base-package probe both failed with `E404` on first public publish under `@theprometheusxyz/*`.
- Repo-side workflow drift was already corrected before retirement.
- The remaining blocker appeared to be npm org/package administration or first-public-package creation behavior, not GitHub workflow configuration.

## Historical Evidence References
- Main release failure: `gh run view 23100542003 --repo theprometheusxyz/xsfire-camp --log-failed`
- First base-package probe: `gh run view 23105771624 --repo theprometheusxyz/xsfire-camp --log-failed`
- Org/package diagnostics: `gh run view 23105536849 --repo theprometheusxyz/xsfire-camp`
- Issue tracker chronology: `gh issue view 1 --repo theprometheusxyz/xsfire-camp`

## Current Guidance
- Do not use this file as an active runbook.
- For shipping and verification, follow `docs/guides/github_registry_release_runbook.md`.
- For ACP registration status, follow `docs/plans/release_registry_unblock_checklist.md`.
