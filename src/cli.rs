use clap::{Parser, Subcommand};
use rmap::delegate::DelegateTarget;
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "rmap")]
#[command(about = "Manage portable roadmap data")]
#[command(version)]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Commands,
}

#[derive(Debug, Subcommand)]
pub(super) enum Commands {
    Validate {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        check_render: bool,
    },
    Render {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
        #[arg(long, conflicts_with = "stdout")]
        dry: bool,
        #[arg(long)]
        stdout: bool,
        /// Also write the self-contained static HTML view to
        /// `roadmap/dist/index.html` (gitignored). With `--stdout`, prints the
        /// HTML instead of writing it.
        #[arg(long)]
        html: bool,
        /// Render the multi-project portfolio view instead. Each value is a
        /// project root or a `data.json` path; requires `--html`. Writes
        /// `roadmap/dist/portfolio.html` (or `--out`). With `--stdout`, prints
        /// the portfolio HTML.
        #[arg(long, num_args = 1.., value_name = "PATH")]
        multi: Vec<PathBuf>,
        /// Output path for the portfolio HTML (default
        /// `roadmap/dist/portfolio.html` under the current project root).
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Watch `roadmap/tasks.toml` and re-render `ROADMAP.md` + `roadmap/data.json`
    /// on every change. Foreground and blocking; stop with Ctrl-C.
    ///
    /// Idempotent — a save that produces no rendered change writes nothing and
    /// prints nothing. Validation failures print to stderr and the loop keeps
    /// running, so a mid-edit broken TOML recovers on the next valid save.
    ///
    /// With `--json`, each render event is emitted as one compact JSON line to
    /// stdout (`rendered` / `error`); no-op renders stay silent. The startup
    /// line and watcher-infrastructure errors remain text on stderr.
    Watch {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
        /// Emit one JSON event line per render to stdout instead of `rendered`.
        #[arg(long)]
        json: bool,
    },
    Status {
        id: String,
        new_status: String,
        /// Set or overwrite `implemented` in the same transition. Required when
        /// transitioning to `done` unless the task already carries the field;
        /// on a TTY without this flag, `rmap` prompts interactively. Ignored
        /// (with a one-line stderr note) on non-`done` transitions.
        #[arg(long)]
        implemented: Option<String>,
        /// Outcome field: which agent or instance actually delivered the task
        /// (free-text, like `model`). Settable only on `done` transitions;
        /// ignored with a one-line stderr note otherwise. Overwrites any
        /// existing value.
        #[arg(long)]
        delivered_by: Option<String>,
        /// Outcome field: mark the task as independently verified by an
        /// evaluator separate from the implementer. Settable only on `done`
        /// transitions; ignored with a one-line stderr note otherwise.
        /// Presence flag (no opposite — to clear, edit `tasks.toml` directly).
        #[arg(long)]
        verified: bool,
        /// Outcome field: independent evaluator that supplied `--verified`.
        /// Required with `--verified`; free-text and done-only.
        #[arg(long)]
        verified_by: Option<String>,
        /// Outcome field: durable pointer to verification evidence (for
        /// example a harness run id, CI URL, or review artifact). Optional,
        /// free-text, and usable only with `--verified` on `done`.
        #[arg(long)]
        verification_ref: Option<String>,
        /// Outcome field: where the work landed (commit SHA / PR ref, free-text
        /// like `delivered_by`). Settable only on `done` transitions; ignored
        /// with a one-line stderr note otherwise. Overwrites any existing value.
        #[arg(long)]
        shipped_in: Option<String>,
        /// Set or overwrite `blocked_reason` in the same transition. Settable
        /// only on `blocked` transitions; ignored with a one-line stderr note
        /// otherwise. Overwrites any existing value. Auto-cleared when a blocked
        /// task leaves the blocked state. Passed non-TTY; no interactive prompt.
        #[arg(long)]
        reason: Option<String>,
        /// Append failure evidence (a reviewer's rejection report) for the
        /// attempt that returned this task to the queue. Settable only on
        /// `pending` transitions; ignored with a one-line stderr note otherwise.
        /// Accumulates — each call appends a new attempt, never overwrites.
        #[arg(long)]
        report: Option<String>,
        /// Attribute the appended `--report` to an agent (free-text). Only
        /// meaningful with `--report` on a `pending` transition.
        #[arg(long)]
        attempt_by: Option<String>,
        /// Record an open landing pointer (PR URL or other free-text ref) on
        /// an `in_progress` task. Rejected on any other target status. An
        /// `in_progress` → `in_progress` call with only this flag is a field
        /// update (`started_at` unchanged). Pass an empty value to clear.
        /// Kept on `done`/`blocked`; cleared on `pending`.
        #[arg(long)]
        landing_ref: Option<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    Next {
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        bundle: Option<String>,
        #[arg(long)]
        milestone: Option<String>,
        #[arg(long, default_value = "1")]
        count: NonZeroUsize,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Show {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// What does this task unblock? Lists every task that transitively depends
    /// on `<id>` — its full downstream subtree, nearest dependency wave first.
    ///
    /// The transitive answer to "if I finish this, what frees up?" Direct edges
    /// are already visible in each task's `depends_on`; this walks the whole
    /// subtree. `--json` is a `list`-shaped envelope (each task carries its
    /// computed `unlocks` leverage). Empty when `<id>` is a leaf.
    Blocks {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// What must finish before this task? Lists every task `<id>` transitively
    /// depends on — its full prerequisite subtree, nearest dependency wave first.
    ///
    /// The transitive answer to "what blocks this?" `--json` is a `list`-shaped
    /// envelope. Empty when `<id>` is a dependency root.
    Deps {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        phase: Option<u32>,
        #[arg(long)]
        bundle: Option<String>,
        /// Repository where this task's own work lands. An omitted task field
        /// targets the roadmap's top-level `project`. Unlike `cross_repo`, this
        /// does not link related tasks in other roadmaps.
        #[arg(long, value_name = "REPO")]
        target_repo: Option<String>,
        #[arg(long)]
        milestone: Option<String>,
        /// Filter to tasks whose `delivered_by` matches this agent id.
        #[arg(long)]
        delivered_by: Option<String>,
        /// Exclude tasks carrying the `handbuild` marker (headless-dispatchable only).
        #[arg(long)]
        dispatchable: bool,
        /// Project --json to these comma-separated ExportedTask fields. Implies --json.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// List the parallel-safe dispatch set: every `pending` task whose
    /// `depends_on` are all `done`, ranked like `rmap next` (4-tier focus ×
    /// active-milestone, then Eff desc).
    ///
    /// The set is mutually independent by construction — a pending task whose
    /// deps are all `done` cannot depend on another pending task — so it is
    /// safe to dispatch every returned task in parallel. `--bundle <B>` yields
    /// the dispatchable layer-0 of that bundle (the answer `next-bundle`'s
    /// serial chain can't give). Unlike `next`, `--count` is optional and
    /// defaults to the whole set. `--json` carries `dep_layer` per task so a
    /// single call answers "what can I dispatch now + which wave each unlocks."
    Ready {
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        bundle: Option<String>,
        #[arg(long)]
        phase: Option<u32>,
        #[arg(long)]
        milestone: Option<String>,
        /// Cap the result to the top N (default: the entire ready set).
        #[arg(long)]
        count: Option<NonZeroUsize>,
        /// Exclude tasks carrying the `handbuild` marker (headless-dispatchable only).
        #[arg(long)]
        dispatchable: bool,
        /// Project --json to these comma-separated ExportedTask fields. Implies --json.
        #[arg(long, value_delimiter = ',')]
        fields: Vec<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Schema,
    Diff {
        #[arg(long)]
        against: Option<String>,
        #[arg(long)]
        json: bool,
        /// Add per-field before/after values to Changed entries (whitelist-gated).
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Delegate {
        id: String,
        /// Target agent. Optional — defaults to the task's stored `assignee`.
        #[arg(long)]
        to: Option<DelegateTarget>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// Emit a paste-ready prompt for migrating a hand-edited ROADMAP.md into tasks.toml.
    ///
    /// Prints to stdout; never writes. Reads an existing `roadmap/tasks.toml` if present
    /// (only to populate the `Project:` line in the prompt) — falls back to a placeholder
    /// when none is found. The emitted prompt instructs an agent to convert ROADMAP.md
    /// into roadmap/tasks.toml and add the marker pairs rmap render manages. Mirrors
    /// `rmap delegate`'s read-only contract.
    Import {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Aggregate health report: validation findings + stale + score-decay + render drift.
    ///
    /// Always exits 0 — informational. Use `rmap validate` for strict schema gating.
    /// Exception: if tasks.toml is unparseable, exits non-zero (nothing to analyze).
    Doctor {
        /// Override the 30-day stale + score-decay cutoff (in days).
        #[arg(long)]
        threshold_days: Option<u32>,
        /// Override the D/B bar for the missing-acceptance_criteria lint (defaults 5/8).
        #[arg(long)]
        ac_threshold: Option<u32>,
        /// Override the transitive-dependent count for the graph-bottleneck lint (default 3).
        #[arg(long)]
        bottleneck_min: Option<u32>,
        /// Override the Jaccard similarity percent (0–100) for near-duplicate open-task pairing (default 80).
        #[arg(long)]
        near_duplicate_min: Option<u32>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
    },
    /// Add or remove markers on a task: `rmap mark 75 +cx -parallel`.
    ///
    /// Each `ops` token must start with `+` (add) or `-` (remove). Add is a no-op
    /// when the marker is already present; remove is a no-op when absent.
    /// The full file is re-validated after edit and any invalid marker name
    /// (per `VALID_MARKERS`) aborts the write.
    Mark {
        id: String,
        /// `+marker` to add, `-marker` to remove. One or more, applied left-to-right.
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        ops: Vec<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Pin or unpin a task's milestone: `rmap milestone 7 v0_1` or `rmap milestone 7 none`.
    ///
    /// `<name>` must match an existing `[milestones.<name>]` key, or the literal
    /// `none` to clear the field. Validates the target before writing; unknown
    /// milestones leave the file byte-equal.
    Milestone {
        id: String,
        name: String,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Set or clear a task's agent routing: `rmap assign 7 cursor --model composer-2.5-fast`,
    /// or `rmap assign 7 none` / `rmap assign 7 human` to unassign.
    ///
    /// A non-`human` assignee on a live (`pending` / `in_progress`) task requires
    /// `--model` (same dispatchable-pin gate as `rmap new`). Clearing via `none` or
    /// `human` removes both `assignee` and `model`; `--model` is forbidden on that
    /// path. Validates before writing; unknown ids leave the file byte-equal.
    Assign {
        id: String,
        assignee: String,
        /// LLM pin for the assignee (required when routing to a real agent).
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Add a dependency to a task.
    ///
    /// In-repo: `rmap depend 75 on 74`. Cross-repo: `rmap depend 75 --cross-repo
    /// ccxt_client:42:blocks` (relation defaults to `blocks` if omitted). Both
    /// forms can be combined in a single call. Cycles and unknown ids are
    /// rejected via re-validation.
    Depend {
        id: String,
        /// Literal `on` keyword separating the in-repo dependency. Required when
        /// the next positional is the target task id.
        on: Option<String>,
        /// Target task id when adding an in-repo dependency.
        other: Option<String>,
        /// Cross-repo dependency: `<repo>:<task_id>[:<relation>]`.
        #[arg(long)]
        cross_repo: Option<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Create a new task and append it to `tasks.toml`.
    ///
    /// With `--from-stdin`, reads a TOML fragment from stdin (one or more
    /// `[[task]]` blocks, accepting the same field set as `schema::Task`).
    /// `target_repo` names where this task's own work lands and defaults to the
    /// roadmap's `project`; `cross_repo` instead links related tasks in other
    /// roadmaps.
    /// Without `--from-stdin`, drops into an interactive `dialoguer` prompt
    /// flow — requires a TTY. In both modes, the id is auto-allocated
    /// (numeric `max + 1`) unless the caller supplies one explicitly, and
    /// `created_at` / `scored_at` default to `today_iso()`. Re-validates and
    /// re-renders ROADMAP.md + data.json on success; on failure the file is
    /// left byte-equal to its pre-call state.
    New {
        /// Read one-or-more `[[task]]` blocks as TOML from stdin instead of prompting.
        #[arg(long)]
        from_stdin: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// List declared bundles with per-bundle counts and next-task hints.
    ///
    /// Read-only discovery aid for `rmap next --bundle <name>` and
    /// `rmap list --bundle <name>` — surfaces every `[bundles.*]` so callers
    /// don't have to grep `tasks.toml`. Bundles group under per-phase headers;
    /// focus-phase bundles sort first.
    Bundles {
        #[arg(long)]
        phase: Option<u32>,
        /// Only bundles whose next_task is non-null.
        #[arg(long)]
        has_next: bool,
        /// Only bundles in `[focus].phase`.
        #[arg(long)]
        in_focus: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// List declared milestones with per-milestone counts and next-task hints.
    ///
    /// Read-only discovery aid for `rmap milestone <id> <name>`,
    /// `rmap list --milestone <name>`, and `rmap next --milestone <name>`.
    /// Active milestones sort first — the answer to "which release am I
    /// cutting next?" Mirrors `rmap bundles`.
    Milestones {
        /// Only milestones whose next_task is non-null.
        #[arg(long)]
        has_next: bool,
        /// Filter by milestone status: `pending`, `active`, `done`.
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// Pick one session-sized bundle and emit every actionable pending task in it.
    ///
    /// Ranking: focus-phase bundles win over non-focus; within a phase, rank by
    /// sum-of-Eff over actionable pending tasks (descending); ties broken by
    /// `bundles.<name>.order`. Bundles with zero actionable tasks are skipped.
    /// `--bundle <name>` bypasses ranking entirely.
    NextBundle {
        /// Override `[focus].phase` for this query.
        #[arg(long)]
        phase: Option<u32>,
        /// Force-pick a specific bundle, bypassing ranking.
        #[arg(long)]
        bundle: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// Longest dependency chain through the DAG — the serial bottleneck to a release.
    ///
    /// Emits the ordered root→leaf task sequence (path length = task count, not
    /// Eff-weighted). `--milestone <name>` scopes to that release line's pinned
    /// tasks and their transitive in-repo dependencies.
    CriticalPath {
        /// Scope to a milestone's pinned tasks and their transitive deps.
        #[arg(long)]
        milestone: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// Print the parallel dispatch schedule of the open tasks.
    ///
    /// Only pending / in_progress / blocked tasks appear; done and superseded
    /// dependencies count as satisfied. Wave 0 has no open prerequisites; each
    /// successive wave starts only after the prior wave completes. Read-only.
    Waves {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// List in-progress tasks idle longer than the given duration.
    Stale {
        /// Duration threshold. Units: d (day), w (7d), m (30d), y (365d). Example: "30d", "2w", "6m", "1y".
        #[arg(long)]
        over: String,
        /// Emit JSON envelope.
        #[arg(long)]
        json: bool,
        /// Path to tasks.toml (overrides discovery).
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
pub(super) enum ExportCommands {
    Json {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// Emit a Graphviz DOT digraph of the in-repo `depends_on` graph.
    ///
    /// Nodes carry status fill colors and Eff tier glyphs; edges run
    /// dependency → dependent. Pipe to `dot` or another Graphviz tool —
    /// rmap does not render images.
    Dot {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}
