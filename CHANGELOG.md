# Changelog

Completed roadmap tasks. For upcoming work, see [ROADMAP.md](ROADMAP.md); for the design contract and deferred-phase design notes, see [DESIGN.md](DESIGN.md).

---

## Phase 14 — Migration tooling

### Phase 14 Task 18: `rmap import` — paste-ready ROADMAP.md→tasks.toml migration prompt (import_command bundle)

**What was done:**
- New `rmap import` command: emits a self-contained Markdown prompt an agent pastes into a session to migrate a hand-edited `ROADMAP.md` into `roadmap/tasks.toml`. Pure read — no file mutation; mirrors the `rmap delegate` pattern.
- Prompt template lives in `templates/import_prompt.md` (same `include_str!` pattern as `templates/roadmap.html.j2`). Two runtime substitution tokens (`{{project}}`, `{{schema_json}}`) keep the project name and JSON Schema current without code changes.
- Live JSON Schema embedded via `schema_json_str()` — the schema section of the prompt auto-updates whenever `schema.rs` changes; no hand-maintenance needed.
- Prompt covers the full migration recipe: field-mapping guide (status glyphs, score brackets, prose fields), marker-pair contract (`TASKS:BEGIN`, `FOCUS:BEGIN`, `MERMAID:BEGIN`), and the `validate → render → validate --check-render` verification loop.
- `~/.claude/includes/rmap.md` command table updated with `rmap import` row; SKILLS.md gained an `import` entry in the Delegation section.

---

## Phase 15 — Schema extensions

### Phase 15 Task 19: `Task::model` field — per-task LLM model pinning (agent_routing bundle)

**What was done:**
- New optional free-text `model` field on `schema::Task` — records which LLM model should do the task (e.g. `claude-opus-4-7`). Unvalidated: model IDs churn, so a closed enum would rot; mirrors `module` / `branch`. Distinct from `assignee` (who owns it) and `rmap delegate --to` (which agent *environment*).
- `rmap delegate` surfaces it as a conditional `- Model:` bullet in the prompt's `## Context`, so the target agent knows which model to run; omitted when unset.
- `rmap show` prints a `model:` line; the field surfaces in `data.json` / `show --json` / `list --json` / `next --json` and in `rmap diff` (on `TASK_VERBOSE_WHITELIST`).
- Settable at creation: `NewTaskFields` + `StdinTask` + `add_task_str` + the interactive `rmap new` prompt all carry `model`; `canonical_task_key_index` places it immediately after `module`.
- Not rendered into `ROADMAP.md` rows — mirrors `assignee`. No `render.rs` change, no golden fixtures, no `validate --check-render` drift.
- No `schema_version` bump — additive optional field.
- Consumer-facing `~/.claude/includes/rmap.md` gained a "Pinning an LLM model per task" subsection; CLAUDE.md gained a "keep rmap.md in sync" discipline note.
- Tests: delegate `- Model:` bullet present/absent; `rmap new --from-stdin` round-trip (tasks.toml canonical order + data.json + `rmap show` human/JSON); `export` surfaces `model` and skips it when unset.

---

## Phase 6 — HTML render

### Phase 6 Task 8: `rmap render --html` single-project view (html_single bundle)

