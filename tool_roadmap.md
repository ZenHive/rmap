# rmap — Cross-Project Roadmap CLI

Single Rust binary that manages `roadmap/tasks.toml` in any project (Elixir, Rust, Python, Go, anything). Renders portable views: `ROADMAP.md` (agent-readable, dense), `data.json` (dashboard-consumable), optionally HTML (human-readable).

**Status.** Phases 1–5, 8, 9, 10, and 11a shipped: `validate`, `render`, `data.json` export, `status` mutator (single + bulk), `next` selector, `show`, `list`, `schema`, `diff`, `delegate`, `stale`, `doctor`, the `--check-render` pre-commit contract, schema completeness (`[focus]`, task timestamps, `blocked_reason`, cycle detection), and health surfaces (score-decay `?` suffix, stale-task surface, composite doctor report). Phases 6–7 and 11b planned — see *Implementation phases* below. The contract is **write-once, read by both agents and humans**: the same `tasks.toml` feeds an agent-queryable JSON view and a human-readable Markdown view.

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
rmap render --format mermaid         # [P11b] phase-gantt block for human viewers
rmap render --html                   # [P6]   single-project view → roadmap/dist/index.html
rmap render --html --multi P1 P2     # [P6]   portfolio HTML across N repos/data.json paths
rmap export json                     # data.json to stdout (for piping)

# validation + health (Phases 1, 5, 11a)
rmap validate                        # schema check + integrity (orphan deps, marker validity, cycles)
rmap validate --check-render         # also verify ROADMAP.md is in sync; exit 2 on drift, 1 on schema error
rmap doctor [--json]                 # health summary: validate findings, stale, score-decay, drift — always exits 0

# query / introspection (Phase 8) — agent read-API
rmap next [--marker parallel] [--json]   # next highest-Eff unblocked pending
rmap show <id> [--json]              # full task detail; --json for piping
rmap list [--status S --marker M --phase N --json]   # generalized query
rmap schema                          # emit JSON Schema for editor completion + agent self-description
rmap diff [--against <ref>] [--json]  # what changed in tasks.toml vs base ref (default: default_branch)
rmap stale --over <duration> [--json]   # in_progress tasks idle > duration

# mutation (Phase 4 + Phase 11a + Phase 11b extensions) — all routed through toml_edit
rmap status <id[,id,id]> <new>       # flip status (bulk form shipped in Phase 11a), re-render
rmap mark <id> +cx -parallel         # [P11b] add/remove markers without TOML editing
rmap depend <id> on <id> [--cross-repo repo:N]     # [P11b] add deps via mutation
rmap new                             # interactive task creation (dialoguer)
rmap new --from-stdin                # [P11b] non-interactive — agent piping

# delegation (Phase 9) — cloud-agent workflow
rmap delegate <id> --to claude|codex|cursor        # emit paste-ready Markdown prompt: title + body + deps + AC + linked refs

