# Post-merge audit: f9e6feb

Revision: `f9e6feb70dc989c4ef9e858bbeafa463db4a42f6`.
Range: `11521eb2f9c9e96a4daa167760458577914f9cb1..f9e6feb70dc989c4ef9e858bbeafa463db4a42f6`.

## Review

Reviewed the landed dependency refreshes, open-task wave scheduling, score-decay filtering, ranking tie-break, task-field registry and typed vocabulary refactor, command/module split, HTML board and detail-panel changes, documentation, ignore rules and roadmap transitions. Checked the registry coverage tests and changed scheduling/ranking tests against the implementations. Searched source/templates for debug output and unimplemented placeholders; no accidental debug code found. Existing changelog describes the user-facing releases. No reviewer rejections were recorded, so no false-rejection finding applies.

## Findings and fixes

1. README still instructed contributors to use three-place field edits. Fixed it to reference the single-source task registry and existing contributor procedure.
2. DESIGN.md described the old three-column board, top-three portfolio cards and obsolete colors/no-animation rule. Updated the layout and semantic-selector descriptions to match the shared five-lane board, dispatch racks, detail panel and repository cords; linked the visual-system document and preserved the accessibility requirement.
3. `ready --help` omitted the computed-unlocks tie-break. Corrected its clap documentation to match the shipped ranking.
4. Portfolio relation resolution accepts project name, input label and path basename, with first-match collision behavior (`src/render_html/portfolio.rs`). Detail links only index project names, with last-match behavior (`templates/_board.js`). This source-level mismatch can leave a valid relation's task link unresolved. Filed **task 64**, assigned to codex / gpt-6-astra, for shared resolution and interaction regression coverage. No browser reproduction was performed; implementation and independent review belong to that task.

**4 findings; 3 fixed; 1 tracked as task 64.** No runtime implementation changed in this audit. The explicit audit discovery-filing instruction was followed: `rmap new --from-stdin --tasks-path <assigned-worktree>/roadmap/tasks.toml` created task 64 and regenerated its views. An initial attempt used an undeclared bundle and was rejected without a write; the successful fragment uses the existing `html_portfolio` bundle.

## Integrated QA

Status: **passed** for the configured checks on the original integrated revision, before any edits.

Configured command (exact):

```sh
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Before edits, executed cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test at the specified revision. Format passed silently; Clippy finished dev profile in 11.73s without warnings; cargo test finished test profile in 17.51s and passed 447 tests across 16 nonempty suites, plus empty binary/doc-test suites. All commands exited 0. CI config contains the same format/Clippy/test checks; no configured coverage or clone check. Elixir analyzers are inapplicable. After documentation edits: cargo fmt --check; cargo test --test cli equal_eff (1 passed); rmap validate (valid); git diff --check (clean); ready --help inspected.

The working tree was clean and HEAD matched the requested revision when the command started. Every conjunct completed successfully; no independent check was skipped because of a failure. Coverage and clone detection are not configured in Cargo or CI; Dialyzer, Reach, Sobelow, Credo and Elixir Doctor are inapplicable to this Rust repository. No external provider/database/server was used. The passing configured suite does not establish browser-interaction coverage or resolve task 64.

Test totals: library 85; changelog 4; CLI 222; delegate 16; diff 12; export 4; field registry 4; import 9; mutate 18; next 17; paths 4; query 4; render 6; roundtrip 2; skills smoke 1; validate 39. Binary unit tests and doc-tests each had zero tests. Total **447 passed, 0 failed, 0 ignored**.

## Captured full test output

```text
   Compiling hyper v1.11.1
   Compiling hyper-util v0.1.21
   Compiling aws-lc-rs v1.18.1
   Compiling rustls v0.23.45
   Compiling rustls-webpki v0.103.15
   Compiling tokio-rustls v0.26.6
   Compiling rustls-platform-verifier v0.7.1
   Compiling hyper-rustls v0.27.10
   Compiling reqwest v0.13.5
   Compiling jsonschema v0.58.2
    Finished `test` profile [unoptimized + debuginfo] target(s) in 17.51s
     Running unittests src/lib.rs (target/debug/deps/rmap-3ea133c4d117b1ca)

