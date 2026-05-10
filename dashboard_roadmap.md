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

1. **Next up** — `pending` + unblocked + matches active filters, sorted by Eff descending. Top 10 per project, all projects collapsed-by-default with an "expand" toggle.
2. **In progress** — `in_progress` tasks with branch name and worktree path (`~/_DATA/worktrees/<repo>/<id>/`). One row each. Click to copy worktree path to clipboard.
3. **Blocked** — grouped by reason (`depends_on` pending in same/other repo, `cross_repo` blocker, free-form `blocked_reason` from `tasks.toml`).
4. **Cross-repo** — DAG view of `cross_repo: { repo, task_id }` edges. Visual: "ccxt_extract Task 78 unblocks ccxt_client Task 42." Use `vis-network` (npm via `npm_ex`) or `d3-dag`.
5. **Recently shipped** — `done` tasks with `shipped_in` within last 7d, grouped by repo, click-through to PR link.

## Filters (always-on left rail)

- **Repo** — multi-select from discovered projects
- **Status** — pending / in_progress / blocked / done
- **Marker** — parallel / cx / csr (multi-select)
- **Min Eff** — slider 0.0-3.0
- **Phase** — per-project phase picker (only relevant when one repo selected)
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

## Build order

**Defer until 2-3 repos use `rmap`** so the `tasks.toml` + `data.json` shape has stabilized. Building the dashboard before the data shape is settled = wasted iteration on UI you'll need to rewrite.

## Out of scope (deliberately)

- No task editing in the dashboard. Source of truth is `tasks.toml` per repo. Dashboard is read-only — author tasks via `rmap new` or by hand.
- No authentication. Local-only personal tool.
- No mobile-first responsive design. Desktop browser is the target surface.
- No notifications / push. Cron-backed weekly digest is a future option, not core.