# live dev (Phase 7)
rmap watch                           # FS watch on tasks.toml, render on change
rmap watch --json                    # [P11b] event stream for agent consumers
```

## Implementation phases

| # | Phase | Status | Headline deliverable |
|---|---|---|---|
| 1 | Parse + validate | ✅ | `rmap validate` against `tasks.toml` |
| 2 | Render `ROADMAP.md` | ✅ | Marker-bounded block replacement, byte-preserved prose |
| 3 | Export `data.json` | ✅ | Dashboard ingestion target with computed `eff` |
| 4 | Mutators | ✅ | `rmap status`, `rmap next` |
| 5 | Pre-commit contract | ✅ | `validate --check-render` exits **2** on drift, **1** on schema error |
| 6 | HTML render | ⬜ | Single-project + portfolio dashboards as one self-contained file (see *HTML render design*) |
| 7 | `rmap watch` *(optional)* | ⬜ | FS watch for live dev |
| 8 | **Read API + self-description** | ✅ | `show`, `list`, `schema --json`, `diff`, `next --json` |
| 9 | **Cloud delegation surface** | ✅ | `delegate`, schema adds `assignee` + `acceptance_criteria` |
| 10 | **Schema completeness** | ✅ | Timestamps, `blocked_reason`, `[focus]`, cycle detection |
| 11a | **Health + cleanup** | ✅ | `doctor`, `stale`, score-decay rendering, bulk `status`, drop `schema --json` no-op |
| 11b | **Polish** | ⬜ | mermaid render, delegate per-agent footer, `new --from-stdin`, `mark`, `depend`, interactive `new`, `diff --verbose` |

**Sequencing rationale.** Phases 9, 10, and 11a are shipped: downstream agents have a paste-ready delegation prompt, richer task lifecycle data (timestamps + blocked reasons), focus-phase signaling for `rmap next`, dependency cycle protection, and a composite health surface (`doctor`) that aggregates validate findings, stale, score-decay, and drift. Phase 11b is the remaining polish — sequence by D/B/U.

**Cross-cutting invariant (Phases 8–9).** The `--json` outputs of `show`, `list`, `next`, `schema`, and `diff` are the agent contract. Treat them like a public API: add fields freely, but never rename or remove without a `schema_version` bump.

**Phase 10 follow-ups (discovered during Phase 8–9 review).**

- When adding new `Task` fields (`created_at`, `started_at`, `done_at`, `blocked_reason`, `scored_at`), each one must land in three places in the same commit: `schema::Task`, the `diff_fields!` invocation in `diff::changed_fields`, and `export::ExportedTask`. The `schemars` derive auto-tracks the schema, but the diff macro and export struct are hand-maintained. CLAUDE.md carries the invariant note.
- Cycle detection (mentioned for Phase 10) belongs in `validate.rs` alongside `validate_dependencies` — a DFS over `task.depends_on` collecting back-edges, with an error pointing at the cycle members.
- `[focus].phase` becomes load-bearing for `rmap next` and dashboards (Phase 6) — wire it through `schema::Tasks` as `Option<Focus>` with `#[serde(deny_unknown_fields)]` and have `next::next_task` prefer focus-phase tasks when set.

**Phase 11b follow-ups (carried forward + discovered during Phase 11a review).**

- `rmap delegate` instructions footer is currently hardcoded. Parameterize per target: Cursor can run the harness pre-PR, Codex cannot reach hex.pm, Claude is local. Source-of-truth lives in `cloud-delegation:cloud-agent-environments` skill; mirror the per-agent reachability into the delegate prompt template.
- Consider surfacing diff value-level before/after for high-signal fields (`status`, `scores`) via `rmap diff --verbose` — currently only the field name is reported. Keep field-key set as the default to avoid noise in pre-commit hook output.
- `rmap doctor` could grow `--threshold-days <N>` to override the hardcoded 30-day stale/decay cutoff. Defer until a real consumer asks; the constants live in `src/doctor.rs` (`STALE_THRESHOLD_DAYS`, `DECAY_THRESHOLD_DAYS`) and `src/scoring.rs::score_decay_suffix`.
- `rmap doctor --json`'s `DoctorFinding` enum is tagged with `kind: "..."` (snake_case). When extending, follow the additive rule — new variants are fine; renaming a `kind` value would break agent consumers and requires a `schema_version` bump.
- `today_iso()` reads `RMAP_TODAY` for test determinism, then falls back to system clock. Document this in CLAUDE.md so future tests don't reinvent date injection.

## HTML render design (Phase 6)

Same source flow: `tasks.toml` → `data.json` → HTML. The HTML is a **derived view**, not a replacement for `ROADMAP.md`. Markdown stays canonical for git diffs, terminal scanning, and skill consumption. HTML is for humans skimming progress and for shareable snapshots that survive outside a checkout.

**Audience: both agents and humans.** Humans read the visual layout. Agents read the rendered HTML by extracting the embedded data island (see invariant 2 below). The same artifact serves both.

### Output modes

- `rmap render --html` — single-project view at `roadmap/dist/index.html` (gitignored).
- `rmap render --html --multi P1 P2 …` — portfolio view across N repos. Each `Pn` is either a project root (rmap discovers its `roadmap/data.json`) or a path to a `data.json` directly. Output: `roadmap/dist/portfolio.html` in the current working repo, or `--out <path>` to redirect.

### Layout (single project)

