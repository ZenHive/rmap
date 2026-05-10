# rmap — Cross-Project Roadmap CLI

Single Rust binary that manages `roadmap/tasks.toml` in any project (Elixir, Rust, Python, Go, anything). Renders portable views: `ROADMAP.md` (agent-readable, dense), `data.json` (dashboard-consumable), optionally HTML (human-readable).

## Why Rust

- Pre-commit boot ~10ms vs ~1.5s for an equivalent `mix` task. Compounds across N repos × commits/day.
- Single static binary, install once via `cargo install` or homebrew tap. No per-project language dependency.
- Language-neutral by construction: works in Elixir, Rust, Python, Go projects identically.
- `serde` + `toml` make the schema layer trivial; `clap` makes the CLI ergonomic; `minijinja` handles templating.

## Source format: `roadmap/tasks.toml`

```toml
schema_version = 1
project = "ccxt_extract"
default_branch = "development"

[linear]
team_key = "INE"                              # validates linear_id format (e.g. "INE-247")
workspace_url = "https://linear.app/efries"   # used to construct issue URLs in rendered views

[phases.12]
name = "Per-Exchange Normalization"
order = 12
status = "in_progress"

[bundles.ticker_normalization]
phase = 12
order = 1
description = "Unified ticker fields across all exchanges"

[[task]]
id = 74
phase = 12
bundle = "ticker_normalization"
status = "done"                               # pending | in_progress | blocked | done | superseded
title = "parseTicker field map + coercion + enums"
scores = { d = 5, b = 8, u = 8 }              # eff = (b+u)/(2d) computed by rmap, never stored
markers = ["parallel"]                        # subset of: parallel | cx | csr
linear_id = "INE-247"                         # optional — when a Linear issue tracks this task
shipped_in = "PR #21"                         # PR title/number or commit SHA

[[task]]
id = 75
phase = 12
bundle = "order_normalization"
status = "pending"
title = "parseOrder field map"
scores = { d = 6, b = 8, u = 8 }
depends_on = [74]                             # in-repo task IDs this is blocked by
linear_id = "INE-300"
# Cross-repo dependencies. relation: "blocks" | "blocked_by" | "related"
cross_repo = [
  { repo = "ccxt_client", task_id = 42, linear_id = "INE-310", relation = "blocks" },
]
body = """
Multi-line prompt-style body when needed. Optional.
"""
```

## CLI surface

```
rmap render                          # write ROADMAP.md + data.json from tasks.toml
rmap render --dry                    # print would-write diff, no file changes
rmap render --stdout                 # render ROADMAP.md to stdout (for diffing in hooks)
rmap validate                        # schema check + integrity (orphan deps, marker validity)
rmap validate --check-render         # also verify ROADMAP.md is in sync (for pre-commit)
rmap next [--marker parallel]        # print highest-Eff pending+unblocked task matching filter
rmap status <id> <new_status>        # flip one task's status, re-render
rmap new                             # interactive task creation, append to tasks.toml
rmap watch                           # FileSystem watch on tasks.toml, render on change
rmap export json                     # data.json to stdout (for piping)
```

## Implementation phases

1. **Parse + validate** `tasks.toml`. Schema structs. `rmap validate` works.
2. **Render** `ROADMAP.md` — find `<!-- TASKS:BEGIN phase=N -->` / `<!-- TASKS:END -->` markers, replace block contents. `rmap render` works.
3. **Emit** `roadmap/data.json` — full task array + metadata. Dashboard ingestion target.
4. **Mutators**: `rmap status` and `rmap next`. These edit `tasks.toml` and re-render in one step.
5. **Pre-commit integration**: `rmap validate --check-render` returns non-zero if drift detected.
6. **(Optional)** `rmap render --html` writes `priv/roadmap/index.html` static viewer + bundles `data.json`.
7. **(Optional)** `rmap watch` for live development.

## Crate dependencies

- `serde` + `serde_derive` — schema deserialization
- `toml` — source format parsing (preserves comments on round-trip via `toml_edit`)
- `toml_edit` — for `rmap status` / `rmap new` (mutations preserve formatting + comments)
- `clap` (derive feature) — CLI argument parsing
- `minijinja` — template rendering for ROADMAP.md
- `serde_json` — `data.json` emission
- `anyhow` + `thiserror` — error handling
- `notify` — `rmap watch` file events
- `dialoguer` — interactive `rmap new`
- `owo-colors` — terminal output

Estimated ~600-1000 LOC.

## Repo location

`~/_DATA/code/rmap/` — new standalone Rust crate, separate repo from any project that uses it.

## Distribution

