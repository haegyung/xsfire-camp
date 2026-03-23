# Release Notes - v0.9.20

## Summary

- Normalized outgoing local markdown file links into canonical `file://` URIs.
- Added a reusable path normalization utility covering Unix and Windows absolute paths.
- Added regression tests to lock link normalization behavior.

## Details

- Added `src/link_paths.rs` with local-link normalization and percent-encoding helpers.
- Wired normalization into outbound ACP text paths:
  - `src/cli_common.rs`
  - `src/thread.rs`
- Added coverage for:
  - Unix absolute path normalization
  - Windows absolute path normalization
  - percent-encoding for spaces/non-ASCII
  - preserving existing URI links

## Verification

- `cargo test`
- `cargo test link_paths::tests::normalizes_unix_absolute_paths_inside_markdown_links`
- `cargo test thread::tests::test_send_agent_text_normalizes_local_markdown_file_links`

## Release

- Tag: `v0.9.20`
- GitHub Release: `https://github.com/theprometheusxyz/xsfire-camp/releases/tag/v0.9.20`
