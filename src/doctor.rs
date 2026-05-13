use std::collections::HashSet;
use std::fmt;

use crate::render::render_roadmap_str_with_today;
use crate::schema::{Scores, TaskId, Tasks};
use crate::scoring::{SCORE_DECAY_DAYS, days_since};
use crate::stale::find_stale;
use crate::validate;

const STALE_THRESHOLD_DAYS: u32 = 30;

/// Minimum D or B at which a pending/in_progress task without `acceptance_criteria`
/// triggers the `missing_acceptance_criteria` doctor finding. Substantive tasks should
/// declare what "done" means; trivial tasks can rely on title-as-prompt.
const AC_DIFFICULTY_THRESHOLD: u32 = 5;
const AC_BENEFIT_THRESHOLD: u32 = 8;

#[derive(serde::Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub findings: Vec<DoctorFinding>,
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
    Drift,
}

impl DoctorReport {
    /// Builds a composite health report: validation findings (non-short-circuiting),
    /// stale in-progress tasks (>`STALE_THRESHOLD_DAYS` via `started_at`), score-decay
    /// candidates (`scored_at` missing or >`SCORE_DECAY_DAYS`), and render drift (when
    /// `roadmap_input` is supplied). Pure — no I/O. `today` is the `YYYY-MM-DD`
    /// reference date.
    pub fn run(
        tasks: &Tasks,
        path: &str,
        input: &str,
        roadmap_input: Option<&str>,
        today: &str,
    ) -> Self {
        let mut findings: Vec<DoctorFinding> = Vec::new();

        // 1. validate findings (all checks, no short-circuit)
        for err in validate::collect_findings(tasks, path, input) {
            findings.push(DoctorFinding::Validate {
                message: err.to_string(),
            });
        }

        // 2. stale in-progress tasks
        for task in find_stale(tasks, STALE_THRESHOLD_DAYS, today) {
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
                    .map(|d| d > SCORE_DECAY_DAYS)
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
                task.scores.d >= AC_DIFFICULTY_THRESHOLD || task.scores.b >= AC_BENEFIT_THRESHOLD;
            let active = task.status == "pending" || task.status == "in_progress";
            if substantive && active && task.acceptance_criteria.is_empty() {
                findings.push(DoctorFinding::MissingAcceptanceCriteria {
                    id: task_id_display(&task.id),
                    scores: task.scores.clone(),
                });
            }
        }

        // 6. render drift — only if a roadmap was provided
        if let Some(roadmap) = roadmap_input
            && let Ok(rendered) = render_roadmap_str_with_today(roadmap, tasks, today)
            && rendered != roadmap
        {
            findings.push(DoctorFinding::Drift);
        }

        DoctorReport {
            ok: findings.is_empty(),
            findings,
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
            writeln!(f, "\nStale (in-progress > {STALE_THRESHOLD_DAYS}d):")?;
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
            writeln!(f, "\nScore decay (> {SCORE_DECAY_DAYS}d or missing):")?;
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
                "\nMissing acceptance_criteria (D >= {AC_DIFFICULTY_THRESHOLD} or B >= {AC_BENEFIT_THRESHOLD}):"
            )?;
            for (id, scores) in missing_ac_findings {
                writeln!(
                    f,
                    "  - task {id} [D:{}/B:{}/U:{}] — add acceptance_criteria",
                    scores.d, scores.b, scores.u
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
