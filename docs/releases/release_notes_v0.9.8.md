# Release Notes Template — v0.9.8

## Release Summary
- Fixed tool-call replay handling for namespaced tool names (`functions.<tool>`), which previously caused “tool call not found” behavior when replayed output from providers included namespace prefixes.
- Added regression coverage to prevent similar breakage for both `FunctionCall` and `CustomToolCall` replay paths.

## What’s Changed

### Fixes
- `src/thread.rs`
  - Normalize replayed tool names with `normalize_tool_name`.
  - Apply normalization in both:
    - `ResponseItem::FunctionCall`
    - `ResponseItem::CustomToolCall`
- `send_completed_tool_call` and shell/apply_patch handling now use normalized names, so output titles and generic fallback paths stay consistent with available tool handlers.

### Tests
- Added unit coverage:
  - `test_normalize_tool_name`
  - `test_replay_history_normalizes_namespaced_custom_tool_name`
  - `test_replay_history_normalizes_namespaced_function_tool_name`
- Validation command:
  - `cargo test -q`  
    - `30 passed; 0 failed`

### Versioning / Packaging
- Bumped:
  - `Cargo.toml` → `0.9.8`
  - `npm/package.json` and optional dependency pins → `0.9.8`
  - `Cargo.lock` root package version -> `0.9.8`
- Tag:
  - `v0.9.8` created and pushed to origin

### Traceability
- `fix: normalize namespaced tool calls` — `d2243b8`
- `chore: sync Cargo.lock version` — `14b2ea4`

## Release Notes Body Template (ready to paste)
```md
## xsfire-camp v0.9.8

### Why this release
In replayed sessions, tool calls containing namespaced names (for example `functions.shell_command` and `functions.apply_patch`) did not match dispatch branches. This caused `"tool call not found"` behavior or inconsistent fallback handling when restoring prior conversations.

### What changed
- Fixed namespaced tool-call normalization in `src/thread.rs`.
- Applied normalization in both replay paths:
  - `ResponseItem::FunctionCall`
  - `ResponseItem::CustomToolCall`
- Kept handling for legacy/non-namespaced tool names unchanged.
- Updated release metadata/version lockstep:
  - `Cargo.toml` → `0.9.8`
  - `npm/package.json` / optional dependency pins → `0.9.8`
  - `Cargo.lock` root package version → `0.9.8`
- Tag created and pushed: `v0.9.8`

### Validation
- `cargo test -q`  
  - `30 passed; 0 failed`
- New regression tests:
  - `test_normalize_tool_name`
  - `test_replay_history_normalizes_namespaced_custom_tool_name`
  - `test_replay_history_normalizes_namespaced_function_tool_name`

### Notes
- Merge/rollback:
  - Rollforward is safe via normal mainline release process.
  - No breaking behavioral changes are expected for normal un-namespaced tool calls.

### Commit trace
- `d2243b8` — `fix: normalize namespaced tool calls`
- `14b2ea4` — `chore: sync Cargo.lock version`
- `f4c221c` — `docs: add v0.9.8 release notes template and workflow trigger checklist`
```

## `.github/workflows/release.yml` Trigger Check

### A. Trigger wiring
- [x] `workflow_dispatch` exists (manual run supported).
- [x] `push` trigger exists for `tags: "v*"`.
- [x] `get-version` job obtains version from `Cargo.toml`.

### B. Version/tag guardrails
- [x] Guardrail in `get-version` checks `tag_name == v$VERSION`.
- [x] For tag-based runs, `tag_name` should equal pushed tag (e.g., `v0.9.8`).

### C. Build/test pipeline readiness
- [x] Build matrix includes all release targets used in manifests/archives.
- [x] Archive names are generated from `needs.get-version.outputs.version`.
- [x] Release job depends on build artifacts (`needs: [get-version, build]`).

### D. Release publish step
- [x] `softprops/action-gh-release` uses `tag_name` and `files` globs for `.tar.gz` / `.zip`.
- [x] `generate_release_notes: true` is set.

### E. Post-release follow-up (manual)
- [ ] Confirm GitHub Release page shows assets and generated notes.
- [ ] Confirm extension manifest references are updated to new version/asset URLs and checksums before merging any dependent release PR.
- [ ] Run manual checklist in `docs/quality/qa_checklist.md` for user-visible flows.