**What was done:**
- New `--html` flag on `rmap render`: an additive output that writes a self-contained static HTML view to `roadmap/dist/index.html` (gitignored). Renders `ROADMAP.md` + `data.json` as usual, then writes the HTML; `roadmap/dist/` is created on first run. With `--dry` it adds a `would write …index.html` line; with `--stdout` the more-specific flag wins — the HTML is printed and nothing is written.
- New `src/render_html.rs`: `render_html_str(tasks, today)` builds `PhaseView` / `TaskView` / `DagView` in pure Rust, then renders the template. Phases sort by `phase.order`; `blocked` tasks share the pending column (distinguished by `data-status="blocked"`).
- New `templates/roadmap.html.j2`: the one minijinja template, `include_str!`'d so it ships in the binary (minijinja was already a declared-but-unused dependency). Inline CSS (screen + `@media print` with `✓ → ⏸ ⛔` status symbols and `N/M done` numerals), a sticky filter bar (marker chips + status toggles), per-phase progress bars + 3-column status layout, a collapsed `<details>` SVG layered-DAG dependency graph, the `<script id="rmap-data">` data island (compact `data.json` via `| safe` — the only `| safe` in the template; everything else relies on HTML autoescape), and ~40 lines of vanilla JS for client-side filtering.
- DAG layout: longest-path layering over in-repo `depends_on` only (`cross_repo` is portfolio-mode, Task 9), memoized; nodes slotted by id within a layer, coords from fixed `NODE_W`/`NODE_H`/`H_GAP`/`V_GAP`/`PAD` constants, one downward edge per resolved dependency.
- Every task card carries six `data-*` attributes (`data-id` / `data-status` / `data-eff` / `data-markers` / `data-depends-on` / `data-phase`); DAG nodes carry `data-id`. These plus the `rmap-data` island id are the agent contract — new CLAUDE.md invariants lock them.
- `ResolvedPaths` gained `html_path` (`project_root/roadmap/dist/index.html`), always derived from `project_root` — no `--html-path` override flag.
- New `export::export_compact_json_str` — the single-line `data.json` variant for the island.
- `.gitignore` gained `/roadmap/dist`; `SKILLS.md` gained a `rmap render --html` block (auto-covered by `skills_smoke.rs`) and a layout-diagram line; CLAUDE.md gained the module entry, pipeline note, and two load-bearing invariants.
- Out of scope (deliberate): `--multi`/portfolio view (Task 9), HTML coverage in `validate --check-render` (it gates `ROADMAP.md` drift only), HTML re-render in `rmap watch` (canonical artifacts only). The <50KB size budget is a target, not an asserted test — rmap's own roadmap renders to ~37KB.
- No `schema_version` bump — additive flag.
- Tests: 8 inline `#[cfg(test)]` unit tests in `src/render_html.rs` (DAG layering: no-deps, chain-of-3, diamond longest-path; single-node coords; one-edge-per-dep; empty-roadmap render; data-island + attrs presence; `truncate_label`). 5 new `tests/cli.rs` black-box tests (writes a self-contained index; no index without the flag; `--dry` writes nothing; `--stdout` prints HTML without writing; six `data-*` attrs present + idempotent re-run).

### Phase 6 — HTML render polish: phase-status badge

**What was done:**
- `PhaseView` now carries the `[phases.N].status` field — previously dropped, so a `done` phase rendered identically to an active one (just with empty Pending/In Progress columns).
- The phase header gains an uppercase status badge (`.phase-status`, colored by status), and each `<section class="phase">` gains a `data-phase-status` attribute. `done` phases are dimmed (opacity 0.6); hover and `@media print` restore full opacity.
- New inline test asserts the badge + `data-phase-status` attribute render.
- No `schema_version` bump — additive: a new badge element, a new `data-*` attribute on the section, and CSS only.

---

## Phase 7 — `rmap watch` (optional)

### Phase 7 Task 10: `rmap watch` FS watcher (watch_core bundle)

