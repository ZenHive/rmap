use super::*;

use crate::schema::Status;

/// Transition-time fields written by a status change, grouped into one argument
/// so the status mutators stay under clippy's argument-count ceiling as the
/// outcome layer grows. All are optional; `None` leaves the field untouched.
/// Each field is gated on its matching transition: the outcome fields (`implemented`,
/// `delivered_by`, `verified`, `verified_by`, `verification_ref`, `shipped_in`)
/// apply only on a `done` transition;
/// `blocked_reason` applies only on a `blocked` transition; `attempt_report` /
/// `attempt_by` apply only on a `pending` transition; `landing_ref` applies
/// only on an `in_progress` transition (hard-rejected otherwise, not ignored).
/// On any other transition the relevant fields are ignored. `Default` yields
/// all-`None` (the common "just change the status" case).
#[derive(Default, Clone, Copy)]
pub struct TransitionFields<'a> {
    /// What was actually built (overwrites any existing value when `Some`).
    /// Done-only.
    pub implemented: Option<&'a str>,
    /// Which agent or instance shipped the task (free-text). Done-only.
    pub delivered_by: Option<&'a str>,
    /// Independent-evaluator confirmation. `Some(true)` = a grader agreed.
    /// Done-only.
    pub verified: Option<bool>,
    /// Independent evaluator behind `verified = true`. Done-only and required
    /// whenever this transition writes a positive verification claim.
    pub verified_by: Option<&'a str>,
    /// Durable pointer to verification evidence (harness run, CI URL, review
    /// artifact). Done-only and meaningful only with `verified = true`.
    pub verification_ref: Option<&'a str>,
    /// Where the work landed — commit SHA / PR ref (free-text). Done-only.
    pub shipped_in: Option<&'a str>,
    /// Why the task is blocked (free-text). Blocked-only: written on a `blocked`
    /// transition, auto-cleared when the task leaves the blocked state.
    pub blocked_reason: Option<&'a str>,
    /// Failure evidence for the attempt that just sent this task back to the
    /// queue (free-text — a reviewer's rejection report). Pending-only: when
    /// `Some`, a new entry is **appended** to the task's `attempts` history
    /// (never overwritten), timestamped with `today_iso()`.
    pub attempt_report: Option<&'a str>,
    /// Which agent the appended attempt is attributed to (free-text). Only
    /// meaningful alongside `attempt_report` on a `pending` transition.
    pub attempt_by: Option<&'a str>,
    /// Open landing pointer (PR URL or other free-text ref). In-progress-only:
    /// written on an `in_progress` transition (overwriting any existing value);
    /// `Some("")` clears the field. Rejected on any other target status.
    /// Kept automatically on `done`/`blocked`; cleared automatically on
    /// `pending`.
    pub landing_ref: Option<&'a str>,
}

