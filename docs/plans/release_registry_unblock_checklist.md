# Release/Registry Unblock Checklist (v0.9.18)

## Goal
Unblock `v0.9.18` ACP registry visibility and release operations with the current binary-first distribution policy.

## Current Status
1. GitHub release binaries are the only active distribution channel
- Evidence:
  - `release.yml` now builds target archives and creates a GitHub release without npm publish jobs.
  - `README.md` now directs users to GitHub release binaries and ACP registry installation notes.
  - Historical npm investigation remains tracked in `docs/guides/npm_publish_recovery.md`, but npm is no longer part of the active release path.

2. ACP registry PR still requires maintainer-side action
- Target PR: `https://github.com/agentclientprotocol/registry/pull/93`
- Evidence:
  - PR head includes the `v0.9.18` entry refresh.
  - Latest `Build Registry` workflow run `23106246048` is `completed` with `conclusion=action_required`.
  - `gh pr checks 93 --repo agentclientprotocol/registry` reports no passing checks while the blocker remains.

3. ACP registry content itself is ready
- Evidence:
  - `registry_work/registry/xsfire-camp/agent.json` targets `0.9.18`.
  - Local auth check and local registry build both pass.
- Interpretation:
  - The ACP PR is blocked on upstream workflow approval, not on local entry correctness.

## Checklist: ACP Registry PR Unblock
1. Check PR metadata:
   ```bash
   gh pr view 93 --repo agentclientprotocol/registry --json state,mergeStateStatus,url
   ```
2. Check PR checks:
   ```bash
   gh pr checks 93 --repo agentclientprotocol/registry
   ```
3. If check conclusion is `action_required`, post exactly one English status comment with:
   - blocker summary
   - run ID/URL
   - requested maintainer action (approve/re-run)
4. Confirm the entry still matches `v0.9.18` release assets before each new push.
5. Wait for state change and re-check checks.
6. Once checks pass, keep PR updates in English until merge.

## English Comment Template (Registry PR)
```text
Status update: the current Build Registry run is blocked with `action_required` (run: <RUN_URL_OR_ID>).
Could a maintainer please approve/re-run this workflow for the fork-originated PR?
I will post a follow-up after the check status changes.
```

## Verification Commands
```bash
gh pr view 93 --repo agentclientprotocol/registry --json state,mergeStateStatus,url
gh pr checks 93 --repo agentclientprotocol/registry
gh run list --repo agentclientprotocol/registry --branch add-xsfire-camp-agent --limit 3
gh run view 23106246048 --repo agentclientprotocol/registry
```

## Done Criteria
1. ACP registry PR check status is no longer blocked by `action_required`, or the blocker is explicitly tracked with owner and next action.
2. Registry PR communication log remains English-only.
3. GitHub release binary URLs referenced by `agent.json` stay aligned with the latest shipped version.
