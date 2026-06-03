use std::fmt::Write;

use thiserror::Error;

use crate::milestones::{MilestoneFilter, MilestoneSummary, list_milestones};
use crate::next::next_task;
use crate::query::TaskFilter;
use crate::schema::{Task, Tasks};
use crate::scoring::{days_since, efficiency, format_efficiency, score_decay_suffix, tier_glyph};

const BEGIN_MARKER: &str = "<!-- TASKS:BEGIN phase=";
const END_MARKER: &str = "<!-- TASKS:END -->";

const FOCUS_BEGIN_MARKER: &str = "<!-- FOCUS:BEGIN -->";
const FOCUS_END_MARKER: &str = "<!-- FOCUS:END -->";

const VISION_BEGIN_MARKER: &str = "<!-- VISION:BEGIN -->";
const VISION_END_MARKER: &str = "<!-- VISION:END -->";

const MERMAID_BEGIN_MARKER: &str = "<!-- MERMAID:BEGIN -->";
const MERMAID_END_MARKER: &str = "<!-- MERMAID:END -->";

const MILESTONES_BEGIN_MARKER: &str = "<!-- MILESTONES:BEGIN -->";
const MILESTONES_END_MARKER: &str = "<!-- MILESTONES:END -->";

/// Tasks whose `done_at` is within this many days of `today` count as "recently
/// shipped" in the FOCUS block. Older shipments don't appear; the line falls
/// back to "no recent shipments".
const SHIPPED_WINDOW_DAYS: i64 = 7;

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("missing <!-- TASKS:END --> for marker starting at byte {start}")]
    MissingEndMarker { start: usize },
    #[error("invalid phase marker at byte {start}")]
    InvalidPhaseMarker { start: usize },
    #[error("missing <!-- FOCUS:END --> for marker starting at byte {start}")]
    MissingFocusEndMarker { start: usize },
    #[error("missing <!-- VISION:END --> for marker starting at byte {start}")]
    MissingVisionEndMarker { start: usize },
    #[error("missing <!-- MERMAID:END --> for marker starting at byte {start}")]
    MissingMermaidEndMarker { start: usize },
    #[error("missing <!-- MILESTONES:END --> for marker starting at byte {start}")]
    MissingMilestonesEndMarker { start: usize },
}

/// Render with an explicit `today` date (`YYYY-MM-DD`). Pass `""` only when
/// the input has no MERMAID markers — empty `today` disables decay suffixes
/// but also produces malformed gantt rows for in_progress/blocked tasks.
pub fn render_roadmap_str_with_today(
    roadmap: &str,
    tasks: &Tasks,
    today: &str,
) -> Result<String, RenderError> {
    let after_focus = render_focus_pass(roadmap, tasks, today)?;
    let after_vision = render_vision_pass(&after_focus, tasks)?;
    let after_mermaid = render_mermaid_pass(&after_vision, tasks, today)?;
    let after_milestones = render_milestones_pass(&after_mermaid, tasks)?;
    render_tasks_pass(&after_milestones, tasks, today)
}

/// Render using `today_iso()` (reads `RMAP_TODAY` env var or the system clock).
pub fn render_roadmap_str(roadmap: &str, tasks: &Tasks) -> Result<String, RenderError> {
    render_roadmap_str_with_today(roadmap, tasks, &crate::today_iso())
}

fn render_focus_pass(roadmap: &str, tasks: &Tasks, today: &str) -> Result<String, RenderError> {
    let Some(begin) = roadmap.find(FOCUS_BEGIN_MARKER) else {
        return Ok(roadmap.to_string());
    };
    let marker_line_end = roadmap[begin..]
        .find('\n')
        .map(|offset| begin + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());
    let end = roadmap[marker_line_end..]
        .find(FOCUS_END_MARKER)
        .map(|offset| marker_line_end + offset)
        .ok_or(RenderError::MissingFocusEndMarker { start: begin })?;
    let end_line_end = roadmap[end..]
        .find('\n')
        .map(|offset| end + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());

    let mut rendered = String::with_capacity(roadmap.len());
    rendered.push_str(&roadmap[..marker_line_end]);
    rendered.push_str(&render_focus_body(tasks, today));
    rendered.push_str(&roadmap[end..end_line_end]);
    rendered.push_str(&roadmap[end_line_end..]);
    Ok(rendered)
}

