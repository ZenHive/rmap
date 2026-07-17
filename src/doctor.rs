use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::render::render_roadmap_str_with_today;
use crate::schema::{Scores, Task, TaskId, Tasks};
use crate::scoring::days_since;
use crate::stale::find_stale;
use crate::topo::{compute_unlocks, forward_adjacency, milestone_reachable_ids};
use crate::validate;

/// Default cutoff (in days) for both the stale-in-progress check and the
/// score-decay check. Overridable per-invocation via `rmap doctor --threshold-days`.
const STALE_THRESHOLD_DAYS: u32 = 30;

/// Minimum D or B at which a pending/in_progress task without `acceptance_criteria`
/// triggers the `missing_acceptance_criteria` doctor finding. Substantive tasks should
/// declare what "done" means; trivial tasks can rely on title-as-prompt.
const AC_DIFFICULTY_THRESHOLD: u32 = 5;
const AC_BENEFIT_THRESHOLD: u32 = 8;

/// Minimum transitive-dependent count at which a still-open task triggers the
/// `bottleneck` doctor finding. Overridable via `rmap doctor --bottleneck-min`.
const BOTTLENECK_MIN_THRESHOLD: u32 = 3;

/// Minimum Jaccard similarity (percent, 0–100) at which two open tasks' normalized
/// title+body token sets are treated as near-duplicates. Overridable via
/// `rmap doctor --near-duplicate-min`. Tuned high so distinct-but-related open
/// tasks in real roadmaps do not pair by default.
const NEAR_DUPLICATE_MIN_PERCENT: u32 = 80;

/// Fixed word list for the unmeasurable-criteria lint. A criterion is vague when
/// every non-stopword token is drawn from this set (purely mechanical — rmap does
/// not judge meaning). Documented for agents; keep the list short and stable.
const VAGUE_WORDS: &[&str] = &["fast", "robust", "properly", "correctly", "works"];

/// Stopwords stripped before the vague-word check so connectors alone do not
/// rescue an otherwise empty criterion ("it works" → `{works}` → vague).
const VAGUE_STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "but", "so", "it", "is", "be", "to", "of", "in", "on", "for",
    "with", "that", "this", "as", "at", "by", "from", "if", "not", "very", "really", "quite",
    "just", "also", "too", "more", "most", "than",
];

/// Effective doctor thresholds for one `rmap doctor` invocation: the constant
/// defaults above unless overridden by `--threshold-days` / `--ac-threshold` /
/// `--bottleneck-min` / `--near-duplicate-min`.
/// `--threshold-days` collapses the stale and score-decay cutoffs into one
/// `days` value; `--ac-threshold` collapses the distinct D/B defaults into one.
#[derive(serde::Serialize, Clone, Copy)]
pub struct DoctorThresholds {
    pub days: u32,
    pub ac_difficulty: u32,
    pub ac_benefit: u32,
    pub bottleneck_min: u32,
    /// Jaccard similarity percent (0–100) for near-duplicate open-task pairing.
    pub near_duplicate_min: u32,
}

impl DoctorThresholds {
    pub fn resolve(
        threshold_days: Option<u32>,
        ac_threshold: Option<u32>,
        bottleneck_min: Option<u32>,
        near_duplicate_min: Option<u32>,
    ) -> Self {
        Self {
            days: threshold_days.unwrap_or(STALE_THRESHOLD_DAYS),
            ac_difficulty: ac_threshold.unwrap_or(AC_DIFFICULTY_THRESHOLD),
            ac_benefit: ac_threshold.unwrap_or(AC_BENEFIT_THRESHOLD),
            bottleneck_min: bottleneck_min.unwrap_or(BOTTLENECK_MIN_THRESHOLD),
            near_duplicate_min: near_duplicate_min.unwrap_or(NEAR_DUPLICATE_MIN_PERCENT),
        }
    }
}

#[derive(serde::Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub findings: Vec<DoctorFinding>,
    pub thresholds: DoctorThresholds,
}

