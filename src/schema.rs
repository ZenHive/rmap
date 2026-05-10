use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq)]
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

#[derive(Debug, Deserialize, PartialEq)]
pub struct Linear {
    pub team_key: String,
    pub workspace_url: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Phase {
    pub name: String,
    pub order: u32,
    pub status: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Bundle {
    pub phase: u32,
    pub order: u32,
    pub description: String,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Task {
    pub id: u32,
    pub phase: u32,
    pub bundle: String,
    pub status: String,
    pub title: String,
    pub scores: Scores,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<u32>,
    pub linear_id: Option<String>,
    pub shipped_in: Option<String>,
    pub body: Option<String>,
    #[serde(default)]
    pub cross_repo: Vec<CrossRepo>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct Scores {
    pub d: u32,
    pub b: u32,
    pub u: u32,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct CrossRepo {
    pub repo: String,
    pub task_id: u32,
    pub linear_id: Option<String>,
    pub relation: String,
}
