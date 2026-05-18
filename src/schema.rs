use std::collections::BTreeMap;
use std::fmt;

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
/// `files_to_modify`, `cross_repo`) intentionally route through
/// `rmap new --from-stdin` rather than dialoguer.
///
/// Transition-time field → owning mutator (`set_status_str` for lifecycle
/// timestamps + `implemented`, etc.) + surfaces 4–6. Stays absent from
/// `StdinTask` / `NewTaskFields` on purpose. Today: `started_at`, `done_at`,
/// `blocked_reason`, `shipped_in`, `implemented`.
#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: TaskId,
    pub phase: u32,
    pub bundle: String,
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
    pub shipped_in: Option<String>,
    pub body: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub done_at: Option<String>,
    pub scored_at: Option<String>,
    pub blocked_reason: Option<String>,
    pub implemented: Option<String>,
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

#[derive(Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrossRepo {
    pub repo: String,
    pub task_id: TaskId,
    pub linear_id: Option<String>,
    pub relation: String,
}

#[derive(Debug, Clone, Eq, Hash, JsonSchema, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum TaskId {
    Number(u32),
    Text(String),
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
        matches!(self, Self::Number(id) if id == other)
    }
}
