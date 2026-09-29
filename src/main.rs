mod cli;
mod commands;

use crate::commands::mutate::{
    add_dependency, update_assignee, update_markers, update_milestone, update_status,
};
use crate::commands::new::create_task;
use crate::commands::output::{
    format_next_bundle_human, git_show_tasks, repo_relative_tasks_path, roadmap_is_current,
};
use crate::commands::render::{render, render_portfolio_cmd, watch_command};
use anyhow::{Context, Result, bail};
use clap::Parser;
use cli::{Cli, Commands, ExportCommands};
use rmap::bundles::{BundleFilter, bundles_json, format_bundles_human, list_bundles};
use rmap::critical_path::{CriticalPathFilter, critical_path, format_critical_path_human};
use rmap::delegate::{format_delegate_prompt, resolve_target};
use rmap::diff::{diff_toml, format_diff};
use rmap::doctor::{DoctorReport, DoctorThresholds};
use rmap::export::{
    export_bundle_pick_json_str, export_filtered_json_str, export_json_str, export_task_json_str,
    export_tasks_array_json_str, project_fields_json_str,
};
use rmap::graph_export::{build_dot, build_waves, format_dot, format_waves, format_waves_json};
use rmap::import::format_import_prompt;
use rmap::milestones::{
    MilestoneFilter, format_milestones_human, list_milestones, milestones_json,
};
use rmap::mutate::TransitionFields;
use rmap::next::{format_next_task, next_tasks, ready_tasks};
use rmap::next_bundle::{NextBundleFilter, pick as pick_next_bundle};
use rmap::paths::resolve_paths;
use rmap::query::{
    TaskFilter, find_task, format_task, format_task_row, is_dispatchable, list_tasks,
};
use rmap::schema::{Task, Tasks};
use rmap::schema_json::schema_json_str;
use rmap::stale::{find_awaiting_landing, find_stale, parse_duration};
use rmap::today_iso;
use rmap::topo::{transitive_dependencies, transitive_dependents};
use rmap::validate::{ValidateError, validate_tasks_file, validate_tasks_str};
use std::num::NonZeroUsize;
use std::process::ExitCode;

/// Exit code for "task <id> not found" — a specific, named task does not exist.
///
/// Distinct from the generic failure code (1) so machine consumers (e.g.
/// harness's `Harness.Roadmap.classify_failure/2`) can branch on the structured
/// signal instead of regex-matching the English stderr.
const EXIT_TASK_NOT_FOUND: u8 = 3;

/// Exit code for "the roadmap file is unreadable, missing, or malformed TOML".
///
/// Corresponds to `ValidateError::Parse`. Semantic validation failures keep the
/// generic code (1) — they mean the roadmap parsed but violates a rule, not that
/// it could not be loaded at all.
const EXIT_INVALID_ROADMAP: u8 = 4;

/// A named task could not be found in the roadmap.
///
/// A typed error (rather than `anyhow::anyhow!`) so `main` can map it to
/// `EXIT_TASK_NOT_FOUND` via `downcast_ref` while preserving the existing stderr
/// wording for humans.
#[derive(Debug, thiserror::Error)]
#[error("task {0} not found")]
struct TaskNotFound(String);

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::from(exit_code_for(&err))
        }
    }
}

/// Maps a failure to a structured exit code, the machine-readable half of the
/// CLI contract. `task <id> not found` and an unloadable/malformed roadmap each
/// get a distinct code; everything else is a generic failure (1).
fn exit_code_for(err: &anyhow::Error) -> u8 {
    if err.downcast_ref::<TaskNotFound>().is_some() {
        EXIT_TASK_NOT_FOUND
    } else if matches!(
        err.downcast_ref::<ValidateError>(),
        Some(ValidateError::Parse { .. })
    ) {
        EXIT_INVALID_ROADMAP
    } else {
        1
    }
}

