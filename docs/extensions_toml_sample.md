# Sample `extensions.toml` Entry

Add an entry like the following to register the extension in `zed-industries/extensions/extensions.toml`:

```toml
[[extension]]
name = "xsfire-camp"
id = "xsfire-camp"
version = "X.Y.Z"
path = "extensions/xsfire-camp"
description = "Codex CLI parity ACP adapter with shared CODEX_HOME."
homepage = "https://github.com/haegyung/theP_codex"
owner = "haegyung"
```

After updating the entry, run `pnpm sort-extensions` at the repo root so the file stays tidy.
