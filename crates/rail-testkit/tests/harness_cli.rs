//! Acceptance harness tests: the `rail-accept` program. Usage errors
//! (BR3.3), exit codes (BR7.4), `--list-groups` (NFR9.17),
//! `--check-ratchet` (NFR9.15), `--compare` (BR7.7) and the terminal
//! summary (BR7.6, NFR9.18).

mod common;

use std::path::Path;
use std::process::{Command, Output};

use rail_testkit::TempDir;
use rail_testkit::harness::report::{
    ObservationWriter, Outcome, Overall, RunReport, SuiteRun, observations_rel_path, write_results,
};
use rail_testkit::harness::settings::{Build, Level, Platform, SuiteId};

use common::{data, valid_text};

fn accept(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rail-accept"))
        .args(args)
        .output()
        .expect("start rail-accept")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn path(p: &Path) -> &str {
    p.to_str().expect("UTF-8 path")
}

#[test]
fn br3_3_bad_arguments_are_usage_errors_with_exit_code_2() {
    let valid = data("valid.toml");
    let settings = path(&valid);
    let cases: Vec<Vec<&str>> = vec![
        vec![],
        vec!["--level", "weekly"],
        vec!["--level", "pr", "--suite", "T99", "--settings", settings],
        vec!["--level", "pr", "--seed", "5", "--settings", settings],
        vec![
            "--level",
            "pr",
            "--seed",
            "5",
            "--suite",
            "T1",
            "--suite",
            "T2",
            "--settings",
            settings,
        ],
        vec![
            "--level",
            "pr",
            "--seed",
            "five",
            "--suite",
            "T1",
            "--settings",
            settings,
        ],
        vec![
            "--level",
            "pr",
            "--rail",
            "debug=/nonexistent/rail",
            "--settings",
            settings,
        ],
        vec![
            "--level",
            "pr",
            "--rail",
            "fast=/bin/sh",
            "--settings",
            settings,
        ],
        vec!["--level", "pr", "--group", "smoke", "--settings", settings],
        vec!["--level", "pr", "--frobnicate"],
        vec!["--check-ratchet", settings],
    ];
    for args in cases {
        let out = accept(&args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", stderr(&out));
        assert!(stderr(&out).contains("usage"), "{args:?}: {}", stderr(&out));
        assert_eq!(stdout(&out), "", "{args:?}");
    }
}

#[test]
fn br1_a_settings_error_exits_2_naming_file_line_and_key() {
    let bad = data("unknown-key.toml");
    let out = accept(&["--level", "pr", "--settings", path(&bad)]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        stderr(&out).contains("unknown-key.toml:20:"),
        "{}",
        stderr(&out)
    );
    assert!(stderr(&out).contains("colour"), "{}", stderr(&out));
}

#[test]
fn nfr9_17_list_groups_prints_the_nightly_groups_as_json() {
    let valid = data("valid.toml");
    let out = accept(&["--list-groups", "--settings", path(&valid)]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert_eq!(stdout(&out), "[\"smoke\"]\n");
}

#[test]
fn nfr9_15_check_ratchet_exit_codes() {
    let dir = TempDir::new("cli-ratchet");
    let base = dir.path().join("base.toml");
    let lowered = dir.path().join("lowered.toml");
    let invalid = dir.path().join("invalid.toml");
    std::fs::write(&base, valid_text()).unwrap();
    std::fs::write(
        &lowered,
        valid_text().replacen("count = 1000000", "count = 10", 1),
    )
    .unwrap();
    std::fs::write(
        &invalid,
        valid_text().replacen("pr_minutes = 10", "pr_minutes = 0", 1),
    )
    .unwrap();

    let same = accept(&["--check-ratchet", path(&base), "--against", path(&base)]);
    assert_eq!(same.status.code(), Some(0), "{}", stdout(&same));

    let out = accept(&["--check-ratchet", path(&base), "--against", path(&lowered)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("suite T1"), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("count 1000000 -> count 10"),
        "{}",
        stdout(&out)
    );

    let missing = dir.path().join("main-has-none.toml");
    let out = accept(&["--check-ratchet", path(&missing), "--against", path(&base)]);
    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout(&out).contains("no base settings"),
        "{}",
        stdout(&out)
    );

    let out = accept(&["--check-ratchet", path(&base), "--against", path(&invalid)]);
    assert_eq!(out.status.code(), Some(2), "{}", stderr(&out));
}

fn results_with_t1(dir: &Path, platform: Platform, stdout_text: &str) -> String {
    let id = SuiteId::parse("T1").unwrap();
    let rel = observations_rel_path(Level::Pr, id, platform);
    let mut writer = ObservationWriter::create(dir, &rel).unwrap();
    writer
        .push(&rail_json::Value::object([
            ("input", rail_json::Value::str("m0")),
            ("stdout", rail_json::Value::str(stdout_text)),
        ]))
        .unwrap();
    writer.finish().unwrap();
    let mut run = SuiteRun::without_running(id, Build::Debug, Outcome::Pass, String::new());
    run.reason = None;
    run.observations_path = Some(rel);
    let report = RunReport {
        level: Level::Pr,
        platform,
        reference_machine: None,
        runs: vec![run],
        overall: Overall::Pass,
        over_budget: false,
        budget: None,
    };
    write_results(&report, dir, None)
        .unwrap()
        .display()
        .to_string()
}

#[test]
fn br7_7_compare_exit_codes() {
    let dir = TempDir::new("cli-compare");
    let valid = data("valid.toml");
    let mac = results_with_t1(&dir.path().join("mac"), Platform::Macos, "same");
    let linux = results_with_t1(&dir.path().join("linux"), Platform::Linux, "same");
    let out = accept(&["--compare", &mac, &linux, "--settings", path(&valid)]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}{}",
        stdout(&out),
        stderr(&out)
    );

    let other = results_with_t1(&dir.path().join("linux2"), Platform::Linux, "different");
    let out = accept(&["--compare", &mac, &other, "--settings", path(&valid)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(stdout(&out).contains("input 0 (m0)"), "{}", stdout(&out));

    let gone = dir.path().join("gone/results-pr.json");
    let out = accept(&["--compare", path(&gone), "--settings", path(&valid)]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).contains("cannot read results file"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn br7_4_a_run_where_no_suite_ran_says_so_and_writes_results() {
    let out_dir = TempDir::new("cli-pending");
    let valid = data("valid.toml");
    let out = accept(&[
        "--level",
        "pr",
        "--suite",
        "T1",
        "--suite",
        "T2",
        "--settings",
        path(&valid),
        "--out",
        path(out_dir.path()),
    ]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("T1"), "{text}");
    assert!(text.contains("pending (U3)"), "{text}");
    assert!(text.contains("no suite ran"), "{text}");
    let results = out_dir.path().join("results-pr.json");
    assert!(text.contains(&results.display().to_string()), "{text}");
    let json = std::fs::read_to_string(results).unwrap();
    assert!(
        json.contains(r#""suite":"T1","outcome":"pending""#),
        "{json}"
    );
    assert_eq!(stderr(&out), "");
}

#[test]
fn br3_5_an_unknown_reference_machine_is_reported_and_t13_does_not_count() {
    let out_dir = TempDir::new("cli-machine");
    let valid = data("valid.toml");
    let out = accept(&[
        "--level",
        "release",
        "--suite",
        "T13",
        "--reference-machine",
        "laptop",
        "--settings",
        path(&valid),
        "--out",
        path(out_dir.path()),
    ]);
    // T13 is pending in the fixture, so the release check fails (BR6.3).
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("not a recorded reference machine"),
        "{}",
        stdout(&out)
    );
}
