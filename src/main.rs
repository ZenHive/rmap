use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rmap::bundles::{BundleFilter, bundles_json, format_bundles_human, list_bundles};
use rmap::delegate::{DelegateTarget, format_delegate_prompt};
use rmap::diff::{diff_toml, format_diff};
use rmap::doctor::DoctorReport;
use rmap::export::{export_filtered_json_str, export_json_str, export_task_json_str};
use rmap::mutate::{
    CrossRepoSpec, MarkerOp, NewTaskFields, add_dependency_str, add_task_str, update_markers_str,
    update_status_many_str,
};
use rmap::next::{format_next_task, next_task};
use rmap::paths::{ResolvedPaths, resolve_paths};
use rmap::query::{TaskFilter, find_task, format_task, format_task_row, list_tasks};
use rmap::render::render_roadmap_str;
use rmap::schema::{Scores, TaskId};
use rmap::schema_json::schema_json_str;
use rmap::stale::{find_stale, parse_duration};
use rmap::today_iso;
use rmap::validate::{validate_tasks_file, validate_tasks_str};

#[derive(Debug, Parser)]
#[command(name = "rmap")]
#[command(about = "Manage portable roadmap data")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Validate {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        check_render: bool,
    },
    Render {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
        #[arg(long, conflicts_with = "stdout")]
        dry: bool,
        #[arg(long)]
        stdout: bool,
    },
    Status {
        id: String,
        new_status: String,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    Next {
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        bundle: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Show {
        id: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        marker: Option<String>,
        #[arg(long)]
        phase: Option<u32>,
        #[arg(long)]
        bundle: Option<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Schema,
    Diff {
        #[arg(long)]
        against: Option<String>,
        #[arg(long)]
        json: bool,
        /// Add per-field before/after values to Changed entries (whitelist-gated).
        #[arg(long)]
        verbose: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Delegate {
        id: String,
        #[arg(long)]
        to: DelegateTarget,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    Export {
        #[command(subcommand)]
        command: ExportCommands,
    },
    /// Aggregate health report: validation findings + stale + score-decay + render drift.
    ///
    /// Always exits 0 — informational. Use `rmap validate` for strict schema gating.
    /// Exception: if tasks.toml is unparseable, exits non-zero (nothing to analyze).
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
    },
    /// Add or remove markers on a task: `rmap mark 75 +cx -parallel`.
    ///
    /// Each `ops` token must start with `+` (add) or `-` (remove). Add is a no-op
    /// when the marker is already present; remove is a no-op when absent.
    /// The full file is re-validated after edit and any invalid marker name
    /// (per `VALID_MARKERS`) aborts the write.
    Mark {
        id: String,
        /// `+marker` to add, `-marker` to remove. One or more, applied left-to-right.
        #[arg(required = true, num_args = 1.., allow_hyphen_values = true)]
        ops: Vec<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Add a dependency to a task.
    ///
    /// In-repo: `rmap depend 75 on 74`. Cross-repo: `rmap depend 75 --cross-repo
    /// ccxt_client:42:blocks` (relation defaults to `blocks` if omitted). Both
    /// forms can be combined in a single call. Cycles and unknown ids are
    /// rejected via re-validation.
    Depend {
        id: String,
        /// Literal `on` keyword separating the in-repo dependency. Required when
        /// the next positional is the target task id.
        on: Option<String>,
        /// Target task id when adding an in-repo dependency.
        other: Option<String>,
        /// Cross-repo dependency: `<repo>:<task_id>[:<relation>]`.
        #[arg(long)]
        cross_repo: Option<String>,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// Create a new task and append it to `tasks.toml`.
    ///
    /// With `--from-stdin`, reads a TOML fragment from stdin (one or more
    /// `[[task]]` blocks, accepting the same field set as `schema::Task`).
    /// Without `--from-stdin`, drops into an interactive `dialoguer` prompt
    /// flow — requires a TTY. In both modes, the id is auto-allocated
    /// (numeric `max + 1`) unless the caller supplies one explicitly, and
    /// `created_at` / `scored_at` default to `today_iso()`. Re-validates and
    /// re-renders ROADMAP.md + data.json on success; on failure the file is
    /// left byte-equal to its pre-call state.
    New {
        /// Read one-or-more `[[task]]` blocks as TOML from stdin instead of prompting.
        #[arg(long)]
        from_stdin: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
        #[arg(long)]
        roadmap_path: Option<PathBuf>,
        #[arg(long)]
        data_path: Option<PathBuf>,
    },
    /// List declared bundles with per-bundle counts and next-task hints.
    ///
    /// Read-only discovery aid for `rmap next --bundle <name>` and
    /// `rmap list --bundle <name>` — surfaces every `[bundles.*]` so callers
    /// don't have to grep `tasks.toml`. Bundles group under per-phase headers;
    /// focus-phase bundles sort first.
    Bundles {
        #[arg(long)]
        phase: Option<u32>,
        /// Only bundles whose next_task is non-null.
        #[arg(long)]
        has_next: bool,
        /// Only bundles in `[focus].phase`.
        #[arg(long)]
        in_focus: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
    /// List in-progress tasks idle longer than the given duration.
    Stale {
        /// Duration threshold. Units: d (day), w (7d), m (30d), y (365d). Example: "30d", "2w", "6m", "1y".
        #[arg(long)]
        over: String,
        /// Emit JSON envelope.
        #[arg(long)]
        json: bool,
        /// Path to tasks.toml (overrides discovery).
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum ExportCommands {
    Json {
        #[arg(long)]
        tasks_path: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
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
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            render(paths, dry, stdout)?;
        }
        Commands::Status {
            id,
            new_status,
            tasks_path,
            roadmap_path,
            data_path,
        } => {
            let paths = resolve_paths(tasks_path, roadmap_path, data_path)?;
            update_status(paths, &id, &new_status)?;
        }
        Commands::Next {
            marker,
            bundle,
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
            };

            let task = next_task(&tasks, &filter);
            if json {
                println!("{}", export_task_json_str(task)?);
            } else if let Some(task) = task {
                println!("{}", format_next_task(task));
            }
        }
        Commands::Show {
            id,
            json,
            tasks_path,
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            let task =
                find_task(&tasks, &id).ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;

            if json {
                println!("{}", export_task_json_str(Some(task))?);
            } else {
                println!("{}", format_task(task));
            }
        }
        Commands::List {
            status,
            marker,
            phase,
            bundle,
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
            };
            let listed = list_tasks(&tasks, &filter);

            if json {
                println!("{}", export_filtered_json_str(&tasks, &listed)?);
            } else {
                for task in listed {
                    println!("{}", format_task_row(task));
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
            let prompt = format_delegate_prompt(&tasks, &id, to)
                .ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;

            print!("{prompt}");
        }
        Commands::Export {
            command: ExportCommands::Json { tasks_path },
        } => {
            let paths = resolve_paths(tasks_path, None, None)?;
            let tasks = validate_tasks_file(&paths.tasks_path)?;
            println!("{}", export_json_str(&tasks)?);
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

            if json {
                println!("{}", export_filtered_json_str(&tasks, &stale)?);
            } else {
                for task in stale {
                    println!("{}", format_task_row(task));
                }
            }
        }
        Commands::Doctor {
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

            let report = DoctorReport::run(
                &tasks,
                &paths.tasks_path.display().to_string(),
                &input,
                roadmap_input.as_deref(),
                &today,
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

fn render(paths: ResolvedPaths, dry: bool, stdout: bool) -> Result<()> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    if stdout {
        print!("{rendered_roadmap}");
        return Ok(());
    }

    if dry {
        println!("would render {}", paths.roadmap_path.display());
        println!("would export {}", paths.data_path.display());
        return Ok(());
    }

    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("rendered");

    Ok(())
}

fn update_markers(paths: ResolvedPaths, task_id: &str, op_tokens: &[String]) -> Result<()> {
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

fn add_dependency(
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

fn update_status(paths: ResolvedPaths, task_id: &str, new_status: &str) -> Result<()> {
    let ids: Vec<&str> = task_id
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if ids.is_empty() {
        anyhow::bail!("no task ids provided (got {:?})", task_id);
    }

    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let updated = update_status_many_str(
        paths.tasks_path.display().to_string(),
        &input,
        &ids,
        new_status,
    )?;

    let tasks = validate_tasks_str(paths.tasks_path.display().to_string(), &updated)?;
    let (rendered_roadmap, rendered_data) = render_outputs(&paths, &tasks)?;

    std::fs::write(&paths.tasks_path, updated)
        .with_context(|| format!("write {}", paths.tasks_path.display()))?;
    write_outputs(&paths, rendered_roadmap, rendered_data)?;
    println!("updated");

    Ok(())
}

/// Top-level for `rmap new` and `rmap new --from-stdin`. Shares the load /
/// mutate / re-validate / re-render / write tail with the other mutators.
fn create_task(paths: ResolvedPaths, from_stdin: bool) -> Result<()> {
    let input = std::fs::read_to_string(&paths.tasks_path)
        .with_context(|| format!("read {}", paths.tasks_path.display()))?;

    let path_label = paths.tasks_path.display().to_string();

    let new_tasks: Vec<StdinTask> = if from_stdin {
        let mut stdin_buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin_buf)
            .context("read stdin")?;
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

        let explicit_id = match &task.id {
            Some(TaskId::Number(n)) => Some(*n),
            Some(TaskId::Text(text)) => bail!(
                "stdin task id {text:?}: text ids are not supported by `rmap new --from-stdin` — omit `id` to auto-allocate or use a numeric id"
            ),
            None => None,
        };

        let fields = NewTaskFields {
            id: explicit_id,
            phase: task.phase,
            bundle: task.bundle.as_str(),
            title: task.title.as_str(),
            scores: (task.scores.d, task.scores.b, task.scores.u),
            status: "pending",
            markers: &markers,
            depends_on: &depends_on,
            acceptance_criteria: &acceptance_criteria,
            assignee: task.assignee.as_deref(),
            linear_id: task.linear_id.as_deref(),
            module: task.module.as_deref(),
            body: task.body.as_deref(),
            created_at: Some(task.created_at.as_deref().unwrap_or(today.as_str())),
            scored_at: Some(task.scored_at.as_deref().unwrap_or(today.as_str())),
        };

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

/// Task-shaped stdin row. Field set matches `schema::Task` except `id` is
/// optional (auto-allocate when absent). `status` is excluded — creation
/// produces `"pending"` tasks only. Lifecycle timestamps (`started_at`,
/// `done_at`, `blocked_reason`, `shipped_in`) are also excluded; `rmap status`
/// owns those transitions.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StdinTask {
    pub id: Option<TaskId>,
    pub phase: u32,
    pub bundle: String,
    pub title: String,
    pub scores: Scores,
    #[serde(default)]
    pub markers: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<TaskId>,
    pub linear_id: Option<String>,
    pub assignee: Option<String>,
    pub module: Option<String>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub body: Option<String>,
    pub created_at: Option<String>,
    pub scored_at: Option<String>,
}

/// Drive a `dialoguer` prompt flow to build one `NewTaskFields` for interactive
/// `rmap new`. Uses the loaded `Tasks` to populate `Select` lists for phase /
/// bundle. Refuses to create a bundle on the fly — the user must author the
/// `[bundles.<name>]` table manually before referencing it.
fn prompt_task_fields(existing: &rmap::schema::Tasks) -> Result<StdinTask> {
    use dialoguer::{Confirm, Input, MultiSelect, Select, theme::ColorfulTheme};

    let theme = ColorfulTheme::default();

    let mut phase_keys: Vec<&String> = existing.phases.keys().collect();
    phase_keys.sort_by_key(|key| existing.phases.get(*key).map_or(u32::MAX, |p| p.order));
    if phase_keys.is_empty() {
        bail!("no phases declared in tasks.toml; add a `[phases.<n>]` table first");
    }
    let phase_labels: Vec<String> = phase_keys
        .iter()
        .map(|key| {
            let phase = &existing.phases[*key];
            format!("{} — {}", key, phase.name)
        })
        .collect();
    let phase_index = Select::with_theme(&theme)
        .with_prompt("Phase")
        .items(&phase_labels)
        .default(0)
        .interact()?;
    let phase_key = phase_keys[phase_index];
    let phase_number: u32 = phase_key
        .parse()
        .with_context(|| format!("phase key {phase_key:?} is not a u32"))?;

    let mut bundle_entries: Vec<(&String, &rmap::schema::Bundle)> = existing
        .bundles
        .iter()
        .filter(|(_, bundle)| bundle.phase == phase_number)
        .collect();
    bundle_entries.sort_by_key(|(_, bundle)| bundle.order);
    let bundle_keys: Vec<&String> = bundle_entries.iter().map(|(key, _)| *key).collect();
    if bundle_keys.is_empty() {
        bail!(
            "no `[bundles.<name>]` declared for phase {phase_number}; add one to tasks.toml before creating a task"
        );
    }
    let bundle_index = Select::with_theme(&theme)
        .with_prompt("Bundle")
        .items(&bundle_keys)
        .default(0)
        .interact()?;
    let bundle = bundle_keys[bundle_index].clone();

    let title: String = Input::with_theme(&theme)
        .with_prompt("Title")
        .validate_with(|input: &String| -> std::result::Result<(), &str> {
            if input.trim().is_empty() {
                Err("title must not be empty")
            } else {
                Ok(())
            }
        })
        .interact_text()?;

    let d: u32 = prompt_score(&theme, "Difficulty (1-10)")?;
    let b: u32 = prompt_score(&theme, "Benefit (1-10)")?;
    let u: u32 = prompt_score(&theme, "Urgency (1-10)")?;

    let marker_choices = rmap::validate::VALID_MARKERS;
    let marker_selection = MultiSelect::with_theme(&theme)
        .with_prompt("Markers (space to toggle)")
        .items(marker_choices)
        .interact()?;
    let markers: Vec<String> = marker_selection
        .into_iter()
        .map(|i| marker_choices[i].to_string())
        .collect();

    let mut acceptance_criteria: Vec<String> = Vec::new();
    loop {
        let next: String = Input::with_theme(&theme)
            .with_prompt("Acceptance criterion (empty to stop)")
            .allow_empty(true)
            .interact_text()?;
        if next.trim().is_empty() {
            break;
        }
        acceptance_criteria.push(next);
        if !Confirm::with_theme(&theme)
            .with_prompt("Add another?")
            .default(false)
            .interact()?
        {
            break;
        }
    }

    let assignee_choices = ["(skip)", "human", "claude", "codex", "cursor"];
    let assignee_index = Select::with_theme(&theme)
        .with_prompt("Assignee")
        .items(&assignee_choices)
        .default(0)
        .interact()?;
    let assignee = if assignee_index == 0 {
        None
    } else {
        Some(assignee_choices[assignee_index].to_string())
    };

    let linear_id_input: String = Input::with_theme(&theme)
        .with_prompt("Linear id (empty to skip)")
        .allow_empty(true)
        .interact_text()?;
    let linear_id = if linear_id_input.trim().is_empty() {
        None
    } else {
        Some(linear_id_input)
    };

    let module_input: String = Input::with_theme(&theme)
        .with_prompt("Module (empty to skip)")
        .allow_empty(true)
        .interact_text()?;
    let module = if module_input.trim().is_empty() {
        None
    } else {
        Some(module_input)
    };

    Ok(StdinTask {
        id: None,
        phase: phase_number,
        bundle,
        title,
        scores: Scores { d, b, u },
        markers,
        depends_on: Vec::new(),
        linear_id,
        assignee,
        module,
        acceptance_criteria,
        body: None,
        created_at: None,
        scored_at: None,
    })
}

fn prompt_score(theme: &dialoguer::theme::ColorfulTheme, label: &str) -> Result<u32> {
    use dialoguer::Input;
    let value: u32 = Input::<u32>::with_theme(theme)
        .with_prompt(label)
        .validate_with(|input: &u32| -> std::result::Result<(), &str> {
            if (1..=10).contains(input) {
                Ok(())
            } else {
                Err("score must be 1-10")
            }
        })
        .interact_text()?;
    Ok(value)
}

fn render_outputs(paths: &ResolvedPaths, tasks: &rmap::schema::Tasks) -> Result<(String, String)> {
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered_roadmap = render_roadmap_str(&roadmap, tasks)?;
    let rendered_data = export_json_str(tasks)?;

    Ok((rendered_roadmap, rendered_data))
}

fn roadmap_is_current(paths: &ResolvedPaths) -> Result<bool> {
    let tasks = validate_tasks_file(&paths.tasks_path)?;
    let roadmap = std::fs::read_to_string(&paths.roadmap_path)
        .with_context(|| format!("read {}", paths.roadmap_path.display()))?;
    let rendered = render_roadmap_str(&roadmap, &tasks)?;

    Ok(rendered == roadmap)
}

fn write_outputs(
    paths: &ResolvedPaths,
    rendered_roadmap: String,
    rendered_data: String,
) -> Result<()> {
    std::fs::write(&paths.roadmap_path, rendered_roadmap)
        .with_context(|| format!("write {}", paths.roadmap_path.display()))?;
    std::fs::write(&paths.data_path, rendered_data)
        .with_context(|| format!("write {}", paths.data_path.display()))?;

    Ok(())
}

fn git_show_tasks(paths: &ResolvedPaths, against: &str) -> Result<String> {
    let repo_path = repo_relative_tasks_path(paths)?;
    let object = format!("{against}:{repo_path}");
    let output = std::process::Command::new("git")
        .arg("show")
        .arg(&object)
        .current_dir(&paths.project_root)
        .output()
        .with_context(|| format!("run git show {object}"))?;

    if !output.status.success() {
        bail!(
            "could not read {} from git ref {}\n{}",
            repo_path,
            against,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8(output.stdout).context("git show returned non-UTF-8 tasks.toml")
}

fn repo_relative_tasks_path(paths: &ResolvedPaths) -> Result<String> {
    let repo_root = git_repo_root(&paths.project_root)?;
    let tasks_path = std::fs::canonicalize(&paths.tasks_path)
        .with_context(|| format!("canonicalize {}", paths.tasks_path.display()))?;
    let relative = tasks_path.strip_prefix(&repo_root).with_context(|| {
        format!(
            "{} is not inside git repository {}",
            tasks_path.display(),
            repo_root.display()
        )
    })?;

    Ok(relative.to_string_lossy().to_string())
}

fn git_repo_root(project_root: &std::path::Path) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(project_root)
        .output()
        .context("run git rev-parse --show-toplevel")?;

    if !output.status.success() {
        bail!(
            "could not find git repository for {}\n{}",
            project_root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    let path = String::from_utf8(output.stdout).context("git rev-parse returned non-UTF-8 path")?;
    std::fs::canonicalize(path.trim()).with_context(|| format!("canonicalize {}", path.trim()))
}
