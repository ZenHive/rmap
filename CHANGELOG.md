# Changelog

Completed roadmap tasks. For upcoming work, see [ROADMAP.md](ROADMAP.md); for the design contract and deferred-phase design notes, see [DESIGN.md](DESIGN.md).

---

## [Unreleased]

### Task 9: `rmap render --html --multi` portfolio view + single-view restyle

**What was done:**
- New `--multi <PATH>...` flag on `rmap render` (requires `--html`): renders a multi-project portfolio HTML view. Each input is a project root, a `tasks.toml` path, or a `data.json` path; output defaults to `roadmap/dist/portfolio.html` (override with `--out`, print with `--stdout`).
- Repos render as expandable rows (click to expand inline to the full single-project layout); cross-repo `blocks` / `blocked_by` / `related` relations resolve into repo→repo edges carried in an `rmap-relations` JSON island and drawn as gutter arrows.
- The existing single-project `--html` view was restyled onto a shared design system: templates split into `roadmap.html.j2` / `portfolio.html.j2` over shared `_components.html.j2` macros and one `_styles.css`. The `rmap-data` island and all `data-*` selector contracts are unchanged.
- `topo.rs` grew `compute_layers_from_edges` so the portfolio's JSON-loaded tasks reuse the same longest-path DAG layering.

### Task 32: `rmap doctor` milestone status drift advisories

**What was done:**
- Added two read-side doctor advisories: `MilestoneFullyDoneButOpen` and `MultipleActiveMilestones`.
- The advisories report milestone state drift (all pinned tasks done but milestone still `pending`/`active`; more than one `active` milestone) without mutating milestone status; messages cite milestone slug and pinned-task counts; `rmap doctor` continues to exit 0 for soft findings.
- No schema changes.

### Task 31: `rmap doctor` phase / focus state drift advisories

**What was done:**
- Added three read-side doctor advisories: `PhaseFullyDoneButOpen`, `PhaseHasInProgressButPending`, and `FocusPhaseClosed`.
- The advisories report phase/focus state drift without mutating phase status or focus; `rmap doctor` continues to exit 0 for soft findings.
- No schema changes.

---

## Phase 14 — Migration tooling

### Phase 14 Task 18: `rmap import` — paste-ready ROADMAP.md→tasks.toml migration prompt (import_command bundle)

**What was done:**
- New `rmap import` command: emits a self-contained Markdown prompt an agent pastes into a session to migrate a hand-edited `ROADMAP.md` into `roadmap/tasks.toml`. Pure read — no file mutation; mirrors the `rmap delegate` pattern.
- Prompt template lives in `templates/import_prompt.md` (same `include_str!` pattern as `templates/roadmap.html.j2`). Two runtime substitution tokens (`{{project}}`, `{{schema_json}}`) keep the project name and JSON Schema current without code changes.
- Live JSON Schema embedded via `schema_json_str()` — the schema section of the prompt auto-updates whenever `schema.rs` changes; no hand-maintenance needed.
- Prompt covers the full migration recipe: field-mapping guide (status glyphs, score brackets, prose fields), marker-pair contract (`TASKS:BEGIN`, `FOCUS:BEGIN`, `MERMAID:BEGIN`), and the `validate → render → validate --check-render` verification loop.
- `~/.claude/includes/rmap.md` command table updated with `rmap import` row; SKILLS.md gained an `import` entry in the Delegation section.

---

## Phase 15 — Schema extensions

### Phase 15 Task 40: Formalize `assignee` as the agent-routing field — `delegate --to` defaults to it (agent_routing bundle)

**What was done:**
- `rmap delegate <id> --to` is now optional. New pure resolver `delegate::resolve_target`: explicit `--to` always wins (override bullet unchanged); without it, the prompt targets the task's stored `assignee` — the validated agent-routing field. A task with no `assignee` (or `assignee = "human"`, which is valid but not delegatable) exits 1 with pass-`--to` guidance; both error strings are agent-grep contract.
- This formalizes the field split consumers route on: **`assignee`** = which agent executes (validated), **`model`** = which LLM that agent runs (free-text pin), **`delegate --to`** = explicit render-time override. Closes the harness workaround of overloading `model` with agent names (`"codex"`/`"cursor"`) for cron-poller routing — harness's `RoadmapPoller` switches to `--fields id,assignee,markers` + assignee-based routing as the first consumer (companion change in `../harness`).
- Docs synced same-commit: `SKILLS.md` (delegate section + no-`--to` / exit-1 examples gated by `skills_smoke`), repo `CLAUDE.md` (delegate invariant + harness consumer section), `~/.claude/includes/rmap.md` (assignee as orchestration fact, three-way field split, command table). New tests `delegate_without_to_defaults_to_assignee`, `delegate_without_to_and_no_assignee_exits_one`, `delegate_without_to_and_human_assignee_exits_one` in `tests/cli.rs`.

### Phase 15 Task 39: Widen delegate targets + assignee set — add `grok`, `antigravity`, `pi`, `droid` (delegate_targets bundle)

**What was done:**
- Widened `rmap delegate --to` (`DelegateTarget`) and `assignee` (`VALID_ASSIGNEES`) from `claude|codex|cursor` to **8 total** — added the four local-CLI agents `grok`, `antigravity`, `pi`, `droid`. Unblocks the harness widening its `Harness.Roadmap.ingest/2` `@valid_agents` set and dropping the ingest-under-a-delegatable-agent two-step dance (separate follow-up in `../harness`).
- Each new target gets a grounded `## Environment notes` footer in `append_agent_notes` (literals mirror `~/.claude/includes/cloud-agent-environments.md`): all four are local execution / full toolchain; **Antigravity** carries the `agy` git-common-dir cwd-isolation caveat (can edit the main checkout even when pointed at a worktree); **Pi** notes free/unmetered local LLM in autonomous mode; **Droid** notes it is rmap-only — not yet a harness executor, prompt is for the Factory Droid CLI or manual paste.
- Interactive `rmap new` assignee picker offers the four new ids; `--to <unknown>` still rejected by clap.
- No `schema_version` bump (additive — `DelegateTarget` and `VALID_ASSIGNEES` are closed enums widened, not renamed). Docs synced same-commit: `SKILLS.md` (delegate section + skills_smoke bash blocks), `DESIGN.md`, `~/.claude/includes/rmap.md`, and `~/.claude/includes/cloud-agent-environments.md` (new "Local CLI agents" section — the canonical footer mirror). New tests `delegate_accepts_new_local_agents_with_footers` + `validate_accepts_new_agent_assignees` in `tests/cli.rs`.

### Phase 15 Tasks 35–38: agent-dispatch ergonomics — `rmap ready` + `dep_layer` + `touches` + `handbuild`/`--dispatchable`/`--fields` (agent_dispatch bundle)

**What was done:**
- Makes "the set of tasks I can dispatch in parallel right now" a first-class server-side query instead of a client-side reconstruction from `list --json`. Five surfaces, four commits:
- **`rmap ready`** (Task 35): all `pending` tasks whose every `depends_on` is `done`, ranked by the same 4-tier key as `rmap next` (now shared via `next::rank_tasks`). The set is **mutually independent by construction** — a pending task with all deps done can't depend on another pending task — so there is no `--independent` flag. `--bundle B` yields a bundle's dispatchable layer-0 (the parallel batch `next-bundle`'s serial chain can't express); `--count` is optional (default = whole set); `--phase` filters the pool. `list`-shaped `--json` envelope.
- **`dep_layer`** (Task 36): longest-path layering extracted from `render_html` into a shared pure `src/topo.rs`; exposed as a computed `ExportedTask` field (like `eff`, never persisted) on every `--json` payload. Always computed over the full `tasks.task` graph, so the slice-taking export fns now take `&Tasks`. Within a result set the lowest `dep_layer` present is the current parallel wave. Additive — no `schema_version` bump.
- **`touches`** (Task 37): optional creation-time `Vec<String>` field, semantically distinct from `files_to_modify` (the write target) — the broader *involvement hint*, typically a superset. Consumers union both fields to predict parallel-dispatch conflicts (`rmap` documents the rule but does not enforce it). All six creation-time mirror surfaces + `diff_fields!` (deliberately not in `TASK_VERBOSE_WHITELIST`). Unvalidated free-text; additive.
- **`handbuild` + `--dispatchable` + `--fields`** (Task 38): `handbuild` ∈ `VALID_MARKERS` flags human-driven-browser work (LiveView/UI/DOM); `--dispatchable` (on `ready`/`list`) excludes it so everything else is headless-dispatchable by default. `--fields a,b,c` projects `--json` to a token-cheap bare array of just the named `ExportedTask` keys (validated against `EXPORTED_TASK_FIELDS`, drift-guarded); implies `--json`, unknown name exits 1.
- No `schema_version` bump (all additive). `SKILLS.md`, `~/.claude/includes/rmap.md`, and repo `CLAUDE.md` updated; new tests in `tests/cli.rs` + unit tests in `src/topo.rs` / `src/export.rs`.

### Phase 15 Task 34: `--reason` flag on `rmap status` — settable, rendered, auto-cleared `blocked_reason` (schema_outcome bundle)

**What was done:**
- New `--reason "<text>"` flag on `rmap status`: sets `blocked_reason` in the same `blocked` transition (free-text, overwrites). Closes the gap where `blocked_reason` was required-on-blocked and surfaced read-side (schema, validation, export, diff, show) but had no mutator — it could previously only be set by hand-editing `tasks.toml`, exactly the workaround the harness merge-train lander (its terminal sink for cap-exhausted tasks) can't take. Blocked-only: non-`blocked` transitions emit a one-line stderr note and skip the write, mirroring the `done`-only outcome flags.
- **Auto-clear:** transitioning a blocked task to any other status drops the now-stale `blocked_reason` (it described a state that no longer holds); re-blocking keeps/overwrites.
- **Render:** blocked tasks now show their reason inline in `ROADMAP.md` as a trailing `⛔ <reason>` segment. Conditional + additive — non-blocked rows render byte-identically; golden-guarded by `tests/golden/mermaid_block`.
- Internal: the four `done`-transition write-fields plus `blocked_reason` are now carried by `TransitionFields` (renamed from `DoneFields`), each gated on its matching transition; the `update_status` wrapper takes the struct to stay under clippy's arg ceiling. No `schema_version` bump — the field already existed; `--reason` stays off the creation surfaces (`StdinTask` / `NewTaskFields`).
- Tests in `tests/mutate.rs` + `tests/cli.rs` (set-on-blocked, ignored-on-non-blocked, auto-clear across pending/in_progress/done, overwrite); `SKILLS.md`, `~/.claude/includes/rmap.md`, and repo `CLAUDE.md` updated.

