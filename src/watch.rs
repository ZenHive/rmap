//! Pure helpers for `rmap watch` — the FS-watch render loop.
//!
//! The `notify` wiring and the blocking event loop live in `main.rs`
//! (`watch_command`); this module holds only the pure, unit-testable pieces:
//! the idempotent write, the event-path filter, and the `--json` event-line
//! serializers.

use std::path::Path;

use serde::Serialize;

/// Schema version for the `rmap watch --json` event stream. Bumped only on a
/// breaking change to the event shape (rename/remove a field); adding a field
/// is additive and does not bump.
const WATCH_SCHEMA_VERSION: u32 = 1;

/// One `rmap watch --json` event line. Serialized compact (one line) — the
/// stream contract is one JSON object per render event, so this is the single
/// place in rmap that uses `serde_json::to_string` rather than `to_string_pretty`.
#[derive(Serialize)]
struct WatchEventJson<'a> {
    schema_version: u32,
    event: &'a str,
    /// Basenames of the outputs written this render. Present on `rendered`
    /// events; empty (and skipped) on `error` events.
    #[serde(skip_serializing_if = "<[_]>::is_empty")]
    outputs: &'a [&'a str],
    /// Rendered error text. Present on `error` events only.
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'a str>,
}

/// Serialize a `rendered` event: one render that wrote `outputs` (basenames).
///
/// Returns a compact, single-line JSON string with no trailing newline — the
/// caller adds the newline (`println!`).
pub fn render_event_line(outputs: &[&str]) -> String {
    let event = WatchEventJson {
        schema_version: WATCH_SCHEMA_VERSION,
        event: "rendered",
        outputs,
        message: None,
    };
    // A struct of `&str` / `u32` / `&[&str]` cannot fail to serialize.
    serde_json::to_string(&event).expect("WatchEventJson serializes")
}

/// Serialize an `error` event: one render that failed validation or rendering.
///
/// Returns a compact, single-line JSON string with no trailing newline.
pub fn error_event_line(message: &str) -> String {
    let event = WatchEventJson {
        schema_version: WATCH_SCHEMA_VERSION,
        event: "error",
        outputs: &[],
        message: Some(message),
    };
    serde_json::to_string(&event).expect("WatchEventJson serializes")
}

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

    #[test]
    fn render_event_line_lists_both_outputs() {
        let line = render_event_line(&["ROADMAP.md", "data.json"]);
        assert_eq!(
            line,
            r#"{"schema_version":1,"event":"rendered","outputs":["ROADMAP.md","data.json"]}"#
        );
        // Compact — exactly one line, no trailing newline.
        assert!(!line.contains('\n'));
    }

    #[test]
    fn render_event_line_lists_only_changed_output() {
        // Only ROADMAP.md changed this render — data.json must be absent.
        let line = render_event_line(&["ROADMAP.md"]);
        assert_eq!(
            line,
            r#"{"schema_version":1,"event":"rendered","outputs":["ROADMAP.md"]}"#
        );
        assert!(!line.contains("data.json"));
    }

    #[test]
    fn error_event_line_carries_message_and_omits_outputs() {
        let line = error_event_line("tasks.toml:3 invalid status");
        assert_eq!(
            line,
            r#"{"schema_version":1,"event":"error","message":"tasks.toml:3 invalid status"}"#
        );
        // `outputs` is skipped entirely on error events.
        assert!(!line.contains("outputs"));
    }
}
