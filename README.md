# thePrometheus Codex ACP

Use [Codex](https://github.com/openai/codex) from [ACP-compatible](https://agentclientprotocol.com) clients such as [Zed](https://zed.dev)!

This fork aligns ACP session metadata with Codex CLI, so Zed ACP threads share the same
session source as CLI sessions while preserving ACP behavior.

This tool implements an ACP adapter around the Codex CLI, supporting:

- Context @-mentions
- Images
- Tool calls (with permission requests)
- Following
- Edit review
- TODO lists
- Slash commands:
  - /review (with optional instructions)
  - /review-branch
  - /review-commit
  - /init
  - /compact
  - /logout
  - Custom Prompts
- Client MCP servers
- Auth Methods:
  - ChatGPT subscription (requires paid subscription and doesn't work in remote projects)
  - CODEX_API_KEY
  - OPENAI_API_KEY

Learn more about the [Agent Client Protocol](https://agentclientprotocol.com/).

## How to use

### Zed (custom agent registration)

Register this binary as a custom ACP agent in Zed. This keeps your setup stable even
when Zed updates its built-in Codex adapter.

Add this to your `settings.json` (paths are examples; Zed may not expand `$HOME` in `env`, so prefer an absolute path):

```
{
  "agent_servers": {
    "thePrometheus Codex": {
      "type": "custom",
      "command": "/absolute/path/to/theprometheus-codex-acp",
      "env": {
        "CODEX_HOME": "/Users/you/.codex"
      }
    }
  }
}
```

Then open the Agent Panel and start a new thread for "thePrometheus Codex".

## Automation

### Build + install (local)

```
scripts/build_and_install.sh
```

Optional overrides:

```
INSTALL_PATH="$HOME/.local/bin/theprometheus-codex-acp" \
CARGO_TARGET_DIR="/tmp/theP_codex-target" \
scripts/build_and_install.sh
```

### Zed settings backup/restore

```
scripts/zed_settings_backup.sh
scripts/zed_settings_restore.sh /path/to/settings.json.bak-YYYYmmddTHHMMSSZ
```

### Release tagging

```
scripts/tag_release.sh
```

Pushing a tag (e.g. `v0.9.1`) triggers the GitHub Actions release workflow.

### Other clients

Or try it with any of the other [ACP compatible clients](https://agentclientprotocol.com/overview/clients)!

#### Installation

Build the binary:

```
cargo build --release
```

The resulting binary is at:

```
target/release/codex-acp
```

Install the upstream adapter from the latest release if you need a prebuilt binary:
https://github.com/zed-industries/codex-acp/releases

You can then use `codex-acp` as a regular ACP agent:

```
OPENAI_API_KEY=sk-... codex-acp
```

Or via npm:

```
npx @zed-industries/codex-acp
```

## License

Apache-2.0