/// `--fields` projection for `list` / `ready`, mapping the unknown-field error
/// to a CLI failure (exit 1) naming the offending field.
fn project_fields(tasks: &Tasks, selection: &[&Task], fields: &[String]) -> Result<String> {
    project_fields_json_str(tasks, selection, fields).map_err(|name| {
        anyhow::anyhow!("unknown --fields name: '{name}' (run `rmap schema` for valid fields)")
    })
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Validate {
            tasks_path,
            check_render,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            if check_render && !roadmap_is_current(&paths)? {
                eprintln!("ROADMAP.md is out of sync; run rmap render");
                return Ok(ExitCode::from(2));
            }
            if !check_render {
                validate_tasks_file(&paths.tasks_path)?;
            }

            println!("valid");
        }
        Commands::Render {
            tasks_path,
            roadmap_path,
            data_path,
            dry,
            stdout,
            html,
            multi,
            out,
        } => {
            if !multi.is_empty() {
                render_portfolio_cmd(&multi, html, dry, stdout, out)?;
            } else {
                let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
                render(paths, dry, stdout, html)?;
            }
        }
        Commands::Watch {
            tasks_path,
            roadmap_path,
            data_path,
            json,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            watch_command(paths, json)?;
        }
        Commands::Status {
            id,
            new_status,
            implemented,
            delivered_by,
            verified,
            verified_by,
            verification_ref,
            shipped_in,
            reason,
            report,
            attempt_by,
            landing_ref,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_status(
                paths,
                &id,
                &new_status,
                TransitionFields {
                    implemented: implemented.as_deref(),
                    delivered_by: delivered_by.as_deref(),
                    verified: if verified { Some(true) } else { None },
                    verified_by: verified_by.as_deref(),
                    verification_ref: verification_ref.as_deref(),
                    shipped_in: shipped_in.as_deref(),
                    blocked_reason: reason.as_deref(),
                    attempt_report: report.as_deref(),
                    attempt_by: attempt_by.as_deref(),
                    landing_ref: landing_ref.as_deref(),
                },
            )?;
        }
        Commands::Next {
            marker,
            bundle,
            milestone,
            count,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = TaskFilter {
                status: None,
                marker,
                phase: None,
                bundle,
                target_repo: None,
                milestone,
                delivered_by: None,
            };

            let count = count.get();
            let selected = next_tasks(&tasks, &filter, count);
            if count == 1 {
                let first = selected.first().copied();
                if json {
                    println!("{}", export_task_json_str(&tasks, first)?);
                } else if let Some(task) = first {
                    println!("{}", format_next_task(task));
                }
            } else if json {
                println!("{}", export_tasks_array_json_str(&tasks, &selected)?);
            } else {
                for task in &selected {
                    println!("{}", format_next_task(task));
                }
            }
        }
        Commands::Show {
            id,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let task = find_task(&tasks, &id).ok_or_else(|| TaskNotFound(id.clone()))?;

            if json {
                println!("{}", export_task_json_str(&tasks, Some(task))?);
            } else {
                println!("{}", format_task(task));
            }
        }
        Commands::Blocks {
            id,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            // Resolve `id` first so an unknown task exits 3 (not an empty list).
            find_task(&tasks, &id).ok_or_else(|| TaskNotFound(id.clone()))?;
            let all: Vec<&Task> = tasks.task.iter().collect();
            let result = transitive_dependents(&all, &id);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &result)?);
            } else {
                for task in &result {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Deps {
            id,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            find_task(&tasks, &id).ok_or_else(|| TaskNotFound(id.clone()))?;
            let all: Vec<&Task> = tasks.task.iter().collect();
            let result = transitive_dependencies(&all, &id);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &result)?);
            } else {
                for task in &result {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Specs { json, tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let specs = rmap::specs::load(&tasks, &paths.tasks_path).map_err(anyhow::Error::msg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&specs)?);
            } else {
                print!("{}", rmap::specs::format_specs(&specs));
            }
        }
        Commands::List {
            rule,
            status,
            marker,
            phase,
            bundle,
            target_repo,
            milestone,
            delivered_by,
            dispatchable,
            fields,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = TaskFilter {
                status,
                marker,
                phase,
                bundle,
                target_repo,
                milestone,
                delivered_by,
            };
            let mut listed = list_tasks(&tasks, &filter);
            if let Some(rule) = rule {
                listed.retain(|task| task.spec_changes.iter().any(|change| change.rule == rule));
            }
            if dispatchable {
                listed.retain(|task| is_dispatchable(task));
            }

            if !fields.is_empty() {
                println!("{}", project_fields(&tasks, &listed, &fields)?);
            } else if json {
                println!("{}", export_filtered_json_str(&tasks, &listed)?);
            } else {
                for task in listed {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Ready {
            marker,
            bundle,
            phase,
            milestone,
            count,
            dispatchable,
            fields,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = TaskFilter {
                status: None,
                marker,
                phase,
                bundle,
                target_repo: None,
                milestone,
                delivered_by: None,
            };
            let selected = ready_tasks(&tasks, &filter, count.map(NonZeroUsize::get), dispatchable);

            if !fields.is_empty() {
                println!("{}", project_fields(&tasks, &selected, &fields)?);
            } else if json {
                println!("{}", export_filtered_json_str(&tasks, &selected)?);
            } else {
                for task in &selected {
                    println!("{}", format_next_task(task));
                }
            }
        }
        Commands::Schema => {
            println!("{}", schema_json_str()?);
        }
        Commands::Diff {
            against,
            json,
            verbose,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let current = validate_tasks_file(&paths.tasks_path)?;
            let against = against.unwrap_or_else(|| current.default_branch.clone());
            let base_input = git_show_tasks(&paths, &against)?;
            let base = validate_tasks_str(
                format!("{}:{}", against, repo_relative_tasks_path(&paths)?),
                &base_input,
            )?;
            let diff = diff_toml(&base, &current, verbose);

            if json {
                println!("{}", serde_json::to_string_pretty(&diff)?);
            } else {
                println!("{}", format_diff(&diff, verbose));
            }
        }
        Commands::Delegate { id, to, tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let task = find_task(&tasks, &id).ok_or_else(|| TaskNotFound(id.clone()))?;
            let target = resolve_target(task, to).map_err(anyhow::Error::msg)?;
            let prompt = format_delegate_prompt(&tasks, &id, target)
                .ok_or_else(|| TaskNotFound(id.clone()))?;

            let specs = rmap::specs::load(&tasks, &paths.tasks_path).map_err(anyhow::Error::msg)?;
            print!("{prompt}{}", rmap::specs::delegate_section(task, &specs));
        }
        Commands::Import { tasks_path } => {
            let project = resolve_paths(tasks_path, None, None)
                .ok()
                .and_then(|paths| validate_tasks_file(&paths.tasks_path).ok())
                .map(|tasks| tasks.project)
                .unwrap_or_else(|| "<your-project>".to_string());
            let prompt = format_import_prompt(&project);
            print!("{prompt}");
        }
        Commands::Export {
            command: ExportCommands::Json { tasks_path },
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            println!("{}", export_json_str(&tasks)?);
        }
        Commands::Export {
            command: ExportCommands::Dot { tasks_path },
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            println!("{}", format_dot(&build_dot(&tasks)));
        }
        Commands::Waves { json, tasks_path } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let waves = build_waves(&tasks);
            if json {
                println!("{}", format_waves_json(&waves));
            } else {
                println!("{}", format_waves(&waves));
            }
        }
        Commands::Mark {
            id,
            ops,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_markers(paths, &id, &ops)?;
        }
        Commands::Milestone {
            id,
            name,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_milestone(paths, &id, &name)?;
        }
        Commands::Assign {
            id,
            assignee,
            model,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_assignee(paths, &id, &assignee, model.as_deref())?;
        }
        Commands::Depend {
            id,
            on,
            other,
            cross_repo,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            add_dependency(
                paths,
                &id,
                on.as_deref(),
                other.as_deref(),
                cross_repo.as_deref(),
            )?;
        }
        Commands::New {
            from_stdin,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            create_task(paths, from_stdin)?;
        }
        Commands::Bundles {
            phase,
            has_next,
            in_focus,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = BundleFilter {
                phase,
                has_next,
                in_focus,
            };
            let summaries = list_bundles(&tasks, &filter);

            if json {
                let envelope = bundles_json(&tasks, summaries);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                print!("{}", format_bundles_human(&tasks, &summaries));
            }
        }
        Commands::Milestones {
            has_next,
            status,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = MilestoneFilter { has_next, status };
            let summaries = list_milestones(&tasks, &filter);

            if json {
                let envelope = milestones_json(&tasks, summaries);
                println!("{}", serde_json::to_string_pretty(&envelope)?);
            } else {
                print!("{}", format_milestones_human(&tasks, &summaries));
            }
        }
        Commands::NextBundle {
            phase,
            bundle,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;

            if let Some(requested) = bundle.as_deref()
                && !tasks.bundles.contains_key(requested)
            {
                bail!("bundle '{requested}' is not declared in tasks.toml");
            }

            let filter = NextBundleFilter {
                phase,
                bundle: bundle.clone(),
            };
            let today = today_iso();
            let pick = pick_next_bundle(&tasks, &filter, &today);
            let effective_focus = phase.or(tasks.focus.as_ref().map(|f| f.phase));

            let non_empty_pick = pick.as_ref().filter(|p| !p.tasks.is_empty());

            if json {
                println!(
                    "{}",
                    export_bundle_pick_json_str(&tasks, effective_focus, non_empty_pick)?
                );
            } else if let Some(p) = non_empty_pick {
                print!("{}", format_next_bundle_human(&tasks, p));
            } else if let Some(name) = bundle.as_deref() {
                eprintln!("none — bundle '{name}' has no actionable pending tasks");
            } else if let Some(focus) = effective_focus {
                eprintln!("none — no actionable bundle in phase {focus}");
            } else {
                eprintln!("none — no actionable bundle in any phase");
            }
        }
        Commands::CriticalPath {
            milestone,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let filter = CriticalPathFilter { milestone };
            let chain = critical_path(&tasks, &filter);

            if json {
                println!("{}", export_tasks_array_json_str(&tasks, &chain)?);
            } else if chain.is_empty() {
                // Empty stdout — same contract as stale / list with no matches.
            } else {
                println!("{}", format_critical_path_human(&chain));
            }
        }
        Commands::Stale {
            over,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let max_age = parse_duration(&over).map_err(|e| anyhow::anyhow!(e))?;
            let today = today_iso();
            let stale = find_stale(&tasks, max_age, &today);
            let awaiting = find_awaiting_landing(&tasks);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &stale)?);
            } else {
                for task in stale {
                    println!("{}", format_task_row(task));
                }
                if !awaiting.is_empty() {
                    println!("awaiting landing ({})", awaiting.len());
                    for task in awaiting {
                        println!("{}", format_task_row(task));
                    }
                }
            }
        }
        Commands::Doctor {
            threshold_days,
            ac_threshold,
            bottleneck_min,
            near_duplicate_min,
            json,
            tasks_path,
            roadmap_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, None)?;
            let input = std::fs::read_to_string(&paths.tasks_path)
                .with_context(|| format!("read {}", paths.tasks_path.display()))?;
            // If tasks.toml won't parse at all, propagate the error (nothing to analyze).
            let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &input)?;
            let roadmap_input = std::fs::read_to_string(&paths.roadmap_path).ok();
            let today = today_iso();
            let thresholds = DoctorThresholds::resolve(
                threshold_days,
                ac_threshold,
                bottleneck_min,
                near_duplicate_min,
            );

            let report = DoctorReport::run(
                &tasks,
                &paths.tasks_path.display().to_string(),
                &input,
                roadmap_input.as_deref(),
                &today,
                thresholds,
            );

            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{report}");
            }
        }
    }

    Ok(ExitCode::SUCCESS)
}
