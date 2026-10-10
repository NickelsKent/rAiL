//! Acceptance harness tests: `--compare` across platforms and reference
//! machines (BR7.7, BR3.5, NFR1.4).

mod common;

use std::path::PathBuf;

use rail_testkit::TempDir;
use rail_testkit::harness::compare::compare;
use rail_testkit::harness::report::{
    ObservationWriter, Outcome, Overall, RunReport, SuiteRun, observations_rel_path, write_results,
};
use rail_testkit::harness::settings::{
    Build, Level, Platform, ReferenceMachine, Settings, SuiteId,
};

use common::valid_text;

fn t1() -> SuiteId {
    SuiteId::parse("T1").unwrap()
}

fn settings() -> Settings {
    Settings::parse(&valid_text(), "valid.toml").unwrap()
}

/// Writes a results file for `platform` into its own directory, with T1 in
/// `outcome` and (when it ran) an observations file holding `entries`.
fn results(
    root: &TempDir,
    name: &str,
    level: Level,
    platform: Platform,
    outcome: Outcome,
    entries: &[&str],
) -> PathBuf {
    let dir = root.path().join(name);
    let mut run = SuiteRun::without_running(t1(), Build::Debug, outcome, String::new());
    run.reason = None;
    if outcome.ran() {
        let rel = observations_rel_path(level, t1(), platform);
        let mut writer = ObservationWriter::create(&dir, &rel).unwrap();
        for e in entries {
            writer
                .push(&rail_json::Value::object([
                    ("input", rail_json::Value::str(e.split(':').next().unwrap())),
                    ("stdout", rail_json::Value::str(e)),
                ]))
                .unwrap();
        }
        writer.finish().unwrap();
        run.observations_path = Some(rel);
    }
    let report = RunReport {
        level,
        platform,
        reference_machine: None,
        runs: vec![run],
        overall: Overall::Pass,
        over_budget: false,
        budget: None,
    };
    write_results(&report, &dir, None).unwrap()
}

fn joined(lines: &[String]) -> String {
    lines.join("\n")
}

#[test]
fn identical_observations_pass() {
    let root = TempDir::new("compare-same");
    let entries = ["a:1", "b:2"];
    let mac = results(
        &root,
        "mac",
        Level::Pr,
        Platform::Macos,
        Outcome::Pass,
        &entries,
    );
    let linux = results(
        &root,
        "linux",
        Level::Pr,
        Platform::Linux,
        Outcome::Pass,
        &entries,
    );
    let report = compare(&settings(), &[mac, linux]);
    assert!(report.passed(), "{report:?}");
}

#[test]
fn a_one_byte_difference_fails_naming_the_first_differing_input() {
    let root = TempDir::new("compare-diff");
    let mac = results(
        &root,
        "mac",
        Level::Pr,
        Platform::Macos,
        Outcome::Pass,
        &["a:1", "b:2", "c:3"],
    );
    let linux = results(
        &root,
        "linux",
        Level::Pr,
        Platform::Linux,
        Outcome::Pass,
        &["a:1", "b:3", "c:4"],
    );
    let report = compare(&settings(), &[mac, linux]);
    assert!(!report.passed());
    let text = joined(&report.failures);
    assert!(text.contains("T1"), "{text}");
    assert!(text.contains("input 1 (b)"), "{text}");
    assert!(text.contains("macos") && text.contains("linux"), "{text}");
}

#[test]
fn a_missing_platform_fails() {
    let root = TempDir::new("compare-missing-platform");
    let mac = results(
        &root,
        "mac",
        Level::Pr,
        Platform::Macos,
        Outcome::Pass,
        &["a:1"],
    );
    let report = compare(&settings(), &[mac]);
    assert!(!report.passed());
    assert!(
        joined(&report.failures).contains("no linux results"),
        "{report:?}"
    );
}