running 85 tests
test critical_path::tests::critical_path_empty_when_milestone_has_no_tasks ... ok
test critical_path::tests::critical_path_chain_of_three ... ok
test critical_path::tests::critical_path_diamond_picks_longer_arm ... ok
test critical_path::tests::critical_path_single_root_task ... ok
test critical_path::tests::critical_path_milestone_scopes_to_pinned_and_transitive_deps ... ok
test doctor::tests::placeholder_ignores_quoted_examples_and_non_stubs ... ok
test doctor::tests::placeholder_detects_unquoted_todo_tbd_and_stubs ... ok
test doctor::tests::jaccard_threshold_integer_percent ... ok
test export::tests::exported_task_fields_cover_serialized_keys ... ok
test doctor::tests::vague_detects_only_vague_wording ... ok
test graph_export::tests::build_waves_skips_terminal_tasks_but_keeps_in_flight_ones ... ok
test mutate::tests::update_status_many_str_empty_ids_returns_empty_ids_error ... ok
test graph_export::tests::format_dot_is_graphviz_digraph_with_styled_nodes ... ok
test mutate::tests::add_task_to_integer_id_file_still_writes_the_id_as_an_integer ... ok
test next_bundle::tests::force_pick_overrides_ranking ... ok
test graph_export::tests::build_dot_emits_in_repo_edges_only ... ok
test graph_export::tests::format_waves_human_lines ... ok
test mutate::tests::update_assignee_str_rejects_agent_without_model_on_live_task ... ok
test mutate::tests::add_task_to_string_id_file_continues_the_numeric_sequence ... ok
test next_bundle::tests::missing_bundle_returns_none ... ok
test mutate::tests::update_assignee_str_unknown_id_errors ... ok
test next_bundle::tests::all_blocked_bundle_skipped ... ok
test render_html::tests::truncate_label_keeps_short_titles_and_clips_long_ones ... ok
test graph_export::tests::build_waves_groups_by_dep_layer ... ok
test next_bundle::tests::focus_phase_beats_higher_eff_other_phase ... ok
test next_bundle::tests::ties_broken_by_bundle_order ... ok
test graph_export::tests::format_waves_json_is_layer_map ... ok
test mutate::tests::add_task_to_string_id_file_writes_the_id_as_a_string ... ok
test render_html::tests::racks_rank_ready_by_eff_then_unlocks ... ok
test render_html::tests::dag_emits_one_edge_per_dependency ... ok
test render_html::tests::dag_single_node_sits_at_padding_origin ... ok
test next_bundle::tests::unmet_external_dep_skips_bundle ... ok
test mutate::tests::update_assignee_str_sets_agent_and_model ... ok
test render_html::tests::render_html_str_carries_phase_status_badge_and_attr ... ok
test scoring::tests::days_since_same_day ... ok
test scoring::tests::tier_glyph_boundaries ... ok
test scoring::tests::days_since_one_day ... ok
test mutate::tests::update_assignee_str_clears_assignee_and_model ... ok
test render_html::tests::lanes_split_pending_by_dependency_state ... ok
test stale::tests::find_awaiting_landing_ignores_blank_and_non_in_progress ... ok
test stale::tests::find_stale_boundary_stale_one_over_threshold ... ok
test stale::tests::find_stale_boundary_not_stale_at_exact_threshold ... ok
test stale::tests::find_stale_empty_when_all_fresh ... ok
test stale::tests::find_stale_excludes_in_progress_with_landing_ref ... ok
test next_bundle::tests::topo_order_respects_internal_chain ... ok
test stale::tests::find_stale_returns_only_old_in_progress ... ok
test stale::tests::find_stale_skips_missing_started_at ... ok
test stale::tests::parse_duration_bad_unit ... ok
test stale::tests::parse_duration_days ... ok
test stale::tests::parse_duration_empty ... ok
test stale::tests::parse_duration_large ... ok
test stale::tests::parse_duration_missing_unit ... ok
test stale::tests::parse_duration_months ... ok
test scoring::tests::days_since_malformed_input ... ok
test scoring::tests::days_since_31_days ... ok
test scoring::tests::days_since_across_year_boundary ... ok
test render_html::tests::render_html_str_carries_data_island_and_attrs ... ok
test stale::tests::parse_duration_non_numeric ... ok
test stale::tests::parse_duration_single_char ... ok
test stale::tests::parse_duration_weeks ... ok
test stale::tests::parse_duration_years ... ok
test tests::civil_from_days_far_future ... ok
test tests::civil_from_days_leap_day_2000 ... ok
test tests::civil_from_days_known_epoch_values ... ok
test tests::civil_from_days_roundtrips_through_days_from_civil ... ok
test render_html::tests::render_html_str_on_empty_roadmap_succeeds ... ok
test watch::tests::error_event_line_carries_message_and_omits_outputs ... ok
test watch::tests::is_tasks_toml_event_ignores_data_json ... ok
test watch::tests::is_tasks_toml_event_ignores_roadmap_md ... ok
test watch::tests::is_tasks_toml_event_matches_exact_path ... ok
test watch::tests::is_tasks_toml_event_matches_by_file_name_after_rename ... ok
test watch::tests::render_event_line_lists_both_outputs ... ok
test watch::tests::render_event_line_lists_only_changed_output ... ok
test topo::tests::dependencies_chain_is_whole_upstream_subtree ... ok
test tests::today_iso_always_produces_well_formed_date ... ok
test topo::tests::layer_no_deps_is_zero ... ok
test tests::days_from_civil_roundtrips_through_civil_from_days ... ok
test topo::tests::traversal_on_unknown_id_is_empty ... ok
test topo::tests::layer_chain_of_three ... ok
test topo::tests::dependents_chain_is_whole_downstream_subtree ... ok
test watch::tests::write_if_changed_skips_when_content_equal ... ok
test watch::tests::write_if_changed_writes_when_file_missing ... ok
test topo::tests::layer_diamond_uses_longest_path ... ok
test topo::tests::diamond_dedups_dependents_and_unlocks ... ok
test watch::tests::write_if_changed_writes_when_content_differs ... ok

