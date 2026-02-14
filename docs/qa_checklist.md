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
   - [x] `extension.toml` references live `vX.Y.Z` binaries for darwin/linux/windows targets with `sha256`.
   - [x] `docs/extensions_toml_sample.md` updated to the latest extension entry format.
   - [x] PR body template updated in `docs/zed_extension_pr_template.md`.
4. **Release Artifacts**
   - [x] Cargo/npm versions are consistent (`Cargo.toml` = `X.Y.Z`, `npm/package.json` = `X.Y.Z`).
   - [x] `vX.Y.Z` tag exists.
   - [x] GitHub Release `vX.Y.Z` created.
   - [x] Additional target assets (`darwin-*`, `linux-*`, `windows-*`) uploaded.
5. **Manual verification**
   - [ ] Launch ACP with `CODEX_HOME` pointing to CLI home and run `/setup` first.
   - [ ] Run `/status` -> `/monitor` -> `/vector` and verify setup plan step `Verify: run /status, /monitor, and /vector` reaches `completed`.
   - [ ] Change one config option (`Model`, `Approval Preset`, or task monitoring options) and confirm Plan progress updates immediately.
   - [ ] Confirm `/monitor` shows task snapshot (`Task monitoring: ...`, `Task queue: ...`).
   - [ ] Inspect `logs/codex_chats/...` for `Plan`, `ToolCall`, and `RequestPermission` entries.
   - [ ] (Optional) Verify canonical log under `ACP_HOME` (default `~/.acp`) is created and appends `canonical.jsonl`.
   - [ ] Confirm Zed agent panel (if available) shows plan/tool call updates as expected.

Mark each step when complete and keep the checklist with the release notes for traceability.

### Design System (MS Fluent) Additions (Optional, for UI frontend)
- [ ] `docs/ms_design_checklist_fluent.md` reviewed and approved.
- [ ] `docs/design-system/MS_FLUENT_TOKEN_SCHEMA.md` is the source of truth for token keys.
- [ ] `docs/design-system/fluent-tokens.json` values are synced with runtime tokens.
- [ ] `docs/design-system/fluent-theme.css` is imported in UI entrypoint and rendered root uses `data-ms-theme`.
- [ ] `docs/design-system/fluent-wrappers.tsx` is adopted for at least one component surface.
- [ ] `docs/design-system/README.md` contains migration notes and applied examples.
- [ ] Accessibility smoke checks include:
  - keyboard focus order + outline visibility
  - contrast check on text and brand backgrounds
  - `forced-colors` and reduced-motion pass-through
- [ ] `docs/design-system/fluent-demo.html` smoke check:
  - 버튼/입력 렌더 상태(기본/호버/비활성) 확인
  - `Tab` 포커스에서 outline/강조가 `--ms-focus-*`로 표시되는지 확인
  - 테마 전환 시 토큰 스와치(`brand-background`, `surface-card`, `focus-color`) 값이 반영되는지 확인
