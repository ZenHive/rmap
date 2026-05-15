use rmap::import::format_import_prompt;

fn prompt() -> String {
    format_import_prompt("my_project")
}

#[test]
fn emits_context_section() {
    assert!(prompt().contains("## Context"), "{}", prompt());
}

#[test]
fn emits_task_section() {
    assert!(prompt().contains("## Task"), "{}", prompt());
}

#[test]
fn emits_schema_section() {
    let p = prompt();
    assert!(p.contains("## Schema"), "{p}");
    assert!(p.contains("schema_version"), "{p}");
}

#[test]
fn emits_field_mapping_guide() {
    assert!(prompt().contains("## Field-mapping guide"), "{}", prompt());
}

#[test]
fn emits_marker_pair_contract() {
    let p = prompt();
    assert!(p.contains("TASKS:BEGIN"), "{p}");
    assert!(p.contains("TASKS:END"), "{p}");
}

#[test]
fn emits_verification_loop() {
    let p = prompt();
    assert!(p.contains("rmap validate"), "{p}");
    assert!(p.contains("rmap render"), "{p}");
}

#[test]
fn emits_instructions() {
    assert!(prompt().contains("## Instructions"), "{}", prompt());
}

#[test]
fn includes_project_name() {
    assert!(prompt().contains("my_project"), "{}", prompt());
}

#[test]
fn placeholder_project_when_no_tasks_toml() {
    let p = format_import_prompt("<your-project>");
    assert!(p.contains("- Project: <your-project>"), "{p}");
}
