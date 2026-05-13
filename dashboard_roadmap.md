# Portfolio Dashboard

Phoenix LiveView app at `~/_DATA/code/portfolio_dashboard/` that aggregates roadmap state across all your repos. Single pane: "what's available to work on right now across the portfolio."

## Why Phoenix LiveView (not Rust, not static SPA)

- Best-in-class for this exact UI shape: server-rendered tables with live updates via websockets, server-side filtering/sorting, no API layer to maintain.
- Reads JSON, writes nothing. The dashboard is a CONSUMER of `data.json` — it doesn't author tasks (`rmap` does that).
- Hot reload during dashboard development.
- You already write Phoenix.
- Tailwind v4 + daisyUI 5 give you a polished UI for ~zero CSS effort.

## Discovery

On boot:

1. Glob `~/_DATA/code/*/roadmap/data.json` (path configurable via `PORTFOLIO_DASHBOARD_ROOT`).
2. Each found path = one project. Parse as `%Project{}` keyed by repo basename.
3. Store in ETS table `:project_data` (no DB needed — the source files ARE the database).
4. Start one `FileSystem` watcher per `data.json`. On `:modified` events, re-parse and broadcast via Phoenix.PubSub. LiveViews subscribe to the topic for the projects they're showing.

## Sections (ordered by frequency of use)

1. **Next up** — `pending` + unblocked + matches active filters, sorted by Eff descending. Top 10 per project, all projects collapsed-by-default with an "expand" toggle. When `[focus].phase` is set in `tasks.toml`, tasks in the focus phase float above out-of-focus ones at equal Eff.
2. **In progress** — `in_progress` tasks with branch name, worktree path (`~/_DATA/worktrees/<repo>/<id>/`), and `assignee`. One row each. Click to copy worktree path to clipboard. Filterable by assignee for "what's Codex on right now."
3. **Blocked** — grouped by reason: `depends_on` pending in same/other repo, `cross_repo` blocker, or free-form `blocked_reason` (required field when `status = "blocked"` per rmap Phase 10).
4. **Cross-repo** — DAG view of `cross_repo: { repo, task_id }` edges. Visual: "ccxt_extract Task 78 unblocks ccxt_client Task 42." Use `vis-network` (npm via `npm_ex`) or `d3-dag`.
5. **Recently shipped** — `done` tasks where `done_at` is within last 7d (rmap Phase 10 timestamp; falls back to git log of `tasks.toml` for older entries), grouped by repo, click-through to PR link via `shipped_in`.
6. **Stale work** — `in_progress` tasks with `started_at` more than 14d ago. Mirrors `rmap stale --over 14d` output; surfaces forgotten branches without requiring a terminal.

## Filters (always-on left rail)

- **Repo** — multi-select from discovered projects
- **Status** — pending / in_progress / blocked / done
- **Marker** — parallel / cx / csr (multi-select)
- **Assignee** — human / claude / codex / cursor (multi-select; needs rmap Phase 9 `assignee` field)
- **Min Eff** — slider 0.0-3.0
- **Phase** — per-project phase picker (only relevant when one repo selected; defaults to each repo's `[focus].phase` when present)
- **Score freshness** — toggle "hide tasks where `scored_at` > 30d" (needs rmap Phase 10–11 score-decay)
- **Search** — substring match on task title

Filter state lives in URL params so bookmarks survive reloads.

## Implementation phases

1. **Phoenix scaffolding** — single LiveView, naive table view, hardcoded list of one project.
2. **Discovery + parsing** — glob `~/_DATA/code/*/roadmap/data.json`, parse, render tables.
3. **FileSystem watchers + PubSub** — live updates when any `rmap render` runs anywhere.
4. **Filter rail + URL sync** — multi-select filters, slider, search.
5. **Cross-repo DAG view** — separate LiveView page, `vis-network` integration via `npm_ex`.
6. **Recently-shipped timeline** — group-by-week table, PR links.
7. **(Optional)** Click-through to open project in editor (`code ~/_DATA/code/<repo>/`) or jump to worktree.

## Stack

- Phoenix 1.8 + LiveView 1.x
- Tailwind v4 + daisyUI 5
- No DB (ETS for cached project state)
- `file_system` for watchers
- `npm_ex` for `vis-network` (DAG view only — keep JS surface minimal)

Single-app project (not umbrella). Bind to a port not in `~/.claude/tidewave-ports.md`; reserve one and add it to the registry.

## Schema dependency

The dashboard parses `data.json` against the schema emitted by `rmap`. Pin the parser to `schema_version: 1`. When `rmap` bumps the schema, the dashboard's parser bumps in lockstep — keep them in the same release cadence.

**Source of truth for the schema** is `rmap schema --json` (rmap Phase 8). The dashboard's parser should consume that JSON Schema at build time rather than maintaining a hand-written struct. Removes the drift class entirely.

Fields the dashboard depends on **once they ship** (rmap Phases 9–11): `assignee`, `acceptance_criteria`, `created_at`, `started_at`, `done_at`, `scored_at`, `blocked_reason`, `[focus].phase`. Treat each as optional with a graceful fallback until the rmap phase that adds it lands across consumer repos.

## Build order

**Defer until 2-3 repos use `rmap`** so the `tasks.toml` + `data.json` shape has stabilized. Building the dashboard before the data shape is settled = wasted iteration on UI you'll need to rewrite.

## Relationship to `rmap render --html` (static snapshot)

`rmap` Phase 6 ships a **static HTML render** of the same data — single self-contained file, no infrastructure. The two surfaces are complementary, not redundant:

| | Static HTML (`rmap render --html`) | Live dashboard (this) |
|---|---|---|
| **Output** | One self-contained file (<50KB, no CDN) | Phoenix app on a local port |
| **Updates** | Snapshot at render time | Live via FS watcher + PubSub |
| **Audience** | Sharing: email, S3 link, PR attachment, leadership | Daily driving on the desktop |
| **Filters** | Client-side against embedded data island | Server-side, URL-synced |
| **Cross-repo** | `--multi` flag re-renders one HTML across N data.json files | DAG view across the discovered fleet |
| **Lives where** | Inside the rmap binary; ships with every install | Separate Phoenix app, must be running |

Both consume the same `data.json` and pin to the same `schema_version`. The HTML render is the **portable** view; the dashboard is the **always-on** view. See `DESIGN.md` § "HTML render design (Phase 6)" for the static-render spec.

## Out of scope (deliberately)

- No task editing in the dashboard. Source of truth is `tasks.toml` per repo. Dashboard is read-only — author tasks via `rmap new` or by hand.
- No authentication. Local-only personal tool.
- No mobile-first responsive design. Desktop browser is the target surface.
- No notifications / push. Cron-backed weekly digest is a future option, not core.
- **No static / shareable snapshot export.** That's `rmap render --html`'s job (Phase 6). The dashboard is live-only — if you need a snapshot to email or archive, run rmap.
