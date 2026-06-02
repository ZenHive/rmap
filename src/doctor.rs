use std::collections::HashSet;
use std::fmt;

use crate::render::render_roadmap_str_with_today;
use crate::schema::{Scores, TaskId, Tasks};
use crate::scoring::days_since;
use crate::stale::find_stale;
use crate::validate;

/// Default cutoff (in days) for both the stale-in-progress check and the
/// score-decay check. Overridable per-invocation via `rmap doctor --threshold-days`.
const STALE_THRESHOLD_DAYS: u32 = 30;

/// Minimum D or B at which a pending/in_progress task without `acceptance_criteria`
/// triggers the `missing_acceptance_criteria` doctor finding. Substantive tasks should
/// declare what "done" means; trivial tasks can rely on title-as-prompt.
const AC_DIFFICULTY_THRESHOLD: u32 = 5;
const AC_BENEFIT_THRESHOLD: u32 = 8;

/// Effective doctor thresholds for one `rmap doctor` invocation: the constant
/// defaults above unless overridden by `--threshold-days` / `--ac-threshold`.
/// `--threshold-days` collapses the stale and score-decay cutoffs into one
/// `days` value; `--ac-threshold` collapses the distinct D/B defaults into one.
#[derive(serde::Serialize, Clone, Copy)]
pub struct DoctorThresholds {
    pub days: u32,
    pub ac_difficulty: u32,
    pub ac_benefit: u32,
}

impl DoctorThresholds {
    pub fn resolve(threshold_days: Option<u32>, ac_threshold: Option<u32>) -> Self {
        Self {
            days: threshold_days.unwrap_or(STALE_THRESHOLD_DAYS),
            ac_difficulty: ac_threshold.unwrap_or(AC_DIFFICULTY_THRESHOLD),
            ac_benefit: ac_threshold.unwrap_or(AC_BENEFIT_THRESHOLD),
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
    Drift,
}

impl DoctorReport {
    /// Builds a composite health report: validation findings (non-short-circuiting),
    /// stale in-progress tasks (>`thresholds.days` via `started_at`), score-decay
    /// candidates (`scored_at` missing or >`thresholds.days`), and render drift (when
    /// `roadmap_input` is supplied). Pure — no I/O. `today` is the `YYYY-MM-DD`
    /// reference date. `thresholds` carries the effective stale/decay and AC cutoffs
    /// for this invocation (defaults unless overridden on the CLI).
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

        // 8. render drift — only if a roadmap was provided
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
