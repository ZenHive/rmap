---
sha: b6827bec36af1709b606f807ae5cec325be972f8
short_sha: b6827be
audited_at: 2026-05-13
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 12d.3 — optional `module` field on Task

**Original commit:** b6827be — `phase 12d.3: optional `module` field on Task`
**Author:** E.FU
**Files touched:** 11
**LOC:** +81 / -5

Adds `module: Option<String>` to `schema::Task` following the documented three-place edit discipline: `schema.rs` + `diff::diff_fields!` + `export::ExportedTask`, with `TASK_VERBOSE_WHITELIST` updated for `rmap diff --verbose`. Renders inline between bundle and title — `🎁 **bundle** · *Module* · title` when set, `🎁 **bundle** · title` (byte-identical to pre-12d.3) when absent. New golden fixture `module_field/` covers the happy path.

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 5 | actionable | src/render.rs:219-223 | `module = ""` or whitespace-only deserializes to `Some(...)` and renders as `*  * · ` (broken italic + dangling separator). No filter on empty strings. | applied — trim + filter empty before formatting |
| 2 | 3 | discuss-design | src/render.rs:222 | Module string is interpolated into Markdown without escaping `*` / `_` / backticks — a module like `Foo*Bar` breaks italics | not applied (see Discuss-tier resolutions) |

## Auto-applied fixes

- `src/render.rs`: chained `str::trim` + `filter(|m| !m.is_empty())` on the `as_deref()` result so `Some("")` and `Some("   ")` collapse to the no-module render shape. Trimming pads behaves consistently with TOML's tendency to preserve surrounding whitespace literally — `module = "  Onchain.Foo  "` now renders as `*Onchain.Foo*`, not `*  Onchain.Foo  *`. The existing `module_field/` golden fixture stays byte-equal (only one task there has a clean module string).
- `tests/golden/module_field_empty/`: regression fixture with three tasks — empty string, whitespace-only, padded. Expected output verifies the empty-render path AND the trim behavior for padded strings.

`cargo test --test render` passes 3/3.

## Discuss-tier resolutions

**Markdown injection via module strings (finding #2).** Codex proposed: HTML-escape or backslash-escape `*`, `_`, backticks before rendering.

Claude position: defer as a project-wide pattern, not a module-field-specific issue. `title`, `bundle`, `blocked_reason`, and phase/bundle `description` all flow into rendered Markdown without escaping today. Adding escapes for `module` only would create an inconsistent contract; doing it everywhere is a larger surface that deserves its own roadmap entry. Tasks.toml is author-controlled (not user-input from the web) so the threat model is "footgun" not "injection" — same severity as a typo in `title`. **Verdict: deferred; document as a phase-12d follow-up if the pattern multiplies, but no escape work for module alone.**

## Findings not applied

- **Finding #2.** Documented deferral — applies project-wide; tackling module in isolation would split the convention.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1 (Codex independently noticed the empty-string render hole and built a minimal repro)
Codex-only findings (verified): #2 (Markdown injection) — verified the concern but resolved as deferred per above
Codex-only findings (discarded as over-flag): Codex suggested promoting `module` to a typed enum / namespace path. Discarded — `Option<String>` matches the schema's overall philosophy (string-shaped TOML fields stay strings; semantic typing happens at consumer boundaries).
Verification: Codex ran `cargo test` (passed) and traced the three-place edit invariant across `schema.rs` / `diff.rs` / `export.rs`. Confirmed `TASK_VERBOSE_WHITELIST` includes `"module"`.
