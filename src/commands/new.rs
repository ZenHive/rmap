use crate::commands::output::{render_outputs, write_outputs};
use crate::commands::prompt::prompt_task_fields;
use anyhow::{Context, Result, bail};
use rmap::creation_input::{CreationBuffers, STDIN_TASK_FIELDS, StdinTask};
use rmap::mutate::add_task_str;
use rmap::paths::ResolvedPaths;
use rmap::schema::TaskId;
use rmap::today_iso;
use rmap::validate::validate_tasks_str;

/// Top-level for `rmap new` and `rmap new --from-stdin`. Shares the load /
/// mutate / re-validate / re-render / write tail with the other mutators.

pub(crate) fn create_task(paths: ResolvedPaths, from_stdin: bool) -> Result<()> {
    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let path_label = paths.tasks_path.display().to_string();

    let new_tasks: Vec<StdinTask> = if from_stdin {
        let mut stdin_buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin_buf)
            .context("read stdin")?;
        // Collect EVERY field-level defect in one pass (unknown / missing /
        // wrong-type / unresolved phase-bundle) before serde's fail-fast deser,
        // so an agent fixes them in one edit instead of N round-trips.
        let existing =
            validate_tasks_str(path_label.clone(), &input).context("load existing tasks")?;
        validate_stdin_payload(&path_label, &stdin_buf, &existing)?;
        let payload: StdinPayload = toml::from_str(&stdin_buf).map_err(|err| {
            anyhow::anyhow!(
                "parse stdin TOML: {err}\nexpected one-or-more `[[task]]` blocks; see SKILLS.md `rmap new --from-stdin`"
            )
        })?;
        if payload.task.is_empty() {
            bail!("stdin payload contained no `[[task]]` blocks");
        }
        payload.task
    } else {
        if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            bail!(
                "interactive `rmap new` requires a TTY — pipe a TOML fragment via `rmap new --from-stdin` for non-interactive contexts"
            );
        }
        let existing =
            validate_tasks_str(path_label.clone(), &input).context("load existing tasks")?;
        vec![prompt_task_fields(&existing)?]
    };

    let today = today_iso();
    let mut current = input.clone();
    let mut allocated_ids: Vec<u32> = Vec::with_capacity(new_tasks.len());

    for task in &new_tasks {
        if let Some(status) = &task.status
            && status != "pending"
        {
            bail!(
                "stdin task status {status:?}: `rmap new` creates pending tasks only — \
                 omit `status` (or set \"pending\"), then run `rmap status <id> {status}` after creation"
            );
        }
        let markers: Vec<&str> = task.markers.iter().map(String::as_str).collect();
        let mut depends_on: Vec<u32> = Vec::with_capacity(task.depends_on.len());
        for dep in &task.depends_on {
            match dep {
                TaskId::Number(n) => depends_on.push(*n),
                TaskId::Text(text) => bail!(
                    "stdin depends_on {text:?}: text ids are not supported by `rmap new --from-stdin` — use a numeric id"
                ),
            }
        }
        let acceptance_criteria: Vec<&str> = task
            .acceptance_criteria
            .iter()
            .map(String::as_str)
            .collect();
        let out_of_scope: Vec<&str> = task.out_of_scope.iter().map(String::as_str).collect();

        let explicit_id = match &task.id {
            Some(TaskId::Number(n)) => Some(*n),
            Some(TaskId::Text(text)) => bail!(
                "stdin task id {text:?}: text ids are not supported by `rmap new --from-stdin` — omit `id` to auto-allocate or use a numeric id"
            ),
            None => None,
        };

        let buffers = CreationBuffers {
            id: explicit_id,
            markers,
            depends_on,
            acceptance_criteria,
            out_of_scope,
        };
        let fields = task.new_fields(&buffers, &today);

        let (updated, allocated) = add_task_str(path_label.clone(), &current, &fields)?;
        current = updated;
        allocated_ids.push(allocated);
    }

    let tasks = validate_tasks_str(path_label.clone(), &current)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, &current)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;

    let ids_csv: Vec<String> = allocated_ids.iter().map(u32::to_string).collect();
    println!("created task {}", ids_csv.join(", "));

    Ok(())
}

/// Required `[[task]]` fields (no serde `default`, not `Option`). `status` is
/// excluded — it is a tolerated no-op, not required.
const STDIN_REQUIRED_FIELDS: &[&str] = &["phase", "bundle", "title", "scores"];

