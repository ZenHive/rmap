# Changelog

Completed roadmap tasks. For upcoming work, see [tool_roadmap.md](tool_roadmap.md).

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
