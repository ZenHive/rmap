# Changelog

Completed roadmap tasks. For upcoming work, see [ROADMAP.md](ROADMAP.md); for the design contract and deferred-phase design notes, see [DESIGN.md](DESIGN.md).

---

## Repo reorganization: `tool_roadmap.md` → `DESIGN.md` + `ROADMAP.md` (2026-05-13)

**What was done:**
- `tool_roadmap.md` split into two files: `DESIGN.md` (design contract — schema example, CLI surface, invariants, deferred designs, out-of-scope, including the Phase 6 HTML render design) and `ROADMAP.md` (live phase tracking, rendered from the new `roadmap/tasks.toml`).
- rmap now dogfoods its own roadmap: `roadmap/tasks.toml` carries 13 open tasks across phases 6 / 7 / 13. `rmap render` writes `ROADMAP.md` + `roadmap/data.json` from that source. `rmap validate --check-render` passes; `rmap doctor` clean except one informational degenerate-bundle flag (`watch` covers all phase-7 tasks).
- Cross-references swept across `AGENTS.md`, `CLAUDE.md`, `README.md`, `SKILLS.md`, `dashboard_roadmap.md`, and `migration_roadmap.md` to point at `DESIGN.md` (contract) and `ROADMAP.md` (active work list) instead of `tool_roadmap.md`.

---

## Phase 13 — Skills-parity polish

### Phase 13b: D/B/U 1..=10 range validation

**What was done:**
- New semantic check `validate::validate_scores` enforces `1..=10` on each of `task.scores.{d,b,u}`, matching the D/B/U rubric documented in `task-prioritization.md`. The check runs in both the short-circuiting (`validate_tasks_str`) and accumulating (`collect_findings`) pipelines so `rmap validate` and `rmap doctor` both surface it. Stops at the first offender per task (matches the existing per-validator shape); accumulation across distinct validators continues to work as before via `collect_findings`.
- Error message: `task <id> scores.<field> = <value> must be in 1..=10`. The `1..=10` substring is the agent-grep contract — bumping the range would be a `schema_version` change. Line locator uses the full inline-table spelling `scores = { d = D, b = B, u = U }` so the reported `path:line` lands on the row carrying the bad value, not the first matched `scores` line.
- Range constants `MIN_SCORE = 1` / `MAX_SCORE = 10` live next to the other `VALID_*` constants in `validate.rs`. `Scores` itself stays `u32`-per-field — no schema migration — because TOML rejects negatives at parse time and `10` fits comfortably in `u32`; the range check enforces the application-level rubric on top of the type-level non-negativity.
- Tests: `rejects_score_below_minimum` (d = 0), `rejects_score_above_maximum` (b = 11), `accepts_scores_at_range_edges` (both 1 and 10 are valid). Line-number asserts pin the locator behavior so a future refactor that re-locates the error reporter has to also update the fixture-line expectations — that's intentional.

### Phase 13a: render polish — Eff tier glyph + phase archive collapse

