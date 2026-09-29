//! Single declaration of persisted task fields, in schema/export/diff order.
//! Each entry records canonical TOML rank, diff/verbose participation, export
//! omission policy, and creation input/borrowed types + conversion/writer policy.
//! An empty creation bracket means a transition-time field.

#[macro_export]
macro_rules! task_fields {
    ($callback:ident) => {
        $callback! {
            id: TaskId => [0, false, false, always] [Option<TaskId>; Option<u32>; id; skip];
            phase: u32 => [1, true, true, always] [u32; u32; copy; number];
            bundle: String => [2, true, true, always] [String; &'a str; string; required];
            /// Repository where this task's own work lands. When omitted, consumers
            /// use the roadmap's top-level `project`. This is distinct from
            /// `cross_repo`, which links this task to related tasks in other roadmaps.
            target_repo: Option<String> => [3, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            milestone: Option<String> => [4, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            status: Status => [5, true, true, always] [Option<String>; &'a str; status; status];
            title: String => [6, true, true, always] [String; &'a str; string; required];
            scores: Scores => [7, true, true, scores] [Scores; (u32, u32, u32); scores; scores];
            #[serde(default)]
            markers: Markers => [8, true, true, always] [Vec<String>; &'a [&'a str]; strings; array];
            #[serde(default)]
            depends_on: Vec<TaskId> => [9, true, true, always] [Vec<TaskId>; &'a [u32]; deps; numbers];
            linear_id: Option<String> => [17, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            assignee: Option<String> => [16, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            module: Option<String> => [18, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            branch: Option<String> => [20, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            model: Option<String> => [19, true, true, optional] [Option<String>; Option<&'a str>; optional; optional];
            #[serde(default)]
            acceptance_criteria: Vec<String> => [10, true, false, nonempty] [Vec<String>; &'a [&'a str]; strings; array];
            #[serde(default)]
            out_of_scope: Vec<String> => [11, true, false, nonempty] [Vec<String>; &'a [&'a str]; strings; array];
            #[serde(default)]
            files_to_modify: Vec<String> => [12, true, false, nonempty] [Vec<String>; &'a [String]; slice; array];
            /// Advisory collision-prediction hint: files this task may read or write —
            /// typically a superset of `files_to_modify`. Used by an orchestrator to
            /// predict parallel-dispatch conflicts (two tasks conflict when the union
            /// of their `touches` + `files_to_modify` overlaps). Free-text, unvalidated
            /// (posture of `model` / `assignee`); creation-time field.
            #[serde(default)]
            touches: Vec<String> => [13, true, false, nonempty] [Vec<String>; &'a [String]; slice; array];
            /// Advisory capability-routing tags consumed by orchestrators. Free-text,
            /// unvalidated in rmap; downstream tooling owns any vocabulary.
            #[serde(default)]
            domains: Vec<String> => [14, true, false, nonempty] [Vec<String>; &'a [String]; slice; array];
            shipped_in: Option<String> => [32, true, true, optional] [];
            /// Open landing pointer (PR URL, Linear issue, GitLab MR, Gerrit change,
            /// or any other free-text ref). Transition-time: settable only on
            /// `in_progress` via `rmap status <id> in_progress --landing-ref <ref>`;
            /// `--landing-ref ""` clears it. Kept on `done` (provenance next to
            /// `shipped_in`) and `blocked` (a PR closed unmerged is when the ref
            /// matters); cleared on `pending` (the work is being redone — the old
            /// ref belongs in `attempts`). Never parsed or fetched.
            landing_ref: Option<String> => [33, true, true, optional] [];
            body: Option<String> => [22, true, false, optional] [Option<String>; Option<&'a str>; optional; optional];
            created_at: Option<String> => [28, true, true, optional] [Option<String>; Option<&'a str>; date; optional];
            started_at: Option<String> => [29, true, true, optional] [];
            done_at: Option<String> => [31, true, true, optional] [];
            scored_at: Option<String> => [30, true, true, optional] [Option<String>; Option<&'a str>; date; optional];
            blocked_reason: Option<String> => [21, true, true, optional] [];
            implemented: Option<String> => [23, true, true, optional] [];
            delivered_by: Option<String> => [24, true, true, optional] [];
            verified: Option<bool> => [25, true, true, optional] [];
            /// Independent evaluator that supplied `verified = true`. Free-text and
            /// transition-time; new verified transitions require a non-empty value.
            verified_by: Option<String> => [26, true, true, optional] [];
            /// Durable pointer to the evidence behind verification (for example a
            /// harness run id, CI URL, or review artifact). Optional, free-text, and
            /// meaningful only when `verified = true`.
            verification_ref: Option<String> => [27, true, true, optional] [];
            /// Append-only history of dispatch attempts that failed and returned the
            /// task to the queue, each carrying its failure evidence (e.g. a reviewer's
            /// rejection report). A transition-time field — appended by
            /// `rmap status <id> pending --report "<text>" [--attempt-by <agent>]`,
            /// never settable at creation. The implementer/reviewer "argument" happens
            /// asynchronously across attempts, with evidence: the next dispatch reads
            /// this history instead of starting blind. Empty = never failed (skipped on
            /// every output surface so untouched tasks round-trip byte-identically).
            #[serde(default)]
            attempts: Vec<Attempt> => [34, true, false, nonempty] [];
            /// Links this task to related tasks in other roadmaps. These relationships
            /// do not choose where this task's own work lands; `target_repo` does that.
            #[serde(default)]
            cross_repo: Vec<CrossRepo> => [15, true, false, always] [Vec<CrossRepo>; &'a [CrossRepo]; slice; cross_repo];
        }
    };
}
