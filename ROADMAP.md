# rmap — Roadmap

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project. This file is rendered from that source — `rmap render` rewrites only the bytes between the marker pairs below. See [DESIGN.md](DESIGN.md) for the schema contract and the Phase 6 HTML render design, and [CHANGELOG.md](CHANGELOG.md) for shipped phases (1–13a).

## Current Focus

<!-- FOCUS:BEGIN -->
**Focus phase:** not set — add [focus] to tasks.toml
<!-- FOCUS:END -->

## Gantt

<!-- MERMAID:BEGIN -->
```mermaid
gantt
    title rmap
    dateFormat YYYY-MM-DD
    section Phase 6 — HTML render
    rmap render --html single-project view :done, 2026-05-14, 2026-05-14
    section Phase 7 — rmap watch (optional)
    rmap watch FS watcher :done, 2026-05-14, 2026-05-14
    rmap watch --json event stream :done, 2026-05-14, 2026-05-14
    section Phase 13 — Skills-parity polish
    Tasks——vision field + VISION marker pair :done, 2026-05-13, 2026-05-13
    Task——branch field on in-progress rows :done, 2026-05-13, 2026-05-13
    Excluded-category markers (bug/security/docs) :done, 2026-05-13, 2026-05-13
    D/B/U 1..=10 range validation :done, 2026-05-13, 2026-05-13
    Task——files_to_modify field :done, 2026-05-13, 2026-05-13
    Task——out_of_scope field :done, 2026-05-13, 2026-05-13
    delegate.rs section restructure (task-writing.md template) :done, 2026-05-13, 2026-05-13
    rmap doctor --threshold-days override :done, 2026-05-14, 2026-05-14
    rmap doctor --ac-threshold override :done, 2026-05-14, 2026-05-14
    rmap list/next --bundle <name> filter :done, 2026-05-13, 2026-05-13
    rmap next --count N returns top-N candidates :done, 2026-05-13, 2026-05-13
    rmap bundles selector listing :done, 2026-05-13, 2026-05-13
    rmap next-bundle— select one session-sized bundle (Option 1— bundle = session— no subsetting) :done, 2026-05-14, 2026-05-14
    rmap status <id> done — auto-fill done_at (and started_at on in_progress) :done, 2026-05-17, 2026-05-17
    `rmap depend <src> on <numeric-id>` fails with 'unknown task' when target id is numeric-only :done, 2026-05-17, 2026-05-17
    section Phase 14 — Migration tooling
    rmap import — emit a paste-ready ROADMAP.md→tasks.toml migration prompt :done, 2026-05-15, 2026-05-15
    section Phase 15 — Schema extensions
    `Task——implemented` field — record what was actually delivered (required when done) :done, 2026-05-17, 2026-05-17
```
<!-- MERMAID:END -->

## Phase 13 — Skills-parity polish (done)

🎁 `schema_parity` — additive schema fields (`vision`, `branch`, `bug`/`security`/`docs` markers) plus D/B/U range validation.
🎁 `delegate_parity` — `rmap delegate` prompt shape matches `task-writing.md` (`files_to_modify`, `out_of_scope`, section restructure).
🎁 `doctor_tuning` — CLI overrides for `rmap doctor`'s hardcoded thresholds (`--threshold-days`, `--ac-threshold`).

Phase 13a (Eff-tier glyph + phase archive collapse) shipped 2026-05-13 — see [CHANGELOG.md](CHANGELOG.md#phase-13a-render-polish--eff-tier-glyph--phase-archive-collapse).

<!-- TASKS:BEGIN phase=13 -->
> 16 tasks. See [CHANGELOG.md](CHANGELOG.md#phase-13-skills-parity-polish).
<!-- TASKS:END -->

## Phase 6 — HTML render (planned)

🎁 `html_single` — `rmap render --html` single-project static view.
🎁 `html_portfolio` — `rmap render --html --multi` cross-repo dashboard.

Full design lives in [DESIGN.md § HTML render design (Phase 6)](DESIGN.md#html-render-design-phase-6).

<!-- TASKS:BEGIN phase=6 -->
| Task | Status | Notes |
|------|--------|-------|
| Task 8 | ✅ | 🎁 **html_single** · rmap render --html single-project view [D:6/B:7/U:6 → Eff:1.08] 📋 |
| Task 9 | ⬜ | 🎁 **html_portfolio** · rmap render --html --multi portfolio view [D:6/B:6/U:5 → Eff:0.92] ⚠️ |
<!-- TASKS:END -->

## Phase 7 — `rmap watch` (done)

🎁 `watch_core` — FS-watch render loop for live dev.
🎁 `watch_stream` — optional `--json` event stream on top of the watch loop.

<!-- TASKS:BEGIN phase=7 -->
> 2 tasks. See [CHANGELOG.md](CHANGELOG.md#phase-7-rmap-watch-optional).
<!-- TASKS:END -->

## Phase 14 — Migration tooling (done)

🎁 `import_command` — `rmap import` paste-ready ROADMAP.md→tasks.toml migration prompt.

<!-- TASKS:BEGIN phase=14 -->
> 1 task. See [CHANGELOG.md](CHANGELOG.md#phase-14-migration-tooling).
<!-- TASKS:END -->

## Phase 15 — Schema extensions (done)

🎁 `agent_routing` — per-task LLM model pinning (`Task::model`, surfaced by `rmap delegate`).

<!-- TASKS:BEGIN phase=15 -->
> 3 tasks. See [CHANGELOG.md](CHANGELOG.md#phase-15-schema-extensions).
<!-- TASKS:END -->
