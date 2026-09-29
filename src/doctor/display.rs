use super::*;

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

        let awaiting_findings: Vec<(&str, &str)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::AwaitingLanding { id, landing_ref } = fi {
                    Some((id.as_str(), landing_ref.as_str()))
                } else {
                    None
                }
            })
            .collect();

        if !awaiting_findings.is_empty() {
            writeln!(f, "\nawaiting landing ({})", awaiting_findings.len())?;
            for (id, landing_ref) in awaiting_findings {
                writeln!(f, "  - task {id} — {landing_ref}")?;
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

        let untested_rules: Vec<(&str, &str)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::UntestedRule { rule, file } = fi {
                    Some((rule.as_str(), file.as_str()))
                } else {
                    None
                }
            })
            .collect();

        if !untested_rules.is_empty() {
            writeln!(f, "\nUntested spec rules (active, no tagging test):")?;
            for (rule, file) in untested_rules {
                writeln!(f, "  - rule {rule} in {file} — no test tags this rule")?;
            }
        }

        let unknown_tags: Vec<(&str, &str)> = self
            .findings
            .iter()
            .filter_map(|fi| {
                if let DoctorFinding::UnknownRuleTag { rule, file } = fi {
                    Some((rule.as_str(), file.as_str()))
                } else {
                    None
                }
            })
            .collect();

        if !unknown_tags.is_empty() {
            writeln!(f, "\nUnknown spec rule tags:")?;
            for (rule, file) in unknown_tags {
                writeln!(
                    f,
                    "  - rule {rule} in {file} — tag names a rule in no registered spec"
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
