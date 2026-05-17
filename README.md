# rmap

A single-binary Rust CLI for portable, language-agnostic project roadmaps.

`rmap` reads `roadmap/tasks.toml` and renders `ROADMAP.md` (for humans) and `roadmap/data.json` (for AI coding agents). One typed source file, two views, zero infrastructure.

```
roadmap/tasks.toml ──► rmap render ──► ROADMAP.md
                  └──► rmap export ──► roadmap/data.json
```

No daemon, no server, no database, no language assumptions. Drop it into a Rust crate, an Elixir umbrella, a TypeScript monorepo, a research repo — all the same.

## Why

Most roadmap tools either lock you into a SaaS UI (Linear, Jira) or scatter intent across hand-edited Markdown that drifts the moment two people touch it. `rmap` keeps the **source of truth as a typed file** and treats `ROADMAP.md` as a generated view. Comments and hand-written prose around marker pairs are byte-preserved across rewrites, so you can edit the Markdown freely and the next render won't fight you.

It is also explicitly designed for **agent-driven workflows**: every read command has a `--json` envelope, mutations are atomic and validate before writing, and `rmap delegate` formats a task as a Markdown prompt for cloud agents (Codex, Cursor).

## Install

```bash
cargo install --path .
# or, from a checkout:
cargo build --release && cp target/release/rmap ~/.local/bin/
```

MSRV: Rust 1.85 (edition 2024).

## Quick start

```bash
mkdir -p roadmap
cat > roadmap/tasks.toml <<'TOML'
schema_version = 2
project = "my_app"
default_branch = "main"

[phases.1]
name = "Bootstrap"
order = 1
status = "in_progress"

[[task]]
id = 1
phase = 1
status = "pending"
title = "Wire up the database"
scores = { d = 4, b = 8, u = 7 }
TOML

cat > ROADMAP.md <<'MD'
# Roadmap

<!-- TASKS:BEGIN phase=1 -->
<!-- TASKS:END -->
MD

rmap validate              # schema + semantic checks
rmap render                # rewrite ROADMAP.md + emit roadmap/data.json
rmap next                  # highest-efficiency pending task with deps satisfied
rmap status 1 in_progress  # transition
rmap status 1 done --implemented "wired up postgres + ecto repo"
```

## Commands

| Command | Purpose | Mutates |
|---|---|---|
| `rmap validate` | Schema + semantic validation. Exit 1 on error. | no |
| `rmap validate --check-render` | Detect drift between TOML and `ROADMAP.md`. Exit 2 on drift. | no |
| `rmap render` | Render `ROADMAP.md` + `roadmap/data.json`; `--html` also writes `roadmap/dist/index.html`. | yes (md + json) |
| `rmap watch` | Re-render on every `tasks.toml` change (FS watch). Ctrl-C to stop. | yes (md + json) |
| `rmap show <id>` | Inspect one task (`--json` for agent envelope). | no |
| `rmap list` | Filter by `--status`, `--phase`, `--marker`, `--bundle` (`--json`). | no |
| `rmap next` | Pick highest-Eff pending task with deps `done`. | no |
| `rmap new` | Interactive create; `--from-stdin` for batch ingest. | yes (toml) |
| `rmap status <ids> <state>` | Bulk status transition (atomic). | yes (toml) |
| `rmap mark <id> +x -y` | Add/remove markers idempotently. | yes (toml) |
| `rmap depend <id> on <id>` | Add in-repo or `--cross-repo` dependency. | yes (toml) |
| `rmap diff` | Show changes vs `--against <ref>` (default: `default_branch`). | no |
| `rmap delegate <id> --to codex\|cursor` | Format task as a cloud-agent prompt. | no |
| `rmap stale` | In-progress tasks idle past a threshold (`7d`/`2w`/`1y`). | no |
| `rmap doctor` | Aggregate soft signals (validate, stale, score decay, drift, …). | no |
| `rmap schema --json` | Emit JSON Schema for `tasks.toml` (agent self-description). | no |

