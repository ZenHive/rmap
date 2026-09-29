use anyhow::{Context, Result, bail};
use rmap::creation_input::StdinTask;
use rmap::schema::Scores;

/// Drive a `dialoguer` prompt flow to build one `NewTaskFields` for interactive
/// `rmap new`. Uses the loaded `Tasks` to populate `Select` lists for phase /
/// bundle. Refuses to create a bundle on the fly — the user must author the
/// `[bundles.<name>]` table manually before referencing it.
pub(crate) fn prompt_task_fields(existing: &rmap::schema::Tasks) -> Result<StdinTask> {
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

    let milestone = prompt_milestone(&theme, existing)?;

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

    let mut out_of_scope: Vec<String> = Vec::new();
    loop {
        let next: String = Input::with_theme(&theme)
            .with_prompt("Out-of-scope item (empty to stop)")
            .allow_empty(true)
            .interact_text()?;
        if next.trim().is_empty() {
            break;
        }
        out_of_scope.push(next);
        if !Confirm::with_theme(&theme)
            .with_prompt("Add another?")
            .default(false)
            .interact()?
        {
            break;
        }
    }

    let mut domains: Vec<String> = Vec::new();
    loop {
        let next: String = Input::with_theme(&theme)
            .with_prompt("Domain (empty to stop)")
            .allow_empty(true)
            .interact_text()?;
        if next.trim().is_empty() {
            break;
        }
        domains.push(next);
        if !Confirm::with_theme(&theme)
            .with_prompt("Add another?")
            .default(false)
            .interact()?
        {
            break;
        }
    }

    let assignee_choices = [
        "(skip)",
        "human",
        "claude",
        "codex",
        "cursor",
        "grok",
        "antigravity",
        "pi",
        "droid",
        "kimi",
    ];
    let assignee_index = Select::with_theme(&theme)
        .with_prompt("Assignee")
        .items(assignee_choices)
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

    let model_input: String = Input::with_theme(&theme)
        .with_prompt("Model (empty to skip)")
        .allow_empty(true)
        .interact_text()?;
    let model = if model_input.trim().is_empty() {
        None
    } else {
        Some(model_input)
    };

    Ok(StdinTask {
        phase: phase_number,
        bundle,
        milestone,
        title,
        scores: Scores { d, b, u },
        markers,
        linear_id,
        assignee,
        module,
        model,
        acceptance_criteria,
        out_of_scope,
        domains,
        // Power-user fields not exposed by the interactive prompt — set via
        // `rmap new --from-stdin` or edit `tasks.toml` directly.
        ..Default::default()
    })
}

/// Interactive milestone selector for `rmap new`. Lists declared milestones
/// sorted by `order` with a leading "Skip" entry. Returns `None` when "Skip"
/// is chosen, or when no `[milestones.*]` are declared (auto-skip — no prompt
/// shown).
fn prompt_milestone(
    theme: &dialoguer::theme::ColorfulTheme,
    existing: &rmap::schema::Tasks,
) -> Result<Option<String>> {
    use dialoguer::Select;

    let mut entries: Vec<(&String, &rmap::schema::Milestone)> =
        existing.milestones.iter().collect();
    if entries.is_empty() {
        return Ok(None);
    }
    entries.sort_by_key(|(_, m)| m.order);

    let mut items: Vec<String> = vec!["(skip)".to_string()];
    items.extend(
        entries
            .iter()
            .map(|(key, m)| format!("{key} — {} [{}]", m.name, m.status)),
    );

    let index = Select::with_theme(theme)
        .with_prompt("Milestone")
        .items(&items)
        .default(0)
        .interact()?;

    Ok(if index == 0 {
        None
    } else {
        Some(entries[index - 1].0.clone())
    })
}

/// When transitioning to `done` without `--implemented`, surface an interactive
/// prompt on a TTY for any matched task that is missing the field. Returns the
/// entered string (applied to every matched task by `update_status_many_str`),
/// or `None` when all matched tasks already carry `implemented` (no prompt
/// needed) or when stdin is not a TTY (let `validate_implemented` surface the
/// error so scripts/agents get a clean message instead of a hang).
pub(crate) fn prompt_for_implemented(input: &str, ids: &[&str]) -> Result<Option<String>> {
    let parsed: rmap::schema::Tasks = match toml::from_str(input) {
        Ok(t) => t,
        Err(_) => return Ok(None),
    };

    let any_missing = parsed
        .task
        .iter()
        .filter(|t| ids.iter().any(|id| *id == t.id.to_string()))
        .any(|t| t.implemented.is_none());

    if !any_missing {
        return Ok(None);
    }

    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return Ok(None);
    }

    use dialoguer::{Input, theme::ColorfulTheme};
    let theme = ColorfulTheme::default();
    let prompt_label = if ids.len() == 1 {
        format!("implemented (what shipped for task {})", ids[0])
    } else {
        format!("implemented (applied to all {} tasks)", ids.len())
    };
    let value: String = Input::with_theme(&theme)
        .with_prompt(prompt_label)
        .validate_with(|s: &String| -> std::result::Result<(), &str> {
            if s.trim().is_empty() {
                Err("implemented cannot be empty when transitioning to done")
            } else {
                Ok(())
            }
        })
        .interact_text()?;
    Ok(Some(value))
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
