# rmap import — ROADMAP.md → tasks.toml Migration

## Context
- Project: {{project}}

## Task
Convert one or more hand-edited `ROADMAP.md` files into a typed `roadmap/tasks.toml`
managed by rmap. Deliver:
  1. `roadmap/tasks.toml` — the canonical source
  2. `ROADMAP.md` updated with the marker pairs `rmap render` manages

Work through the steps below in order.

## Schema
The live `tasks.toml` JSON Schema (derived from source — never hand-edited):

```json
{{schema_json}}
```

## Field-mapping guide
Phases: `## Phase N — <name>` → `[phases.N]` with `name`, `order`, `description`, `status`
Tasks: each task row → `[[task]]` with `id`, `phase`, `bundle`, `title`, `status`, `scores`
Status glyphs (as emitted by `rmap render`): ⬜ → pending, 🔄 → in_progress, ✅ → done, 🔶 → blocked, ⛔ → superseded
Score brackets `[D:X/B:Y/U:Z → Eff:W]` → `scores = { d = X, b = Y, u = Z }` (eff is computed,
never stored — omit it from tasks.toml)
Prose body / acceptance_criteria / out_of_scope map to those same-named fields on `[[task]]`.

## Marker-pair contract
After writing tasks.toml, add these marker pairs to ROADMAP.md so `rmap render` manages them:

Per-phase task table (one pair per phase, immediately inside the phase's section):
  <!-- TASKS:BEGIN phase=N -->
  <!-- TASKS:END -->

Optional focus block (add only if `[focus]` is set in tasks.toml):
  <!-- FOCUS:BEGIN -->
  <!-- FOCUS:END -->

Optional Gantt chart (add only if you want the mermaid timeline rendered):
  <!-- MERMAID:BEGIN -->
  <!-- MERMAID:END -->

Prose outside every marker pair is byte-preserved by `rmap render` — keep existing headers
and description text in place.

## Verification loop
Run these in order; each must pass before the next:

1. `rmap validate`                 — schema + semantic checks, exit 0
2. `rmap render`                   — rewrites ROADMAP.md + data.json from tasks.toml
3. `rmap validate --check-render`  — confirms no render drift, exit 0

If `rmap diff` is available (tasks.toml is under git), run it after step 3 to review
what changed vs the base branch.

## Instructions
- Read all ROADMAP.md files first before writing tasks.toml — capture every phase, bundle,
  and task; don't cherry-pick.
- `Task.id` is REQUIRED by the schema, so every `[[task]]` you write to `tasks.toml` needs
  one. Preserve ids from the source document where they exist. For rows with no id in the
  source, either (a) assign your own sequential numeric ids before writing, or (b) skip
  those rows in the initial `tasks.toml` and add them afterward with
  `rmap new --from-stdin` (omitting `id` there triggers auto-allocation). Do NOT write
  `[[task]]` blocks with the `id` field omitted directly into `tasks.toml` — `rmap validate`
  will reject the file.
- If a field in the source doesn't map cleanly, add a note in the task's `body` and move on
  — don't block the whole migration on one ambiguous row.
- When done, report: total tasks ingested, any fields that couldn't be mapped, and the output
  of the verification loop.