- Local dev: `cargo install --path ~/_DATA/code/rmap`
- Future: homebrew tap (`brew install efries/tap/rmap`) once stable
- CI: each project documents `rmap` in its README setup section, alongside language toolchain
- Per-project pin: optional `roadmap/.rmap-version` file the binary checks against

## Schema versioning

`schema_version = 1` at the top of `tasks.toml` is load-bearing. `rmap` refuses to render tasks files whose schema version it doesn't understand. Bump on breaking changes; ship a `rmap migrate --from 1 --to 2` for upgrades.

## First-pass build guidance (read first)

Read this section before reading anything else. The schema example above IS the contract — don't deviate even when something looks more idiomatic.

**Scope the first implementation session to Phases 1-2 ONLY:**

- Phase 1: parse + validate `tasks.toml`, schema structs, `rmap validate` subcommand.
- Phase 2: render `ROADMAP.md` with `<!-- TASKS:BEGIN phase=N -->` / `<!-- TASKS:END -->` markers, `rmap render` subcommand.

Do NOT build `rmap status` / `next` / `new` / `watch`, `data.json` emission, or HTML rendering yet — those ship in later sessions (Phases 3-7 above).

**Schema invariants — fail validation if violated:**

- `schema_version = 1` required at file top. Refuse to parse unknown versions; emit `rmap migrate` hint.
- Tasks are `[[task]]` (TOML array of tables), NOT `[task.74]` (table-per-id). Order is file order. This is deliberate — preserves user authoring order, keeps diffs small.
- `eff = (b+u)/(2d)` is **computed at render time**, never stored. Don't add an `eff` field to the schema.
- `markers` must be a subset of `{"parallel", "cx", "csr"}`.
- `status` must be one of `{"pending", "in_progress", "blocked", "done", "superseded"}`.
- `linear_id` (when present) must match `<team_key>-<integer>` per `[linear].team_key`. Skip the format check entirely if the `[linear]` table is absent — Linear is opt-in per project.
- `<!-- TASKS:BEGIN phase=N -->` / `<!-- TASKS:END -->` are preservation boundaries. Render replaces ONLY contents between matching markers; everything else in `ROADMAP.md` is hand-edited prose and must be byte-preserved.

**Mutation crate boundary (load-bearing for later phases):**

The deps list includes both `toml` (read/parse) and `toml_edit` (write/mutate). Phase 1-2 only reads, so `toml_edit` isn't called yet — but leave it in `Cargo.toml`. Phase 4 (`rmap status`) MUST round-trip through `toml_edit` to preserve comments and formatting. Removing `toml_edit` because "we don't use it yet" breaks the future contract that user-authored comments survive automated status flips.

**Out-of-scope guards for the first session:**

- No migration of any existing project's `ROADMAP.md` to `tasks.toml`. Per-consumer migration plans live in their own repos (e.g. `~/_DATA/code/ccxt_extract/migration_roadmap.md`).
- No CI workflow, no GitHub Actions config, no `rustfmt.toml`. Cargo defaults are fine.
- No `--help` polish beyond what `clap` derives for free, no man pages, no shell completions. Personal-productivity tool, not an OSS release.
- No Linear API calls. The schema CARRIES `linear_id` as data; rmap never talks to the Linear API. Skills/scripts that want to sync with Linear do so externally and feed data back via `rmap status` (Phase 4).

## Testing strategy

- **Unit tests** for parse: valid `tasks.toml` deserializes to the expected `Tasks` struct; invalid `schema_version` / marker / status / `linear_id` format produce specific errors with file path + line location.
- **Golden tests** for render: `tests/golden/<name>/tasks.toml` + `tests/golden/<name>/ROADMAP.md` (expected output). Runner reads input, renders, asserts byte-equal against expected. Adding a test case = drop in a fixture pair. Render-template changes that break expected output fail the suite loudly.
- **Round-trip test** (Phase 4 lookahead — write now to lock the contract): parse → serialize via `toml_edit` → assert no spurious diff. Catches comment-preservation regressions before mutation code lands in Phase 4.
- **Marker preservation test**: a `ROADMAP.md` fixture with hand-edited prose surrounding `<!-- TASKS:BEGIN -->` / `<!-- TASKS:END -->` blocks. After render, the prose is byte-equal; only marked block contents change.

## Out of scope (deliberately)

- No web server in `rmap` itself. Dashboard is a separate Phoenix app.
- No remote sync / multi-user collaboration. Single-developer tool.
- No git integration beyond reading the working tree (no commit, no push).
- No Linear / GitHub / Jira API integration in `rmap` core. The schema carries identifiers (`linear_id`, `shipped_in` PR refs) as data; external skills/scripts handle the API side and feed updates back via `rmap status`.