fn render_focus_body(tasks: &Tasks, today: &str) -> String {
    let mut body = String::new();
    match tasks.focus.as_ref() {
        Some(focus) => {
            let phase = focus.phase;
            writeln!(body, "{}", focus_header_line(tasks, phase)).expect("write to string");
            body.push('\n');
            writeln!(body, "{}", last_shipped_line(tasks, phase, today)).expect("write to string");
            body.push('\n');
            writeln!(body, "{}", up_next_line(tasks)).expect("write to string");
        }
        None => {
            writeln!(body, "**Focus phase:** not set — add [focus] to tasks.toml")
                .expect("write to string");
        }
    }
    body
}

fn focus_header_line(tasks: &Tasks, focus_phase: u32) -> String {
    let in_phase: Vec<&Task> = tasks
        .task
        .iter()
        .filter(|task| task.phase == focus_phase)
        .collect();
    let total = in_phase.len();
    let done = in_phase.iter().filter(|task| task.status == "done").count();
    let in_progress = in_phase
        .iter()
        .filter(|task| task.status == "in_progress")
        .count();
    let name = tasks
        .phases
        .get(&focus_phase.to_string())
        .map(|phase| phase.name.clone())
        .unwrap_or_else(|| format!("phase {focus_phase}"));
    format!(
        "**Focus phase:** {focus_phase} — {name} ({done} of {total} done · {in_progress} in progress)"
    )
}

fn last_shipped_line(tasks: &Tasks, focus_phase: u32, today: &str) -> String {
    let recent: Vec<&Task> = tasks
        .task
        .iter()
        .filter(|task| task.phase == focus_phase && task.status == "done")
        .filter(|task| {
            task.done_at
                .as_deref()
                .and_then(|done_at| days_since(today, done_at))
                .map(|days| (0..=SHIPPED_WINDOW_DAYS).contains(&days))
                .unwrap_or(false)
        })
        .collect();

    if recent.is_empty() {
        return "**Last shipped:** no recent shipments".to_string();
    }

    let latest = recent
        .iter()
        .filter_map(|task| task.done_at.as_deref())
        .max()
        .unwrap_or("");

    let same_day: Vec<&Task> = recent
        .iter()
        .copied()
        .filter(|task| task.done_at.as_deref() == Some(latest))
        .collect();

    let names = same_day
        .iter()
        .map(|task| format!("Task {} — {}", task.id, task.title))
        .collect::<Vec<_>>()
        .join(", ");

    format!("**Last shipped:** {names} on {latest}")
}

fn up_next_line(tasks: &Tasks) -> String {
    match next_task(tasks, &TaskFilter::default()) {
        Some(task) => {
            let eff = efficiency(task);
            format!(
                "**Up next:** Task {} — {} [D:{}/B:{}/U:{} → Eff:{}] {}",
                task.id,
                task.title,
                task.scores.d,
                task.scores.b,
                task.scores.u,
                format_efficiency(eff),
                tier_glyph(eff)
            )
        }
        None => "**Up next:** none — focus phase complete or all blocked".to_string(),
    }
}

fn render_vision_pass(roadmap: &str, tasks: &Tasks) -> Result<String, RenderError> {
    let Some(begin) = roadmap.find(VISION_BEGIN_MARKER) else {
        return Ok(roadmap.to_string());
    };
    let marker_line_end = roadmap[begin..]
        .find('\n')
        .map(|offset| begin + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());
    let end = roadmap[marker_line_end..]
        .find(VISION_END_MARKER)
        .map(|offset| marker_line_end + offset)
        .ok_or(RenderError::MissingVisionEndMarker { start: begin })?;
    let end_line_end = roadmap[end..]
        .find('\n')
        .map(|offset| end + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());

    let mut rendered = String::with_capacity(roadmap.len());
    rendered.push_str(&roadmap[..marker_line_end]);
    rendered.push_str(&render_vision_body(tasks));
    rendered.push_str(&roadmap[end..end_line_end]);
    rendered.push_str(&roadmap[end_line_end..]);
    Ok(rendered)
}