**What was done:**
- New `rmap watch` subcommand: a foreground, blocking FS-watch loop that re-renders `ROADMAP.md` + `roadmap/data.json` whenever `roadmap/tasks.toml` changes. Flags: `--tasks-path` / `--roadmap-path` / `--data-path` (same path-override set as `rmap render`). Stop with Ctrl-C (default SIGINT — no `ctrlc` crate added).
- New `notify = "8"` dependency, used via the synchronous `recommended_watcher` + `std::sync::mpsc` channel path — no async runtime. `notify` was already named in `DESIGN.md`'s planned crate-dependency list.
- New `src/watch.rs` module carries two pure, unit-tested helpers: `write_if_changed(path, content) -> io::Result<bool>` (writes only when rendered bytes differ from disk; returns whether it wrote — the idempotency primitive) and `is_tasks_toml_event(tasks_path, &notify::Event) -> bool` (filters directory-level FS events to `tasks.toml` by exact-path OR `file_name()` match). The `notify` wiring and the blocking `for res in rx` loop live in `main.rs::watch_command`.
- The watcher is registered on the `roadmap/` **directory** (`RecursiveMode::NonRecursive`), not on `tasks.toml` directly — watching the file breaks on editor atomic-save renames (the held inode gets replaced). The filename filter is the infinite-loop guard: `roadmap/data.json` is rewritten by every render and lives in the same watched directory, so without it our own `data.json` write would re-trigger the loop endlessly.
- Idempotent (AC #3): every render goes through `write_if_changed`, so a save that produces no rendered change (a TOML comment edit, `touch`) writes nothing and prints nothing.
- Resilient: a mid-edit invalid `tasks.toml` prints its error to stderr and the loop continues — it never aborts on a bad save. The startup `watching …` line and all errors go to stderr; stdout carries exactly one `rendered` line per real re-render (the stdout/stderr split leaves room for a future `watch --json`, Task 11). `watch_command` also does one initial idempotent render at startup so outputs are current before the first event.
- New CLAUDE.md "Load-bearing invariants" bullets lock the directory-watch + filename-filter guard and the idempotency / stdout-stderr split. SKILLS.md gained a "Live dev" section — deliberately in a `text` fence, not `bash`, because `rmap watch` never exits and would hang `skills_smoke.rs`. Exit-code table gained a `watch` row.
- No `schema_version` bump — additive subcommand.
- Tests: 7 inline `#[cfg(test)]` unit tests in `src/watch.rs` covering `write_if_changed` (writes-on-missing, writes-on-differ, skips-on-equal) and `is_tasks_toml_event` (exact match, file-name match after rename, ignores `data.json`, ignores `ROADMAP.md`). Per the chosen test strategy no integration test was added; the `notify` wiring was verified by manual smoke — atomic-save re-render, comment-edit no-op, `data.json`-edit loop-guard, and broken-TOML resilience + recovery.

### Phase 7 Task 11: `rmap watch --json` event stream (watch_stream bundle)

**What was done:**
- New `rmap watch --json` flag: the same FS-watch loop, but each render event is emitted as one **compact JSON line** to stdout instead of the `rendered` text line. Two event shapes — `{"schema_version":1,"event":"rendered","outputs":[...]}` on a real write and `{"schema_version":1,"event":"error","message":"..."}` on a failed render (AC #1). No-op renders stay silent in both modes.
- `outputs` lists the basenames of *exactly* the files written this render — a single-entry array when only `ROADMAP.md` or only `data.json` changed. Basenames, not absolute paths, keep host filesystem layout out of the stable agent contract.
- An `error` is treated as a render event and goes to **stdout** as a JSON `error` event — the `--json` stream is self-contained. Only the startup `watching …` line and `notify` watcher-infrastructure errors stay text on stderr; `--json` changes stdout only.
- New `src/watch.rs` pure helpers `render_event_line(outputs)` / `error_event_line(message)` build a private `WatchEventJson` and serialize via `serde_json::to_string` (compact) — the one place in rmap that uses `to_string` over `to_string_pretty`, because the stream contract is one object per line. `WATCH_SCHEMA_VERSION = 1` is the bump point for the event shape.
- `main.rs`: `rerender_if_changed` now returns `RenderOutcome { roadmap_changed, data_changed }` instead of a bare `bool` so `report_render` can list the changed outputs; `report_render(result, &paths, json)` branches on the flag. `--json` lines go through `emit_json_line`, which flushes stdout so a piped consumer (`jq`) isn't batched.
- Event shape documented in `SKILLS.md` (AC #2) — extended the existing `text`-fenced "Live dev" block; kept out of `bash` fences since `rmap watch` never exits. CLAUDE.md's `rmap watch` invariant rewritten from the old "future `watch --json`" forward-reference to the shipped contract (event spellings, field names, `schema_version` bump rule).
- No `schema_version` bump for `tasks.toml` — additive flag; the `watch --json` event stream is a new contract surface versioned by its own `WATCH_SCHEMA_VERSION`.
- Tests: 3 new inline `#[cfg(test)]` unit tests in `src/watch.rs` (`render_event_line` with both outputs, with one output, `error_event_line`). No integration test — `watch_command` blocks indefinitely; consistent with Task 10's strategy. Manual smoke verified: one JSON line per real change, no-op silence, `error` event on broken TOML with the loop continuing, startup line on stderr.

---

## Repo reorganization: `tool_roadmap.md` → `DESIGN.md` + `ROADMAP.md` (2026-05-13)

**What was done:**
- `tool_roadmap.md` split into two files: `DESIGN.md` (design contract — schema example, CLI surface, invariants, deferred designs, out-of-scope, including the Phase 6 HTML render design) and `ROADMAP.md` (live phase tracking, rendered from the new `roadmap/tasks.toml`).
- rmap now dogfoods its own roadmap: `roadmap/tasks.toml` carries 13 open tasks across phases 6 / 7 / 13. `rmap render` writes `ROADMAP.md` + `roadmap/data.json` from that source. `rmap validate --check-render` passes; `rmap doctor` clean except one informational degenerate-bundle flag (`watch` covers all phase-7 tasks).
- Cross-references swept across `AGENTS.md`, `CLAUDE.md`, `README.md`, `SKILLS.md`, `dashboard_roadmap.md`, and `migration_roadmap.md` to point at `DESIGN.md` (contract) and `ROADMAP.md` (active work list) instead of `tool_roadmap.md`.

---

## Phase 13 — Skills-parity polish

### Phase 13 Tasks 12 & 13: `rmap doctor` threshold CLI overrides (doctor_tuning bundle)

**What was done:**
- New `rmap doctor --threshold-days <N>` flag (`Option<u32>`): overrides the hardcoded 30-day cutoff for **both** the stale-in-progress check and the score-decay check. The flag name carries its unit, so it takes a bare integer (not a `7d`/`2w` duration string like `rmap stale --over`).
- New `rmap doctor --ac-threshold <N>` flag (`Option<u32>`): a single value that overrides **both** `AC_DIFFICULTY_THRESHOLD` (5) and `AC_BENEFIT_THRESHOLD` (8) for the missing-`acceptance_criteria` lint — when set, the predicate becomes `d >= N || b >= N`. Defaults stay distinct (5 / 8) when the flag is absent.
- New `DoctorThresholds { days, ac_difficulty, ac_benefit }` struct in `src/doctor.rs` with `resolve(threshold_days, ac_threshold)` applying the constant defaults. `DoctorReport::run` takes it as a parameter; the three threshold-sensitive call sites (`find_stale`, score-decay comparison, missing-AC predicate) read from it instead of module constants.
- `DoctorReport` gained a `thresholds` field — the effective thresholds are now echoed in the `rmap doctor --json` envelope, so an agent running with an override can interpret the findings. Additive JSON field; the `Display` impl reads the runtime values for its section headers.
- `STALE_THRESHOLD_DAYS` / `AC_*` constants stay as the defaults inside `resolve`; the now-unused `SCORE_DECAY_DAYS` import was dropped from `doctor.rs` (it still lives in `scoring.rs` for the render path).
- SKILLS.md Health section gained a `rmap doctor --threshold-days 60 --ac-threshold 6` fenced example; `skills_smoke.rs` locks the exit code.
- No `schema_version` bump — two additive optional flags + one additive JSON field; no `kind` string, render shape, or existing field touched.
- Tests: three new in `tests/cli.rs` (`doctor_command_threshold_days_suppresses_stale_and_decay`, `doctor_command_ac_threshold_changes_missing_ac_set`, `doctor_command_thresholds_in_json`).

### Phase 13 Task 17: `rmap next-bundle` selects one session-sized bundle (bundle_selector bundle)

**What was done:**
- New `rmap next-bundle` subcommand: picks one bundle worth of dep-satisfied pending tasks for a single Claude Code session. Flags: `--phase <N>` (override `[focus].phase`), `--bundle <name>` (force-pick, bypasses ranking), `--json`, `--tasks-path`. No D-budget subsetting (Option 1) — output is *every* actionable pending task in the chosen bundle.
- Ranking rule: `(in_focus_phase desc, sum_eff desc, bundle.order asc)`. Bundles with zero actionable tasks are skipped. Actionability is **broad**: a pending task T is actionable iff every dep is either `done` (anywhere) OR an in-bundle pending task that is itself broadly actionable (recursive, memoized DFS with belt-and-suspenders cycle defense — `validate` already rejects cycles).
- Force-pick (`--bundle X`) bypasses ranking entirely, including focus-phase preference and `--phase N` (when both passed, `--bundle` wins silently). Force-pick on a declared-but-zero-actionable bundle returns `Some` with empty tasks; force-pick on an undeclared bundle is a hard error (exit 1, stderr `bundle '<X>' is not declared in tasks.toml`).
- Tasks emitted in dep-topological order via Kahn's algorithm with stable Eff-desc tie-break.
- New `src/next_bundle.rs` carries the pure selector `pick(tasks, filter, today) -> Option<BundlePick>`. Reuses `query::matches_bundle` and `scoring::efficiency` — no duplicated dep-satisfaction logic. `today` parameter mirrors `next::next_tasks` for future score-decay-sensitive ranking; unused today.
- `src/export.rs` gained `export_bundle_pick_json_str(tasks, effective_focus, pick)` — JSON envelope `{ schema_version, focus_phase, bundle: {name, phase, description} | null, tasks: [ExportedTask, ...] }`. `focus_phase` is the **effective** focus (post-`--phase` override). Empty pick emits `bundle: null, tasks: []` in the same envelope (mirrors `rmap bundles --json` rather than `rmap next --json`).
- Human output: header `bundle <name>  phase <N> — <phase_name>  [<done>/<total>]  — <description>`, then one row per task using `format_task_row` (mirrors `rmap list`).
- Three empty-state spellings (stderr, exit 0): `none — no actionable bundle in any phase`, `none — no actionable bundle in phase N`, `none — bundle '<X>' has no actionable pending tasks`. Missing-bundle (`--bundle X` where X is not declared) is a hard error (exit 1).
- New CLAUDE.md "Load-bearing invariants" bullet locks the ranking rule, empty-state spellings, JSON envelope shape, no-subsetting rule, and force-pick precedence.
- SKILLS.md gained three fenced examples (`rmap next-bundle`, `rmap next-bundle --json`, `rmap next-bundle --bundle alpha`); `skills_smoke.rs` locks them.
- No `schema_version` bump — additive subcommand.
- Tests: 10 new in `tests/cli.rs` covering focus-phase-wins, tie-by-order, all-blocked-skipped, unmet-external-dep-skips, force-pick-bypass, JSON envelope shape, topo order, `--phase` override, empty-pick stderr+exit-0, force-pick-zero-actionable stderr, and missing-bundle exit-1. 7 in-module unit tests in `src/next_bundle.rs` covering the same matrix at selector depth.

### Phase 13 Task 16: `rmap bundles` selector listing (batch_selection bundle)

**What was done:**
- New `rmap bundles` subcommand: lists authoring-time `[bundles.*]` with per-bundle status counts and the next dep-satisfied pending task. Flags: `--phase <N>`, `--has-next`, `--in-focus`, `--json`, `--tasks-path`.
- New `src/bundles.rs`: `list_bundles(tasks, filter) -> Vec<BundleSummary>` + `BundleFilter { phase, has_next, in_focus }`. Per-bundle `next_task` reuses `next::next_task` with a bundle-restricted `TaskFilter` — single source of truth for dep-satisfaction.
- Human row shape `<name>    <done>/<total> <glyph_or_next>  — <description>` under per-phase headers `phase <N> — <phase_name>  [<phase_status>{, focus}]`. Five-branch `status_glyph_or_next` ladder: all-done → `✅`, in-progress only → `🚧`, all-blocked → `all-blocked ⛔`, pending-with-no-next → `pending:<n> (deps unmet) ⏸`, otherwise → `next:<id> [Eff:<x.y>] <tier_glyph>`.
- JSON envelope `{ schema_version, focus_phase, bundles }`; `focus_phase` serializes as explicit `null` when unset (deliberately not skipped). `status_counts` always zero-fills all four keys. `NextTaskSummary.eff` matches `ExportedTask.eff` rounding.
- New CLAUDE.md "Load-bearing invariants" bullet locks the row shape, status-glyph ladder, separator spacing, JSON envelope, and `next_task` reuse rule.
- SKILLS.md gained `rmap bundles`, `rmap bundles --in-focus --has-next --json` fenced examples; `skills_smoke.rs` locks them.
- No `schema_version` bump — additive read surface.

### Phase 13 Task 15: `rmap next --count N` returns top-N candidates (batch_selection bundle)

**What was done:**
- New `--count N` clap arg on `rmap next` (`NonZeroUsize`, default `1`). `--count 0` is rejected at clap parse time — the `NonZeroUsize` guard is part of the agent contract documented in `CLAUDE.md`.
- `src/next.rs` gained `next_tasks(tasks, filter, count) -> Vec<&Task>`: focus-phase candidates fill first (Eff desc, stable on ties to preserve TOML declaration order), and the selector falls through to non-focus candidates only when the focus pool has fewer than `count` entries. `next_task` is now a thin wrapper over `next_tasks(..., 1).into_iter().next()`; the old fold-based `highest_efficiency` was replaced with a stable `sort_by_eff_desc` so multi-count and single-count share one ranking rule. Behavior on count == 1 is preserved bit-for-bit (stable descending sort returns the same first-encountered task on Eff ties as the previous `fold` with `>=`).
- `src/export.rs` gained `export_tasks_array_json_str(&[&Task])` — emits a bare JSON array of `ExportedTask`, NOT the `ExportedTasks` envelope used by `rmap render` / `list --json` (no `schema_version` / `project` / `phases` / `bundles` wrapper). Documented invariant: array-branch shape is part of the agent contract.
- `src/main.rs` routes `count == 1` through the old `export_task_json_str` path (bare object or `null` — byte-identical to pre-Task-15 releases for the default invocation) and `count > 1` through the new array helper. Human output prints one line per task in both cases (zero lines when nothing eligible).
- New CLAUDE.md "Load-bearing invariants" block locks the JSON shape split: `--count 1` bare object/null, `--count >1` array (possibly empty). Renaming the flag, changing the default, dropping the `NonZeroUsize` guard, or flipping `--count 1` to always emit an array is a `schema_version` bump.
- SKILLS.md gained a `rmap next --count 3 --bundle alpha --json` fenced example; `skills_smoke.rs` locks the exit code.
- No `schema_version` bump — additive flag, default-invocation contract preserved.
- Tests: five new in `tests/cli.rs` (`next_command_count_one_default_emits_bare_object_json`, `next_command_count_three_json_emits_eff_ranked_array`, `next_command_count_exceeds_eligible_returns_min_without_error`, `next_command_count_three_human_prints_one_line_per_task`, `next_command_count_zero_is_rejected_at_parse_time`) and four new in `tests/next.rs` (singleton-matches-`next_task`, count==3 Eff-ranked, exceeds-eligible-returns-min, focus-phase-fills-before-other-phases).

### Phase 13 Task 14: `rmap list/next --bundle <name>` filter (batch_selection bundle)

**What was done:**
- New `--bundle <name>` clap arg on `rmap list` and `rmap next`. `TaskFilter` (in `src/query.rs`) gained a fourth `bundle: Option<String>` field; `list_tasks` chains a fourth `matches_bundle` predicate alongside the existing status/marker/phase filters.
- `next_task` switched from `(tasks, marker: Option<&str>)` to `(tasks, &TaskFilter)`, sharing the filter struct with `list_tasks`. Honors `filter.marker` and `filter.bundle` only — `filter.status` is ignored (next is implicitly `pending`-only) and `filter.phase` is ignored (focus is sourced from `[focus].phase`, not the user). Bundle filter applies BEFORE the focus-phase partition so `--bundle X` restricts the candidate pool first and focus only ranks within the bundle (AC #3: bundle wins over focus when both explicitly set). `matches_marker` and `matches_bundle` are now `pub(crate)` in `query.rs` so `next.rs` reuses the same predicates instead of duplicating them.
- Unknown bundle name returns empty (`list --bundle nonexistent --json` → empty `task` array; `next --bundle nonexistent --json` → `null`) without erroring — same lenient stance as `--marker`. We do NOT validate the user-supplied `--bundle` against known bundle keys.
- `src/render.rs::up_next_line` updated to pass `&TaskFilter::default()` (was `None`); behavior unchanged.
- No `schema_version` bump — additive flag, `bundle` was already in `ExportedTask`, `Task.bundle` was already required.
- Tests: five new in `tests/cli.rs` (`list_command_filters_by_bundle`, `next_command_filters_by_bundle`, `next_command_composes_bundle_and_marker`, `next_command_unknown_bundle_returns_null_json`, `list_command_unknown_bundle_returns_empty_envelope`). Existing `tests/next.rs` call sites threaded through a small `marker_filter` helper. `tests/query.rs` literal `TaskFilter` construction extended with `bundle: None`. SKILLS.md gained two fenced examples (`rmap list --bundle alpha`, `rmap next --bundle alpha`) — `skills_smoke.rs` locks them in.

### Phase 13c: `Task::files_to_modify` field (delegate_parity bundle)

**What was done:**
- New optional `files_to_modify: Vec<String>` field on `schema::Task`, mirroring the `out_of_scope` / `acceptance_criteria` shape (`#[serde(default)]`, no `skip_serializing_if` on the schema struct, no `schemars` per-field attribute). Existing `tasks.toml` files parse unchanged — the field is invisible when absent.
- Three-place edit per the documented invariant: `src/schema.rs` (field), `src/diff.rs::diff_fields!` (so `rmap diff` notices add/remove/change), `src/export.rs::ExportedTask` (so `data.json` / `show --json` / `list --json` / `next --json` surface the field additively with `skip_serializing_if = "<[_]>::is_empty"`). Deliberately NOT added to `TASK_VERBOSE_WHITELIST` — paths are short strings individually, but a non-trivial `files_to_modify` list would bloat verbose diff payloads alongside `out_of_scope` / `acceptance_criteria` / `body`; the changed-field name itself is enough signal for the agent contract.
- `rmap delegate` is the only human-readable surface: new `append_files_to_modify` function renders a `## Files to modify` section between `## Acceptance criteria` and `## Out of scope`. Reads naturally as "what (AC) → where (files) → not where (out_of_scope)". Bullet shape is plain `- {path}`, NOT the `- [ ] {item}` checkbox form used by acceptance criteria — file paths are scope hints, not todos. Empty `Vec` → section omitted entirely. `ROADMAP.md` TASKS table, FOCUS block, MERMAID gantt, `rmap show` stdout, and `rmap list` stdout are unchanged (and the `validate --check-render` gate stays clean by construction).
- No `schema_version` bump — additive field, no breakage for existing consumers.
- Tests: `tests/delegate.rs` extended with a non-empty `files_to_modify` fixture (positive assertions on section header, plain-bullet rendering, no-checkbox guard) and the minimal fixture (negative assertion that the section is absent). `tests/export.rs` asserts the JSON array shape when set and the `skip_serializing_if` behavior when empty. `tests/diff.rs::verbose_emits_before_after_for_whitelisted_changed_fields` extended to confirm `files_to_modify` surfaces in `changed_fields` but is omitted from the `values` whitelist payload. Existing golden fixtures, `roundtrip`, and `skills_smoke` pass unchanged. The `Task` struct literal in `src/stale.rs::tests::make_task` updated to include the new field.

### Phase 13c: `Task::out_of_scope` field (delegate_parity bundle)

**What was done:**
- New optional `out_of_scope: Vec<String>` field on `schema::Task`, mirroring the `acceptance_criteria` shape (`#[serde(default)]`, no `skip_serializing_if` on the schema struct, no `schemars` per-field attribute). Existing `tasks.toml` files parse unchanged — the field is invisible when absent.
- Three-place edit per the documented invariant: `src/schema.rs` (field), `src/diff.rs::diff_fields!` (so `rmap diff` notices add/remove/change), `src/export.rs::ExportedTask` (so `data.json` / `show --json` / `list --json` / `next --json` surface the field additively with `skip_serializing_if = "<[_]>::is_empty"`). Deliberately NOT added to `TASK_VERBOSE_WHITELIST` — `out_of_scope` mirrors `acceptance_criteria` and `body` (free-form string arrays would bloat verbose diff payloads without aiding decisions).
- `rmap delegate` is the only human-readable surface: new `append_out_of_scope` function renders a `## Out of scope` section between `## Acceptance criteria` and `## Environment notes`. Bullet shape is plain `- {item}`, NOT the `- [ ] {item}` checkbox form used by acceptance criteria — out-of-scope items are guardrails, not todos. Empty `Vec` → section omitted entirely. `ROADMAP.md` TASKS table, FOCUS block, MERMAID gantt, `rmap show` stdout, and `rmap list` stdout are unchanged (and the `validate --check-render` gate stays clean by construction).
- Dogfooded on Task 6 itself: the task ships with `out_of_scope` listing what was deliberately deferred (`NewTaskFields` extension, ROADMAP rendering, `schema_version` bump).
- No `schema_version` bump — additive field, no breakage for existing consumers.
- Tests: `tests/delegate.rs` extended with a non-empty fixture (positive assertions on section header, plain-bullet rendering, no-checkbox guard) and the minimal fixture (negative assertion that the section is absent). `tests/export.rs` asserts the JSON array shape when set and the `skip_serializing_if` behavior when empty. `tests/diff.rs::verbose_emits_before_after_for_whitelisted_changed_fields` extended to confirm `out_of_scope` surfaces in `changed_fields` but is omitted from the `values` whitelist payload. Existing golden fixtures, `roundtrip`, and `skills_smoke` pass unchanged. The `Task` struct literal in `src/stale.rs::tests::make_task` updated to include the new field.

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
