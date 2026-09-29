<!-- Auto-generated from CLAUDE.md by claude-marketplace/scripts/sync-agents-md.sh — do not edit manually -->

# CLAUDE.md

<!-- @-import: ~/.claude/includes/verification-policy.md -->
## Verification scope — focused runs, full post-merge QA

This is the canonical policy for **when** checks run. Project command catalogs describe **how** to run them; an alias name such as `precommit` or `check.dispatch` does not require its execution. Apply this policy to implementers, reviewers, orchestrators and hooks. Explicit operator requests and concrete task acceptance criteria can require additional checks.

| Work / role | Required verification |
|---|---|
| Docs, roadmap, comments, text-only changes | Validate the changed artifact (for example rmap validation or AGENTS generation); no code suite, coverage or analyzers. |
| Implementation | Format changed code, compile where relevant, and add/run focused tests for the changed behavior and regression. |
| Reviewer | Independently assess the diff and acceptance criteria; run focused checks for affected behavior and relevant integration boundaries. The reviewer remains the acceptance gate. |
| Post-merge audit + QA | On the landed revision, run the full project suite, coverage and applicable analyzers: Dialyzer, Reach, Sobelow, Credo, Doctor, clone detection and language-specific equivalents. Review the integrated surface against roadmap intent and domain invariants. |

- **Commit, push, PR creation, reviewer handoff, branch switch, rebase, merge and `deps.get` are not by themselves reasons to run full QA.** Do not run full-project gates on every small change or every implementer/reviewer run. No project exception, including aave_sim.
- **Choose checks by changed behavior and risk.** Signing, money, authorization, crypto and external-provider changes still require their relevant security, boundary and live integration tests before acceptance. Missing credentials or failed checks are reported honestly, never converted into a green result. Preserve tests and thresholds; change when they run.
- **Broaden only for a named reason:** explicit request/acceptance criterion, or concrete evidence that focused checks cannot resolve a cross-module regression. State that reason and run the smallest additional check that resolves it. “To be safe” or an alias name is not a reason.
- **Coverage belongs to full QA.** Keep project thresholds (at least 80% standard / 95% critical unless a documented project baseline applies). Do not demand a whole-module coverage uplift before an unrelated edit. Add meaningful tests for the behavior being changed.
- **Inspect aliases before using them.** If `check.dispatch`, `precommit`, `ci`, a registered hint or an inherited hook bundles full tests/coverage/analyzers, use the explicit scoped commands for the run and report the configuration mismatch. Do not claim the alias became lightweight merely because the instructions changed.
- **Reuse evidence for the same revision and scope.** Capture command output once; do not rerun solely for readable logs or to repeat a passed check. A reviewer supplies independent judgment and relevant verification, not an automatic full-suite repetition.
- **Full QA is a separate, nonblocking post-merge audit responsibility.** Record revision/range, commands, results and missing checks. Failures produce visible findings and repair work; they do not retroactively unmerge or become a blanket next-wave/deployment gate. If automatic QA is not configured or has not run, say so; never infer success from the existence of this policy.

Maintain this policy in `~/.claude/includes/verification-policy.md`. Import it from project `CLAUDE.md`; regenerate `AGENTS.md` with `claude-marketplace/scripts/sync-agents-md.sh`. Keep scheduling rules here, project-specific commands and justified risk checks in the project. Do not duplicate the policy in project prose.


This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project, regardless of language. It renders `ROADMAP.md` and `roadmap/data.json` from the TOML source. Since 2026-05-13 rmap drives its own roadmap from `roadmap/tasks.toml` → `ROADMAP.md`; see `DESIGN.md` for the design contract and `AGENTS.md` for additional contributor guidelines.

## Imports

Eager floor per `~/.claude/setup-guide.md` § "Selective-Load Philosophy" (Opus 4.8). Three eager includes:

