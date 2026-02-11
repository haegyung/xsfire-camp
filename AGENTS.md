# Repository Guidelines

## Project Structure & Module Organization
- `src/` holds the Rust crate (`xsfire-camp`), with `src/main.rs` as the primary binary entry and `src/lib.rs` exposing shared pieces.
- `scripts/` hosts operational shell scripts (build/install, tagging, Zed settings backup/restore).
- `npm/` keeps the npm wrapper, platform metadata, and JS testing helpers.
- `logs/` and `target/` are runtime and build artifacts; keep `logs/` clean and never commit `target/`.

## Build, Test, and Development Commands
- `cargo build --release`: compiles the optimized binary at `target/release/xsfire-camp`.
- `cargo test`: runs Rust unit tests, currently centered in `src/prompt_args.rs`.
- `scripts/build_and_install.sh`: builds the binary and installs it; respects `INSTALL_PATH` and `CARGO_TARGET_DIR`.
- `scripts/tag_release.sh vX.Y.Z`: tags a release (e.g., `v0.9.1`) to trigger CI workflows.
- `node npm/testing/test-platform-detection.js`: validates npm platform resolution logic.

## Coding Style & Naming Conventions
- Follow `rustfmt` formatting (do not commit unformatted Rust). Use `snake_case` for functions/modules, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Shell scripts in `scripts/` should stay POSIX-compatible and use consistent indentation (2 spaces or tabs for clarity).
- Keep comments brief and aligned with the code’s intent; add inline rationale only when behavior is not obvious.

## Testing Guidelines
- Unit tests are colocated with the modules they cover, most currently under `src/`.
- Test files should reflect the module name (`modname.rs` tests in `modname` module) and describe the feature being asserted.
- Run `cargo test` after changes that affect logic or behavior; add targeted tests when introducing new code paths.

## Commit & Pull Request Guidelines
- Commit messages follow Conventional Commits (e.g., `feat:`, `fix:`, `chore:`). Keep the subject short and descriptive, and omit a body if unnecessary.
- PRs should explain the user impact, include relevant logs/screenshots when behavior changes, and mention linked issues when available.
- Ensure PRs cite any configuration changes (env vars, scripts) in the description for reviewers.

## Security & Configuration Tips
- Never commit API keys or other secrets; rely on environment variables such as `OPENAI_API_KEY` or `CODEX_API_KEY`.
- Keep Zed settings synchronized via `scripts/zed_settings_backup.sh`/`restore` when adjusting local config (if used).