#[derive(serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DoctorFinding {
    Validate {
        message: String,
    },
    Stale {
        id: String,
        started_at: String,
        days_idle: i64,
    },
    ScoreDecay {
        id: String,
        scored_at: Option<String>,
    },
    DegenerateBundle {
        bundle: String,
        phase: u32,
        task_count: usize,
    },
    MissingAcceptanceCriteria {
        id: String,
        scores: Scores,
    },
    /// Soft outcome-layer advisory: a `done` task without `verified` was
    /// declared finished by an implementer but no independent evaluator
    /// confirmed it. Never failing — hand-built / bootstrap tasks legitimately
    /// land without external verification.
    ClaimedNotGraded {
        id: String,
    },
    /// Migration advisory for legacy positive verification claims that predate
    /// evaluator provenance. New CLI transitions reject this shape.
    VerifiedWithoutProvenance {
        id: String,
    },
    PhaseFullyDoneButOpen {
        phase: u32,
        status: String,
        task_count: usize,
    },
    PhaseHasInProgressButPending {
        phase: u32,
        task_ids: Vec<String>,
    },
    FocusPhaseClosed {
        phase: u32,
        status: String,
        task_count: usize,
    },
    /// Soft milestone drift: every pinned task is `done` but the milestone
    /// remains `pending` or `active`. Milestone status is user-curated.
    MilestoneFullyDoneButOpen {
        milestone: String,
        status: String,
        task_count: usize,
    },
    /// Soft milestone drift: more than one milestone is `active` (rmap.md:
    /// keep exactly one active release line for `rmap next` auto-bias).
    MultipleActiveMilestones {
        milestones: Vec<ActiveMilestone>,
    },
    /// Soft graph-health advisory: a still-open task gates many downstream
    /// tasks (transitive-dependent count >= threshold).
    Bottleneck {
        id: String,
        dependent_count: usize,
    },
    /// Soft graph-health advisory: a task is disconnected from the dependency
    /// graph (orphan) or not downstream of any milestone-pinned task.
    IsolatedNode {
        id: String,
        reason: IsolatedNodeReason,
    },
    /// Soft AC-quality advisory: a live agent-assigned task has an
    /// `acceptance_criteria` entry containing an unresolved placeholder token
    /// (TODO / TBD / ??? / `<stub>`). Mechanical only — rmap does not judge meaning.
    PlaceholderCriteria {
        id: String,
    },
    /// Soft AC-quality advisory: a live agent-assigned task has a criterion whose
    /// non-stopword tokens are drawn entirely from the fixed vague-word list
    /// (`works` / `properly` / `correctly` / `fast` / `robust`). A criterion that
    /// mixes those words with measurable content does not fire.
    VagueCriteria {
        id: String,
    },
    /// Soft near-duplicate advisory: two open (`pending`/`in_progress`/`blocked`)
    /// tasks have title+body token sets at or above the Jaccard threshold.
    /// `done` / `superseded` tasks never pair. `ids` is length 2, sorted for stability.
    NearDuplicateTasks {
        ids: Vec<String>,
    },
    Drift,
}

/// Why an [`DoctorFinding::IsolatedNode`] fired.
#[derive(serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IsolatedNodeReason {
    Orphan,
    UnreachableFromMilestone,
}

/// One `active` milestone cited by a `MultipleActiveMilestones` finding.
#[derive(serde::Serialize)]
pub struct ActiveMilestone {
    pub milestone: String,
    pub task_count: usize,
}

