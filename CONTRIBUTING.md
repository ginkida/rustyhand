# Contributing to RustyHand

Thanks for your interest in contributing!

## Getting Started

```bash
git clone https://github.com/ginkida/rustyhand.git
cd rustyhand
cargo build --workspace --lib
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Requires Rust 1.75+ (stable).

## Development Workflow

1. Fork the repository
2. Create a feature branch: `git checkout -b my-feature`
3. Make your changes
4. Run the full check suite:
   ```bash
   cargo build --workspace --lib
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   cargo fmt --all -- --check
   ```
5. Commit with a clear message
6. Open a Pull Request

## Project Structure

See [README.md](README.md#architecture) for the crate layout and architecture overview.

## Key Guidelines

- All PRs must pass CI (build, test, clippy with zero warnings, fmt)
- New API routes must be registered in `server.rs` AND implemented in `routes.rs`
- New config fields need: struct field + `#[serde(default)]` + `Default` impl entry
- Don't touch `rusty-hand-cli` without coordination — it's under active development
- Keep security in mind: no command injection, XSS, SQL injection, or SSRF

## Reporting Issues

Open an issue at https://github.com/ginkida/rustyhand/issues with:
- Steps to reproduce
- Expected vs actual behavior
- OS, Rust version, and RustyHand version