**What was done:**
- Centralized the Eff tier rubric as `scoring::tier_glyph(eff: f64) -> &'static str` — `>= 2.0 → 🎯`, `>= 1.5 → 🚀`, `>= 1.0 → 📋`, else `⚠️`. Single source of truth; the previous file-local `render::priority_symbol` is gone. Glyph rendering wired into every Eff surface: TASKS-table row (placement unchanged), FOCUS Up next line (`**Up next:** Task C — <title> [D/B/U → Eff] <tier_glyph>`), `rmap show` scores line, `rmap list` row, `rmap next` row, `rmap delegate` scores line. JSON payloads (`data.json`, `show --json`, `list --json`, `next --json`, `diff --json`) stay numeric — `format_efficiency` is untouched and the glyph is a pure render-time concatenation.
- Phase archive collapse: when `[phases.N].status = "done"`, `render_phase_table` short-circuits and replaces the body of the matching `<!-- TASKS:BEGIN phase=N -->` block with a single line `> N tasks. See [CHANGELOG.md](CHANGELOG.md#phase-N-<slug>).`. Slug = `phase_slug(name)` (lowercased, split on `!c.is_alphanumeric()`, empty segments dropped, joined with `-`), so a phase named `"Skills-parity polish"` collapses to `phase-N-skills-parity-polish`. Marker boundaries stay byte-preserved; per-phase scoped (non-done phases in the same file still render the full task table).
- Two new golden fixtures: `tests/golden/eff_tier/` (all four tier ranges in one TASKS table) and `tests/golden/phase_archive_collapse/` (done phase collapses, adjacent in-progress phase renders normally). Existing fixtures `focus_block` (Up next glyph) and `mermaid_block` (phase 11 done → collapse line) updated to match the new shape.
- New invariants in `CLAUDE.md`: tier glyph centralized in `scoring::tier_glyph` with the four-tier rubric as the agent-grep contract; archive collapse line shape `> N tasks. See [CHANGELOG.md](CHANGELOG.md#phase-N-<slug>).` is agent-grep + `validate --check-render` locked-in. The existing FOCUS invariant amended to include the trailing tier glyph on Up next.

---

## Phase 11b (correction): Codex environment notes — distinguish hex.pm gap from a general no-HTTP claim

**What was done:**
- `delegate.rs::append_agent_notes` Codex footer rewritten. Old bullet "No external HTTP: hex.pm, crates.io, npm, RFCs, vendor docs not reachable." was Elixir-shaped and overstated reach for other ecosystems. Per OpenAI's Codex Cloud docs ([Agent internet access](https://developers.openai.com/codex/cloud/internet-access)), network is environment-configurable: default offline, but the "Common dependencies" preset reaches crates.io / npmjs / PyPI and ~70 dev domains. **hex.pm is NOT in the common preset** — and the default image has no Elixir/Erlang/mix, so Elixir/Erlang is the actual special case Codex hasn't fixed. New bullets call out: (a) reach varies by config and the common-preset coverage, (b) toolchain absence for Elixir/Erlang specifically, (c) try-before-trust + fall back to in-prompt context, (d) reviewer-runs-the-harness.
- Test update in `tests/delegate.rs`: codex/cursor/claude footer assertions switched from the now-removed "No external HTTP" string to the stable "Network access varies" + "hex.pm" pair.
- Source-of-truth follow-up: `~/.claude/includes/cloud-agent-environments.md` (user-scope, not in this repo) still carries the older Elixir-shaped "No external HTTP" framing and needs the same hedge applied by hand.

---

## Phase 11b (close-out): mermaid render + `mark` canonical-position helper

