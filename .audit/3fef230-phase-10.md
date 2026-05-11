---
sha: 3fef23069f4fa8130f18ab372952926d8ef14d6e
short_sha: 3fef230
audited_at: 2026-05-11
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 10

**Original commit:** 3fef230 — `phase 10`
**Author:** E.FU
**Files touched:** 11
**LOC:** +499 / -15

Phase 10 ships schema completeness: top-level `[focus]` table, four optional task timestamps (`created_at`/`started_at`/`done_at`/`scored_at`), `blocked_reason` conditionally required when `status = "blocked"`, DFS dependency-cycle detection, and `[focus].phase` integrity. New fields wired through `schema::Task` → `diff_fields!` → `export::ExportedTask`. Documentation (CHANGELOG, CLAUDE.md, tool_roadmap.md) updated.

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 7 | bug | src/export.rs:7 | `ExportedTasks` omits top-level `[focus]` — `data.json` silently drops it | applied |
| 2 | 7 | bug | src/diff.rs:90 | `diff_metadata` doesn't compare `Tasks.focus` — focus add/remove/change is silently missed by `rmap diff` | applied |
| 3 | 4 | doc-gap | CLAUDE.md:56 | Three-place invariant only covers `schema::Task` fields, not top-level `Tasks` fields (the case Phase 10 itself exposed) | applied |
| 4 | — | discuss-design | src/validate.rs:284-289 | Timestamp error message says `"must be an ISO-8601 date"` but validator is digit-shape only (`9999-99-99` passes) | applied (dialogue-resolved: tighten message) |
| 5 | 3 | minor | src/validate.rs:211 | `validate_dependency_cycles` reports the first global `depends_on` line, not the offender's line | not applied — see below |
| 6 | 3 | minor | src/validate.rs:315 | `validate_blocked_reasons` reports the first global `status = "blocked"` line, not the failing task's | not applied — see below |
| 7 | 2 | minor | src/validate.rs:334 | `validate_focus_phase` line search can match a task's `phase = N` line before the `[focus]` table | not applied — error message already names `[focus].phase N` |

## Auto-applied fixes

- `src/export.rs`: added `focus: Option<&'a Focus>` to `ExportedTasks` with `skip_serializing_if = "Option::is_none"`; wired through `exported_tasks_with`. New test `exports_top_level_focus_when_set` + `omits_focus_key_when_absent` in `tests/export.rs`.
- `src/diff.rs`: added `diff_optional<T: PartialEq>` helper; `diff_metadata` now calls it for `focus` so add/remove/change surfaces in `rmap diff` and `rmap diff --json`. New test `focus_add_remove_and_change_show_up_in_metadata_diff` in `tests/diff.rs`.
- `src/validate.rs`: error wording changed from `"must be an ISO-8601 date (YYYY-MM-DD)"` to `"must match YYYY-MM-DD format (4-digit year, 2-digit month, 2-digit day)"`. Existing test assertion updated in `tests/validate.rs`.
- `CLAUDE.md`: new "Top-level `Tasks` fields have the same three-place contract" invariant; existing timestamp invariant reworded from "Timestamp fields are ISO-8601 dates" to "Timestamp fields validate by shape, not semantics" with explicit note that `9999-99-99` passes the validator on purpose.
- `CHANGELOG.md`: new "Phase 10 audit fixes" entry above the Phase 10 entry.

## Discuss-tier resolutions

**ISO-8601 wording (finding #4).** Claude proposed: tighten the error message, keep validator shape-only. Reasoning: CLAUDE.md explicitly documents shape-only as intentional (no `chrono` dependency, round-trip TOML preservation). The bug is the message, not the validator.

Codex independently reached the same resolution and same wording structure (`"must match YYYY-MM-DD format (...)"`) and explicitly verified `cargo test` (75 passed) + `cargo clippy` (clean). Verdict: **converged, applied.**

## Findings not applied

- **#5 / #6 / #7** (line-resolution imprecision). These are real but **pre-existing project-wide patterns** — `validate_dependencies` at line 179, `validate_phase_and_bundle_references` at lines 372/383, and `validate_statuses` at line 96 all use the same `line_containing(input, <coarse needle>)` approach. Fixing them in isolation for the Phase 10 additions would create inconsistency. The right fix is a project-wide validator-line-resolution refactor (introduce a `task_id_needle(&TaskId) -> String` helper and thread per-task line context through every validator). That's a separate Phase 11 / health-and-polish item, not Phase 10 scope. The error messages already name the failing task/cycle, so the line number is at worst a navigation hint, not a correctness gap.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1, 2, 3, 4, 5, 6, 7 (all six Codex findings overlapped with Claude's read; no Claude-only finding survived independent verification)
Codex-only findings (verified): —
Codex-only findings (discarded as over-flag): —
Dialogue (finding #4): convergence, applied.
