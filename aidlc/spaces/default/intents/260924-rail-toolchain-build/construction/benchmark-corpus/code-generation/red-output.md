# Red output — benchmark-corpus (U16) corpus checker

The failing output of each Red step, trimmed to the relevant lines (test names, results, and the first panic message). Command for every checker step: `cargo test --locked --no-fail-fast --manifest-path corpus/tools/check/Cargo.toml`. Stubs return `todo!()`, so each failure is "not yet implemented", not a compile error.

## Step 2 (helper, written test-first alongside Step 1)

`cargo test --locked --manifest-path corpus/tools/helper/Cargo.toml` with an empty `main`:

```
test unknown_or_malformed_commands_exit_two_with_one_usage_line ... FAILED
test echo_prints_its_arguments_on_stdout ... FAILED
test exit_uses_the_given_status ... FAILED
test stderr_prints_on_stderr_only ... FAILED
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out
```

## Step 3 — Data model — Red

```
     Running tests/index.rs (corpus/tools/check/target/debug/deps/index-826c1be31a741f91)
test an_undelivered_class_passes_unless_the_corpus_must_be_complete ... FAILED
test a_missing_run_data_field_names_the_program_and_the_field ... FAILED
test definition_ids_must_be_well_formed_and_unique ... FAILED
test composition_requires_the_exact_count_for_each_delivered_class ... FAILED
test parses_every_index_entry_field ... FAILED
test run_data_must_be_consistent ... FAILED
test composition_rejects_unapproved_programs ... FAILED
test only_schema_version_one_and_unique_programs_are_accepted ... FAILED
test layout_names_each_missing_part ... FAILED
test layout_passes_for_a_complete_program ... FAILED
test result: FAILED. 1 passed; 10 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/pins.rs (corpus/tools/check/target/debug/deps/pins-0316d3e2bfe8b758)
test matching_versions_are_accepted ... FAILED
test the_rust_pin_must_equal_the_workspace_toolchain ... FAILED
test parses_approved_libraries ... FAILED
test reads_the_version_from_each_tool_output ... FAILED
test a_missing_pin_is_named ... FAILED
test parses_the_pins ... FAILED
test a_version_mismatch_names_the_toolchain_and_both_versions ... FAILED
test result: FAILED. 0 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
     Running tests/toml.rs (corpus/tools/check/target/debug/deps/toml-6c78cd954cc1e6b6)
test reads_arrays_of_tables_and_multiline_arrays ... FAILED
test reads_scalars_tables_arrays_and_comments ... FAILED
test errors_name_the_line ... FAILED
test reads_inline_tables_and_string_escapes ... FAILED
test result: FAILED. 0 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
thread 'an_undelivered_class_passes_unless_the_corpus_must_be_complete'  panicked at src/index.rs:50:5:
```

Totals at Red: 1 passing (the data-only approved-list test), 21 failing across `tests/index.rs`, `tests/pins.rs` and `tests/toml.rs`; every failure panics with `not yet implemented` (src/index.rs:50 and the other stubs). The test `entries_must_be_ordered_by_class_then_program_id` was added to `tests/index.rs` after this run and before any implementation; it would also have failed against the stubs.

Green (Step 4): `index` 12 passed, `pins` 7 passed, `toml` 4 passed. Refactor (Step 5): clippy fixes, plus `tests/finding.rs` pinning the finding message format.

## Step 6 — Repository / data access — Red

```
$ cargo test --locked --no-fail-fast --manifest-path corpus/tools/check/Cargo.toml --test repo
test sha256_matches_the_published_test_vectors ... FAILED
test sha256_handles_every_padding_boundary ... FAILED
test a_run_becomes_expected_output_only_in_the_two_allowed_shapes ... FAILED
test check_mode_passes_a_matching_run ... FAILED
test expected_output_round_trips_through_the_expected_directory ... FAILED
test bless_mode_writes_from_the_rust_version_only ... FAILED
test tree_listing_ignores_timestamps_but_not_content ... FAILED
test check_mode_never_writes_expected_output ... FAILED
test a_fixed_input_file_over_64_kib_is_refused_by_name ... FAILED
test tree_listing_is_sorted_by_path_bytes_with_content_checksums ... FAILED
test fixed_inputs_at_the_limit_pass ... FAILED
test listed_fixed_inputs_must_exist_and_unlisted_ones_must_not ... FAILED
test more_than_eight_fixed_inputs_are_refused ... FAILED
test a_deterministic_generator_passes ... FAILED
test a_generator_whose_file_set_changes_between_runs_fails ... FAILED
test a_generator_that_fails_is_reported_with_its_exit_code ... FAILED
test a_generator_whose_bytes_change_between_runs_fails_by_name ... FAILED
(each failure: panicked ... not yet implemented, from the todo!() stubs in src/sha256.rs, tree.rs, proc.rs, inputs.rs, expected.rs)
```

