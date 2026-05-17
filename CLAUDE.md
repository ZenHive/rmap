# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project, regardless of language. It renders `ROADMAP.md` and `roadmap/data.json` from the TOML source. Since 2026-05-13 rmap drives its own roadmap from `roadmap/tasks.toml` → `ROADMAP.md`; see `DESIGN.md` for the design contract and `AGENTS.md` for additional contributor guidelines.

## Imports

Universal includes (per `~/.claude/setup-guide.md`). No Rust-specific template exists; rmap takes the universal baseline only — the Elixir/Phoenix includes don't apply. Delegation includes are intentionally omitted (no `.mcp.json`, no git remote — rmap is not in the Linear/cloud-agent queue).

@~/.claude/includes/across-instances.md
@~/.claude/includes/critical-rules.md
@~/.claude/includes/worktree-workflow.md
@~/.claude/includes/task-prioritization.md
@~/.claude/includes/task-writing.md
@~/.claude/includes/rmap.md
@~/.claude/includes/workflow-philosophy.md
@~/.claude/includes/web-command.md

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

**Reinstall after any source change.** rmap is dogfooded on itself, so a stale `~/.cargo/bin/rmap` will silently render with the old schema or reject TOML using a newly-added field. After any change to `src/`, run `cargo install --path .` before invoking `rmap` again. The `skills_smoke` test compiles fresh under cargo and catches `SKILLS.md` regressions regardless of the installed binary; interactive `rmap` calls do not.

**Prefer `rmap` CLI over direct `tasks.toml` edits when a mutator exists.** Today the mutator surface is `rmap status` (single + bulk), `rmap mark`, `rmap depend`, and `rmap new` — all go through `toml_edit` and validate-then-write. Direct edits are the only path for everything else (bundle/phase CRUD, focus, scores, titles, ACs, top-level metadata); after any direct edit run `cargo run -- validate --check-render` before committing.

**Keep `~/.claude/includes/rmap.md` in sync.** It's the consumer-facing decision-layer doc (which command, when) imported by every rmap-using project's CLAUDE.md, including this one. Update it in the same commit when the command surface or a user-visible schema affordance changes. `rmap.md` deliberately does NOT enumerate fields — `rmap schema` / `rmap --help` is authoritative.

**Evaluate roadmap & task design from the consuming-agent POV.** rmap IS a tool for Claude. When picking up rmap tasks or reviewing the roadmap, evaluate the design from the perspective of the agent (Claude) that will actually consume the tool day-to-day — not just the perspective of the AC author. If the AC overspecifies in ways that hurt daily ergonomics, push back and propose the consumer-first alternative in the plan. Examples: a transition that requires manual TOML edit between two commands is friction Claude will route around; a field that's not surfaced in `rmap show` is invisible to the consumer; a section ordering optimized for one rare-path command (delegate) at the cost of the daily-path command (show) is the wrong tradeoff. (`feedback_decide_as_consumer.md` captures the same posture as personal memory; this rule lifts it to the project so it's authoritative for all rmap work.)

## Architecture

Pipeline: `tasks.toml` → `schema::Tasks` → `render_roadmap_str` (markdown), `export_json_str` (data.json), or `render_html_str` (static HTML, opt-in via `--html`) → write back. Mutations route through `toml_edit` to preserve user formatting and comments.

Modules — each file's doc comment is the authoritative reference for its internals:

