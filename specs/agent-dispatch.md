# Agent dispatch contract

These rules govern roadmap validity and selection before harness dispatches work.

DISPATCH-1: Roadmaps require schema_version = 2; unsupported versions fail validation with a migration hint.

DISPATCH-2: A pending or in_progress task with a non-human assignee requires a non-empty model. Done, superseded, blocked, human-assigned and unassigned tasks are exempt. The rejection contains missing model.

DISPATCH-3: A pending or in_progress task with a non-human assignee requires a non-empty acceptance_criteria list whose entries are all non-blank. Done, superseded, blocked, human-assigned and unassigned tasks are exempt. The rejection contains missing acceptance_criteria; validation does not judge semantic quality.

DISPATCH-4: Each D/B/U score must be in 1..=10 inclusive; out-of-range diagnostics contain must be in 1..=10.

DISPATCH-5: next and ready share the ranking key: focus-and-active-milestone, focus-only, active-milestone-only, neither; within each tier Eff descending, then unlocks descending, then stable TOML order. Any active milestone qualifies. Without focus every task counts as in focus; without an active milestone ranking reduces to focus then Eff and unlocks.

DISPATCH-6: ready returns pending tasks whose dependencies are all done, in the shared ranking order, with all eligible tasks returned when count is omitted. Its JSON output is a list-shaped envelope. Pending tasks in this set cannot depend on each other.
