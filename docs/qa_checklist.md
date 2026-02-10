# Release QA Checklist

Use this checklist before tagging/publishing the extension release.

1. **Documentation**
   - [x] `docs/install_shared_settings.md` describes shared `CODEX_HOME` usage.
   - [x] `docs/event_handling.md` maps CLI events to ACP notifications.
   - [x] `docs/verification_guidance.md` outlines test steps.
   - [x] `docs/codex_home_overview.md` lists `threads/`, `credentials/`, etc.
2. **Code/Tests**
   - [x] `cargo test` (unit tests and event coverage) passes locally.
   - [x] `TaskState` delegates to `PromptState` to reuse event handling.
3. **Zed-specific**
   - [ ] `extensions/thePrometheus-codex-acp/manifest.toml` references vX.Y.Z binaries.
   - [ ] `extensions.toml` entry matches sample snippet and uses `pnpm sort-extensions`.
   - [ ] PR body follows `docs/zed_extension_pr_template.md` content.
4. **Release Artifacts**
   - [ ] Cargo/npm versions bumped consistently (`Cargo.toml`, `npm/package.json`).
   - [ ] `scripts/tag_release.sh vX.Y.Z` run to create the tag (or `git tag` manually).
   - [ ] GitHub Actions release workflow triggered by pushing the tag.
5. **Manual verification**
   - [ ] Launch ACP with `CODEX_HOME` pointing to CLI home and run `/review`, `/compact`, `/undo`.
   - [ ] Inspect `logs/codex_chats/...` for `Plan`, `ToolCall`, and `RequestPermission` entries.
   - [ ] (Optional) Verify canonical log under `ACP_HOME` (default `~/.acp`) is created and appends `canonical.jsonl`.
   - [ ] Confirm Zed agent panel (if available) shows plan/tool call updates as expected.

Mark each step when complete and keep the checklist with the release notes for traceability.
