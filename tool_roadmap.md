# rmap — Cross-Project Roadmap CLI

Single Rust binary that manages `roadmap/tasks.toml` in any project (Elixir, Rust, Python, Go, anything). Renders portable views: `ROADMAP.md` (agent-readable, dense), `data.json` (dashboard-consumable), optionally HTML (human-readable).

**Status.** Phases 1–5 shipped: `validate`, `render`, `data.json` export, `status` mutator, `next` selector, and the `--check-render` pre-commit contract. Phases 6–11 planned — see *Implementation phases* below. The contract is **write-once, read by both agents and humans**: the same `tasks.toml` feeds an agent-queryable JSON view and a human-readable Markdown view.

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

[focus]
phase = 12                                    # active phase — `rmap next` and dashboards key off this (Phase 10)

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
scored_at = "2026-04-15"                      # last D/B/U revision; >30d renders with `?` suffix (Phase 11)
markers = ["parallel"]                        # subset of: parallel | cx | csr
assignee = "claude"                           # human | claude | codex | cursor (Phase 9, optional)
linear_id = "INE-247"                         # optional — when a Linear issue tracks this task
created_at = "2026-04-01"                     # ISO-8601 date (Phase 10, optional)
started_at = "2026-04-12"                     # set when status → in_progress
done_at    = "2026-04-30"                     # set when status → done
shipped_in = "PR #21"                         # PR title/number or commit SHA

[[task]]
id = 75
phase = 12
bundle = "order_normalization"
status = "pending"
title = "parseOrder field map"
scores = { d = 6, b = 8, u = 8 }
scored_at = "2026-05-01"
depends_on = [74]                             # in-repo task IDs this is blocked by
linear_id = "INE-300"
assignee = "codex"                            # `rmap delegate 75 --to codex` reads this (Phase 9)
acceptance_criteria = [                       # rendered as bullet list; agents verify against this (Phase 9)
  "parseOrder accepts spot + futures payloads",
  'Empty venues field maps to null, not ""',
]
# Cross-repo dependencies. relation: "blocks" | "blocked_by" | "related"
cross_repo = [
  { repo = "ccxt_client", task_id = 42, linear_id = "INE-310", relation = "blocks" },
]
body = """
Multi-line prompt-style body when needed. Optional.
"""

# When status = "blocked", blocked_reason is required (Phase 10):
#   status = "blocked"
#   blocked_reason = "Waiting on legal review of session token storage"
```

## CLI surface

Shipped commands (Phases 1–5) and planned ones (Phases 6–11). Planned commands carry a `[P<n>]` tag.

```
# read / render (Phases 1–3, 6, 8, 11)
rmap render                          # write ROADMAP.md + data.json from tasks.toml
rmap render --dry                    # print would-write diff, no file changes
rmap render --stdout                 # render ROADMAP.md to stdout (for diffing in hooks)
rmap render --format mermaid         # [P11] phase-gantt block for human viewers
rmap render --html                   # [P6]  priv/roadmap/index.html static viewer
rmap export json                     # data.json to stdout (for piping)

# validation (Phases 1, 5)
rmap validate                        # schema check + integrity (orphan deps, marker validity, cycles)
rmap validate --check-render         # also verify ROADMAP.md is in sync; exit 2 on drift, 1 on schema error
rmap doctor                          # [P11] health summary: drift, cycles, orphans, stale, score-decay

# query / introspection (Phase 8) — agent read-API
rmap next [--marker parallel] [--for codex] [--json]   # next highest-Eff unblocked pending
rmap show <id> [--json]              # full task detail; --json for piping
rmap list [--status S --marker M --phase N --assignee A --json]   # generalized query
rmap schema [--json]                 # emit JSON Schema for editor completion + agent self-description
rmap diff [--against main]           # what changed in tasks.toml vs base ref
rmap stale --over <duration>         # in_progress tasks idle > duration (needs Phase 10 timestamps)

# mutation (Phase 4 + Phase 11 extensions) — all routed through toml_edit
rmap status <id[,id,id]> <new>       # flip status (bulk form added in Phase 11), re-render
rmap mark <id> +cx -parallel         # [P11] add/remove markers without TOML editing
rmap depend <id> on <id> [--cross-repo repo:N]     # [P11] add deps via mutation
rmap new                             # interactive task creation (dialoguer)
rmap new --from-stdin                # [P11] non-interactive — agent piping

# delegation (Phase 9) — cloud-agent workflow
rmap delegate <id> --to claude|codex|cursor        # emit paste-ready prompt: title + body + deps + AC + linked PRs