All mutators follow a **validate-then-write** contract: mutate in memory, re-validate, refuse to write if invalid. Bulk `rmap status 1,2,3 done` is all-or-nothing.

## Data model

`roadmap/tasks.toml` minimum:

```toml
schema_version = 2
project = "my_app"
default_branch = "main"

[[task]]
id = 1                            # numeric or string
phase = 1
status = "pending"                # pending | in_progress | blocked | done
title = "Wire up the database"
scores = { d = 4, b = 8, u = 7 }  # difficulty, benefit, urgency
```

Optional, all additive:

- `[phases.<N>]`, `[bundles.<name>]` — grouping and ordering
- `[focus] phase = N` — bias `rmap next` toward a focus phase; surfaces a FOCUS block in `ROADMAP.md`
- `[linear] team_key = "ENG"` — opt-in Linear cross-reference (validated only when present)
- per-task: `markers`, `depends_on`, `cross_repo`, `module`, `assignee`, `model`, `linear_id`, `acceptance_criteria`, `blocked_reason`, `started_at`, `done_at`, `shipped_in`, `scored_at`

**Efficiency** is computed at read time as `(b + u) / (2 * d)`, never stored. Scores older than 30 days get a decay suffix in the rendered view.

## Render marker contract

`rmap render` only rewrites bytes between matched marker pairs in `ROADMAP.md`:

```markdown
<!-- TASKS:BEGIN phase=1 -->
…generated rows…
<!-- TASKS:END -->

<!-- FOCUS:BEGIN -->
…generated focus block…
<!-- FOCUS:END -->

<!-- MERMAID:BEGIN -->
…generated mermaid gantt block (one section per phase, one row per task with started_at)…
<!-- MERMAID:END -->
```

Everything outside the markers — your headings, prose, links, blank lines — is **byte-equal in/out**. You can freely hand-edit narrative around the generated tables. Drop in a marker pair when you want the corresponding block; omit it for zero-config default.

## Agent contract

The `--json` outputs of `show`, `list`, `next`, `schema`, `diff`, and `doctor` are stable surfaces:

- **append-only** — fields may be added; renames or removals require bumping `schema_version`
- `schema --json` emits the live JSON Schema derived from the Rust types (no hand-authored parallel schema)
- `SKILLS.md` is the agent-facing user manual; every fenced command in it runs in CI (`tests/skills_smoke.rs`)
- `rmap delegate <id> --to <agent>` formats a Markdown prompt + per-agent environment notes (mirroring `~/.claude/includes/cloud-agent-environments.md` when present)

## Determinism

Any "today"-sensitive logic (score decay, `stale`, focus shipping window) routes through `today_iso()`, which honors the `RMAP_TODAY` env var. Useful for golden tests and reproducible CI:

```bash
RMAP_TODAY=2026-05-12 rmap render
```

## Exit codes

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | Validation error (schema, dependency cycle, unknown id, bad marker, …) |
| 2 | `validate --check-render` drift only |

`rmap doctor` always exits 0 on parseable input — it is informational, not a gate. Pipe through `jq` for CI: `rmap doctor --json | jq -e '.findings | length == 0'`.

## Development

```bash
cargo build
cargo test                                  # unit + integration + golden + skills smoke
cargo test --test cli                       # CLI-only
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Golden fixtures live under `tests/golden/<case>/` — drop a `tasks.toml` + `ROADMAP.input.md` + expected `ROADMAP.md` triplet and the runner picks it up. Pin date-sensitive fixtures with a `today.txt`.

See `CLAUDE.md` for the load-bearing invariants (marker byte-preservation, validate-then-write contract, three-place edits for new `Task` fields, …), `DESIGN.md` for the design contract and Phase 6 HTML render design, and `ROADMAP.md` (rendered from `roadmap/tasks.toml`) for the active work list.

## Scope

Deliberately **in scope**: validation, rendering, queries, mutations, agent contracts, `git show`-based diff.

Deliberately **out of scope**: Linear API calls, web server, deep git integration, shell completions, CI workflows, multi-user sync. `rmap` is a file format and a CLI; orchestration lives elsewhere.

## License

TBD.