#[test]
fn a_suite_that_ran_nowhere_is_skipped_but_one_that_ran_somewhere_must_run_everywhere() {
    let root = TempDir::new("compare-pending");
    let mac = results(
        &root,
        "mac",
        Level::Pr,
        Platform::Macos,
        Outcome::Pending,
        &[],
    );
    let linux = results(
        &root,
        "linux",
        Level::Pr,
        Platform::Linux,
        Outcome::NotRun,
        &[],
    );
    let report = compare(&settings(), &[mac, linux.clone()]);
    assert!(report.passed(), "{report:?}");
    assert!(joined(&report.notices).contains("T1"), "{report:?}");

    let mac = results(
        &root,
        "mac2",
        Level::Pr,
        Platform::Macos,
        Outcome::Pass,
        &["a:1"],
    );
    let report = compare(&settings(), &[mac, linux]);
    assert!(!report.passed());
    assert!(joined(&report.failures).contains("T1"), "{report:?}");
}

#[test]
fn results_of_different_levels_cannot_be_compared() {
    let root = TempDir::new("compare-levels");
    let mac = results(
        &root,
        "mac",
        Level::Pr,
        Platform::Macos,
        Outcome::Pass,
        &["a:1"],
    );
    let linux = results(
        &root,
        "linux",
        Level::Nightly,
        Platform::Linux,
        Outcome::Pass,
        &["a:1"],
    );
    let report = compare(&settings(), &[mac, linux]);
    assert!(!report.passed());
    assert!(joined(&report.failures).contains("level"), "{report:?}");
}

#[test]
fn a_missing_results_file_fails_with_a_clear_message() {
    let root = TempDir::new("compare-no-file");
    let missing = root.path().join("nowhere/results-pr.json");
    let report = compare(&settings(), &[missing]);
    assert!(!report.passed());
    let text = joined(&report.failures);
    assert!(text.contains("cannot read results file"), "{text}");
    assert!(text.contains("nowhere/results-pr.json"), "{text}");
}

/// A release results file from one reference machine with T13 `outcome`.
fn t13_results(root: &TempDir, machine: &str, platform: Platform, outcome: Outcome) -> PathBuf {
    let dir = root.path().join(machine);
    let mut run = SuiteRun::without_running(SuiteId::T13, Build::Release, outcome, String::new());
    run.reason = None;
    run.reference_machine = Some(machine.to_string());
    let report = RunReport {
        level: Level::Release,
        platform,
        reference_machine: Some(machine.to_string()),
        runs: vec![run],
        overall: Overall::Pass,
        over_budget: false,
        budget: None,
    };
    write_results(&report, &dir, None).unwrap()
}

#[test]
fn nfr1_4_t13_needs_a_passing_result_from_every_reference_machine() {
    let mut settings = settings();
    settings.reference_machines = vec![
        ReferenceMachine {
            id: "studio".into(),
            platform: Platform::Macos,
            description: "M2 Ultra, 64 GB".into(),
        },
        ReferenceMachine {
            id: "tower".into(),
            platform: Platform::Linux,
            description: "Ryzen 9, 64 GB".into(),
        },
    ];
    let root = TempDir::new("compare-t13");
    let studio = t13_results(&root, "studio", Platform::Macos, Outcome::Pass);
    let report = compare(&settings, std::slice::from_ref(&studio));
    assert!(!report.passed());
    assert!(joined(&report.failures).contains("tower"), "{report:?}");

    let tower = t13_results(&root, "tower", Platform::Linux, Outcome::Fail);
    let report = compare(&settings, &[studio.clone(), tower]);
    assert!(!report.passed());

    let tower = t13_results(&root, "tower-ok", Platform::Linux, Outcome::Pass);
    let text = std::fs::read_to_string(&tower)
        .unwrap()
        .replace("tower-ok", "tower");
    std::fs::write(&tower, text).unwrap();
    let report = compare(&settings, &[studio, tower]);
    assert!(report.passed(), "{report:?}");
}
