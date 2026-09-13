<!-- Auto-generated from CLAUDE.md by claude-marketplace/scripts/sync-agents-md.sh — do not edit manually -->

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

`rmap` is a single-binary Rust CLI that manages portable roadmap data (`roadmap/tasks.toml`) for any project, regardless of language. It renders `ROADMAP.md` and `roadmap/data.json` from the TOML source. Since 2026-05-13 rmap drives its own roadmap from `roadmap/tasks.toml` → `ROADMAP.md`; see `DESIGN.md` for the design contract and `AGENTS.md` for additional contributor guidelines.

## Imports

Eager floor per `~/.claude/setup-guide.md` § "Selective-Load Philosophy" (Opus 4.8). Two eager includes only:

- **`critical-rules`** — the portfolio-wide hard-guardrail floor; must stay ambient (a guardrail the model invokes "when relevant" fails exactly when it doesn't realize the rule applies).
- **`harness-workflow`** — the new default second eager include for harness-registered repos, and rmap IS one (registered in harness's `config/dev.local.exs`, MCP wired via `.mcp.json` — see § "Driving harness from this repo"). The implement→review→land loop and its delegation roster (cursor/codex/grok first, opus last) are load-bearing every session, not on-demand reference.

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

## Raise coverage before mutating

Before any code-changing task on an existing module, its `mix test.json --cover` must be at tier — **≥80%** standard, **≥95%** critical (money, signing, crypto, low-level encoders, security-sensitive parsers; when in doubt, critical). Below tier → write the missing tests first, in this task.

1. `mix test.json --cover --quiet --output /tmp/cov.json`
2. `jq '.coverage.modules[] | select(.module == "MyApp.Foo")' /tmp/cov.json`
3. Below tier → cover the uncovered lines, even ones you didn't come to change. Then mutate.

Exempt: doc-only edits, formatting/alias reordering, pure renames, typo fixes in strings/messages.

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
- Don't re-run a check the hook just ran on the same files. Full-suite re-runs earn their cost only before a PR/merge, after `mix deps.get`, after a branch switch, or on request.

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

<!-- @-import: ~/.claude/includes/harness-workflow.md -->
## Harness Workflow

OTP-native **implement → review → land** loop for roadmap-driven development. An AI orchestrator drives harness; harness dispatches headless implementer agents into isolated git worktrees, then a **cross-family reviewer AI** gates every deliverable (runs the project's checks itself, fixes inline, writes `.harness/review.json`). Optional auto-landing ff-merges approved work; a post-merge audit agent sweeps hygiene.

**Promoted from** `docs/dogfooding-workflow.md` in the harness repo — that file remains the **incubator runbook** for harness-specific history, driver-script templates, and per-batch run logs. This include is the **portfolio-wide contract**. Version-controlled source: `priv/includes/harness-workflow.md` in the harness repo; install to `~/.claude/includes/harness-workflow.md` via `mix harness.install_includes`.

### Relationship to Other Includes (Layered — No Supersession)

| Include | Role relative to harness-workflow |
|---|---|
| `workflow-philosophy.md` | **Foundation.** Evaluator separation, session-per-phase, verification-before-completion. Harness automates the loop while preserving these principles — the **reviewer AI** is the grader, never the implementer's self-report. |
| `task-prioritization.md` | **Task selection.** D/B/U scoring, `rmap next`, parallel markers, refine-don't-duplicate. Harness executes whatever rmap returns; it does not replace prioritization. |
| `worktree-workflow.md` | **Manual parallel sessions.** For hand-build work outside harness dispatch — operator-created worktrees, PR flow, post-merge audit. Harness manages its own per-run worktrees (`harness/<run-id>`); manual worktree rules still apply for hand-build sessions. |
| `dev-lifecycle.md` | **Manual five-phase chain** (`task-driver → worktree → bots → merge → audit-review`). Use when *not* driving through harness. Harness is the automated alternative for dispatchable roadmap tasks; dev-lifecycle still governs plan-and-file, pre-commit review, and post-merge audit. |
| `agent-dispatch.md` / cloud-delegation stack | **Linear/Codex/Cursor PR delegation** without a running harness BEAM. Orthogonal path — projects can use cloud delegation *or* harness; harness subsumes the dispatch+review loop when the OTP node is running. |
| `skills/harness-driver/SKILL.md` (harness repo) | **API surface contract** — MCP tools, `project_eval` patterns, `%LogRecord{}` fields, sharp edges. Load on demand when driving harness; this include covers *workflow*, the skill covers *surfaces*. |

**Adopt per repo:** `@~/.claude/includes/harness-workflow.md` in the project's `CLAUDE.md` (load-on-demand row — not eager; same pattern as `workflow-philosophy.md`).

### The Loop

```
rmap task → implementer AI (worktree) → commit harness/<run-id> → reviewer AI (THE GATE) → done | failed
                                                                              ↓ (done + auto policy)
                                                              MERGE (lander: rebase + ff-push, no re-verify)
                                                                              ↓
                                                              AUDIT (post-merge audit agent, best-effort)
```

One run = one supervised `Harness.Run` gen_statem: fork worktree off target `HEAD`, dispatch implementer, commit diff to `harness/<run-id>`, dispatch cross-family reviewer into the same worktree. The reviewer runs the project's `check_command` hint, fixes what it can, writes `.harness/review.json`. **Success = reviewer `approve`** — never implementer exit code or self-report. There is **no mechanical verification gate** in harness; judgment lives in agents.

Rejections put the task back in the queue for re-dispatch. Fix-and-approve is the near-absolute default for the reviewer.

**🚨 "Cross-family" is routing doctrine, not a mechanical guarantee.** Harness excludes only the *identical* agent from the reviewer slate (`Harness.Agents.reviewers/1` → `reject_implementer/2`); there is **no family concept in harness code**, so a `cursor` implementer can draw a `grok` reviewer even though both run SpaceXAI weights. The orchestrator owns the separation when it matters. This is deliberate, not an oversight: measured 2026-08-23 over 1,627 harness reviews, controlling for reviewer identity leaves no per-pair signal — review intervention is a **per-reviewer** trait (median `reviewer_diff_size`: Codex 96, Cursor 4, Claude 1, Grok 0), and the most capable reviewer in the ledger finds median 0 in the same work a heavier reviewer rewrites. Don't add a family scheduler to make the code match the older wording.

### When to Dispatch vs Hand-Build

**An rmap task is not automatically a harness run.** Dispatch only when the full
implement→review→land cycle buys meaningful safety, independent verification, or
parallel throughput. Historical run cost stays material even for D≤2 work, so the
old D≤2 / 30-LOC conjunctive exception was too narrow.

**Work inline by default when it is bounded and local:** one coherent surface,
typically D≤4, roughly ≤100 LOC across ≤5 files, focused-testable, and no positive
dispatch trigger below. These are routing hints, not an ALL-of gate — a risky D2
task can earn dispatch, while a routine D4 task can stay inline.

Positive dispatch triggers:

- Signing, money handling, cryptography, security, or authorization
- A public API/schema/contract change or a migration
- Harness runtime, CI/check infrastructure, or a repo-wide invariant
- Live/external-system semantics that need independent evidence
- Multiple subsystems, or genuinely useful parallel execution

Hand-build when harness cannot perform or judge the work:

- Scaffolding that reshapes harness runtime (supervision tree, dep stack, Endpoint) **while the run lifecycle itself is in flux**
- Work requiring live human/browser judgment, such as exploratory visual identity; routine spec-anchored UI remains dispatchable
- A harness gap — file via `rmap new`, fix harness, re-dispatch; do not work around the gap inside the target task

**🚨 The routing gate fires at `assignee =`, not at dispatch time.** rmap requires `assignee` + `model` at task creation, so the inline-vs-dispatch decision is made — and frozen — the moment the task is filed: a task carrying an agent assignee reads as "routing already decided" to every later session, and this section never gets consulted again. Three rules close that hole:

- **Filing a task: run this section BEFORE typing `assignee` — and a FILED task defaults to an agent.** The inline-vs-dispatch question above governs work you can execute *now*: inline-doable work is done inline and never filed. A task that reaches filing is cross-session by definition, so default-route it to a dispatch agent with a pinned `model` (roster spread per § "Delegation roster"); `assignee = "human"` must be earned by a hand-build reason named in the body — an operator-gated step (license, credential, purchase), no-spec visual identity, harness-loop-in-flux, or the user claiming the work. (Flipped 2026-08-13 from the old default-`human` rule after trading_dashboard tasks 86/87 — both dispatchable — were filed `human` by reflex. The ccxt_client-470 lesson survives with its real moral: a D2 one-file fix should be *done inline*, not filed at all — the filing was the defect, not the assignee.) Mirrored as question 6 of `task-writing.md`'s Pre-Creation Gate.
- **Reviewer `proposed_tasks` carry no routing authority.** Proposals arrive dispatch-shaped (suggested scores/markers), but the orchestrator owns routing the same way it owns filing — re-route each proposal through this gate instead of inheriting dispatchability from its shape. Sibling of task-writing's "Re-Generalize an Agent's Decomposition": that filters whose *architecture* a task encodes; this filters whose *routing* it encodes.
- **🚨 Under `dispatch_mode: "auto"` there is no such thing as an open decision in a task body — decide it at filing or don't file the task `pending`.** `task-writing.md`'s gate 6 permits a `pending` task to carry named open decisions because "the orchestrator asks before it dispatches." That sentence assumes a human-driven orchestrator seat between the queue and the run. **A cron poller is not that seat**: in `:auto` mode it dispatches the ready set unattended on its schedule, reads no bodies, and asks no one — so the decision reaches an implementer as a question addressed to nobody, and the implementer answers it silently. Before filing into an auto-dispatching project, check `autonomy-status` (per-project `dispatch_mode` + `effective`) and `project_registry-lookup`, then:
  - **Decide it yourself and write the decision in, vetoable.** Name the choice, the reasoning, and the alternatives you rejected. `critical-rules.md` § SURFACE THE OVERRIDE is satisfied by a decision the operator can read and reverse; it does not require a blocking question.
  - **When one premise could genuinely flip the answer, ship the decision with an evidence gate** — "ship (a) unless a live probe disproves X, in which case (b); say in the delivery which you found." That is a decision the implementer can execute, not a question it must route back.
  - **`blocked` is still not the escape hatch.** It hides the task from the queue, so the decision is never surfaced at all — the same failure with a quieter shape. Reserve it for an external blocker with an unblock path.
  - Under `:manual` cron mode the parked-decision drain (`dispatch-pending` / `dispatch-approve`) *does* restore the asking seat, and gate 6 reads as written. Know which mode the project is in before relying on it. (Observed 2026-08-28 on bourse: two tasks filed `pending` with "open decision the operator owns, answer it before building" into a project on `dispatch_mode: auto` at `0 * * * *` — the next poll was 24 minutes out and would have dispatched both.)

### Running a Task

**Prerequisites:** long-lived harness BEAM (`iex -S mix` in the harness checkout), target project registered in `Harness.ProjectRegistry`, clean `git status` on the target's dispatch branch (runs fork worktrees off `HEAD`). **🚨 The roadmap side does NOT self-sync.** The run's *code* base is fresh (Task 196: with a `target_branch` set, `Run.Actions.Worktree.worktree_opts/1` fetches and forks off `origin/<target>`), but `dispatch-task` / `dispatch-bundle` **ingest `tasks.toml` via `rmap` from the on-disk checkout at `project.roadmap_path`** — no fetch, no pull. When the harness node's checkout lives on another host (e.g. `/data/postgresql/code/<project>`), a task you just filed and pushed from your Mac does not exist there until that checkout is pulled: `git -C <roadmap_path> pull --ff-only` on the node **before** dispatching (via `project_eval` or a shell there). The `roadmap: task <id> -> in_progress` commits harness pushes are also made in that checkout, so a stale one produces a non-ff push. (Observed 2026-09-11 on mpp: the operator pulled by hand before the wave; the orchestrator had not.)

**Three dispatch paths** (prefer top to bottom):

1. **Native MCP — default.** `dispatch-task` (fire-and-forget) against `http://localhost:4018/harness/mcp`; wait for the wave by watching `origin/<target>` for the lander's commits, never by blocking on `dispatch-await` / `dispatch-await_runs` (§ "Never block on `dispatch-await*`"). Observe via `dispatch-status`, `dispatch-transcript`, `dispatch-verdict_detail`. `scrub_anthropic_key: true` (default) forces subscription OAuth over inherited `ANTHROPIC_API_KEY`.
2. **Tidewave `project_eval` — escape hatch.** Struct-level control the flat tools don't expose (`retry_policy`, fail-over adapter lists, `subscriber: self()`). Run persists to `Harness.ResultStore` even when the eval process exits.
3. **`mix run` driver script — fallback.** Full transcript + reviewer report to terminal. See harness repo `docs/dogfooding-workflow.md` for the canonical template.

> **Never start a second driver BEAM while runs are in flight.** Boot-time worktree sweeps can prune live sibling worktrees. Drive all parallel batches from one long-lived node.

**In-flight idempotency (Task 286):** a second `dispatch-task` / `dispatch-bundle` of the same `{project, task_id}` while a non-terminal run exists returns the **existing** `run_id` (Oban `conflict?: true`), not a duplicate — a retried dispatch is safe and free.

**Coalesce small related tasks:** `dispatch-coalesce` accepts an explicit task-id list and runs it as one worktree, implementer invocation, reviewer gate, and landing unit. Use it when small tasks share a bundle/surface and separating them would only repeat fixed run costs; keep independent tasks in `dispatch-bundle` so write-disjoint work still parallelizes. Coalesced members share the same landing SHA and never partially land — the reviewer must mark every member `approved` in the verdict's `task_outcomes` or the run fails as a unit. The call returns the coalesced `write_set` (the union of every member's `touches`/`files_to_modify`); serialize the next wave against that union, since harness executes the coalesce but never picks what to coalesce.

**Write-set serialization (Task 292):** `dispatch-bundle` and cron ready-set dispatch compute each task's `touches ∪ files_to_modify` before enqueue. Tasks with overlapping write-sets are logged and serialized into later waves instead of fanned out together. Callers no longer hand-dedupe ready sets; they must keep `touches` / `files_to_modify` accurate because harness does not infer paths from task prose.

**Renderable vs executable:** `rmap delegate --to` renders native prompts for all six harness adapters (`claude`, `codex`, `cursor`, `grok`, `antigravity`, `pi`). `droid` renders but has no harness adapter — rejected at ingest. All six shipped adapters declare `worktree_isolation: true`.

### Routing & Model Management

- **Resolve `assignee` + `model` from facts, not by reading code.** `routing-brief` is the thin task-writer index: dispatchable agent roster, each agent's standing model (`Config.agent_model/1`), model availability/blocks, and per-agent KPI rollups — every metric carries `n`, no ranking. A model-capable agent with no configured model shows `model: nil, model_required: true`.
- **Scout routing (advisory).** `dispatch-recommend` returns the cross-family scout AI's per-facet `:exploit` pick (with rationale) or a safe `:explore` / `:fallback_no_data` when a facet is unmeasured; `dispatch-assess_facets` forces a fresh scout assessment. The caller decides whether to dispatch the pick — legacy composite scores are not used for routing.
- **Model is required, never defaulted.** Implementer precedence: **task `model` → `{:agent_model, agent}` → REJECT** (`{:model_required, agent}`) — harness never falls through to the CLI's ambient default. The **reviewer has no task-pin axis**: its model comes solely from `{:agent_model, agent}` for the reviewer adapter's agent (`Run.reviewer_model/1`), and a model-capable reviewer with no configured model is rejected *before* the reviewer spawns. Antigravity is model-capable as of `agy` 1.0.10 (`--model` + `agy models`); harness validates pins against its catalog because the CLI silently falls back on unknown ids.
- **Block exhausted premium models.** A monthly budget can exhaust (e.g. cursor-Opus) while harness still lists the pair as available and routes to it. `model_availability-block_model` (with a `blocked_until` window) removes the pair from routing/cron; `model_availability-unblock_model` clears it.
- **Cost-aware A/B.** `dispatch-compare` runs one task across N adapters (optional per-adapter model overrides) and returns per-adapter `verdict` / `reviewer_diff_size` / `duration_ms` / `token_usage` for selection.

### Reading the Verdict

| `state` / `reason` | Meaning | Action |
|---|---|---|
| `:done` / `:approved` | Reviewer AI approved (possibly after inline fixes — check `reviewer_diff_size`). | Deliverable on `harness/<run-id>`. Review diff, integrate (or let auto-lander handle it), `rmap status <id> done`. |
| `:failed` / `{:review_rejected, report}` | Reviewer rejected (degenerate — near-never by design). | Read `report`. Task back in queue; re-dispatch. |
| `:failed` / `{:review_stuck, report}` | No verdict: reviewer unavailable, crashed, or missing/malformed `.harness/review.json`. | Read `report`. Fix environment or re-dispatch. |
| `:failed` / `{:worktree_failed,_}` `{:agent_spawn_failed,_}` `{:driver_crashed,_}` `{:commit_failed,_}` | Harness-side mechanical failure. | **Harness bug.** File via `rmap new`. |
| `:failed` / `{:checkout_polluted, status}` | Agent wrote outside the run worktree into the main checkout — surfaces as `:failed` **only after bounded AI recovery was exhausted** (see "Self-healing recovery" below). | Recovery declared the run dead. Likely an agent/adapter isolation issue; re-dispatch with a worktree-honoring adapter. |
| `:failed` / `{:checkout_pollution_check_failed, _}` | Post-run pollution `git status` errored. | Rare; transient git/IO. Re-run; inspect checkout if persistent. |
| `:failed` / `:timed_out` | Lifetime budget elapsed. | Raise `:lifetime_timeout` or investigate hang. |
| run process **crashed** (no settle) | gen_statem died. | **Harness bug.** File via `rmap new`. |

Failed runs retain the worktree at `result.worktree_path` for inspection. Approved runs keep branch `harness/<run-id>` after worktree teardown. Use `dispatch-verdict_detail` for the reviewer report, ratings, checks, concerns, proposed tasks, warning flag, and `reviewer_diff_size` — no harness-run mechanical per-check stdout.

**The verdict artifact** `.harness/review.json` is `{verdict, run_id, review_attempt, report, checks, concerns, proposed_tasks, facets, skills, ratings}`: `verdict` (`approve`/`reject`) is the gate; `run_id` and `review_attempt` fence the file to the reviewer invocation that wrote it (echoed from `HARNESS_RUN_ID` / `HARNESS_REVIEW_ATTEMPT`; a mismatch is treated as missing); `report` is the reviewer's prose; `checks` is the reviewer-written record of commands run and their pass/fail claim; `concerns` is the reviewer's self-flagged caveat list; `proposed_tasks` is an optional list of structured discovery proposals (`title`, `body`, suggested scores/markers, and evidence); **`facets`** (open-vocabulary routing KEY — the kind of task) and **`skills`** (v0_13 two-axis rubric, routing VALUE) feed per-facet capability routing; `ratings` is the legacy flat-score fallback. Harness persists proposals verbatim but never files them. After a run lands, the orchestrator reads them from `dispatch-verdict_detail`, dedupes/merges them against the live pending set, and files only warranted tasks through its own task-writing gate. Reviewers never edit `roadmap/tasks.toml`, `roadmap/data.json`, `ROADMAP.md`, or `CHANGELOG.md`; those files are excluded from delivery commits alongside `.harness/`. Approved runs with non-empty concerns or a reviewer-authored failed check surface a warning fact; harness never auto-blocks or classifies prose. The artifact lives under `.harness/` (excluded from staging) so it never rides in the deliverable commit. The file is removed before every reviewer spawn so a killed reviewer's stale approve cannot settle the run.

**External-system evidence is reviewer-owned judgment.** When acceptance criteria touch an API or external service, the reviewer must look for reality rather than plausibility: a live success call, a relevant live error, the provider's official docs/spec/SDK for semantic meaning, and an integration test pinning the observed domain semantics. Third-party clients, aggregators, wrappers, and reference implementations (including CCXT) are compatibility/reference evidence only; they never establish correctness or override the provider-owned contract. Mocks, fixtures, and the implementer's self-report are not independent evidence. Missing credentials or an unreachable sandbox are surfaced as a failed check/concern (or rejection when the criterion cannot be verified), never silently treated as green. The lander records the reviewer identity plus `harness-run:<run-id>` as rmap verification provenance.

**Self-healing recovery (the `:recovering` state).** Before settling `:failed` for an *interpretive* non-rejection failure — checkout pollution is currently the one wired call-site — the run spawns a **bounded cross-family recovery AI** (`:recovering` state, budget 1/run) with minimal context (the error term + the main checkout's `git status` + the implementer transcript tail + the failing-check output, never the full transcript). It writes `.harness/recovery.json` `{outcome: "repaired"|"dead", report, repaired}`; harness reads it mechanically and **decides nothing itself**: `repaired` resumes at `:committing` and **re-runs the reviewer gate** (never skips to `:done`); `dead` / missing / malformed settles `:failed` with the original reason. A genuine `verdict: reject` is never routed through recovery. The `Result` carries `recovery_attempts` / `recovery_outcome` / `recovery_repaired` / `recovery_token_usage`. (Tier-1 mechanical self-heal precedes it: the reviewer is re-prompted once on a missing/malformed `review.json` — `reviewer_reprompt_count`, capped at 1 — and rotates to the next cross-family candidate on a reviewer timeout — `reviewer_rotation_count`.)

### 🚨 Recover, Don't Redo — Never Burn Tokens Re-Implementing Committed Work

**A run that committed to `harness/<run-id>` already paid for the implementer. Recovering that branch costs a fraction of a fresh dispatch — re-dispatching from `pending` throws the work away and makes the agent redo all of it.** The reflex to "reset → pending → dispatch again" is a token bonfire whenever a retained branch with commits exists. Check for the branch *first*; pick the cheapest primitive that fits:

| Run state — committed `harness/<run-id>` branch exists | Recover with | Agent tokens |
|---|---|---|
| Approved but unlanded (land-cap, lander crash) | `dispatch-reland` | **zero** — pure git rebase + push |
| Committed, review-stage failure (work is good) | `dispatch-rereview` | zero implementer — re-enters at the reviewer gate |
| Committed, implement-stage incomplete/`:failed` | `dispatch-resume_failed` (`escalate: true` to re-route agent) | **re-spends implementer tokens** — a fresh implementer invocation branched off the retained commits with the failure report injected (contrast `rereview`, which re-runs only the reviewer) |
| Live `:held` run (paused, not dead) | `dispatch-resume` | none — un-pauses in place |
| **No commits / no retained branch** | reset → `pending` + fresh `dispatch-task` | full redo — **the only case where this is correct** |

**Live-run intervention (not recovery of a dead run):** `dispatch-hold` (optionally `interrupt: true`) parks a live run mid-turn, `dispatch-steer` stashes guidance applied on resume, `dispatch-resume` un-pauses in place, `dispatch-cancel` kills it (idempotent). Use hold → steer → resume to force-hand a grinding implementer to the reviewer gate instead of burning the lifetime budget.

**The gate before any reset-to-pending + re-dispatch:** `git branch -a | grep harness/<run-id>` and `git log --oneline origin/<target>..harness/<run-id>`. Commits present ⇒ recover, never redo.

**🚨 First, confirm the run actually *didn't* land — check `origin`, not your local checkout.** Under `landing_policy: :auto` the lander pushes to `origin/<target>` from a detached worktree, then `Harness.Git.TargetSync` may fast-forward the operator's local target when that is safe (off-target → ff the branch ref; on-target + clean tree → `merge --ff-only`). It skips — witnessed, never `--force` — when the tree is dirty, the update is not a fast-forward, or the target is this running node's own source tree (self-host: path identity, not the project name). Under dogfooding that self-host skip is the common case, so after an autonomous land your local `tasks.toml` is **stale**: it still reads `in_progress` for a task the lander already marked `done --shipped-in` on origin. **Reading that stale local status as "the run didn't land" is the trap** — it triggers a wasteful reset-to-`pending` + re-dispatch that *duplicate-lands already-shipped work*. Before concluding anything from task status, `git fetch origin <target> && git rebase origin/<target>` (the existing "Sync main before committing" rule) or read ground truth directly:
- `git log --oneline origin/<target>` — does it already show `task <id> -> done (shipped …)` and the agent-delivery commit? Then it **landed**; your local view was just behind. Do nothing but rebase.
- `dispatch-status <run-id>` / `result_store-list_run_records run_id:<id>` — a record with `state: done, verdict: approve` means the run succeeded; cross-check landing against origin before touching the roadmap.

> **Observed 2026-06-12 (the cautionary tale this section exists for):** three approved runs (246/249/251) landed cleanly to `origin/development` — `done --shipped-in`, audited. But the operator's local checkout hadn't rebased, so `rmap show` read stale `in_progress`. That was misread as "approved but didn't land," the tasks were reset to `pending` and re-dispatched, and task 246 **landed a second time** (duplicate delivery) before the mistake surfaced. Root cause: reading stale local state instead of rebasing on `origin` first. The lander was working perfectly the whole time.

The recovery primitives (`reland`/`rereview`/`resume_failed`) read the persisted `ResultStore` record, which **survives** worktree teardown and node restarts — so a genuinely approved-but-unlanded run (lander hit its land-cap, or a real rebase conflict retained the branch) is recoverable token-free via `dispatch-reland`. Reserve reset-to-`pending` for runs with **no committed branch and no settled record** — and only after confirming against `origin` that the work isn't already shipped.

### Parallel Dispatch

`Harness.Run.Supervisor` is a `DynamicSupervisor` — N crash-isolated runs, each with its own worktree.

- **Batch by dependency graph, then write-set.** Every pending task whose `depends_on` is satisfied can enter the ready set, but harness dispatches only the first wave whose `touches ∪ files_to_modify` are disjoint. Overlapping tasks wait for a later wave after the landed base moves forward.
- **Keep write-set fields accurate.** The dispatcher counts declared path intersections; it does not infer paths from the task body. If two tasks really edit the same function, either let write-set serialization sequence them or fold the coupled work into one rmap task (`task-prioritization.md` § "Refine, Don't Duplicate").
- **One driver BEAM** for all concurrent runs in a wave.
- **Integration order (manual landing):** smallest/isolated diffs onto target first; rebase siblings; run the project's check command on target after last merge.
- **While a wave is in flight:** do not run `rmap status` / `rmap mark` / `rmap new` in parallel sessions against the same checkout — triggers `:checkout_polluted` false-positive.
- **Repo-wide invariant tasks run EXCLUSIVE.** A task whose real write-set is "the whole surface" — introduce a repo-wide guard/invariant and convert every violating site (e.g. an AST-scan test over all of `test/`) — cannot be write-set-serialized by declared `touches`: any sibling land that adds a new violating site after the fork reddens the guard at landing time (observed ccxt_client task 433 × 435, 2026-07-19). Dispatch such tasks as a solo wave — nothing lands in parallel — or accept that the orchestrator repairs at landing.
- **Land-conflict repair is a standard orchestrator move, not an incident.** When the lander blocks on a rebase conflict (reason retains the branch): fork a repair worktree off `origin/<target>`, cherry-pick the run commits, resolve (for additive `tasks.toml` collisions: renumber the branch-side new task to the next free id on origin **and rewrite in-diff string references to it** — CHANGELOG lines, code comments; then `rmap validate && rmap render`), point the retained `harness/<run-id>` branch at the repaired tip, and `dispatch-reland` — the lander keeps push authority and advances rmap itself. **Do not re-run gates on a roadmap/doc-only repair:** the reviewer already graded the code; renumbering tasks, merging doc entries, and re-rendering the roadmap change nothing the gates measure, and a clean disjoint auto-merge of verified code needs no re-grade (same token-economy rule as everywhere else). Re-run a check ONLY when the repair touched code, or when the conflict overlapped a repo-wide invariant the sibling lands could have violated (e.g. a new suite-wide guard vs tests added after the fork — run just that guard, not the stack). Never reset-to-pending (that redoes paid work), never hand-push to the target when a reland can land it.

### Autonomous Landing

Projects with `landing_policy: :auto` and `target_branch`:

1. Approved run enqueues one job on serialized `landing_<name>` Oban queue (limit 1)
2. `Harness.Lander.land/1` rebases `harness/<run-id>` onto `origin/<target>` in a detached worktree
3. **ff-pushes without re-verification** — the reviewer already gated the work
4. Successful push enqueues post-merge audit; advances rmap (`done --verified --verified-by <reviewer> --verification-ref harness-run:<run-id> --shipped-in <sha>`)

Conflict / push-rejected retains the branch for repair — never lands red. Witness notification (read-only sink) alerts the operator; it is **not** a merge gate.

**🚨 Never block on `dispatch-await*` — monitor `origin` for the landing commit instead.**
This is the standing rule for waiting on a wave, not a fallback. `dispatch-await` /
`dispatch-await_runs` hold an MCP request open for the entire run, and an MCP client
kills a tool call that emits no progress for its idle timeout (Claude Code's default is
300s — far shorter than any real run). The call dies, the orchestrator learns nothing,
and the runs keep going regardless. Worse, awaiting the wrong signal: **await returns at
reviewer settle, which fires BEFORE the serialized `landing_<name>` job rebases and
ff-pushes** — so even a successful `approve` means "approved and *queued* to land," never
"on `origin/<target>`."

**The primitive that actually works — watch the target branch for the lander's own
commits.** The lander pushes `task <id> -> done (shipped <sha>)` to `origin/<target>`;
that commit IS the landed signal, it is durable, and it survives a dead MCP call, a
restarted session, and a node bounce. Arm one background watcher per wave and keep
working:

```bash
# one notification per landed task, exits when the whole wave is in
cd <source-checkout>
WAVE="615 623 569 619"; seen=""; BASE=$(git rev-parse origin/<target>)
DEADLINE=$(($(date +%s) + 10800))  # bound the wait; tune to the wave's slowest run
while true; do
  git fetch -q origin <target> || true
  for t in $WAVE; do
    case " $seen " in *" $t "*) continue;; esac
    if git log --oneline "$BASE"..origin/<target> | grep -q "task $t -> done"; then
      echo "LANDED task $t"; seen="$seen $t"
    fi
  done
  [ "$(echo $seen | wc -w)" -eq "$(echo $WAVE | wc -w)" ] && { echo "WAVE COMPLETE"; break; }
  [ "$(date +%s)" -gt "$DEADLINE" ] && {
    echo "DEADLINE EXCEEDED — wave incomplete"
    git log --oneline "$BASE"..origin/<target> | grep "task.*-> done" | sed 's/.*task \([0-9]*\).*/  landed: \1/' || echo "  (no tasks landed in range)"
    for t in $WAVE; do
      case " $seen " in *" $t "*) continue;; esac
      echo "  missing: $t"
    done
    break
  }
  sleep 60
done
```

🚨 **The baseline is load-bearing — grep the range, never the whole log.** A task that
landed before already carries `roadmap: task <id> -> done (shipped …)` in the history, and a
task that was reset and re-dispatched carries one per attempt. Without `BASE`, the watcher
matches those stale commits on its first iteration and reports `LANDED` before the implementer
has written a line — the same false-green this whole section exists to prevent, wearing the
costume of the fix. Observed 2026-08-29 on bourse task 687, which had landed and been reset four
times: the unbaselined watcher exited successfully within a second of arming.

The deadline branch is the other half. A run that fails review or blocks on a land conflict never
produces a landing commit, so a watcher with no bound waits forever on a wave that is already
dead; on expiry it must print what did land in the range and name what did not, so the missing
tasks get reconciled through `dispatch-status` instead of assumed.

Poll `dispatch-status <run-id>` only to diagnose a run that the watcher shows as *not*
landing — a `:failed` verdict, a rebase conflict that retained the branch, a hung
implementer. Status is for diagnosis; git is for waiting.

**Silence is not success** — a run that fails review or blocks on a land conflict never
produces a landing commit, so a watcher greping only for `-> done` stays quiet forever.
Bound every wave watch with a deadline, and when it expires without `WAVE COMPLETE`,
reconcile the missing tasks through `dispatch-status` / `result_store-list_run_records`
before assuming anything.

Same root cause as the duplicate-land trap above, seen from the dispatch side: **origin is
the source of truth for what landed** — not an await return value, not a local
`tasks.toml`, not a transcript.

**Herdr panes are an optional operator convenience for watching, never a harness
surface.** When the orchestrator session runs inside Herdr (`HERDR_ENV=1` — the
operator's default), the wave watcher above and ad-hoc run babysitting can run
*visibly*: `herdr pane split --current --no-focus` + `pane run` for the watcher
loop, an attach pane tailing `dispatch-transcript` for a run under scrutiny,
`herdr worktree open --path <retained-worktree>` to inspect a failed run, and
`herdr notification show "…" --sound done` as a configured witness-notification
sink. Strictly operator-side: dispatched agents stay headless over Ports, and
Herdr's `idle`/`blocked` classification is never a harness signal (adjudicated —
harness repo `docs/orchestration-library-evaluation.md`, Addendum 2026-08-25,
incl. the deliberately unmitigated `HERDR_*` env-inheritance risk for dispatched
agents).

**Cron manual-approval mode.** A per-project cron poller in `:auto` mode dispatches unattended; in `:manual` mode it **parks** each dispatch decision instead of enqueuing — drain the parked decisions with `dispatch-pending` and approve them with `dispatch-approve`, keeping the orchestrator in the loop for autonomous polling.

### Orchestrator Loop — the Architect Seat the Per-Task Reviewer Can't Fill

The sections above document the *mechanisms*; this is the **continuous loop** the driving AI runs across waves:

```
plan wave → dispatch → watch origin for the landing commits → run integration suite on the landed base
          ↑                                                     + review whole surface vs roadmap intent & domain invariants
          └── reconcile rmap ← encode any whole-surface finding as a criterion/test ←┘
```

Each arrow reuses an existing mechanism — don't restate them here: *watch origin for the landing commits* (§ "Never block on `dispatch-await*`", and § "Recover, Don't Redo" → the duplicate-land trap), *reconcile rmap* (the lander already advanced `done --shipped-in` under auto-land — verify, don't double-write), *next wave* (§ "Parallel Dispatch" + write-set serialization).

**🚨 Three review seats, each blind where the next sees — the orchestrator seat is mandatory, not optional.** The per-task reviewer gates *one diff against one task* and is **structurally blind** to two defect classes that land clean through it (worked evidence: delta_calc tasks 24/25/26, see its `## Review Blind Spots` / `## Domain Invariants`):

| Seat | What it sees | What it CANNOT see |
|---|---|---|
| **Per-task reviewer** (cross-family, the gate) | one diff vs one task's acceptance criteria + mechanical checks, in an isolated worktree off a base | the whole surface; domain ground truth |
| **Post-merge audit AI** (best-effort) | cold build of the merged commit range; hygiene | whether a domain constant is *wrong*; roadmap-intent fit |
| **Orchestrator** (the architect seat — you) | whole integrated surface vs roadmap intent + domain invariants across all landed waves | — (this is the seat of last resort) |

The two blind classes, both real-correctness, both passing every per-task check:

- **Domain ground truth** — a wrong venue constant (`@funding_periods_per_day 3`, overstating Deribit's hourly funding ~8×) is internally consistent and fully tested *because the golden was computed with the same wrong constant* — coverage ratifies the bug. The reviewer has no signal; that knowledge lives in the architect's head.
- **Cross-module global invariants** — write-set-disjoint parallel dispatch means two worktrees can each define `project_payback_timeline` and neither review sees the other; the collision only exists once both have landed on the integrated base. Only a whole-surface seat catches it.

**🚨 Run the integration suite on the landed base — this is NOT redundant with per-task review.** After each wave lands, run the project's full check (`mix ci` / `mix precommit.full`) on the freshly-landed `origin/<target>`. The per-task reviewer ran the dispatch-scale check hint (for Elixir, `mix check.dispatch` plus focused `mix test.json ...` for touched behavior) in an *isolated worktree off an earlier base, before sibling waves landed* — cross-module breakage doesn't exist until multiple landed diffs coexist. This generalizes the manual-landing-only "run the project's check command on target after last merge" (§ "Parallel Dispatch") into a standing per-wave step.

**Capture dispatch-check output once, to a unique tmp log.** Dispatch checks are normally verbose. The reviewer should capture the first run instead of re-running for readability: `LOG=$(mktemp -t harness-check-dispatch.XXXXXX.log)` then `mix check.dispatch > "$LOG" 2>&1`; inspect with `tail -200 "$LOG"` / `rg "error|failed|warning" "$LOG"` and record the log path in `.harness/review.json`. The random `mktemp` path prevents parallel agents from clobbering each other's logs.

**🚨 Architect/QA is a workflow responsibility, not a harness runtime gate.** After a wave lands, the orchestrator must run the full landed-base gate, review the integrated surface against roadmap intent/domain invariants, fix findings, and only then dispatch the next wave. Harness does not pause dispatches or store a completion marker for this step; this is the driving AI's seat.

**Two framing guards — keep this consistent with the harness mantra:**

- **It's an agent seat, not harness code.** The mantra ("count facts in code; judge with an AI") forbids *harness* computing meaning — it does **not** forbid the orchestrator AI from reviewing the whole surface or running the suite. This adds no mechanical gate to harness; it's judgment in an agent, which is exactly where judgment belongs.
- **The output crystallizes into encoded invariants — don't leave it a manual sweep.** When the architect seat catches a whole-surface or domain defect, the highest-value move is not the manual catch — it's pushing the rule into an **acceptance criterion or a manifest-wide CI test** (the delta_calc rule) so the per-task gate absorbs that class going forward. Orchestrator review *feeds* the criteria/CI; it must not become a permanent re-review of every diff. A finding caught twice by hand is a missing test.

**Convergence sweep (append-only).** The architect seat's whole-surface pass has a disciplined output shape (inspired by spec-kit's `/speckit.converge`, github/spec-kit): assess the landed code against the **roadmap + acceptance criteria as the sole source of intent** — never against the orchestrator's memory of what it dispatched or what a transcript claimed. Three rules:

- **Sole source of intent.** The gap being measured is code vs. `tasks.toml` ACs and roadmap/milestone intent. If the intent itself was wrong, that's a task edit first, then a sweep against the corrected intent.
- **Append, never rewrite.** Every unmet criterion, partial delivery, or intent gap becomes a **new `rmap new` task** (D/B/U-scored, gated per `task-writing.md`) referencing the task it converges on. Never reopen, rewrite, renumber, or edit the history of existing tasks to make the gap disappear — `attempts`/`implemented` records are evidence, not scratch space.
- **Clean sweep = zero mutations.** When the surface already satisfies the roadmap, the sweep leaves `tasks.toml` **byte-for-byte unchanged** — no empty "convergence" ceremony entries, no touched timestamps. A sweep that always writes something is measuring itself, not the code.

### Portfolio Conventions

- **Agent does not commit unless asked.** Staged-but-uncommitted is the default handoff between implementer and reviewer sessions (`workflow-philosophy.md` § "Implementer / Reviewer Handoff"). Harness runs commit agent work to `harness/<run-id>` automatically — that is harness's deliverable branch, not the operator's main checkout.
- **Reviewer discoveries arrive as proposals, and the ORCHESTRATOR files them post-land.** A reviewer that filed a discovery by editing `roadmap/tasks.toml` in its worktree assigned ids from a stale fork (id collisions that block the lander — observed ccxt_client 2026-07-19), couldn't see the live pending set (so the one-session=one-task merge gate never fired), and made roadmap files a universal write-set overlap across "disjoint" waves. That channel is closed: reviewers now emit `proposed_tasks` in `.harness/review.json`, and `roadmap/tasks.toml`, `roadmap/data.json`, `ROADMAP.md`, and `CHANGELOG.md` are excluded from delivery commits, so a run diff carries only code. After each land, read the proposals via `dispatch-verdict_detail` and file only the warranted ones through your own task-writing gate — dedupe against the live pending set, merge per `task-writing.md`, score with real ids off `origin`. Harness persists proposals verbatim and never files them.
  - **🚨 Default-DECLINE — the proposal pipeline outproduces the backlog's right to grow.** Reviewer + audit agents emit ~1 proposal per run; an orchestrator that files "everything evidenced and cross-session" lands N tasks and files N new ones per wave — net backlog delta ±0, the roadmap never converges (observed ccxt_client 2026-07-22: 11 landed, 11 filed in one session, including a D2 one-file fix filed+dispatched instead of done inline, a B4/U3 cosmetic filed instead of declined, and a follow-up that existed only because its parent was scoped as a patch instead of the invariant). Evidence + cross-session is the FLOOR, not the bar. File a proposal only when ALL THREE hold: (a) real defect or invariant gap with evidence, (b) not foldable into an existing pending task — and when the proposal patches an instance of a class, scope the filing as the CLASS invariant so the next instance can't spawn a sibling task, (c) not inline-doable in minutes by the orchestrator — if it is, DO it now instead of filing. Declined proposals need no ceremony: the verdict record in the ResultStore is their evidence trail.
  - **Report the net backlog delta** (landed − filed) as an explicit number in every wave/session wrap-up. A session trending ±0 or negative-growth is the churn alarm firing — tighten the decline bar, don't normalize it.
- **Witness notification is sakshi (read-only).** Landing outcomes notify via configured command sink; the sink grants no merge capability. Human operator reviews blocked/conflict outcomes — harness does not silently force-push past conflicts.
- **`check_command` is a dispatch-scale hint to the reviewer.** Free text (e.g. `"mix check.dispatch"` for Elixir, with focused tests chosen by the reviewer) — the reviewer runs and judges it; harness does not execute it mechanically. Keep full-suite commands like `mix precommit.full` for the landed-base Architect/QA pass. For verbose checks, capture to a per-run `mktemp` log on the first execution; never re-run only to recover truncated output.
- **The cross-family reviewer reads `AGENTS.md`, not your Claude skills/includes.** `AGENTS.md` is generated from `CLAUDE.md` by `claude-marketplace/scripts/sync-agents-md.sh`, which recursively inlines every `@`-import. **Regenerate it after any `CLAUDE.md` change** (`bash ~/_DATA/code/claude-marketplace/scripts/sync-agents-md.sh`, or `--dry-run` to preview) so the reviewer gates against current rules — a stale `AGENTS.md` makes codex/cursor/grok judge against rules you've already changed. **`--check` is the freshness gate** — it re-renders in memory and exits non-zero if `AGENTS.md` has drifted (diffs rendered output, not mtimes, so it catches drift in transitive `@`-imports too); wire it into CI / a pre-commit hook / the `check_command` so staleness fails loudly instead of silently. Consequence under Opus-4.8 skill-on-demand: once `CLAUDE.md` slims to the eager floor, reviewer-critical facts that *were* carried by eager includes (the `check_command` gate; that `mix test.json` / `mix dialyzer.json` emit JSON **by design** — parse for real failures, never flag the envelope; plain `mix dialyzer` is authoritative when the JSON encoder can't serialize a warning) no longer reach `AGENTS.md` via those imports. Put them in a **self-contained `## Toolchain & check commands` section in `CLAUDE.md`** so they survive the slim-down and flow into `AGENTS.md` on regen (ref: `tapakly/CLAUDE.md`, `ccxt_extract/CLAUDE.md`).
- **Delegation roster — opus last, and don't over-default to codex.** When assigning a dispatchable task to a harness adapter, prefer the external agents — **cursor, codex, grok** — and reserve the **claude/opus** adapter for work that genuinely needs it (harness-surface changes, judgment-heavy review, tasks the cheaper adapters keep bouncing). Opus tokens are precious: spend them last, not by default. Mix adapters across a wave for review coverage — but `cursor`+`grok` is one family, not two (see cursor bullet). A repo may override the roster in its own CLAUDE.md.
  - **Observed failure mode: reflex-routing everything to `codex`.** Run ledgers skew heavily codex-over-cursor/grok. Actively spread `assignee` across all three; reserve codex for tasks it's genuinely scored best on, not as the default.
  - **`cursor` is back on the roster (operator unblocked 2026-08-15).** SuperGrok Heavy entitles Cursor Ultra; the 2026-07-13 `cursor/all` block is lifted. Pin `model = "cursor-grok-4.6-high"` — **operator decision 2026-08-17: no more Composer pins.** The older `composer-2.5` guidance (cheapest cost-to-green, and where every cursor capability KPI was measured) is retired; that ledger data describes a model the operator no longer wants routed to. Confirm the live id with `cursor-agent --list-models` / `model_availability-list_available_models cursor` (the catalog also carries `cursor-grok-4.6-xhigh` / `-fast` variants and `claude-opus-5-*` — Opus/frontier pins through cursor still exhaust and get operator-blocked, so don't reach for them as the "design-heavy" reflex). **`cursor` and `grok` are the same SpaceXAI family** (SpaceX closed the Cursor acquisition 2026-08-14): three adapters, two families. A cursor implementer must not get a grok reviewer (and vice versa) — pair either with `codex`.
  - **`model` is REQUIRED at creation for any non-`human` assignee** (`rmap new` rejects a model-less dispatchable task — "a dispatchable task must pin the LLM it runs on"; see `rmap.md` § "Pinning an LLM model"); "leave `model` unset for the agent default" does NOT work. Set `assignee` **and** `model` at task creation per `rmap.md`.
  - **`grok` runs on `grok-4.6` — the frontier default since 2026-08-13; `grok-4.5` is gone from the live catalog** (lineage: `grok-build` → `grok-4.5` 2026-07 → `grok-4.6`; a catalog refresh on 2026-08-13 listed only `grok-4.6`). Re-pin any task still carrying `grok-4.5` when you touch it — a retired pin fails at dispatch. `grok-4.6` carries **no** capability/cost-to-green data yet — route to it to *gather* that data (A/B via `dispatch-compare` grok-4.6 vs codex/gpt-5.6-sol), not on a performance claim the ledger doesn't yet show. A newly-probed grok model lands in the catalog as `selected?: false`; select it (`model_availability` toggle) before it's dispatchable. Confirm live ids with `grok models` / `model_availability-list_available_models grok`.
  - **`codex` runs on `gpt-6-astra` — the standing default since 2026-09-06 (operator decision); `gpt-5.5` is RETIRED from the live catalog.** **`gpt-6-astra`** is OpenAI's flagship since 2026-09-03 (~$10/$50 per 1M tok ≈ 2.5× Sol, 1M context; in the catalog and selected, codex CLI ≥ 0.153) — both `agent_model.codex` and `reviewer_model.codex` are pinned to it. The GPT-5.6 family stays available as the cheaper tier: **Sol** = prior flagship ($5/$30 per 1M tok), **Terra** = balanced (~5.5-competitive at 2× cheaper, $2.50/$15), **Luna** = fast/cheap ($1/$6) — ids `gpt-5.6-sol`, `gpt-5.6-terra`, `gpt-5.6-luna`. **Pin `model = "gpt-6-astra"` for new codex tasks**, and re-pin any task still carrying `gpt-5.5` when you touch it — a retired pin fails at dispatch. `gpt-6-astra` carries **no** capability/cost-to-green data yet — watch the ledger as runs accrue; `terra` remains the cost-to-green candidate — A/B via `dispatch-compare` before routing bulk work to it. Confirm live ids with `codex debug models` / `model_availability-list_available_models codex`; a probe failure falls back to the builtin seed.
### Known Sharp Edges

- **Fresh worktrees lack `deps/` / `_build/`.** Implementer and reviewer each run project bootstrap (e.g. `mix deps.get`) when needed — budget timeouts for cold worktrees.
- **Reviewer runs the checks.** No mechanical check stack. Correct-but-not-pristine work → reviewer fixes and approves (`reviewer_diff_size` > 0).
- **Cold dialyzer PLT** dominates first reviewer check run in Elixir worktrees.
- **Nested Claude auth.** `ANTHROPIC_API_KEY` shadows subscription OAuth — scrub per run (`scrub_anthropic_key: true` or `env: %{"ANTHROPIC_API_KEY" => false}`).
- **Parallel-session rmap mutations** during a run can false-positive `:checkout_polluted` — wait for the wave or use a separate worktree.

### Repo-Specific Detail

| Need | Where |
|---|---|
| Harness API surfaces, MCP tool shapes | `skills/harness-driver/SKILL.md` in harness repo |
| Driver script template, cutover history, run log | `docs/dogfooding-workflow.md` in harness repo |
| Agent-gate architecture spec | `docs/agent-gate-workflow.md` in harness repo |
| Cross-checkout consumer setup | `skills/harness-driver/SKILL.md` § "Context A" |
| D/B/U scoring, task writing | `task-prioritization.md`, `task-writing.md` |
| Manual session/PR/audit chain | `dev-lifecycle.md`, `worktree-workflow.md` |

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
- `render_html.rs` — `rmap render --html` single-project view (`templates/roadmap.html.j2`) and `--html --multi` portfolio view (`templates/portfolio.html.j2`), sharing component macros (`templates/_components.html.j2`) and one stylesheet (`templates/_styles.css`); all minijinja, all `include_str!`'d. Builds DAGs via the longest-path layering in `topo.rs`. Portfolio inputs are `ProjectInput` envelopes (project root, `tasks.toml`, or `data.json` path — see `main.rs::load_project_input`).
- `topo.rs` — pure longest-path layering `compute_layers(tasks) -> {id → depth}` over the in-repo `depends_on` graph (extracted from `render_html`), plus `compute_layers_from_edges` over pre-extracted `(id, deps)` lists (the portfolio's JSON-loaded tasks). Shared by `render_html.rs` (DAG vertical slotting) and `export.rs` (the computed `dep_layer` field).
- `graph_export.rs` — read-only graph surfaces over the in-repo `depends_on` graph. `build_waves` / `format_waves` / `format_waves_json` (`rmap waves` — parallel dispatch schedule grouped by `topo` `dep_layer`, wave 0 = roots) and `build_dot` / `format_dot` (`rmap export dot` — Graphviz DOT digraph, edges dependency → dependent). Both pure; never mutate.
- `mutate.rs` — `status` / `mark` / `depend` / `new` paths. All use `toml_edit::DocumentMut` and end with `validate_tasks_str` before returning.
- `next.rs` — pure selectors: `next_tasks(tasks, filter, count)` (highest-Eff pending unblocked) and `ready_tasks(tasks, filter, count, dispatchable)` (the whole parallel-safe set), both ranked via shared `next::rank_tasks` — a 4-tier lexicographic key (focus phase × active milestone, **focus dominant**) then Eff descending. `is_unblocked` is the dep-satisfied predicate; the `active` milestone set is computed once per call from `tasks.milestones`.
- `query.rs` — `show` / `list` read paths. `TaskFilter` + `find_task` + `list_tasks` pure; `effective_target_repo` resolves an omitted task target to the roadmap's `project`; humans get `format_task*`, JSON delegates to `export.rs`.
- `bundles.rs` — `rmap bundles` read path. Per-bundle `next_task` reuses `next::next_task`.
- `next_bundle.rs` — `rmap next-bundle` pure selector. Broad actionability (in-bundle pending deps satisfy if themselves actionable). Topological emit via Kahn's.
- `delegate.rs` — `rmap delegate` prompt formatter. Read-only Markdown; never calls Linear/GitHub/Slack.
- `import.rs` — `rmap import` prompt formatter. Interpolates project name + live JSON Schema into `templates/import_prompt.md`.
- `diff.rs` — `rmap diff` engine. `diff_toml(base, current, verbose) -> TomlDiff`. Per-task field walk hand-maintained via `diff_fields!` macro.
- `schema_json.rs` — JSON Schema for `Tasks` via `schemars` derives.
- `scoring.rs` — shared `efficiency`, `format_efficiency`, `tier_glyph`, date helpers. Pure-integer Howard Hinnant date math; no chrono.
- `stale.rs` — pure `parse_duration` + `find_stale`. No I/O.
- `doctor.rs` — soft-signal aggregator. Always exits 0; strict gates remain on `validate`.
- `paths.rs` — ancestor walk to find `roadmap/tasks.toml`. CLI flag overrides; `html_path` is fixed-derived.
- `watch.rs` — pure helpers for `rmap watch`. `write_if_changed` (idempotency primitive), `is_tasks_toml_event` (filter), JSON event-line builders.
- `main.rs` — `clap` derive CLI. `run() -> Result<ExitCode>`. Wires every command to its module.

## Load-bearing invariants

Easy to violate without breaking tests immediately. The "why" lives in source doc comments and tests; this list is the index.

**Render & markers**
- **Marker boundaries are byte-preserved** (TASKS / FOCUS / MERMAID / VISION / MILESTONES). Don't normalize input bytes outside matched pairs.
- **FOCUS / MERMAID / VISION line shape and empty-state strings are agent-grep contract.** Changing wording is a `schema_version` bump.
- **Archive collapse triggers on `phases.N.status = "done"`** and emits the one-line `See [CHANGELOG.md#…]` body. Line shape is locked by `validate --check-render`.

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
- **`landing_ref` is a transition-time free-text field, settable only on `status = "in_progress"`.** Records an open landing pointer (PR URL or other ref — never parsed or fetched). Set via `rmap status <id> in_progress --landing-ref <ref>`; an `in_progress` → `in_progress` call with only this flag is a field update (`started_at` unchanged). `--landing-ref ""` clears it. The flag is a hard error on any other target status (message names the rule; file stays byte-equal). Lifecycle without the flag: **kept on `done`** (provenance next to `shipped_in`) and **kept on `blocked`** (a PR closed unmerged is when the ref matters); **cleared on `pending`** (the work is being redone — the old ref belongs in the appended `attempts` report). Surfaces: `rmap show` prints `landing_ref:` when present; ROADMAP.md rows get a conditional `🔗 <ref>` segment; `--json` / `data.json` / `rmap diff --verbose` (on `TASK_VERBOSE_WHITELIST`). `rmap stale` and `rmap doctor` exclude in_progress tasks carrying a `landing_ref` from the stalled list and report them under `awaiting landing (N)` (JSON kind `awaiting_landing`). Absent from `StdinTask` / `NewTaskFields`. No `schema_version` bump — additive optional field.
- **`attempts` is an append-only transition-time list, settable only on `status = "pending"`.** Each entry is an inline-table `{ at, by?, report }` (stored like `cross_repo`): `at` is auto-filled from `today_iso()`, `by` is free-text agent attribution (optional), `report` is the failure evidence (a reviewer's rejection report). Appended — never overwritten — by `rmap status <id> pending --report "<text>" [--attempt-by <agent>]`; each call adds exactly one entry (no dedup). `--report` on a non-`pending` transition is ignored with a one-line stderr note (mirrors the outcome/`--reason` flags); `--attempt-by` without `--report` is likewise a no-op note. Renders in `rmap show` and `rmap delegate`'s `## Prior attempts` section; surfaces in `--json` / `data.json` (skipped when empty, so attemptless tasks round-trip byte-identically). Transition-time field → mirror surfaces `canonical_task_key_index` (index 34, trailing) + `diff_fields!` + `ExportedTask`/`EXPORTED_TASK_FIELDS`; deliberately NOT in `TASK_VERBOSE_WHITELIST` (a list, like `cross_repo`).
- **`rmap doctor` milestone drift advisories are soft (exit 0, no auto-mutation).** `MilestoneFullyDoneButOpen` when every task pinned to a milestone is `done` but milestone status is `pending` or `active`; `MultipleActiveMilestones` when more than one milestone is `active` (rmap.md: keep exactly one). Human lines cite milestone slug and pinned-task count; JSON kinds `milestone_fully_done_but_open` / `multiple_active_milestones`.
- **`rmap doctor` phase / focus drift advisories are soft (exit 0, no auto-mutation; phase/focus state is user-curated).** `PhaseFullyDoneButOpen` when every task in a phase is `done` but phase status is still `pending`/`active`; `PhaseHasInProgressButPending` when a `pending` phase has ≥1 `in_progress` task; `FocusPhaseClosed` when `focus.phase` points at a phase that appears closed (done status or all tasks done). Human lines cite the phase number (and task ids for the in-progress case); JSON kinds `phase_fully_done_but_open` / `phase_has_in_progress_but_pending` / `focus_phase_closed`.
- **`rmap doctor` graph-health advisories are soft (exit 0, no auto-mutation).** `Bottleneck` when a `pending`/`blocked` task's transitive-dependent count (via `topo::compute_unlocks`) is ≥ `DoctorThresholds.bottleneck_min` (default 3, overridable via `--bottleneck-min`); `IsolatedNode` when a task is an orphan (no in-repo deps and no dependents) or unreachable forward from any milestone-pinned task when milestones exist. Human lines cite task id + dependent count (bottleneck) or id + reason (isolated); JSON kinds `bottleneck` / `isolated_node`.
- **`rmap doctor` spec-quality advisories are soft (exit 0, no auto-mutation).** Token-mechanical only (no semantic/LLM judgment). **AC quality** is scoped to *live agent-assigned* tasks (same predicate as `validate_dispatch_model`: `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`): `PlaceholderCriteria` when any criterion has an unquoted whole-word `TODO`/`TBD`, substring `???`, or `<stub>` angle-bracket token (quoted example mentions are ignored so prose ACs that name the tokens do not self-flag); `VagueCriteria` when a criterion's non-stopword tokens are drawn entirely from the fixed list `fast` / `robust` / `properly` / `correctly` / `works` (a measurable criterion that merely contains one of those words does not fire). **Near-duplicates** pair open tasks (`pending`/`in_progress`/`blocked` only — never `done`/`superseded`) whose normalized title+body token Jaccard is ≥ `DoctorThresholds.near_duplicate_min` percent (default 80, overridable via `--near-duplicate-min`). Human lines cite task id(s); JSON kinds `placeholder_criteria` / `vague_criteria` / `near_duplicate_tasks` (additive).
- **`touches` is an optional creation-time free-text list, advisory and unvalidated** (posture of `model` / `assignee`). Semantically distinct from `files_to_modify`: `files_to_modify` is the implementer's *write target*; `touches` is the broader *involvement hint* (files that may be read or written) — typically a superset. Consumer collision rule (documented, NOT enforced in rmap): two tasks conflict iff `(touches(A) ∪ files_to_modify(A)) ∩ (touches(B) ∪ files_to_modify(B)) ≠ ∅`. Creation-time field → all six mirror surfaces (see the `Task` doc comment in `schema.rs`); on `diff_fields!` but deliberately NOT in `TASK_VERBOSE_WHITELIST`.
- **`domains` is an optional creation-time free-text list, advisory and unvalidated.** It tags the capability/domain evidence an orchestrator may group by (for example `rust`, `otp`, `ecto`); rmap owns no vocabulary and only carries/export the strings. Creation-time field → all six mirror surfaces; on `diff_fields!` but deliberately NOT in `TASK_VERBOSE_WHITELIST` (a list, like `touches` / `cross_repo`).
- **`target_repo` is the optional creation-time landing-repository field.** It names where this task's own work lands; absent resolves to the roadmap's top-level `project`, preserving existing source/render bytes. An explicit blank value is invalid. `cross_repo` remains a list of relationships to related tasks in other roadmaps and does not affect landing. `rmap list --target-repo` filters on the effective value and composes with other list selectors; `rmap delegate` always renders the effective value. Creation-time field → all six mirror surfaces and `TASK_VERBOSE_WHITELIST`.
- **Timestamps validate by shape (`YYYY-MM-DD`), not semantics.** `9999-99-99` passes on purpose; values live next to user-edited TOML.
- **Status / marker / cross-repo-relation enums live in `validate.rs` constants**; render-time match arms in `render.rs` don't share a source — keep both in sync.
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
- **`rmap next` ranking is 4-tier lexicographic** `(in_focus_phase × in_active_milestone) ⇒ Eff desc`, focus dominant: tier 0 (both) > tier 1 (focus-only) > tier 2 (active-milestone-only) > tier 3 (neither). Tasks pinned to ANY milestone with `status = "active"` qualify. Without `[focus]`, every task counts as "in focus" → tiers collapse to 0/1. The focus-dominance bit (tier 1 > tier 2) is the load-bearing decision. `next::rank_tasks` is the single source of this sort, shared with `ready`.
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
- **`rmap delegate`'s seven canonical `##` sections** (`Context` → `Task` → `Acceptance criteria` → `Out of scope` → `Files to modify` → `Scoring` → `Environment notes`) and the `[D:_/B:_/U:_ → Eff:_] <glyph>` Scoring shape are locked by `emits_canonical_section_order_with_distinguishing_line`.
- **`rmap delegate` always emits `Target repo` in `## Context`.** It uses explicit `task.target_repo` or falls back to top-level `project`, so the receiving agent can distinguish the planning repository from the checkout its work targets.
- **`rmap delegate --to` is optional; `assignee` is the routing default.** `delegate::resolve_target` resolves the target: explicit `--to` always wins (and renders the `Stored assignee: ... (overridden)` bullet when it differs); without it the task's `assignee` IS the target. No assignee → exit 1 `has no assignee; pass --to <agent>`; `assignee = "human"` → exit 1 `is assigned to human; pass --to <agent> to delegate anyway`. Both error strings are agent-grep contract. Routing metadata is split: `assignee` = which agent executes, `model` = free-text LLM id/pin, `domains` = free-text capability tags for downstream scoring, `delegate --to` = render-time override.
- **`rmap delegate`'s per-agent footer mirrors `~/.claude/includes/cloud-agent-environments.md`** — sync manually when the skill changes.
- **`rmap diff --against` defaults to `current.default_branch`** — never hardcode `"main"`.
- **`rmap diff --verbose` is additive.** Non-verbose output stays byte-identical to pre-11b. Whitelist members in `TASK_VERBOSE_WHITELIST` / `METADATA_VERBOSE_WHITELIST` are part of the contract.
- **HTML data island id is `rmap-data`, script type `application/json`.** Agents parse the element's text; they do NOT scrape the DOM.
- **HTML task cards carry six `data-*` attributes** (`data-id`, `-status`, `-eff`, `-markers`, `-depends-on`, `-phase`); DAG nodes carry `data-id`; phase sections carry `data-phase-status`. Stable selector contract.
- **Portfolio HTML (`--html --multi`) adds two islands**: `rmap-data` (aggregate `{"projects":[…]}` of every input's envelope) and `rmap-relations` (resolved cross-repo edge array `{source, target, relation}`); repo rows carry `data-slug` / `data-name` / `data-has-rel`. Same parse-the-island-not-the-DOM rule.

**Mirror-surface edit rules**

When adding a field to `schema::Task`, decide whether it is a **creation-time** field (set at `rmap new` time) or a **transition-time** field (set later by `rmap status` / `rmap mark` / `rmap depend` / etc.), then update the appropriate surfaces in the same commit. The full invariant lives on the `Task` doc comment in `src/schema.rs`; this is the working summary.

- **Creation-time field → SIX surfaces:**
  - `main.rs::StdinTask` (stdin parse shape)
  - `mutate.rs::NewTaskFields` (mutator argument struct)
  - `mutate.rs::add_task_str` (TOML writer)
  - `mutate.rs::canonical_task_key_index` (key ordering for serialization)
  - `diff.rs::diff_fields!` (drift surface for `rmap diff`)
  - `export.rs::ExportedTask` (`--json` / `data.json` shape) — AND `export.rs::EXPORTED_TASK_FIELDS` (the `--fields` projection's validation set; `exported_task_fields_cover_serialized_keys` guards drift). Any new `ExportedTask` field (creation-time, transition-time, or computed like `eff` / `dep_layer`) must be added to this const.
  - Then decide whether to add to `diff::TASK_VERBOSE_WHITELIST`. Interactive `prompt_task_fields` (main.rs) is optional — power-user fields (`branch`, `files_to_modify`, `touches`, `target_repo`, `cross_repo`) intentionally require `--from-stdin` rather than dialoguer.
- **Transition-time field** (lifecycle timestamps, `implemented`, outcome-layer, etc.) → update the owning mutator (`set_status_str` for status transitions, etc.) plus `diff::diff_fields!` and `export::ExportedTask`. Stays absent from `StdinTask` / `NewTaskFields` on purpose — today: `started_at`, `done_at`, `blocked_reason`, `shipped_in`, `landing_ref`, `implemented`, `delivered_by`, `verified`, `verified_by`, `verification_ref`, `attempts`.
- **New top-level field on `schema::Tasks`** → also edit `diff::diff_metadata` AND `export::ExportedTasks` (Task-level macro doesn't cover them; hand-walked).

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
- `rmap delegate <id> --to <agent>` — the verbatim output IS the prompt dispatched to the agent
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

`ROADMAP.md` (rendered from `roadmap/tasks.toml`) tracks open phases; `DESIGN.md` carries the design contract and out-of-scope list; `CHANGELOG.md` is the shipped-phase record. Implemented today: validate, render (incl. `--html` static single-project view and `--html --multi` portfolio view), watch, export json, export dot (Graphviz), waves, status (single + bulk), mark, depend, new (interactive + `--from-stdin`), next, ready, show, list, blocks, deps, bundles, milestones, schema, diff, delegate, import, stale, doctor, critical-path. Score-decay rendering is automatic on tasks with `scored_at` >30d or missing. Deliberately out of scope (per `DESIGN.md`): Linear API calls, web server, git integration beyond `git show <ref>:<path>` for `rmap diff`, shell completions, CI workflow, multi-user sync. Don't add these without checking the roadmap first.