test result: ok. 85 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running unittests src/main.rs (target/debug/deps/rmap-f34f642a3a57d11e)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/changelog.rs (target/debug/deps/changelog-c53a336504404b25)

running 4 tests
test archive_link_precedence_and_omission ... ok
test changelog_metadata_exports_and_diffs ... ok
test schema_describes_changelog_path_or_false_at_both_levels ... ok
test invalid_changelog_values_fail_cli_validation_and_render_before_writes ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running tests/cli.rs (target/debug/deps/cli-49c1e939fdd2351c)

running 222 tests
test bundles_command_filter_phase ... ok
test blocks_lists_transitive_downstream_subtree ... ok
test bundles_command_emits_json_envelope ... ok
test blocks_on_leaf_is_empty ... ok
test bundles_command_all_done_renders_check_glyph ... ok
test blocks_human_output_lists_task_rows ... ok
test bundles_command_all_blocked_renders_blocked_glyph ... ok
test bundles_command_filter_in_focus ... ok
test critical_path_command_emits_longest_chain_human ... ok
test bundles_command_filter_has_next ... ok
test critical_path_command_diamond_picks_longer_arm ... ok
test assign_command_rejects_missing_model ... ok
test blocks_unknown_task_exits_task_not_found ... ok
test assign_command_sets_kimi_agent_and_model ... ok
test assign_command_unsets_assignee ... ok
test bundles_command_empty_bundles ... ok
test critical_path_command_milestone_scopes_release_line ... ok
test assign_command_rejects_unknown_id ... ok
test critical_path_command_emits_ordered_json_array ... ok
test delegate_command_prints_kimi_prompt ... ok
test delegate_includes_prior_attempts ... ok
test bundles_command_groups_by_phase_with_focus_first ... ok
test bundles_command_in_flight_renders_construction_glyph ... ok
test delegate_command_prints_agent_prompt ... ok
test delegate_without_to_defaults_to_assignee ... ok
test delegate_without_to_and_human_assignee_exits_one ... ok
test delegate_unknown_task_exits_task_not_found ... ok
test delegate_without_to_and_no_assignee_exits_one ... ok
test depend_command_requires_at_least_one_dependency ... ok
test depend_command_rejects_on_without_target ... ok
test depend_command_rejects_malformed_cross_repo_spec ... ok
test delegate_rejects_unknown_target ... ok
test deps_lists_transitive_upstream_subtree ... ok
test delegate_accepts_new_local_agents_with_footers ... ok
test doctor_command_lint_findings_in_json ... ok
test doctor_command_skips_multiple_active_milestones_when_only_one_active ... ok
test doctor_command_skips_bottleneck_for_done_gating_task ... ok
test doctor_command_clean_fixture_reports_no_findings ... ok
test depend_command_handles_numeric_only_string_target_id ... ok
test doctor_command_clean_fixture_json_ok ... ok
test doctor_command_skips_milestone_fully_done_when_open_work_remains ... ok
test depend_command_rejects_cycle ... ok
test depend_command_unknown_task_id_aborts_write ... ok
test doctor_command_emits_json ... ok
test doctor_command_surfaces_degenerate_bundle ... ok
test doctor_command_bottleneck_min_threshold_suppresses_finding ... ok
test doctor_command_surfaces_milestone_fully_done_when_active ... ok
test doctor_command_surfaces_missing_acceptance_criteria ... ok
test doctor_command_surfaces_graph_bottleneck_and_isolated_nodes ... ok
test doctor_command_surfaces_stale_and_decay ... ok
test doctor_command_surfaces_multiple_active_milestones ... ok
test doctor_command_detects_drift ... ok
test doctor_command_surfaces_milestone_fully_done_but_open ... ok
test depend_command_adds_in_repo_dependency ... ok
test bundles_command_pending_with_unmet_deps_renders_pause_glyph ... ok
test doctor_command_ac_threshold_changes_missing_ac_set ... ok
test depend_command_adds_cross_repo_dependency_with_default_relation ... ok
test doctor_command_surfaces_focus_phase_closed ... ok
test doctor_command_surfaces_phase_fully_done_but_open ... ok
test doctor_score_decay_skips_terminal_tasks ... ok
test doctor_command_surfaces_phase_has_in_progress_but_pending ... ok
test export_json_command_prints_json_to_stdout ... ok
test export_dot_command_emits_graphviz_digraph ... ok
test doctor_command_thresholds_in_json ... ok
test doctor_command_surfaces_placeholder_and_vague_criteria ... ok
test doctor_command_threshold_days_suppresses_stale_and_decay ... ok
test fields_unknown_name_errors_with_offending_field ... ok
test list_command_filters_by_delivered_by ... ok
test doctor_command_surfaces_near_duplicate_open_tasks_and_threshold_flag ... ok
test list_command_filters_by_milestone ... ok
test list_command_filters_by_bundle ... ok
test doctor_emits_claimed_not_graded_for_done_without_verified ... ok
test list_command_unknown_bundle_returns_empty_envelope ... ok
test list_fields_projects_to_named_keys_only ... ok
test list_fields_projects_domains ... ok
test list_command_filters_tasks_and_prints_json_envelope ... ok
test mark_command_rejects_invalid_op_prefix ... ok
test milestones_command_filter_status ... ok
test diff_verbose_surfaces_delivered_by_and_verified_changes ... ok
test milestones_command_filter_has_next ... ok
test milestones_command_empty_milestones ... ok
test milestones_command_emits_json_envelope ... ok
test diff_verbose_surfaces_landing_ref ... ok
test milestones_command_lists_with_next_glyphs ... ok
test list_command_filters_by_target_repo_and_composes_with_status_and_bundle ... ok
test mark_command_unknown_task_id_aborts_write ... ok
test mark_command_is_idempotent_on_repeated_add_or_remove ... ok
test mark_command_adds_and_removes_markers_atomically ... ok
test new_from_stdin_batch_validates_all_field_errors_in_one_pass ... ok
test mark_command_rejects_invalid_marker_name_with_no_partial_write ... ok
test milestone_command_rejects_unknown_target ... ok
test new_command_accepts_milestone_via_stdin ... ok
test new_from_stdin_appends_task_and_rerenders ... ok
test new_from_stdin_auto_allocates_id_when_omitted ... ok
test new_from_stdin_multi_task_atomic_on_failure ... ok
test new_from_stdin_inherits_validation_for_cycles ... ok
test milestone_command_sets_then_unsets ... ok
test mark_command_places_new_markers_field_in_canonical_position ... ok
test new_from_stdin_rejects_blank_target_repo_without_writing ... ok
test new_from_stdin_rejects_landing_ref_as_unknown_field ... ok
test new_from_stdin_rejects_non_pending_status ... ok
test new_from_stdin_pinned_today ... ok
test new_from_stdin_field_list_matches_struct ... ok
test new_from_stdin_emits_canonical_order_for_creation_fields ... ok
test new_from_stdin_rejects_duplicate_id ... ok
test new_from_stdin_rejects_unknown_bundle ... ok
test next_bundle_focus_phase_wins_over_higher_sum_eff_other_phase ... ok
test new_from_stdin_reports_wrong_types_for_required_fields ... ok
test next_bundle_all_blocked_bundle_is_skipped ... ok
test next_bundle_emits_topological_order_within_bundle ... ok
test next_bundle_empty_pick_goes_to_stderr_with_exit_zero ... ok
test next_and_ready_break_equal_eff_on_unlocks ... ok
test next_bundle_json_envelope_shape_is_stable ... ok
test new_from_stdin_round_trips_kimi_assignee ... ok
test next_bundle_force_pick_bypasses_ranking ... ok
test new_from_stdin_unknown_phase_lists_valid_phases_inline ... ok
test new_from_stdin_round_trips_cross_repo_field ... ok
test new_from_stdin_round_trips_branch_field ... ok
test next_bundle_force_pick_zero_actionable_uses_bundle_specific_stderr ... ok
test next_bundle_missing_bundle_errors_with_exit_one ... ok
test new_from_stdin_tolerates_pending_status ... ok
test next_bundle_phase_override_changes_effective_focus ... ok
test next_bundle_tie_broken_by_bundle_order ... ok
test diff_command_reports_added_removed_and_changed_tasks_against_git_ref ... ok
test next_bundle_unmet_external_dep_skips_bundle ... ok
test next_command_count_zero_is_rejected_at_parse_time ... ok
test new_from_stdin_omits_out_of_scope_section_when_unset ... ok
test next_command_count_three_human_prints_one_line_per_task ... ok
test next_command_count_exceeds_eligible_returns_min_without_error ... ok
test next_command_count_three_json_emits_eff_ranked_array ... ok
test next_command_count_one_default_emits_bare_object_json ... ok
test new_from_stdin_round_trips_touches_field ... ok
test new_from_stdin_round_trips_model_field ... ok
test new_from_stdin_round_trips_files_to_modify_field ... ok
test next_command_composes_bundle_and_marker ... ok
test read_command_on_missing_roadmap_exits_invalid_roadmap ... ok
test read_command_on_malformed_roadmap_exits_invalid_roadmap ... ok
test next_command_filters_by_milestone ... ok
test new_from_stdin_round_trips_domains_field ... ok
test next_command_composes_milestone_and_bundle ... ok
test next_command_prints_highest_eff_pending_unblocked_task ... ok
test next_command_prints_nothing_and_exits_zero_when_no_task_matches ... ok
test next_command_unknown_bundle_returns_null_json ... ok
test next_command_filters_by_bundle ... ok
test next_json_prints_task_object_or_null ... ok
test ready_fields_projection_includes_dep_layer ... ok
test ready_returns_only_dep_satisfied_pending_ranked ... ok
test render_command_uses_conventional_paths_by_default ... ok
test render_html_multi_requires_html_flag ... ok
test render_command_updates_roadmap_file ... ok
test ready_bundle_filters_to_dispatchable_layer_zero_of_bundle ... ok
test ready_dispatchable_excludes_handbuild_tasks ... ok
test render_dry_reports_without_writing_files ... ok
test render_html_dry_writes_nothing ... ok
test render_command_updates_milestones_block ... ok
test render_html_writes_self_contained_index ... ok
test render_stdout_prints_roadmap_without_writing_files ... ok
test render_without_html_flag_does_not_write_index ... ok
test render_html_stdout_prints_html_without_writing ... ok
test schema_command_rejects_removed_json_flag ... ok
test schema_describes_landing_ref_as_transition_time ... ok
test render_html_multi_writes_portfolio_from_project_roots ... ok
test render_html_multi_draws_cross_repo_blocker_edges ... ok
test new_from_stdin_round_trips_out_of_scope_field ... ok
test show_unknown_task_exits_task_not_found ... ok
test show_command_renders_delivered_by_and_verified ... ok
test show_command_prints_human_and_json_views ... ok
test render_html_multi_accepts_data_json_paths ... ok
test render_html_carries_data_attrs_and_is_idempotent ... ok
test stale_command_lists_in_progress_tasks_older_than_threshold ... ok
test render_html_multi_honors_out_override_and_stdout ... ok
test stale_command_empty_result_exits_zero ... ok
test new_from_stdin_round_trips_target_repo_across_read_and_render_surfaces ... ok
test show_json_carries_computed_unlocks ... ok
test show_command_surfaces_milestone_line ... ok
test stale_command_emits_filtered_json ... ok
test status_attempt_by_without_report_warns_and_writes_nothing ... ok
test show_json_includes_dep_layer ... ok
test status_bulk_done_with_shipped_in_applies_to_all ... ok
test status_bulk_auto_fills_each_task_independently ... ok
test status_blocked_with_reason_writes_field ... ok
test stale_command_rejects_malformed_duration ... ok
test status_blocked_reason_overwrites_existing ... ok
test status_done_verified_with_evaluator_writes_provenance ... ok
test status_command_updates_tasks_and_rerenders_outputs ... ok
test status_done_verified_without_evaluator_fails_without_mutating ... ok
test status_done_with_delivered_by_and_verified_persists_both_fields ... ok
test status_done_with_shipped_in_persists_field ... ok
test status_command_multi_id_flips_all_tasks ... ok
test show_and_json_surface_landing_ref ... ok
test status_command_unknown_id_aborts_entire_write ... ok
test status_command_single_id_still_works ... ok
test status_non_pending_with_report_emits_warning_and_skips_write ... ok
test status_in_progress_landing_ref_persists_and_does_not_churn_started_at ... ok
test status_non_done_with_shipped_in_emits_warning_and_skips_write ... ok
test status_in_progress_auto_fills_started_at ... ok
test status_empty_landing_ref_clears_field ... ok
test stale_and_doctor_fold_landing_ref_under_awaiting_landing ... ok
test status_pending_does_not_set_timestamps ... ok
test status_non_blocked_with_reason_emits_warning_and_skips_write ... ok
test validate_check_render_exits_two_when_roadmap_is_stale ... ok
test validate_check_render_detects_stale_milestones_block ... ok
test schema_json_command_emits_parseable_schema_for_tasks_file ... ok
test status_non_done_with_delivered_by_emits_warning_and_skips_write ... ok
test status_preserves_existing_done_at_on_reflip ... ok
test status_landing_ref_rejected_on_other_statuses ... ok
test validate_check_render_keeps_validation_errors_as_exit_one ... ok
test version_flag_prints_crate_version ... ok
test validate_command_rejects_duplicate_task_ids ... ok
test target_repo_is_documented_in_schema_and_list_help ... ok
test validate_command_rejects_invalid_tasks_file ... ok
test status_pending_report_accumulates_across_attempts ... ok
test validate_command_accepts_valid_tasks_file ... ok
test waves_command_json_emits_layer_map ... ok
test waves_command_prints_dispatch_schedule ... ok
test validate_check_render_tasks_path_derives_conventional_roadmap_path ... ok
test status_pending_with_report_appends_attempt_and_leaves_siblings_intact ... ok
test validate_check_render_accepts_current_roadmap ... ok
test status_leaving_blocked_clears_blocked_reason ... ok
test status_done_and_blocked_keep_landing_ref_pending_clears ... ok
test validate_accepts_new_agent_assignees ... ok
test validate_requires_model_for_live_agent_tasks ... ok