**What was done:**
- Added a third optional marker pair `<!-- MERMAID:BEGIN -->` / `<!-- MERMAID:END -->` in `ROADMAP.md`. When present, `rmap render` populates the body with a fenced ```` ```mermaid ```` `gantt` block: one `section` per phase (sorted by `phases[*].order`), one row per task with `started_at`. Row shape by status: `done` → `:done, started_at, done_at`; `in_progress` → `:active, started_at, today`; `blocked` → `:crit, started_at, today`. Pending tasks (and tasks without `started_at`) are omitted — mermaid gantt requires dates. Empty-state body collapses to a minimal valid gantt with `%% no tasks with started_at yet`. Title colons / commas / semicolons (`:`, `,`, `;`) are rewritten to em-dash because mermaid parses `task :status, start, end` (`,` is the field separator; `;` is reserved for `dateformat`/etc.). Zero-config default — absence of the markers renders no mermaid block. The pass uses the same byte-preservation rule as TASKS/FOCUS and is automatically covered by `validate --check-render`.
- Closed Phase 12c.4: `rmap mark <id> +<marker>` on a task that didn't have a `markers` field used to append the field at the end of the task block (after multi-line entries like `acceptance_criteria`). `update_markers_str` now sorts the affected task's keys into canonical order via `task.sort_values_by(canonical_task_key_index)` immediately after the new-key insert, so the new field lands between `scores` and `depends_on`/`acceptance_criteria`. Idempotent ops and ops on a pre-existing `markers` field do NOT trigger the sort — author-placed ordering is preserved when the field already exists. `add_dependency_str` is intentionally not auto-sorted (helper is `mark`-only until a second consumer asks).
- Three new golden fixtures: `tests/golden/mermaid_block/` (populated gantt with mixed statuses), `mermaid_block_no_markers/` (zero-config no-op), `mermaid_block_no_dates/` (empty-state body). `tests/skills_fixture/ROADMAP.md` gained MERMAID markers so the existing `skills_smoke` suite exercises the new pass on every documented `rmap render` invocation.
- Three new mutate tests cover the canonical-position rule (new-insert reorders, idempotent add is byte-equal, existing-markers add preserves author order) plus a black-box CLI test (`mark_command_places_new_markers_field_in_canonical_position`).
- New invariants in `CLAUDE.md`: third marker pair (MERMAID byte-preservation + body-shape contract), `mark` canonical-position rule (`mark` auto-sorts on new-key insert; other mutators do not).
- `README.md` and `SKILLS.md` updated with the third marker pair.

**Phase 11b is now complete.** Remaining roadmap bets are Phase 6 (HTML render) and Phase 7 (`rmap watch`).

---

## Phase 11b (continued): `rmap new` + `rmap new --from-stdin`

**What was done:**
- Added `rmap new --from-stdin` — reads one-or-more `[[task]]` blocks as TOML from stdin and appends them to `tasks.toml`. The id is auto-allocated (numeric `max + 1`) when omitted; explicit ids that collide with an existing task are rejected as `DuplicateId`. Multi-task fragments are atomic: any validation failure (unknown phase, unknown bundle, cycle, duplicate id) aborts the entire batch with the file byte-equal to its pre-call state.
- Added interactive `rmap new` (no `--from-stdin`) — `dialoguer`-driven prompt flow (phase → bundle → title → D/B/U → markers → acceptance criteria → assignee → linear_id → module). Requires a TTY; non-interactive contexts must use `--from-stdin`. Bundles cannot be created on the fly — the user authors `[bundles.<name>]` in `tasks.toml` first so bundle metadata (`order`, `description`) stays author-visible.
- Both front-ends share `mutate::add_task_str(path, input, &NewTaskFields)` and the validate-then-write contract. Lifecycle timestamps (`started_at`, `done_at`, `blocked_reason`, `shipped_in`) are NOT settable on creation — `rmap status` owns those transitions. `created_at` and `scored_at` default to `today_iso()` when omitted.
- `tests/skills_smoke.rs` extended with a `# stdin: <<EOF` heredoc annotation so `SKILLS.md`-documented commands that read stdin (`rmap new --from-stdin`) are gated on their declared exit codes alongside the rest of the agent contract.
- `SKILLS.md` gained a "Creating tasks" section covering the happy path and the atomic-batch-failure case.

**Deferred to Phase 11b's remaining polish:** mermaid render.

---

## Phase 11b (continued): delegate per-agent footer + diff verbose

**What was done:**
- `rmap delegate` now emits a per-agent `## Environment notes` section between acceptance criteria and the standard instructions. The bullets are target-specific (Codex: no external HTTP / no project runtime; Cursor: full network, run-the-harness-green pre-PR, asdf-shim gotcha; Claude: local execution, report actual output). Content mirrors `~/.claude/includes/cloud-agent-environments.md` — that skill is the source of truth and is kept in sync by hand.
- `DelegateTarget` moved from `main.rs` into `delegate.rs` and `format_delegate_prompt` now takes the enum, not `&str`. Closes the silent-no-op risk if a new target variant is added: matching on the enum forces every branch to be considered.
- `rmap diff --verbose` adds per-field before/after on whitelisted Changed entries. Task whitelist excludes `body`, `acceptance_criteria`, and `cross_repo` (long / multi-table content would dominate the output); metadata whitelist excludes `phases.*` and `bundles.*` (already key-granular). Human output indents the before→after lines under each Changed entry; JSON output gains a `values: [{field, before, after}]` array per Changed entry. Added/Removed entries leave `values` as `None` (skipped on serialize).
- Without `--verbose`, `rmap diff` output is byte-identical to prior releases — agent contract additivity is preserved.
- New invariants in `CLAUDE.md`: whitelist sync discipline (when extending `Task`, decide on whitelist membership), diff verbose additivity (no `values` key when verbose is off), delegate footer manual-sync rule (skill include is source of truth).

