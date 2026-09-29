//! Plain-text spec-rule tags. No language parser: a configured marker on a
//! line, then rule ids in the rest of that line.
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::Path;

use crate::schema::Tasks;
use crate::specs::{SpecStatus, is_rule_id};

use super::DoctorFinding;

/// Used when `[spec_tests]` omits `marker`.
pub(super) const DEFAULT_MARKER: &str = "spec-tags:";

pub(super) fn findings(tasks: &Tasks, tasks_path: &str) -> Vec<DoctorFinding> {
    let Some(config) = &tasks.spec_tests else {
        return Vec::new();
    };
    if tasks.specs.is_empty() {
        return Vec::new();
    }
    let globs: Vec<&str> = config
        .globs
        .iter()
        .map(|glob| glob.trim().trim_start_matches("./"))
        .filter(|glob| !glob.is_empty())
        .collect();
    if globs.is_empty() {
        return Vec::new();
    }
    let marker = match config.marker.as_deref() {
        None => DEFAULT_MARKER,
        Some(marker) if !marker.trim().is_empty() => marker.trim(),
        Some(_) => return Vec::new(),
    };
    let tasks_path = Path::new(tasks_path);
    if !tasks_path.is_file() {
        return Vec::new();
    }
    let Ok(catalog) = crate::specs::load(tasks, tasks_path) else {
        return Vec::new();
    };

    let mut known = HashSet::new();
    let mut active = Vec::new();
    for spec in catalog.values() {
        for rule in &spec.rules {
            if rule.text.is_none() {
                continue;
            }
            known.insert(rule.id.clone());
            if *spec.status == SpecStatus::Active {
                active.push((rule.id.clone(), spec.path.to_string()));
            }
        }
    }
    active.sort_by(|left, right| left.0.cmp(&right.0));

    let root = crate::paths::project_root_for_tasks_path(tasks_path);
    let tagged = scan_tags(&root, &globs, marker);

    let mut findings = Vec::new();
    for (rule, file) in active {
        if !tagged.contains_key(&rule) {
            findings.push(DoctorFinding::UntestedRule { rule, file });
        }
    }
    for (rule, files) in &tagged {
        if known.contains(rule) {
            continue;
        }
        for file in files {
            findings.push(DoctorFinding::UnknownRuleTag {
                rule: rule.clone(),
                file: file.clone(),
            });
        }
    }
    findings
}

/// Rule id → project-root-relative files that tag it. Both maps are sorted.
fn scan_tags(root: &Path, globs: &[&str], marker: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut tagged: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                if entry.file_name() == ".git" {
                    continue;
                }
                let path = entry.path();
                // Skip trees no glob can reach (`target/`, `node_modules/`, ...).
                if let Ok(relative) = path.strip_prefix(root) {
                    let relative = relative.to_string_lossy().replace('\\', "/");
                    if !globs.iter().any(|glob| glob_may_contain(glob, &relative)) {
                        continue;
                    }
                }
                stack.push(path);
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let relative = relative.to_string_lossy().replace('\\', "/");
            if !globs.iter().any(|glob| glob_match(glob, &relative)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let mut rules = BTreeSet::new();
            for line in text.lines() {
                rules.extend(rule_ids_after_marker(line, marker));
            }
            for rule in rules {
                tagged.entry(rule).or_default().insert(relative.clone());
            }
        }
    }
    tagged
}