test result: ok. 222 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.08s

     Running tests/delegate.rs (target/debug/deps/delegate-e65bf3b520c88c0e)

running 16 tests
test claude_target_emits_local_environment_footer ... ok
test files_to_modify_falls_back_to_module_when_empty ... ok
test delegate_defaults_target_repo_to_roadmap_project ... ok
test codex_target_emits_codex_environment_footer ... ok
test delegate_emits_milestone_bullet_without_target_version ... ok
test cursor_target_emits_cursor_environment_footer ... ok
test formats_minimal_delegate_prompt_without_optional_sections ... ok
test delegate_emits_milestone_bullet_with_target_version ... ok
test delegate_omits_milestone_bullet_when_unset ... ok
test files_to_modify_section_omitted_when_both_empty ... ok
test emits_canonical_section_order_with_distinguishing_line ... ok
test omits_model_bullet_when_unset ... ok
test formats_full_delegate_prompt_for_agent_target ... ok
test missing_delegate_task_returns_none ... ok
test omits_stored_assignee_when_target_matches ... ok
test omits_domains_bullet_when_unset ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/diff.rs (target/debug/deps/diff-b37a0d1f5abba80a)

running 12 tests
test linear_subfield_changes_report_with_field_granularity ... ok
test classifies_metadata_scalar_optional_and_map_changes ... ok
test unchanged_metadata_produces_no_entries ... ok
test focus_add_remove_and_change_show_up_in_metadata_diff ... ok
test verbose_metadata_emits_before_after_for_whitelisted_keys ... ok
test diff_surfaces_milestone_pinned_on_task ... ok
test diff_surfaces_added_milestone_entry ... ok
test classifies_added_removed_and_changed_tasks ... ok
test diff_surfaces_removed_milestone_entry ... ok
test verbose_emits_before_after_for_whitelisted_changed_fields ... ok
test verbose_off_produces_no_values_field ... ok
test verbose_added_and_removed_task_entries_carry_no_values ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/export.rs (target/debug/deps/export-b90354565a314cff)