Green (Step 7): `repo` 17 passed. Refactor (Step 8): SHA-256 moved to `as_chunks` for clippy; tests stay green.

## Step 9 — Business logic — Red

```
$ PATH="/opt/homebrew/bin:$HOME/.opam/default/bin:$PATH" cargo test --locked --no-fail-fast --manifest-path corpus/tools/check/Cargo.toml
a_clean_program_passes ... FAILED
a_go_requirement_is_refused ... FAILED
a_matching_run_passes ... FAILED
a_mismatch_names_the_first_differing_byte_and_both_values ... FAILED
a_missing_sandbox_tool_is_an_error_that_stops_everything ... FAILED
a_one_byte_mismatch_fails_with_the_first_differing_byte ... FAILED
a_program_past_its_deadline_is_killed ... FAILED
a_program_past_the_deadline_fails_and_does_not_hold_the_check ... FAILED
a_rust_build_failure_carries_the_compiler_message ... FAILED
a_rust_dependency_is_refused_in_every_dependency_table ... FAILED
a_rust_package_without_a_lockfile_is_not_built ... FAILED
a_setup_file_belongs_to_a_listed_fixed_input ... FAILED
a_shorter_or_longer_output_reports_where_it_ends ... FAILED
a_stray_file_in_the_run_directory_is_caught_by_the_tree_check ... FAILED
a_thread_dependent_program_fails_naming_both_thread_counts ... FAILED
a_write_outside_the_run_directory_is_stopped_and_reported ... FAILED
builds_a_go_module ... FAILED
builds_a_rust_package_offline_from_its_lockfile ... FAILED
builds_an_ocaml_dune_project ... FAILED
domainslib_is_allowed_only_in_data_parallel_programs ... FAILED
every_rust_package_commits_its_lockfile ... FAILED
input_arguments_and_stdin_come_from_the_copied_input ... FAILED
nothing_runs_when_the_sandbox_cannot_be_prepared ... FAILED
ocaml_libraries_outside_the_distribution_are_refused ... FAILED
opening_a_socket_is_stopped_and_reported ... FAILED
permission_denied_cases_are_created_at_run_time ... FAILED
rayon_is_allowed_only_in_data_parallel_programs ... FAILED
rayon_needs_an_approved_pin_even_in_data_parallel ... FAILED
reading_the_users_home_directory_is_stopped_and_reported ... FAILED
runaway_output_is_cut_off_and_the_program_stopped ... FAILED
setup_creates_unreadable_files_and_read_only_directories_then_restores_them ... FAILED
setup_files_describe_run_time_cases ... FAILED
the_budget_stops_the_running_program_and_records_the_rest_as_not_run ... FAILED
the_canary_catches_a_write_a_loose_sandbox_allowed ... FAILED
the_generator_manifest_is_checked_too ... FAILED
the_linux_command_exposes_only_what_a_program_needs ... FAILED
the_macos_command_passes_the_profile_and_its_parameters ... FAILED
the_probe_reports_whether_the_operation_was_allowed ... FAILED
the_rail_version_is_not_built ... FAILED
the_sandbox_command_for_this_platform_starts_the_wrapper ... FAILED
the_self_test_fails_for_a_sandbox_that_does_not_confine ... FAILED
the_self_test_passes_on_this_platform ... FAILED
the_whole_process_group_is_killed_at_the_deadline ... FAILED
versions_compare_as_dotted_numbers ... FAILED
```

Totals at Red: 44 failing and 1 passing (the proc test that only captures output) across `tests/build.rs` (6 failing), `tests/manifest.rs` (9), `tests/proc.rs` (3 failing: deadline, process-group kill, output limit; the stub `run_command` waits for the child and enforces nothing), `tests/repo.rs` (4 new failing), `tests/runs.rs` (14) and `tests/sandbox.rs` (8). The earlier suites (index, pins, toml, finding, the first 17 repo tests) stay green. Stubs panic with `not yet implemented` (`src/sandbox.rs`, `check.rs`, `manifest.rs`, `build.rs`, `setup.rs`, `probe.rs`, `budget.rs`).

