//! Acceptance harness tests: the results and observation files (BR7.2,
//! BR7.7, NFR3.7) and loading settings from disk.

mod common;

use std::path::PathBuf;
use std::time::Duration;

use rail_testkit::TempDir;
use rail_testkit::harness::report::{
    FailureRecord, ObservationWriter, Outcome, Overall, RunReport, SeedOrigin, SuiteRun,
    observations_rel_path, results_file_name, results_json, write_results,
};
use rail_testkit::harness::settings::{Build, Level, Platform, Settings, SuiteId, Volume};

use common::data;

fn t(n: &str) -> SuiteId {
    SuiteId::parse(n).unwrap()
}

/// A report with a failed S0, a pending T1 and a passing cross-platform T3,
/// carrying terminal-only data (elapsed, kept directory) that must never
/// reach the results file.
fn sample(kept: PathBuf) -> RunReport {
    let mut s0 = SuiteRun::without_running(SuiteId::S0, Build::Debug, Outcome::Fail, String::new());
    s0.reason = None;
    s0.volume = vec![Volume::Count { count: 1 }];
    s0.first_failure = Some(FailureRecord {
        input_index: 2,
        input: "step 3 protocol session".into(),
        reason: "differs from tree-answer.json".into(),
    });
    s0.rerun_command = Some("rail-accept --level pr --suite S0".into());
    s0.elapsed = Duration::from_millis(1234);
    s0.kept_dir = Some(kept);
    let t1 = SuiteRun::without_running(
        t("T1"),
        Build::Debug,
        Outcome::Pending,
        "pending (U3)".into(),
    );
    let mut t3 = SuiteRun::without_running(t("T3"), Build::Release, Outcome::Pass, String::new());
    t3.reason = None;
    t3.volume = vec![Volume::Exhaustive {
        over: "the corpus".into(),
    }];
    t3.seed = Some(u64::MAX);
    t3.seed_origin = Some(SeedOrigin::Fresh);
    t3.observations_path = Some(observations_rel_path(Level::Pr, t("T3"), Platform::Linux));
    RunReport {
        level: Level::Pr,
        platform: Platform::Linux,
        reference_machine: None,
        runs: vec![t3, s0, t1],
        overall: Overall::Fail,
        over_budget: false,
        budget: Some(Duration::from_secs(600)),
    }
}

#[test]
fn the_results_file_has_a_fixed_shape_and_order() {
    let dir = TempDir::new("report-shape");
    let json = results_json(&sample(dir.path().join("kept")));
    assert_eq!(
        json,
        concat!(
            r#"{"schema_version":1,"level":"pr","platform":"linux","overall":"fail","runs":["#,
            r#"{"suite":"S0","outcome":"fail","build":"debug","volume":[{"kind":"count","count":1}],"#,
            r#""first_failure":{"input_index":2,"input":"step 3 protocol session","reason":"differs from tree-answer.json"},"#,
            r#""rerun_command":"rail-accept --level pr --suite S0"},"#,
            r#"{"suite":"T1","outcome":"pending","build":"debug","volume":[],"reason":"pending (U3)"},"#,
            r#"{"suite":"T3","outcome":"pass","build":"release","volume":[{"kind":"exhaustive","over":"the corpus"}],"#,
            r#""seed":18446744073709551615,"seed_origin":"fresh","observations_path":"pr/T3/observations-linux.json"}"#,
            "]}\n"
        )
    );
}

#[test]
fn br7_2_the_same_report_written_from_two_directories_is_byte_identical() {
    let a = TempDir::new("report-a");
    let b = TempDir::new("report-b");
    let mut written = Vec::new();
    for dir in [&a, &b] {
        std::env::set_current_dir(dir.path()).unwrap();
        let report = sample(dir.path().join("kept"));
        let path = write_results(&report, dir.path(), None).unwrap();
        assert_eq!(path, dir.path().join("results-pr.json"));
        written.push(std::fs::read(path).unwrap());
    }
    assert_eq!(written[0], written[1]);
}

#[test]
fn nfr3_7_the_results_file_has_no_paths_timings_hosts_or_users() {
    let dir = TempDir::new("report-clean");
    let json = results_json(&sample(dir.path().join("kept")));
    let tmp = std::env::temp_dir();
    for forbidden in [
        dir.path().display().to_string(),
        tmp.display().to_string(),
        "1234".to_string(),
        "1.234".to_string(),
        "elapsed".to_string(),
        "kept".to_string(),
        "600".to_string(),
    ] {
        assert!(!json.contains(&forbidden), "{forbidden:?} in {json}");
    }
    for var in ["USER", "LOGNAME", "HOSTNAME"] {
        if let Ok(value) = std::env::var(var)
            && value.len() > 2
        {
            assert!(!json.contains(&value), "{var} in {json}");
        }
    }
}

#[test]
fn br7_7_observation_files_are_written_under_level_suite_and_platform() {
    let dir = TempDir::new("report-observations");
    let rel = observations_rel_path(Level::Nightly, t("T1"), Platform::Macos);
    assert_eq!(rel, "nightly/T1/observations-macos.json");
    let mut writer = ObservationWriter::create(dir.path(), &rel).unwrap();
    writer.push(&rail_json::Value::str("first")).unwrap();
    writer.push(&rail_json::Value::int(2)).unwrap();
    writer.finish().unwrap();
    let text = std::fs::read_to_string(dir.path().join(&rel)).unwrap();
    assert_eq!(text, "[\"first\",\n2\n]\n");
    assert!(rail_json::parse(text.as_bytes()).is_ok());

    let empty = observations_rel_path(Level::Pr, t("T7"), Platform::Linux);
    ObservationWriter::create(dir.path(), &empty)
        .unwrap()
        .finish()
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(empty)).unwrap(),
        "[]\n"
    );
}

#[test]
fn results_file_names_follow_level_and_group() {
    assert_eq!(
        results_file_name(Level::Release, None),
        "results-release.json"
    );
    assert_eq!(
        results_file_name(Level::Nightly, Some("smoke")),
        "results-nightly-smoke.json"
    );
}

#[test]
fn settings_load_from_disk_and_report_a_missing_file() {
    let settings = Settings::load(&data("valid.toml"), "valid.toml").unwrap();
    assert_eq!(settings.suites.len(), 15);
    let e = Settings::load(&data("no-such.toml"), "no-such.toml").unwrap_err();
    assert!(e.to_string().starts_with("no-such.toml: "), "{e}");
    assert!(e.to_string().contains("cannot read"), "{e}");
}
