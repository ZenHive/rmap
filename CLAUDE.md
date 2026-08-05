# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project, regardless of language. It renders `ROADMAP.md` and `roadmap/data.json` from the TOML source. Since 2026-05-13 rmap drives its own roadmap from `roadmap/tasks.toml` → `ROADMAP.md`; see `DESIGN.md` for the design contract and `AGENTS.md` for additional contributor guidelines.

## Imports

Eager floor per `~/.claude/setup-guide.md` § "Selective-Load Philosophy" (Opus 4.8). Two eager includes only:

- **`critical-rules`** — the portfolio-wide hard-guardrail floor; must stay ambient (a guardrail the model invokes "when relevant" fails exactly when it doesn't realize the rule applies).
- **`harness-workflow`** — the new default second eager include for harness-registered repos, and rmap IS one (registered in harness's `config/dev.local.exs`, MCP wired via `.mcp.json` — see § "Driving harness from this repo"). The implement→review→land loop and its delegation roster (cursor/codex/grok first, opus last) are load-bearing every session, not on-demand reference.

```markdown
@~/.claude/includes/critical-rules.md
@~/.claude/includes/harness-workflow.md
```

Everything else is **skill-on-demand** (Opus 4.8 self-invokes the matching skill): `worktree-workflow` (`workflow:git-worktrees`), `task-prioritization` (`tasks:roadmap-planning`), `task-writing` (`tasks:task-writing`), `rmap` (`tasks:rmap`), `workflow-philosophy` (`workflow:workflow-philosophy`). Re-add an `@`-import per-surface only if you observe Opus failing on it eager.

Deliberately NOT imported: `across-instances` (rmap is a work/tool repo, not a presence repo); Elixir/Phoenix includes (don't apply to a Rust CLI); delegation includes (rmap has a GitHub remote `ZenHive/rmap` but no Linear/cloud-agent PR flow); `web-command` (rmap does no browser work — the skill covers the rare case).

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

**Bump `version` in `Cargo.toml` on every user-visible change.** `rmap --version` reports the crate version, and rmap is dogfooded across the portfolio — a stale version means neither the user nor a consuming agent can tell which binary is installed, and "reinstall and retry" stops being a checkable instruction. Bump in the same commit as the change, never as a follow-up. On a 0.x CLI: a new command, flag, task field, JSON key, or render affordance is a **minor** bump (`0.N+1.0`); a bugfix, message change, or docs-only edit is a **patch** bump. This is orthogonal to `schema_version` — a breaking agent-contract change bumps both. Pair the bump with a `CHANGELOG.md` entry under `[Unreleased]` (cutting a release renames that heading to the version with a date), and re-run `cargo install --path .` afterwards so `rmap --version` on this machine matches the source.

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
- `render_html.rs` — `rmap render --html` single-project view (`templates/roadmap.html.j2`) and `--html --multi` portfolio view (`templates/portfolio.html.j2`), sharing component macros (`templates/_components.html.j2`) and one stylesheet (`templates/_styles.css`); all minijinja, all `include_str!`'d. Builds DAGs via the longest-path layering in `topo.rs`. Portfolio inputs are `ProjectInput` envelopes (project root, `tasks.toml`, or `data.json` path — see `main.rs::load_project_input`).
- `topo.rs` — pure longest-path layering `compute_layers(tasks) -> {id → depth}` over the in-repo `depends_on` graph (extracted from `render_html`), plus `compute_layers_from_edges` over pre-extracted `(id, deps)` lists (the portfolio's JSON-loaded tasks). Shared by `render_html.rs` (DAG vertical slotting) and `export.rs` (the computed `dep_layer` field).
- `graph_export.rs` — read-only graph surfaces over the in-repo `depends_on` graph. `build_waves` / `format_waves` / `format_waves_json` (`rmap waves` — parallel dispatch schedule grouped by `topo` `dep_layer`, wave 0 = roots) and `build_dot` / `format_dot` (`rmap export dot` — Graphviz DOT digraph, edges dependency → dependent). Both pure; never mutate.
- `mutate.rs` — `status` / `mark` / `depend` / `new` paths. All use `toml_edit::DocumentMut` and end with `validate_tasks_str` before returning.
- `next.rs` — pure selectors: `next_tasks(tasks, filter, count)` (highest-Eff pending unblocked) and `ready_tasks(tasks, filter, count, dispatchable)` (the whole parallel-safe set), both ranked via shared `next::rank_tasks` — a 4-tier lexicographic key (focus phase × active milestone, **focus dominant**) then Eff descending. `is_unblocked` is the dep-satisfied predicate; the `active` milestone set is computed once per call from `tasks.milestones`.
- `query.rs` — `show` / `list` read paths. `TaskFilter` + `find_task` + `list_tasks` pure; `effective_target_repo` resolves an omitted task target to the roadmap's `project`; humans get `format_task*`, JSON delegates to `export.rs`.
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
- **Marker boundaries are byte-preserved** (TASKS / FOCUS / MERMAID / VISION / MILESTONES). Don't normalize input bytes outside matched pairs.
- **FOCUS / MERMAID / VISION line shape and empty-state strings are agent-grep contract.** Changing wording is a `schema_version` bump.
- **Archive collapse triggers on `phases.N.status = "done"`** and emits the one-line `See [CHANGELOG.md#…]` body. Line shape is locked by `validate --check-render`.

**Schema & validation**
- **`schema_version = 2` is required.** Bump on any breaking schema change.
- **`eff` is never persisted.** Computed at render/export time; `schema::Task` would reject it via `deny_unknown_fields`.
- **D/B/U range is `1..=10`.** The error message string `must be in 1..=10` is part of the agent-grep contract.
- **`linear_id` validation is conditional** on `[linear]` table presence (Linear is opt-in).
- **`blocked_reason` is required iff `status = "blocked"`.** Mutator re-validates, so the transition can't write without one. Settable via `rmap status <id> blocked --reason "<text>"` (free-text, overwrites, blocked-only — ignored with a one-line stderr note on other transitions, mirroring the outcome flags). **Auto-cleared when a blocked task leaves the blocked state** (the reason described a state that no longer holds); re-blocking keeps/overwrites. No interactive prompt — if neither `--reason` nor an existing value is present, the validation error fires and the file stays byte-equal.
- **`implemented` is required and non-empty iff `status = "done"`.** Mirrors the `blocked_reason` pattern. Error string `is done but missing implemented` is agent-grep contract.
- **`model` is required and non-empty iff a *live agent-assigned* task** — `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`. Hard `validate` error (`validate_dispatch_model`), not a soft advisory: harness hard-rejects a dispatch that resolves to no model (it never falls through to the agent CLI's ambient default), and the roadmap is the source of truth harness ingests, so rmap refuses to author a dispatch that would be rejected. Scope is narrow by design — **terminal tasks** (`done`/`superseded`/`blocked`) are exempt (a missing model is now historical, not an authoring error, and requiring it would retroactively fail `validate` on shipped work), and **assignee-unset / `human`** tasks are exempt (no agent chosen → nothing to pin). Mutators inherit the gate via validate-then-write (`rmap new --assignee <agent>` on a model-less task fails before write). Error string `missing model` is agent-grep contract. No `schema_version` bump (`model` already exists per Task 19).
- **`acceptance_criteria` is required and mechanically non-blank iff a *live agent-assigned* task** — the same predicate and exemptions as `model`. rmap enforces only the presence of observable completion criteria; AI task authors and reviewers judge whether those criteria are meaningful and reality-grounded. Error substring `missing acceptance_criteria` is agent-grep contract.
- **`delivered_by`, `verified`, `verified_by`, and `verification_ref` are transition-time outcome fields settable only on `status = "done"`.** `delivered_by` names the implementer. `verified = true` means an independent evaluator confirmed the task; a new `--verified` transition hard-requires non-empty `--verified-by`, while optional `--verification-ref` points durably to the evidence (harness run, CI URL, review artifact). Direct TOML provenance is rejected when blank or when `verified != true`. Legacy schema-v2 rows with `verified = true` but no `verified_by` remain valid and produce soft `VerifiedWithoutProvenance` doctor findings; this avoids retroactively invalidating every existing roadmap without a schema-v3 migration. `verified` absent remains honestly ungraded and produces `ClaimedNotGraded`.
- **`attempts` is an append-only transition-time list, settable only on `status = "pending"`.** Each entry is an inline-table `{ at, by?, report }` (stored like `cross_repo`): `at` is auto-filled from `today_iso()`, `by` is free-text agent attribution (optional), `report` is the failure evidence (a reviewer's rejection report). Appended — never overwritten — by `rmap status <id> pending --report "<text>" [--attempt-by <agent>]`; each call adds exactly one entry (no dedup). `--report` on a non-`pending` transition is ignored with a one-line stderr note (mirrors the outcome/`--reason` flags); `--attempt-by` without `--report` is likewise a no-op note. Renders in `rmap show` and `rmap delegate`'s `## Prior attempts` section; surfaces in `--json` / `data.json` (skipped when empty, so attemptless tasks round-trip byte-identically). Transition-time field → mirror surfaces `canonical_task_key_index` (index 32, trailing) + `diff_fields!` + `ExportedTask`/`EXPORTED_TASK_FIELDS`; deliberately NOT in `TASK_VERBOSE_WHITELIST` (a list, like `cross_repo`).
- **`rmap doctor` milestone drift advisories are soft (exit 0, no auto-mutation).** `MilestoneFullyDoneButOpen` when every task pinned to a milestone is `done` but milestone status is `pending` or `active`; `MultipleActiveMilestones` when more than one milestone is `active` (rmap.md: keep exactly one). Human lines cite milestone slug and pinned-task count; JSON kinds `milestone_fully_done_but_open` / `multiple_active_milestones`.
- **`rmap doctor` phase / focus drift advisories are soft (exit 0, no auto-mutation; phase/focus state is user-curated).** `PhaseFullyDoneButOpen` when every task in a phase is `done` but phase status is still `pending`/`active`; `PhaseHasInProgressButPending` when a `pending` phase has ≥1 `in_progress` task; `FocusPhaseClosed` when `focus.phase` points at a phase that appears closed (done status or all tasks done). Human lines cite the phase number (and task ids for the in-progress case); JSON kinds `phase_fully_done_but_open` / `phase_has_in_progress_but_pending` / `focus_phase_closed`.
- **`rmap doctor` graph-health advisories are soft (exit 0, no auto-mutation).** `Bottleneck` when a `pending`/`blocked` task's transitive-dependent count (via `topo::compute_unlocks`) is ≥ `DoctorThresholds.bottleneck_min` (default 3, overridable via `--bottleneck-min`); `IsolatedNode` when a task is an orphan (no in-repo deps and no dependents) or unreachable forward from any milestone-pinned task when milestones exist. Human lines cite task id + dependent count (bottleneck) or id + reason (isolated); JSON kinds `bottleneck` / `isolated_node`.
- **`rmap doctor` spec-quality advisories are soft (exit 0, no auto-mutation).** Token-mechanical only (no semantic/LLM judgment). **AC quality** is scoped to *live agent-assigned* tasks (same predicate as `validate_dispatch_model`: `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`): `PlaceholderCriteria` when any criterion has an unquoted whole-word `TODO`/`TBD`, substring `???`, or `<stub>` angle-bracket token (quoted example mentions are ignored so prose ACs that name the tokens do not self-flag); `VagueCriteria` when a criterion's non-stopword tokens are drawn entirely from the fixed list `fast` / `robust` / `properly` / `correctly` / `works` (a measurable criterion that merely contains one of those words does not fire). **Near-duplicates** pair open tasks (`pending`/`in_progress`/`blocked` only — never `done`/`superseded`) whose normalized title+body token Jaccard is ≥ `DoctorThresholds.near_duplicate_min` percent (default 80, overridable via `--near-duplicate-min`). Human lines cite task id(s); JSON kinds `placeholder_criteria` / `vague_criteria` / `near_duplicate_tasks` (additive).
- **`touches` is an optional creation-time free-text list, advisory and unvalidated** (posture of `model` / `assignee`). Semantically distinct from `files_to_modify`: `files_to_modify` is the implementer's *write target*; `touches` is the broader *involvement hint* (files that may be read or written) — typically a superset. Consumer collision rule (documented, NOT enforced in rmap): two tasks conflict iff `(touches(A) ∪ files_to_modify(A)) ∩ (touches(B) ∪ files_to_modify(B)) ≠ ∅`. Creation-time field → all six mirror surfaces (see the `Task` doc comment in `schema.rs`); on `diff_fields!` but deliberately NOT in `TASK_VERBOSE_WHITELIST`.
- **`domains` is an optional creation-time free-text list, advisory and unvalidated.** It tags the capability/domain evidence an orchestrator may group by (for example `rust`, `otp`, `ecto`); rmap owns no vocabulary and only carries/export the strings. Creation-time field → all six mirror surfaces; on `diff_fields!` but deliberately NOT in `TASK_VERBOSE_WHITELIST` (a list, like `touches` / `cross_repo`).
- **`target_repo` is the optional creation-time landing-repository field.** It names where this task's own work lands; absent resolves to the roadmap's top-level `project`, preserving existing source/render bytes. An explicit blank value is invalid. `cross_repo` remains a list of relationships to related tasks in other roadmaps and does not affect landing. `rmap list --target-repo` filters on the effective value and composes with other list selectors; `rmap delegate` always renders the effective value. Creation-time field → all six mirror surfaces and `TASK_VERBOSE_WHITELIST`.
- **Timestamps validate by shape (`YYYY-MM-DD`), not semantics.** `9999-99-99` passes on purpose; values live next to user-edited TOML.
- **Status / marker / cross-repo-relation enums live in `validate.rs` constants**; render-time match arms in `render.rs` don't share a source — keep both in sync.
- **Milestone status enum (`pending | active | done`) lives in `validate.rs::VALID_MILESTONE_STATUSES`** — distinct vocabulary from task status. `rmap milestones` sort order is `(status_rank: active=0/pending=1/done=2 asc, milestone.order asc)`; "active first" is load-bearing for the daily release-cut query.
- **`task.milestone` references must resolve in `tasks.milestones`.** `validate_milestone_references` enforces this; mutator pre-validates before writing.
- **`TaskId` `Eq`/`Hash` are normalizing across `Number(n)` ↔ `Text("n")`.** A task id is a primary key; the disk form (TOML integer vs TOML string of the same digits) does not change which task it names. `validate_unique_ids` relies on this to catch cross-form collisions; `validate_dependencies` / `validate_dependency_cycles` / `doctor.rs` degenerate-bundle check / `next_bundle.rs` actionability memo are correct on mixed-form files because of it. Text ids that don't parse as `u32` (e.g. `"INE-5"`, `"alpha"`) keep their own canonical key. Agent-grep substring for the duplicate-id error is `duplicate task id`.

**Mutations**
- **All mutators use `toml_edit::DocumentMut`** (never `toml::from_str`) and end with `validate_tasks_str(...)` before returning. Invalid mutations leave the file byte-equal.
- **Bulk `rmap status 1,2,3 done` is atomic** — all-resolve-or-no-write. Don't add a "best effort" flag without explicit user request.
- **`rmap status` is the only mutator that auto-fills lifecycle timestamps** (`done_at`, `started_at`). Never overwrites existing values. Changing this is a `schema_version` bump.
- **`rmap mark` and `rmap status` auto-sort task keys when inserting a new field**; idempotent calls do not. `add_dependency_str` deliberately does NOT auto-sort.
- **`rmap new` auto-allocates numeric IDs only.** `TaskId::Text` is never auto-generated. Duplicate explicit IDs error before any write.
- **Lifecycle timestamps are NOT settable on creation.** `started_at`, `done_at`, `blocked_reason`, `shipped_in` are absent from `NewTaskFields` — those are transition fields owned by `rmap status`.

**Agent contract (renaming/removing breaks consumers)**
- **`--json` outputs of `show` / `list` / `next` / `next-bundle` / `ready` / `bundles` / `schema` / `diff` / `doctor` / `blocks` / `deps` are additive-only.** Add fields freely; rename/remove → `schema_version` bump. `blocks` / `deps` emit a `list`-shaped envelope (`export_filtered_json_str`).
- **`rmap next --count` JSON shape is split by N**: default (`--count 1`) emits a bare object/null; `--count >1` emits an array. Flipping default-to-array is a bump.
- **`rmap next` ranking is 4-tier lexicographic** `(in_focus_phase × in_active_milestone) ⇒ Eff desc`, focus dominant: tier 0 (both) > tier 1 (focus-only) > tier 2 (active-milestone-only) > tier 3 (neither). Tasks pinned to ANY milestone with `status = "active"` qualify. Without `[focus]`, every task counts as "in focus" → tiers collapse to 0/1. The focus-dominance bit (tier 1 > tier 2) is the load-bearing decision. `next::rank_tasks` is the single source of this sort, shared with `ready`.
- **`rmap ready` is the parallel-safe dispatch set**: all `pending` tasks whose every `depends_on` is `done`, ranked by the same 4-tier key as `next` (via `next::rank_tasks`). The set is **mutually independent by construction** — a pending task whose deps are all `done` cannot depend on another pending task — so there is NO `--independent` flag (it would be a no-op). Unlike `next`, `--count` is optional (default = the whole set) and `--phase` filters the pool. `--bundle B` = the dispatchable layer-0 of B. `--json` is a `list`-shaped envelope.
- **`dep_layer` is never persisted.** Computed at export time (`src/topo.rs` longest-path depth over the in-repo `depends_on` graph); like `eff`, `schema::Task` rejects it via `deny_unknown_fields`. Always built from the FULL `tasks.task` graph, never a filtered slice — `export`'s slice-taking fns (`export_task_json_str`, `export_tasks_array_json_str`) take `&Tasks` for exactly this. Surfaces on every `--json` payload (additive).
- **`unlocks` is never persisted.** Computed at export time (`src/topo::compute_unlocks` — transitive-dependent count over the reverse `depends_on` graph); like `eff` / `dep_layer`, rejected by `deny_unknown_fields` and built from the FULL graph (`export::graph_metrics` bundles `layers` + `unlocks`, computed once per export call). It is the set-size of `rmap blocks <id>`; turns the `U`-score's unlock-leverage component into a graph fact. Surfaces on every `--json` payload (additive).
- **`--dispatchable` (on `ready` / `list`) excludes `handbuild`-marked tasks.** `handbuild` ∈ `VALID_MARKERS` flags human-driven-browser work (LiveView/UI/DOM) — the minority exception, so everything else is headless-dispatchable by default. `query::is_dispatchable` is the predicate; on `ready` it filters before `rank`+`count` so the cap counts only dispatchable tasks.
- **`rmap list --target-repo <repo>` filters by where each task's own work lands.** Explicit `task.target_repo` wins; an omitted field matches top-level `project`. The filter is AND-composed with status / phase / marker / bundle / milestone / delivered-by / dispatchable selectors. It never follows or filters `cross_repo` relationships.
- **`--fields a,b,c` (on `ready` / `list`) projects `--json` to a bare array** of objects carrying only the named keys (envelope dropped — token-cheap). Implies `--json`; unknown name → exit 1 naming the offender, validated against `export::EXPORTED_TASK_FIELDS`. Absent optional keys simply don't appear per task.
- **`rmap next-bundle` ranking is `(in_focus_phase desc, sum_eff desc, bundle.order asc)`** and the three empty-state stderr spellings are load-bearing.
- **`rmap bundles` row separator and five-branch glyph ladder** (`✅` / `🚧` / `all-blocked ⛔` / `pending:<n> (deps unmet) ⏸` / `next:<id> [Eff:x.y] <tier_glyph>`) are agent-grep contract.
- **`rmap milestones` mirrors `rmap bundles`'s five-branch glyph ladder** and adds a trailing `[target=<version>]` segment when `milestone.target_version` is set. Sort key and row shape are agent-grep contract.
- **MILESTONES marker section is opt-in and grouped**: `<!-- MILESTONES:BEGIN -->` / `<!-- MILESTONES:END -->` renders one block per milestone sorted like `rmap milestones`, including name, target_version, status glyph, hypothesis description, and done/total pinned-task counts. Roadmaps without the marker pair render byte-identically.
- **Render-row 🚀 segment is conditional + positional**: `🎁 **bundle** · 🚀 **milestone** · {module} · {category} {title}`. Inserted between bundle and module_segment; emitted only when `task.milestone.is_some()`. Rows without a milestone render byte-identically to pre-Task-24 — regression-guarded by golden fixtures.
- **Render-row `⛔ {blocked_reason}` segment is conditional + trailing**: appended after the tier glyph, emitted only when `task.status == "blocked"` and `blocked_reason` is non-empty. Non-blocked rows (and blocked rows are the only ones affected) render byte-identically otherwise — additive, golden-guarded by `tests/golden/mermaid_block`.
- **Eff tier glyph is centralized in `scoring::tier_glyph`** (`>=2.0 🎯 / >=1.5 🚀 / >=1.0 📋 / else ⚠️`). NEVER fold into `format_efficiency` — JSON payloads must stay numeric.
- **`rmap delegate`'s seven canonical `##` sections** (`Context` → `Task` → `Acceptance criteria` → `Out of scope` → `Files to modify` → `Scoring` → `Environment notes`) and the `[D:_/B:_/U:_ → Eff:_] <glyph>` Scoring shape are locked by `emits_canonical_section_order_with_distinguishing_line`.
- **`rmap delegate` always emits `Target repo` in `## Context`.** It uses explicit `task.target_repo` or falls back to top-level `project`, so the receiving agent can distinguish the planning repository from the checkout its work targets.
- **`rmap delegate --to` is optional; `assignee` is the routing default.** `delegate::resolve_target` resolves the target: explicit `--to` always wins (and renders the `Stored assignee: ... (overridden)` bullet when it differs); without it the task's `assignee` IS the target. No assignee → exit 1 `has no assignee; pass --to <agent>`; `assignee = "human"` → exit 1 `is assigned to human; pass --to <agent> to delegate anyway`. Both error strings are agent-grep contract. Routing metadata is split: `assignee` = which agent executes, `model` = free-text LLM id/pin, `domains` = free-text capability tags for downstream scoring, `delegate --to` = render-time override.
- **`rmap delegate`'s per-agent footer mirrors `~/.claude/includes/cloud-agent-environments.md`** — sync manually when the skill changes.
- **`rmap diff --against` defaults to `current.default_branch`** — never hardcode `"main"`.
- **`rmap diff --verbose` is additive.** Non-verbose output stays byte-identical to pre-11b. Whitelist members in `TASK_VERBOSE_WHITELIST` / `METADATA_VERBOSE_WHITELIST` are part of the contract.
- **HTML data island id is `rmap-data`, script type `application/json`.** Agents parse the element's text; they do NOT scrape the DOM.
- **HTML task cards carry six `data-*` attributes** (`data-id`, `-status`, `-eff`, `-markers`, `-depends-on`, `-phase`); DAG nodes carry `data-id`; phase sections carry `data-phase-status`. Stable selector contract.
- **Portfolio HTML (`--html --multi`) adds two islands**: `rmap-data` (aggregate `{"projects":[…]}` of every input's envelope) and `rmap-relations` (resolved cross-repo edge array `{source, target, relation}`); repo rows carry `data-slug` / `data-name` / `data-has-rel`. Same parse-the-island-not-the-DOM rule.

**Mirror-surface edit rules**

When adding a field to `schema::Task`, decide whether it is a **creation-time** field (set at `rmap new` time) or a **transition-time** field (set later by `rmap status` / `rmap mark` / `rmap depend` / etc.), then update the appropriate surfaces in the same commit. The full invariant lives on the `Task` doc comment in `src/schema.rs`; this is the working summary.

- **Creation-time field → SIX surfaces:**
  - `main.rs::StdinTask` (stdin parse shape)
  - `mutate.rs::NewTaskFields` (mutator argument struct)
  - `mutate.rs::add_task_str` (TOML writer)
  - `mutate.rs::canonical_task_key_index` (key ordering for serialization)
  - `diff.rs::diff_fields!` (drift surface for `rmap diff`)
  - `export.rs::ExportedTask` (`--json` / `data.json` shape) — AND `export.rs::EXPORTED_TASK_FIELDS` (the `--fields` projection's validation set; `exported_task_fields_cover_serialized_keys` guards drift). Any new `ExportedTask` field (creation-time, transition-time, or computed like `eff` / `dep_layer`) must be added to this const.
  - Then decide whether to add to `diff::TASK_VERBOSE_WHITELIST`. Interactive `prompt_task_fields` (main.rs) is optional — power-user fields (`branch`, `files_to_modify`, `touches`, `target_repo`, `cross_repo`) intentionally require `--from-stdin` rather than dialoguer.
- **Transition-time field** (lifecycle timestamps, `implemented`, outcome-layer, etc.) → update the owning mutator (`set_status_str` for status transitions, etc.) plus `diff::diff_fields!` and `export::ExportedTask`. Stays absent from `StdinTask` / `NewTaskFields` on purpose — today: `started_at`, `done_at`, `blocked_reason`, `shipped_in`, `implemented`, `delivered_by`, `verified`, `verified_by`, `verification_ref`, `attempts`.
- **New top-level field on `schema::Tasks`** → also edit `diff::diff_metadata` AND `export::ExportedTasks` (Task-level macro doesn't cover them; hand-walked).

**Time & determinism**
- **`today_iso()` is the only source of "now".** Reads `RMAP_TODAY` env var first, falls back to `SystemTime::now()`. Date-sensitive tests MUST set `RMAP_TODAY` on the `Command` env (or `today.txt` for golden fixtures).

**Watch**
- **`rmap watch` watches the `roadmap/` directory** (not the file) with `RecursiveMode::NonRecursive`, and filters via `is_tasks_toml_event`. The filter is the infinite-loop guard against our own `data.json` write.
- **`rmap watch` event shape is the agent contract** — `schema_version` + `event` discriminator (`rendered` / `error`) + `outputs` / `message` are additive-only. Bump `WATCH_SCHEMA_VERSION` for renames.

**Exit codes**
- **`rmap doctor` always exits 0 on success** (informational only). Strict gates: `validate` (exit 1 on schema error), `validate --check-render` (exit 2 on render drift).

## Downstream consumer: harness (`../harness/`)

The "consumers" the agent contract above protects are not hypothetical — the primary one is **harness**, a sibling Elixir/OTP project at `../harness/` (`/Users/efries/_DATA/code/harness/`). Harness is an AI-orchestrator-driven task-execution engine: it pulls tasks from rmap roadmaps, dispatches each to a headless coding agent (Claude Code, Codex, Cursor, Grok, Antigravity, Pi) in an isolated git worktree, grades the result with the target project's own check stack, and writes the verified outcome back via `rmap status`. Harness's CLAUDE.md § "rmap is ours" sends roadmap-CLI gaps *here* to be fixed, never worked around in harness — this section is the reciprocal pointer.

**The shell-out surface (`../harness/lib/harness/roadmap.ex`, `Harness.Roadmap`).** Harness never parses `tasks.toml` itself; it shells out to the installed `rmap` binary and treats stdout as API. Every call passes an explicit `--tasks-path`; success is gated on JSON-decode (or non-empty delegate output), not exit 0. Commands consumed:

- `rmap next --json` · `rmap show <id> --json` · `rmap list --json [--status S]` · `rmap next-bundle --json` — browse/ingest
- `rmap ready --dispatchable --fields id,assignee,markers` — the cron poller's autonomous selection surface (MCP tool `roadmap__ready`); the poller routes each task on its `assignee`
- `rmap delegate <id> --to <agent>` — the verbatim output IS the prompt dispatched to the agent
- Write-backs: `rmap status <id> in_progress` (on dispatch), `rmap status <id> done --verified --verified-by <reviewer> --verification-ref harness-run:<run-id> --shipped-in <sha>` (lander, after reviewer approval + push), `rmap status <id> blocked --reason "..."` (terminal sink)

Changing any of these — JSON shapes, `--fields` projection, `delegate` section format, status flags, the `handbuild` semantics of `--dispatchable` — means checking `Harness.Roadmap` (and `Harness.Dispatch` / `Harness.Lander` / `Harness.Cron.RoadmapPoller`) in the same change, plus the consumer-side docs below.

**Per-task target repositories are renderable, not executable in harness.** rmap carries `target_repo`, filters it, exports it, and renders it into delegate prompts. Harness does not yet resolve that value to a checkout or route dispatch/landing to another repository; that consumer-side work remains a harness roadmap task.

**Renderable ≠ executable (the two-sided executor contract).** `rmap delegate --to` renders for **eight** agents (`claude` / `codex` / `cursor` / `grok` / `antigravity` / `pi` / `droid` / `kimi`); harness has `AgentAdapter`s for only **six** — `droid` and `kimi` are renderable but rejected at harness's dispatch boundary (`{:unknown_adapter, "droid"}` / `{:unknown_adapter, "kimi"}`). Adding a new `--to` target in rmap is half the job: the agent only becomes dispatchable once harness grows a matching `AgentAdapter` + `@valid_agents` entry. When widening rmap's delegate/assignee set, note the harness-side gap explicitly (a task in harness's roadmap, or a line in the commit) rather than implying end-to-end support.

**Consumer-side contract docs (update when rmap's surface changes underneath them):**

- `../harness/skills/harness-driver/SKILL.md` — the AI-orchestrator contract for driving harness (dispatch patterns, MCP tool surface `dispatch__*` / `roadmap__*`, result shapes). It documents rmap-derived behavior (the `ready --dispatchable` set, delegate-rendered prompts, the renderable-vs-executable split) and carries an explicit anti-staleness contract.
- `../harness/CLAUDE.md` — § "Agent Headless Entry Points" and § "Dogfooding" reference rmap's delegate targets and selection commands.
- `../harness/docs/dogfooding-workflow.md` — the operator runbook; verdict table references rmap status write-backs.

Harness registers projects (including itself) with a `roadmap_path` and drives them through this surface unattended (`Oban.Plugins.Cron`) — a silent break in rmap's JSON or prompt output surfaces as failed autonomous dispatches there, not as an rmap test failure here. The `skills_smoke` test and the additive-only invariants above are the local proxies for that contract; treat them as guarding harness specifically.

### Driving harness from this repo

The relationship also runs the other way: rmap's own roadmap tasks can be dispatched *through* harness (Context A of the harness-driver skill — consuming repo drives the harness BEAM). The wiring:

- **`.mcp.json`** registers two HTTP servers against the harness BEAM (user-started `iex -S mix` in `../harness/`; never boot it yourself): `harness` → `mcp__harness__*` (the native flat driver tools — `dispatch__task`, `dispatch__await`, `dispatch__status`, `dispatch__verdict_detail`, `roadmap__*`; **primary surface**) and `harness_eval` → `mcp__harness_eval__project_eval` (arbitrary-Elixir escape hatch into harness's BEAM, for struct-level ops the flat tools omit).
- **rmap is registered as a harness project** in harness's gitignored `config/dev.local.exs` (`:rust` preset, `roadmap_path` = this repo). Registration changes need a harness BEAM restart (the user does that). Per-project cron autonomy defaults OFF — registration alone does not start autonomous dispatch.
- **The workflow loop is eager** (`@~/.claude/includes/harness-workflow.md` — see § "Imports"): the delegate → verify → repair → land loop and delegation roster are in context every session. **Load on demand when driving:** `../harness/skills/harness-driver/SKILL.md` (MCP tool shapes, dispatch patterns, sharp edges) and the `harness:harness-driver` skill.

## Tests

- `tests/cli.rs` — black-box CLI tests via `Command::new(env!("CARGO_BIN_EXE_rmap"))`. Each test gets a unique temp dir from a per-test atomic counter. Date-sensitive tests set `RMAP_TODAY` on the `Command` env.
- `tests/skills_smoke.rs` + `tests/skills_fixture/` — parses every fenced ```bash``` block in `SKILLS.md`, extracts the optional `# exit: <N>` annotation (default 0), and runs each `rmap ` invocation in a fresh fixture copy with `RMAP_TODAY=2026-05-12` pinned. Agent-contract gate for `SKILLS.md` — renaming or removing a documented command requires updating both `SKILLS.md` and the fixture in the same commit.
- `tests/golden/<case>/` — fixture triples: `tasks.toml`, `ROADMAP.input.md`, `ROADMAP.md` (expected output). Optional `today.txt` pins the render date. Add `today.txt` to any fixture using `scored_at` to avoid drift into score-decay.
- `tests/roundtrip.rs` — parse → `toml_edit` round-trip → assert no spurious diff. Catches comment-preservation regressions.

## Scope discipline

`ROADMAP.md` (rendered from `roadmap/tasks.toml`) tracks open phases; `DESIGN.md` carries the design contract and out-of-scope list; `CHANGELOG.md` is the shipped-phase record. Implemented today: validate, render (incl. `--html` static single-project view and `--html --multi` portfolio view), watch, export json, export dot (Graphviz), waves, status (single + bulk), mark, depend, new (interactive + `--from-stdin`), next, ready, show, list, blocks, deps, bundles, milestones, schema, diff, delegate, import, stale, doctor, critical-path. Score-decay rendering is automatic on tasks with `scored_at` >30d or missing. Deliberately out of scope (per `DESIGN.md`): Linear API calls, web server, git integration beyond `git show <ref>:<path>` for `rmap diff`, shell completions, CI workflow, multi-user sync. Don't add these without checking the roadmap first.