Note: the stand-in `hang` sleeps 8 seconds and `flood` writes 200 MiB then stops, so the Red run finishes instead of hanging; the checker enforces the deadline and the output limit well inside those bounds.

Green (Step 10): `build` 6, `manifest` 9, `proc` 4, `repo` 21, `runs` 14, `sandbox` 8 all pass (index 12, pins 7, toml 4, finding 1 unchanged). Implementation notes: the macOS profile imports Apple's `system.sb` (a bare deny-default profile aborts in dyld); the three sandbox tests check the program was refused by the OS (it exits 7 on a refusal) and that no file, listing or connection got through. `dune` crashes (SIGSEGV) in this environment when it picks its own job count, so the OCaml build passes `-j <n>` explicitly. Refactor (Step 11): `cargo clippy --all-targets -D warnings` and `cargo fmt` clean.

## Step 12 — API / endpoint — Red

```
$ PATH="/opt/homebrew/bin:$HOME/.opam/default/bin:$PATH" cargo test --locked --no-fail-fast --manifest-path corpus/tools/check/Cargo.toml --test cli
bad_arguments_are_refused_with_a_message ... FAILED
defaults_cover_the_whole_corpus_in_check_mode ... FAILED
rail_bless_in_the_environment_is_the_same_as_the_flag ... FAILED
every_flag_is_understood ... FAILED
check_sh_runs_the_checker_against_another_corpus_directory ... FAILED
check_sh_builds_the_checker_and_passes_its_arguments_and_exit_code_through ... FAILED
check_sh_passes_on_the_corpus_as_it_stands ... FAILED
usage_errors_exit_2 ... FAILED
a_nondeterministic_generator_fails_the_check ... FAILED
a_mismatch_exits_1_naming_program_language_input_and_byte ... FAILED
a_passing_corpus_exits_0_with_one_line_per_program_and_language ... FAILED
bless_writes_expected_output_from_rust_and_the_next_check_passes ... FAILED
a_corpus_problem_exits_1_and_nothing_runs ... FAILED
a_toolchain_mismatch_exits_2_naming_both_versions ... FAILED
the_time_budget_stops_the_check_and_fails_it ... FAILED
class_and_program_flags_scope_the_run ... FAILED
all_three_languages_each_get_a_line ... FAILED
result: FAILED. 0 passed; 17 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
```

All 17 fail: `corpus_check::cli::{parse_args, run}`, `pins::detect_installed` and `Report::lines` are `todo!()` stubs, and `corpus/check.sh` is a placeholder that exits 0 (so the script tests fail on missing output, not on a missing file). Coverage of the 17: argument parsing (4), end-to-end exit codes 0/1/2 and one line per program and language (8), the budget flag (1), `check.sh` (3), generator determinism (1).

Green (Step 13): `cli` 18 passed (4 argument-parsing tests, 11 end-to-end tests through the real binary and the sandbox, 3 `check.sh` tests). Refactor (Step 14): `run_programs` takes a `Dirs` struct instead of eight arguments (no clippy allow); clippy and rustfmt clean on both corpus packages.

Design change found at Step 13 (tests adjusted, not weakened): BR1.1 "ten per class" cannot hold while a class is still being written (BR9.1 delivers one class at a time), so `check_composition` now takes the list of classes that must be whole. A class is whole once its owner-written `corpus/<class>/APPROVAL.md` exists (BR4.2), or for all four classes under `--complete`. An unapproved program name (BR1.2) is refused in every class at all times. The three composition tests in `tests/index.rs` were updated for the new signature, and `an_approved_class_must_be_complete_and_complete_requires_all_four` was added in `tests/cli.rs`.

Final run, all green (macOS, Rust 1.98.1, Go 1.27.2, OCaml 5.5.1, dune 3.24.2):

```
$ PATH="/opt/homebrew/bin:$HOME/.opam/default/bin:$PATH" CORPUS_CHECK_REQUIRE_TOOLCHAINS=1 cargo test --locked --manifest-path corpus/tools/check/Cargo.toml
build 6, cli 18, finding 1, index 12, manifest 9, pins 7, proc 4, repo 21, runs 14, sandbox 8, toml 4  = 104 passed, 0 failed
$ cargo test --locked --manifest-path corpus/tools/helper/Cargo.toml
cli 4 passed
```
