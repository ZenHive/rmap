use std::fmt::Write;

use thiserror::Error;

use crate::schema::{Task, Tasks};
use crate::scoring::{efficiency, format_efficiency, score_decay_suffix};

const BEGIN_MARKER: &str = "<!-- TASKS:BEGIN phase=";
const END_MARKER: &str = "<!-- TASKS:END -->";

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("missing <!-- TASKS:END --> for marker starting at byte {start}")]
    MissingEndMarker { start: usize },
    #[error("invalid phase marker at byte {start}")]
    InvalidPhaseMarker { start: usize },
}

/// Render with an explicit `today` date (`YYYY-MM-DD`). Pass `""` to disable decay suffixes.
pub fn render_roadmap_str_with_today(
    roadmap: &str,
    tasks: &Tasks,
    today: &str,
) -> Result<String, RenderError> {
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

/// Render using `today_iso()` (reads `RMAP_TODAY` env var or the system clock).
pub fn render_roadmap_str(roadmap: &str, tasks: &Tasks) -> Result<String, RenderError> {
    render_roadmap_str_with_today(roadmap, tasks, &crate::today_iso())
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
        writeln!(
            table,
            "| Task {}{} | {} | 🎁 **{}** · {} [D:{}/B:{}/U:{} → Eff:{}{}] {} |",
            task.id,
            marker_suffix(task),
            status_symbol(&task.status),
            task.bundle,
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
