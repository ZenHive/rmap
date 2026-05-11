# Changelog

Completed roadmap tasks. For upcoming work, see [tool_roadmap.md](tool_roadmap.md).

---

## Phase 9: Cloud Delegation Surface

### Delegate Command
**Completed** | Phase 9

**What was done:**
- Added optional `assignee` and `acceptance_criteria` task fields.
- Added validation for `assignee` values.
- Added `rmap delegate <id> --to claude|codex|cursor` as a pure read-only command that emits structured Markdown for cloud agents.
- Included the new fields in agent JSON exports as additive fields.