/// Pre-validate a `--from-stdin` payload, collecting EVERY field-level defect in
/// one pass rather than serde's fail-fast first-error. Reports unknown fields,
/// missing required fields, wrong-typed required fields, and unresolved
/// phase/bundle references (with the valid values listed inline — the zero-guess
/// discoverability path). Returns `Ok(())` when the payload is field-clean, so
/// the caller's serde deser then succeeds. A genuine TOML syntax error is
/// returned as-is, since field-walking needs a parse tree.
fn validate_stdin_payload(label: &str, stdin: &str, existing: &rmap::schema::Tasks) -> Result<()> {
    let value: toml::Value = toml::from_str(stdin).map_err(|err| {
        anyhow::anyhow!(
            "parse stdin TOML: {err}\nexpected one-or-more `[[task]]` blocks; see SKILLS.md `rmap new --from-stdin`"
        )
    })?;

    let mut defects: Vec<String> = Vec::new();

    let Some(table) = value.as_table() else {
        bail!("stdin payload must be a TOML table containing `[[task]]` blocks");
    };

    // Top-level: only `task` is allowed. A mis-named array (`[[tasks]]`) is
    // still walked for per-task field defects so the typo and the field errors
    // surface in one pass — not one round-trip for the typo, then more for the
    // fields it was hiding.
    for (key, val) in table {
        if key != "task" {
            defects.push(format!(
                "top-level: unknown key `{key}` — tasks are `[[task]]` blocks (not `[[{key}]]`)"
            ));
            if let toml::Value::Array(tasks) = val {
                for (i, task) in tasks.iter().enumerate() {
                    validate_stdin_task(i, task, existing, &mut defects);
                }
            }
        }
    }

    match table.get("task") {
        None if defects.is_empty() => defects.push(
            "no `[[task]]` blocks found — each task is a `[[task]]` block; see SKILLS.md"
                .to_string(),
        ),
        None => {}
        Some(toml::Value::Array(tasks)) => {
            if tasks.is_empty() {
                defects.push("`[[task]]` array is empty — declare at least one task".to_string());
            }
            for (i, task) in tasks.iter().enumerate() {
                validate_stdin_task(i, task, existing, &mut defects);
            }
        }
        Some(_) => defects.push("`task` must be an array of `[[task]]` blocks".to_string()),
    }

    if defects.is_empty() {
        return Ok(());
    }

    let body = defects
        .iter()
        .map(|d| format!("  - {d}"))
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "{label}: `rmap new --from-stdin` payload has {} field error(s):\n{body}\nfix all of the above in one edit; see SKILLS.md `rmap new --from-stdin`",
        defects.len()
    );
}

/// Collect field-level defects for a single `[[task]]` block into `defects`.
fn validate_stdin_task(
    index: usize,
    task: &toml::Value,
    existing: &rmap::schema::Tasks,
    defects: &mut Vec<String>,
) {
    let prefix = format!("task[{index}]");
    let Some(table) = task.as_table() else {
        defects.push(format!("{prefix}: must be a `[[task]]` table"));
        return;
    };

    for key in table.keys() {
        if !STDIN_TASK_FIELDS.contains(&key.as_str()) {
            defects.push(format!("{prefix}: unknown field `{key}`"));
        }
    }

    for req in STDIN_REQUIRED_FIELDS {
        if !table.contains_key(*req) {
            defects.push(format!("{prefix}: missing required field `{req}`"));
        }
    }

    if let Some(phase) = table.get("phase") {
        match phase.as_integer() {
            None => defects.push(format!(
                "{prefix}: `phase` must be an integer — {}",
                valid_phases_hint(existing)
            )),
            Some(n) if !existing.phases.contains_key(&n.to_string()) => defects.push(format!(
                "{prefix}: unknown phase {n} — {}",
                valid_phases_hint(existing)
            )),
            Some(_) => {}
        }
    }

    if let Some(bundle) = table.get("bundle") {
        match bundle.as_str() {
            None => defects.push(format!("{prefix}: `bundle` must be a string")),
            Some(b) if !existing.bundles.contains_key(b) => defects.push(format!(
                "{prefix}: unknown bundle \"{b}\" — {}",
                valid_bundles_hint(existing)
            )),
            Some(_) => {}
        }
    }

    if let Some(title) = table.get("title")
        && title.as_str().is_none()
    {
        defects.push(format!("{prefix}: `title` must be a string"));
    }

    if let Some(scores) = table.get("scores") {
        match scores.as_table() {
            None => defects.push(format!(
                "{prefix}: `scores` must be an inline table `{{ d = .., b = .., u = .. }}`"
            )),
            Some(s) => {
                for k in ["d", "b", "u"] {
                    match s.get(k) {
                        None => defects.push(format!("{prefix}: `scores` missing `{k}`")),
                        Some(v) if v.as_integer().is_none() => {
                            defects.push(format!("{prefix}: `scores.{k}` must be an integer"));
                        }
                        Some(_) => {}
                    }
                }
                for k in s.keys() {
                    if !matches!(k.as_str(), "d" | "b" | "u") {
                        defects.push(format!("{prefix}: `scores` has unknown key `{k}`"));
                    }
                }
            }
        }
    }
}

/// `valid phases: 1, 2, 14` — sorted by phase order, for inline error hints.
fn valid_phases_hint(existing: &rmap::schema::Tasks) -> String {
    if existing.phases.is_empty() {
        return "no phases declared in tasks.toml".to_string();
    }
    let mut phases: Vec<(&String, u32)> =
        existing.phases.iter().map(|(k, p)| (k, p.order)).collect();
    phases.sort_by_key(|(_, order)| *order);
    let list = phases
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    format!("valid phases: {list}")
}

/// `valid bundles: auth, import_command` — sorted by name, for inline error hints.
fn valid_bundles_hint(existing: &rmap::schema::Tasks) -> String {
    if existing.bundles.is_empty() {
        return "no bundles declared in tasks.toml".to_string();
    }
    let mut names: Vec<&str> = existing.bundles.keys().map(String::as_str).collect();
    names.sort_unstable();
    format!("valid bundles: {}", names.join(", "))
}

/// Top-level deserialization shape for `rmap new --from-stdin`. Mirrors the
/// `schema::Tasks` envelope's `[[task]]` array but with `id` optional (so the
/// caller can omit it and let `add_task_str` auto-allocate). `deny_unknown_fields`
/// rejects payloads that would not round-trip cleanly through the rest of the
/// pipeline.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StdinPayload {
    #[serde(default)]
    task: Vec<StdinTask>,
}