- `schema.rs` — `serde` structs, `#[serde(deny_unknown_fields)]` everywhere. `TaskId` is `Number(u32) | Text(String)` untagged.
- `validate.rs` — semantic checks on a parsed `Tasks` (versions, statuses, markers, score range, deps, cycles, references, timestamps).
- `render.rs` — three-pass marker walker over `ROADMAP.md`: focus → mermaid → tasks. Only bytes between matched marker pairs are rewritten.
- `export.rs` — `ExportedTask` JSON shape; adds computed `eff`. Pretty by default; `_compact_` variant feeds the HTML data island.
- `render_html.rs` — `rmap render --html` single-project view via the one minijinja template (`templates/roadmap.html.j2`, `include_str!`'d). Builds DAG via longest-path layered layout.
- `mutate.rs` — `status` / `mark` / `depend` / `new` paths. All use `toml_edit::DocumentMut` and end with `validate_tasks_str` before returning.
- `next.rs` — pure selector: `next_tasks(tasks, filter, count)` returns highest-Eff pending tasks with deps done. Focus-phase candidates win over higher-Eff out-of-focus.
- `query.rs` — `show` / `list` read paths. `TaskFilter` + `find_task` + `list_tasks` pure; humans get `format_task*`, JSON delegates to `export.rs`.
- `bundles.rs` — `rmap bundles` read path. Per-bundle `next_task` reuses `next::next_task`.
- `next_bundle.rs` — `rmap next-bundle` pure selector. Broad actionability (in-bundle pending deps satisfy if themselves actionable). Topological emit via Kahn's.
- `delegate.rs` — `rmap delegate` prompt formatter. Read-only Markdown; never calls Linear/GitHub/Slack.
- `import.rs` — `rmap import` prompt formatter. Interpolates project name + live JSON Schema into `templates/import_prompt.md`.
- `diff.rs` — `rmap diff` engine. `diff_toml(base, current, verbose) -> TomlDiff`. Per-task field walk hand-maintained via `diff_fields!` macro.
- `schema_json.rs` — JSON Schema for `Tasks` via `schemars` derives.
- `scoring.rs` — shared `efficiency`, `format_efficiency`, `tier_glyph`, date helpers. Pure-integer Howard Hinnant date math; no chrono.
- `stale.rs` — pure `parse_duration` + `find_stale`. No I/O.
- `doctor.rs` — soft-signal aggregator. Always exits 0; strict gates remain on `validate`.
- `paths.rs` — ancestor walk to find `roadmap/tasks.toml`. CLI flag overrides; `html_path` is fixed-derived.
- `watch.rs` — pure helpers for `rmap watch`. `write_if_changed` (idempotency primitive), `is_tasks_toml_event` (filter), JSON event-line builders.
- `main.rs` — `clap` derive CLI. `run() -> Result<ExitCode>`. Wires every command to its module.

## Load-bearing invariants

Easy to violate without breaking tests immediately. The "why" lives in source doc comments and tests; this list is the index.

**Render & markers**
- **Marker boundaries are byte-preserved** (TASKS / FOCUS / MERMAID / VISION). Don't normalize input bytes outside matched pairs.
- **FOCUS / MERMAID / VISION line shape and empty-state strings are agent-grep contract.** Changing wording is a `schema_version` bump.
- **Archive collapse triggers on `phases.N.status = "done"`** and emits the one-line `See [CHANGELOG.md#…]` body. Line shape is locked by `validate --check-render`.

**Schema & validation**
- **`schema_version = 2` is required.** Bump on any breaking schema change.
- **`eff` is never persisted.** Computed at render/export time; `schema::Task` would reject it via `deny_unknown_fields`.
- **D/B/U range is `1..=10`.** The error message string `must be in 1..=10` is part of the agent-grep contract.
- **`linear_id` validation is conditional** on `[linear]` table presence (Linear is opt-in).
- **`blocked_reason` is required iff `status = "blocked"`.** Mutator re-validates, so the transition can't write without one.
- **`implemented` is required and non-empty iff `status = "done"`.** Mirrors the `blocked_reason` pattern. Error string `is done but missing implemented` is agent-grep contract.
- **Timestamps validate by shape (`YYYY-MM-DD`), not semantics.** `9999-99-99` passes on purpose; values live next to user-edited TOML.
- **Status / marker / cross-repo-relation enums live in `validate.rs` constants**; render-time match arms in `render.rs` don't share a source — keep both in sync.
- **Milestone status enum (`pending | active | done`) lives in `validate.rs::VALID_MILESTONE_STATUSES`** — distinct vocabulary from task status. `rmap milestones` sort order is `(status_rank: active=0/pending=1/done=2 asc, milestone.order asc)`; "active first" is load-bearing for the daily release-cut query.
- **`task.milestone` references must resolve in `tasks.milestones`.** `validate_milestone_references` enforces this; mutator pre-validates before writing.

**Mutations**
- **All mutators use `toml_edit::DocumentMut`** (never `toml::from_str`) and end with `validate_tasks_str(...)` before returning. Invalid mutations leave the file byte-equal.
- **Bulk `rmap status 1,2,3 done` is atomic** — all-resolve-or-no-write. Don't add a "best effort" flag without explicit user request.
- **`rmap status` is the only mutator that auto-fills lifecycle timestamps** (`done_at`, `started_at`). Never overwrites existing values. Changing this is a `schema_version` bump.
- **`rmap mark` and `rmap status` auto-sort task keys when inserting a new field**; idempotent calls do not. `add_dependency_str` deliberately does NOT auto-sort.
- **`rmap new` auto-allocates numeric IDs only.** `TaskId::Text` is never auto-generated. Duplicate explicit IDs error before any write.
- **Lifecycle timestamps are NOT settable on creation.** `started_at`, `done_at`, `blocked_reason`, `shipped_in` are absent from `NewTaskFields` — those are transition fields owned by `rmap status`.

**Agent contract (renaming/removing breaks consumers)**
- **`--json` outputs of `show` / `list` / `next` / `next-bundle` / `bundles` / `schema` / `diff` / `doctor` are additive-only.** Add fields freely; rename/remove → `schema_version` bump.
- **`rmap next --count` JSON shape is split by N**: default (`--count 1`) emits a bare object/null; `--count >1` emits an array. Flipping default-to-array is a bump.
- **`rmap next-bundle` ranking is `(in_focus_phase desc, sum_eff desc, bundle.order asc)`** and the three empty-state stderr spellings are load-bearing.
- **`rmap bundles` row separator and five-branch glyph ladder** (`✅` / `🚧` / `all-blocked ⛔` / `pending:<n> (deps unmet) ⏸` / `next:<id> [Eff:x.y] <tier_glyph>`) are agent-grep contract.
- **`rmap milestones` mirrors `rmap bundles`'s five-branch glyph ladder** and adds a trailing `[target=<version>]` segment when `milestone.target_version` is set. Sort key and row shape are agent-grep contract.
- **Render-row 🚀 segment is conditional + positional**: `🎁 **bundle** · 🚀 **milestone** · {module} · {category} {title}`. Inserted between bundle and module_segment; emitted only when `task.milestone.is_some()`. Rows without a milestone render byte-identically to pre-Task-24 — regression-guarded by golden fixtures.
- **Eff tier glyph is centralized in `scoring::tier_glyph`** (`>=2.0 🎯 / >=1.5 🚀 / >=1.0 📋 / else ⚠️`). NEVER fold into `format_efficiency` — JSON payloads must stay numeric.
- **`rmap delegate`'s seven canonical `##` sections** (`Context` → `Task` → `Acceptance criteria` → `Out of scope` → `Files to modify` → `Scoring` → `Environment notes`) and the `[D:_/B:_/U:_ → Eff:_] <glyph>` Scoring shape are locked by `emits_canonical_section_order_with_distinguishing_line`.
- **`rmap delegate`'s per-agent footer mirrors `~/.claude/includes/cloud-agent-environments.md`** — sync manually when the skill changes.
- **`rmap diff --against` defaults to `current.default_branch`** — never hardcode `"main"`.
- **`rmap diff --verbose` is additive.** Non-verbose output stays byte-identical to pre-11b. Whitelist members in `TASK_VERBOSE_WHITELIST` / `METADATA_VERBOSE_WHITELIST` are part of the contract.
- **HTML data island id is `rmap-data`, script type `application/json`.** Agents parse the element's text; they do NOT scrape the DOM.
- **HTML task cards carry six `data-*` attributes** (`data-id`, `-status`, `-eff`, `-markers`, `-depends-on`, `-phase`); DAG nodes carry `data-id`; phase sections carry `data-phase-status`. Stable selector contract.

**Three-place edit rules**
- **New field on `schema::Task` → also edit `diff::diff_fields!` AND `export::ExportedTask`** in the same commit. Otherwise `diff` misses it and `show --json` / `list --json` don't surface it. Decide whether to add to `TASK_VERBOSE_WHITELIST`.
- **New top-level field on `schema::Tasks` → also edit `diff::diff_metadata` AND `export::ExportedTasks`** (Task-level macro doesn't cover them; hand-walked).

**Time & determinism**
- **`today_iso()` is the only source of "now".** Reads `RMAP_TODAY` env var first, falls back to `SystemTime::now()`. Date-sensitive tests MUST set `RMAP_TODAY` on the `Command` env (or `today.txt` for golden fixtures).

**Watch**
- **`rmap watch` watches the `roadmap/` directory** (not the file) with `RecursiveMode::NonRecursive`, and filters via `is_tasks_toml_event`. The filter is the infinite-loop guard against our own `data.json` write.
- **`rmap watch` event shape is the agent contract** — `schema_version` + `event` discriminator (`rendered` / `error`) + `outputs` / `message` are additive-only. Bump `WATCH_SCHEMA_VERSION` for renames.

**Exit codes**
- **`rmap doctor` always exits 0 on success** (informational only). Strict gates: `validate` (exit 1 on schema error), `validate --check-render` (exit 2 on render drift).

## Tests

- `tests/cli.rs` — black-box CLI tests via `Command::new(env!("CARGO_BIN_EXE_rmap"))`. Each test gets a unique temp dir from a per-test atomic counter. Date-sensitive tests set `RMAP_TODAY` on the `Command` env.
- `tests/skills_smoke.rs` + `tests/skills_fixture/` — parses every fenced ```bash``` block in `SKILLS.md`, extracts the optional `# exit: <N>` annotation (default 0), and runs each `rmap ` invocation in a fresh fixture copy with `RMAP_TODAY=2026-05-12` pinned. Agent-contract gate for `SKILLS.md` — renaming or removing a documented command requires updating both `SKILLS.md` and the fixture in the same commit.
- `tests/golden/<case>/` — fixture triples: `tasks.toml`, `ROADMAP.input.md`, `ROADMAP.md` (expected output). Optional `today.txt` pins the render date. Add `today.txt` to any fixture using `scored_at` to avoid drift into score-decay.
- `tests/roundtrip.rs` — parse → `toml_edit` round-trip → assert no spurious diff. Catches comment-preservation regressions.

## Scope discipline

`ROADMAP.md` (rendered from `roadmap/tasks.toml`) tracks open phases; `DESIGN.md` carries the design contract and out-of-scope list; `CHANGELOG.md` is the shipped-phase record. Implemented today: validate, render (incl. `--html` static single-project view), watch, export json, status (single + bulk), mark, depend, new (interactive + `--from-stdin`), next, show, list, bundles, schema, diff, delegate, import, stale, doctor. Score-decay rendering is automatic on tasks with `scored_at` >30d or missing. `--html --multi` portfolio view is still open (Task 9). Deliberately out of scope (per `DESIGN.md`): Linear API calls, web server, git integration beyond `git show <ref>:<path>` for `rmap diff`, shell completions, CI workflow, multi-user sync. Don't add these without checking the roadmap first.