fn render_vision_body(tasks: &Tasks) -> String {
    match tasks.vision.as_deref() {
        Some(text) => format!("{}\n", text.trim_end_matches('\n')),
        None => "**Vision:** not set — add `vision = \"...\"` to tasks.toml\n".to_string(),
    }
}

fn render_mermaid_pass(roadmap: &str, tasks: &Tasks, today: &str) -> Result<String, RenderError> {
    let Some(begin) = roadmap.find(MERMAID_BEGIN_MARKER) else {
        return Ok(roadmap.to_string());
    };
    let marker_line_end = roadmap[begin..]
        .find('\n')
        .map(|offset| begin + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());
    let end = roadmap[marker_line_end..]
        .find(MERMAID_END_MARKER)
        .map(|offset| marker_line_end + offset)
        .ok_or(RenderError::MissingMermaidEndMarker { start: begin })?;
    let end_line_end = roadmap[end..]
        .find('\n')
        .map(|offset| end + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());

    let mut rendered = String::with_capacity(roadmap.len());
    rendered.push_str(&roadmap[..marker_line_end]);
    rendered.push_str(&render_mermaid_body(tasks, today));
    rendered.push_str(&roadmap[end..end_line_end]);
    rendered.push_str(&roadmap[end_line_end..]);
    Ok(rendered)
}

fn render_mermaid_body(tasks: &Tasks, today: &str) -> String {
    let mut body = String::new();
    body.push_str("```mermaid\n");
    body.push_str("gantt\n");
    writeln!(body, "    title {}", tasks.project).expect("write to string");
    body.push_str("    dateFormat YYYY-MM-DD\n");

    let mut phase_entries: Vec<(&String, &crate::schema::Phase)> = tasks.phases.iter().collect();
    phase_entries.sort_by_key(|(_, phase)| phase.order);

    let mut any_row = false;
    for (key, phase) in phase_entries {
        let Ok(phase_number) = key.parse::<u32>() else {
            continue;
        };
        let rows: Vec<String> = tasks
            .task
            .iter()
            .filter(|task| task.phase == phase_number)
            .filter_map(|task| gantt_row(task, today))
            .collect();
        if rows.is_empty() {
            continue;
        }
        if !any_row {
            any_row = true;
        }
        writeln!(body, "    section Phase {} — {}", phase_number, phase.name)
            .expect("write to string");
        for row in rows {
            writeln!(body, "    {row}").expect("write to string");
        }
    }

    if !any_row {
        body.push_str("    %% no tasks with started_at yet\n");
    }

    body.push_str("```\n");
    body
}

fn render_milestones_pass(roadmap: &str, tasks: &Tasks) -> Result<String, RenderError> {
    let Some(begin) = roadmap.find(MILESTONES_BEGIN_MARKER) else {
        return Ok(roadmap.to_string());
    };
    let marker_line_end = roadmap[begin..]
        .find('\n')
        .map(|offset| begin + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());
    let end = roadmap[marker_line_end..]
        .find(MILESTONES_END_MARKER)
        .map(|offset| marker_line_end + offset)
        .ok_or(RenderError::MissingMilestonesEndMarker { start: begin })?;
    let end_line_end = roadmap[end..]
        .find('\n')
        .map(|offset| end + offset + '\n'.len_utf8())
        .unwrap_or(roadmap.len());

    let mut rendered = String::with_capacity(roadmap.len());
    rendered.push_str(&roadmap[..marker_line_end]);
    rendered.push_str(&render_milestones_body(tasks));
    rendered.push_str(&roadmap[end..end_line_end]);
    rendered.push_str(&roadmap[end_line_end..]);
    Ok(rendered)
}

