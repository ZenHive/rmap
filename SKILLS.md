# SKILLS.md — agent guide to `rmap`

> **Verified:** 2026-05-13 with `rmap @ development`. Re-run `cargo test --test skills_smoke` after schema or render changes.

`rmap` is a single-binary Rust CLI that manages `roadmap/tasks.toml` in any project. This file teaches cloud agents (Claude, Codex, Cursor) how to drive `rmap` from inside a consumer repo. The fenced `bash` blocks below run against `tests/skills_fixture/` via `tests/skills_smoke.rs`; the exit codes are part of the agent contract.

## Project layout

```
<project_root>/
├── ROADMAP.md            # rendered output, hand-edited prose between TASKS markers preserved
├── roadmap/
│   ├── tasks.toml        # canonical source — author this
│   └── data.json         # generated; agents read it for structured access
```

`ROADMAP.md` lives at the project root, **not** inside `roadmap/`. The renderer walks ancestors of `cwd` to find `roadmap/tasks.toml`; pass `--tasks-path` / `--roadmap-path` / `--data-path` to override.

## Reading state

`rmap show <id>` for the human view, `rmap show <id> --json` for the agent contract envelope. IDs can be numeric (`74`) or text (`"78b"`).

```bash
rmap show 1
# exit: 0
```

```bash
rmap show 1 --json
# exit: 0
```

`rmap list` filters across the whole tasks file. The `--json` envelope mirrors `data.json` (computed `eff` included).

```bash
rmap list --status pending
# exit: 0
```

```bash
rmap list --phase 1 --json
# exit: 0
```

## Picking work

`rmap next` returns the highest-Eff `pending` task whose dependencies are all `done`. When `[focus].phase` is set, focus-phase candidates win over higher-Eff candidates in other phases.

```bash
rmap next
# exit: 0
```

```bash
rmap next --json
# exit: 0
```

Filter by marker (e.g. `[P]` parallel-safe, `[CX]` Codex-delegated, `[CSR]` Cursor-delegated):

```bash
rmap next --marker parallel
# exit: 0
```

Marker absence returns exit 0 with no task printed when no candidate exists (focus phase complete, all blocked, or no marker matches).

## Mutating state

All mutators route through `toml_edit` to preserve comments + formatting, then re-validate before writing. Invalid mutations leave the file byte-equal to its pre-call state.

`rmap status <id> <new>` flips a single task; bulk form (`1,2,3`) is atomic — all IDs must resolve or nothing writes.

```bash
rmap status 4 in_progress
# exit: 0
```

```bash
rmap status 3,4 in_progress
# exit: 0
```

When flipping to `blocked`, supply `blocked_reason` directly in the TOML; the mutator refuses to write a blocked task without one.

`rmap mark <id> +marker -marker …` toggles markers. Adds are idempotent; removes are idempotent. Token tier: `parallel | cx | csr`.

```bash
rmap mark 3 +parallel
# exit: 0
```

`rmap depend <id> on <id>` adds an in-repo dependency. `--cross-repo <repo>:<task_id>[:<relation>]` adds a cross-repo dependency (relation defaults to `blocks`). Cycles are rejected by post-edit re-validation.

```bash
rmap depend 4 on 3
# exit: 0
```

## Creating tasks

`rmap new --from-stdin` reads one-or-more `[[task]]` blocks as TOML from stdin and appends them to `tasks.toml`. Omitting `id` auto-allocates the next numeric id (`max + 1`). `created_at` and `scored_at` default to today if not provided. Lifecycle timestamps (`started_at`, `done_at`, `blocked_reason`, `shipped_in`) cannot be set on creation — those transitions belong to `rmap status`.

```bash
rmap new --from-stdin
# exit: 0
# stdin: <<EOF
# [[task]]
# phase = 1
# bundle = "alpha"
# title = "follow-up: write docs"
# scores = { d = 2, b = 5, u = 4 }
# EOF
```

A multi-task fragment is atomic: if any insertion fails validation (unknown phase / bundle / cycle / duplicate id), no row lands and `tasks.toml` is byte-equal to its pre-call state. Example: a second task with an unknown phase number rejects the entire batch.

```bash
rmap new --from-stdin
# exit: 1
# stdin: <<EOF
# [[task]]
# phase = 1
# bundle = "alpha"
# title = "first"
# scores = { d = 1, b = 3, u = 3 }
#
# [[task]]
# phase = 99
# bundle = "alpha"
# title = "second — invalid phase, aborts batch"
# scores = { d = 1, b = 3, u = 3 }
# EOF
```

`rmap new` without `--from-stdin` drops into an interactive `dialoguer` flow (phase → bundle → title → D/B/U → markers → acceptance criteria → assignee → linear_id → module). Requires a TTY — non-interactive contexts must use `--from-stdin`. Bundles cannot be created on the fly; author the `[bundles.<name>]` table in `tasks.toml` first.

## Reading change signal

`rmap diff` shows what's changed in `tasks.toml` vs. a git ref (default: `default_branch` from the TOML). Read-only — never mutates working tree or git state.

```bash
rmap diff
# exit: 0
```