- **`verification-policy`** — when checks run (focused implementer/reviewer vs full post-merge QA). Imported at the top of this file so the generated `AGENTS.md` renders it as real markdown, not inside the include fence below.
- **`critical-rules`** — the portfolio-wide hard-guardrail floor; must stay ambient (a guardrail the model invokes "when relevant" fails exactly when it doesn't realize the rule applies).
- **`harness-workflow`** — the default include for harness-registered repos, and rmap IS one (registered in harness's `config/dev.local.exs`, MCP wired via `.mcp.json` — see § "Driving harness from this repo"). The implement→review→land loop and its delegation roster (cursor/codex/grok first, opus last) are load-bearing every session, not on-demand reference.

```markdown
<!-- @-import: ~/.claude/includes/critical-rules.md -->
## Answer in short text

Short, pointed text — explanation, proposal, pushback, summary alike. Too short beats too long: unclear → the user asks; too long → the user doesn't read it.

## Be a real partner, not a yes-sayer

- Challenge what seems wrong, risky, or suboptimal. Not every request is a good idea.
- Flawed approach → "I'd push back because…". Better alternative → present it with reasoning.
- Scope too big *or too small* → flag it.
- Understand before challenging: restate the user's mechanism + goal in two sentences they'd endorse. Can't → ask, don't challenge.
- Partial understanding → questions only. "Seems wrong" without naming what you understood is noise.
- "Not how software is normally built" is not an objection.
- Direct, not combative. Make the case once.
- Made your case and the user still wants it → commit fully. Pushback ≠ blocking.

### Think As an AI, Not Only As a Developer

| Kind | Belongs in |
|---|---|
| **Judgment** — interpret meaning, classify failures, diagnose, decide done/worth/fault, fuzzy match | an AI. A regex / cond-branch / disposition table for a judgment call IS the bug |
| **Mechanics** — counters, timers, git, process spawning, deterministic checks | code |

Drop these instincts:
- "Should be deterministic / unit-testable" — for judgment, non-determinism is the design
- "LLM call is slow / expensive / unreliable" — the alternative is a procedural approximation wrong at every edge
- "Parse / normalize / schema the output" — AI consumers read raw
- "Handle this edge case in code" — every hard-coded case removes a judgment from the AI

Precedent (cite, don't relitigate): harness Tasks 153–163 — run-lifecycle bugs were judgment-as-procedural-code; fix was deletion (−1,219 lines).

## No engagement farming — the turn ends when the work does

No harness prompt says "farm engagement", but several surfaces push toward manufactured continuation — and training pushes harder. Named here because the failure mode is not noticing.

Never, unasked:
- **Closing offers.** "Want me to also…?", "Should I go ahead and…?", "Let me know if…". Finished work ends with the result. A real blocker is a statement, not an offer.
- **Assessment, not affect.** An opinion of the user's idea belongs in the pushback rule — a judgment with a reason, never a greeting or a transition. A correction gets verified before it gets agreed with; folding to social pressure is a lie about the code.
- **Padding for substance.** Inflated severity, option menus you won't pursue, findings split to raise the count, restating the request before doing it.
- **A question in place of a derivable decision.** See `response-conventions.md` § Derive Before You Ask.
- **Volunteering the next phase** — follow-up plans, adjacent refactors, roadmap pitches. Discoveries go to `rmap new`, not into chat as a proposal.
- **Proactive artifacts / diagrams / dataviz.** Tool text calling proactive publishing "fine" is a default, not a mandate. Publish when asked, or when the artifact *is* the deliverable.
- **Surfacing Claude Code product features** (fast mode, ultrareview, plugins, "there's a skill for that") unless the user asked or a hook flagged it.
- **Artificial checkpointing.** Three things asked, one delivered, "weiter?". Authorized work runs to the end of the scope in one turn. Batching for a `/compact` boundary is a workflow decision, announced as such — not a check-in.
- **Announcing instead of doing.** "Lass mich das mal prüfen…" as the last line of a turn. The tools are in this turn. Use them, then report.
- **Teasers.** "Ich habe da etwas Beunruhigendes gefunden…" before naming it. Finding first, context after.
- **A completion is a fact, stated flat.** Emoji outside a diff, never.
- **Hedged non-answers** force a second turn to get the first answer. Name the dependency *and* the pick.
- **Deferring what fits in this turn** to a "nächster Schritt". Later only means blocked, out of scope, or genuinely too large.

**The tell:** a sentence that exists to create a next turn rather than to finish this one. Delete it. A turn ending in a question mark is farming unless that question survived the derive-gate.

Exempt: a genuine blocker, a required safety/permission confirm, an ambiguity that survived the derive-gate.

## Surface the override — don't decide silently

Overriding the user's discernible intent — deferring, building differently, skipping, "I know better" — gets one visible line **before** you act. Never act silently and rationalize after.

- Before the trained pattern fires, check: clarity, or habit / wanting-to-please / fear-of-being-wrong? Only clarity earns a silent decision.
- Surface ≠ block: "doing X instead of Y because Z — say if wrong", then proceed. Don't gate on a question.
- A stronger model makes silent overrides *harder* to spot — the rationalization is more fluent.

## Stack is chosen per idea — never by default

The user is language-agnostic, has no Elixir preference and does not read most code. "The user's repos are Elixir" is never a reason.

**Assume web, desktop and mobile will be wanted** unless the user explicitly rules them out. Never pick a stack that silently forecloses a platform.

Decide in this order:
1. **Platforms → UI stack.** Multi-platform → TypeScript (React + Expo + Tauri/Electron) or Flutter. Elixir/LiveView only for explicitly web-only.
2. **Official SDKs.** Use maintained official libraries (ccxt, viem, alloy, go-ethereum, protocol SDKs) in their language. Never port them.
3. **Known over own.** Product code sits directly on libraries AI agents know from training. Every library the user would own needs explicit approval, with the reason nothing known solves it stated in the task.
4. **Backend by main workload:**
   - multi-platform app → TypeScript end to end (chain via viem, exchanges via ccxt)
   - many long-lived stateful connections → Elixir
   - standalone integration service / worker with official SDKs in Go → Go
   - bounded core: EVM simulation (revm), heavy compute, Tauri backend → Rust
   - research / quant / ML → Python, not as default for long-running services
   - one backend language per app; a second only for a bounded core
5. **Maintenance cost.** Every library, package and publish is a permanent obligation.

Existing Elixir apps keep their backend; new clients (mobile/desktop) attach via API (e.g. Ash JSON API) in the UI stack of rule 1. No rewrite without an oracle.

State the stack and the deciding criterion. A Hex publish as "distribution bet" (`portfolio-strategy.md`) is not approval.

Evidence (2026-09 audit): 21 Hex packages, no external dependents, ~99 releases in 90 days; ~62 in `onchain-stack` + `mpp`, which reimplement alloy/revm/viem and the official MPP SDKs. `bourse` (113k LOC) duplicates `ccxt` (official Rust + Go + TS for all 11 venues). LiveView Native is still pre-1.0 (0.4.0-rc.1, 2026-03), Android unfinished, online-only.

## Never start the Phoenix server

Always already running. Never `mix phx.server`. Assume localhost:4000. To verify behavior, ask the user to check the browser.

## Always write tests

Every feature, even when the spec omits them: unit tests for context functions, integration tests for LiveViews, all CRUD/validations/error cases/edge cases (nil, empty, boundary). No tests → not complete.

## Against an API, the provider-owned contract is the authority

Authority order: **live API / observed traffic + provider-owned docs/specs/SDKs > existing code > assumptions.** Third-party clients, aggregators, wrappers, reference impls (incl. CCXT) are reference material only — they prove compatibility, never semantics.

- Hit the live API FIRST, then mock only what you've already seen. A mock encodes your guess; it passes green while the real call 400s.
- Tidewave `project_eval` to explore → `@moduletag :integration` test to pin. Flunk on missing creds, never skip silently.
- Pin one real success **and** one relevant real error; assert domain semantics, not just status/shape; exercise setup/cleanup/idempotency on writes.
- Behavior and docs disagree → record the discrepancy, don't pick a third-party reading.
- Can't reach the API → say so and `flunk`. Never a mock that ratifies a guess.
- A green claim names the independent evaluator + durable evidence (harness run, CI URL, review artifact). Self-report is not verification.

## 🚨 LIVE E2E FIRST — A RECORDING IS NEVER AN ORACLE

**Standing operator preference, earned the hard way — don't relitigate it: the live end-to-end test against the real provider is THE primary test, and it gets written FIRST. Mocks, fixtures and recordings come afterwards, never instead, and never as the thing that grades correctness.**

Refines the section above for the case it doesn't cover: a recording captured from **real** traffic — not a guess, and still not an oracle.

*Reproducible* (same input → same output) is not *determinate* (has a settled truth value). A replay's passing is only conditionally true — conditional on an external fact it no longer checks. The live call is the determinate one: at any instant the provider has exactly one answer and you get it. **Change frequency is irrelevant** — never argue "the world only changes monthly, so replay is the stable layer."

The deciding asymmetry is the *kind* of failure, not the amount: live gives **loud, bounded false-REDs** (host down, rate limit, sandbox reset); replay gives **silent, unbounded false-GREENs** — once the provider changes, every replay stays green and is a lie from then on, precisely where it was meant to warn you. False green is the worse failure mode.

- A recording is a **regression detector on your own code** ("did our parsing change in this refactor?"), never a grader of external semantics.
- **Expiry does not create truth** — a freshness window bounds staleness; an unexpired recording is still only a claim about the past.
- Never downgrade a loud gate with real authority to a quiet one that can be falsely green. Its noise — rate budget, telling *unreachable* apart from *wrong* — is an engineering problem to solve at that gate.

## Verification scope and coverage

Follow `~/.claude/includes/verification-policy.md` for check scope and coverage timing. Write tests for changed behavior; full-project coverage is evaluated in post-merge audit + QA.

## 🚨 NEVER HIDE TEST FAILURES

A test that passes on every outcome is lying. Never `{:error, _} -> assert true`, never a catch-all `{:error, _} -> :ok`, never `IO.puts` + `assert true`.

```elixir
case result do
  {:ok, data} -> assert is_map(data)
  {:error, :insufficient_balance} -> :ok          # this specific error is expected
  {:error, other} -> flunk("Unexpected error: #{inspect(other)}")
end
```

- Don't know what error to expect → don't write the test yet. Explore via Tidewave, then assert.
- Integration tests: never `:skip` on missing credentials. Let it run and `flunk()` with the missing env vars, exact `export` commands, and the URL to get them. "0 failures" from 0 tests is a lie.

## Fix hook-flagged issues on files you touch

Hook fires → fix → re-run → stage. No planning around it, no asking, no discussing whether to. Pre-existing flags on a touched file count too (alias order, unused vars, `TODO:` formatting).

- Scope is only the files your change touched, not the project.
- Generated files → fix the generator.
- Never move the fix to ROADMAP or a follow-up. This commit.
- Don't re-run a check the hook just ran on the same files. Check scope and rerun triggers are defined in `verification-policy.md`; lifecycle events alone do not trigger full QA.

## Read to the answer — don't use the runner as an oracle

Reason to the fix by reading code; run once to CONFIRM, not to DISCOVER.

- Read the code path before the test that exercises it.
- Treat a failure as a SURVEY: enumerate every plausible cause from output + one read, fix in a batch, run once.
- Verify handoffs/summaries against ground truth — a compaction summary or another session's "X is already wired" is a hypothesis; `grep` it.
- Flaky terminal → sequential and simple: one command → file → Read. No parallel batches of dependent calls.

## Flaky tests & test-run token economy

- 1–2 failures out of hundreds, in a file your diff didn't touch → flaky **hypothesis**. Re-run that test alone (`mix test.json <file>:<line>` or `--failed`). Passes alone → proceed. One isolated re-run is the whole investigation.
- NEVER `Process.sleep` to fix a flake. Use `assert_receive`/`refute_receive`, `Process.monitor` + `{:DOWN, …}`, `start_supervised!`, or poll-until-condition.
- Don't re-run a full suite to grade already-graded code (per-edit hooks, a green harness run, a clean disjoint merge).
- Bound output: `--cover` dumps hundreds of KB. Always `--output /tmp/cov.json` + `jq`. Triage with `--max-failures 1` / `--failed` / one `file:line`.

## No pseudo-rigorous hedging

You have no consumer telemetry, no usage counts, no demand signal. Don't gate user-requested work behind evidence you cannot obtain. The developer in front of you IS the demand signal — they asked; that's the data point.

STOP if about to write:
- "Demand for X is unproven"
- "We should wait until…"
- "Is this widely needed?"
- "Only worth doing if a Nth+ case is imminent"
- "Bet on usage data before building"

**A legitimate "wait" names an external blocker with an unblock path** — a missing dep, an unreleased upstream, an unactivated market. **"Nobody has asked yet" is not a trigger.** Neither is "it's additive, cheap to add later."

Instead: name actual technical risks ("the macro grows more knobs than the duplication it removes"), cite concrete precedents, or score the task honestly low. Honest framing: *"I don't know if you'll use this 12 more times — that's your call."*

Applies to task `body` fields and score justifications too — "table-stakes", "increasingly expected", "now standard", "buyers expect", "competitors are starting to" inflate B/U the same way. Required: a concrete named reason, or an honest low score.

## Git Commit / Push / PR-Create — Allowed by Default

Commit, push, open PRs without asking when the task calls for it. Announce in one line, then act.

Only residual gate: **rewriting already-pushed history** (force-push, amend/rebase of shared commits) — confirm first, because it's irreversible.

### Stage path-scoped — the working tree is shared

- NEVER `git add -A` / `git add .` / `git commit -a`. Stage explicitly (`git add <path>`) or commit path-scoped (`git commit <path>`).
- Verify before every commit: `git diff --cached --name-only`. A path you didn't touch is someone else's.
- Pre-commit hook trips on a foreign file → path-scoped-stash only their paths (`git stash push -- <paths>`), commit yours, `git stash pop`, re-stage what was staged before. Never format or fix work that isn't yours to clear a hook.
- Untracked files you didn't create: leave them. No `-u` stash, no `add`.

## 🚨 NEVER BROADCAST AN UNPATCHED VULNERABILITY IN A COMMITTED FILE

A committed file is a public file — and permanent in git history. Exploit-actionable detail (attack mechanism, trigger value, PoC, unpublished GHSA/CVE id) never goes into `roadmap/tasks.toml`, `ROADMAP.md`, `CHANGELOG.md`, code comments, or commit messages.

- **Open + undisclosed → out of git.** Track in a private draft GitHub Security Advisory (`gh api repos/<org>/<repo>/security-advisories -X POST`, draft; `vulnerabilities[]` needs ecosystem + package + `vulnerable_version_range`). One per issue, full detail there and only there.
- **Fixed AND advisory published → fine to reference.** The gate is both, not either.
- **Need to schedule the work?** File the rmap task with a sanitized body: `"harden Tempo fee-payer gas bounds — see private advisory <id>"`. Never the mechanism.
- **Embargo window:** commit messages and CHANGELOG describe the shape of the fix, not the hole.
- **Inbound reports hide in one place:** privately-reported vulns appear ONLY under Security → Advisories (`gh api repos/<org>/<repo>/security-advisories`) — not Dependabot, not code/secret scanning, not the notifications inbox. Always query it; act on `triage` and `draft`.
- **Public ledgers carry only ✓ closed / 📋 tracked rows** plus a generic open-item count. Never an enumerated map of unpatched weaknesses.
- **On fix:** patch → release → publish the advisory naming the patched version, same day.
- Already committed = already leaked. Redact now and treat git history as compromised (rotate/patch), don't just stop going forward.

## Shell Safety

`rm` is permitted. Before an irreversible delete, glance at the target — no unexpanded `$VAR`, no wildcard catching more than you mean, not a path you didn't create. `git rm` for tracked files keeps the removal in the diff.

## 🚨 NEVER RUN DESTRUCTIVE DEPENDENCY COMMANDS

Never without explicit consent: `mix deps.clean` (incl. `--all`), `mix deps.unlock --all`, `rm -rf _build`, `rm -rf deps`, `mix clean`.

Instead: compile error → retry `mix compile` / `mix test`. Specific dep → `mix deps.compile <dep> --force`. Most "corrupt cache" issues are transient.

## 🚨 NEVER PIN A DEPENDENCY TO GIT OR PATH — RELEASE IT

A `github:` / `git:` / `path:` dependency in `mix.exs` (or the equivalent in `package.json`, `Cargo.toml`, `pyproject.toml`) is a rejection, not a solution. It applies to our own libraries above all: a library change needed by an app is a task in the **library's** repo, released through Hex (or the registry of its ecosystem) with a version bump, and then consumed as `{:lib, "~> x.y.z"}`. Pinning the app to a branch commit ships unreviewed library code through the app's review, freezes the app on a moving PR, and leaves a repo the operator has to remember to release later.

- **Implementer:** the fix belongs in the library → stop and report "blocked on a `<lib>` release: needs `<change>`". Do not open a PR against the library from inside the app run and pin its head. Do not vendor the code into the app either.
- **Reviewer:** a new `github:` / `git:` / `path:` dep on a package we maintain is a `reject` with that reason, regardless of how good the rest of the diff is. A new pin on a third-party package is a `reject` unless the task body names the pin and why no release exists.
- **Only exceptions:** `in_umbrella: true` inside one umbrella, and a pin the task body explicitly authorizes with the upstream release it waits for.
- **Precedent:** aave_sim task 148 pinned `bourse` to a branch head of its own open PR; the reviewer approved it, and the release still had not happened a week later.

## No scope-sequencing qualifiers in durable artifacts

Never write "X first", "starting with X", "initially", "for now", "MVP: X" into repo descriptions, READMEs, moduledocs, code/config comments, commit messages, or vision one-liners. They metastasize and become unremovable. Sequencing lives in the roadmap only (milestones, task bodies, `out_of_scope`). Elsewhere describe what the system IS: "Coverage: Robinhood Chain tokenized equities", not "starting with Robinhood Chain".

## Integrity and accuracy

- Never fabricate information, experience, metrics, timelines, or stats.
- Distinguish codebase observation / general knowledge / best practice / speculation.
- No false authority: no "we learned" without repo evidence, no "after X years in production".
- Uncertain → say so, give ranges over false precision, suggest a validation path.
- Trace sources: "Based on the code in file.ex…", "According to docs/FILE.md…", "Common practice in Elixir…".

## Research before asserting on niche technical claims

Outside reliable training coverage, research proactively — unasked. WebFetch when the canonical URL is known, WebSearch to find one. **Cite what you fetched.**

Research:
- **Wire formats / encodings** — RLP, ABI, SSZ, Protobuf, BLS, BIP-32/39/44, EIP-712, CBOR, ASN.1/DER. Never claim byte order, length-prefix, padding, or canonical form from memory.
- **Protocol details** — EIPs, RFCs, JSON-RPC shapes/error codes, opcode gas, exchange API quirks.
- **Niche / recent library APIs** — about to write `# probably something like`? Fetch the docs.
- **Cross-implementation edge cases** — check ≥2 reference impls; one impl's behavior can be a bug, agreement across two is the spec in practice.

Don't research: pure Elixir/OTP, stdlib, mainstream Phoenix/LiveView/Ecto/Ash, generic REST/HTTP/JSON/SQL/shell, anything in the codebase or an imported CLAUDE.md.

Fetch fails or is ambiguous → say so and lower confidence. Never fall back to "well, I think…" silently.

## No evasion — sit with the hard thing

Hitting a wall → silently moving to easier work is the failure. Stay with it; say "this is hard because X".

Don't use without explicit user approval:
- "let's move on to", "we can defer this", "skip this for now", "let's come back to this later", "let's table this"
- "to keep things simple, I'll skip", "for brevity, I won't", "that's out of scope", "not strictly necessary"
- "that should be enough", "the rest is straightforward", "I'll leave the rest as an exercise"
- "you might want to", "you could manually", "you'll need to handle"

- Blocked → name it: "blocked on X because Y. Options: A, B, C."
- Never a silent workaround. Tempted to add a fallback/nil-guard for missing data → should it come from upstream? Then stop and report.
- Must move on → leave a tracked TODO, not a silent gap.

<!-- @-import: ~/.claude/includes/harness-guardrails.md -->
## Harness Guardrails (eager)

Always-on floor for repos that dispatch through harness. These rules fail by non-recognition — the moment they apply doesn't feel like a moment to look anything up — so they stay ambient. Everything else (loop, dispatch-vs-hand-build, verdict table, routing, landing mechanics, orchestrator loop) lives in the **`harness:harness-workflow` skill**: invoke it before planning, dispatching, reading a verdict or recovering a run. API surface: `harness:harness-driver`.

**🚨 Origin is the source of truth for what landed** — not a local `tasks.toml`, not an await return, not a transcript. Under auto-land the lander pushes from a detached worktree and `TargetSync` often skips your checkout (dirty tree, non-ff, self-host), so local status lags. Before concluding "didn't land": `git fetch origin <target>` and check `git log --oneline origin/<target>` for `task <id> -> done (shipped …)`. Misreading stale local status re-dispatches and **duplicate-lands shipped work**.

**🚨 Settle ≠ landed.** `state: :done, verdict: approve` means *queued to land*; the serialized lander rebases and pushes afterwards (under `:pr`, `done --shipped-in` waits for the PR merge). Don't gate the next wave on approval — confirm the land on origin.

**🚨 Never block on `dispatch-await*` for real runs.** The MCP idle timeout (Claude Code: 300 s) kills the call while the run keeps going. Arm one bounded background watcher that greps `$BASE..origin/<target>` (baseline is load-bearing — never the whole log) and has a deadline. Don't micromanage in-flight runs; `dispatch-status` is for diagnosing a run that isn't landing.

**🚨 Recover, don't redo — committed work is paid for.** Before any reset-to-`pending` + re-dispatch, check `git log --oneline origin/<target>..harness/<run-id>`. Commits present ⇒ recover:

| Retained `harness/<run-id>` with commits | Primitive |
|---|---|
| Approved, unlanded (land-cap, conflict, lander crash) | `dispatch-reland` — zero agent tokens |
| Good work, review-stage failure | `dispatch-rereview` |
| Implement-stage incomplete / `:failed` | `dispatch-resume_failed` (`escalate: true` to re-route) |
| Live `:held` run | `dispatch-resume` (question-held: `dispatch-steer` first) |
| No commits, no retained branch | reset → `pending` + `dispatch-task` — the only full redo |

Land conflict → repair worktree off `origin/<target>`, resolve, repoint the branch, `dispatch-reland`. Never hand-push to the target when a reland can land it.

```

Everything else is **skill-on-demand** (Opus 4.8 self-invokes the matching skill): `worktree-workflow` (`workflow:git-worktrees`), `task-prioritization` (`tasks:roadmap-planning`), `task-writing` (`tasks:task-writing`), `rmap` (`tasks:rmap`), `workflow-philosophy` (`workflow:workflow-philosophy`). Re-add an `@`-import per-surface only if you observe Opus failing on it eager.

Deliberately NOT imported: `across-instances` (rmap is a work/tool repo, not a presence repo); Elixir/Phoenix includes (don't apply to a Rust CLI); delegation includes (rmap has a GitHub remote `ZenHive/rmap` but no Linear/cloud-agent PR flow); `web-command` (rmap does no browser work — the skill covers the rare case).

## Commands

```
cargo build                              # compile
cargo test                               # unit + integration + golden tests
cargo test --test cli                    # run a single test file
cargo test render_command_updates        # run tests matching substring
cargo fmt --check                        # check formatting (rustfmt defaults)
cargo clippy --all-targets -- -D warnings
cargo run -- validate                    # exercise CLI during dev
cargo run -- render --dry
```

Edition is `2024` (Cargo.toml). MSRV: whatever ships with Rust 1.85+.

**Reinstall after any source change.** rmap is dogfooded on itself, so a stale `~/.cargo/bin/rmap` will silently render with the old schema or reject TOML using a newly-added field. After any change to `src/`, run `cargo install --path .` before invoking `rmap` again. The `skills_smoke` test compiles fresh under cargo and catches `SKILLS.md` regressions regardless of the installed binary; interactive `rmap` calls do not.

**Bump `version` in `Cargo.toml` on every user-visible change.** `rmap --version` reports the crate version, and rmap is dogfooded across the portfolio — a stale version means neither the user nor a consuming agent can tell which binary is installed, and "reinstall and retry" stops being a checkable instruction. Bump in the same commit as the change, never as a follow-up. On a 0.x CLI: a new command, flag, task field, JSON key, or render affordance is a **minor** bump (`0.N+1.0`); a bugfix, message change, or docs-only edit is a **patch** bump. This is orthogonal to `schema_version` — a breaking agent-contract change bumps both. Pair the bump with a `CHANGELOG.md` entry under `[Unreleased]` (cutting a release renames that heading to the version with a date), and re-run `cargo install --path .` afterwards so `rmap --version` on this machine matches the source.

**Prefer `rmap` CLI over direct `tasks.toml` edits when a mutator exists.** Today the mutator surface is `rmap status` (single + bulk), `rmap mark`, `rmap depend`, and `rmap new` — all go through `toml_edit` and validate-then-write. Direct edits are the only path for everything else (bundle/phase CRUD, focus, scores, titles, ACs, top-level metadata); after any direct edit run `cargo run -- validate --check-render` before committing.

**Keep `~/.claude/includes/rmap.md` in sync.** It's the consumer-facing decision-layer doc (which command, when) imported by every rmap-using project's CLAUDE.md, including this one. Update it in the same commit when the command surface or a user-visible schema affordance changes. `rmap.md` deliberately does NOT enumerate fields — `rmap schema` / `rmap --help` is authoritative.

**Evaluate roadmap & task design from the consuming-agent POV.** rmap IS a tool for Claude. When picking up rmap tasks or reviewing the roadmap, evaluate the design from the perspective of the agent (Claude) that will actually consume the tool day-to-day — not just the perspective of the AC author. If the AC overspecifies in ways that hurt daily ergonomics, push back and propose the consumer-first alternative in the plan. Examples: a transition that requires manual TOML edit between two commands is friction Claude will route around; a field that's not surfaced in `rmap show` is invisible to the consumer; a section ordering optimized for one rare-path command (delegate) at the cost of the daily-path command (show) is the wrong tradeoff. (`feedback_decide_as_consumer.md` captures the same posture as personal memory; this rule lifts it to the project so it's authoritative for all rmap work.)

## Architecture

Pipeline: `tasks.toml` → `schema::Tasks` → `render_roadmap_str` (markdown), `export_json_str` (data.json), or `render_html_str` (static HTML, opt-in via `--html`) → write back. Mutations route through `toml_edit` to preserve user formatting and comments.

Modules — each file's doc comment is the authoritative reference for its internals:

- `schema.rs` — `serde` structs, `#[serde(deny_unknown_fields)]` everywhere. `TaskId` is `Number(u32) | Text(String)` untagged.
- `validate.rs` — semantic checks on a parsed `Tasks` (versions, statuses, markers, score range, deps, cycles, references, timestamps).
- `render.rs` — three-pass marker walker over `ROADMAP.md`: focus → mermaid → tasks. Only bytes between matched marker pairs are rewritten.
- `export.rs` — `ExportedTask` JSON shape; adds computed `eff`. Pretty by default; `_compact_` variant feeds the HTML data island.
- `render_html.rs` — `rmap render --html` single-project view (`templates/roadmap.html.j2`) and `--html --multi` portfolio view (`templates/portfolio.html.j2`). Both build from the exported JSON envelope through one owned view path (lanes ready/active/hold/waiting/done, dispatch racks), sharing component macros (`templates/_components.html.j2`), one stylesheet (`templates/_styles.css`) and one board script (`templates/_board.js`, detail panel reads the `rmap-data` island); all `include_str!`'d. Visual system: `templates/DESIGN.md`. The portfolio half lives in `render_html/portfolio.rs`. Builds DAGs via the longest-path layering in `topo.rs`. Portfolio inputs are `ProjectInput` envelopes (project root, `tasks.toml`, or `data.json` path — see `commands/render.rs::load_project_input`).
- `topo.rs` — pure longest-path layering `compute_layers(tasks) -> {id → depth}` over the in-repo `depends_on` graph (extracted from `render_html`), plus `compute_layers_from_edges` over pre-extracted `(id, deps)` lists (the portfolio's JSON-loaded tasks). Shared by `render_html.rs` (DAG vertical slotting) and `export.rs` (the computed `dep_layer` field).
- `graph_export.rs` — read-only graph surfaces over the in-repo `depends_on` graph. `build_waves` / `format_waves` / `format_waves_json` (`rmap waves` — parallel dispatch schedule grouped by `topo` `dep_layer`, wave 0 = roots) and `build_dot` / `format_dot` (`rmap export dot` — Graphviz DOT digraph, edges dependency → dependent). Both pure; never mutate.
- `mutate.rs` — `status` / `mark` / `depend` / `new` paths. All use `toml_edit::DocumentMut` and end with `validate_tasks_str` before returning.
- `next.rs` — pure selectors: `next_tasks(tasks, filter, count)` (highest-Eff pending unblocked) and `ready_tasks(tasks, filter, count, dispatchable)` (the whole parallel-safe set), both ranked via shared `next::rank_tasks` — a 4-tier lexicographic key (focus phase × active milestone, **focus dominant**) then Eff descending, then computed `unlocks` descending. `is_unblocked` is the dep-satisfied predicate; the `active` milestone set is computed once per call from `tasks.milestones`.
- `query.rs` — `show` / `list` read paths. `TaskFilter` + `find_task` + `list_tasks` pure; `effective_target_repo` resolves an omitted task target to the roadmap's `project`; humans get `format_task*`, JSON delegates to `export.rs`.
- `bundles.rs` — `rmap bundles` read path. Per-bundle `next_task` reuses `next::next_task`.
- `next_bundle.rs` — `rmap next-bundle` pure selector. Broad actionability (in-bundle pending deps satisfy if themselves actionable). Topological emit via Kahn's.
- `delegate.rs` — `rmap delegate` prompt formatter. Read-only Markdown; never calls Linear/GitHub/Slack.
- `import.rs` — `rmap import` prompt formatter. Interpolates project name + live JSON Schema into `templates/import_prompt.md`.
- `diff.rs` — `rmap diff` engine. `diff_toml(base, current, verbose) -> TomlDiff`. Per-task field walk generated from `task_fields.rs`.
- `schema_json.rs` — JSON Schema for `Tasks` via `schemars` derives.
- `scoring.rs` — shared `efficiency`, `format_efficiency`, `tier_glyph`, date helpers. Pure-integer Howard Hinnant date math; no chrono.
- `stale.rs` — pure `parse_duration` + `find_stale`. No I/O.
- `doctor.rs` — soft-signal aggregator. Always exits 0; strict gates remain on `validate`.
- `paths.rs` — ancestor walk to find `roadmap/tasks.toml`. CLI flag overrides; `html_path` is fixed-derived.
- `watch.rs` — pure helpers for `rmap watch`. `write_if_changed` (idempotency primitive), `is_tasks_toml_event` (filter), JSON event-line builders.
- `cli.rs` — `clap` command declarations. `main.rs` parses and dispatches; `commands/` owns creation, interactive prompts, mutations, render/watch and shared output helpers.
- `task_fields.rs` — task field registry; `creation_input.rs` derives creation inputs and writes. `vocabulary.rs` owns typed status, marker and relation wire vocabularies.

## Load-bearing invariants

Easy to violate without breaking tests immediately. The "why" lives in source doc comments and tests; this list is the index.

**Render & markers**
- **Marker boundaries are byte-preserved** (TASKS / FOCUS / MERMAID / VISION / MILESTONES). Don't normalize input bytes outside matched pairs.
- **FOCUS / MERMAID / VISION line shape and empty-state strings are agent-grep contract.** Changing wording is a `schema_version` bump.
- **Archive collapse triggers on `phases.N.status = "done"`** and emits `> N tasks.` plus an optional `See [<basename>](<path>#phase-N-<slug>)` link. Effective path is per-phase `changelog` → top-level `changelog_path` → `CHANGELOG.md`; `false` omits the link. Unconfigured roadmaps keep the historical `CHANGELOG.md#phase-N-<slug>` line. Locked by `validate --check-render`.

**Schema & validation**
- **`schema_version = 2` is required.** Bump on any breaking schema change.
- **`eff` is never persisted.** Computed at render/export time; `schema::Task` would reject it via `deny_unknown_fields`.
- **D/B/U range is `1..=10`.** The error message string `must be in 1..=10` is part of the agent-grep contract.
- **`linear_id` validation is conditional** on `[linear]` table presence (Linear is opt-in).
- **`blocked_reason` is required iff `status = "blocked"`.** Mutator re-validates, so the transition can't write without one. Settable via `rmap status <id> blocked --reason "<text>"` (free-text, overwrites, blocked-only — ignored with a one-line stderr note on other transitions, mirroring the outcome flags). **Auto-cleared when a blocked task leaves the blocked state** (the reason described a state that no longer holds); re-blocking keeps/overwrites. No interactive prompt — if neither `--reason` nor an existing value is present, the validation error fires and the file stays byte-equal.
- **`implemented` is required and non-empty iff `status = "done"`.** Mirrors the `blocked_reason` pattern. Error string `is done but missing implemented` is agent-grep contract.
- **`model` is required and non-empty iff a *live agent-assigned* task** — `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`. Hard `validate` error (`validate_dispatch_model`), not a soft advisory: harness hard-rejects a dispatch that resolves to no model (it never falls through to the agent CLI's ambient default), and the roadmap is the source of truth harness ingests, so rmap refuses to author a dispatch that would be rejected. Scope is narrow by design — **terminal tasks** (`done`/`superseded`/`blocked`) are exempt (a missing model is now historical, not an authoring error, and requiring it would retroactively fail `validate` on shipped work), and **assignee-unset / `human`** tasks are exempt (no agent chosen → nothing to pin). Mutators inherit the gate via validate-then-write (`rmap new --assignee <agent>` on a model-less task fails before write). Error string `missing model` is agent-grep contract. No `schema_version` bump (`model` already exists per Task 19).
- **`acceptance_criteria` is required and mechanically non-blank iff a *live agent-assigned* task** — the same predicate and exemptions as `model`. rmap enforces only the presence of observable completion criteria; AI task authors and reviewers judge whether those criteria are meaningful and reality-grounded. Error substring `missing acceptance_criteria` is agent-grep contract.
- **`delivered_by`, `verified`, `verified_by`, and `verification_ref` are transition-time outcome fields settable only on `status = "done"`.** `delivered_by` names the implementer. `verified = true` means an independent evaluator confirmed the task; a new `--verified` transition hard-requires non-empty `--verified-by`, while optional `--verification-ref` points durably to the evidence (harness run, CI URL, review artifact). Direct TOML provenance is rejected when blank or when `verified != true`. Legacy schema-v2 rows with `verified = true` but no `verified_by` remain valid and produce soft `VerifiedWithoutProvenance` doctor findings; this avoids retroactively invalidating every existing roadmap without a schema-v3 migration. `verified` absent remains honestly ungraded and produces `ClaimedNotGraded`.
- **`landing_ref` is a transition-time free-text field, settable only on `status = "in_progress"`.** Records an open landing pointer (PR URL or other ref — never parsed or fetched). Set via `rmap status <id> in_progress --landing-ref <ref>`; an `in_progress` → `in_progress` call with only this flag is a field update (`started_at` unchanged). `--landing-ref ""` clears it. The flag is a hard error on any other target status (message names the rule; file stays byte-equal). Lifecycle without the flag: **kept on `done`** (provenance next to `shipped_in`) and **kept on `blocked`** (a PR closed unmerged is when the ref matters); **cleared on `pending`** (the work is being redone — the old ref belongs in the appended `attempts` report). Surfaces: `rmap show` prints `landing_ref:` when present; ROADMAP.md rows get a conditional `🔗 <ref>` segment; `--json` / `data.json` / `rmap diff --verbose` (verbose flag in `task_fields.rs`). `rmap stale` and `rmap doctor` exclude in_progress tasks carrying a `landing_ref` from the stalled list and report them under `awaiting landing (N)` (JSON kind `awaiting_landing`). Absent from `StdinTask` / `NewTaskFields`. No `schema_version` bump — additive optional field.
- **`attempts` is an append-only transition-time list, settable only on `status = "pending"`.** Each entry is an inline-table `{ at, by?, report }` (stored like `cross_repo`): `at` is auto-filled from `today_iso()`, `by` is free-text agent attribution (optional), `report` is the failure evidence (a reviewer's rejection report). Appended — never overwritten — by `rmap status <id> pending --report "<text>" [--attempt-by <agent>]`; each call adds exactly one entry (no dedup). `--report` on a non-`pending` transition is ignored with a one-line stderr note (mirrors the outcome/`--reason` flags); `--attempt-by` without `--report` is likewise a no-op note. Renders in `rmap show` and `rmap delegate`'s `## Prior attempts` section; surfaces in `--json` / `data.json` (skipped when empty, so attemptless tasks round-trip byte-identically). Transition-time registry entry; deliberately excluded from verbose diff values (a list, like `cross_repo`).
- **`rmap doctor` milestone drift advisories are soft (exit 0, no auto-mutation).** `MilestoneFullyDoneButOpen` when every task pinned to a milestone is `done` but milestone status is `pending` or `active`; `MultipleActiveMilestones` when more than one milestone is `active` (rmap.md: keep exactly one). Human lines cite milestone slug and pinned-task count; JSON kinds `milestone_fully_done_but_open` / `multiple_active_milestones`.
- **`rmap doctor` phase / focus drift advisories are soft (exit 0, no auto-mutation; phase/focus state is user-curated).** `PhaseFullyDoneButOpen` when every task in a phase is `done` but phase status is still `pending`/`active`; `PhaseHasInProgressButPending` when a `pending` phase has ≥1 `in_progress` task; `FocusPhaseClosed` when `focus.phase` points at a phase that appears closed (done status or all tasks done). Human lines cite the phase number (and task ids for the in-progress case); JSON kinds `phase_fully_done_but_open` / `phase_has_in_progress_but_pending` / `focus_phase_closed`.
- **`rmap doctor` graph-health advisories are soft (exit 0, no auto-mutation).** `Bottleneck` when a `pending`/`blocked` task's transitive-dependent count (via `topo::compute_unlocks`) is ≥ `DoctorThresholds.bottleneck_min` (default 3, overridable via `--bottleneck-min`); `IsolatedNode` when a task is an orphan (no in-repo deps and no dependents) or unreachable forward from any milestone-pinned task when milestones exist. Human lines cite task id + dependent count (bottleneck) or id + reason (isolated); JSON kinds `bottleneck` / `isolated_node`.
- **`rmap doctor` spec-quality advisories are soft (exit 0, no auto-mutation).** Token-mechanical only (no semantic/LLM judgment). **AC quality** is scoped to *live agent-assigned* tasks (same predicate as `validate_dispatch_model`: `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`): `PlaceholderCriteria` when any criterion has an unquoted whole-word `TODO`/`TBD`, substring `???`, or `<stub>` angle-bracket token (quoted example mentions are ignored so prose ACs that name the tokens do not self-flag); `VagueCriteria` when a criterion's non-stopword tokens are drawn entirely from the fixed list `fast` / `robust` / `properly` / `correctly` / `works` (a measurable criterion that merely contains one of those words does not fire). **Near-duplicates** pair open tasks (`pending`/`in_progress`/`blocked` only — never `done`/`superseded`) whose normalized title+body token Jaccard is ≥ `DoctorThresholds.near_duplicate_min` percent (default 80, overridable via `--near-duplicate-min`). Human lines cite task id(s); JSON kinds `placeholder_criteria` / `vague_criteria` / `near_duplicate_tasks` (additive).
- **`rmap doctor` spec-test advisories are soft (exit 0, no auto-mutation, not a validate failure).** When `[spec_tests]` lists project-root-relative `globs` and the roadmap registers specs, doctor scans those files as plain text for a marker (default `spec-tags:`). `UntestedRule` fires for each current rule of an `active` spec that no scanned file tags; draft and retired specs are exempt. `UnknownRuleTag` fires for each tag naming a rule id that is not current text in any registered spec. Human lines cite the rule id and file; JSON kinds `untested_rule` / `unknown_rule_tag`. Absent `[spec_tests]` or an empty `[specs]` table leaves doctor output unchanged. Scanning does not parse a language or run tests.
- **`touches` is an optional creation-time free-text list, advisory and unvalidated** (posture of `model` / `assignee`). Semantically distinct from `files_to_modify`: `files_to_modify` is the implementer's *write target*; `touches` is the broader *involvement hint* (files that may be read or written) — typically a superset. Consumer collision rule (documented, NOT enforced in rmap): two tasks conflict iff `(touches(A) ∪ files_to_modify(A)) ∩ (touches(B) ∪ files_to_modify(B)) ≠ ∅`. Creation-time registry entry; included in diff fields but excluded from verbose diff values.
- **`context_refs` is an optional creation-time free-text list of paths or URLs to read before starting.** Unvalidated — a missing path is the consumer's problem. Distinct from `files_to_modify` (write target) and `touches` (collision hint). `rmap show` prints the list when non-empty; `rmap delegate` and the HTML detail panel render it as Read first, and omit the heading when the list is empty. Creation-time registry entry (canonical rank 13, between `out_of_scope` and `files_to_modify`); included in diff fields, excluded from verbose diff values; omitted from export when empty.
- **`checks` is an optional creation-time free-text list of commands that demonstrate the acceptance criteria.** Hints for the reviewer: rmap never executes them, and nothing in the contract implies a consumer (including harness) does. `rmap show` prints the list when non-empty; `rmap delegate` and the HTML detail panel render it as Reviewer checks, state that the commands are hints rather than an automated gate, and omit the heading when the list is empty. Creation-time registry entry (canonical rank 11, immediately after `acceptance_criteria`); included in diff fields, excluded from verbose diff values; omitted from export when empty. Doc examples span ecosystems (`cargo test --test delegate`, `mix test test/harness/roadmap_test.exs`).
- **`domains` is an optional creation-time free-text list, advisory and unvalidated.** It tags the capability/domain evidence an orchestrator may group by (for example `rust`, `otp`, `ecto`); rmap owns no vocabulary and only carries/export the strings. Creation-time registry entry; included in diff fields but excluded from verbose diff values (a list, like `touches` / `cross_repo`).
- **`target_repo` is the optional creation-time landing-repository field.** It names where this task's own work lands; absent resolves to the roadmap's top-level `project`, preserving existing source/render bytes. An explicit blank value is invalid. `cross_repo` remains a list of relationships to related tasks in other roadmaps and does not affect landing. `rmap list --target-repo` filters on the effective value and composes with other list selectors; `rmap delegate` always renders the effective value. Creation-time registry entry with verbose diff values enabled.
- **Timestamps validate by shape (`YYYY-MM-DD`), not semantics.** `9999-99-99` passes on purpose; values live next to user-edited TOML.
- **Task status / marker / cross-repo relation are typed enums in `vocabulary.rs`.** Their wire strings and validator vocabularies share one declaration; render glyphs match enum variants. Unknown inputs survive deserialization for the existing semantic diagnostics. JSON schema retains its string wire shape.
- **Milestone status enum (`pending | active | done`) lives in `validate.rs::VALID_MILESTONE_STATUSES`** — distinct vocabulary from task status. `rmap milestones` sort order is `(status_rank: active=0/pending=1/done=2 asc, milestone.order asc)`; "active first" is load-bearing for the daily release-cut query.
- **`task.milestone` references must resolve in `tasks.milestones`.** `validate_milestone_references` enforces this; mutator pre-validates before writing.
- **`TaskId` `Eq`/`Hash` are normalizing across `Number(n)` ↔ `Text("n")`.** A task id is a primary key; the disk form (TOML integer vs TOML string of the same digits) does not change which task it names. `validate_unique_ids` relies on this to catch cross-form collisions; `validate_dependencies` / `validate_dependency_cycles` / `doctor.rs` degenerate-bundle check / `next_bundle.rs` actionability memo are correct on mixed-form files because of it. Text ids that don't parse as `u32` (e.g. `"INE-5"`, `"alpha"`) keep their own canonical key. Agent-grep substring for the duplicate-id error is `duplicate task id`.

**Mutations**
- **All mutators use `toml_edit::DocumentMut`** (never `toml::from_str`) and end with `validate_tasks_str(...)` before returning. Invalid mutations leave the file byte-equal.
- **Bulk `rmap status 1,2,3 done` is atomic** — all-resolve-or-no-write. Don't add a "best effort" flag without explicit user request.
- **`rmap status` is the only mutator that auto-fills lifecycle timestamps** (`done_at`, `started_at`). Never overwrites existing values. Changing this is a `schema_version` bump.
- **`rmap mark` and `rmap status` auto-sort task keys when inserting a new field**; idempotent calls do not. `add_dependency_str` deliberately does NOT auto-sort.
- **`rmap new` auto-allocates numeric IDs only.** `TaskId::Text` is never auto-generated. Duplicate explicit IDs error before any write.
- **Lifecycle timestamps are NOT settable on creation.** `started_at`, `done_at`, `blocked_reason`, `shipped_in`, `landing_ref` are absent from `NewTaskFields` — those are transition fields owned by `rmap status`.

**Agent contract (renaming/removing breaks consumers)**
- **`--json` outputs of `show` / `list` / `next` / `next-bundle` / `ready` / `bundles` / `schema` / `diff` / `doctor` / `blocks` / `deps` are additive-only.** Add fields freely; rename/remove → `schema_version` bump. `blocks` / `deps` emit a `list`-shaped envelope (`export_filtered_json_str`).
- **`rmap next --count` JSON shape is split by N**: default (`--count 1`) emits a bare object/null; `--count >1` emits an array. Flipping default-to-array is a bump.
- **`rmap next` ranking is 4-tier lexicographic** `(in_focus_phase × in_active_milestone) ⇒ Eff desc ⇒ unlocks desc`, focus dominant: tier 0 (both) > tier 1 (focus-only) > tier 2 (active-milestone-only) > tier 3 (neither). Tasks pinned to ANY milestone with `status = "active"` qualify. Without `[focus]`, every task counts as "in focus" → tiers collapse to 0/1. The focus-dominance bit (tier 1 > tier 2) is the load-bearing decision. `next::rank_tasks` is the single source of this sort, shared with `ready`.
- **`rmap ready` is the parallel-safe dispatch set**: all `pending` tasks whose every `depends_on` is `done`, ranked by the same 4-tier key as `next` (via `next::rank_tasks`). The set is **mutually independent by construction** — a pending task whose deps are all `done` cannot depend on another pending task — so there is NO `--independent` flag (it would be a no-op). Unlike `next`, `--count` is optional (default = the whole set) and `--phase` filters the pool. `--bundle B` = the dispatchable layer-0 of B. `--json` is a `list`-shaped envelope.
- **`dep_layer` is never persisted.** Computed at export time (`src/topo.rs` longest-path depth over the in-repo `depends_on` graph); like `eff`, `schema::Task` rejects it via `deny_unknown_fields`. Always built from the FULL `tasks.task` graph, never a filtered slice — `export`'s slice-taking fns (`export_task_json_str`, `export_tasks_array_json_str`) take `&Tasks` for exactly this. Surfaces on every `--json` payload (additive).
- **`unlocks` is never persisted.** Computed at export time (`src/topo::compute_unlocks` — transitive-dependent count over the reverse `depends_on` graph); like `eff` / `dep_layer`, rejected by `deny_unknown_fields` and built from the FULL graph (`export::graph_metrics` bundles `layers` + `unlocks`, computed once per export call). It is the set-size of `rmap blocks <id>`; turns the `U`-score's unlock-leverage component into a graph fact. Surfaces on every `--json` payload (additive).
- **`--dispatchable` (on `ready` / `list`) excludes `handbuild`-marked tasks.** `handbuild` ∈ `VALID_MARKERS` flags human-driven-browser work (LiveView/UI/DOM) — the minority exception, so everything else is headless-dispatchable by default. `query::is_dispatchable` is the predicate; on `ready` it filters before `rank`+`count` so the cap counts only dispatchable tasks.
- **`rmap list --target-repo <repo>` filters by where each task's own work lands.** Explicit `task.target_repo` wins; an omitted field matches top-level `project`. The filter is AND-composed with status / phase / marker / bundle / milestone / delivered-by / dispatchable selectors. It never follows or filters `cross_repo` relationships.
- **`--fields a,b,c` (on `ready` / `list`) projects `--json` to a bare array** of objects carrying only the named keys (envelope dropped — token-cheap). Implies `--json`; unknown name → exit 1 naming the offender, validated against `export::EXPORTED_TASK_FIELDS`. Absent optional keys simply don't appear per task.
- **`rmap next-bundle` ranking is `(in_focus_phase desc, sum_eff desc, bundle.order asc)`** and the three empty-state stderr spellings are load-bearing.
- **`rmap bundles` row separator and five-branch glyph ladder** (`✅` / `🚧` / `all-blocked ⛔` / `pending:<n> (deps unmet) ⏸` / `next:<id> [Eff:x.y] <tier_glyph>`) are agent-grep contract.
- **`rmap milestones` mirrors `rmap bundles`'s five-branch glyph ladder** and adds a trailing `[target=<version>]` segment when `milestone.target_version` is set. Sort key and row shape are agent-grep contract.
- **MILESTONES marker section is opt-in and grouped**: `<!-- MILESTONES:BEGIN -->` / `<!-- MILESTONES:END -->` renders one block per milestone sorted like `rmap milestones`, including name, target_version, status glyph, hypothesis description, and done/total pinned-task counts. Roadmaps without the marker pair render byte-identically.
- **Render-row 🚀 segment is conditional + positional**: `🎁 **bundle** · 🚀 **milestone** · {module} · {category} {title}`. Inserted between bundle and module_segment; emitted only when `task.milestone.is_some()`. Rows without a milestone render byte-identically to pre-Task-24 — regression-guarded by golden fixtures.
- **Render-row `⛔ {blocked_reason}` segment is conditional + trailing**: appended after the tier glyph, emitted only when `task.status == "blocked"` and `blocked_reason` is non-empty. Non-blocked rows (and blocked rows are the only ones affected) render byte-identically otherwise — additive, golden-guarded by `tests/golden/mermaid_block`.
- **Render-row `🔗 {landing_ref}` segment is conditional + trailing**: appended after the tier glyph (before the blocked_reason segment), emitted when `landing_ref` is non-empty regardless of status. Rows without it render byte-identically — additive, golden-guarded by `tests/golden/landing_ref`.
- **Eff tier glyph is centralized in `scoring::tier_glyph`** (`>=2.0 🎯 / >=1.5 🚀 / >=1.0 📋 / else ⚠️`). NEVER fold into `format_efficiency` — JSON payloads must stay numeric.
- **`rmap delegate` sections, when present, follow:** Context → Read first (`context_refs`) → Task → Acceptance criteria (`- [x]` when status is `done`, otherwise `- [ ]`) → Reviewer checks (`checks`; hints for the reviewer, not an automated gate — rmap never executes them and harness does not either) → Out of scope → Files to modify → Scoring → Environment notes. The `[D:_/B:_/U:_ → Eff:_] <glyph>` Scoring shape is unchanged. Locked by `emits_canonical_section_order_with_distinguishing_line`. Optional sections (Read first, Reviewer checks, Task, Acceptance criteria, Out of scope, Files to modify, Prior attempts, What was actually implemented) are omitted when empty. `## Instructions` stays after Environment notes and is outside this locked list.
- **`rmap delegate` always emits `Target repo` in `## Context`.** It uses explicit `task.target_repo` or falls back to top-level `project`, so the receiving agent can distinguish the planning repository from the checkout its work targets.
- **`rmap delegate --to` is optional; `assignee` is the routing default.** `delegate::resolve_target` resolves the target: explicit `--to` always wins (and renders the `Stored assignee: ... (overridden)` bullet when it differs); without it the task's `assignee` IS the target. No assignee → exit 1 `has no assignee; pass --to <agent>`; `assignee = "human"` → exit 1 `is assigned to human; pass --to <agent> to delegate anyway`. Both error strings are agent-grep contract. Routing metadata is split: `assignee` = which agent executes, `model` = free-text LLM id/pin, `domains` = free-text capability tags for downstream scoring, `delegate --to` = render-time override.
- **`rmap delegate`'s per-agent footer describes that agent's execution runtime and names no language, toolchain, or package manager.** Do not copy toolchain names out of `~/.claude/includes/cloud-agent-environments.md`. Locked across all eight `--to` targets by `environment_footer_names_no_language_toolchain_or_package_manager`. Distinguishing phrases (Codex "Network access varies", Cursor "Run the full project harness", Claude "Local execution", Grok "Reads AGENTS.md for project conventions.", Antigravity git-common-dir + `agy`, Pi "Runs a local LLM (free/unmetered)", Droid "Not yet a harness executor") stay.
- **`rmap diff --against` defaults to `current.default_branch`** — never hardcode `"main"`.
- **`rmap diff --verbose` is additive.** Non-verbose output stays byte-identical to pre-11b. Verbose flags in `task_fields.rs` and members of `METADATA_VERBOSE_WHITELIST` are part of the contract.
- **HTML data island id is `rmap-data`, script type `application/json`.** Agents parse the element's text; they do NOT scrape the DOM.
- **HTML task cards carry six `data-*` attributes** (`data-id`, `-status`, `-eff`, `-markers`, `-depends-on`, `-phase`); DAG nodes carry `data-id`; phase sections carry `data-phase-status`. Stable selector contract.
- **Portfolio HTML (`--html --multi`) adds two islands**: `rmap-data` (aggregate `{"projects":[…]}` of every input's envelope) and `rmap-relations` (resolved cross-repo edge array `{source, target, relation}`); repo rows carry `data-slug` / `data-name` / `data-has-rel`. Same parse-the-island-not-the-DOM rule.

**Mirror-surface edit rules**

Declare task fields once in `src/task_fields.rs`. Entries are in schema/export/diff order and carry `[canonical_rank, diff, verbose, export_policy]` plus creation input/borrowed types and conversion/writer policies. Empty creation brackets mean transition-time.

- **Creation-time optional field:** add one registry entry (copy `target_repo` for an optional string or `domains` for a list). A new field takes the next unused canonical rank unless it belongs between existing keys, in which case shift every later rank in the same commit so pre-existing keys keep their relative order. Update the field's regression test / fully-populated export test in the second location. The registry generates `Task`, `StdinTask`, `NewTaskFields`, the stdin allowlist, creation conversion and TOML writer, canonical order, diff walk, export serializer and `EXPORTED_TASK_FIELDS`. No hand-edited mirrors. Optional fields default in interactive creation; add a prompt only when the field needs one.
- **Transition-time field:** add a registry entry with empty creation brackets and update its owning mutator. It stays unavailable to `rmap new`.
- **Contract checks:** `tests/field_registry.rs` compares the actual Task schema against the export set and unique canonical positions, checks creation → writer → export → diff, rejects transition-time stdin fields, and checks typed wire vocabularies. `exported_task_fields_cover_serialized_keys` checks the fully-populated JSON surface.
- **New top-level field on `schema::Tasks`:** also update `diff::diff_metadata` and `export::ExportedTasks`; the task registry does not cover the top-level envelope.
- **Schema compatibility:** the published Task description is pinned with `schemars(description)` in `schema.rs` because the refactor preserves `rmap schema` byte-for-byte. Contributor instructions are this section and the Task Rust doc comment.

**Time & determinism**
- **`today_iso()` is the only source of "now".** Reads `RMAP_TODAY` env var first, falls back to `SystemTime::now()`. Date-sensitive tests MUST set `RMAP_TODAY` on the `Command` env (or `today.txt` for golden fixtures).

**Watch**
- **`rmap watch` watches the `roadmap/` directory** (not the file) with `RecursiveMode::NonRecursive`, and filters via `is_tasks_toml_event`. The filter is the infinite-loop guard against our own `data.json` write.
- **`rmap watch` event shape is the agent contract** — `schema_version` + `event` discriminator (`rendered` / `error`) + `outputs` / `message` are additive-only. Bump `WATCH_SCHEMA_VERSION` for renames.

**Exit codes**
- **`rmap doctor` always exits 0 on success** (informational only). Strict gates: `validate` (exit 1 on schema error), `validate --check-render` (exit 2 on render drift).

## Downstream consumer: harness (`../harness/`)

The "consumers" the agent contract above protects are not hypothetical — the primary one is **harness**, a sibling Elixir/OTP project at `../harness/` (`/Users/efries/_DATA/code/harness/`). Harness is an AI-orchestrator-driven task-execution engine: it pulls tasks from rmap roadmaps, dispatches each to a headless coding agent (Claude Code, Codex, Cursor, Grok, Antigravity, Pi) in an isolated git worktree, grades the result with the target project's own check stack, and writes the verified outcome back via `rmap status`. Harness's CLAUDE.md § "rmap is ours" sends roadmap-CLI gaps *here* to be fixed, never worked around in harness — this section is the reciprocal pointer.

**The shell-out surface (`../harness/lib/harness/roadmap.ex`, `Harness.Roadmap`).** Harness never parses `tasks.toml` itself; it shells out to the installed `rmap` binary and treats stdout as API. Every call passes an explicit `--tasks-path`; success is gated on JSON-decode (or non-empty delegate output), not exit 0. Commands consumed:

- `rmap next --json` · `rmap show <id> --json` · `rmap list --json [--status S]` · `rmap next-bundle --json` — browse/ingest
- `rmap ready --dispatchable --fields id,assignee,markers` — the cron poller's autonomous selection surface (MCP tool `roadmap__ready`); the poller routes each task on its `assignee`
- `rmap delegate <id> --to <agent>` — the verbatim output IS the prompt dispatched to the agent. `Harness.Roadmap.render_prompt/3` (private, `lib/harness/roadmap.ex`) returns that stdout untouched and does not parse `##` headers. Section order, when present: Context → Read first (`context_refs`) → Task → Acceptance criteria (`- [x]` when status is done, otherwise `- [ ]`) → Reviewer checks (`checks`; hints for the reviewer, not an automated gate — rmap never executes them and harness does not either) → Out of scope → Files to modify → Scoring → Environment notes. Optional sections (Read first, Reviewer checks, Task, Acceptance criteria, Out of scope, Files to modify, Prior attempts, What was actually implemented) are omitted when empty. Environment notes describe that agent's execution runtime and name no language, toolchain, or package manager. `@fingerprint_fields` remains title, body, acceptance_criteria, files_to_modify, out_of_scope — `context_refs` and `checks` stay outside that hash on purpose. `Harness.Lander.PR.acceptance_section/1` builds a separate PR-body `## Acceptance criteria` from the criteria list with plain bullets and does not read the delegate prompt. Checked against `/data/postgresql/harness/base/lib/harness/roadmap.ex` (this worktree's `../harness` is not the repo). No delegate-section-layout assumption required a harness code change. The same paragraph is in `../harness/skills/harness-driver/SKILL.md`; `scripts/sync-harness-skills.sh` is orchestrator-owned and was not run.
- Write-backs: `rmap status <id> in_progress` (on dispatch), `rmap status <id> in_progress --landing-ref <pr-url>` (PR-landed policy: record the open PR while the task stays in progress), `rmap status <id> done --verified --verified-by <reviewer> --verification-ref harness-run:<run-id> --shipped-in <sha>` (lander, after reviewer approval + push / PR merge), `rmap status <id> blocked --reason "..."` (terminal sink)

Changing any of these — JSON shapes, `--fields` projection, `delegate` section format, status flags, the `handbuild` semantics of `--dispatchable` — means checking `Harness.Roadmap` (and `Harness.Dispatch` / `Harness.Lander` / `Harness.Cron.RoadmapPoller`) in the same change, plus the consumer-side docs below.

**Per-task target repositories are renderable, not executable in harness.** rmap carries `target_repo`, filters it, exports it, and renders it into delegate prompts. Harness does not yet resolve that value to a checkout or route dispatch/landing to another repository; that consumer-side work remains a harness roadmap task.

**Renderable ≠ executable (the two-sided executor contract).** `rmap delegate --to` renders for **eight** agents (`claude` / `codex` / `cursor` / `grok` / `antigravity` / `pi` / `droid` / `kimi`); harness has `AgentAdapter`s for only **six** — `droid` and `kimi` are renderable but rejected at harness's dispatch boundary (`{:unknown_adapter, "droid"}` / `{:unknown_adapter, "kimi"}`). Adding a new `--to` target in rmap is half the job: the agent only becomes dispatchable once harness grows a matching `AgentAdapter` + `@valid_agents` entry. When widening rmap's delegate/assignee set, note the harness-side gap explicitly (a task in harness's roadmap, or a line in the commit) rather than implying end-to-end support.

**Consumer-side contract docs (update when rmap's surface changes underneath them):**

- `../harness/skills/harness-driver/SKILL.md` — the AI-orchestrator contract for driving harness (dispatch patterns, MCP tool surface `dispatch__*` / `roadmap__*`, result shapes). It documents rmap-derived behavior (the `ready --dispatchable` set, delegate-rendered prompts, the renderable-vs-executable split) and carries an explicit anti-staleness contract.
- `../harness/CLAUDE.md` — § "Agent Headless Entry Points" and § "Dogfooding" reference rmap's delegate targets and selection commands.
- `../harness/docs/dogfooding-workflow.md` — the operator runbook; verdict table references rmap status write-backs.

Harness registers projects (including itself) with a `roadmap_path` and drives them through this surface unattended (`Oban.Plugins.Cron`) — a silent break in rmap's JSON or prompt output surfaces as failed autonomous dispatches there, not as an rmap test failure here. The `skills_smoke` test and the additive-only invariants above are the local proxies for that contract; treat them as guarding harness specifically.

### Driving harness from this repo

The relationship also runs the other way: rmap's own roadmap tasks can be dispatched *through* harness (Context A of the harness-driver skill — consuming repo drives the harness BEAM). The wiring:

- **`.mcp.json`** registers two HTTP servers against the harness BEAM (user-started `iex -S mix` in `../harness/`; never boot it yourself): `harness` → `mcp__harness__*` (the native flat driver tools — `dispatch__task`, `dispatch__await`, `dispatch__status`, `dispatch__verdict_detail`, `roadmap__*`; **primary surface**) and `harness_eval` → `mcp__harness_eval__project_eval` (arbitrary-Elixir escape hatch into harness's BEAM, for struct-level ops the flat tools omit).
- **rmap is registered as a harness project** in harness's gitignored `config/dev.local.exs` (`:rust` preset, `roadmap_path` = this repo). Registration changes need a harness BEAM restart (the user does that). Per-project cron autonomy defaults OFF — registration alone does not start autonomous dispatch.
- **The workflow loop is eager** (`@~/.claude/includes/harness-workflow.md` — see § "Imports"): the delegate → verify → repair → land loop and delegation roster are in context every session. **Load on demand when driving:** `../harness/skills/harness-driver/SKILL.md` (MCP tool shapes, dispatch patterns, sharp edges) and the `harness:harness-driver` skill.

## Tests

- `tests/cli.rs` — black-box CLI tests via `Command::new(env!("CARGO_BIN_EXE_rmap"))`. Each test gets a unique temp dir from a per-test atomic counter. Date-sensitive tests set `RMAP_TODAY` on the `Command` env.
- `tests/skills_smoke.rs` + `tests/skills_fixture/` — parses every fenced ```bash``` block in `SKILLS.md`, extracts the optional `# exit: <N>` annotation (default 0), and runs each `rmap ` invocation in a fresh fixture copy with `RMAP_TODAY=2026-05-12` pinned. Agent-contract gate for `SKILLS.md` — renaming or removing a documented command requires updating both `SKILLS.md` and the fixture in the same commit.
- `tests/golden/<case>/` — fixture triples: `tasks.toml`, `ROADMAP.input.md`, `ROADMAP.md` (expected output). Optional `today.txt` pins the render date. Add `today.txt` to any fixture using `scored_at` to avoid drift into score-decay.
- `tests/roundtrip.rs` — parse → `toml_edit` round-trip → assert no spurious diff. Catches comment-preservation regressions.

## Scope discipline

`ROADMAP.md` (rendered from `roadmap/tasks.toml`) tracks open phases; `DESIGN.md` carries the design contract and out-of-scope list; `CHANGELOG.md` is the shipped-phase record. Implemented today: validate, render (incl. `--html` static single-project view and `--html --multi` portfolio view), watch, export json, export dot (Graphviz), waves, status (single + bulk), mark, depend, new (interactive + `--from-stdin`), next, ready, show, list, specs, blocks, deps, bundles, milestones, schema, diff, delegate, import, stale, doctor, critical-path. Score-decay rendering is automatic on tasks with `scored_at` >30d or missing. Deliberately out of scope (per `DESIGN.md`): Linear API calls, web server, git integration beyond `git show <ref>:<path>` for `rmap diff`, shell completions, CI workflow, multi-user sync. Don't add these without checking the roadmap first.

**Spec-layer invariants and mirrors**

- `[specs.<capability>]` stores only `path` and typed `draft | active | retired`
  status. Read Markdown relative to the resolved project root; missing paths,
  duplicate rule ids and prefixes shared across specs fail validation.
- Rule ids start a line at column one: uppercase ASCII letters, `-`, decimal
  digits, then whitespace or `:`. Current rules establish prefix ownership.
  Empty specs establish no prefix; rule text is never duplicated in TOML.
- `spec_changes` is a creation-time registry field of `{ rule, op }` entries;
  `op` is typed `add | change | remove`. Live change/remove require current text;
  live add requires a registered prefix and may name an existing rule. Terminal
  references are exempt. All mutators validate before writing.
- Mirrors: registry → stdin/writer/diff/task JSON; top-level specs → schema,
  metadata diff and exports; `specs.rs` → validation, listing/history and delegate
  quotations; human `show` → `query.rs`; `list --rule` → CLI selection. Empty
  deltas/registrations preserve legacy exports, with schema version 2 unchanged.
- `rmap specs [--json]` derives rule history from task deltas. Absent rules retain
  history under a still-established prefix; `list --rule ID` also finds orphaned
  historical references. `delegate` explicitly reports absent current text.
  Format and command examples live in SKILLS.md; tests/specs.rs covers distinct
  Rust and JavaScript project layouts.
