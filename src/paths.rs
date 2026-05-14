use std::env;
use std::path::{Path, PathBuf};

use thiserror::Error;

const TASKS_RELATIVE_PATH: &str = "roadmap/tasks.toml";
const ROADMAP_FILE_NAME: &str = "ROADMAP.md";
const DATA_RELATIVE_PATH: &str = "roadmap/data.json";
const HTML_RELATIVE_PATH: &str = "roadmap/dist/index.html";
const ROADMAP_DIR_NAME: &str = "roadmap";
const TASKS_FILE_NAME: &str = "tasks.toml";

#[derive(Debug, PartialEq)]
pub struct ResolvedPaths {
    pub project_root: PathBuf,
    pub tasks_path: PathBuf,
    pub roadmap_path: PathBuf,
    pub data_path: PathBuf,
    /// `roadmap/dist/index.html` — the gitignored static HTML view written by
    /// `rmap render --html`. Always derived from `project_root`; not separately
    /// overridable (no `--html-path` flag).
    pub html_path: PathBuf,
}

#[derive(Debug, Error)]
pub enum PathError {
    #[error("could not read current directory: {0}")]
    CurrentDir(String),
    #[error("could not find roadmap/tasks.toml from {start}")]
    MissingTasks { start: String },
}

pub fn resolve_paths(
    tasks_path: Option<PathBuf>,
    roadmap_path: Option<PathBuf>,
    data_path: Option<PathBuf>,
) -> Result<ResolvedPaths, PathError> {
    let start = env::current_dir().map_err(|err| PathError::CurrentDir(err.to_string()))?;

    resolve_paths_from(start, tasks_path, roadmap_path, data_path)
}

pub fn resolve_paths_from(
    start: impl AsRef<Path>,
    tasks_path: Option<PathBuf>,
    roadmap_path: Option<PathBuf>,
    data_path: Option<PathBuf>,
) -> Result<ResolvedPaths, PathError> {
    let tasks_path = match tasks_path {
        Some(path) => path,
        None => find_tasks_path(start.as_ref())?,
    };
    let project_root = project_root_for_tasks_path(&tasks_path);
    let roadmap_path = roadmap_path.unwrap_or_else(|| project_root.join(ROADMAP_FILE_NAME));
    let data_path = data_path.unwrap_or_else(|| project_root.join(DATA_RELATIVE_PATH));
    let html_path = project_root.join(HTML_RELATIVE_PATH);

    Ok(ResolvedPaths {
        project_root,
        tasks_path,
        roadmap_path,
        data_path,
        html_path,
    })
}

fn find_tasks_path(start: &Path) -> Result<PathBuf, PathError> {
    for ancestor in start.ancestors() {
        let candidate = ancestor.join(TASKS_RELATIVE_PATH);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(PathError::MissingTasks {
        start: start.display().to_string(),
    })
}

fn project_root_for_tasks_path(tasks_path: &Path) -> PathBuf {
    let Some(parent) = tasks_path.parent() else {
        return PathBuf::from(".");
    };

    if tasks_path.file_name().and_then(|name| name.to_str()) == Some(TASKS_FILE_NAME)
        && parent.file_name().and_then(|name| name.to_str()) == Some(ROADMAP_DIR_NAME)
    {
        return parent
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
    }

    parent.to_path_buf()
}
