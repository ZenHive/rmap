use crate::commands::output::{render_outputs, write_outputs};
use crate::commands::prompt::prompt_for_implemented;
use anyhow::{Context, Result, bail};
use rmap::mutate::{
    CrossRepoSpec, MarkerOp, TransitionFields, add_dependency_str, update_assignee_str,
    update_markers_str, update_milestone_str, update_status_many_str,
};
use rmap::paths::ResolvedPaths;
use rmap::validate::validate_tasks_str;

pub(crate) fn update_markers(
    paths: ResolvedPaths,
    task_id: &str,
    op_tokens: &[String],
) -> Result<()> {
    let ops: Vec<MarkerOp<'_>> = op_tokens
        .iter()
        .map(|token| MarkerOp::parse(token))
        .collect::<Result<_, _>>()?;

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = update_markers_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        &ops,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

pub(crate) fn update_milestone(paths: ResolvedPaths, task_id: &str, name: &str) -> Result<()> {
    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let milestone: Option<&str> = if name == "none" { None } else { Some(name) };

    let updated = update_milestone_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        milestone,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

pub(crate) fn update_assignee(
    paths: ResolvedPaths,
    task_id: &str,
    assignee_arg: &str,
    model: Option<&str>,
) -> Result<()> {
    if (assignee_arg == "none" || assignee_arg == "human") && model.is_some() {
        bail!("cannot pass --model when clearing assignee (use `none` or `human` without --model)");
    }

    let (assignee, model_to_set) = if assignee_arg == "none" || assignee_arg == "human" {
        (None, None)
    } else {
        (Some(assignee_arg), model)
    };

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = update_assignee_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        assignee,
        model_to_set,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

pub(crate) fn add_dependency(
    paths: ResolvedPaths,
    task_id: &str,
    on_token: Option<&str>,
    other: Option<&str>,
    cross_repo_spec: Option<&str>,
) -> Result<()> {
    let in_repo: Option<&str> = match (on_token, other) {
        (Some("on"), Some(other_id)) => Some(other_id),
        (Some("on"), None) => {
            bail!("missing target task id after `on` (use `rmap depend <id> on <other-id>`)")
        }
        (Some(kw), _) => bail!("unexpected positional {:?} — expected `on <task_id>`", kw),
        // Clap fills optional positionals in declaration order, so `on` is always
        // None before `other`; the wildcard is defensive against future clap changes.
        (None, _) => None,
    };

    let cross_repo = cross_repo_spec.map(CrossRepoSpec::parse).transpose()?;

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = add_dependency_str(
        paths.tasks_path.display().to_string(),
        &input,
        task_id,
        in_repo,
        cross_repo.as_ref(),
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

pub(crate) fn update_status(
    paths: ResolvedPaths,
    task_id: &str,
    new_status: &str,
    fields: TransitionFields<'_>,
) -> Result<()> {
    let ids: Vec<&str> = task_id
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if ids.is_empty() {
        anyhow::bail!("no task ids provided (got {:?})", task_id);
    }

    if fields.implemented.is_some() && new_status != "done" {
        eprintln!(
            "warning: --implemented ignored for status `{new_status}` (only applies to `done`)"
        );
    }

    if fields.delivered_by.is_some() && new_status != "done" {
        eprintln!(
            "warning: --delivered-by ignored for status `{new_status}` (only applies to `done`)"
        );
    }

    if fields.verified.is_some() && new_status != "done" {
        eprintln!("warning: --verified ignored for status `{new_status}` (only applies to `done`)");
    }

    if (fields.verified_by.is_some() || fields.verification_ref.is_some()) && new_status != "done" {
        eprintln!(
            "warning: --verified-by/--verification-ref ignored for status `{new_status}` (only apply to `done`)"
        );
    }

    if fields.shipped_in.is_some() && new_status != "done" {
        eprintln!(
            "warning: --shipped-in ignored for status `{new_status}` (only applies to `done`)"
        );
    }

    if fields.blocked_reason.is_some() && new_status != "blocked" {
        eprintln!(
            "warning: --reason ignored for status `{new_status}` (only applies to `blocked`)"
        );
    }

    if (fields.attempt_report.is_some() || fields.attempt_by.is_some()) && new_status != "pending" {
        eprintln!(
            "warning: --report/--attempt-by ignored for status `{new_status}` (only applies to `pending`)"
        );
    } else if fields.attempt_report.is_none() && fields.attempt_by.is_some() {
        eprintln!("warning: --attempt-by ignored without --report");
    }

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let prompted: Option<String> = if new_status == "done" && fields.implemented.is_none() {
        prompt_for_implemented(&input, &ids)?
    } else {
        None
    };
    let effective_implemented = fields.implemented.or(prompted.as_deref());

    let updated = update_status_many_str(
        paths.tasks_path.display().to_string(),
        &input,
        &ids,
        new_status,
        TransitionFields {
            implemented: effective_implemented,
            ..fields
        },
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}
