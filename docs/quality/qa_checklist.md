# Release QA Checklist

Use this checklist before tagging/publishing the extension release.

1. **Documentation**
   - [x] `docs/zed/install_shared_settings.md` describes shared `CODEX_HOME` usage.
   - [x] `docs/reference/event_handling.md` maps CLI events to ACP notifications.
   - [x] `docs/quality/verification_guidance.md` outlines test steps.
   - [x] `docs/reference/codex_home_overview.md` lists `threads/`, `credentials/`, etc.
2. **Code/Tests**
   - [x] `cargo test` (unit tests and event coverage) passes locally.
   - [x] `TaskState` delegates to `PromptState` to reuse event handling.
3. **Sequential Release Fitness (X/X')**
   - [ ] `fit_score`, `Context-Fit` decision, and `Release / Feedback` are recorded in `docs/quality/iteration_fit_template.md`.
   - [ ] Core invariants (`X`: safety, correctness, traceability, operability) remain retained in release candidate `X'`.
   - [ ] `X'` growth trend is documented: added value signals and resolved weak points.
4. **ACP registry-specific**
   - [x] `extension.toml` references live `vX.Y.Z` binaries for darwin/linux/windows targets with `sha256`.
   - [x] `docs/guides/github_registry_release_runbook.md` is updated and linked from `README.md` and `docs/README.md`.
   - [ ] ACP registry PR status/check snapshot is captured (`gh pr view` + `gh pr checks`) and attached to release evidence.
   - [ ] Any ACP registry PR comment is written in English only and includes run/check evidence.
   - [x] `docs/zed/extensions_toml_sample.md` and `docs/zed/zed_extension_pr_template.md` are marked as legacy reference docs.
5. **Release Artifacts**
   - [x] Cargo version and release tag are consistent (`Cargo.toml` = `X.Y.Z`, tag = `vX.Y.Z`).
   - [x] `vX.Y.Z` tag exists.
   - [x] GitHub Release `vX.Y.Z` created.
   - [x] Additional target assets (`darwin-*`, `linux-*`, `windows-*`) uploaded.
6. **Manual verification**
   - [ ] Launch ACP with `CODEX_HOME` pointing to CLI home and run `/setup` first.
   - [ ] Run `/status` -> `/monitor` -> `/vector` and verify setup plan step `Verify: run /status, /monitor, and /vector` reaches `completed`.
   - [ ] Change one config option (`Model`, `Approval Preset`, or task monitoring options) and confirm Plan progress updates immediately.
   - [ ] Confirm `/monitor` shows task snapshot (`Task monitoring: ...`, `Task queue: ...`).
   - [ ] Inspect `logs/codex_chats/...` for `Plan`, `ToolCall`, and `RequestPermission` entries.
   - [ ] (Optional) Verify canonical log under `ACP_HOME` (default `~/.acp`) is created and appends `canonical.jsonl`.
   - [ ] Confirm Zed agent panel (if available) shows plan/tool call updates as expected.
7. **ACP compatibility (based on `docs/reference/acp_standard_spec.md`)**
   - [ ] Run `scripts/acp_compat_smoke.sh --strict` and archive the generated report under `logs/smoke/`.
   - [ ] If strict mode fails, attach the corresponding failure log from `logs/smoke/logs/*.log` to the release issue/PR.
   - [ ] `initialize` returns `protocolVersion=v1` and advertises capability contract (`embeddedContext=true`, `image=true`, `audio=false`, `mcp.http=true`, `mcp.sse=false`, `session.list=true`).
   - [ ] `codex` backend passes core ACP flow: `authenticate` -> `session/new|load` -> repeated `session/prompt` -> `session/cancel` and returns valid JSON-RPC 2.0 envelopes.
   - [ ] `claude-code`/`gemini` backends keep declared behavior: `session/load` and `session/set_model` are supported; `session/set_mode` returns `invalid_params`; `session/set_config_option` supports model changes and rejects unsupported options; `session/cancel` remains documented no-op.
   - [ ] `session/update` stream includes expected update types (`AgentMessageChunk`, `AgentThoughtChunk`, `ToolCall`, `ToolCallUpdate`, `Plan`, `AvailableCommandsUpdate`, `CurrentModeUpdate`) without schema violations.
   - [ ] `ToolCall`/`Plan` status transitions stay in allowed enums (`pending`, `in_progress`, `completed`, `failed`) and do not regress state order during one turn.
   - [ ] `session/request_permission` round-trip is recorded with request/response pair in canonical logs when `ACP_HOME` logging is enabled.
   - [ ] `fs/*` capability path enforces session-root boundary checks and falls back to local FS access only when ACP FS capability is not advertised.
   - [ ] `terminal/*` integration works only when client capability is present, and tool execution progress is surfaced via ACP updates.
   - [ ] `session/list`, `session/set_model`, `session/set_config_option` (unstable) are smoke-tested against current schema versions and tracked as release risk if behavior changes.

Mark each step when complete and keep the checklist with the release notes for traceability.

### Design System (MS Fluent) Additions (Optional, for UI frontend)
- [ ] `docs/design-system/ms_design_checklist_fluent.md` reviewed and approved.
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
- [ ] `docs/design-system/fluent-react-demo.tsx` smoke check:
  - `FluentReactDemoApp`이 `fluent-react-demo-root`에 React 마운트 되는지 확인
  - `MsDialog` 오픈/클로즈가 `open` 상태 전환으로 동작하는지 확인
  - `MsButton/MsInput` 상호작용(기본/호버/비활성/포커스) 및 토큰 반영 확인
  - 다크/고대비/라이트 전환 시 `data-ms-theme` 기준 토큰이 즉시 반영되는지 확인