impl DoctorReport {
    /// Builds a composite health report: validation findings (non-short-circuiting),
    /// stale in-progress tasks (>`thresholds.days` via `started_at`), score-decay
    /// candidates (`scored_at` missing or >`thresholds.days`), phase/focus/milestone
    /// state-drift advisories, and render drift (when `roadmap_input` is supplied).
    /// Pure — no I/O. `today` is the `YYYY-MM-DD` reference date. `thresholds`
    /// carries the effective stale/decay and AC cutoffs for this invocation (defaults
    /// unless overridden on the CLI).
    pub fn run(
        tasks: &Tasks,
        path: &str,
        input: &str,
        roadmap_input: Option<&str>,
        today: &str,
        thresholds: DoctorThresholds,
    ) -> Self {
        let mut findings: Vec<DoctorFinding> = Vec::new();

        // 1. validate findings (all checks, no short-circuit)
        for err in validate::collect_findings(tasks, path, input) {
            findings.push(DoctorFinding::Validate {
                message: err.to_string(),
            });
        }

        // 2. stale in-progress tasks
        for task in find_stale(tasks, thresholds.days, today) {
            let started_at = task.started_at.clone().unwrap_or_default();
            let days_idle = days_since(today, &started_at).unwrap_or(0);
            findings.push(DoctorFinding::Stale {
                id: task_id_display(&task.id),
                started_at,
                days_idle,
            });
        }

        // 3. score decay (missing or stale scored_at)
        for task in &tasks.task {
            let decayed = match task.scored_at.as_deref() {
                None => true,
                Some(s) => days_since(today, s)
                    .map(|d| d > i64::from(thresholds.days))
                    .unwrap_or(false),
            };
            if decayed {
                findings.push(DoctorFinding::ScoreDecay {
                    id: task_id_display(&task.id),
                    scored_at: task.scored_at.clone(),
                });
            }
        }

        // 4. degenerate bundles — bundle covers every task of its declared phase.
        // Adds zero information beyond the phase itself; real bundles span multiple
        // phases or cluster a strict subset sharing infrastructure. Single-task
        // phases are skipped — the bundle name still adds a label (it's not
        // redundant in the same way a 3-task bundle covering a 3-task phase is).
        for (name, bundle) in &tasks.bundles {
            let bundle_ids: HashSet<&TaskId> = tasks
                .task
                .iter()
                .filter(|t| t.bundle == *name)
                .map(|t| &t.id)
                .collect();
            if bundle_ids.len() < 2 {
                continue;
            }
            let phase_ids: HashSet<&TaskId> = tasks
                .task
                .iter()
                .filter(|t| t.phase == bundle.phase)
                .map(|t| &t.id)
                .collect();
            if bundle_ids == phase_ids {
                findings.push(DoctorFinding::DegenerateBundle {
                    bundle: name.clone(),
                    phase: bundle.phase,
                    task_count: bundle_ids.len(),
                });
            }
        }

        // 5. missing acceptance_criteria on substantive active tasks
        for task in &tasks.task {
            let substantive =
                task.scores.d >= thresholds.ac_difficulty || task.scores.b >= thresholds.ac_benefit;
            let active = task.status == "pending" || task.status == "in_progress";
            if substantive && active && task.acceptance_criteria.is_empty() {
                findings.push(DoctorFinding::MissingAcceptanceCriteria {
                    id: task_id_display(&task.id),
                    scores: task.scores.clone(),
                });
            }
        }

        // 6. outcome layer — done tasks without `verified` (claimed, not graded).
        // Soft advisory only; hand-built and bootstrap tasks legitimately land
        // without an external grader and ClaimedNotGraded never fails the run.
        for task in &tasks.task {
            if task.status == "done" && task.verified.is_none() {
                findings.push(DoctorFinding::ClaimedNotGraded {
                    id: task_id_display(&task.id),
                });
            }
            if task.status == "done"
                && task.verified == Some(true)
                && task
                    .verified_by
                    .as_deref()
                    .is_none_or(|verified_by| verified_by.trim().is_empty())
            {
                findings.push(DoctorFinding::VerifiedWithoutProvenance {
                    id: task_id_display(&task.id),
                });
            }
        }

        // 7. phase/focus state drift — soft advisories only; phase/focus state
        // is intentionally user-curated rather than auto-mutated.
        for (phase_key, phase) in &tasks.phases {
            let Ok(phase_id) = phase_key.parse::<u32>() else {
                continue;
            };
            let phase_tasks: Vec<_> = tasks
                .task
                .iter()
                .filter(|task| task.phase == phase_id)
                .collect();
            let task_count = phase_tasks.len();

            if task_count > 0
                && (phase.status == "in_progress" || phase.status == "pending")
                && phase_tasks.iter().all(|task| task.status == "done")
            {
                findings.push(DoctorFinding::PhaseFullyDoneButOpen {
                    phase: phase_id,
                    status: phase.status.clone(),
                    task_count,
                });
            }

            if phase.status == "pending" {
                let task_ids: Vec<String> = phase_tasks
                    .iter()
                    .filter(|task| task.status == "in_progress")
                    .map(|task| task_id_display(&task.id))
                    .collect();

                if !task_ids.is_empty() {
                    findings.push(DoctorFinding::PhaseHasInProgressButPending {
                        phase: phase_id,
                        task_ids,
                    });
                }
            }
        }

        if let Some(focus) = &tasks.focus
            && let Some(phase) = tasks.phases.get(&focus.phase.to_string())
        {
            let phase_tasks: Vec<_> = tasks
                .task
                .iter()
                .filter(|task| task.phase == focus.phase)
                .collect();
            let task_count = phase_tasks.len();
            let all_tasks_done =
                task_count > 0 && phase_tasks.iter().all(|task| task.status == "done");

            if phase.status == "done" || all_tasks_done {
                findings.push(DoctorFinding::FocusPhaseClosed {
                    phase: focus.phase,
                    status: phase.status.clone(),
                    task_count,
                });
            }
        }

        // 8. milestone state drift — soft advisories only; milestone status
        // is intentionally user-curated rather than auto-mutated.
        for (milestone_key, milestone) in &tasks.milestones {
            let pinned: Vec<_> = tasks
                .task
                .iter()
                .filter(|task| task.milestone.as_deref() == Some(milestone_key.as_str()))
                .collect();
            let task_count = pinned.len();

            if task_count > 0
                && (milestone.status == "pending" || milestone.status == "active")
                && pinned.iter().all(|task| task.status == "done")
            {
                findings.push(DoctorFinding::MilestoneFullyDoneButOpen {
                    milestone: milestone_key.clone(),
                    status: milestone.status.clone(),
                    task_count,
                });
            }
        }

        let active_milestones: Vec<ActiveMilestone> = tasks
            .milestones
            .iter()
            .filter(|(_, milestone)| milestone.status == "active")
            .map(|(key, _)| ActiveMilestone {
                milestone: key.clone(),
                task_count: pinned_task_count(tasks, key),
            })
            .collect();

        if active_milestones.len() > 1 {
            findings.push(DoctorFinding::MultipleActiveMilestones {
                milestones: active_milestones,
            });
        }

        // 9. dependency-graph health — soft advisories only; never auto-mutate.
        let task_refs: Vec<&Task> = tasks.task.iter().collect();
        let unlocks = compute_unlocks(&task_refs);
        let forward = forward_adjacency(&task_refs);
        let milestone_reachable = if tasks.milestones.is_empty() {
            None
        } else {
            Some(milestone_reachable_ids(&task_refs))
        };
        let graph_has_edges = forward.values().any(|deps| !deps.is_empty())
            || unlocks.values().any(|&count| count > 0);

        for task in &tasks.task {
            let id = task_id_display(&task.id);
            let dependent_count = unlocks.get(&id).copied().unwrap_or(0);

            if (task.status == "pending" || task.status == "blocked")
                && dependent_count >= thresholds.bottleneck_min as usize
            {
                findings.push(DoctorFinding::Bottleneck {
                    id: id.clone(),
                    dependent_count,
                });
            }

            if graph_has_edges && is_orphan(&id, &forward, dependent_count) {
                findings.push(DoctorFinding::IsolatedNode {
                    id: id.clone(),
                    reason: IsolatedNodeReason::Orphan,
                });
                continue;
            }

            if let Some(reachable) = &milestone_reachable
                && !reachable.contains(&id)
            {
                findings.push(DoctorFinding::IsolatedNode {
                    id,
                    reason: IsolatedNodeReason::UnreachableFromMilestone,
                });
            }
        }

        // 10. AC quality on live agent-assigned tasks — soft only. Same live-agent
        // predicate as validate_dispatch_model: pending/in_progress + assignee set
        // and != "human". Placeholders and vague-only criteria are token-mechanical.
        for task in &tasks.task {
            if !is_live_agent_assigned(task) {
                continue;
            }
            let id = task_id_display(&task.id);
            let mut saw_placeholder = false;
            let mut saw_vague = false;
            for criterion in &task.acceptance_criteria {
                if !saw_placeholder && criterion_has_placeholder(criterion) {
                    findings.push(DoctorFinding::PlaceholderCriteria { id: id.clone() });
                    saw_placeholder = true;
                }
                if !saw_vague && criterion_is_vague(criterion) {
                    findings.push(DoctorFinding::VagueCriteria { id: id.clone() });
                    saw_vague = true;
                }
                if saw_placeholder && saw_vague {
                    break;
                }
            }
        }

        // 11. near-duplicate open tasks — soft only. Pairwise Jaccard over
        // normalized title+body tokens; done/superseded never enter the pool.
        let open: Vec<(&Task, String, HashSet<String>)> = tasks
            .task
            .iter()
            .filter(|task| is_open_for_near_duplicate(task))
            .map(|task| {
                let id = task_id_display(&task.id);
                let tokens = title_body_tokens(task);
                (task, id, tokens)
            })
            .collect();
        for i in 0..open.len() {
            for j in (i + 1)..open.len() {
                let (_, id_a, tokens_a) = &open[i];
                let (_, id_b, tokens_b) = &open[j];
                if jaccard_at_least(tokens_a, tokens_b, thresholds.near_duplicate_min) {
                    let mut ids = vec![id_a.clone(), id_b.clone()];
                    ids.sort();
                    findings.push(DoctorFinding::NearDuplicateTasks { ids });
                }
            }
        }

        // 12. render drift — only if a roadmap was provided
        if let Some(roadmap) = roadmap_input
            && let Ok(rendered) = render_roadmap_str_with_today(roadmap, tasks, today)
            && rendered != roadmap
        {
            findings.push(DoctorFinding::Drift);
        }

        DoctorReport {
            ok: findings.is_empty(),
            findings,
            thresholds,
        }
    }
}