fn render_milestones_body(tasks: &Tasks) -> String {
    let summaries = list_milestones(tasks, &MilestoneFilter::default());
    if summaries.is_empty() {
        return "(no milestones declared)\n".to_string();
    }

    let mut body = String::new();
    for (index, summary) in summaries.iter().enumerate() {
        if index > 0 {
            body.push('\n');
        }
        render_milestone_block(&mut body, summary);
    }
    body
}

fn render_milestone_block(body: &mut String, summary: &MilestoneSummary<'_>) {
    writeln!(body, "### {} — {}", summary.key, summary.name).expect("write to string");
    body.push('\n');
    writeln!(
        body,
        "- **target_version:** {}",
        summary.target_version.unwrap_or("none")
    )
    .expect("write to string");
    writeln!(
        body,
        "- **status:** {} {}",
        milestone_status_glyph(summary.status),
        summary.status
    )
    .expect("write to string");
    writeln!(
        body,
        "- **hypothesis:** {}",
        summary.description.unwrap_or("not set")
    )
    .expect("write to string");
    writeln!(
        body,
        "- **pinned tasks:** {}/{} done",
        summary.status_counts.done, summary.task_count
    )
    .expect("write to string");
}

fn milestone_status_glyph(status: &str) -> &str {
    match status {
        "active" => "🔄",
        "pending" => "⬜",
        "done" => "✅",
        _ => status,
    }
}

fn gantt_row(task: &Task, today: &str) -> Option<String> {
    let started_at = task.started_at.as_deref()?;
    let title = task.title.replace([':', ',', ';'], "—");
    match task.status.as_str() {
        "done" => {
            let done_at = task.done_at.as_deref()?;
            Some(format!("{title} :done, {started_at}, {done_at}"))
        }
        "in_progress" => Some(format!("{title} :active, {started_at}, {today}")),
        "blocked" => Some(format!("{title} :crit, {started_at}, {today}")),
        _ => None,
    }
}

fn render_tasks_pass(roadmap: &str, tasks: &Tasks, today: &str) -> Result<String, RenderError> {
    let mut rendered = String::with_capacity(roadmap.len());
    let mut cursor = 0;

    while let Some(relative_begin) = roadmap[cursor..].find(BEGIN_MARKER) {
        let begin = cursor + relative_begin;
        rendered.push_str(&roadmap[cursor..begin]);

        let marker_line_end = roadmap[begin..]
            .find('\n')
            .map(|offset| begin + offset + '\n'.len_utf8())
            .ok_or(RenderError::InvalidPhaseMarker { start: begin })?;
        let marker_line = &roadmap[begin..marker_line_end];
        let phase =
            parse_phase(marker_line).ok_or(RenderError::InvalidPhaseMarker { start: begin })?;

        rendered.push_str(marker_line);
        rendered.push_str(&render_phase_table(tasks, phase, today));

        let end = roadmap[marker_line_end..]
            .find(END_MARKER)
            .map(|offset| marker_line_end + offset)
            .ok_or(RenderError::MissingEndMarker { start: begin })?;
        let end_line_end = roadmap[end..]
            .find('\n')
            .map(|offset| end + offset + '\n'.len_utf8())
            .unwrap_or(roadmap.len());

        rendered.push_str(&roadmap[end..end_line_end]);
        cursor = end_line_end;
    }

    rendered.push_str(&roadmap[cursor..]);
    Ok(rendered)
}

fn parse_phase(marker_line: &str) -> Option<u32> {
    let phase_start = marker_line.find(BEGIN_MARKER)? + BEGIN_MARKER.len();
    let phase_end = marker_line[phase_start..].find(" -->")? + phase_start;

    marker_line[phase_start..phase_end].parse().ok()
}