`--verbose` adds `values: [{field, before, after}]` entries on Changed tasks/metadata, filtered by an explicit whitelist. Adding a field to `schema::Task` does **not** automatically put it on the whitelist — touch `diff.rs` in the same commit.

```bash
rmap diff --verbose
# exit: 0
```

`--json` is the agent contract envelope; pipe through `jq` for filtering.

```bash
rmap diff --json
# exit: 0
```

## Health

`rmap doctor` is the soft-signal aggregator — validate findings + stale (>30d in-progress) + score-decay (>30d `scored_at` or missing) + degenerate-bundle + missing-`acceptance_criteria` + render drift. **Always exits 0** on success; only fails when input is unparseable. CI gates should pipe through `jq`:

```bash
rmap doctor --json
# exit: 0
```

```bash
rmap doctor
# exit: 0
```

## Strict gates

`rmap validate` (exit 1 on schema error) and `rmap validate --check-render` (exit 2 on render drift) are the strict gates. Use them in pre-commit hooks; use `doctor` for soft signals.

```bash
rmap validate
# exit: 0
```

```bash
rmap validate --check-render
# exit: 0
```

## Schema

`rmap schema` emits a JSON Schema for `tasks.toml`. It auto-tracks `schema::Tasks` via `schemars` derives — agents can read it to discover available fields without grepping the source.

```bash
rmap schema
# exit: 0
```

Adding fields to `schema::Task` is additive (safe); renaming or removing a field requires bumping `schema_version` in `tasks.toml`. Same rule applies to the `--json` envelopes of `show`, `list`, `next`, `diff`, and `doctor`.

## Delegation

`rmap delegate <id> --to claude|codex|cursor` emits a paste-ready Markdown prompt: title, body, in-repo dep context, acceptance criteria, plus a per-agent environment-notes footer tailored to that agent's runtime constraints (Codex sandbox / Cursor full-network / Claude local). Pure read — never calls Linear/GitHub/Slack.

```bash
rmap delegate 3 --to claude
# exit: 0
```

```bash
rmap delegate 3 --to codex
# exit: 0
```

```bash
rmap delegate 3 --to cursor
# exit: 0
```

## Rendering

`rmap render` writes `ROADMAP.md` (project root) + `roadmap/data.json` from `roadmap/tasks.toml`. Idempotent — no-change re-runs are byte-equal.

```bash
rmap render
# exit: 0
```

`--dry` prints what would change without writing; `--stdout` writes the rendered markdown to stdout instead of the file.

```bash
rmap render --dry
# exit: 0
```

```bash
rmap render --stdout
# exit: 0
```

### Marker conventions

Three marker pairs live inside `ROADMAP.md`. Bytes outside these markers are preserved exactly — hand-edited prose, headings, and blank lines all round-trip byte-equal.

- `<!-- TASKS:BEGIN phase=N -->` … `<!-- TASKS:END -->` — one pair per phase. Body is the rendered task table for phase N. The phase number is parsed from the BEGIN line.
- `<!-- FOCUS:BEGIN -->` … `<!-- FOCUS:END -->` — at most one pair per file. When `[focus]` is set in `tasks.toml`, the body emits three lines: `**Focus phase:** N — <name> (M of K done · J in progress)`, `**Last shipped:** Task A — <title>, Task B — <title> on YYYY-MM-DD` (within 7 days; `"no recent shipments"` otherwise), `**Up next:** Task C — <title> [D/B/U → Eff]` (or `"none — focus phase complete or all blocked"`). When `[focus]` is absent, emits a single `**Focus phase:** not set — add [focus] to tasks.toml`.
- `<!-- MERMAID:BEGIN -->` … `<!-- MERMAID:END -->` — at most one pair per file. Body is a fenced ```` ```mermaid ```` `gantt` diagram, one `section` per phase (in `phases.order` order), one row per task with `started_at`. Rows: `done` → `<title> :done, <started_at>, <done_at>`; `in_progress` → `:active, <started_at>, <today>`; `blocked` → `:crit, <started_at>, <today>`. Pending tasks and tasks without `started_at` are omitted (mermaid gantt requires dates). When no task qualifies, the body collapses to a placeholder gantt with `%% no tasks with started_at yet`. Colons / commas / semicolons (`:`, `,`, `;`) in task titles are rewritten to em-dash (`—`) at render time so mermaid's `task :status, start, end` grammar doesn't get confused.

If a marker pair isn't present in `ROADMAP.md`, the corresponding block isn't rendered — zero-config default for all three pairs.

## Exit code reference

| Command | 0 | 1 | 2 |
|---|---|---|---|
| `validate` | schema valid | schema error | — |
| `validate --check-render` | no drift | schema error | render drift |
| `doctor` | always (informational) | unparseable input | — |
| `diff` | success | schema/git error | — |
| `render` | success | schema/IO error | — |
| `show` | found | unknown id | — |
| `list`, `next`, `schema`, `delegate`, `stale` | success | schema/IO error | — |
| Mutators (`status`, `mark`, `depend`, `new`) | success + re-rendered | mutation rejected by re-validation | — |

For finer health signals, pipe `rmap doctor --json` through `jq`:

```bash
rmap doctor --json
# exit: 0
```
