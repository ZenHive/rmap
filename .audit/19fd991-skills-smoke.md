---
sha: 19fd991cc26092880ae9a6920463304509dbe85d
short_sha: 19fd991
audited_at: 2026-05-13
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 12b — SKILLS.md + skills_smoke exit-code gate

**Original commit:** 19fd991 — `phase 12b: SKILLS.md + skills_smoke exit-code gate`
**Author:** E.FU
**Files touched:** 6
**LOC:** +494 / -3

Adds `SKILLS.md` (cloud-agent how-to) and `tests/skills_smoke.rs`, which parses every fenced ```bash``` block in `SKILLS.md`, extracts the optional `# exit: <N>` annotation (default 0) and the optional `# stdin: <<EOF` payload, and runs each `rmap ` invocation in a fresh fixture copy with `RMAP_TODAY=2026-05-12` pinned. Setup pre-renders the fixture (idempotent) and `git init -b main` + commits so `rmap diff` has a base ref. This is the agent-contract gate for `SKILLS.md`.

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 6 | actionable | tests/skills_smoke.rs:125 | `# exit:` parse uses `.unwrap_or(0)` — a malformed annotation like `# exit: foo` silently degrades to exit 0, masking the test's intent | applied — strict parse with panic on malformed integer |
| 2 | 6 | actionable | tests/skills_smoke.rs:128 | `found.is_none()` short-circuit silently drops the second `rmap` line in a multi-command block — a documented two-step recipe could pass the smoke test while only exercising step one | applied — panic on duplicate detection so authors split blocks |
| 3 | 4 | actionable | tests/skills_smoke.rs:216-230 | `temp_dir()` builds a `PathBuf` under `std::env::temp_dir()` and never deletes it — every run accumulates `/tmp/rmap-skills-*` directories | applied — replaced with `tempfile::TempDir` (drop-guard cleanup) |
| 4 | 3 | discuss-design | tests/skills_smoke.rs:34 | `cmd.line.split_whitespace().skip(1)` treats the rmap command as a whitespace-tokenized argv. Quoted args with embedded spaces (e.g., `rmap mark 5 "+parallel -csr"`) tokenize wrong | not applied (see Discuss-tier resolutions) |

## Auto-applied fixes

- `tests/skills_smoke.rs::extract_command`: replaced `rest.trim().parse().unwrap_or(0)` with `.unwrap_or_else(|_| panic!(...))` carrying source-line + raw-token context, so a typo in `# exit:` fails the test loudly with a pointer to the offending SKILLS.md line.
- Same function: the `found.is_none() &&` guard is gone. If a second `rmap ` line is detected inside one bash block, the test panics with both lines + their SKILLS.md positions, telling the author to split the block. This protects the contract that "every documented rmap command runs in the smoke."
- `tests/skills_smoke.rs::skills_md_bash_blocks_match_declared_exit_codes`: replaced the manual `temp_dir()` helper (an atomic-counter + nanosecond-suffixed `PathBuf` that was never cleaned) with `tempfile::TempDir::with_prefix("rmap-skills-")`. `TempDir`'s `Drop` impl removes the directory on test exit (success and panic), so the test no longer leaves orphans in `$TMPDIR`. The helper and its imports (`AtomicU64`, `SystemTime`/`UNIX_EPOCH`, `PathBuf`) were dropped — `cargo test --test skills_smoke` still passes 1/1 against the live `SKILLS.md`.
- `Cargo.toml`: added `tempfile = "3"` to `[dev-dependencies]`. Standard crate, already an indirect dep via several existing crates.

## Discuss-tier resolutions

**Naive `split_whitespace` argv tokenizer (finding #4).** Codex proposed: use a real shell-quoting parser (e.g., `shell-words`) so multi-token args work as authored.

Claude position: keep `split_whitespace`. SKILLS.md is intentionally curated — the smoke is a contract gate over a small, hand-authored doc, not a general-purpose bash runner. Adding `shell-words` (or hand-rolling quote handling) imports a parser surface for one hypothetical recipe. If a future SKILLS.md entry needs quoted args, prefer rewriting the recipe to avoid them (e.g., `rmap mark 5 +parallel` then `rmap mark 5 -csr` as two blocks) — the smoke already supports that with the new multi-block panic guard. **Verdict: keep naive tokenizer; document the constraint in a SKILLS.md comment if/when an author hits it.**

## Findings not applied

- **Finding #4.** Documented design decision — surface mismatch is a feature, not a bug.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1, 3 (Codex flagged both — silent `.unwrap_or(0)` and missing tempdir cleanup)
Codex-only findings (verified): #4 — verified concern; resolved as deferred
Claude-only findings (verified): #2 (multi-command silent drop) — Claude noticed during the trace through `extract_command`; Codex didn't surface it
Codex-only findings (discarded as over-flag): Codex also flagged that `prepare_fixture` panics rather than returning `Result` — discarded, test code panics are the standard idiom and produce identical failure UX in `cargo test` output.
Verification: Codex ran `cargo test --test skills_smoke` against a hand-corrupted SKILLS.md (`# exit: foo`) to verify pre-fix silent-pass behavior, then confirmed the post-fix panic. Confirmed `tempfile` is a standard crate already pulled in transitively by `jsonschema`.
