---
sha: 593d0ff8ae1b6cd76e8ce69d5b41a7704b5ba949
short_sha: 593d0ff
audited_at: 2026-05-13
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 12d (partial) — doctor lints (degenerate_bundle + missing_acceptance_criteria)

**Original commit:** 593d0ff — `phase 12d (partial): doctor lints — degenerate_bundle + missing_acceptance_criteria`
**Author:** E.FU
**Files touched:** 5
**LOC:** +314 / -12

Adds two new `DoctorFinding` variants and matching Display + JSON output: `DegenerateBundle` (bundle whose task set equals every task with the same phase) and `MissingAcceptanceCriteria` (pending/in_progress tasks with `d >= 5 OR b >= 8` and empty `acceptance_criteria`). New constants `AC_DIFFICULTY_THRESHOLD = 5` and `AC_BENEFIT_THRESHOLD = 8` are hardcoded; `Scores` gains a `Clone` derive so the variant payload owns its scores. Existing `DOCTOR_CLEAN_TASKS` / `DOCTOR_DIRTY_TASKS` fixtures were extended with a second bundle each so neither covers its whole phase (avoids the new lint firing in unrelated tests).

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 3 | discuss-design | src/doctor.rs:14-16 | `AC_DIFFICULTY_THRESHOLD = 5` / `AC_BENEFIT_THRESHOLD = 8` are hardcoded global policy — may over/under-warn across repos with different scoring calibration | not applied (intentionally deferred — see Discuss-tier resolutions) |
| 2 | 3 | discuss-design | src/doctor.rs:103-126 | Bundle of 1 task in a 1-task phase warns as degenerate, even though a single-task bundle still adds a label | applied — gate on `bundle_ids.len() >= 2` |

## Auto-applied fixes

- `src/doctor.rs`: `degenerate_bundle` detection now skips bundles with fewer than 2 tasks (was: skips only empty bundles). A 1-task bundle in a 1-task phase still attaches a meaningful label; the lint targets "redundant grouping of a multi-task phase," not "any bundle that happens to cover its phase." Existing test fixture `DOCTOR_LINT_TASKS` uses 3 tasks in the `everything` bundle so the lint still fires as documented; `doctor_command_surfaces_degenerate_bundle` and `doctor_command_lint_findings_in_json` remain green.

## Discuss-tier resolutions

**Hardcoded AC thresholds (finding #1).** Codex proposed: keep if intentionally dogfood-derived; otherwise consider a CLI/config escape hatch before more consumers depend on the lint.

Claude position: keep. The commit's own CLAUDE.md update explicitly says "thresholds hardcoded; revisit when a real consumer asks for `--ac-threshold`." The lint is informational (doctor always exits 0), so over-warning is a soft cost. The `--ac-threshold` neighbor is the natural place to add overrides once a second consumer's calibration is known — premature flag surface would lock in a contract before there's signal. **Verdict: converged-by-deferral, no code change.**

## Findings not applied

- **Finding #1.** Documented deferral, not a hidden issue — the constants are intentionally hardcoded with a documented escape route.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1, 2 (Codex flagged both, Claude independently agreed)
Codex-only findings (verified): —
Codex-only findings (discarded as over-flag): —
Verification: Codex ran `cargo test` (passed) and `cargo clippy` (failed on macOS sandbox lock-file permission only — not a code warning).
