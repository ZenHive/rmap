---
sha: 64cbf1743ad666434a92c1ec33a9f5a26717709c
short_sha: 64cbf17
audited_at: 2026-06-03
auditor_model: claude-opus-4-8
verdict: findings-applied
codex_status: single-reviewer (out of batched-codex scope)
audited_by: audit-review v1
---

# Audit: harness: agent delivery — task 31 rmap doctor: advisories for phase / focus state drift (run run-1780364198810-3551a56f)

**Original commit:** 64cbf17 — `harness: agent delivery — task 31 rmap doctor: advisories for phase / focus state drift (run run-1780364198810-3551a56f)`
**Author:** harness
**Files touched:** 3
**LOC:** ±633

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 4   | doc-gap  | CLAUDE.md:87 | New soft doctor advisories (Phase/Focus drift) added with the same load-bearing pattern as the milestone-drift advisories, but absent from CLAUDE.md's "Load-bearing invariants" index | applied |

## Auto-applied fixes

- CLAUDE.md: added a "phase / focus drift advisories" bullet to the load-bearing-invariants index, adjacent to the milestone-drift bullet — documents the three advisories (`PhaseFullyDoneButOpen` / `PhaseHasInProgressButPending` / `FocusPhaseClosed`), their trigger conditions, soft/no-mutation semantics (phase/focus state is user-curated), and the JSON kinds `phase_fully_done_but_open` / `phase_has_in_progress_but_pending` / `focus_phase_closed` (verified against the `#[serde(tag = "kind", rename_all = "snake_case")]` derive on `DoctorFinding`).

## Codex second-opinion

Status: not dispatched — doc-drift finding (Category 6), not correctness-sensitive; outside this batch's Codex scope. Claude single-reviewer pass; finding verified directly against `src/doctor.rs` and `CLAUDE.md`.