**Deferred to Phase 11b's remaining polish:** mermaid render, `rmap new --from-stdin`, interactive `rmap new`.

---

## Phase 11b (partial): mark + depend mutators

**What was done:**
- Added `rmap mark <id> +marker -marker ...` — appends or drops markers atomically on a single task. Idempotent (add of an existing marker / remove of an absent marker are no-ops). Each token must start with `+` or `-`; clap's `allow_hyphen_values` lets the parser accept `-marker` without treating it as a flag, so named flags (`--tasks-path`, `--roadmap-path`, `--data-path`) precede the positional ops on the command line.
- Added `rmap depend <id> on <other-id> [--cross-repo <repo>:<task_id>[:<relation>]]` — appends in-repo and/or cross-repo dependencies. Relation defaults to `blocks` when omitted from the `--cross-repo` spec. Idempotent on the `(repo, task_id, relation)` triple for cross-repo entries and on the target id for in-repo. At least one of `on <id>` or `--cross-repo` must be present.
- Both mutators route through `toml_edit::DocumentMut` (preserving comments + formatting), re-validate the full file after edit, and refuse to write on any validation error — cycles, unknown ids, invalid markers, and invalid cross-repo relations all abort with the file byte-equal to its pre-mutation state.
- Both mutators re-render `ROADMAP.md` and `roadmap/data.json` on success, mirroring the `status` flow.
- `MutateError` gained `EmptyOps`, `InvalidMarkerOp`, `NoDependency`, and `InvalidCrossRepoSpec` variants. `MarkerOp` and `CrossRepoSpec` are public so the binary can parse positional tokens upfront and surface errors before any file I/O.

**Deferred to Phase 11b's remaining polish:** mermaid render, delegate per-agent footer parameterization, `rmap new --from-stdin`, interactive `rmap new`, `rmap diff --verbose`.

---

## Phase 11a: Health surfaces + cleanup

**Completed** | Phase 11a (Phase 11b still ⬜)

**What was done:**
- Added `rmap stale --over <duration>` — lists in-progress tasks whose `started_at` is older than the given duration (`7d`, `2w`, `30d`, `1y`). Pure selector in `src/stale.rs`; non-JSON and `--json` envelopes follow the existing `list` pattern.
- Added `rmap doctor` — composite informational report aggregating validation findings, stale in-progress tasks (>30d), score-decay candidates (`scored_at` missing or >30d ago), and render drift. Always exits 0 — `doctor` is the soft-signal aggregator; the strict gates remain `rmap validate` (exit 1) and `validate --check-render` (exit 2). `DoctorReport` serializes to a stable `{ ok, findings[] }` envelope keyed by `kind`.
- Added score-decay rendering. Tasks with `scored_at` missing or >30 days old now render with a `?` suffix on the `Eff` cell (e.g. `Eff:1.33?`). Threshold lives in `score_decay_suffix` in `src/scoring.rs`.
- Added bulk `rmap status 1,2,3 done`. Single-ID syntax is unchanged; comma-separated lists run atomically — all-or-nothing on file write, with validation re-run once at the end. New `update_status_many_str` in `src/mutate.rs`; the existing `update_status_str` is now a thin wrapper.
- Added pure-Rust date math in `src/scoring.rs` (`days_from_civil`, `days_since`) using the Howard Hinnant algorithm. No new dependencies. Reused by `stale`, `doctor`, and score-decay rendering.
- Added `today_iso()` helper in `src/lib.rs` that respects the `RMAP_TODAY` environment variable for deterministic tests, otherwise reads the system clock. Golden fixtures opt in by dropping a `today.txt` next to their `tasks.toml`.
- Removed the `--json` flag from `rmap schema`. It was a hidden no-op since Phase 8–9; `rmap schema` always emitted JSON. `rmap schema --json` now errors out with clap's unknown-argument message.
- Added `pub fn collect_findings(tasks, path, input) -> Vec<ValidateError>` to `src/validate.rs`. Each of the 12 existing semantic checks runs independently and accumulates errors instead of short-circuiting. `validate_tasks_str` is unchanged.