running 4 tests
test rejects_eff_in_source_toml ... ok
test exports_top_level_focus_when_set ... ok
test omits_focus_key_when_absent ... ok
test exports_validated_tasks_with_computed_efficiency ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/field_registry.rs (target/debug/deps/field_registry-2c8d9dd43cbaa436)

running 4 tests
test vocabularies_roundtrip_known_and_invalid_wire_strings ... ok
test transition_fields_are_excluded_from_creation ... ok
test every_task_field_has_export_and_unique_canonical_position ... ok
test registry_creation_fields_flow_through_input_writer_export_and_diff ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/import.rs (target/debug/deps/import-5adfe1b17eb709f0)

running 9 tests
test emits_verification_loop ... ok
test emits_task_section ... ok
test emits_instructions ... ok
test includes_project_name ... ok
test emits_schema_section ... ok
test emits_field_mapping_guide ... ok
test emits_context_section ... ok
test emits_marker_pair_contract ... ok
test placeholder_project_when_no_tasks_toml ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/mutate.rs (target/debug/deps/mutate-5b527d16b971078c)

running 18 tests
test update_markers_existing_field_does_not_reorder ... ok
test update_markers_idempotent_add_does_not_reorder ... ok
test update_status_rejects_landing_ref_on_non_in_progress ... ok
test update_status_changes_only_target_status_and_preserves_comments ... ok
test update_status_verified_requires_evaluator_provenance ... ok
test update_status_rejects_provenance_without_verification ... ok
test update_markers_new_field_lands_in_canonical_position ... ok
test update_status_rejects_unknown_task_id ... ok
test update_status_ignores_reason_on_non_blocked_transition ... ok
test update_status_rejects_invalid_status ... ok
test update_status_landing_ref_persists_on_in_progress ... ok
test update_status_writes_verification_provenance ... ok
test update_status_landing_ref_on_already_in_progress_leaves_started_at ... ok
test update_status_empty_landing_ref_clears_field ... ok
test update_status_supports_string_task_ids ... ok
test update_status_clears_landing_ref_on_pending ... ok
test update_status_blocked_writes_then_auto_clears_reason ... ok
test update_status_preserves_landing_ref_on_done_and_blocked ... ok

