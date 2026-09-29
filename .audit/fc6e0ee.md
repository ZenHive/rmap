# Audit fc6e0ee

Integrated revision: `fc6e0ee6ac5ac237ec6dd50dbe8b026c22349bc7`.
Range: `f9e6feb70dc989c4ef9e858bbeafa463db4a42f6..fc6e0ee6ac5ac237ec6dd50dbe8b026c22349bc7`.

Reviewed the capability-spec schema, parsing, validation, creation/export/diff/query/delegate surfaces; doctor tag scanning and pruning; delegate context/check sections; shared portfolio alias lookup and browser regression; tests, documentation, generated roadmap outcomes and removal/ignoring of agent-local artifacts. No dead code, leftover debug output or actionable code defect found in this pass. No reviewer rejections were supplied, so no false-rejection finding.

## Findings and fixes

1. README omitted `specs`, `list --rule`, spec registrations/deltas and doctor tag configuration. Added them and linked the detailed manual.
2. README still advertised the removed `schema --json` flag. Corrected it to `schema`, checked against CLI help and the existing rejection regression.
3. DESIGN omitted the portfolio `repo_aliases` contract and said “No npm” despite the new browser-test package. Documented unchanged project envelopes, shared case-insensitive aliases, first-input collision handling and unresolved references; clarified that the output has no npm runtime dependency.
4. CHANGELOG has no entries for tasks 62 and 64 (versions 0.9.0/0.9.1). Left it untouched under the operational prohibition; filed **task 65** with `rmap new --from-stdin --tasks-path <this worktree>/roadmap/tasks.toml`, assigned to codex / gpt-6-astra. Inspected existing tasks before filing; the original feature tasks are done and no pending release-note repair duplicates this work. The initial creation attempt rejected a missing required `bundle` without writing; corrected the fragment to `bundle = "spec_layer"` and creation succeeded. The task and generated data are included under the audit-specific discovery-filing instruction.

Four findings; three fixed; one tracked. Only documentation and discovery metadata changed. No runtime code or dependency changes, deployments, server operations or merge reversals.

## Full-project QA

Before audit edits, verified clean status and exact HEAD, then ran:

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

- `cargo fmt --check`: passed, no output.
- `cargo clippy --all-targets -- -D warnings`: passed, no warnings; cold dev build completed in 13.46 seconds.
- `cargo test`: passed through doc-tests. All reported suites had zero failures and zero ignored tests. The nonempty suites were library unit tests, cli, delegate, diff, export, field_registry, import, mutate, next, paths, query, render, roundtrip, skills_smoke, spec_coverage, specs and validate. Binary and doc-test suites were empty. In particular cli passed 224 tests, delegate 18, spec_coverage 5 and specs 7. This covers malformed/missing specs, prefix ownership, terminal history, atomic mutation refusal, git-snapshot diff, tag syntax/configuration, alias collisions and existing CLI behavior.
- Inspected Cargo.toml, the configured `.github/workflows` Rust workflow and project command guidance: no configured coverage or clone-analysis command. Dialyzer, Reach, Sobelow, Credo and Elixir Doctor are inapplicable to this Rust project. `rmap doctor` is a roadmap advisory command, covered by the Rust suite, not a Rust static analyzer.

Additional integrated-revision check, before audit edits, from `tests/browser`:

```sh
npm ci && npx playwright install chromium && npm test
```

All three commands completed successfully. The test builds this worktree's rmap and opens temporary local HTML in Chromium, with no server or database. Its assertions cover preserved project envelopes, relation cords, clickable project/label/basename/special aliases, collisions, unresolved tasks and absence of browser page errors. Combined tool output was truncated; npm's retained logs independently record exit 0 for installation, browser installation and test execution: `2026-09-29T10_09_15_460Z-debug-0.log`, `2026-09-29T10_09_15_923Z-debug-0.log`, and `2026-09-29T10_09_16_398Z-debug-0.log`. Each records this worktree's `tests/browser` cwd, Node v26.7.0, npm v11.19.0 and `info ok`. No check was rerun merely for presentation.

QA judgment: **passed** at the original integrated revision. Every configured check completed, and the added browser boundary check also completed. No failed/incomplete QA repair task is needed.

## Artifact verification

- `target/debug/rmap validate --check-render`: `valid` after task 65 creation.
- `target/debug/rmap schema --help`, `specs --help`, and `list --help`: confirmed documented command shapes.
- README link target `SKILLS.md#capability-specs` exists.
- `git diff --check`: clean.

The audit report and fixes are committed together. `.harness/audit.json` is harness-internal and excluded from the commit.
