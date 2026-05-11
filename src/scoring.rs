use crate::schema::Task;

pub fn efficiency(task: &Task) -> f64 {
    f64::from(task.scores.b + task.scores.u) / (2.0 * f64::from(task.scores.d))
}

pub fn format_efficiency(value: f64) -> String {
    let formatted = format!("{value:.2}");
    let trimmed = formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();

    if trimmed.contains('.') {
        trimmed
    } else {
        format!("{trimmed}.0")
    }
}
