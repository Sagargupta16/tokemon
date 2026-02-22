# CLAUDE.md

## Build

Rust 1.83+ required. All cargo commands must be run inside Docker on this machine:

```bash
docker run --rm \
  -v $(pwd):/app -w /app \
  -v /tmp/ca-bundle.pem:/etc/ssl/certs/ca-certificates.crt:ro \
  -e SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt \
  -e CARGO_HTTP_CAINFO=/etc/ssl/certs/ca-certificates.crt \
  tokemon-dev cargo build --release
```

If `tokemon-dev` image doesn't exist: `docker build -t tokemon-dev --target builder .`

Regenerate CA bundle if needed:
```bash
security export -t certs -f pemseq -k /Library/Keychains/System.keychain -o /tmp/sys.pem
security export -t certs -f pemseq -k /System/Library/Keychains/SystemRootCertificates.keychain -o /tmp/root.pem
cat /tmp/sys.pem /tmp/root.pem > /tmp/ca-bundle.pem
```

## Test

```bash
# In Docker:
docker run --rm -v $(pwd):/app -w /app tokemon-dev cargo test
```

## Run against real data

```bash
docker run --rm \
  -v $(pwd):/app -w /app \
  -v ~/.claude:/root/.claude:ro \
  -v ~/.cache/tokemon:/root/.cache/tokemon:ro \
  tokemon-dev ./target/release/tokemon [ARGS]
```

## Git

- Remote: `https://github.com/mm65x/tokemon.git` (private)
- Push: `GH_CONFIG_DIR=/tmp/tokemon-gh gh auth token` for HTTPS auth

## Code Conventions

- **New JSONL providers**: Use `GenericJsonlProvider<C>` from `jsonl_provider.rs` — implement `JsonlProviderConfig` (~15 lines)
- **Cline-derived providers**: Use `ClineFormatParser` from `cline_format.rs`
- **Timestamps**: Always use `parse_utils::parse_timestamp()`, never inline
- **Glob patterns**: Use `PathBuf::join("**/*.jsonl").display().to_string()`
- **Errors**: Skip malformed lines with `continue`, warnings to stderr only
- **Pure functions**: Annotate with `#[must_use]`