fn render_phase_table(tasks: &Tasks, phase: u32, today: &str) -> String {
    if let Some(phase_entry) = tasks.phases.get(&phase.to_string())
        && phase_entry.status == "done"
    {
        let count = tasks.task.iter().filter(|task| task.phase == phase).count();
        let noun = if count == 1 { "task" } else { "tasks" };
        let slug = phase_slug(&phase_entry.name);
        return format!(
            "> {count} {noun}. See [CHANGELOG.md](CHANGELOG.md#phase-{phase}-{slug}).\n"
        );
    }

    let mut table = String::new();
    table.push_str("| Task | Status | Notes |\n");
    table.push_str("|------|--------|-------|\n");

    for task in tasks.task.iter().filter(|task| task.phase == phase) {
        let eff = efficiency(task);
        let decay = score_decay_suffix(task, today);
        let milestone_segment = task
            .milestone
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(|m| format!("🚀 **{m}** · "))
            .unwrap_or_default();
        let module_segment = task
            .module
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(|m| format!("*{m}* · "))
            .unwrap_or_default();
        let category_segment = category_prefix(task);
        // Surface the block reason inline so an orchestrator scanning the
        // rendered roadmap sees the dead-end cause without opening tasks.toml.
        // Conditional + trailing: non-blocked rows render byte-identically.
        let blocked_segment = if task.status == "blocked" {
            task.blocked_reason
                .as_deref()
                .map(str::trim)
                .filter(|r| !r.is_empty())
                .map(|r| format!(" ⛔ {r}"))
                .unwrap_or_default()
        } else {
            String::new()
        };
        let status_cell = match (task.status.as_str(), task.branch.as_deref()) {
            ("in_progress", Some(branch)) => {
                let trimmed = branch.trim();
                if trimmed.is_empty() {
                    status_symbol(&task.status).to_string()
                } else {
                    format!("{} {}", status_symbol(&task.status), trimmed)
                }
            }
            _ => status_symbol(&task.status).to_string(),
        };
        writeln!(
            table,
            "| Task {}{} | {} | 🎁 **{}** · {}{}{}{} [D:{}/B:{}/U:{} → Eff:{}{}] {}{} |",
            task.id,
            marker_suffix(task),
            status_cell,
            task.bundle,
            milestone_segment,
            module_segment,
            category_segment,
            task.title,
            task.scores.d,
            task.scores.b,
            task.scores.u,
            format_efficiency(eff),
            decay,
            tier_glyph(eff),
            blocked_segment
        )
        .expect("write to string");
    }

    table
}

fn marker_suffix(task: &Task) -> String {
    let markers = task
        .markers
        .iter()
        .filter_map(|marker| match marker.as_str() {
            "parallel" => Some("`[P]`"),
            "cx" => Some("`[CX]`"),
            "csr" => Some("`[CSR]`"),
            _ => None,
        })
        .collect::<Vec<_>>();

    if markers.is_empty() {
        String::new()
    } else {
        format!(" {}", markers.join(" "))
    }
}

fn category_prefix(task: &Task) -> String {
    let glyphs = task
        .markers
        .iter()
        .filter_map(|marker| match marker.as_str() {
            "bug" => Some("🐛"),
            "security" => Some("🔒"),
            "docs" => Some("📝"),
            _ => None,
        })
        .collect::<Vec<_>>();

    if glyphs.is_empty() {
        String::new()
    } else {
        format!("{} ", glyphs.join(" "))
    }
}

/// Kebab-case a phase name for the archive-collapse CHANGELOG anchor.
/// Lowercases, splits on any non-alphanumeric run, drops empty segments,
/// joins with `-`. Used only by `render_phase_table` when collapsing a
/// `status = "done"` phase.
fn phase_slug(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn status_symbol(status: &str) -> &str {
    match status {
        "pending" => "⬜",
        "in_progress" => "🔄",
        "blocked" => "🔶",
        "done" => "✅",
        "superseded" => "⛔",
        _ => status,
    }
}
