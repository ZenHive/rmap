use std::fs;

use rmap::paths::resolve_paths_from;

#[test]
fn resolves_default_paths_from_project_root() {
    let root = temp_dir();
    fs::create_dir_all(root.join("roadmap")).expect("create roadmap dir");
    fs::write(root.join("roadmap/tasks.toml"), "").expect("write tasks file");

    let paths = resolve_paths_from(&root, None, None, None).expect("resolve paths");

    assert_eq!(paths.project_root, root);
    assert_eq!(
        paths.tasks_path,
        paths.project_root.join("roadmap/tasks.toml")
    );
    assert_eq!(paths.roadmap_path, paths.project_root.join("ROADMAP.md"));
    assert_eq!(
        paths.data_path,
        paths.project_root.join("roadmap/data.json")
    );
}

#[test]
fn resolves_default_paths_from_nested_directory() {
    let root = temp_dir();
    let nested = root.join("apps/worker/src");
    fs::create_dir_all(root.join("roadmap")).expect("create roadmap dir");
    fs::create_dir_all(&nested).expect("create nested dir");
    fs::write(root.join("roadmap/tasks.toml"), "").expect("write tasks file");

    let paths = resolve_paths_from(&nested, None, None, None).expect("resolve paths");

    assert_eq!(paths.project_root, root);
}

#[test]
fn explicit_tasks_path_derives_project_root_for_conventional_layout() {
    let root = temp_dir();
    let tasks_path = root.join("roadmap/tasks.toml");
    fs::create_dir_all(root.join("roadmap")).expect("create roadmap dir");
    fs::write(&tasks_path, "").expect("write tasks file");

    let paths =
        resolve_paths_from(root.join("elsewhere"), Some(tasks_path), None, None).expect("paths");

    assert_eq!(paths.project_root, root);
    assert_eq!(paths.roadmap_path, paths.project_root.join("ROADMAP.md"));
    assert_eq!(
        paths.data_path,
        paths.project_root.join("roadmap/data.json")
    );
}

#[test]
fn missing_default_tasks_file_returns_clear_error() {
    let root = temp_dir();

    let err = resolve_paths_from(&root, None, None, None).expect_err("missing tasks file");

    assert!(
        err.to_string()
            .contains("could not find roadmap/tasks.toml"),
        "{err}"
    );
}

fn temp_dir() -> std::path::PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rmap-paths-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after epoch")
            .as_nanos()
    ));

    fs::create_dir_all(&dir).expect("create temp test dir");
    dir
}
