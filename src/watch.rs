//! Pure helpers for `rmap watch` — the FS-watch render loop.
//!
//! The `notify` wiring and the blocking event loop live in `main.rs`
//! (`watch_command`); this module holds only the pure, unit-testable pieces:
//! the idempotent write and the event-path filter.

use std::path::Path;

/// Write `content` to `path` only when it differs from what is already there.
///
/// Returns `Ok(true)` when a write happened, `Ok(false)` when the on-disk bytes
/// already matched. A missing file — or any read error — counts as changed, so
/// the write is attempted.
///
/// This is what makes the watch loop idempotent: a no-change event re-renders
/// to identical bytes and skips the write entirely. It is also a second line of
/// defense against a render → write → event feedback loop on `data.json`.
pub fn write_if_changed(path: &Path, content: &str) -> std::io::Result<bool> {
    match std::fs::read(path) {
        Ok(existing) if existing == content.as_bytes() => Ok(false),
        _ => {
            std::fs::write(path, content)?;
            Ok(true)
        }
    }
}

/// True when a filesystem event touches the watched `tasks.toml`.
///
/// `rmap watch` registers the watcher on the `roadmap/` directory rather than
/// on `tasks.toml` directly, so it survives editor atomic-save renames — but
/// that also means it sees writes to the sibling `roadmap/data.json`. Filtering
/// events down to the `tasks.toml` file name here is the guard that stops our
/// own `data.json` writes from re-triggering the render loop. Both an exact
/// path match and a bare file-name match count; the latter covers atomic-rename
/// events whose reported path may not equal `tasks_path`.
pub fn is_tasks_toml_event(tasks_path: &Path, event: &notify::Event) -> bool {
    let target_name = tasks_path.file_name();
    event
        .paths
        .iter()
        .any(|p| p == tasks_path || (target_name.is_some() && p.file_name() == target_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn event_for(paths: &[&str]) -> notify::Event {
        notify::Event {
            kind: notify::EventKind::Any,
            paths: paths.iter().map(PathBuf::from).collect(),
            attrs: Default::default(),
        }
    }

    #[test]
    fn write_if_changed_writes_when_file_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.md");
        let wrote = write_if_changed(&path, "hello").expect("write");
        assert!(wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_if_changed_writes_when_content_differs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.md");
        std::fs::write(&path, "old").unwrap();
        let wrote = write_if_changed(&path, "new").expect("write");
        assert!(wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn write_if_changed_skips_when_content_equal() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("out.md");
        std::fs::write(&path, "same").unwrap();
        // The `false` return value is the contract — an mtime assertion would
        // be racy. The content is left untouched either way.
        let wrote = write_if_changed(&path, "same").expect("write");
        assert!(!wrote);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "same");
    }

    #[test]
    fn is_tasks_toml_event_matches_exact_path() {
        let tasks_path = PathBuf::from("/project/roadmap/tasks.toml");
        assert!(is_tasks_toml_event(
            &tasks_path,
            &event_for(&["/project/roadmap/tasks.toml"])
        ));
    }

    #[test]
    fn is_tasks_toml_event_matches_by_file_name_after_rename() {
        // An editor atomic-save can report a path that is not byte-equal to
        // `tasks_path`; the file-name fallback still matches it.
        let tasks_path = PathBuf::from("/project/roadmap/tasks.toml");
        assert!(is_tasks_toml_event(
            &tasks_path,
            &event_for(&["/tmp/.tasks.toml.tmp", "tasks.toml"])
        ));
    }

    #[test]
    fn is_tasks_toml_event_ignores_data_json() {
        // The infinite-loop guard: our own `data.json` write must not match.
        let tasks_path = PathBuf::from("/project/roadmap/tasks.toml");
        assert!(!is_tasks_toml_event(
            &tasks_path,
            &event_for(&["/project/roadmap/data.json"])
        ));
    }

    #[test]
    fn is_tasks_toml_event_ignores_roadmap_md() {
        let tasks_path = PathBuf::from("/project/roadmap/tasks.toml");
        assert!(!is_tasks_toml_event(
            &tasks_path,
            &event_for(&["/project/ROADMAP.md"])
        ));
    }
}
