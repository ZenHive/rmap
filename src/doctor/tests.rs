use super::*;

#[test]
fn placeholder_detects_unquoted_todo_tbd_and_stubs() {
    assert!(criterion_has_placeholder("TODO finish the API surface"));
    assert!(criterion_has_placeholder("details are TBD"));
    assert!(criterion_has_placeholder("still ??? here"));
    assert!(criterion_has_placeholder(
        "implement <feature_name> handler"
    ));
    assert!(criterion_has_placeholder("todo: lowercase whole word"));
}

#[test]
fn placeholder_ignores_quoted_examples_and_non_stubs() {
    assert!(!criterion_has_placeholder(
        "quoted examples like 'TODO' or 'TBD' in prose do not count"
    ));
    assert!(!criterion_has_placeholder(
        "contains 'TBD' as prose example"
    ));
    // Possessive apostrophe must not open a quote span and un-quote later examples.
    assert!(!criterion_has_placeholder(
        "a live agent-assigned task's acceptance_criteria contains 'TODO' or 'TBD'"
    ));
    assert!(!criterion_has_placeholder(
        "compare a < b and c > d without stubs"
    ));
    assert!(!criterion_has_placeholder("no placeholders at all"));
    assert!(!criterion_has_placeholder("empty <> is not a stub"));
}

#[test]
fn vague_detects_only_vague_wording() {
    assert!(criterion_is_vague("works properly"));
    assert!(criterion_is_vague("it works"));
    assert!(criterion_is_vague("fast and robust"));
    assert!(criterion_is_vague("correctly"));
    assert!(!criterion_is_vague(
        "renders in under 200ms so it feels fast"
    ));
    assert!(!criterion_is_vague("tests pass under cargo test"));
    assert!(!criterion_is_vague(""));
}

#[test]
fn jaccard_threshold_integer_percent() {
    let a: HashSet<String> = ["add", "auth", "flow", "login"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let b: HashSet<String> = ["add", "auth", "flow", "signup"]
        .into_iter()
        .map(str::to_string)
        .collect();
    // |∩|=3, |∪|=5 → 60%
    assert!(jaccard_at_least(&a, &b, 60));
    assert!(!jaccard_at_least(&a, &b, 61));
    assert!(!jaccard_at_least(&a, &HashSet::new(), 50));
}
