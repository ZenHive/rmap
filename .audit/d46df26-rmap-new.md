---
sha: d46df267740ba404e170dd99c87b2245c202449f
short_sha: d46df26
audited_at: 2026-05-13
auditor_model: claude-opus-4-7
verdict: findings-applied
codex_status: dual-reviewer
audited_by: audit-review v1
---

# Audit: phase 11b — rmap new + new --from-stdin

**Original commit:** d46df26 — `phase 11b: rmap new + new --from-stdin`
**Author:** E.FU
**Files touched:** 11
**LOC:** +1087 / -20

Adds `rmap new` (interactive via `dialoguer`) and `rmap new --from-stdin` (TOML payload). Both route through `create_task(paths, from_stdin)`, which calls `add_task_str` per task and only writes after the loop validates cleanly. `next_task_id` auto-allocates numeric ids (max+1); explicit ids may be interleaved; text ids are skipped. Stdin parsing uses a dedicated `StdinPayload { task: Vec<StdinTask> }` shape with `deny_unknown_fields`.

## Findings

| # | Pri | Category | File:Line | Description | Resolution |
|---|-----|----------|-----------|-------------|------------|
| 1 | 7 | actionable | src/main.rs:684 | `StdinTask::status: Option<String>` lets `--from-stdin` create a task in any lifecycle state. Contradicts the documented invariant ("create surface must produce tasks in the `pending` lifecycle phase only"). | applied — field removed; handler hardcodes `"pending"` |
| 2 | 5 | actionable | src/mutate.rs:391 | `next_task_id` does `max + 1` with no overflow guard. On `max == u32::MAX` the arithmetic panics in debug or wraps to 0 in release, silently colliding with task `id = 0`. | applied — `checked_add(1)` + new `MutateError::IdExhausted` variant |
| 3 | 3 | discuss-design | src/main.rs:570-647 | No file lock around the read→mutate→write sequence; two concurrent `rmap new` invocations can race and one loses its `[[task]]` block | not applied (see Discuss-tier resolutions) |
| 4 | 3 | discuss-design | src/main.rs:584 | Aborts interactive `new` when stdin is non-TTY, but doesn't check stdout — interactive prompts print to stdout, so a piped-stdout invocation produces unreadable noise | not applied (see Discuss-tier resolutions) |
| 5 | 3 | discuss-design | src/main.rs:680-699 | `StdinTask` has no `cross_repo` field — depending on a cross-repo task at creation time requires a follow-up `rmap depend --cross-repo` invocation | not applied (see Discuss-tier resolutions) |
| 6 | 3 | discuss-design | src/main.rs (title interpolation) | Title text flows into Markdown rendering without escape (same pattern as the b6827be module finding) | not applied — deferred to project-wide Markdown-escape decision |

## Auto-applied fixes

- `src/main.rs::StdinTask`: removed `pub status: Option<String>` and updated the doc-comment to call out the deliberate exclusion. The `deny_unknown_fields` derive now rejects any stdin payload that includes `status =` with the standard serde error message (`unknown field 'status'`). `prompt_task_fields` no longer needs to set `status: None` in its struct literal.
- `src/main.rs::create_task`: dropped `let status = task.status.as_deref().unwrap_or("pending");` and inlined `status: "pending"` directly into the `NewTaskFields` constructor. The lifecycle invariant is now load-bearing at the type level.
- `src/mutate.rs::MutateError`: added `IdExhausted(u32)` variant — `error("cannot auto-allocate task id: u32 id space exhausted (highest existing id = {0})")`. Carries the highest existing id so the caller can confirm the projection without re-walking the array.
- `src/mutate.rs::next_task_id`: signature changed from `fn(...) -> u32` to `fn(...) -> Result<u32, MutateError>` and the body uses `max.checked_add(1).ok_or(MutateError::IdExhausted(max))`. The lone call site in `add_task_str` propagates with `?`.
- `CLAUDE.md`: updated the `main.rs` paragraph to reflect that `StdinTask::status` is now gone — the prior "Option's — schema::Task::id is required" sentence misled future audits.
- `tests/cli.rs::new_from_stdin_rejects_status_field`: new regression test piping a payload with `status = "done"`, asserting non-zero exit, that stderr mentions `unknown field` + `status`, and that `tasks.toml` is byte-equal before/after (atomicity preserved on serde reject). Locks the deny-unknown contract.

`cargo test` passes 9/9 test binaries green. No new clippy warnings.

## Discuss-tier resolutions

**Concurrency race on `tasks.toml` (finding #3).** Codex proposed: take an advisory `flock` on `tasks.toml` for the entire read→validate→render→write window.

Claude position: defer. The race is real but the project's stated workflow is single-developer-per-checkout; `rmap` doesn't run on a shared server. Adding a fcntl-flock crate dependency and the OS-portability surface (Windows uses `LockFileEx`) for a hypothetical concurrency model is premature. Calling out the constraint in SKILLS.md ("don't run two `rmap new` invocations in parallel") is the cheap intervention if it ever bites. **Verdict: deferred; revisit if a real multi-author workflow emerges.**

**Piped-stdout interactive prompts (finding #4).** Codex proposed: also reject when `std::io::stdout().is_terminal()` is false.

Claude position: keep current behavior. `dialoguer` writes to stderr by default (which is what the user actually sees in a piped context), so the prompt is readable even when stdout is redirected. Hard-rejecting would break the legitimate pattern `rmap new > /tmp/created-id.txt` once a future `rmap new` learns to print only the id to stdout. **Verdict: deferred; the current behavior is broken in a noisy-not-broken way that's worth keeping.**

**Missing `cross_repo` in `StdinTask` (finding #5).** Codex proposed: add `cross_repo: Vec<CrossRepoSpec>` to the stdin shape.

Claude position: defer. The current two-step flow (`rmap new --from-stdin` then `rmap depend --cross-repo <spec>`) is explicit and re-uses the existing depend code path. Folding cross-repo into the create surface would duplicate the `CrossRepoSpec::parse` shape into a second deserialization path (TOML rather than `<repo>:<task_id>:<relation>` string), and any failure during the create write would already include the cross_repo block — partial-write semantics get harder to reason about. **Verdict: deferred unless real usage demands it.**

**Markdown injection via `title` (finding #6).** Same as the b6827be deferral — applies project-wide; tackling `title` in isolation would split the convention. Not module-specific, not new in this commit.

## Findings not applied

- **Findings #3, #4, #5, #6.** All documented above as design decisions.

## Codex second-opinion

Status: dual-reviewer
Corroborated findings: 1 (Codex flagged the `status` field as the most likely-to-leak invariant), 2 (Codex independently noticed the missing overflow guard)
Codex-only findings (verified): #3, #4, #5 — all verified concerns; resolved as deferred
Claude-only findings: #6 — surfaced during the parallel module-field audit; resolved consistently
Codex-only findings (discarded as over-flag): Codex also flagged `prompt_task_fields` for allowing zero-character titles. Discarded — `validate_tasks_str` will reject empty `title` strings at the round-trip step, and the dialoguer prompt requires user input anyway (empty string is hard to type accidentally). The validation layer is the right place for the check, not the prompt.
Verification: Codex ran `cargo test` (passed) and confirmed `cargo clippy` exits clean on the post-fix tree (modulo macOS sandbox lock-file noise unrelated to source warnings).