/// Atomically flip the `status` field on every task in `ids` to `new_status`.
/// All-or-nothing: if any ID is missing, returns `UnknownTaskId` and the input
/// string is not modified. Duplicates in `ids` are idempotent; an empty slice
/// errors with `EmptyIds`. Preserves TOML comments and whitespace via `toml_edit`.
///
/// Transitioning into `done` auto-fills `done_at = today_iso()` when absent;
/// transitioning into `in_progress` auto-fills `started_at = today_iso()` when
/// absent. Existing timestamps are never overwritten — re-runs are idempotent,
/// and re-opening a task (`done → pending → done`) preserves the original
/// `done_at`. Other transitions (`pending`, `blocked`, `superseded`) do not
/// touch lifecycle timestamps. When a timestamp is newly inserted, the task's
/// keys are re-sorted into canonical order so the field lands near the other
/// small scalars instead of being appended after multi-line entries.
///
/// `implemented`: when transitioning to `done` and a value is provided, write
/// it to every matched task's `implemented` field (overwriting any existing
/// value — the transition-time content is the most current). Ignored on
/// non-`done` transitions. When `None`, no write; the conditional-required
/// `validate_implemented` check will reject the transition unless the task
/// already carries an `implemented` field.
///
/// `fields.delivered_by` / `fields.verified` / `fields.verified_by` /
/// `fields.verification_ref` / `fields.shipped_in`: outcome-layer
/// transition-time fields. Mirror `implemented`'s semantics — write on `done`
/// transitions when `Some`, overwrite any existing value, ignored on non-`done`
/// transitions. `verified = Some(true)` means an independent check passed; absent
/// means not yet graded. `shipped_in` records where the work landed (commit/PR,
/// free-text). All are optional (the validator does not require them);
/// `rmap doctor` emits a soft "claimed, not graded" advisory when `done` and
/// `verified` is absent.
///
/// `fields.blocked_reason`: written only on a `blocked` transition (overwriting
/// any existing value), ignored otherwise. Conversely, leaving the blocked state
/// (current status `blocked`, new status anything else) auto-removes the stale
/// `blocked_reason` — it described a state that no longer holds. Re-blocking
/// (`blocked → blocked`) keeps or overwrites the existing reason.
///
/// `fields.attempt_report` / `fields.attempt_by`: written only on a `pending`
/// transition, ignored otherwise. When `attempt_report` is `Some`, a new
/// `attempts` entry is **appended** (never overwritten) — `{ at = today_iso(),
/// by = attempt_by?, report }` — recording why the prior dispatch attempt was
/// rejected so the next implementer sees the history. Each call appends exactly
/// one entry; re-running appends again (append semantics, no dedup).
///
/// `fields.landing_ref`: written only on an `in_progress` transition. `Some("")`
/// clears the field; any other `Some` overwrites. Passing the flag on any other
/// target status is a hard error (`LandingRefWrongStatus`) and leaves the file
/// byte-equal. An `in_progress` → `in_progress` call with only `--landing-ref`
/// is a field update: `started_at` is never overwritten. Lifecycle without the
/// flag: kept on `done`/`blocked` (provenance / closed-unmerged PR), cleared on
/// `pending` (the work is being redone).
pub fn update_status_many_str(
    path: impl Into<String>,
    input: &str,
    ids: &[&str],
    new_status: &str,
    fields: TransitionFields<'_>,
) -> Result<String, MutateError> {
    if ids.is_empty() {
        return Err(MutateError::EmptyIds);
    }

    // Parse once so every gate matches a variant. The original wire string is
    // what gets written, including values validation will later reject.
    let status = Status::from(new_status);

    if status == Status::Done
        && fields.verified == Some(true)
        && fields
            .verified_by
            .is_none_or(|verified_by| verified_by.trim().is_empty())
    {
        return Err(MutateError::MissingVerifiedBy);
    }
    if status == Status::Done
        && fields.verified != Some(true)
        && (fields.verified_by.is_some() || fields.verification_ref.is_some())
    {
        return Err(MutateError::ProvenanceWithoutVerification);
    }
    if fields.landing_ref.is_some() && status != Status::InProgress {
        return Err(MutateError::LandingRefWrongStatus(new_status.to_string()));
    }

    let path = path.into();
    let mut document =
        DocumentMut::from_str(input).map_err(|err| MutateError::Toml(err.to_string()))?;
    let tasks = document["task"]
        .as_array_of_tables_mut()
        .ok_or(MutateError::MissingTasks)?;

    let target: HashSet<&str> = ids.iter().copied().collect();
    let mut matched: HashSet<&str> = HashSet::new();
    let timestamp_field = match &status {
        Status::Done => Some("done_at"),
        Status::InProgress => Some("started_at"),
        Status::Pending | Status::Blocked | Status::Superseded | Status::Unknown(_) => None,
    };
    // Each transition-field group is gated on its matching new status: the
    // done-only outcome fields apply on `done`, the blocked-only reason applies
    // on `blocked`. Everything else is ignored (set to `None`).
    let TransitionFields {
        implemented,
        delivered_by,
        verified,
        verified_by,
        verification_ref,
        shipped_in,
        blocked_reason,
        attempt_report,
        attempt_by,
        landing_ref,
    } = fields;
    let (implemented, delivered_by, verified, verified_by, verification_ref, shipped_in) =
        if status == Status::Done {
            (
                implemented,
                delivered_by,
                verified,
                verified_by,
                verification_ref,
                shipped_in,
            )
        } else {
            (None, None, None, None, None, None)
        };
    let blocked_reason = if status == Status::Blocked {
        blocked_reason
    } else {
        None
    };
    let (attempt_report, attempt_by) = if status == Status::Pending {
        (attempt_report, attempt_by)
    } else {
        (None, None)
    };
    let landing_ref = if status == Status::InProgress {
        landing_ref
    } else {
        None
    };
    // Snapshot once per call so a bulk transition that straddles midnight
    // stamps every matched task with the same date.
    let today = timestamp_field.map(|_| crate::today_iso());
    // Same snapshot rule for an appended attempt's `at` timestamp.
    let attempt_today = attempt_report.map(|_| crate::today_iso());

    for task in tasks.iter_mut() {
        let Some(id) = task.get("id").and_then(item_to_task_id) else {
            continue;
        };

        if target.contains(id.as_str()) {
            let was_blocked = task
                .get("status")
                .and_then(|item| item.as_str())
                .is_some_and(|value| Status::Blocked == value);
            task["status"] = Item::Value(Value::from(new_status));

            let mut needs_sort = false;

            if let Some(field) = timestamp_field
                && !task.contains_key(field)
            {
                let today = today.as_deref().expect("today set when timestamp_field is");
                task.insert(field, Item::Value(Value::from(today)));
                needs_sort = true;
            }

            if let Some(value) = implemented {
                let was_present = task.contains_key("implemented");
                task.insert("implemented", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = delivered_by {
                let was_present = task.contains_key("delivered_by");
                task.insert("delivered_by", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = verified {
                let was_present = task.contains_key("verified");
                task.insert("verified", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = verified_by {
                let was_present = task.contains_key("verified_by");
                task.insert("verified_by", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = verification_ref {
                let was_present = task.contains_key("verification_ref");
                task.insert("verification_ref", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = shipped_in {
                let was_present = task.contains_key("shipped_in");
                task.insert("shipped_in", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = blocked_reason {
                let was_present = task.contains_key("blocked_reason");
                task.insert("blocked_reason", Item::Value(Value::from(value)));
                if !was_present {
                    needs_sort = true;
                }
            }

            if let Some(value) = landing_ref {
                if value.is_empty() {
                    task.remove("landing_ref");
                } else {
                    let was_present = task.contains_key("landing_ref");
                    task.insert("landing_ref", Item::Value(Value::from(value)));
                    if !was_present {
                        needs_sort = true;
                    }
                }
            }

            // Redoing the work drops the old landing pointer — it belongs in
            // the appended `attempts` report, not on the new pending row.
            if status == Status::Pending {
                task.remove("landing_ref");
            }

            // Leaving the blocked state drops the now-stale reason (gating above
            // guarantees `blocked_reason` is `None` here, so no write to undo).
            if was_blocked && status != Status::Blocked {
                task.remove("blocked_reason");
            }

            // Append one attempt entry (history, never overwrite). Stored as an
            // inline table per entry, mirroring `cross_repo`.
            if let Some(report) = attempt_report {
                let was_present = task.contains_key("attempts");
                {
                    let entry = task
                        .entry("attempts")
                        .or_insert(Item::Value(Value::Array(Array::new())));
                    let array = entry
                        .as_array_mut()
                        .ok_or_else(|| MutateError::Toml("attempts is not an array".to_string()))?;
                    let mut inline = InlineTable::new();
                    let at = attempt_today
                        .as_deref()
                        .expect("attempt_today set when attempt_report is");
                    inline.insert("at", Value::from(at));
                    if let Some(by) = attempt_by {
                        inline.insert("by", Value::from(by));
                    }
                    inline.insert("report", Value::from(report));
                    array.push(Value::InlineTable(inline));
                }
                if !was_present {
                    needs_sort = true;
                }
            }

            if needs_sort {
                task.sort_values_by(|a, _, b, _| {
                    canonical_task_key_index(a.get()).cmp(&canonical_task_key_index(b.get()))
                });
            }

            // Record which input ID this matched (preserving the original &str).
            if let Some(&orig) = ids.iter().find(|&&s| s == id.as_str()) {
                matched.insert(orig);
            }
        }
    }

    // Report the first unmatched ID in slice order for determinism.
    if let Some(&missing) = ids.iter().find(|&&s| !matched.contains(s)) {
        return Err(MutateError::UnknownTaskId(missing.to_string()));
    }

    let output = document.to_string();
    validate_tasks_str(path, &output)?;
    Ok(output)
}

pub fn update_status_str(
    path: impl Into<String>,
    input: &str,
    task_id: &str,
    new_status: &str,
    fields: TransitionFields<'_>,
) -> Result<String, MutateError> {
    update_status_many_str(path, input, &[task_id], new_status, fields)
}