test result: ok. 18 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/next.rs (target/debug/deps/next-9d872a4db9219bc6)

running 17 tests
test focus_phase_falls_back_when_no_candidate_in_focus ... ok
test focus_phase_beats_active_milestone_when_dominance_diverges ... ok
test excludes_tasks_blocked_by_incomplete_dependencies ... ok
test active_milestone_wins_over_higher_eff_in_other_milestones ... ok
test focus_and_active_milestone_combined_win_over_either_alone ... ok
test focus_phase_wins_over_higher_eff_in_other_phases ... ok
test multiple_active_milestones_all_qualify ... ok
test next_tasks_count_exceeds_eligible_returns_min ... ok
test marker_filter_excludes_non_matching_tasks ... ok
test explicit_milestone_filter_within_inactive_milestone_falls_back_to_eff ... ok
test next_tasks_count_three_returns_eff_ranked_array ... ok
test next_tasks_focus_phase_fills_before_other_phases ... ok
test next_tasks_count_one_returns_singleton_matching_next_task ... ok
test no_active_milestones_preserves_focus_only_behavior ... ok
test selects_highest_eff_pending_unblocked_task ... ok
test ties_preserve_toml_order ... ok
test no_matching_next_task_returns_none ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/paths.rs (target/debug/deps/paths-668c527057bd7167)