pub(super) fn rule_ids_after_marker(line: &str, marker: &str) -> Vec<String> {
    if marker.is_empty() {
        return Vec::new();
    }
    let Some(index) = line.find(marker) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    for token in line[index + marker.len()..]
        .split(|c: char| !(c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'))
    {
        if is_rule_id(token) {
            ids.push(token.to_string());
        }
    }
    ids
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let path: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
    match_parts(&pattern, &path)
}

/// Whether some path below directory `dir` could match `pattern`.
fn glob_may_contain(pattern: &str, dir: &str) -> bool {
    let pattern: Vec<&str> = pattern.split('/').filter(|part| !part.is_empty()).collect();
    let dir: Vec<&str> = dir.split('/').filter(|part| !part.is_empty()).collect();
    prefix_parts(&pattern, &dir)
}

fn prefix_parts(pattern: &[&str], dir: &[&str]) -> bool {
    match (pattern, dir) {
        (_, []) => true,
        ([], _) => false,
        (["**", ..], _) => true,
        ([head, rest @ ..], [segment, dir_rest @ ..]) => {
            segment_match(head, segment) && prefix_parts(rest, dir_rest)
        }
    }
}

fn match_parts(pattern: &[&str], path: &[&str]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if pattern[0] == "**" {
        if match_parts(&pattern[1..], path) {
            return true;
        }
        return !path.is_empty() && match_parts(pattern, &path[1..]);
    }
    !path.is_empty() && segment_match(pattern[0], path[0]) && match_parts(&pattern[1..], &path[1..])
}

fn segment_match(pattern: &str, segment: &str) -> bool {
    fn rec(pattern: &[u8], segment: &[u8]) -> bool {
        match pattern {
            [] => segment.is_empty(),
            [b'*', rest @ ..] => {
                rec(rest, segment) || (!segment.is_empty() && rec(pattern, &segment[1..]))
            }
            [b'?', rest @ ..] => !segment.is_empty() && rec(rest, &segment[1..]),
            [byte, rest @ ..] => {
                segment.first().is_some_and(|head| head == byte) && rec(rest, &segment[1..])
            }
        }
    }
    rec(pattern.as_bytes(), segment.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globs_match_nested_and_single_segment_stars() {
        assert!(glob_match("tests/**/*.rs", "tests/a.rs"));
        assert!(glob_match("tests/**/*.rs", "tests/nested/a.rs"));
        assert!(!glob_match("tests/**/*.rs", "src/a.rs"));
        assert!(!glob_match("tests/**/*.rs", "tests/a.toml"));
        assert!(glob_match("*.rs", "a.rs"));
        assert!(!glob_match("*.rs", "src/a.rs"));
        assert!(glob_match("**/*.exs", "test/a.exs"));
        assert!(glob_match("db/**/*.sql", "db/covered.sql"));
    }

    #[test]
    fn directory_pruning_keeps_only_reachable_trees() {
        assert!(glob_may_contain("tests/**/*.rs", "tests"));
        assert!(glob_may_contain("tests/**/*.rs", "tests/a/b"));
        assert!(!glob_may_contain("tests/**/*.rs", "target"));
        assert!(!glob_may_contain("*.rs", "src"));
        assert!(glob_may_contain("**/*.exs", "deep/nested"));
        assert!(glob_may_contain("t*/unit/*.py", "test/unit"));
        assert!(!glob_may_contain("t*/unit/*.py", "test/other"));
    }

    #[test]
    fn marker_scan_is_plain_text_across_comment_spellings() {
        assert_eq!(
            rule_ids_after_marker("// spec-tags: ACCESS-1, ACCESS-9", "spec-tags:"),
            vec!["ACCESS-1".to_string(), "ACCESS-9".to_string()]
        );
        assert_eq!(
            rule_ids_after_marker("# spec-tags: ACCESS-2", "spec-tags:"),
            vec!["ACCESS-2".to_string()]
        );
        assert_eq!(
            rule_ids_after_marker("-- spec-tags: ACCESS-3 ACCESS-4", "spec-tags:"),
            vec!["ACCESS-3".to_string(), "ACCESS-4".to_string()]
        );
        assert_eq!(
            rule_ids_after_marker("spec-tags: ACCESS-1", "spec-tags:"),
            vec!["ACCESS-1".to_string()]
        );
        assert_eq!(
            rule_ids_after_marker("mentions ACCESS-9 then spec-tags: ACCESS-1", "spec-tags:"),
            vec!["ACCESS-1".to_string()]
        );
        assert!(rule_ids_after_marker("spec-tags: access-1", "spec-tags:").is_empty());
        assert!(rule_ids_after_marker("no marker ACCESS-1", "spec-tags:").is_empty());
    }
}
