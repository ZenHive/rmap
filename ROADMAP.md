# rmap — Roadmap

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project. This file is rendered from that source — `rmap render` rewrites only the bytes between the marker pairs below. See [DESIGN.md](DESIGN.md) for the schema contract and the Phase 6 HTML render design, and [CHANGELOG.md](CHANGELOG.md) for shipped phases (1–13a).

## Current Focus

<!-- FOCUS:BEGIN -->
**Focus phase:** 13 — Skills-parity polish (5 of 9 done · 0 in progress)

**Last shipped:** Task 1 — Tasks::vision field + VISION marker pair, Task 2 — Task::branch field on in-progress rows, Task 3 — Excluded-category markers (bug/security/docs), Task 4 — D/B/U 1..=10 range validation on 2026-05-13

**Up next:** Task 5 — Task::files_to_modify field [D:3/B:5/U:5 → Eff:1.67] 🚀
<!-- FOCUS:END -->

## Gantt

<!-- MERMAID:BEGIN -->
```mermaid
gantt
    title rmap
    dateFormat YYYY-MM-DD
    section Phase 13 — Skills-parity polish
    Tasks——vision field + VISION marker pair :done, 2026-05-13, 2026-05-13
    Task——branch field on in-progress rows :done, 2026-05-13, 2026-05-13
    Excluded-category markers (bug/security/docs) :done, 2026-05-13, 2026-05-13
    D/B/U 1..=10 range validation :done, 2026-05-13, 2026-05-13
```
<!-- MERMAID:END -->

## Phase 13 — Skills-parity polish (in progress)

🎁 `schema_parity` — additive schema fields (`vision`, `branch`, `bug`/`security`/`docs` markers) plus D/B/U range validation.
🎁 `delegate_parity` — `rmap delegate` prompt shape matches `task-writing.md` (`files_to_modify`, `out_of_scope`, section restructure).
🎁 `doctor_tuning` — CLI overrides for `rmap doctor`'s hardcoded thresholds. Deferred until a real consumer asks.

Phase 13a (Eff-tier glyph + phase archive collapse) shipped 2026-05-13 — see [CHANGELOG.md](CHANGELOG.md#phase-13a-render-polish--eff-tier-glyph--phase-archive-collapse).

<!-- TASKS:BEGIN phase=13 -->
| Task | Status | Notes |
|------|--------|-------|
| Task 1 | ✅ | 🎁 **schema_parity** · Tasks::vision field + VISION marker pair [D:3/B:6/U:5 → Eff:1.83] 🚀 |
| Task 2 | ✅ | 🎁 **schema_parity** · Task::branch field on in-progress rows [D:2/B:5/U:4 → Eff:2.25] 🎯 |
| Task 3 | ✅ | 🎁 **schema_parity** · Excluded-category markers (bug/security/docs) [D:3/B:6/U:5 → Eff:1.83] 🚀 |
| Task 4 | ✅ | 🎁 **schema_parity** · D/B/U 1..=10 range validation [D:2/B:5/U:5 → Eff:2.5] 🎯 |
| Task 5 | ⬜ | 🎁 **delegate_parity** · Task::files_to_modify field [D:3/B:5/U:5 → Eff:1.67] 🚀 |
| Task 6 | ✅ | 🎁 **delegate_parity** · Task::out_of_scope field [D:2/B:5/U:5 → Eff:2.5] 🎯 |
| Task 7 | ⬜ | 🎁 **delegate_parity** · delegate.rs section restructure (task-writing.md template) [D:3/B:6/U:5 → Eff:1.83] 🚀 |
| Task 12 | ⬜ | 🎁 **doctor_tuning** · rmap doctor --threshold-days override [D:2/B:3/U:3 → Eff:1.5] 🚀 |
| Task 13 | ⬜ | 🎁 **doctor_tuning** · rmap doctor --ac-threshold override [D:2/B:3/U:3 → Eff:1.5] 🚀 |
<!-- TASKS:END -->

## Phase 6 — HTML render (planned)

🎁 `html_single` — `rmap render --html` single-project static view.
🎁 `html_portfolio` — `rmap render --html --multi` cross-repo dashboard.

Full design lives in [DESIGN.md § HTML render design (Phase 6)](DESIGN.md#html-render-design-phase-6).

<!-- TASKS:BEGIN phase=6 -->
| Task | Status | Notes |
|------|--------|-------|
| Task 8 | ⬜ | 🎁 **html_single** · rmap render --html single-project view [D:6/B:7/U:6 → Eff:1.08] 📋 |
| Task 9 | ⬜ | 🎁 **html_portfolio** · rmap render --html --multi portfolio view [D:6/B:6/U:5 → Eff:0.92] ⚠️ |
<!-- TASKS:END -->

## Phase 7 — `rmap watch` (optional)

🎁 `watch_core` — FS-watch render loop for live dev.
🎁 `watch_stream` — optional `--json` event stream on top of the watch loop.

<!-- TASKS:BEGIN phase=7 -->
| Task | Status | Notes |
|------|--------|-------|
| Task 10 | ⬜ | 🎁 **watch_core** · rmap watch FS watcher [D:4/B:5/U:4 → Eff:1.12] 📋 |
| Task 11 | ⬜ | 🎁 **watch_stream** · rmap watch --json event stream [D:3/B:4/U:4 → Eff:1.33] 📋 |
<!-- TASKS:END -->
