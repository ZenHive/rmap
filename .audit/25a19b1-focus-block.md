---
sha: 25a19b1011f7732d38c74cc7ae4a37877ddd8459
short_sha: 25a19b1
audited_at: 2026-05-13
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 12d.1 — auto-rendered FOCUS block

**Original commit:** 25a19b1 — `phase 12d.1: auto-rendered FOCUS block`
**Author:** E.FU
**Files touched:** 15
**LOC:** +388 / -10

Adds a second optional marker pair `<!-- FOCUS:BEGIN --> / <!-- FOCUS:END -->` to `render.rs`. Renders three lines from `[focus].phase` + tasks: header (`**Focus phase:** N — <name> (M of K done · J in progress)`), last shipped (within `SHIPPED_WINDOW_DAYS = 7`), and up next (via `next_task`). Zero-config default: absent markers leave the document unchanged. Three golden fixtures shipped (`focus_block`, `focus_block_empty`, `focus_block_no_markers`). Drift is caught automatically by `validate --check-render`.

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 5 | actionable | tests/render.rs | `MissingFocusEndMarker` error path has no test — `render_rejects_unclosed_marker` covers TASKS only | applied — added `render_rejects_unclosed_focus_marker` |
| 2 | 5 | actionable | tests/golden/ | `[focus]` absent + FOCUS markers present (the "not set" empty-state line) has no golden fixture | applied — added `focus_block_no_focus_table/` |
| 3 | 4 | actionable | tests/golden/ | Two-FOCUS-pair byte-preservation invariant (only first replaced) has no fixture — documented in CLAUDE.md but untested | applied — added `focus_block_two_pairs/` |

## Auto-applied fixes

- `tests/render.rs`: added `render_rejects_unclosed_focus_marker` mirroring the TASKS variant. Locks the error message contract (`"missing <!-- FOCUS:END -->"`) so a refactor that swaps the marker name fails loudly.
- `tests/golden/focus_block_no_focus_table/`: tasks.toml omits `[focus]` and ROADMAP.input.md includes the marker pair. Expected output is the single `**Focus phase:** not set — add [focus] to tasks.toml` line — the path the CLAUDE.md invariant calls out as "keeps the marker honest." `today.txt` pinned to 2026-05-12 (avoids score-decay drift).
- `tests/golden/focus_block_two_pairs/`: two FOCUS pairs separated by prose. Expected output has the first pair rendered (focus-set body) and the second pair byte-equal to its input. Verifies the documented one-pair-replaced convention.

All three additions are picked up by the existing `golden_render_fixtures_are_byte_equal` runner without code change to the test harness. `cargo test --test render` passes 3/3.

## Discuss-tier resolutions

No `discuss-design` findings.

## Findings not applied

None.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1, 2 (Codex raised the "missing-error-path" coverage gap and noted the "not set" line was code-only). Claude independently added #3 (two-pairs convention) — uncorroborated by Codex but documented as an invariant in CLAUDE.md, so worth a regression fixture.
Codex-only findings (verified): —
Codex-only findings (discarded as over-flag): Codex also flagged the SHIPPED_WINDOW_DAYS = 7 day-window as "magic constant" and suggested making it configurable. Discarded — the CLAUDE.md invariant explicitly fixes the window as part of the agent contract (greppable string `no recent shipments` depends on a stable cutoff). A flag would bake configuration noise into the contract before any consumer has asked for it.
Verification: Codex ran `cargo test` (passed) and inspected the three new golden directories.
