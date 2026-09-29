//! Capability rules read from Markdown; TOML stores only registrations and deltas.
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::schema::{Task, TaskId, Tasks};
use crate::vocabulary::Status;

#[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Spec {
    pub path: String,
    pub status: SpecStatus,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Active,
    Retired,
}

#[derive(Clone, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpecChange {
    pub rule: String,
    pub op: SpecOp,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SpecOp {
    Add,
    Change,
    Remove,
}

impl SpecOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Change => "change",
            Self::Remove => "remove",
        }
    }
}

#[derive(Debug, Serialize)]
pub struct RuleHistory<'a> {
    pub task: &'a TaskId,
    pub title: &'a str,
    pub status: &'a crate::schema::Status,
    pub op: SpecOp,
}

#[derive(Debug, Serialize)]
pub struct Rule<'a> {
    pub id: String,
    pub text: Option<String>,
    pub history: Vec<RuleHistory<'a>>,
}

#[derive(Debug, Serialize)]
pub struct LoadedSpec<'a> {
    pub path: &'a str,
    pub status: &'a SpecStatus,
    pub rules: Vec<Rule<'a>>,
}

pub type Catalog<'a> = BTreeMap<&'a str, LoadedSpec<'a>>;

pub(crate) fn is_rule_id(id: &str) -> bool {
    rule_prefix(id).is_some()
}

/// IDs start at column one, with an ASCII letter prefix and decimal suffix.
/// Whitespace or a colon separates the ID from its declarative text.
fn rule_prefix(id: &str) -> Option<&str> {
    let (prefix, number) = id.rsplit_once('-')?;
    (!prefix.is_empty()
        && prefix.bytes().all(|b| b.is_ascii_uppercase())
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit()))
    .then_some(prefix)
}

pub fn load<'a>(tasks: &'a Tasks, tasks_path: &Path) -> Result<Catalog<'a>, String> {
    let root = crate::paths::project_root_for_tasks_path(tasks_path);
    let mut catalog = BTreeMap::new();
    let mut ids = HashSet::new();
    let mut owners = HashMap::new();
    for (name, spec) in &tasks.specs {
        let path = root.join(&spec.path);
        let markdown = std::fs::read_to_string(&path)
            .map_err(|err| format!("spec {}: {err}", path.display()))?;
        let mut rules = Vec::new();
        for line in markdown.lines() {
            let id = line
                .split(|c: char| c.is_whitespace() || c == ':')
                .next()
                .unwrap_or("");
            let Some(prefix) = rule_prefix(id) else {
                continue;
            };
            if !ids.insert(id.to_string()) {
                return Err(format!("duplicate rule id {id} in {}", path.display()));
            }
            if let Some(owner) = owners.insert(prefix.to_string(), name.as_str())
                && owner != name
            {
                return Err(format!(
                    "rule {id}: prefix {prefix} belongs to both specs {owner} and {name}"
                ));
            }
            rules.push(Rule {
                id: id.to_string(),
                text: Some(line.to_string()),
                history: Vec::new(),
            });
        }
        catalog.insert(
            name.as_str(),
            LoadedSpec {
                path: &spec.path,
                status: &spec.status,
                rules,
            },
        );
    }
    // Keep absent rules visible for removed-rule history and proposed additions.
    for task in &tasks.task {
        for change in &task.spec_changes {
            let Some(owner) = rule_prefix(&change.rule).and_then(|prefix| owners.get(prefix))
            else {
                continue;
            };
            let rules = &mut catalog.get_mut(owner).expect("registered owner").rules;
            let index = rules
                .iter()
                .position(|rule| rule.id == change.rule)
                .unwrap_or_else(|| {
                    rules.push(Rule {
                        id: change.rule.clone(),
                        text: None,
                        history: Vec::new(),
                    });
                    rules.len() - 1
                });
            rules[index].history.push(RuleHistory {
                task: &task.id,
                title: &task.title,
                status: &task.status,
                op: change.op,
            });
        }
    }
    Ok(catalog)
}

pub fn validate_changes(tasks: &Tasks, specs: &Catalog<'_>) -> Result<(), String> {
    let rules: Vec<_> = specs
        .values()
        .flat_map(|spec| &spec.rules)
        .filter(|rule| rule.text.is_some())
        .collect();
    for task in &tasks.task {
        if task.status != Status::Pending && task.status != Status::InProgress {
            continue;
        }
        for change in &task.spec_changes {
            let exists = rules.iter().any(|rule| rule.id == change.rule);
            let prefix = rule_prefix(&change.rule);
            let registered =
                prefix.is_some() && rules.iter().any(|rule| rule_prefix(&rule.id) == prefix);
            if (change.op == SpecOp::Add && !registered) || (change.op != SpecOp::Add && !exists) {
                return Err(format!(
                    "task {}: {} rule {} {}",
                    task.id,
                    change.op.as_str(),
                    change.rule,
                    if change.op == SpecOp::Add {
                        "has no registered spec prefix"
                    } else {
                        "is absent from its spec file"
                    }
                ));
            }
        }
    }
    Ok(())
}

pub fn format_specs(specs: &Catalog<'_>) -> String {
    let mut output = String::new();
    for (name, spec) in specs {
        let status = match spec.status {
            SpecStatus::Draft => "draft",
            SpecStatus::Active => "active",
            SpecStatus::Retired => "retired",
        };
        output.push_str(&format!("{name} [{status}] {}\n", spec.path));
        for rule in &spec.rules {
            output.push_str(&format!("  {}\n", rule.text.as_deref().unwrap_or(&rule.id)));
            for entry in &rule.history {
                output.push_str(&format!(
                    "    task {} [{}] {}: {}\n",
                    entry.task,
                    entry.status,
                    entry.op.as_str(),
                    entry.title
                ));
            }
        }
    }
    output
}

pub fn delegate_section(task: &Task, specs: &Catalog<'_>) -> String {
    if task.spec_changes.is_empty() {
        return String::new();
    }
    let mut output = String::from("\n## Spec changes\n");
    for change in &task.spec_changes {
        let text = specs
            .values()
            .flat_map(|spec| &spec.rules)
            .find(|rule| rule.id == change.rule)
            .and_then(|rule| rule.text.as_deref());
        output.push_str(&format!(
            "\n### {} ({})\n\n",
            change.rule,
            change.op.as_str()
        ));
        match text {
            Some(text) => output.push_str(&format!("> {text}\n")),
            None => output.push_str("No current rule text in the registered specs.\n"),
        }
    }
    output
}
