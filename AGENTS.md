# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains the Rust crate (`codex-acp`) source; `src/main.rs` is the binary entry point and `src/lib.rs` exposes shared library code.
- `scripts/` holds operational shell scripts for build/install, release tagging, and Zed settings backup/restore.
- `npm/` contains the npm wrapper package, platform binaries metadata, and JS test utilities.
- `logs/` is used for local runtime logs; keep it clean and avoid committing generated artifacts.
- `target/` is Cargo output and is ignored by git.

## Build, Test, and Development Commands
- `cargo build --release` builds the optimized binary at `target/release/codex-acp`.
- `cargo test` runs Rust unit tests (currently in `src/prompt_args.rs`).
- `scripts/build_and_install.sh` builds and installs the binary; supports overrides like `INSTALL_PATH` and `CARGO_TARGET_DIR`.
- `scripts/tag_release.sh` tags a release (e.g., `v0.9.1`) and is intended to trigger CI release workflows.
- `node npm/testing/test-platform-detection.js` checks npm platform resolution logic.

## Coding Style & Naming Conventions
- Rust follows standard `rustfmt` formatting; run `cargo fmt` before PRs.
- Prefer idiomatic Rust naming: `snake_case` for functions/modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants.
- Shell scripts in `scripts/` should remain POSIX-compatible where practical.

## Testing Guidelines
- Use `cargo test` for Rust unit tests; add new tests near the module being changed.
- JS test helpers live under `npm/testing/` and can be executed directly with `node`.

## Commit & Pull Request Guidelines
- Commit messages follow Conventional Commits (e.g., `feat: ...`, `docs: ...`, `chore: ...`).
- PRs should explain the user impact, include relevant CLI outputs or screenshots when behavior changes, and reference linked issues when available.

## Configuration & Security Notes
- API keys are provided via environment variables (e.g., `OPENAI_API_KEY`, `CODEX_API_KEY`); never commit secrets.
- For Zed integration, keep your local `settings.json` in sync using the backup/restore scripts in `scripts/`.
