use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::hash::{Hash, Hasher};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tasks {
    pub schema_version: u32,
    pub project: String,
    pub default_branch: String,
    pub vision: Option<String>,
    pub focus: Option<Focus>,
    pub linear: Option<Linear>,
    #[serde(default)]
    pub phases: BTreeMap<String, Phase>,
    #[serde(default)]
    pub bundles: BTreeMap<String, Bundle>,
    #[serde(default)]
    pub milestones: BTreeMap<String, Milestone>,
    #[serde(default)]
    pub task: Vec<Task>,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Focus {
    pub phase: u32,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Linear {
    pub team_key: String,
    pub workspace_url: String,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub name: String,
    pub order: u32,
    pub status: String,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub phase: u32,
    pub order: u32,
    pub description: String,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Milestone {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub order: u32,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_version: Option<String>,
}

/// A single roadmap task.
///
/// MIRROR SURFACES — when adding a field here, decide whether it is a
/// **creation-time** field (set at `rmap new` time) or a **transition-time**
/// field (set by `rmap status` / `rmap mark` / `rmap depend`), and update the
/// appropriate surfaces in the same commit:
///
/// Creation-time field → SIX surfaces:
///   1. `main.rs::StdinTask`                  — stdin parse shape
///   2. `mutate.rs::NewTaskFields`            — mutator argument struct
///   3. `mutate.rs::add_task_str`             — TOML writer
///   4. `mutate.rs::canonical_task_key_index` — key ordering for serialization
///   5. `diff.rs::diff_fields!`               — drift surface for `rmap diff`
///   6. `export.rs::ExportedTask`             — JSON shape for `--json` / `data.json`
///
/// Plus decide whether to add to `diff::TASK_VERBOSE_WHITELIST`. Interactive
/// `prompt_task_fields` (main.rs) is optional — power-user fields (`branch`,
/// `files_to_modify`, `touches`, `cross_repo`) intentionally route through
/// `rmap new --from-stdin` rather than dialoguer.
///
/// Transition-time field → owning mutator (`set_status_str` for lifecycle
/// timestamps + `implemented` + outcome layer, etc.) + surfaces 4–6. Stays
/// absent from `StdinTask` / `NewTaskFields` on purpose. Today: `started_at`,
/// `done_at`, `blocked_reason`, `shipped_in`, `implemented`, `delivered_by`,
/// `verified`, `verified_by`, `verification_ref`, `attempts`.
#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: TaskId,
    pub phase: u32,
    pub bundle: String,
    /// Repository where this task's own work lands. When omitted, consumers
    /// use the roadmap's top-level `project`. This is distinct from
    /// `cross_repo`, which links this task to related tasks in other roadmaps.
    pub target_repo: Option<String>,
    pub milestone: Option<String>,
    pub status: String,
    pub title: String,
    pub scores: Scores,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub linear_id: Option<String>,
    pub assignee: Option<String>,
    pub module: Option<String>,
    pub branch: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub out_of_scope: Vec<String>,
    #[serde(default)]
    pub files_to_modify: Vec<String>,
    /// Advisory collision-prediction hint: files this task may read or write —
    /// typically a superset of `files_to_modify`. Used by an orchestrator to
    /// predict parallel-dispatch conflicts (two tasks conflict when the union
    /// of their `touches` + `files_to_modify` overlaps). Free-text, unvalidated
    /// (posture of `model` / `assignee`); creation-time field.
    #[serde(default)]
    pub touches: Vec<String>,
    /// Advisory capability-routing tags consumed by orchestrators. Free-text,
    /// unvalidated in rmap; downstream tooling owns any vocabulary.
    #[serde(default)]
    pub domains: Vec<String>,
    pub shipped_in: Option<String>,
    pub body: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub done_at: Option<String>,
    pub scored_at: Option<String>,
    pub blocked_reason: Option<String>,
    pub implemented: Option<String>,
    pub delivered_by: Option<String>,
    pub verified: Option<bool>,
    /// Independent evaluator that supplied `verified = true`. Free-text and
    /// transition-time; new verified transitions require a non-empty value.
    pub verified_by: Option<String>,
    /// Durable pointer to the evidence behind verification (for example a
    /// harness run id, CI URL, or review artifact). Optional, free-text, and
    /// meaningful only when `verified = true`.
    pub verification_ref: Option<String>,
    /// Append-only history of dispatch attempts that failed and returned the
    /// task to the queue, each carrying its failure evidence (e.g. a reviewer's
    /// rejection report). A transition-time field — appended by
    /// `rmap status <id> pending --report "<text>" [--attempt-by <agent>]`,
    /// never settable at creation. The implementer/reviewer "argument" happens
    /// asynchronously across attempts, with evidence: the next dispatch reads
    /// this history instead of starting blind. Empty = never failed (skipped on
    /// every output surface so untouched tasks round-trip byte-identically).
    #[serde(default)]
    pub attempts: Vec<Attempt>,
    /// Links this task to related tasks in other roadmaps. These relationships
    /// do not choose where this task's own work lands; `target_repo` does that.
    #[serde(default)]
    pub cross_repo: Vec<CrossRepo>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scores {
    pub d: u32,
    pub b: u32,
    pub u: u32,
}

/// One recorded dispatch attempt that did not stick — e.g. a cross-family
/// reviewer's rejection report on a run that sent the task back to `pending`.
/// Accumulated (never overwritten) in [`Task::attempts`]; appended by
/// `rmap status <id> pending --report`. Stored as an inline table per entry
/// (mirroring [`CrossRepo`]).
#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    /// ISO date (`YYYY-MM-DD`) the attempt was recorded. Auto-filled from
    /// `today_iso()` at write time — `RMAP_TODAY`-aware like every timestamp.
    pub at: String,
    /// Which agent or instance made the attempt (free-text, unvalidated —
    /// posture of `model` / `delivered_by`). Optional: the writer supplies it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// Free-text failure evidence (a reviewer's rejection report, a harness
    /// verdict). An attempt without a report carries no signal, so the mutator
    /// only appends an entry when a report is supplied.
    pub report: String,
}

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrossRepo {
    pub repo: String,
    pub task_id: TaskId,
    pub linear_id: Option<String>,
    pub relation: String,
}

/// Primary key for a `Task`. Stored on-disk as either a TOML integer
/// (`id = 1`) or a TOML string (`id = "INE-5"`, `id = "alpha"`); kept as a
/// dual-form enum so both shapes round-trip byte-identically.
///
/// **`Eq` / `Hash` are normalizing.** `Number(1)` and `Text("1")` compare
/// equal and hash identically — identity is identity, and a task id is a
/// primary key. The disk form (integer vs string) does not change which
/// task the id names. The manual impls below replace the derived ones for
/// this reason; without them a `HashSet<TaskId>` would treat `Number(1)`
/// and `Text("1")` as distinct keys, which `validate_unique_ids`,
/// `validate_dependencies`, `validate_dependency_cycles`,
/// `doctor::find_degenerate_bundles`, and `next_bundle`'s actionability
/// memo all rely on NOT being the case.
///
/// `Text` ids that do not parse as a `u32` (e.g. `"INE-5"`, `"alpha"`)
/// keep their own canonical key — `Number(5)` is NOT equal to
/// `Text("INE-5")`.
#[derive(Debug, Clone, JsonSchema, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskId {
    Number(u32),
    Text(String),
}

impl TaskId {
    /// Returns the form used by `Eq` / `Hash` to determine identity.
    ///
    /// `Number(n)` → owned `"n"`. `Text(s)` → borrowed `s` (zero-alloc
    /// unless it parses as a `u32`, in which case it's folded to the
    /// canonical decimal form via `Cow::Owned`).
    fn canonical_key(&self) -> Cow<'_, str> {
        match self {
            Self::Number(id) => Cow::Owned(id.to_string()),
            Self::Text(id) => match id.parse::<u32>() {
                Ok(n) => Cow::Owned(n.to_string()),
                Err(_) => Cow::Borrowed(id.as_str()),
            },
        }
    }
}

impl PartialEq for TaskId {
    fn eq(&self, other: &Self) -> bool {
        self.canonical_key() == other.canonical_key()
    }
}

impl Eq for TaskId {}

impl Hash for TaskId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.canonical_key().hash(state);
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(id) => write!(formatter, "{id}"),
            Self::Text(id) => formatter.write_str(id),
        }
    }
}

impl PartialEq<u32> for TaskId {
    fn eq(&self, other: &u32) -> bool {
        match self {
            Self::Number(id) => id == other,
            Self::Text(id) => id.parse::<u32>().is_ok_and(|n| n == *other),
        }
    }
}
