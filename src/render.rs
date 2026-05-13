use std::fmt::Write;

use thiserror::Error;

use crate::next::next_task;
use crate::schema::{Task, Tasks};
use crate::scoring::{days_since, efficiency, format_efficiency, score_decay_suffix};

const BEGIN_MARKER: &str = "<!-- TASKS:BEGIN phase=";
const END_MARKER: &str = "<!-- TASKS:END -->";

const FOCUS_BEGIN_MARKER: &str = "<!-- FOCUS:BEGIN -->";
const FOCUS_END_MARKER: &str = "<!-- FOCUS:END -->";

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
}

/// Render with an explicit `today` date (`YYYY-MM-DD`). Pass `""` to disable decay suffixes.
pub fn render_roadmap_str_with_today(
    roadmap: &str,
    tasks: &Tasks,
    today: &str,
) -> Result<String, RenderError> {
    let after_focus = render_focus_pass(roadmap, tasks, today)?;
    render_tasks_pass(&after_focus, tasks, today)
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
    match next_task(tasks, None) {
        Some(task) => {
            let eff = efficiency(task);
            format!(
                "**Up next:** Task {} — {} [D:{}/B:{}/U:{} → Eff:{}]",
                task.id,
                task.title,
                task.scores.d,
                task.scores.b,
                task.scores.u,
                format_efficiency(eff)
            )
        }
        None => "**Up next:** none — focus phase complete or all blocked".to_string(),
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
    let mut table = String::new();
    table.push_str("| Task | Status | Notes |\n");
    table.push_str("|------|--------|-------|\n");

    for task in tasks.task.iter().filter(|task| task.phase == phase) {
        let eff = efficiency(task);
        let decay = score_decay_suffix(task, today);
        let module_segment = task
            .module
            .as_deref()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .map(|m| format!("*{m}* · "))
            .unwrap_or_default();
        writeln!(
            table,
            "| Task {}{} | {} | 🎁 **{}** · {}{} [D:{}/B:{}/U:{} → Eff:{}{}] {} |",
            task.id,
            marker_suffix(task),
            status_symbol(&task.status),
            task.bundle,
            module_segment,
            task.title,
            task.scores.d,
            task.scores.b,
            task.scores.u,
            format_efficiency(eff),
            decay,
            priority_symbol(eff)
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

fn priority_symbol(efficiency: f64) -> &'static str {
    if efficiency >= 2.0 {
        "🎯"
    } else if efficiency >= 1.5 {
        "🚀"
    } else if efficiency >= 1.0 {
        "📋"
    } else {
        "⚠️"
    }
}