running 4 tests
test missing_default_tasks_file_returns_clear_error ... ok
test explicit_tasks_path_derives_project_root_for_conventional_layout ... ok
test resolves_default_paths_from_project_root ... ok
test resolves_default_paths_from_nested_directory ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/query.rs (target/debug/deps/query-fc2db1d2d3e7d999)

running 4 tests
test finds_task_by_numeric_or_string_id ... ok
test lists_tasks_matching_all_filters_in_toml_order ... ok
test target_repo_filter_uses_explicit_value_and_roadmap_project_default ... ok
test empty_filter_lists_every_task_in_toml_order ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/render.rs (target/debug/deps/render-00723b355cec1562)

running 6 tests
test render_rejects_unclosed_milestones_marker ... ok
test render_rejects_unclosed_focus_marker ... ok
test render_rejects_unclosed_mermaid_marker ... ok
test render_rejects_unclosed_marker ... ok
test render_rejects_unclosed_vision_marker ... ok
test golden_render_fixtures_are_byte_equal ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/roundtrip.rs (target/debug/deps/roundtrip-e98f784c5988ba47)

running 2 tests
test toml_edit_round_trip_preserves_unmodified_tasks_file ... ok
test golden_tasks_round_trip_without_spurious_diff ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/skills_smoke.rs (target/debug/deps/skills_smoke-bc9a47cf9521554d)

