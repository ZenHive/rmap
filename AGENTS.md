# Repository Guidelines

## Project Structure & Module Organization

This repository is for `rmap`, a standalone Rust CLI that manages portable roadmap data. The current source of truth is [tool_roadmap.md](/Users/efries/_DATA/code/rmap/tool_roadmap.md), which defines the intended schema, CLI surface, and first implementation scope.

When the crate is scaffolded, keep the layout conventional:

- `src/main.rs` for CLI entrypoint and `clap` command wiring.
- `src/lib.rs` plus focused modules such as `schema`, `validate`, and `render`.
- `tests/golden/<case>/` for render fixtures.
- `roadmap/tasks.toml` as the input format in consumer projects, not this tool’s own task tracker unless needed.

## Build, Test, and Development Commands

Use Cargo defaults unless the repo later documents otherwise:

- `cargo build` compiles the binary.
- `cargo run -- validate <path>` runs the CLI during development.
- `cargo test` runs unit and integration tests.
- `cargo fmt --check` verifies formatting.
- `cargo clippy --all-targets -- -D warnings` catches lint issues before review.

Do not add CI, shell completions, HTML output, or mutation commands in the first implementation pass unless explicitly requested.

## Coding Style & Naming Conventions

Follow standard Rust style with `rustfmt` defaults. Use `snake_case` for functions, modules, and fields; `PascalCase` for types; and `SCREAMING_SNAKE_CASE` for constants. Keep modules small and purpose-specific. Prefer explicit error variants with `thiserror` for validation failures and `anyhow` only at command boundaries.

## Testing Guidelines

Write tests for every implemented behavior. Prioritize:

- Unit tests for TOML parsing and schema validation.
- Golden tests for `ROADMAP.md` rendering.
- Preservation tests proving only marked roadmap blocks change.
- Regression tests for invalid status, marker, schema version, and Linear ID cases.

Tests must fail loudly on unexpected errors; never convert unknown failures into passing assertions.

## Commit & Pull Request Guidelines

There is no existing commit history, so use concise imperative commits such as `Add schema validation` or `Render marked roadmap blocks`. PRs should include the problem solved, scope boundaries, test evidence, and any intentionally deferred work. Link issues when available and include screenshots only for future visual outputs.

## Agent-Specific Instructions

Implement only the requested scope. For the first build session, follow `tool_roadmap.md`: parse, validate, and render only. Preserve user-authored roadmap prose byte-for-byte outside render markers.