# live dev (Phase 7)
rmap watch                           # FS watch on tasks.toml, render on change
rmap watch --json                    # [P11] event stream for agent consumers
```

## Implementation phases

| # | Phase | Status | Headline deliverable |
|---|---|---|---|
| 1 | Parse + validate | ✅ | `rmap validate` against `tasks.toml` |
| 2 | Render `ROADMAP.md` | ✅ | Marker-bounded block replacement, byte-preserved prose |
| 3 | Export `data.json` | ✅ | Dashboard ingestion target with computed `eff` |
| 4 | Mutators | ✅ | `rmap status`, `rmap next` |
| 5 | Pre-commit contract | ✅ | `validate --check-render` exits **2** on drift, **1** on schema error |
| 6 | HTML render *(optional)* | ⬜ | `priv/roadmap/index.html` static viewer |
| 7 | `rmap watch` *(optional)* | ⬜ | FS watch for live dev |
| 8 | **Read API + self-description** | ⬜ | `show`, `list`, `schema --json`, `diff` |
| 9 | **Cloud delegation surface** | ⬜ | `delegate`, schema adds `assignee` + `acceptance_criteria` |
| 10 | **Schema completeness** | ⬜ | Timestamps, `blocked_reason`, `[focus]`, cycle detection |
| 11 | **Health + polish** | ⬜ | `doctor`, `stale`, mermaid render, bulk mutators, score-decay |

**Sequencing rationale.** Phase 8 first: today `rmap` only mutates. Every downstream agent (`task-driver`, `audit-review`, the dashboard) wants a read API more than it wants more mutators. Phase 9 next: it unblocks the cloud-agent delegation workflow that's already the user's main consumer of roadmap data. Phases 10–11 compound but don't unblock anything — sequence inside each phase by D/B/U.

**Cross-cutting invariant (Phases 8–9).** The `--json` outputs of `show`, `list`, `next`, and `schema` are the agent contract. Treat them like a public API: add fields freely, but never rename or remove without a `schema_version` bump.

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

## Invariants & boundaries (load-bearing)

Read this section before changing anything. The schema example above IS the contract — don't deviate even when something looks more idiomatic.

**Schema invariants — fail validation if violated:**

- `schema_version = 1` required at file top. Refuse to parse unknown versions; emit `rmap migrate` hint.
- Tasks are `[[task]]` (TOML array of tables), NOT `[task.74]` (table-per-id). Order is file order — preserves user authoring order, keeps diffs small.
- `eff = (b+u)/(2d)` is **computed at render time**, never stored. Don't add an `eff` field to the schema.
- `markers` must be a subset of `{"parallel", "cx", "csr"}`.
- `status` must be one of `{"pending", "in_progress", "blocked", "done", "superseded"}`. When `status = "blocked"`, `blocked_reason` is required (Phase 10).
- `linear_id` (when present) must match `<team_key>-<integer>` per `[linear].team_key`. Skip the format check entirely if the `[linear]` table is absent — Linear is opt-in.
- `assignee` (when present) must be one of `{"human", "claude", "codex", "cursor"}` (Phase 9).
- Timestamps (`created_at`, `started_at`, `done_at`, `scored_at`) are ISO-8601 dates (`YYYY-MM-DD`). All optional — presence is what unlocks decay / stale / recently-shipped features (Phase 10).
- `<!-- TASKS:BEGIN phase=N -->` / `<!-- TASKS:END -->` are preservation boundaries. Render replaces ONLY contents between matching markers; everything else in `ROADMAP.md` is hand-edited prose and must be byte-preserved.

**Mutation crate boundary.** All write paths (`status`, `new`, future `mark`, `depend`, bulk `status`) round-trip through `toml_edit::DocumentMut` to preserve comments and formatting. Never use `toml::from_str` → `to_string` on a file that will be written back — it strips comments and reflows. New mutators inherit this contract.

**Self-description boundary** *(Phase 8)*. `rmap schema --json` emits a JSON Schema that must match the live `schema::Tasks` deserializer. CI: a unit test parses the emitted schema and validates the example `tasks.toml` against it. Keeps the agent-consumable contract honest as fields evolve.

**Delegation boundary** *(Phase 9)*. `rmap delegate` is a **pure read** — it never mutates state and never calls external APIs. It emits text to stdout for the user (or a wrapper script) to paste. Any Linear / GitHub / Slack posting stays in skills, not in rmap.

## Testing strategy

- **Unit tests** for parse: valid `tasks.toml` deserializes to the expected `Tasks` struct; invalid `schema_version` / marker / status / `linear_id` / `assignee` / timestamp formats produce specific errors with file path + line location.
- **Golden tests** for render: `tests/golden/<name>/tasks.toml` + `tests/golden/<name>/ROADMAP.md` (expected output). Runner reads input, renders, asserts byte-equal against expected. Adding a test case = drop in a fixture pair. Render-template changes that break expected output fail the suite loudly.
- **Round-trip test**: parse → serialize via `toml_edit` → assert no spurious diff. Catches comment-preservation regressions across mutator additions.
- **Marker preservation test**: a `ROADMAP.md` fixture with hand-edited prose surrounding `<!-- TASKS:BEGIN -->` / `<!-- TASKS:END -->` blocks. After render, the prose is byte-equal; only marked block contents change.
- **Schema self-consistency test** *(Phase 8)*: `rmap schema --json` output validates the example `tasks.toml`. Fails if the emitted JSON Schema drifts from the live deserializer.
- **Cycle detection test** *(Phase 10)*: a `tasks.toml` fixture with a `depends_on` cycle fails `validate` with a clear error pointing at the cycle members.

## Out of scope (deliberately)

- **No web server** in `rmap` itself. Dashboard is a separate Phoenix app — see `dashboard_roadmap.md`.
- **No remote sync / multi-user collaboration.** Single-developer tool.
- **No git integration** beyond reading the working tree (no commit, no push). `rmap diff` reads `git show <ref>:tasks.toml` but never writes.
- **No Linear / GitHub / Jira API integration** in `rmap` core. The schema carries identifiers (`linear_id`, `shipped_in` PR refs) as data; external skills handle the API side and feed updates back via `rmap status`. `rmap delegate` emits a paste-ready prompt — it does not post anything.
- **No per-consumer migration scripts.** Per-repo migration plans live in their own repos (e.g. `~/_DATA/code/ccxt_extract/migration_roadmap.md`, mirrored in this repo as a design reference only).
- **No CI workflow, no GitHub Actions, no `rustfmt.toml`** for rmap itself. Cargo defaults are fine; this is a personal-productivity tool, not an OSS release.
- **No `--help` polish beyond `clap` defaults**, no man pages, no shell completions.
- **No render-template authoring surface.** Templates ship inside the binary; users don't fork `minijinja` files. If a render shape is wrong, fix it upstream and ship a new binary.