```
┌──────────────────────────────────────────────────────────────────┐
│ rmap   Phase 1: 8/10 ▓▓▓▓▓▓▓▓░░    Phase 2: 3/7 ▓▓▓░░░░          │
│ markers: [all][chain][api][docs]   status: [✓][→][⏸][⛔]          │
├──────────────────┬──────────────────┬────────────────────────────┤
│  PENDING         │  IN PROGRESS     │  DONE                      │
│  ┌────────────┐  │  ┌────────────┐  │  ┌──────────────────┐      │
│  │ #14  eff:6 │  │  │ #12  eff:5 │  │  │ #11 render-html  │      │
│  │ html-out   │  │  │ dep-graph  │  │  │ #10 export-json  │      │
│  │ ⊃ #12 #11  │  │  │ ●chain    │  │  └──────────────────┘      │
│  │ ●chain ●ui │  │  └────────────┘  │                            │
│  └────────────┘  │                  │                            │
├──────────────────┴──────────────────┴────────────────────────────┤
│ DEPENDENCY GRAPH (SVG, layered DAG)                        [+]   │
└──────────────────────────────────────────────────────────────────┘
```

Vertical phases stacked; each phase header carries a progress bar so a top-down skim conveys the whole project in 5 seconds. Within a phase, three horizontal status columns (pending / in_progress / done) read left→right as progress. A sticky filter bar exposes marker chips and status toggles. The dependency graph sits below, collapsed by default.

### Portfolio layout

Same shell, but the top level is **repos-as-rows**: each row is a mini progress strip (phase-bar summary) + the top 3 open tasks by `eff`. A top-of-page panel renders **cross-repo blockers** driven by the `cross_repo` field — arrows between repo cards visualize "X blocks Y across the fleet." Click a repo row → expands inline to the single-project layout.

### Design invariants

1. **Self-contained single file.** Inline CSS + vanilla JS. No CDN, no framework, no external assets, no build step. <50KB target per page. Opens offline; uploads to S3 as one blob; attaches to email; survives indefinitely without a server.
2. **Embedded data island.** A `<script id="rmap-data" type="application/json">…</script>` carries the full `data.json` verbatim inside the HTML. Agents extract structured data without DOM scraping; client-side filters read from this island. This is the single most important choice for "agents read it too" — the visual layer never becomes the bottleneck for an agent reading the file.
3. **Semantic data attributes** on every task element: `data-id`, `data-status`, `data-eff`, `data-markers`, `data-depends-on`, `data-phase`. Stable selectors, greppable, future-proof.
4. **Color = status, chips = markers.** Status carries the primary visual signal (done=green, in_progress=blue, pending=slate, blocked=amber). Markers render as small muted text chips, not loud colors. Status colors must remain WCAG AA at body text size; print stylesheet falls back to status symbols (`✓ → ⏸ ⛔`) for monochrome output.
5. **Dep graph as SVG, layered DAG.** Topological layers, downward arrows. Collapsed by default. SVG (not Canvas) because nodes are selectable, text is searchable, and each node carries `data-id` for agent introspection. In portfolio mode, cross-repo edges render distinctly (dashed + repo label) from in-repo edges.
6. **Print stylesheet** included. Leadership prints, hands around, marks up. Status symbols + grayscale layout, no progress bars (they don't print well — replaced with `N/M done` numerals).

### What stays out

- No dark mode toggle, no user preferences, no animations, no tooltips that hide content.
- No JS framework dependency. No npm. No bundler. Vanilla JS, hand-written, single file.
- No per-user customization. The HTML is a report artifact, not an app.
- No live updates — that's the Phoenix dashboard's job (see `dashboard_roadmap.md`). `rmap render --html` produces a **static snapshot** taken at render time.
- No template authoring surface. The HTML template ships inside the rmap binary alongside the markdown template. Both evolve in lockstep with the schema.

### Boundary with the Phoenix dashboard

`rmap render --html` is the **portable static** view: one file, offline-readable, share-friendly, zero infrastructure. The Phoenix LiveView dashboard (`dashboard_roadmap.md`) is the **always-on live** view: real-time file watchers, filter persistence, cross-repo aggregation across an entire fleet. Different surfaces for different rhythms — both consume the same `data.json` schema, both pin to `schema_version`.

When to use which:
- **Static HTML** — share a snapshot in email, attach to a PR, hand to leadership, archive a milestone, work offline.
- **Live dashboard** — daily driving on the desktop while authoring + flipping statuses across repos.

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
