# Changelog

Completed roadmap tasks. For upcoming work, see [tool_roadmap.md](tool_roadmap.md).

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
