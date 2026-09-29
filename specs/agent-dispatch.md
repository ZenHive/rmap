# Agent dispatch contract

These rules govern roadmap validity, selection and status write-backs used by harness.

DISPATCH-1: Roadmaps require schema_version = 2; unsupported versions fail validation with a migration hint.

DISPATCH-2: A pending or in_progress task with a non-human assignee requires a non-empty model. Done, superseded, blocked, human-assigned and unassigned tasks are exempt. The rejection contains missing model.

DISPATCH-3: A pending or in_progress task with a non-human assignee requires a non-empty acceptance_criteria list whose entries are all non-blank. Done, superseded, blocked, human-assigned and unassigned tasks are exempt. The rejection contains missing acceptance_criteria; validation does not judge semantic quality.

DISPATCH-4: Each D/B/U score must be in 1..=10 inclusive; out-of-range diagnostics contain must be in 1..=10.

DISPATCH-5: next and ready share the ranking key: focus-and-active-milestone, focus-only, active-milestone-only, neither; within each tier Eff descending, then unlocks descending, then stable TOML order. Any active milestone qualifies. Without focus every task counts as in focus; without an active milestone ranking reduces to focus then Eff and unlocks.

DISPATCH-6: ready returns pending tasks whose dependencies are all done, in the shared ranking order, with all eligible tasks returned when count is omitted. Its JSON output without --fields is a list-shaped envelope. Pending tasks in this set cannot depend on each other.

DISPATCH-7: ready --dispatchable excludes tasks marked handbuild from the otherwise eligible ready set; ready without this flag includes them.

DISPATCH-8: status <id> in_progress persists the status and fills started_at with today's date when absent, preserving an existing started_at. It does not set done_at.

DISPATCH-9: status <id> in_progress --landing-ref <ref> persists landing_ref, including on an already in_progress task without changing started_at. An empty ref clears the field. Passing --landing-ref on another target status fails without writing. Without the flag, done and blocked retain landing_ref, while pending clears it.

DISPATCH-10: status <id> done --verified --verified-by <reviewer> --verification-ref <evidence> --shipped-in <ref> persists verified = true, verified_by, verification_ref and shipped_in together. The done task must have non-empty implemented, supplied via --implemented or already stored. --verified requires a non-empty --verified-by; omission fails without writing.

DISPATCH-11: status <id> blocked --reason <text> persists blocked_reason, overwriting an existing reason. Leaving blocked clears blocked_reason; --reason on a non-blocked target is ignored with a stderr warning.