**Deferred to Phase 11b:** mermaid render, delegate per-agent footer parameterization, `rmap new --from-stdin`, `rmap mark`, `rmap depend`, interactive `rmap new`, `rmap diff --verbose`.

---

## Phase 10 audit fixes

**What was done:**
- `data.json` now surfaces the top-level `[focus]` table. `export::ExportedTasks` previously omitted it, so consumers of `rmap export json` and `rmap render` couldn't see which phase the project was focused on.
- `rmap diff` now detects `[focus]` add/remove/change between two `tasks.toml` revisions. `diff::diff_metadata` previously walked `linear`/`phases`/`bundles` but not `focus`. Added `diff_optional` helper for any future top-level `Option<T>` fields.
- Tightened the timestamp-validation error message: `"must match YYYY-MM-DD format (4-digit year, 2-digit month, 2-digit day)"` instead of `"must be an ISO-8601 date"`. The validator is shape-only by design (per CLAUDE.md) — the previous wording falsely implied semantic validation.
- CLAUDE.md gains a matching invariant: when adding a top-level `Tasks` field, edit `diff::diff_metadata` and `export::ExportedTasks` in the same commit. Phase 10's `focus` field was the case that surfaced the gap.

---

## Phase 10: Schema Completeness

**Completed** | Phase 10

**What was done:**
- Added top-level `[focus]` table with `Tasks.focus: Option<Focus>`. `rmap next` now prefers tasks in the focus phase before falling back to other phases; highest Eff still wins inside each tier.
- Added optional task timestamps: `created_at`, `started_at`, `done_at`, `scored_at`. Validated as ISO-8601 dates (`YYYY-MM-DD`); presence is what unlocks future decay / stale / recently-shipped surfaces.
- Added `blocked_reason: Option<String>`. Required whenever `status = "blocked"` — validation fails with a clear error otherwise.
- Added dependency cycle detection. DFS over `task.depends_on` reports the cycle members (e.g. `1 -> 2 -> 1`) and the source line of the offending `depends_on`.
- All new task fields wired through the three-place contract: `schema::Task`, the `diff_fields!` invocation in `diff::changed_fields`, and `export::ExportedTask`.

---

## Phase 8–9 Polish (review fixes)

**What was done:**
- `diff::changed_fields` now uses a local `diff_fields!` macro so the per-field comparison list lives in one declaration instead of thirteen `if`-blocks. Reduces the risk of forgetting to add a new `Task` field; CLAUDE.md carries the matching invariant note.
- `rmap schema` always emits JSON. `--json` is accepted as a hidden no-op for backward compatibility — `rmap schema` and `rmap schema --json` produce identical output.
- `rmap delegate` no longer prints a redundant `Stored assignee:` line when the stored assignee equals the `--to` target. When they differ, the line is suffixed with `(overridden)` so the receiving agent sees the routing intent at a glance.
- `delegate.rs` `writeln!(prompt, …).expect("write to string")` boilerplate replaced with a module-level `line!` macro; behavior unchanged, signal-to-noise improved.

---

## Phase 9: Cloud Delegation Surface

### Delegate Command
**Completed** | Phase 9

**What was done:**
- Added optional `assignee` and `acceptance_criteria` task fields.
- Added validation for `assignee` values.
- Added `rmap delegate <id> --to claude|codex|cursor` as a pure read-only command that emits structured Markdown for cloud agents.
- Included the new fields in agent JSON exports as additive fields.
