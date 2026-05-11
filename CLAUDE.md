# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project, regardless of language. It renders `ROADMAP.md` and `roadmap/data.json` from the TOML source. See `tool_roadmap.md` for the design contract and `AGENTS.md` for additional contributor guidelines.

## Commands

```
cargo build                              # compile
cargo test                               # unit + integration + golden tests
cargo test --test cli                    # run a single test file
cargo test render_command_updates        # run tests matching substring
cargo fmt --check                        # check formatting (rustfmt defaults)
cargo clippy --all-targets -- -D warnings
cargo run -- validate                    # exercise CLI during dev
cargo run -- render --dry
```

Edition is `2024` (Cargo.toml). MSRV: whatever ships with Rust 1.85+.

## Architecture

Pipeline: `tasks.toml` → `schema::Tasks` → either `render_roadmap_str` (markdown) or `export_json_str` (data.json) → write back. Mutations route through `toml_edit` to preserve user formatting and comments.

Module layout:

- `schema.rs` — `serde` structs with `#[serde(deny_unknown_fields)]` on every type. `TaskId` is an `untagged` enum (`Number(u32) | Text(String)`) so task IDs can be either. Custom `PartialEq<u32>` lets numeric IDs compare directly.
- `validate.rs` — parses TOML into `Tasks`, then runs semantic checks: schema_version, statuses, markers, linear_id format (skipped if `[linear]` table absent), dependency existence, cross-repo relations, and phase/bundle reference integrity. Errors carry `path:line` location.
- `render.rs` — byte-cursor walk over the input `ROADMAP.md`. Finds `<!-- TASKS:BEGIN phase=N -->` / `<!-- TASKS:END -->` pairs and replaces ONLY the body between them. Bytes outside markers are preserved exactly. The phase number is parsed from the BEGIN marker line.
- `export.rs` — separate `ExportedTask` struct that adds the computed `eff` field for the JSON output. `eff` is never stored in `tasks.toml` or in `schema::Task` — it's derived from `(b+u)/(2d)` and rounded to 2 decimals on emit.
- `mutate.rs` — `rmap status` path. Loads with `toml_edit::DocumentMut` (NOT `toml::from_str`) to preserve comments/whitespace, mutates the `status` cell in place, then re-parses through `validate_tasks_str` to reject invalid mutations before the file is written.
- `next.rs` — pure selector: highest-Eff `pending` task whose `depends_on` are all `done`, optionally filtered by marker. No I/O.
- `paths.rs` — walks ancestors of `cwd` to find `roadmap/tasks.toml`. Project root is the directory containing `roadmap/`. CLI flags (`--tasks-path`, `--roadmap-path`, `--data-path`) override discovery.
- `main.rs` — `clap` derive CLI. Top-level `run()` returns `Result<ExitCode>`; specifically `--check-render` returns exit code **2 on render drift** (distinct from exit 1 on validation errors) so pre-commit hooks can distinguish "needs re-render" from "schema broken."

## Load-bearing invariants

These are easy to violate without breaking tests immediately:

- **Marker boundaries are byte-preserved.** `render` only rewrites bytes between matched `<!-- TASKS:BEGIN phase=N -->` and `<!-- TASKS:END -->`. Everything else — including hand-edited prose, blank lines, the marker lines themselves — is byte-equal in/out. Don't "normalize" the input string anywhere in `render.rs`.
- **`eff` is never persisted.** Don't add it to `schema::Task`. Recompute at render/export time. The schema struct has `#[serde(deny_unknown_fields)]` and would reject it on load anyway.
- **Mutations go through `toml_edit`, not `toml`.** `toml::from_str` + `to_string` would strip comments and reflow formatting. `mutate.rs` uses `DocumentMut` for this reason. New mutator commands (Phase 4+) must do the same.
- **`schema_version = 1` is required.** Refuse to render unknown versions; `validate.rs` emits a `rmap migrate` hint. Bump on any breaking schema change.
- **`linear_id` validation is conditional.** If the `[linear]` table is absent, `linear_id` format is not checked — Linear is opt-in per project.
- **Status / marker / relation enums live as `&[&str]` constants in `validate.rs`** (`VALID_STATUSES`, `VALID_MARKERS`, `VALID_CROSS_REPO_RELATIONS`). Any new value must also be added to the render-time match arms in `render.rs` (status symbols, marker suffixes) — they don't share a source.

## Tests

- `tests/cli.rs` — black-box CLI tests via `Command::new(env!("CARGO_BIN_EXE_rmap"))`. Each test gets a unique temp dir from a per-test atomic counter.
- `tests/golden/<case>/` — fixture pairs: `tasks.toml`, `ROADMAP.input.md` (input), `ROADMAP.md` (expected output). Adding a case = drop in a triplet; `tests/render.rs` picks it up. Render-template changes that break expected output fail loudly — that's intended.
- `tests/roundtrip.rs` — Phase 4 lookahead: parse → `toml_edit` round-trip → assert no spurious diff. Catches comment-preservation regressions before mutation code expands.

## Scope discipline

`tool_roadmap.md` documents the intended phases. Implemented today: validate, render, export json, status (mutator), next. Deliberately out of scope (per `tool_roadmap.md`): Linear API calls, web server, git integration beyond reading the working tree, shell completions, CI workflow, multi-user sync. Don't add these without checking the roadmap first.
