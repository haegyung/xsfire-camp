use anyhow::Result;
use clap::Parser;
use codex_arg0::arg0_dispatch_or_else;
use codex_common::CliConfigOverrides;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendKind {
    Codex,
    ClaudeCode,
    Gemini,
}

impl BackendKind {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "codex" => Some(Self::Codex),
            "claude-code" | "claude" => Some(Self::ClaudeCode),
            "gemini" | "gemini-cli" => Some(Self::Gemini),
            _ => None,
        }
    }
}

fn extract_backend_arg(args: &mut Vec<std::ffi::OsString>) -> anyhow::Result<BackendKind> {
    // Default to codex to preserve current behavior.
    let mut backend = BackendKind::Codex;

    let mut i = 1;
    while i < args.len() {
        let arg = args[i].to_string_lossy();
        if arg == "--backend" {
            let Some(value) = args.get(i + 1) else {
                anyhow::bail!("--backend requires a value (codex|claude-code|gemini)");
            };
            let value = value.to_string_lossy();
            backend = BackendKind::parse(&value)
                .ok_or_else(|| anyhow::anyhow!("unknown backend: {value}"))?;
            // Remove flag + value so upstream clap parsing doesn't see unknown args.
            args.drain(i..=i + 1);
            continue;
        }

        if let Some(value) = arg.strip_prefix("--backend=") {
            backend = BackendKind::parse(value)
                .ok_or_else(|| anyhow::anyhow!("unknown backend: {value}"))?;
            args.remove(i);
            continue;
        }

        i += 1;
    }

    Ok(backend)
}

fn main() -> Result<()> {
    arg0_dispatch_or_else(|codex_linux_sandbox_exe| async move {
        // Some ACP clients/extensions invoke agents as `<command> acp` or with `--acp`.
        // This binary already speaks ACP over stdio, so those tokens are no-ops.
        let mut args: Vec<std::ffi::OsString> = std::env::args_os().collect();
        if args.len() > 1 {
            let arg1 = args[1].to_string_lossy();
            if arg1 == "acp" || arg1 == "--acp" {
                args.remove(1);
            }
        }

        let backend = extract_backend_arg(&mut args)?;
        if backend != BackendKind::Codex {
            anyhow::bail!("backend not supported yet: {backend:?} (current: codex only).");
        }

        let cli_config_overrides = CliConfigOverrides::parse_from(args);
        xsfire_camp::run_main(codex_linux_sandbox_exe, cli_config_overrides).await?;
        Ok(())
    })
}