running 1 test
test skills_md_bash_blocks_match_declared_exit_codes ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.88s

     Running tests/validate.rs (target/debug/deps/validate-9cb0e832a04be888)

running 39 tests
test accepts_dependency_across_id_forms ... ok
test accepts_blocked_status_with_reason ... ok
test accepts_focus_phase_matching_declared_phase ... ok
test accepts_done_status_with_implemented ... ok
test accepts_scores_at_range_edges ... ok
test accepts_kimi_assignee_with_model_and_acceptance_criteria ... ok
test accepts_unique_ids_mixed_forms ... ok
test detects_self_cycle_across_id_forms ... ok
test accepts_task_with_declared_milestone ... ok
test rejects_blank_acceptance_criterion_on_agent_assigned_live_task ... ok
test accepts_well_formed_timestamps ... ok
test rejects_agent_assigned_live_task_without_acceptance_criteria ... ok
test rejects_blank_target_repo ... ok
test rejects_dependency_cycle_between_tasks ... ok
test rejects_blocked_status_without_blocked_reason ... ok
test rejects_blank_or_contradictory_verification_provenance ... ok
test permits_missing_acceptance_criteria_for_human_or_unassigned_tasks ... ok
test rejects_done_status_without_implemented ... ok
test rejects_duplicate_task_id_cross_form ... ok
test rejects_duplicate_task_id_same_form ... ok
test rejects_score_above_maximum ... ok
test rejects_invalid_assignee ... ok
test rejects_score_below_minimum ... ok
test rejects_focus_phase_referencing_unknown_phase ... ok
test rejects_invalid_cross_repo_relation ... ok
test rejects_invalid_status ... ok
test rejects_linear_id_that_does_not_match_team_key ... ok
test rejects_orphan_dependencies ... ok
test rejects_milestone_with_invalid_status ... ok
test rejects_invalid_marker ... ok
test task_id_partial_eq_u32_accepts_numeric_text_form ... ok
test rejects_self_dependency_cycle ... ok
test rejects_invalid_timestamp_format ... ok
test skips_linear_id_format_check_when_linear_table_is_absent ... ok
test rejects_unknown_schema_version_with_migration_hint ... ok
test rejects_task_that_references_unknown_bundle ... ok
test rejects_task_referencing_unknown_milestone ... ok
test rejects_task_that_references_unknown_phase ... ok
test valid_tasks_toml_deserializes_and_validates ... ok

test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests rmap

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


```

