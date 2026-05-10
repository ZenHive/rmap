use std::fmt::Write;
use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::schema::{Task, Tasks};

const BEGIN_MARKER: &str = "<!-- TASKS:BEGIN phase=";
const END_MARKER: &str = "<!-- TASKS:END -->";

#[derive(Debug, Error)]
pub enum RenderError {
    #[error("{path}: {message}")]
    Io { path: String, message: String },
    #[error("missing <!-- TASKS:END --> for marker starting at byte {start}")]
    MissingEndMarker { start: usize },
    #[error("invalid phase marker at byte {start}")]
    InvalidPhaseMarker { start: usize },
}

pub fn render_roadmap_file(roadmap_path: &Path, tasks: &Tasks) -> Result<(), RenderError> {
    let roadmap = fs::read_to_string(roadmap_path).map_err(|err| RenderError::Io {
        path: roadmap_path.display().to_string(),
        message: err.to_string(),
    })?;
    let rendered = render_roadmap_str(&roadmap, tasks)?;

    fs::write(roadmap_path, rendered).map_err(|err| RenderError::Io {
        path: roadmap_path.display().to_string(),
        message: err.to_string(),
    })
}

pub fn render_roadmap_str(roadmap: &str, tasks: &Tasks) -> Result<String, RenderError> {
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
        rendered.push_str(&render_phase_table(tasks, phase));

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

fn render_phase_table(tasks: &Tasks, phase: u32) -> String {
    let mut table = String::new();
    table.push_str("| Task | Status | Eff | Markers | Title |\n");
    table.push_str("|------|--------|-----|---------|-------|\n");

    for task in tasks.task.iter().filter(|task| task.phase == phase) {
        writeln!(
            table,
            "| {} | {} | {:.2} | {} | {} |",
            task.id,
            task.status,
            efficiency(task),
            task.markers.join(", "),
            task.title
        )
        .expect("write to string");
    }

    table
}

fn efficiency(task: &Task) -> f64 {
    f64::from(task.scores.b + task.scores.u) / (2.0 * f64::from(task.scores.d))
}
