# Release Notes Template — v0.9.10

## Release Summary
- Added a new `monitor` retrospective reporting mode that outputs a structured, lane-based status format with progress ticks, risks/blockers, and next actions.
- Synced release manifests and package versions to `0.9.10` across core Rust and npm metadata.

## What’s Changed

### Features
- `src/thread.rs`
  - Added `MonitorMode::Retrospective` and argument parsing for `/monitor retro`.
  - Implemented `render_monitor_retrospective` to render a fixed multi-item format matching the requested layout.
  - Added command hints in setup messaging for `/monitor retro` and in validation docs references.
  - Added unit coverage for `/monitor retro` output behavior.

### Packaging / Versioning
- `Cargo.toml`
  - `version` bumped to `0.9.10`.
- `Cargo.lock`
  - root crate version updated to `0.9.10`.
- `npm/package.json`
  - Base package version bumped to `0.9.10`.
  - Optional dependency pins bumped to `0.9.10`.
- `extension.toml`
  - Manifest version and release archive URLs updated to `0.9.10`.
- `extensions/xsfire-camp/manifest.toml`
  - Manifest version and release archive URLs updated to `0.9.10`.
- `extension.toml` / `extensions/xsfire-camp/manifest.toml`
  - SHA256 checksums refreshed against the `v0.9.10` GitHub release artifacts.

## Tests
- `cargo test`
  - `31 passed` (verified during this release cycle).
- `scripts/tag_release.sh`
  - Verified version/tag consistency and tag creation/push flow (`v0.9.10`).

## Versioning / Packaging
- Tag: `v0.9.10` (published on GitHub Release).
- Commit history since `v0.9.9`: `6012948`, `83edaf0`, `7e0c885`, `8ada25b`.

## Traceability
- `feat: add monitor retrospective reporting mode` — `83edaf0`
- `chore: sync 0.9.9 manifests and checksums` — `6012948`
- `chore: bump version to 0.9.10` — `7e0c885`
- `chore: refresh v0.9.10 release checksums` — `8ada25b`
