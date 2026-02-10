# Zed Marketplace Extension PR Template

Use this template when submitting the extension update to `zed-industries/extensions`.

```markdown
## Extension Submission: thePrometheus Codex ACP

### Summary
- registers thePrometheus Codex ACP as a Zed extension
- uses `theprometheus-codex-acp` binary (vX.Y.Z) and shares CODEX_HOME with CLI
- adds documentation links for installation, verification, and CODEX_HOME sharing

### Testing
- `cargo test`
- `theprometheus-codex-acp` run with `/review`, `/compact`, `/undo` (manual)
- Verified Plan/ToolCall/RequestPermission logging via `docs/event_handling.md`

### Files added/updated
- `extensions/thePrometheus-codex-acp/manifest.toml`
- (if updates) `extensions.toml` entry for the extension
```

Fill in any additional data required by the Zed repo's PR checklist (e.g., supported architectures, maintainers). Replace bullet list with real commit references if needed.