fn task_id_display(id: &TaskId) -> String {
    match id {
        TaskId::Number(n) => n.to_string(),
        TaskId::Text(s) => s.clone(),
    }
}

fn is_orphan(id: &str, forward: &HashMap<String, Vec<String>>, dependent_count: usize) -> bool {
    let has_in_repo_deps = forward.get(id).is_some_and(|deps| !deps.is_empty());
    !has_in_repo_deps && dependent_count == 0
}

fn pinned_task_count(tasks: &Tasks, milestone_key: &str) -> usize {
    tasks
        .task
        .iter()
        .filter(|task| task.milestone.as_deref() == Some(milestone_key))
        .count()
}

/// Live agent-assigned predicate shared with `validate_dispatch_model`:
/// `status ∈ {pending, in_progress}` AND `assignee` set AND `assignee != "human"`.
fn is_live_agent_assigned(task: &Task) -> bool {
    if task.status != "pending" && task.status != "in_progress" {
        return false;
    }
    matches!(task.assignee.as_deref(), Some(a) if !a.is_empty() && a != "human")
}

fn is_open_for_near_duplicate(task: &Task) -> bool {
    matches!(task.status.as_str(), "pending" | "in_progress" | "blocked")
}

/// Strip double-quoted spans and *non-possessive* single-quoted spans so prose
/// that *mentions* placeholders as examples (e.g. `contains 'TODO'`) does not
/// trip the lint; unquoted `TODO` still does. Mid-word apostrophes (`task's`)
/// are kept so they cannot open a span and un-quote a later example token.
fn strip_quoted_spans(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            if i < chars.len() {
                i += 1; // consume closing "
            }
            out.push(' ');
            continue;
        }
        if c == '\'' {
            let prev_is_word =
                i > 0 && (chars[i - 1].is_ascii_alphanumeric() || chars[i - 1] == '_');
            if !prev_is_word && let Some(rel) = chars[i + 1..].iter().position(|&x| x == '\'') {
                i = i + 1 + rel + 1; // skip opening, body, closing
                out.push(' ');
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

/// True when a criterion contains an unresolved placeholder after quote-stripping:
/// whole-word `TODO` / `TBD` (case-insensitive), substring `???`, or `<stub>` angle
/// brackets whose body is a simple identifier/phrase (no comparison operators).
fn criterion_has_placeholder(criterion: &str) -> bool {
    let unquoted = strip_quoted_spans(criterion);
    let lower = unquoted.to_ascii_lowercase();

    if contains_whole_word(&lower, "todo") || contains_whole_word(&lower, "tbd") {
        return true;
    }
    if unquoted.contains("???") {
        return true;
    }
    has_angle_bracket_stub(&unquoted)
}

fn contains_whole_word(haystack_lower: &str, word: &str) -> bool {
    let bytes = haystack_lower.as_bytes();
    let w = word.as_bytes();
    let mut i = 0;
    while i + w.len() <= bytes.len() {
        if &bytes[i..i + w.len()] == w {
            let before_ok = i == 0 || !is_word_char(bytes[i - 1]);
            let after_ok = i + w.len() == bytes.len() || !is_word_char(bytes[i + w.len()]);
            if before_ok && after_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

fn is_word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `<feature_name>` / `<todo>` style single-token stubs. Rejects empty `<>` and
/// multi-token / comparison-like spans.
fn has_angle_bracket_stub(s: &str) -> bool {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<'
            && let Some(end) = bytes[i + 1..].iter().position(|&b| b == b'>')
        {
            let inner = &s[i + 1..i + 1 + end];
            if is_stub_body(inner) {
                return true;
            }
            i += 1 + end + 1;
            continue;
        }
        i += 1;
    }
    false
}

fn is_stub_body(inner: &str) -> bool {
    let trimmed = inner.trim();
    // Single-token stubs only (`<feature_name>`, `<todo>`). Reject spaces so
    // comparison prose like `a < b and c > d` never matches.
    if trimmed.is_empty() || trimmed.len() > 64 || trimmed.contains(' ') {
        return false;
    }
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// True when every non-stopword token is in [`VAGUE_WORDS`] and at least one such
/// token exists. "works properly" → true; "renders in under 200ms so it feels fast" → false.
fn criterion_is_vague(criterion: &str) -> bool {
    let tokens = alphanumeric_tokens(criterion);
    let content: Vec<&str> = tokens
        .iter()
        .map(String::as_str)
        .filter(|t| !VAGUE_STOPWORDS.contains(t))
        .collect();
    !content.is_empty() && content.iter().all(|t| VAGUE_WORDS.contains(t))
}

fn alphanumeric_tokens(s: &str) -> Vec<String> {
    let lower = s.to_ascii_lowercase();
    let mut tokens = Vec::new();
    let mut current = String::new();
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            current.push(c);
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn title_body_tokens(task: &Task) -> HashSet<String> {
    let mut text = task.title.clone();
    if let Some(body) = &task.body {
        text.push(' ');
        text.push_str(body);
    }
    alphanumeric_tokens(&text).into_iter().collect()
}

/// Integer Jaccard: `|A∩B| / |A∪B| >= min_percent/100`. Empty sets never pair.
fn jaccard_at_least(a: &HashSet<String>, b: &HashSet<String>, min_percent: u32) -> bool {
    if a.is_empty() || b.is_empty() {
        return false;
    }
    let inter = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        return false;
    }
    inter * 100 >= union * min_percent as usize
}

impl fmt::Display for DoctorReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.findings.is_empty() {
            return write!(f, "rmap doctor — all checks passed");
        }

        let count = self.findings.len();
        writeln!(
            f,
            "rmap doctor — {count} finding{}",
            if count == 1 { "" } else { "s" }
        )?;

        let validate_findings: Vec<&str> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::Validate { message } = fi {
                    Some(message.as_str())
                } else {
                    None
                }
            })
            .collect();

        if !validate_findings.is_empty() {
            writeln!(f, "\nValidation:")?;
            for msg in validate_findings {
                writeln!(f, "  - {msg}")?;
            }
        }

        let stale_findings: Vec<(&str, &str, i64)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::Stale {
                    id,
                    started_at,
                    days_idle,
                } = fi
                {
                    Some((id.as_str(), started_at.as_str(), *days_idle))
                } else {
                    None
                }
            })
            .collect();

        if !stale_findings.is_empty() {
            writeln!(f, "\nStale (in-progress > {}d):", self.thresholds.days)?;
            for (id, started_at, days) in stale_findings {
                writeln!(f, "  - task {id} — started {started_at} ({days} days idle)")?;
            }
        }

        let decay_findings: Vec<(&str, Option<&str>)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::ScoreDecay { id, scored_at } = fi {
                    Some((id.as_str(), scored_at.as_deref()))
                } else {
                    None
                }
            })
            .collect();

        if !decay_findings.is_empty() {
            writeln!(f, "\nScore decay (> {}d or missing):", self.thresholds.days)?;
            for (id, scored_at) in decay_findings {
                let label = scored_at.unwrap_or("<missing>");
                writeln!(f, "  - task {id} — scored_at: {label}")?;
            }
        }

        let degenerate_findings: Vec<(&str, u32, usize)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::DegenerateBundle {
                    bundle,
                    phase,
                    task_count,
                } = fi
                {
                    Some((bundle.as_str(), *phase, *task_count))
                } else {
                    None
                }
            })
            .collect();

        if !degenerate_findings.is_empty() {
            writeln!(f, "\nDegenerate bundles (cover entire phase):")?;
            for (bundle, phase, count) in degenerate_findings {
                writeln!(
                    f,
                    "  - Bundle \"{bundle}\" contains all {count} tasks of phase {phase}; consider clustering across phases or removing the bundle."
                )?;
            }
        }

        let missing_ac_findings: Vec<(&str, &Scores)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::MissingAcceptanceCriteria { id, scores } = fi {
                    Some((id.as_str(), scores))
                } else {
                    None
                }
            })
            .collect();

        if !missing_ac_findings.is_empty() {
            writeln!(
                f,
                "\nMissing acceptance_criteria (D >= {} or B >= {}):",
                self.thresholds.ac_difficulty, self.thresholds.ac_benefit
            )?;
            for (id, scores) in missing_ac_findings {
                writeln!(
                    f,
                    "  - task {id} [D:{}/B:{}/U:{}] — add acceptance_criteria",
                    scores.d, scores.b, scores.u
                )?;
            }
        }

        let claimed_not_graded_findings: Vec<&str> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::ClaimedNotGraded { id } = fi {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();

        if !claimed_not_graded_findings.is_empty() {
            writeln!(f, "\nClaimed, not graded (done without `verified`):")?;
            for id in claimed_not_graded_findings {
                writeln!(
                    f,
                    "  - task {id} — set `--verified` on `rmap status done` once an independent check passes"
                )?;
            }
        }

        let provenance_findings: Vec<&str> = self
            .findings
            .iter()
            .filter_map(|finding| {
                if let DoctorFinding::VerifiedWithoutProvenance { id } = finding {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();

        if !provenance_findings.is_empty() {
            writeln!(
                f,
                "\nVerified without provenance (`verified = true` but no `verified_by`):"
            )?;
            for id in provenance_findings {
                writeln!(
                    f,
                    "  - task {id} — record the independent evaluator in `verified_by`"
                )?;
            }
        }

        let phase_done_open_findings: Vec<(u32, &str, usize)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::PhaseFullyDoneButOpen {
                    phase,
                    status,
                    task_count,
                } = fi
                {
                    Some((*phase, status.as_str(), *task_count))
                } else {
                    None
                }
            })
            .collect();

        if !phase_done_open_findings.is_empty() {
            writeln!(f, "\nPhase fully done but open:")?;
            for (phase, status, count) in phase_done_open_findings {
                writeln!(
                    f,
                    "  - phase {phase} has all {count} tasks done but phase status is `{status}`; advance the phase status when ready"
                )?;
            }
        }

        let phase_pending_started_findings: Vec<(u32, &Vec<String>)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::PhaseHasInProgressButPending { phase, task_ids } = fi {
                    Some((*phase, task_ids))
                } else {
                    None
                }
            })
            .collect();

        if !phase_pending_started_findings.is_empty() {
            writeln!(f, "\nPhase has in-progress tasks but is pending:")?;
            for (phase, task_ids) in phase_pending_started_findings {
                writeln!(
                    f,
                    "  - phase {phase} is pending but has in-progress {}; set the phase status when ready",
                    task_refs(task_ids)
                )?;
            }
        }

        let focus_closed_findings: Vec<(u32, &str, usize)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::FocusPhaseClosed {
                    phase,
                    status,
                    task_count,
                } = fi
                {
                    Some((*phase, status.as_str(), *task_count))
                } else {
                    None
                }
            })
            .collect();

        if !focus_closed_findings.is_empty() {
            writeln!(f, "\nFocus phase closed:")?;
            for (phase, status, count) in focus_closed_findings {
                writeln!(
                    f,
                    "  - phase {phase} is the focus phase but appears closed (status `{status}`, {count} tasks); move focus.phase when ready"
                )?;
            }
        }

        let milestone_done_open_findings: Vec<(&str, &str, usize)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::MilestoneFullyDoneButOpen {
                    milestone,
                    status,
                    task_count,
                } = fi
                {
                    Some((milestone.as_str(), status.as_str(), *task_count))
                } else {
                    None
                }
            })
            .collect();

        if !milestone_done_open_findings.is_empty() {
            writeln!(f, "\nMilestone fully done but open:")?;
            for (milestone, status, count) in milestone_done_open_findings {
                writeln!(
                    f,
                    "  - milestone {milestone} has all {count} pinned tasks done but milestone status is `{status}`; advance the milestone status when ready"
                )?;
            }
        }

        let multiple_active_findings: Vec<&[ActiveMilestone]> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::MultipleActiveMilestones { milestones } = fi {
                    Some(milestones.as_slice())
                } else {
                    None
                }
            })
            .collect();

        if !multiple_active_findings.is_empty() {
            writeln!(f, "\nMultiple active milestones:")?;
            for milestones in multiple_active_findings {
                writeln!(
                    f,
                    "  - milestones {} are all active; keep exactly one milestone at status='active'",
                    milestone_refs_with_counts(milestones)
                )?;
            }
        }

        let bottleneck_findings: Vec<(&str, usize)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::Bottleneck {
                    id,
                    dependent_count,
                } = fi
                {
                    Some((id.as_str(), *dependent_count))
                } else {
                    None
                }
            })
            .collect();

        if !bottleneck_findings.is_empty() {
            writeln!(
                f,
                "\nGraph bottleneck (>= {} transitive dependents, pending/blocked):",
                self.thresholds.bottleneck_min
            )?;
            for (id, count) in bottleneck_findings {
                writeln!(
                    f,
                    "  - task {id} gates {count} others — finish or unblock this task to release downstream work"
                )?;
            }
        }

        let isolated_findings: Vec<(&str, &IsolatedNodeReason)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::IsolatedNode { id, reason } = fi {
                    Some((id.as_str(), reason))
                } else {
                    None
                }
            })
            .collect();

        if !isolated_findings.is_empty() {
            writeln!(f, "\nIsolated / unreachable nodes:")?;
            for (id, reason) in isolated_findings {
                let label = match reason {
                    IsolatedNodeReason::Orphan => "orphan (no deps, no dependents)",
                    IsolatedNodeReason::UnreachableFromMilestone => {
                        "unreachable from any milestone"
                    }
                };
                writeln!(f, "  - task {id} — {label}")?;
            }
        }

        let placeholder_findings: Vec<&str> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::PlaceholderCriteria { id } = fi {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();

        if !placeholder_findings.is_empty() {
            writeln!(
                f,
                "\nPlaceholder acceptance criteria (TODO/TBD/???/<stub> on live agent-assigned tasks):"
            )?;
            for id in placeholder_findings {
                writeln!(
                    f,
                    "  - task {id} — replace placeholder tokens in acceptance_criteria with measurable criteria"
                )?;
            }
        }

        let vague_findings: Vec<&str> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::VagueCriteria { id } = fi {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();

        if !vague_findings.is_empty() {
            writeln!(
                f,
                "\nVague / unmeasurable acceptance criteria (live agent-assigned):"
            )?;
            for id in vague_findings {
                writeln!(
                    f,
                    "  - task {id} — criterion uses only vague wording ({}); add a measurable object",
                    VAGUE_WORDS.join("/")
                )?;
            }
        }

        let near_dup_findings: Vec<&[String]> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::NearDuplicateTasks { ids } = fi {
                    Some(ids.as_slice())
                } else {
                    None
                }
            })
            .collect();

        if !near_dup_findings.is_empty() {
            writeln!(
                f,
                "\nNear-duplicate open tasks (title+body Jaccard >= {}%):",
                self.thresholds.near_duplicate_min
            )?;
            for ids in near_dup_findings {
                writeln!(
                    f,
                    "  - tasks {} look near-identical — refine rather than duplicate",
                    task_refs(ids)
                )?;
            }
        }

        let has_drift = self
            .findings
            .iter()
            .any(|fi| matches!(fi, DoctorFinding::Drift));

        if has_drift {
            writeln!(f, "\nDrift:")?;
            writeln!(
                f,
                "  - ROADMAP.md is out of sync with current rendered output. Run `rmap render`."
            )?;
        }

        Ok(())
    }
}

