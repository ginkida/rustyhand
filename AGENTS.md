# Repository Guidelines

## Project Structure & Module Organization
RustyHand is a Rust workspace rooted at `Cargo.toml`. Core code lives in `crates/`: `rusty-hand-runtime` runs the agent loop and tools, `rusty-hand-kernel` handles orchestration, `rusty-hand-api` serves the Axum API and dashboard, and `rusty-hand-cli` builds the `rustyhand` binary. Shared types live in `rusty-hand-types`; adapters, skills, extensions, memory, and wire protocol each have their own crate. Prebuilt agent templates live in `agents/*/agent.toml`. SDKs are in `sdk/javascript` and `sdk/python`. Dashboard assets are static files under `crates/rusty-hand-api/static`, so UI edits do not require a Node build step.

## Build, Test, and Development Commands
Use the Makefile for the common path:

- `make build` builds the release CLI binary at `target/release/rustyhand`.
- `make clippy` runs workspace linting with `-D warnings`.
- `make test` runs `cargo test --workspace --exclude rusty-hand-runtime`.
- `make check` runs build, clippy, and tests together.
- `cargo fmt --all -- --check` verifies formatting.
- `docker compose up --build` starts the full stack locally on port `4200`.

For a fast edit cycle, use `make dev` or `cargo build --workspace --lib`.

## Coding Style & Naming Conventions
Target the stable toolchain from `rust-toolchain.toml`; `rustfmt` and `clippy` are required. Follow standard Rust conventions: 4-space indentation, `snake_case` for modules/functions/files, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants. Keep crate names kebab-case to match the existing workspace. Prefer small, crate-local changes over cross-workspace refactors.

## Testing Guidelines
Place unit tests inline with `mod tests` and integration tests in `crates/<crate>/tests/*_test.rs`. Run focused tests while iterating, for example `cargo test -p rusty-hand-api --test api_integration_test -- --nocapture`, then finish with the workspace suite. There is no published coverage threshold; contributors are expected to add regression tests for behavior changes.

## Commit & Pull Request Guidelines
Recent history uses short, imperative, sentence-case subjects such as `Fix scheduler bugs, harden cron engine, improve dashboard UX`. Keep commits focused and explain behavior, not implementation trivia. PRs should include a concise summary, linked issues, screenshots for dashboard changes, and the commands you ran. If you add an API route, update both `crates/rusty-hand-api/src/server.rs` and `crates/rusty-hand-api/src/routes.rs`. If you add config, wire in `#[serde(default)]`, `Default`, and example config updates without committing secrets.
