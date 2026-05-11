# Changelog

Completed roadmap tasks. For upcoming work, see [tool_roadmap.md](tool_roadmap.md).

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
