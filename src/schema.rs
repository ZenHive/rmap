use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tasks {
    pub schema_version: u32,
    pub project: String,
    pub default_branch: String,
    pub linear: Option<Linear>,
    #[serde(default)]
    pub phases: BTreeMap<String, Phase>,
    #[serde(default)]
    pub bundles: BTreeMap<String, Bundle>,
    #[serde(default)]
    pub task: Vec<Task>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Linear {
    pub team_key: String,
    pub workspace_url: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Phase {
    pub name: String,
    pub order: u32,
    pub status: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Bundle {
    pub phase: u32,
    pub order: u32,
    pub description: String,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: TaskId,
    pub phase: u32,
    pub bundle: String,
    pub status: String,
    pub title: String,
    pub scores: Scores,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub linear_id: Option<String>,
    pub shipped_in: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub cross_repo: Vec<CrossRepo>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scores {
    pub d: u32,
    pub b: u32,
    pub u: u32,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CrossRepo {
    pub repo: String,
    pub task_id: TaskId,
    pub linear_id: Option<String>,
    pub relation: String,
}

#[derive(Debug, Clone, Eq, Hash, PartialEq, Deserialize, Serialize)]
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
