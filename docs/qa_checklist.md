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
   - [x] `extension.toml` references live v0.9.5 binary for `darwin-aarch64` with `sha256`.
   - [x] `docs/extensions_toml_sample.md` updated to v0.9.5 entry format.
   - [x] PR body template updated in `docs/zed_extension_pr_template.md`.
4. **Release Artifacts**
   - [ ] Cargo/npm versions bumped consistently (`Cargo.toml`, `npm/package.json`).
   - [x] `v0.9.5` tag exists.
   - [x] GitHub Release `v0.9.5` created with darwin-aarch64 asset upload.
   - [ ] Additional target assets (`darwin-x86_64`, `linux-*`, `windows-*`) uploaded.
5. **Manual verification**
   - [ ] Launch ACP with `CODEX_HOME` pointing to CLI home and run `/review`, `/compact`, `/undo`.
   - [ ] Inspect `logs/codex_chats/...` for `Plan`, `ToolCall`, and `RequestPermission` entries.
   - [ ] (Optional) Verify canonical log under `ACP_HOME` (default `~/.acp`) is created and appends `canonical.jsonl`.
   - [ ] Confirm Zed agent panel (if available) shows plan/tool call updates as expected.

Mark each step when complete and keep the checklist with the release notes for traceability.