### Phase 15 Task 33: `--shipped-in` flag on `rmap status` — complete the outcome layer (schema_outcome bundle)

**What was done:**
- New `--shipped-in <sha>` flag on `rmap status`: persists `shipped_in` (commit/PR ref, free-text) in the same `done` transition as `--implemented` / `--delivered-by` / `--verified`. Closes the gap where `shipped_in` existed end-to-end on the read side (schema, export, diff, show) but had no mutator — it could previously only be set by hand-editing `tasks.toml`. Completes the outcome triple: `delivered_by` (who), `verified` (graded?), `shipped_in` (where it landed).
- Mirrors `--delivered-by` exactly: `done`-only (non-`done` transitions emit a one-line stderr warning and skip the write), overwrites on re-set, bulk `rmap status 1,2 done --shipped-in <sha>` applies the same value to every matched task. No sha-shape validation (free-text), no git auto-derivation — the caller supplies it.
- Stays off the creation surfaces (`rmap new` / `StdinTask` / `NewTaskFields`) — `shipped_in` is a transition-time outcome fact, not creation-time intent. Field already in the schema, so **no `schema_version` bump**.

### Phase 15 Task 28: Outcome layer — `delivered_by` + `verified` fields (schema_outcome bundle)

**What was done:**
- Two new optional transition-time fields on `schema::Task`: `delivered_by: Option<String>` (free-text agent id, like `model`) and `verified: Option<bool>` (independent-evaluator confirmation). Both land via `rmap status <id> done --delivered-by <agent> --verified` alongside `--implemented`. Set only on `done` transitions; non-`done` transitions emit a one-line stderr warning and skip the write, mirroring `--implemented`. Both overwrite on re-set.
- **Two-state `verified` semantics:** `Some(true)` = an independent check passed (verification stack green, code-review approved); absent = not yet graded (hand-built, bootstrap, merged directly). `--verified` is a presence flag; to clear, edit `tasks.toml` directly. Encodes evaluator separation per `workflow-philosophy.md` as a queryable fact — `done` means "an implementer said so", `verified` means "a grader agreed".
- **Read surfaces:** `rmap show` renders `delivered_by: <agent>` and `verified: yes/no` lines right after `implemented`, so a cold reader sees the outcome layer as one block. `ExportedTask` (and therefore `roadmap/data.json` + every `--json` envelope) carries both fields with `skip_serializing_if = Option::is_none`. `rmap diff --verbose` surfaces drift via the `TASK_VERBOSE_WHITELIST` (matching `implemented`'s precedent — verbose-mode only).
- **`rmap list --delivered-by <agent>` filter:** new `TaskFilter::delivered_by` + `matches_delivered_by` helper, status-agnostic (matches field value regardless of status, consistent with `--milestone`). Turns the roadmap into an agent-delivery ledger as a free side effect of normal completion.
- **`rmap doctor` soft advisory:** new `DoctorFinding::ClaimedNotGraded { id }` variant fires for `status == "done" && verified.is_none()`. Always exit 0 — hand-built and bootstrap tasks legitimately land ungraded. Rendered as "Claimed, not graded (done without `verified`)" with a per-task nudge to set `--verified` once an independent check passes.
- **Took the lighter transition-time mirror path** the `Task` doc-comment describes: `schema.rs` + `set_status_str` write block + `canonical_task_key_index` (inserted after `implemented` at slots 21–22, shifting timestamps down by 2) + `diff_fields!` + `ExportedTask` + `query.rs` render. Deliberately stayed off `StdinTask` / `NewTaskFields` — these are outcome facts, not creation-time intent.
- **No `schema_version` bump.** Purely additive optional fields; existing `tasks.toml` files round-trip validate → render byte-identically (round-trip suite green; existing `DOCTOR_CLEAN_TASKS` / `DOCTOR_LINT_TASKS` fixtures gained `verified = true` on their done fixtures so the new soft advisory doesn't fire under the "clean fixture" tests).
- **Docs:** `CLAUDE.md` mirror-surface rule lists both fields under transition-time; load-bearing-invariants gained the outcome-layer paragraph (`Some(false)` permitted by schema but mutator never writes it; doctor advisory always exit 0). `DESIGN.md` schema example carries both fields with comments, and the invariants section gained the outcome-layer paragraph framing it as evaluator-separation-as-queryable-fact.

### Phase 15 Task 30: `rmap validate` does not detect duplicate task ids (schema_milestones bundle, 🐛 bug)

**What was done:**
- `validate_tasks_str` ran sixteen semantic checks but none asserted task-id uniqueness. A roadmap with two `[[task]]` entries sharing an id — from a hand-edit, a botched `rmap import`, or the `next_task_id` bug fixed in Task 29 — passed `rmap validate` clean. Added `validate_unique_ids` as the 10th of 17 checks (between `validate_implemented` and `validate_dependencies` — ordering matters: duplicate ids would otherwise confuse dependency resolution).
- **Made `TaskId::Eq` / `Hash` normalizing across `Number(n)` ↔ `Text("n")`.** A task id is a primary key; the disk form (TOML integer vs string of the same digits) does not change which task it names. Without this, a `HashSet<TaskId>` treats `Number(1)` and `Text("1")` as distinct keys — exactly the gap that let the Task-29 bug's bad output reach disk uncaught. New canonical-key helper (`Cow`-returning, zero-alloc for non-numeric text ids) backs the manual `PartialEq` / `Hash` impls.
- **Four call sites become correct on mixed-form files for free** — no edits required: `validate_dependencies`, `validate_dependency_cycles`, `doctor.rs` degenerate-bundle check, `next_bundle.rs` actionability memo. All key on `TaskId` (directly or via `&TaskId`) and inherit the normalizing behavior.
- **Cross-form error message uses TOML-literal forms** so the human can locate both lines: `duplicate task id "1" (already defined as 1)` for mixed-form collisions, plain `duplicate task id 1` for same-form. `format_id_literal` helper renders ids as they appear on disk (bare for `Number`, quoted for `Text`) — distinct from `Display`, which strips quotes for the human-identity case. `PartialEq<u32>` also tightened to accept `Text("1") == 1u32` for consistency with the normalizing `Eq`.
- **Text ids that do not parse as `u32` (`"INE-5"`, `"alpha"`) keep their own canonical key** — `Number(5)` ≠ `Text("INE-5")`. Agent-grep contract substring: `duplicate task id`.
- **No `schema_version` bump.** Purely additive validator + tighter in-memory equality; on-disk format and roundtrip unchanged. `validate_command_rejects_duplicate_task_ids` (CLI test) plus five `tests/validate.rs` tests cover same-form, cross-form, mixed-form happy path, cross-form self-cycle, and distinct mixed-form ids.
- **Docs:** `CLAUDE.md` load-bearing-invariants section gained a TaskId Eq/Hash bullet; `DESIGN.md` schema-invariants gained the matching paragraph.

### Phase 15 Task 29: `rmap new` allocates a colliding task id on a string-id roadmap (schema_milestones bundle, 🐛 bug)

**What was done:**
- `next_task_id` (`src/mutate.rs`) computed the next auto-allocated id with `value.as_integer()`, which matches only TOML integer-typed ids. `TaskId` is dual-form — a roadmap may store ids as integers (`id = 28`, rmap's own convention) or as numeric strings (`id = "28"`, the convention harness's roadmap uses). On a string-id roadmap every `as_integer()` returned `None`, `max` stayed `0`, and `next_task_id` returned `1` — silently allocating a task whose id collided with the existing `id = "1"`. The colliding task was unreachable by `rmap show`, and `rmap validate` passed it clean (duplicate detection keys on `TaskId`, whose `Number(1)` and `Text("1")` are distinct values).
- **Fix 1 — count both id forms.** New `task_id_value_as_u32` helper reads a task `id` value as a `u32` whether it is integer-typed or a numeric string; `next_task_id` now folds it over every task, so `max + 1` is correct on a string-id roadmap. A non-numeric text id (`id = "MW-7"`) is still skipped — it has no place in the numeric sequence.
- **Fix 2 — serialize in the file's form.** `add_task_str` always wrote the allocated id as a TOML integer, so even a corrected allocation drifted `id = 29` into a file of `id = "28"` strings. New `ids_are_string_typed` helper detects the roadmap's id form; an auto-allocated id is now serialized to match, so `rmap new` never mixes the two forms within one file (a mix defeats `TaskId`-keyed duplicate detection).
- **No regression for integer-id roadmaps.** rmap's own roadmap uses integer ids — `ids_are_string_typed` returns `false`, the integer path is byte-identical to before. Verified by a dedicated no-regression test.
- **Three unit tests** in `mutate.rs`: string-id roadmap continues the numeric sequence (`"1"`, `"2"` → `3`, not `1`); the allocated id is written string-typed on a string-id file; an integer-id file still gets an integer id. Full `cargo test` suite and `cargo clippy` green.
- A sibling defect — `rmap validate` has no duplicate-task-id check at all — was filed separately as Task 30 (it is why this bug's bad output reached disk uncaught).

### Phase 15 Task 27: Active-milestone preference in `rmap next` (schema_milestones bundle)

**What was done:**
- `next_tasks` (in `src/next.rs`) now ranks pending unblocked tasks by a 4-tier lexicographic key (`focus-phase × active-milestone`, **focus dominant**) then Eff desc. Tier 0 = both, tier 1 = focus-only, tier 2 = active-milestone-only, tier 3 = neither. Replaces the prior `partition(focus)` + per-partition `sort_by_eff_desc` + concat strategy with a single stable sort over the candidate vector. The `next::tier` helper centralizes the cross-product; the active-milestone set is computed once per call from `tasks.milestones`.
- **Focus-dominance over milestone is the load-bearing decision.** A task in `[focus].phase` but outside any active milestone (tier 1) beats a task pinned to an active milestone but outside the focus phase (tier 2). This was the consumer-first call: focus answers "what am I working on right now?", milestone answers "what release line is this for?" — when they diverge, the daily-path question wins.
- **Degenerate cases preserve prior behavior.** Without `[focus]`, every task is treated as in-focus → tiers collapse to 0/1 (pure milestone bias). With no `active` milestone, tiers collapse to 1/3 — equivalent to the original focus-phase partition; the regression test `no_active_milestones_preserves_focus_only_behavior` pins this byte-identically.
- **No `schema_version` bump.** Pure read-path ranking change — schema, validator, mutators, and JSON envelopes are untouched. `--milestone <name>` filter semantics also unchanged (narrows the pool before tiering, so an explicit filter onto a non-active milestone reduces to pure Eff desc within that pool).
- **Six new tests** in `tests/next.rs`: active-ms wins on low Eff (no focus); focus beats ms on divergence (tier 1 > tier 2); tier-0/1/2 ordering when all three are populated; multi-active milestones (any `active` qualifies); no-active regression (byte-identical to focus-only behavior); explicit `--milestone` filter falls back to pure Eff within the filtered pool.
- **Docs**: `src/next.rs` module doc rewritten around the tier table; `CLAUDE.md` "Agent contract" section gained a new invariant pinning the 4-tier key (focus-dominance bit called out as load-bearing); `SKILLS.md` "Picking work" and "Milestones" sections updated to describe the auto-bias; `~/.claude/includes/rmap.md` Milestones section gained the auto-bias + focus-dominance note.

### Phase 15 Task 25: Backfill creation paths with `branch` / `files_to_modify` / `cross_repo` (schema_milestones bundle, 🐛 bug)

**What was done:**
- Backfilled the three power-user fields onto the `rmap new` creation surfaces so callers no longer have to manually edit `tasks.toml` after creation. `branch`, `files_to_modify`, and `cross_repo` already existed on `schema::Task`, were already surfaced by `rmap diff` / `--json` / `rmap delegate`, but had no path through `StdinTask` (stdin parse shape) or `NewTaskFields` + `add_task_str` (writer). Originally surfaced by audit-review on 2026-05-17 (`audit(dcad8e5..794036e)`), deferred from commit 180329c as out-of-scope for that targeted fix.
- **Four-place writer edit:** `main.rs::StdinTask` gained the three fields (with `#[serde(default)]` on the two `Vec` types), `mutate.rs::NewTaskFields` gained the matching slice borrows, `mutate.rs::add_task_str` gained the three conditional writers (empty-slice → skip serialization; `cross_repo` task_id is integer-when-parseable, mirroring `add_dependency_str`'s shape), and `mutate.rs::canonical_task_key_index` gained `"files_to_modify" => 11` (between `out_of_scope` and `cross_repo`), shifting subsequent indices by one.
- **Interactive `prompt_task_fields` (main.rs) intentionally not extended.** Documented skip path = use `rmap new --from-stdin` for `branch` / `files_to_modify` / `cross_repo`. The interactive flow stays focused on common-path fields; `dialoguer::Input` for path lists and inline-table cross-repo entries would be heavier than the value warrants. The `StdinTask` doc comment now spells out the power-user-fields-via-stdin contract explicitly.
- **Mirror-surface invariant promoted to load-bearing doc**: `src/schema.rs::Task` doc comment now carries the full SIX-surfaces (creation-time) vs three-surfaces (transition-time) decision tree on its head. `CLAUDE.md` "Mirror-surface edit rules" section restructured around the same split — the old "Three-place edit rules" subhead was incomplete (only covered surfaces 4–6, missed the StdinTask / NewTaskFields / add_task_str / canonical_task_key_index mirror that this very bug drifted across). Future field-adds now have one canonical checklist.
- **Four new round-trip tests** in `tests/cli.rs`: `new_from_stdin_round_trips_branch_field`, `new_from_stdin_round_trips_files_to_modify_field` (asserts `rmap delegate` renders `## Files to modify` end-to-end), `new_from_stdin_round_trips_cross_repo_field` (asserts integer-vs-text `task_id` shape preservation + optional `linear_id`), and `new_from_stdin_emits_canonical_order_for_creation_fields` (asserts `out_of_scope` < `files_to_modify` < `cross_repo` < `model` < `branch` byte ordering in the written TOML).
- **No `schema_version` bump.** Purely a writer-surface backfill — the schema and validator are unchanged; existing `tasks.toml` files validate identically.

### Phase 15 Task 24: Milestones — first-class release lines (schema_milestones bundle)

**What was done:**
- New `[milestones.<name>]` top-level table on `schema::Tasks` — fourth orthogonal concept alongside phases / bundles / markers. **Phase** orders work, **bundle** groups topically, **markers** modify execution, **milestone** pins a task to a release line. Real release lines cross phases by design (a `v1.0` cut typically pulls from several phases) — previously unrepresentable without smuggling it into markers (`to-prod`, `to-v0.1`) or naming-convention bundles (phase-bound). `Milestone { name, description?, order, status, target_version? }` mirrors the `Bundle` shape; `status` is `pending | active | done` (distinct vocabulary from task status — `active` surfaces first in `rmap milestones` and is the load-bearing affordance for the "what release am I cutting next?" daily-path query).
- New optional `milestone: Option<String>` field on `schema::Task` — pins a task to one milestone. Absent = unpinned (default). Validated in `validate_milestone_references`: any `milestone = "..."` value must reference a declared `[milestones.<name>]` key.
- **No `schema_version` bump.** Purely additive optional fields (mirrors the Phase 15 Task 19 `Task::model` precedent). Existing tasks at `schema_version = 2` validate identically; the new validation rule only fires when a task opts in by setting `milestone`. The AC initially asked for a bump; reconciled in the plan against the consumer-first criterion that bumps churn ~20 golden fixtures + the skills fixture + the migration-hint test for zero protective value.
- **New mutator** `rmap milestone <id> <name|none>` — pins or unpins a task. `none` is the literal unpin sentinel. Validate-then-write: unknown milestone names reject before any file mutation. `canonical_task_key_index` gained `"milestone" => 3` (between `bundle` and `status`); newly-inserted fields land in the canonical position when the mutator auto-sorts.
- **New selector listing** `rmap milestones` — mirrors `rmap bundles` end-to-end (own module `src/milestones.rs`). Five-branch glyph ladder (`✅` / `🚧` / `all-blocked ⛔` / `pending:<n> (deps unmet) ⏸` / `next:<id> [Eff:_] <tier_glyph>`), with a trailing `[target=<version>]` segment when `milestone.target_version` is set. Sort key: `(status_rank: active=0/pending=1/done=2 asc, milestone.order asc)`. `--has-next`, `--status`, `--json` compose with AND semantics.
- **New filter** `--milestone <name>` on `rmap list` and `rmap next` — drives the daily release-cycle loop. Composes with `--bundle`, `--phase`, `--marker`. Unknown milestone returns empty / null (mirrors `--bundle` precedent).
- **Render surfacing** (`src/render.rs`): conditional `🚀 **<milestone>** ·` segment in the task row, between bundle and module_segment. Inline-segment placement was chosen over (b) a fourth column (breaks the 3-col `validate --check-render` agent contract; 20+ fixture regen) and (c) a grouped milestone section (rare-path view; invisible during daily phase-scan). Rows without a milestone render byte-identically to pre-Task-24 — golden fixtures unaffected unless the fixture declares a milestone.
- **Show & delegate surfacing**: `rmap show` prints a `milestone:` line (`milestone: v0_1 (target=0.1.0)` when `target_version` is set); `rmap delegate` adds a conditional `- Milestone:` bullet to `## Context` so the target agent knows which release ships their work.
- **Three-place edit per the documented invariant** plus the two-place edit for the top-level table: `src/schema.rs` (the new field + struct), `src/diff.rs::diff_fields!` + `TASK_VERBOSE_WHITELIST + 1` + `diff_metadata::map_diff("milestones", …)` + `impl MapEntry for Milestone`, `src/export.rs::ExportedTask` + `ExportedTasks` (both gain the new fields with `skip_serializing_if = "Option::is_none"` on the task-level scalar).
- **Interactive `rmap new` prompt**: when milestones are declared, a Select dropdown over `tasks.milestones.keys()` with a "Skip" choice surfaces the milestone field. `--from-stdin` accepts `milestone = "..."` on the TOML input.
- **New tests**: 17 new tests across `validate` (3: reject unknown reference, accept declared milestone, reject invalid milestone status), `diff` (3: surface adds/removes/changes for the table + the task field), `cli` (10: filter, mutator happy path + unknown-target reject + unpin, selector listing with glyphs + JSON envelope + filters + empty case, show, stdin-new), `delegate` (3: with-target / no-target / no-milestone bullet shapes). New golden fixture `tests/golden/milestone_inline/` exercises the conditional 🚀 segment.
- **Docs**: `SKILLS.md` gained a Milestones section (fenced bash blocks for `rmap milestones`, `rmap milestone <id> <name|none>`, `rmap list --milestone`, `rmap next --milestone` — auto-covered by `skills_smoke.rs` via a `[milestones.demo]` addition to the skills fixture). `~/.claude/includes/rmap.md` gained a "Milestones — first-class release lines" subsection. `DESIGN.md` schema example + CLI surface + invariants extended. `CLAUDE.md` gained three load-bearing invariants (milestone-status enum location, render-row 🚀 segment shape, `rmap milestones` row contract).

### Phase 15 Task 22: `Task::implemented` field — record what was actually delivered (schema_implemented bundle)

**What was done:**
- New `implemented: Option<String>` field on `schema::Task` — parallel to `body`, captures *what shipped* vs the original *intent*. Conditionally required when `status = "done"` (mirrors the `blocked_reason` rule when `status = "blocked"`); optional/absent on pending / in_progress / blocked. `validate.rs` rejects done tasks lacking `implemented` with the error `task <id> is done but missing implemented` (agent-grep substring `done but missing implemented`).
- `schema_version` bumped 1 → 2. Existing tasks.toml files at version 1 are rejected at validate time with `unsupported schema_version 1 (expected 2); see CHANGELOG.md for v2 migration` — the migration is one-time backfill (option A from the task body): set `implemented = "..."` on every existing done task. The earlier "run rmap migrate" hint was retired (no `rmap migrate` command exists).
- Three-place edit per the documented invariant: `src/schema.rs` (the field), `src/diff.rs::diff_fields!` (so `rmap diff` notices add/remove/change), `src/export.rs::ExportedTask` (so `data.json` / `show --json` / `list --json` / `next --json` surface the field additively with `skip_serializing_if = "Option::is_none"`). Added to `TASK_VERBOSE_WHITELIST` — scalar string; surfaces in `rmap diff --verbose`.
- **Consumer-first additions** (over and above the AC, decided by evaluating the feature from the daily-use POV of the agent that will actually consume it):
  - `rmap status <id> done --implemented "..."` flag added on the mutator. Eliminates the two-step "transition fails → manually edit TOML → re-transition" friction that would otherwise drive agents to write `implemented = "as specified"` reflexively just to make the error go away. Flag overrides existing values (the transition-time content is the most current); ignored on non-`done` transitions with a one-line stderr note. Works in bulk mode (`rmap status 1,2,3 done --implemented "..."`) — the value is applied to every matched task.
  - **Interactive TTY prompt** for `rmap status <id> done` without `--implemented`: when stdin is a TTY and any matched task is missing `implemented`, `rmap` prompts via `dialoguer::Input`. Non-TTY (agents/scripts) skip the prompt and let `validate_implemented` surface the existing error message — agents get a clean failure with the field-name substring instead of a hang.
  - **Visual distinction in `rmap show`**: when a done task carries both `body` and `implemented`, the human view renders `body (original intent):` and `implemented (what shipped):` headers so drift between spec and reality is obvious at a glance. Plain `body:` / `implemented:` headers preserve the legacy shape when only one is present.
- `rmap delegate` gains a conditional `## What was actually implemented` section between `## Task` and `## Acceptance criteria` — only emitted when `implemented` is populated (typically a no-op for delegation, which is normally for pending work). The seven-canonical-section contract for delegate output is now seven sections by default + one conditional eighth.
- `mutate.rs::canonical_task_key_index` gained `"implemented" => 18` (right after `body`); newly-inserted `implemented` fields land near the other small scalars in `tasks.toml` rather than after multi-line entries. `update_status_many_str` extended with an `implemented: Option<&str>` parameter; existing callers updated.
- **One-time stub backfill applied:** all 21 done tasks in rmap's own `roadmap/tasks.toml` got rich `implemented` summaries sourced from the existing CHANGELOG entries. Every test fixture with done tasks (~22 golden fixtures + inline test constants in 10 .rs files) got a generic `implemented = "fixture"` stub. The cutoff-softening alternative was rejected because it would leave permanent code & cognitive overhead ("this old task has no implemented because of cutoff date X") that defeats the auditability goal.
- Project-level `CLAUDE.md` gained a top-level rule **"Evaluate roadmap & task design from the consuming-agent POV"** — codifies the posture used to design this very feature.
- `~/.claude/includes/rmap.md` (cross-project consumer-facing include) updated with the `--implemented` flag note and the conditional-required rule.
- Tests: two new `tests/validate.rs` tests mirror the `blocked_reason` pair (`rejects_done_status_without_implemented`, `accepts_done_status_with_implemented`); existing CLI status tests extended with `--implemented "..."`; `tests/diff.rs` extended to assert `implemented` appears in `changed_fields` and (whitelisted) `values`; `rejects_unknown_schema_version_with_migration_hint` updated to use 99 as the wrong version (since 2 is now valid) and asserts the new error-message substrings (`expected 2`, `CHANGELOG.md`).

### Phase 15 Task 19: `Task::model` field — per-task LLM model pinning (agent_routing bundle)

**What was done:**
- New optional free-text `model` field on `schema::Task` — records which LLM model should do the task (e.g. `claude-opus-4-7`). Unvalidated: model IDs churn, so a closed enum would rot; mirrors `module` / `branch`. Distinct from `assignee` (who owns it) and `rmap delegate --to` (which agent *environment*).
- `rmap delegate` surfaces it as a conditional `- Model:` bullet in the prompt's `## Context`, so the target agent knows which model to run; omitted when unset.
- `rmap show` prints a `model:` line; the field surfaces in `data.json` / `show --json` / `list --json` / `next --json` and in `rmap diff` (on `TASK_VERBOSE_WHITELIST`).
- Settable at creation: `NewTaskFields` + `StdinTask` + `add_task_str` + the interactive `rmap new` prompt all carry `model`; `canonical_task_key_index` places it immediately after `module`.
- Not rendered into `ROADMAP.md` rows — mirrors `assignee`. No `render.rs` change, no golden fixtures, no `validate --check-render` drift.
- No `schema_version` bump — additive optional field.
- Consumer-facing `~/.claude/includes/rmap.md` gained a "Pinning an LLM model per task" subsection; CLAUDE.md gained a "keep rmap.md in sync" discipline note.
- Tests: delegate `- Model:` bullet present/absent; `rmap new --from-stdin` round-trip (tasks.toml canonical order + data.json + `rmap show` human/JSON); `export` surfaces `model` and skips it when unset.

---

## Phase 6 — HTML render

### Phase 6 Task 8: `rmap render --html` single-project view (html_single bundle)

**What was done:**
- New `--html` flag on `rmap render`: an additive output that writes a self-contained static HTML view to `roadmap/dist/index.html` (gitignored). Renders `ROADMAP.md` + `data.json` as usual, then writes the HTML; `roadmap/dist/` is created on first run. With `--dry` it adds a `would write …index.html` line; with `--stdout` the more-specific flag wins — the HTML is printed and nothing is written.
- New `src/render_html.rs`: `render_html_str(tasks, today)` builds `PhaseView` / `TaskView` / `DagView` in pure Rust, then renders the template. Phases sort by `phase.order`; `blocked` tasks share the pending column (distinguished by `data-status="blocked"`).
- New `templates/roadmap.html.j2`: the one minijinja template, `include_str!`'d so it ships in the binary (minijinja was already a declared-but-unused dependency). Inline CSS (screen + `@media print` with `✓ → ⏸ ⛔` status symbols and `N/M done` numerals), a sticky filter bar (marker chips + status toggles), per-phase progress bars + 3-column status layout, a collapsed `<details>` SVG layered-DAG dependency graph, the `<script id="rmap-data">` data island (compact `data.json` via `| safe` — the only `| safe` in the template; everything else relies on HTML autoescape), and ~40 lines of vanilla JS for client-side filtering.
- DAG layout: longest-path layering over in-repo `depends_on` only (`cross_repo` is portfolio-mode, Task 9), memoized; nodes slotted by id within a layer, coords from fixed `NODE_W`/`NODE_H`/`H_GAP`/`V_GAP`/`PAD` constants, one downward edge per resolved dependency.
- Every task card carries six `data-*` attributes (`data-id` / `data-status` / `data-eff` / `data-markers` / `data-depends-on` / `data-phase`); DAG nodes carry `data-id`. These plus the `rmap-data` island id are the agent contract — new CLAUDE.md invariants lock them.
- `ResolvedPaths` gained `html_path` (`project_root/roadmap/dist/index.html`), always derived from `project_root` — no `--html-path` override flag.
- New `export::export_compact_json_str` — the single-line `data.json` variant for the island.
- `.gitignore` gained `/roadmap/dist`; `SKILLS.md` gained a `rmap render --html` block (auto-covered by `skills_smoke.rs`) and a layout-diagram line; CLAUDE.md gained the module entry, pipeline note, and two load-bearing invariants.
- Out of scope (deliberate): `--multi`/portfolio view (Task 9), HTML coverage in `validate --check-render` (it gates `ROADMAP.md` drift only), HTML re-render in `rmap watch` (canonical artifacts only). The <50KB size budget is a target, not an asserted test — rmap's own roadmap renders to ~37KB.
- No `schema_version` bump — additive flag.
- Tests: 8 inline `#[cfg(test)]` unit tests in `src/render_html.rs` (DAG layering: no-deps, chain-of-3, diamond longest-path; single-node coords; one-edge-per-dep; empty-roadmap render; data-island + attrs presence; `truncate_label`). 5 new `tests/cli.rs` black-box tests (writes a self-contained index; no index without the flag; `--dry` writes nothing; `--stdout` prints HTML without writing; six `data-*` attrs present + idempotent re-run).

### Phase 6 — HTML render polish: phase-status badge

**What was done:**
- `PhaseView` now carries the `[phases.N].status` field — previously dropped, so a `done` phase rendered identically to an active one (just with empty Pending/In Progress columns).
- The phase header gains an uppercase status badge (`.phase-status`, colored by status), and each `<section class="phase">` gains a `data-phase-status` attribute. `done` phases are dimmed (opacity 0.6); hover and `@media print` restore full opacity.
- New inline test asserts the badge + `data-phase-status` attribute render.
- No `schema_version` bump — additive: a new badge element, a new `data-*` attribute on the section, and CSS only.

---

## Phase 7 — `rmap watch` (optional)

### Phase 7 Task 10: `rmap watch` FS watcher (watch_core bundle)

**What was done:**
- New `rmap watch` subcommand: a foreground, blocking FS-watch loop that re-renders `ROADMAP.md` + `roadmap/data.json` whenever `roadmap/tasks.toml` changes. Flags: `--tasks-path` / `--roadmap-path` / `--data-path` (same path-override set as `rmap render`). Stop with Ctrl-C (default SIGINT — no `ctrlc` crate added).
- New `notify = "8"` dependency, used via the synchronous `recommended_watcher` + `std::sync::mpsc` channel path — no async runtime. `notify` was already named in `DESIGN.md`'s planned crate-dependency list.
- New `src/watch.rs` module carries two pure, unit-tested helpers: `write_if_changed(path, content) -> io::Result<bool>` (writes only when rendered bytes differ from disk; returns whether it wrote — the idempotency primitive) and `is_tasks_toml_event(tasks_path, &notify::Event) -> bool` (filters directory-level FS events to `tasks.toml` by exact-path OR `file_name()` match). The `notify` wiring and the blocking `for res in rx` loop live in `main.rs::watch_command`.
- The watcher is registered on the `roadmap/` **directory** (`RecursiveMode::NonRecursive`), not on `tasks.toml` directly — watching the file breaks on editor atomic-save renames (the held inode gets replaced). The filename filter is the infinite-loop guard: `roadmap/data.json` is rewritten by every render and lives in the same watched directory, so without it our own `data.json` write would re-trigger the loop endlessly.
- Idempotent (AC #3): every render goes through `write_if_changed`, so a save that produces no rendered change (a TOML comment edit, `touch`) writes nothing and prints nothing.
- Resilient: a mid-edit invalid `tasks.toml` prints its error to stderr and the loop continues — it never aborts on a bad save. The startup `watching …` line and all errors go to stderr; stdout carries exactly one `rendered` line per real re-render (the stdout/stderr split leaves room for a future `watch --json`, Task 11). `watch_command` also does one initial idempotent render at startup so outputs are current before the first event.
- New CLAUDE.md "Load-bearing invariants" bullets lock the directory-watch + filename-filter guard and the idempotency / stdout-stderr split. SKILLS.md gained a "Live dev" section — deliberately in a `text` fence, not `bash`, because `rmap watch` never exits and would hang `skills_smoke.rs`. Exit-code table gained a `watch` row.
- No `schema_version` bump — additive subcommand.
- Tests: 7 inline `#[cfg(test)]` unit tests in `src/watch.rs` covering `write_if_changed` (writes-on-missing, writes-on-differ, skips-on-equal) and `is_tasks_toml_event` (exact match, file-name match after rename, ignores `data.json`, ignores `ROADMAP.md`). Per the chosen test strategy no integration test was added; the `notify` wiring was verified by manual smoke — atomic-save re-render, comment-edit no-op, `data.json`-edit loop-guard, and broken-TOML resilience + recovery.

### Phase 7 Task 11: `rmap watch --json` event stream (watch_stream bundle)

**What was done:**
- New `rmap watch --json` flag: the same FS-watch loop, but each render event is emitted as one **compact JSON line** to stdout instead of the `rendered` text line. Two event shapes — `{"schema_version":1,"event":"rendered","outputs":[...]}` on a real write and `{"schema_version":1,"event":"error","message":"..."}` on a failed render (AC #1). No-op renders stay silent in both modes.
- `outputs` lists the basenames of *exactly* the files written this render — a single-entry array when only `ROADMAP.md` or only `data.json` changed. Basenames, not absolute paths, keep host filesystem layout out of the stable agent contract.
- An `error` is treated as a render event and goes to **stdout** as a JSON `error` event — the `--json` stream is self-contained. Only the startup `watching …` line and `notify` watcher-infrastructure errors stay text on stderr; `--json` changes stdout only.
- New `src/watch.rs` pure helpers `render_event_line(outputs)` / `error_event_line(message)` build a private `WatchEventJson` and serialize via `serde_json::to_string` (compact) — the one place in rmap that uses `to_string` over `to_string_pretty`, because the stream contract is one object per line. `WATCH_SCHEMA_VERSION = 1` is the bump point for the event shape.
- `main.rs`: `rerender_if_changed` now returns `RenderOutcome { roadmap_changed, data_changed }` instead of a bare `bool` so `report_render` can list the changed outputs; `report_render(result, &paths, json)` branches on the flag. `--json` lines go through `emit_json_line`, which flushes stdout so a piped consumer (`jq`) isn't batched.
- Event shape documented in `SKILLS.md` (AC #2) — extended the existing `text`-fenced "Live dev" block; kept out of `bash` fences since `rmap watch` never exits. CLAUDE.md's `rmap watch` invariant rewritten from the old "future `watch --json`" forward-reference to the shipped contract (event spellings, field names, `schema_version` bump rule).
- No `schema_version` bump for `tasks.toml` — additive flag; the `watch --json` event stream is a new contract surface versioned by its own `WATCH_SCHEMA_VERSION`.
- Tests: 3 new inline `#[cfg(test)]` unit tests in `src/watch.rs` (`render_event_line` with both outputs, with one output, `error_event_line`). No integration test — `watch_command` blocks indefinitely; consistent with Task 10's strategy. Manual smoke verified: one JSON line per real change, no-op silence, `error` event on broken TOML with the loop continuing, startup line on stderr.

---

## Repo reorganization: `tool_roadmap.md` → `DESIGN.md` + `ROADMAP.md` (2026-05-13)

**What was done:**
- `tool_roadmap.md` split into two files: `DESIGN.md` (design contract — schema example, CLI surface, invariants, deferred designs, out-of-scope, including the Phase 6 HTML render design) and `ROADMAP.md` (live phase tracking, rendered from the new `roadmap/tasks.toml`).
- rmap now dogfoods its own roadmap: `roadmap/tasks.toml` carries 13 open tasks across phases 6 / 7 / 13. `rmap render` writes `ROADMAP.md` + `roadmap/data.json` from that source. `rmap validate --check-render` passes; `rmap doctor` clean except one informational degenerate-bundle flag (`watch` covers all phase-7 tasks).
- Cross-references swept across `AGENTS.md`, `CLAUDE.md`, `README.md`, `SKILLS.md`, `dashboard_roadmap.md`, and `migration_roadmap.md` to point at `DESIGN.md` (contract) and `ROADMAP.md` (active work list) instead of `tool_roadmap.md`.

---

## Phase 13 — Skills-parity polish

### Phase 13 Task 23 (bug): `rmap depend` accepts numeric-only string target ids (delegate_parity bundle)

**What was done:**
- `rmap depend <src> on <target>` no longer rejects a target whose id is stored as a numeric-only string (e.g. `id = "1"`). Previously the mutator parsed `target` with `i64::from_str` and pushed an integer into `depends_on`; the validator then couldn't reconcile `TaskId::Number(1)` against `TaskId::Text("1")` and returned `unknown task 1`. Hit in the wild while running `rmap depend 18 on 7` in the `ccxt_ocx` project.
- Fix mirrors the target's actual storage shape: a new `target_id_shape(tasks, target)` scan in `add_dependency_str` finds the target task in the document and returns `TaskIdShape::Integer(i64)` or `TaskIdShape::Text`. The push then uses the matching value type. If the target isn't found, the existing `parse::<i64>()` fallback keeps the original behaviour so the validator still produces the canonical "unknown task" error post-write.
- Source-id format was never the trigger — only the target's was. The bug was local to `mutate.rs`; `query.rs` / `validate.rs` already key correctly on `TaskId`.
- Regression test `depend_command_handles_numeric_only_string_target_id` builds a fixture with `id = "1"` / `"2"` / `"2b"`, runs `rmap depend 2 on 1`, asserts the written `depends_on = ["1"]` (string form) and that `rmap validate` passes.
- No `schema_version` bump — bug fix; the storage shape (`Number | Text`) and validator behaviour are unchanged.

### Phase 13 Task 20: `rmap status` auto-fills lifecycle timestamps (schema_parity bundle)

**What was done:**
- `rmap status <id> done` now writes `done_at = today_iso()` when the field is absent; mirror for `rmap status <id> in_progress` / `started_at`. Closes the long-standing gap where a status flip left the file internally inconsistent (status changed, lifecycle timestamp missing). Observed downstream in `onchain_tempo` PR #1 — CodeRabbit flagged both `tasks.toml` and `data.json` for missing `done_at` after a status flip; gap is universal across consumers.
- Existing timestamps are never overwritten — re-flipping a `done` task, or moving `done → pending → done`, preserves the original `done_at` (audit trail). Other transitions (`pending`, `blocked`, `superseded`) touch no timestamps.
- Bulk form (`rmap status 1,2,3 done`) auto-fills each task independently inside the same atomic write — the existing per-task loop in `update_status_many_str` runs the new logic once per match.
- When a new timestamp is inserted, the affected task's keys are re-sorted into canonical order via `task.sort_values_by(canonical_task_key_index)` so the new field lands between `body` and `shipped_in` rather than being appended after multi-line entries. Mirrors the existing `update_markers_str` pattern: sort only on new-insert, never on idempotent calls (author-placed ordering for tasks that already declared the field is preserved).
- `today_iso()` is the single source of truth for "now" — same helper as score-decay rendering, `stale`, and `doctor`. `RMAP_TODAY` overrides for deterministic tests.
- Two new CLAUDE.md "Load-bearing invariants" bullets: (1) generalized the `rmap mark` auto-sort rule to also cover `rmap status`; (2) new bullet locking the auto-fill rule (which fields, which transitions, never-overwrite, today-resolution) as the agent contract.
- `created_at` and `scored_at` deliberately untouched — creation owns `created_at`; score edits own `scored_at`. Backfilling `done_at` on already-done legacy tasks is a separate doctor-fix task.
- No `schema_version` bump — the lifecycle-timestamp fields already exist in the schema; this only changes when the existing mutator path writes them.
- Tests: extended `status_command_updates_tasks_and_rerenders_outputs` to assert the auto-filled `done_at` lands in both `tasks.toml` and `data.json`. Four new tests: `status_in_progress_auto_fills_started_at`, `status_preserves_existing_done_at_on_reflip`, `status_pending_does_not_set_timestamps`, `status_bulk_auto_fills_each_task_independently`.

### Phase 13 Tasks 12 & 13: `rmap doctor` threshold CLI overrides (doctor_tuning bundle)

**What was done:**
- New `rmap doctor --threshold-days <N>` flag (`Option<u32>`): overrides the hardcoded 30-day cutoff for **both** the stale-in-progress check and the score-decay check. The flag name carries its unit, so it takes a bare integer (not a `7d`/`2w` duration string like `rmap stale --over`).
- New `rmap doctor --ac-threshold <N>` flag (`Option<u32>`): a single value that overrides **both** `AC_DIFFICULTY_THRESHOLD` (5) and `AC_BENEFIT_THRESHOLD` (8) for the missing-`acceptance_criteria` lint — when set, the predicate becomes `d >= N || b >= N`. Defaults stay distinct (5 / 8) when the flag is absent.
- New `DoctorThresholds { days, ac_difficulty, ac_benefit }` struct in `src/doctor.rs` with `resolve(threshold_days, ac_threshold)` applying the constant defaults. `DoctorReport::run` takes it as a parameter; the three threshold-sensitive call sites (`find_stale`, score-decay comparison, missing-AC predicate) read from it instead of module constants.
- `DoctorReport` gained a `thresholds` field — the effective thresholds are now echoed in the `rmap doctor --json` envelope, so an agent running with an override can interpret the findings. Additive JSON field; the `Display` impl reads the runtime values for its section headers.
- `STALE_THRESHOLD_DAYS` / `AC_*` constants stay as the defaults inside `resolve`; the now-unused `SCORE_DECAY_DAYS` import was dropped from `doctor.rs` (it still lives in `scoring.rs` for the render path).
- SKILLS.md Health section gained a `rmap doctor --threshold-days 60 --ac-threshold 6` fenced example; `skills_smoke.rs` locks the exit code.
- No `schema_version` bump — two additive optional flags + one additive JSON field; no `kind` string, render shape, or existing field touched.
- Tests: three new in `tests/cli.rs` (`doctor_command_threshold_days_suppresses_stale_and_decay`, `doctor_command_ac_threshold_changes_missing_ac_set`, `doctor_command_thresholds_in_json`).

### Phase 13 Task 17: `rmap next-bundle` selects one session-sized bundle (bundle_selector bundle)

**What was done:**
- New `rmap next-bundle` subcommand: picks one bundle worth of dep-satisfied pending tasks for a single Claude Code session. Flags: `--phase <N>` (override `[focus].phase`), `--bundle <name>` (force-pick, bypasses ranking), `--json`, `--tasks-path`. No D-budget subsetting (Option 1) — output is *every* actionable pending task in the chosen bundle.
- Ranking rule: `(in_focus_phase desc, sum_eff desc, bundle.order asc)`. Bundles with zero actionable tasks are skipped. Actionability is **broad**: a pending task T is actionable iff every dep is either `done` (anywhere) OR an in-bundle pending task that is itself broadly actionable (recursive, memoized DFS with belt-and-suspenders cycle defense — `validate` already rejects cycles).
- Force-pick (`--bundle X`) bypasses ranking entirely, including focus-phase preference and `--phase N` (when both passed, `--bundle` wins silently). Force-pick on a declared-but-zero-actionable bundle returns `Some` with empty tasks; force-pick on an undeclared bundle is a hard error (exit 1, stderr `bundle '<X>' is not declared in tasks.toml`).
- Tasks emitted in dep-topological order via Kahn's algorithm with stable Eff-desc tie-break.
- New `src/next_bundle.rs` carries the pure selector `pick(tasks, filter, today) -> Option<BundlePick>`. Reuses `query::matches_bundle` and `scoring::efficiency` — no duplicated dep-satisfaction logic. `today` parameter mirrors `next::next_tasks` for future score-decay-sensitive ranking; unused today.
- `src/export.rs` gained `export_bundle_pick_json_str(tasks, effective_focus, pick)` — JSON envelope `{ schema_version, focus_phase, bundle: {name, phase, description} | null, tasks: [ExportedTask, ...] }`. `focus_phase` is the **effective** focus (post-`--phase` override). Empty pick emits `bundle: null, tasks: []` in the same envelope (mirrors `rmap bundles --json` rather than `rmap next --json`).
- Human output: header `bundle <name>  phase <N> — <phase_name>  [<done>/<total>]  — <description>`, then one row per task using `format_task_row` (mirrors `rmap list`).
- Three empty-state spellings (stderr, exit 0): `none — no actionable bundle in any phase`, `none — no actionable bundle in phase N`, `none — bundle '<X>' has no actionable pending tasks`. Missing-bundle (`--bundle X` where X is not declared) is a hard error (exit 1).
- New CLAUDE.md "Load-bearing invariants" bullet locks the ranking rule, empty-state spellings, JSON envelope shape, no-subsetting rule, and force-pick precedence.
- SKILLS.md gained three fenced examples (`rmap next-bundle`, `rmap next-bundle --json`, `rmap next-bundle --bundle alpha`); `skills_smoke.rs` locks them.
- No `schema_version` bump — additive subcommand.
- Tests: 10 new in `tests/cli.rs` covering focus-phase-wins, tie-by-order, all-blocked-skipped, unmet-external-dep-skips, force-pick-bypass, JSON envelope shape, topo order, `--phase` override, empty-pick stderr+exit-0, force-pick-zero-actionable stderr, and missing-bundle exit-1. 7 in-module unit tests in `src/next_bundle.rs` covering the same matrix at selector depth.

### Phase 13 Task 16: `rmap bundles` selector listing (batch_selection bundle)

**What was done:**
- New `rmap bundles` subcommand: lists authoring-time `[bundles.*]` with per-bundle status counts and the next dep-satisfied pending task. Flags: `--phase <N>`, `--has-next`, `--in-focus`, `--json`, `--tasks-path`.
- New `src/bundles.rs`: `list_bundles(tasks, filter) -> Vec<BundleSummary>` + `BundleFilter { phase, has_next, in_focus }`. Per-bundle `next_task` reuses `next::next_task` with a bundle-restricted `TaskFilter` — single source of truth for dep-satisfaction.
- Human row shape `<name>    <done>/<total> <glyph_or_next>  — <description>` under per-phase headers `phase <N> — <phase_name>  [<phase_status>{, focus}]`. Five-branch `status_glyph_or_next` ladder: all-done → `✅`, in-progress only → `🚧`, all-blocked → `all-blocked ⛔`, pending-with-no-next → `pending:<n> (deps unmet) ⏸`, otherwise → `next:<id> [Eff:<x.y>] <tier_glyph>`.
- JSON envelope `{ schema_version, focus_phase, bundles }`; `focus_phase` serializes as explicit `null` when unset (deliberately not skipped). `status_counts` always zero-fills all four keys. `NextTaskSummary.eff` matches `ExportedTask.eff` rounding.
- New CLAUDE.md "Load-bearing invariants" bullet locks the row shape, status-glyph ladder, separator spacing, JSON envelope, and `next_task` reuse rule.
- SKILLS.md gained `rmap bundles`, `rmap bundles --in-focus --has-next --json` fenced examples; `skills_smoke.rs` locks them.
- No `schema_version` bump — additive read surface.

### Phase 13 Task 15: `rmap next --count N` returns top-N candidates (batch_selection bundle)

**What was done:**
- New `--count N` clap arg on `rmap next` (`NonZeroUsize`, default `1`). `--count 0` is rejected at clap parse time — the `NonZeroUsize` guard is part of the agent contract documented in `CLAUDE.md`.
- `src/next.rs` gained `next_tasks(tasks, filter, count) -> Vec<&Task>`: focus-phase candidates fill first (Eff desc, stable on ties to preserve TOML declaration order), and the selector falls through to non-focus candidates only when the focus pool has fewer than `count` entries. `next_task` is now a thin wrapper over `next_tasks(..., 1).into_iter().next()`; the old fold-based `highest_efficiency` was replaced with a stable `sort_by_eff_desc` so multi-count and single-count share one ranking rule. Behavior on count == 1 is preserved bit-for-bit (stable descending sort returns the same first-encountered task on Eff ties as the previous `fold` with `>=`).
- `src/export.rs` gained `export_tasks_array_json_str(&[&Task])` — emits a bare JSON array of `ExportedTask`, NOT the `ExportedTasks` envelope used by `rmap render` / `list --json` (no `schema_version` / `project` / `phases` / `bundles` wrapper). Documented invariant: array-branch shape is part of the agent contract.
- `src/main.rs` routes `count == 1` through the old `export_task_json_str` path (bare object or `null` — byte-identical to pre-Task-15 releases for the default invocation) and `count > 1` through the new array helper. Human output prints one line per task in both cases (zero lines when nothing eligible).
- New CLAUDE.md "Load-bearing invariants" block locks the JSON shape split: `--count 1` bare object/null, `--count >1` array (possibly empty). Renaming the flag, changing the default, dropping the `NonZeroUsize` guard, or flipping `--count 1` to always emit an array is a `schema_version` bump.
- SKILLS.md gained a `rmap next --count 3 --bundle alpha --json` fenced example; `skills_smoke.rs` locks the exit code.
- No `schema_version` bump — additive flag, default-invocation contract preserved.
- Tests: five new in `tests/cli.rs` (`next_command_count_one_default_emits_bare_object_json`, `next_command_count_three_json_emits_eff_ranked_array`, `next_command_count_exceeds_eligible_returns_min_without_error`, `next_command_count_three_human_prints_one_line_per_task`, `next_command_count_zero_is_rejected_at_parse_time`) and four new in `tests/next.rs` (singleton-matches-`next_task`, count==3 Eff-ranked, exceeds-eligible-returns-min, focus-phase-fills-before-other-phases).

### Phase 13 Task 14: `rmap list/next --bundle <name>` filter (batch_selection bundle)

**What was done:**
- New `--bundle <name>` clap arg on `rmap list` and `rmap next`. `TaskFilter` (in `src/query.rs`) gained a fourth `bundle: Option<String>` field; `list_tasks` chains a fourth `matches_bundle` predicate alongside the existing status/marker/phase filters.
- `next_task` switched from `(tasks, marker: Option<&str>)` to `(tasks, &TaskFilter)`, sharing the filter struct with `list_tasks`. Honors `filter.marker` and `filter.bundle` only — `filter.status` is ignored (next is implicitly `pending`-only) and `filter.phase` is ignored (focus is sourced from `[focus].phase`, not the user). Bundle filter applies BEFORE the focus-phase partition so `--bundle X` restricts the candidate pool first and focus only ranks within the bundle (AC #3: bundle wins over focus when both explicitly set). `matches_marker` and `matches_bundle` are now `pub(crate)` in `query.rs` so `next.rs` reuses the same predicates instead of duplicating them.
- Unknown bundle name returns empty (`list --bundle nonexistent --json` → empty `task` array; `next --bundle nonexistent --json` → `null`) without erroring — same lenient stance as `--marker`. We do NOT validate the user-supplied `--bundle` against known bundle keys.
- `src/render.rs::up_next_line` updated to pass `&TaskFilter::default()` (was `None`); behavior unchanged.
- No `schema_version` bump — additive flag, `bundle` was already in `ExportedTask`, `Task.bundle` was already required.
- Tests: five new in `tests/cli.rs` (`list_command_filters_by_bundle`, `next_command_filters_by_bundle`, `next_command_composes_bundle_and_marker`, `next_command_unknown_bundle_returns_null_json`, `list_command_unknown_bundle_returns_empty_envelope`). Existing `tests/next.rs` call sites threaded through a small `marker_filter` helper. `tests/query.rs` literal `TaskFilter` construction extended with `bundle: None`. SKILLS.md gained two fenced examples (`rmap list --bundle alpha`, `rmap next --bundle alpha`) — `skills_smoke.rs` locks them in.

### Phase 13c: `Task::files_to_modify` field (delegate_parity bundle)

**What was done:**
- New optional `files_to_modify: Vec<String>` field on `schema::Task`, mirroring the `out_of_scope` / `acceptance_criteria` shape (`#[serde(default)]`, no `skip_serializing_if` on the schema struct, no `schemars` per-field attribute). Existing `tasks.toml` files parse unchanged — the field is invisible when absent.
- Three-place edit per the documented invariant: `src/schema.rs` (field), `src/diff.rs::diff_fields!` (so `rmap diff` notices add/remove/change), `src/export.rs::ExportedTask` (so `data.json` / `show --json` / `list --json` / `next --json` surface the field additively with `skip_serializing_if = "<[_]>::is_empty"`). Deliberately NOT added to `TASK_VERBOSE_WHITELIST` — paths are short strings individually, but a non-trivial `files_to_modify` list would bloat verbose diff payloads alongside `out_of_scope` / `acceptance_criteria` / `body`; the changed-field name itself is enough signal for the agent contract.
- `rmap delegate` is the only human-readable surface: new `append_files_to_modify` function renders a `## Files to modify` section between `## Acceptance criteria` and `## Out of scope`. Reads naturally as "what (AC) → where (files) → not where (out_of_scope)". Bullet shape is plain `- {path}`, NOT the `- [ ] {item}` checkbox form used by acceptance criteria — file paths are scope hints, not todos. Empty `Vec` → section omitted entirely. `ROADMAP.md` TASKS table, FOCUS block, MERMAID gantt, `rmap show` stdout, and `rmap list` stdout are unchanged (and the `validate --check-render` gate stays clean by construction).
- No `schema_version` bump — additive field, no breakage for existing consumers.
- Tests: `tests/delegate.rs` extended with a non-empty `files_to_modify` fixture (positive assertions on section header, plain-bullet rendering, no-checkbox guard) and the minimal fixture (negative assertion that the section is absent). `tests/export.rs` asserts the JSON array shape when set and the `skip_serializing_if` behavior when empty. `tests/diff.rs::verbose_emits_before_after_for_whitelisted_changed_fields` extended to confirm `files_to_modify` surfaces in `changed_fields` but is omitted from the `values` whitelist payload. Existing golden fixtures, `roundtrip`, and `skills_smoke` pass unchanged. The `Task` struct literal in `src/stale.rs::tests::make_task` updated to include the new field.

### Phase 13c: `Task::out_of_scope` field (delegate_parity bundle)

**What was done:**
- New optional `out_of_scope: Vec<String>` field on `schema::Task`, mirroring the `acceptance_criteria` shape (`#[serde(default)]`, no `skip_serializing_if` on the schema struct, no `schemars` per-field attribute). Existing `tasks.toml` files parse unchanged — the field is invisible when absent.
- Three-place edit per the documented invariant: `src/schema.rs` (field), `src/diff.rs::diff_fields!` (so `rmap diff` notices add/remove/change), `src/export.rs::ExportedTask` (so `data.json` / `show --json` / `list --json` / `next --json` surface the field additively with `skip_serializing_if = "<[_]>::is_empty"`). Deliberately NOT added to `TASK_VERBOSE_WHITELIST` — `out_of_scope` mirrors `acceptance_criteria` and `body` (free-form string arrays would bloat verbose diff payloads without aiding decisions).
- `rmap delegate` is the only human-readable surface: new `append_out_of_scope` function renders a `## Out of scope` section between `## Acceptance criteria` and `## Environment notes`. Bullet shape is plain `- {item}`, NOT the `- [ ] {item}` checkbox form used by acceptance criteria — out-of-scope items are guardrails, not todos. Empty `Vec` → section omitted entirely. `ROADMAP.md` TASKS table, FOCUS block, MERMAID gantt, `rmap show` stdout, and `rmap list` stdout are unchanged (and the `validate --check-render` gate stays clean by construction).
- Dogfooded on Task 6 itself: the task ships with `out_of_scope` listing what was deliberately deferred (`NewTaskFields` extension, ROADMAP rendering, `schema_version` bump).
- No `schema_version` bump — additive field, no breakage for existing consumers.
- Tests: `tests/delegate.rs` extended with a non-empty fixture (positive assertions on section header, plain-bullet rendering, no-checkbox guard) and the minimal fixture (negative assertion that the section is absent). `tests/export.rs` asserts the JSON array shape when set and the `skip_serializing_if` behavior when empty. `tests/diff.rs::verbose_emits_before_after_for_whitelisted_changed_fields` extended to confirm `out_of_scope` surfaces in `changed_fields` but is omitted from the `values` whitelist payload. Existing golden fixtures, `roundtrip`, and `skills_smoke` pass unchanged. The `Task` struct literal in `src/stale.rs::tests::make_task` updated to include the new field.

### Phase 13b: D/B/U 1..=10 range validation

**What was done:**
- New semantic check `validate::validate_scores` enforces `1..=10` on each of `task.scores.{d,b,u}`, matching the D/B/U rubric documented in `task-prioritization.md`. The check runs in both the short-circuiting (`validate_tasks_str`) and accumulating (`collect_findings`) pipelines so `rmap validate` and `rmap doctor` both surface it. Stops at the first offender per task (matches the existing per-validator shape); accumulation across distinct validators continues to work as before via `collect_findings`.
- Error message: `task <id> scores.<field> = <value> must be in 1..=10`. The `1..=10` substring is the agent-grep contract — bumping the range would be a `schema_version` change. Line locator uses the full inline-table spelling `scores = { d = D, b = B, u = U }` so the reported `path:line` lands on the row carrying the bad value, not the first matched `scores` line.
- Range constants `MIN_SCORE = 1` / `MAX_SCORE = 10` live next to the other `VALID_*` constants in `validate.rs`. `Scores` itself stays `u32`-per-field — no schema migration — because TOML rejects negatives at parse time and `10` fits comfortably in `u32`; the range check enforces the application-level rubric on top of the type-level non-negativity.
- Tests: `rejects_score_below_minimum` (d = 0), `rejects_score_above_maximum` (b = 11), `accepts_scores_at_range_edges` (both 1 and 10 are valid). Line-number asserts pin the locator behavior so a future refactor that re-locates the error reporter has to also update the fixture-line expectations — that's intentional.

### Phase 13a: render polish — Eff tier glyph + phase archive collapse

**What was done:**
- Centralized the Eff tier rubric as `scoring::tier_glyph(eff: f64) -> &'static str` — `>= 2.0 → 🎯`, `>= 1.5 → 🚀`, `>= 1.0 → 📋`, else `⚠️`. Single source of truth; the previous file-local `render::priority_symbol` is gone. Glyph rendering wired into every Eff surface: TASKS-table row (placement unchanged), FOCUS Up next line (`**Up next:** Task C — <title> [D/B/U → Eff] <tier_glyph>`), `rmap show` scores line, `rmap list` row, `rmap next` row, `rmap delegate` scores line. JSON payloads (`data.json`, `show --json`, `list --json`, `next --json`, `diff --json`) stay numeric — `format_efficiency` is untouched and the glyph is a pure render-time concatenation.
- Phase archive collapse: when `[phases.N].status = "done"`, `render_phase_table` short-circuits and replaces the body of the matching `<!-- TASKS:BEGIN phase=N -->` block with a single line `> N tasks. See [CHANGELOG.md](CHANGELOG.md#phase-N-<slug>).`. Slug = `phase_slug(name)` (lowercased, split on `!c.is_alphanumeric()`, empty segments dropped, joined with `-`), so a phase named `"Skills-parity polish"` collapses to `phase-N-skills-parity-polish`. Marker boundaries stay byte-preserved; per-phase scoped (non-done phases in the same file still render the full task table).
- Two new golden fixtures: `tests/golden/eff_tier/` (all four tier ranges in one TASKS table) and `tests/golden/phase_archive_collapse/` (done phase collapses, adjacent in-progress phase renders normally). Existing fixtures `focus_block` (Up next glyph) and `mermaid_block` (phase 11 done → collapse line) updated to match the new shape.
- New invariants in `CLAUDE.md`: tier glyph centralized in `scoring::tier_glyph` with the four-tier rubric as the agent-grep contract; archive collapse line shape `> N tasks. See [CHANGELOG.md](CHANGELOG.md#phase-N-<slug>).` is agent-grep + `validate --check-render` locked-in. The existing FOCUS invariant amended to include the trailing tier glyph on Up next.

---

## Phase 11b (correction): Codex environment notes — distinguish hex.pm gap from a general no-HTTP claim

**What was done:**
- `delegate.rs::append_agent_notes` Codex footer rewritten. Old bullet "No external HTTP: hex.pm, crates.io, npm, RFCs, vendor docs not reachable." was Elixir-shaped and overstated reach for other ecosystems. Per OpenAI's Codex Cloud docs ([Agent internet access](https://developers.openai.com/codex/cloud/internet-access)), network is environment-configurable: default offline, but the "Common dependencies" preset reaches crates.io / npmjs / PyPI and ~70 dev domains. **hex.pm is NOT in the common preset** — and the default image has no Elixir/Erlang/mix, so Elixir/Erlang is the actual special case Codex hasn't fixed. New bullets call out: (a) reach varies by config and the common-preset coverage, (b) toolchain absence for Elixir/Erlang specifically, (c) try-before-trust + fall back to in-prompt context, (d) reviewer-runs-the-harness.
- Test update in `tests/delegate.rs`: codex/cursor/claude footer assertions switched from the now-removed "No external HTTP" string to the stable "Network access varies" + "hex.pm" pair.
- Source-of-truth follow-up: `~/.claude/includes/cloud-agent-environments.md` (user-scope, not in this repo) still carries the older Elixir-shaped "No external HTTP" framing and needs the same hedge applied by hand.

---

## Phase 11b (close-out): mermaid render + `mark` canonical-position helper

**What was done:**
- Added a third optional marker pair `<!-- MERMAID:BEGIN -->` / `<!-- MERMAID:END -->` in `ROADMAP.md`. When present, `rmap render` populates the body with a fenced ```` ```mermaid ```` `gantt` block: one `section` per phase (sorted by `phases[*].order`), one row per task with `started_at`. Row shape by status: `done` → `:done, started_at, done_at`; `in_progress` → `:active, started_at, today`; `blocked` → `:crit, started_at, today`. Pending tasks (and tasks without `started_at`) are omitted — mermaid gantt requires dates. Empty-state body collapses to a minimal valid gantt with `%% no tasks with started_at yet`. Title colons / commas / semicolons (`:`, `,`, `;`) are rewritten to em-dash because mermaid parses `task :status, start, end` (`,` is the field separator; `;` is reserved for `dateformat`/etc.). Zero-config default — absence of the markers renders no mermaid block. The pass uses the same byte-preservation rule as TASKS/FOCUS and is automatically covered by `validate --check-render`.
- Closed Phase 12c.4: `rmap mark <id> +<marker>` on a task that didn't have a `markers` field used to append the field at the end of the task block (after multi-line entries like `acceptance_criteria`). `update_markers_str` now sorts the affected task's keys into canonical order via `task.sort_values_by(canonical_task_key_index)` immediately after the new-key insert, so the new field lands between `scores` and `depends_on`/`acceptance_criteria`. Idempotent ops and ops on a pre-existing `markers` field do NOT trigger the sort — author-placed ordering is preserved when the field already exists. `add_dependency_str` is intentionally not auto-sorted (helper is `mark`-only until a second consumer asks).
- Three new golden fixtures: `tests/golden/mermaid_block/` (populated gantt with mixed statuses), `mermaid_block_no_markers/` (zero-config no-op), `mermaid_block_no_dates/` (empty-state body). `tests/skills_fixture/ROADMAP.md` gained MERMAID markers so the existing `skills_smoke` suite exercises the new pass on every documented `rmap render` invocation.
- Three new mutate tests cover the canonical-position rule (new-insert reorders, idempotent add is byte-equal, existing-markers add preserves author order) plus a black-box CLI test (`mark_command_places_new_markers_field_in_canonical_position`).
- New invariants in `CLAUDE.md`: third marker pair (MERMAID byte-preservation + body-shape contract), `mark` canonical-position rule (`mark` auto-sorts on new-key insert; other mutators do not).
- `README.md` and `SKILLS.md` updated with the third marker pair.

**Phase 11b is now complete.** Remaining roadmap bets are Phase 6 (HTML render) and Phase 7 (`rmap watch`).

---

## Phase 11b (continued): `rmap new` + `rmap new --from-stdin`

**What was done:**
- Added `rmap new --from-stdin` — reads one-or-more `[[task]]` blocks as TOML from stdin and appends them to `tasks.toml`. The id is auto-allocated (numeric `max + 1`) when omitted; explicit ids that collide with an existing task are rejected as `DuplicateId`. Multi-task fragments are atomic: any validation failure (unknown phase, unknown bundle, cycle, duplicate id) aborts the entire batch with the file byte-equal to its pre-call state.
- Added interactive `rmap new` (no `--from-stdin`) — `dialoguer`-driven prompt flow (phase → bundle → title → D/B/U → markers → acceptance criteria → assignee → linear_id → module). Requires a TTY; non-interactive contexts must use `--from-stdin`. Bundles cannot be created on the fly — the user authors `[bundles.<name>]` in `tasks.toml` first so bundle metadata (`order`, `description`) stays author-visible.
- Both front-ends share `mutate::add_task_str(path, input, &NewTaskFields)` and the validate-then-write contract. Lifecycle timestamps (`started_at`, `done_at`, `blocked_reason`, `shipped_in`) are NOT settable on creation — `rmap status` owns those transitions. `created_at` and `scored_at` default to `today_iso()` when omitted.
- `tests/skills_smoke.rs` extended with a `# stdin: <<EOF` heredoc annotation so `SKILLS.md`-documented commands that read stdin (`rmap new --from-stdin`) are gated on their declared exit codes alongside the rest of the agent contract.
- `SKILLS.md` gained a "Creating tasks" section covering the happy path and the atomic-batch-failure case.

**Deferred to Phase 11b's remaining polish:** mermaid render.

---

## Phase 11b (continued): delegate per-agent footer + diff verbose

**What was done:**
- `rmap delegate` now emits a per-agent `## Environment notes` section between acceptance criteria and the standard instructions. The bullets are target-specific (Codex: no external HTTP / no project runtime; Cursor: full network, run-the-harness-green pre-PR, asdf-shim gotcha; Claude: local execution, report actual output). Content mirrors `~/.claude/includes/cloud-agent-environments.md` — that skill is the source of truth and is kept in sync by hand.
- `DelegateTarget` moved from `main.rs` into `delegate.rs` and `format_delegate_prompt` now takes the enum, not `&str`. Closes the silent-no-op risk if a new target variant is added: matching on the enum forces every branch to be considered.
- `rmap diff --verbose` adds per-field before/after on whitelisted Changed entries. Task whitelist excludes `body`, `acceptance_criteria`, and `cross_repo` (long / multi-table content would dominate the output); metadata whitelist excludes `phases.*` and `bundles.*` (already key-granular). Human output indents the before→after lines under each Changed entry; JSON output gains a `values: [{field, before, after}]` array per Changed entry. Added/Removed entries leave `values` as `None` (skipped on serialize).
- Without `--verbose`, `rmap diff` output is byte-identical to prior releases — agent contract additivity is preserved.
- New invariants in `CLAUDE.md`: whitelist sync discipline (when extending `Task`, decide on whitelist membership), diff verbose additivity (no `values` key when verbose is off), delegate footer manual-sync rule (skill include is source of truth).

**Deferred to Phase 11b's remaining polish:** mermaid render, `rmap new --from-stdin`, interactive `rmap new`.

---

## Phase 11b (partial): mark + depend mutators

**What was done:**
- Added `rmap mark <id> +marker -marker ...` — appends or drops markers atomically on a single task. Idempotent (add of an existing marker / remove of an absent marker are no-ops). Each token must start with `+` or `-`; clap's `allow_hyphen_values` lets the parser accept `-marker` without treating it as a flag, so named flags (`--tasks-path`, `--roadmap-path`, `--data-path`) precede the positional ops on the command line.
- Added `rmap depend <id> on <other-id> [--cross-repo <repo>:<task_id>[:<relation>]]` — appends in-repo and/or cross-repo dependencies. Relation defaults to `blocks` when omitted from the `--cross-repo` spec. Idempotent on the `(repo, task_id, relation)` triple for cross-repo entries and on the target id for in-repo. At least one of `on <id>` or `--cross-repo` must be present.
- Both mutators route through `toml_edit::DocumentMut` (preserving comments + formatting), re-validate the full file after edit, and refuse to write on any validation error — cycles, unknown ids, invalid markers, and invalid cross-repo relations all abort with the file byte-equal to its pre-mutation state.
- Both mutators re-render `ROADMAP.md` and `roadmap/data.json` on success, mirroring the `status` flow.
- `MutateError` gained `EmptyOps`, `InvalidMarkerOp`, `NoDependency`, and `InvalidCrossRepoSpec` variants. `MarkerOp` and `CrossRepoSpec` are public so the binary can parse positional tokens upfront and surface errors before any file I/O.

**Deferred to Phase 11b's remaining polish:** mermaid render, delegate per-agent footer parameterization, `rmap new --from-stdin`, interactive `rmap new`, `rmap diff --verbose`.

---

## Phase 11a: Health surfaces + cleanup

**Completed** | Phase 11a (Phase 11b still ⬜)

**What was done:**
- Added `rmap stale --over <duration>` — lists in-progress tasks whose `started_at` is older than the given duration (`7d`, `2w`, `30d`, `1y`). Pure selector in `src/stale.rs`; non-JSON and `--json` envelopes follow the existing `list` pattern.
- Added `rmap doctor` — composite informational report aggregating validation findings, stale in-progress tasks (>30d), score-decay candidates (`scored_at` missing or >30d ago), and render drift. Always exits 0 — `doctor` is the soft-signal aggregator; the strict gates remain `rmap validate` (exit 1) and `validate --check-render` (exit 2). `DoctorReport` serializes to a stable `{ ok, findings[] }` envelope keyed by `kind`.
- Added score-decay rendering. Tasks with `scored_at` missing or >30 days old now render with a `?` suffix on the `Eff` cell (e.g. `Eff:1.33?`). Threshold lives in `score_decay_suffix` in `src/scoring.rs`.
- Added bulk `rmap status 1,2,3 done`. Single-ID syntax is unchanged; comma-separated lists run atomically — all-or-nothing on file write, with validation re-run once at the end. New `update_status_many_str` in `src/mutate.rs`; the existing `update_status_str` is now a thin wrapper.
- Added pure-Rust date math in `src/scoring.rs` (`days_from_civil`, `days_since`) using the Howard Hinnant algorithm. No new dependencies. Reused by `stale`, `doctor`, and score-decay rendering.
- Added `today_iso()` helper in `src/lib.rs` that respects the `RMAP_TODAY` environment variable for deterministic tests, otherwise reads the system clock. Golden fixtures opt in by dropping a `today.txt` next to their `tasks.toml`.
- Removed the `--json` flag from `rmap schema`. It was a hidden no-op since Phase 8–9; `rmap schema` always emitted JSON. `rmap schema --json` now errors out with clap's unknown-argument message.
- Added `pub fn collect_findings(tasks, path, input) -> Vec<ValidateError>` to `src/validate.rs`. Each of the 12 existing semantic checks runs independently and accumulates errors instead of short-circuiting. `validate_tasks_str` is unchanged.

**Deferred to Phase 11b:** mermaid render, delegate per-agent footer parameterization, `rmap new --from-stdin`, `rmap mark`, `rmap depend`, interactive `rmap new`, `rmap diff --verbose`.

---

## Phase 10 audit fixes

**What was done:**
- `data.json` now surfaces the top-level `[focus]` table. `export::ExportedTasks` previously omitted it, so consumers of `rmap export json` and `rmap render` couldn't see which phase the project was focused on.
- `rmap diff` now detects `[focus]` add/remove/change between two `tasks.toml` revisions. `diff::diff_metadata` previously walked `linear`/`phases`/`bundles` but not `focus`. Added `diff_optional` helper for any future top-level `Option<T>` fields.
- Tightened the timestamp-validation error message: `"must match YYYY-MM-DD format (4-digit year, 2-digit month, 2-digit day)"` instead of `"must be an ISO-8601 date"`. The validator is shape-only by design (per CLAUDE.md) — the previous wording falsely implied semantic validation.
- CLAUDE.md gains a matching invariant: when adding a top-level `Tasks` field, edit `diff::diff_metadata` and `export::ExportedTasks` in the same commit. Phase 10's `focus` field was the case that surfaced the gap.

---

## Phase 10: Schema Completeness

**Completed** | Phase 10

**What was done:**
- Added top-level `[focus]` table with `Tasks.focus: Option<Focus>`. `rmap next` now prefers tasks in the focus phase before falling back to other phases; highest Eff still wins inside each tier.
- Added optional task timestamps: `created_at`, `started_at`, `done_at`, `scored_at`. Validated as ISO-8601 dates (`YYYY-MM-DD`); presence is what unlocks future decay / stale / recently-shipped surfaces.
- Added `blocked_reason: Option<String>`. Required whenever `status = "blocked"` — validation fails with a clear error otherwise.
- Added dependency cycle detection. DFS over `task.depends_on` reports the cycle members (e.g. `1 -> 2 -> 1`) and the source line of the offending `depends_on`.
- All new task fields wired through the three-place contract: `schema::Task`, the `diff_fields!` invocation in `diff::changed_fields`, and `export::ExportedTask`.

---

## Phase 8–9 Polish (review fixes)

**What was done:**
- `diff::changed_fields` now uses a local `diff_fields!` macro so the per-field comparison list lives in one declaration instead of thirteen `if`-blocks. Reduces the risk of forgetting to add a new `Task` field; CLAUDE.md carries the matching invariant note.
- `rmap schema` always emits JSON. `--json` is accepted as a hidden no-op for backward compatibility — `rmap schema` and `rmap schema --json` produce identical output.
- `rmap delegate` no longer prints a redundant `Stored assignee:` line when the stored assignee equals the `--to` target. When they differ, the line is suffixed with `(overridden)` so the receiving agent sees the routing intent at a glance.
- `delegate.rs` `writeln!(prompt, …).expect("write to string")` boilerplate replaced with a module-level `line!` macro; behavior unchanged, signal-to-noise improved.

---

## Phase 9: Cloud Delegation Surface

### Delegate Command
**Completed** | Phase 9

**What was done:**
- Added optional `assignee` and `acceptance_criteria` task fields.
- Added validation for `assignee` values.
- Added `rmap delegate <id> --to claude|codex|cursor` as a pure read-only command that emits structured Markdown for cloud agents.
- Included the new fields in agent JSON exports as additive fields.