fn task_refs(task_ids: &[String]) -> String {
    task_ids
        .iter()
        .map(|id| format!("task {id}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn milestone_refs_with_counts(milestones: &[ActiveMilestone]) -> String {
    milestones
        .iter()
        .map(|m| format!("{} ({} pinned)", m.milestone, m.task_count))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_detects_unquoted_todo_tbd_and_stubs() {
        assert!(criterion_has_placeholder("TODO finish the API surface"));
        assert!(criterion_has_placeholder("details are TBD"));
        assert!(criterion_has_placeholder("still ??? here"));
        assert!(criterion_has_placeholder(
            "implement <feature_name> handler"
        ));
        assert!(criterion_has_placeholder("todo: lowercase whole word"));
    }

    #[test]
    fn placeholder_ignores_quoted_examples_and_non_stubs() {
        assert!(!criterion_has_placeholder(
            "quoted examples like 'TODO' or 'TBD' in prose do not count"
        ));
        assert!(!criterion_has_placeholder(
            "contains 'TBD' as prose example"
        ));
        // Possessive apostrophe must not open a quote span and un-quote later examples.
        assert!(!criterion_has_placeholder(
            "a live agent-assigned task's acceptance_criteria contains 'TODO' or 'TBD'"
        ));
        assert!(!criterion_has_placeholder(
            "compare a < b and c > d without stubs"
        ));
        assert!(!criterion_has_placeholder("no placeholders at all"));
        assert!(!criterion_has_placeholder("empty <> is not a stub"));
    }

    #[test]
    fn vague_detects_only_vague_wording() {
        assert!(criterion_is_vague("works properly"));
        assert!(criterion_is_vague("it works"));
        assert!(criterion_is_vague("fast and robust"));
        assert!(criterion_is_vague("correctly"));
        assert!(!criterion_is_vague(
            "renders in under 200ms so it feels fast"
        ));
        assert!(!criterion_is_vague("tests pass under cargo test"));
        assert!(!criterion_is_vague(""));
    }

    #[test]
    fn jaccard_threshold_integer_percent() {
        let a: HashSet<String> = ["add", "auth", "flow", "login"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let b: HashSet<String> = ["add", "auth", "flow", "signup"]
            .into_iter()
            .map(str::to_string)
            .collect();
        // |∩|=3, |∪|=5 → 60%
        assert!(jaccard_at_least(&a, &b, 60));
        assert!(!jaccard_at_least(&a, &b, 61));
        assert!(!jaccard_at_least(&a, &HashSet::new(), 50));
    }
}
