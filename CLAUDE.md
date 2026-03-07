# CLAUDE.md

## Build & Test

Build from the symlinked directory (`~/code/tokenusage`), which avoids OneDrive filesystem hook issues:

```bash
cargo build --release            # debug: cargo build
cargo test                       # run all tests
cp target/release/tokemon ~/.local/bin/tokemon
```

## Lint

```bash
cargo clippy -- -W clippy::pedantic -A clippy::module_name_repetitions
cargo fmt -- --check
```

Note: CI uses the same clippy flags above (see `.github/workflows/ci.yml`). Local `cargo clippy -- -D warnings` may show additional warnings that CI does not enforce.

## Git

- **Remote**: `git@github-mm65x:mm65x/tokemon.git` (SSH)
- **Identity**: `mm65x <mm65x@users.noreply.github.com>` (set in local git config)
- **Branch**: `master` is the release branch. Use feature branches for non-trivial changes.

## CI / Release

- **CI** (`.github/workflows/ci.yml`): Runs on push/PR to `master`. Checks fmt, clippy, and tests on Linux/macOS/Windows.
- **Release** (`.github/workflows/release.yml`): Triggers on `v*` tags. Builds binaries for 5 targets (Linux x86/ARM, macOS x86/ARM, Windows), creates a GitHub release, and publishes to crates.io.

To release:
```bash
# Update version in Cargo.toml if needed, then:
git tag v0.1.0
git push origin v0.1.0
```

## Code Conventions

- **New JSONL sources**: Implement `JsonlSourceConfig` (~15 lines) and use `JsonlSource<C>` from `source/jsonl_source.rs`
- **Cline-derived sources**: Use `ClineFormat` from `source/cline_format.rs`
- **SQLite sources**: See `source/opencode.rs` for the pattern — open read-only, busy_timeout, `json_extract` for JSON columns
- **Timestamps**: Always use `timestamp::parse_timestamp()`, never inline parsing
- **File discovery**: Each `Source` implements `discover_files()` using helpers from `source/discover.rs` (`collect_by_ext`, `walk_by_ext`). No glob crate — use bounded `read_dir` walking only.
- **Display names**: Use `display.rs` functions (`display_client`, `display_model`, `infer_api_provider`) for presentation
- **API provider detection**: Prefix model names with `vertexai.`, `openai/`, `bedrock/`, etc. so `infer_api_provider` works. `cost.rs::find_pricing` strips `vertexai.` before lookup.
- **Errors**: Skip malformed lines with `continue`, warnings to stderr only
- **Pure functions**: Annotate with `#[must_use]`
- **Pre-filtering**: JSONL parsers should `line.contains()` check before `serde_json::from_str` to skip non-matching lines cheaply
- **BufReader**: Use `BufReader::with_capacity(64 * 1024, file)` for line-by-line parsers

## Content Policy

- **Never reference other tools by name** in README, comments, commit messages, or documentation. No comparisons, no "inspired by X", no "like Y". tokemon stands on its own.
- File paths that contain third-party tool names (e.g., `~/.config/tokscale/cursor-cache/`) are acceptable since those are factual filesystem locations.
