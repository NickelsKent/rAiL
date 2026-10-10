//! Acceptance harness tests: the scheduler, with fake suites registered
//! only in tests. Levels and seeds (BR3.1, BR4.1-BR4.3), outcomes (BR3.2,
//! BR3.4, BR3.5, BR6.2, BR6.3, BR7.1, BR7.3-BR7.5), no retries (NFR3.6),
//! run directories (NFR5.8) and the time budget (NFR9.18).

mod common;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rail_testkit::TempDir;
use rail_testkit::harness::report::{Outcome, Overall, RunReport, SeedOrigin, SuiteRun};
use rail_testkit::harness::run::{
    Events, RunOptions, SplitMix64, exit_code, fresh_seed, run_suites,
};
use rail_testkit::harness::settings::{
    Build, InputSource, Level, Platform, ReferenceMachine, Settings, Status, SuiteId, Volume,
};
use rail_testkit::repo_root;
use rail_testkit::suites::{Registry, Suite};

use common::{Behaviour, FakeSuite, fake_rail, valid_text};

fn t(n: &str) -> SuiteId {
    SuiteId::parse(n).unwrap()
}

fn count(n: u64) -> Vec<Volume> {
    vec![Volume::Count { count: n }]
}

/// The valid settings with `ids` implemented: pr 2, nightly 3, full 4
/// inputs, recorded seed 42, group "g".
fn settings_with(ids: &[SuiteId]) -> Settings {
    let mut settings = Settings::parse(&valid_text(), "valid.toml").unwrap();
    for id in ids {
        let s = &mut settings.suites[id.index()];
        s.status = Status::Implemented;
        s.volumes.pr = Some(count(2));
        s.volumes.nightly = Some(count(3));
        s.volumes.full = count(4);
        s.seed_policy.uses_seed = true;
        s.seed_policy.pr_seed = Some(42);
        s.nightly_group = Some("g".into());
        s.build = Build::Debug;
        s.cross_platform = false;
        s.inputs = vec![InputSource::Fixed {
            path: "fixtures".into(),
        }];
    }
    settings
}

/// Records events in a shared log (the same one fake suites write to).
struct Log(Arc<Mutex<Vec<String>>>);

impl Events for Log {
    fn seed_drawn(&mut self, suite: SuiteId, seed: u64) {
        self.0.lock().unwrap().push(format!("seed {suite} {seed}"));
    }
}

struct Run {
    report: RunReport,
    out: TempDir,
}

impl Run {
    fn run(&self, id: SuiteId) -> &SuiteRun {
        self.report
            .runs
            .iter()
            .find(|r| r.suite == id)
            .expect("run")
    }
}

fn options(level: Level, out: &TempDir) -> RunOptions {
    let mut rails = BTreeMap::new();
    rails.insert(Build::Debug, fake_rail());
    RunOptions {
        level,
        suites: None,
        group: None,
        seed: None,
        reference_machine: None,
        platform: Platform::Macos,
        budget: None,
        rails,
        root: repo_root(),
        out_dir: out.path().to_path_buf(),
    }
}

fn run_with(
    settings: &Settings,
    suites: Vec<FakeSuite>,
    tweak: impl FnOnce(&mut RunOptions),
) -> (Run, Arc<Mutex<Vec<String>>>) {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut boxed: Vec<Box<dyn Suite>> = vec![Box::new(FakeSuite::passing(SuiteId::S0))];
    for s in suites {
        boxed.push(Box::new(s.with_log(log.clone())));
    }
    let registry = Registry::new(boxed);
    let out = TempDir::new("run-out");
    let mut opts = options(Level::Pr, &out);
    tweak(&mut opts);
    let mut events = Log(log.clone());
    let report = run_suites(settings, &registry, &opts, &mut events);
    (Run { report, out }, log)
}

fn only(id: SuiteId) -> impl FnOnce(&mut RunOptions) {
    move |o| o.suites = Some(vec![id])
}

#[test]
fn br3_1_the_level_selects_volume_and_seed() {
    let settings = settings_with(&[t("T1")]);
    for (level, expected_count) in [(Level::Pr, 2), (Level::Nightly, 3), (Level::Release, 4)] {
        let (run, log) = run_with(&settings, vec![FakeSuite::passing(t("T1"))], |o| {
            o.level = level;
            o.suites = Some(vec![t("T1")]);
        });
        let entries = log.lock().unwrap().clone();
        let generate = entries.iter().find(|e| e.starts_with("generate")).unwrap();
        assert!(
            generate.ends_with(&format!("count {expected_count}")),
            "{generate}"
        );
        let r = run.run(t("T1"));
        assert_eq!(r.volume, count(expected_count));
        if level == Level::Pr {
            assert!(generate.contains("seed 42 "), "{generate}");
            assert_eq!(r.seed_origin, Some(SeedOrigin::Recorded));
        } else {
            assert_eq!(r.seed_origin, Some(SeedOrigin::Fresh));
        }
    }
}

#[test]
fn br4_1_a_fresh_seed_is_reported_before_inputs_are_generated_and_seed_overrides() {
    let settings = settings_with(&[t("T1")]);
    let (run, log) = run_with(&settings, vec![FakeSuite::passing(t("T1"))], |o| {
        o.level = Level::Nightly;
        o.suites = Some(vec![t("T1")]);
    });
    let entries = log.lock().unwrap().clone();
    let seed = run.run(t("T1")).seed.unwrap();
    assert!(
        entries[0].starts_with(&format!("seed T1 {seed}")),
        "{entries:?}"
    );
    assert!(
        entries[1].starts_with(&format!("generate T1 seed {seed} ")),
        "{entries:?}"
    );

    let (run, log) = run_with(&settings, vec![FakeSuite::passing(t("T1"))], |o| {
        o.level = Level::Nightly;
        o.suites = Some(vec![t("T1")]);
        o.seed = Some(99);
    });
    assert_eq!(run.run(t("T1")).seed, Some(99));
    assert_eq!(run.run(t("T1")).seed_origin, Some(SeedOrigin::Override));
    assert!(
        log.lock()
            .unwrap()
            .iter()
            .any(|e| e.starts_with("generate T1 seed 99 "))
    );
}

#[test]
fn br4_2_br7_3_a_failure_records_its_first_failing_input_and_rerun_command() {
    let settings = settings_with(&[t("T1")]);
    let (run, _) = run_with(
        &settings,
        vec![FakeSuite::new(t("T1"), Behaviour::FailAt(1))],
        only(t("T1")),
    );
    let r = run.run(t("T1"));
    assert_eq!(r.outcome, Outcome::Fail);
    let failure = r.first_failure.as_ref().unwrap();
    assert_eq!(failure.input_index, 1);
    assert_eq!(failure.input, "42-1-0");
    assert_eq!(failure.reason, "input 1 is wrong");
    assert_eq!(
        r.rerun_command.as_deref(),
        Some("rail-accept --level pr --suite T1 --seed 42")
    );
    // The failed run's directory is kept for inspection (NFR5.8).
    let kept = r.kept_dir.as_ref().expect("kept directory");
    assert!(kept.join("work").is_dir());
    std::fs::remove_dir_all(kept).unwrap();
    assert_eq!(exit_code(&run.report), 1);
}

#[test]
fn br7_1_an_observation_error_is_error_not_fail() {
    let settings = settings_with(&[t("T1")]);
    let (run, _) = run_with(
        &settings,
        vec![FakeSuite::new(t("T1"), Behaviour::ErrorAt(0))],
        only(t("T1")),
    );
    let r = run.run(t("T1"));
    assert_eq!(r.outcome, Outcome::Error);
    assert!(
        r.first_failure
            .as_ref()
            .unwrap()
            .reason
            .contains("cannot observe")
    );
    if let Some(dir) = &r.kept_dir {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn br5_2_a_timed_out_process_is_an_error() {
    let mut settings = settings_with(&[t("T1")]);
    settings.suites[1].process_deadline_seconds = 1;
    let fake = FakeSuite::new(t("T1"), Behaviour::Rail(vec!["sleep", "30"]));
    let start = Instant::now();
    let (run, _) = run_with(&settings, vec![fake], only(t("T1")));
    let r = run.run(t("T1"));
    assert_eq!(r.outcome, Outcome::Error);
    assert!(
        r.first_failure
            .as_ref()
            .unwrap()
            .reason
            .contains("timed out"),
        "{r:?}"
    );
    assert!(start.elapsed() < Duration::from_secs(20));
    if let Some(dir) = &r.kept_dir {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn br4_3_a_nondeterministic_generator_is_an_error() {
    let settings = settings_with(&[t("T1")]);
    let (run, _) = run_with(
        &settings,
        vec![FakeSuite::new(t("T1"), Behaviour::Nondeterministic)],
        only(t("T1")),
    );
    let r = run.run(t("T1"));
    assert_eq!(r.outcome, Outcome::Error);
    assert!(
        r.reason
            .as_deref()
            .unwrap_or("")
            .contains("not deterministic"),
        "{r:?}"
    );
}

#[test]
fn br4_3_the_shared_generator_is_fixed_across_runs_and_platforms() {
    let mut a = SplitMix64::new(0);
    assert_eq!(a.next_u64(), 0xE220_A839_7B1D_CDAF);
    let first: Vec<u64> = (0..5).map(|_| SplitMix64::new(7).next_u64()).collect();
    assert!(first.windows(2).all(|w| w[0] == w[1]));
    let mut b = SplitMix64::new(7);
    let seq: Vec<u64> = (0..3).map(|_| b.next_u64()).collect();
    assert_ne!(seq[0], seq[1]);
    assert!(fresh_seed().is_ok());
}

#[test]
fn br3_2_br3_4_br6_2_pending_not_run_and_other_platform_outcomes() {
    let mut settings = settings_with(&[t("T1"), t("T3")]);
    settings.suites[1].volumes.pr = Some(vec![Volume::NotRun {
        reason: "too slow for a pull request".into(),
    }]);
    settings.suites[3].platforms = vec![Platform::Linux];
    let suites = vec![FakeSuite::passing(t("T1")), FakeSuite::passing(t("T3"))];
    let (run, log) = run_with(&settings, suites, |_| {});
    assert_eq!(run.run(t("T1")).outcome, Outcome::NotRun);
    assert_eq!(
        run.run(t("T1")).reason.as_deref(),
        Some("too slow for a pull request")
    );
    assert_eq!(run.run(t("T2")).outcome, Outcome::Pending);
    assert_eq!(run.run(t("T2")).reason.as_deref(), Some("pending (U3)"));
    assert_eq!(run.run(t("T3")).outcome, Outcome::SkippedPlatform);
    assert_eq!(run.run(SuiteId::S0).outcome, Outcome::Pass);
    assert!(log.lock().unwrap().is_empty(), "nothing ran but S0");
    let order: Vec<SuiteId> = run.report.runs.iter().map(|r| r.suite).collect();
    assert_eq!(order, SuiteId::ALL.to_vec());
    assert_eq!(run.report.overall, Overall::Pass);
    assert_eq!(exit_code(&run.report), 0);
}

#[test]
fn br6_3_a_release_fails_while_any_suite_is_pending_or_not_run() {
    let settings = settings_with(&[]);
    let (run, _) = run_with(&settings, vec![], |o| o.level = Level::Release);
    assert_eq!(run.run(SuiteId::S0).outcome, Outcome::Pass);
    assert_eq!(run.report.overall, Overall::Fail);
    assert_eq!(exit_code(&run.report), 1);
}

#[test]
fn br3_5_t13_counts_only_on_a_declared_reference_machine() {
    let mut settings = settings_with(&[SuiteId::T13]);
    settings.suites[13].build = Build::Release;
    settings.reference_machines = vec![ReferenceMachine {
        id: "studio".into(),
        platform: Platform::Macos,
        description: "M2 Ultra, 64 GB".into(),
    }];
    let mut rails = BTreeMap::new();
    rails.insert(Build::Release, fake_rail());
    for machine in [None, Some("laptop")] {
        let rails = rails.clone();
        let (run, _) = run_with(
            &settings,
            vec![FakeSuite::passing(SuiteId::T13)],
            move |o| {
                o.suites = Some(vec![SuiteId::T13]);
                o.reference_machine = machine.map(str::to_string);
                o.rails = rails;
            },
        );
        let r = run.run(SuiteId::T13);
        assert_eq!(r.outcome, Outcome::NotRun, "{machine:?}");
        assert_eq!(r.reason.as_deref(), Some("not a reference machine"));
        assert_eq!(run.report.reference_machine, None);
    }
    let (run, _) = run_with(
        &settings,
        vec![FakeSuite::passing(SuiteId::T13)],
        move |o| {
            o.suites = Some(vec![SuiteId::T13]);
            o.reference_machine = Some("studio".into());
            o.rails = rails;
        },
    );
    let r = run.run(SuiteId::T13);
    assert_eq!(r.outcome, Outcome::Pass);
    assert_eq!(r.reference_machine.as_deref(), Some("studio"));
    assert_eq!(run.report.reference_machine.as_deref(), Some("studio"));
}

#[test]
fn br7_5_a_failing_suite_does_not_stop_the_next() {
    let settings = settings_with(&[t("T1"), t("T2")]);
    let suites = vec![
        FakeSuite::new(t("T1"), Behaviour::FailAt(0)),
        FakeSuite::passing(t("T2")),
    ];
    let (run, _) = run_with(&settings, suites, |_| {});
    assert_eq!(run.run(t("T1")).outcome, Outcome::Fail);
    assert_eq!(run.run(t("T2")).outcome, Outcome::Pass);
    if let Some(dir) = &run.run(t("T1")).kept_dir {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn nfr3_6_a_suite_that_fails_once_is_reported_fail_and_run_once() {
    let settings = settings_with(&[t("T1")]);
    let (run, log) = run_with(
        &settings,
        vec![FakeSuite::new(t("T1"), Behaviour::FlakyOnce)],
        only(t("T1")),
    );
    assert_eq!(run.run(t("T1")).outcome, Outcome::Fail);
    let entries = log.lock().unwrap().clone();
    assert_eq!(
        entries.iter().filter(|e| e.starts_with("judge")).count(),
        1,
        "{entries:?}"
    );
    if let Some(dir) = &run.run(t("T1")).kept_dir {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn nfr9_18_a_budget_overrun_stops_the_suite_and_marks_the_rest_not_run() {
    let settings = settings_with(&[t("T1"), t("T2")]);
    let suites = vec![
        FakeSuite::new(t("T1"), Behaviour::Rail(vec!["sleep", "30"])),
        FakeSuite::passing(t("T2")),
    ];
    let start = Instant::now();
    let (run, _) = run_with(&settings, suites, |o| {
        o.budget = Some(Duration::from_secs(1))
    });
    assert!(start.elapsed() < Duration::from_secs(15));
    assert_eq!(run.run(SuiteId::S0).outcome, Outcome::Pass);
    let t1 = run.run(t("T1"));
    assert_eq!(t1.outcome, Outcome::Error);
    assert_eq!(t1.reason.as_deref(), Some("over time budget"));
    let t2 = run.run(t("T2"));
    assert_eq!(t2.outcome, Outcome::NotRun);
    assert_eq!(t2.reason.as_deref(), Some("budget exhausted"));
    assert!(run.report.over_budget);
    assert_eq!(exit_code(&run.report), 1);
    if let Some(dir) = &t1.kept_dir {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

#[test]
fn br7_4_the_report_says_when_no_suite_ran() {
    let settings = settings_with(&[]);
    let (run, _) = run_with(&settings, vec![], only(t("T2")));
    assert!(run.report.no_suite_ran());
    assert_eq!(exit_code(&run.report), 0);
    let (run, _) = run_with(&settings, vec![], only(SuiteId::S0));
    assert!(!run.report.no_suite_ran());
}

#[test]
fn nfr9_17_a_nightly_group_selects_its_suites() {
    let mut settings = settings_with(&[t("T1"), t("T2")]);
    settings.suites[2].nightly_group = Some("other".into());
    let suites = vec![FakeSuite::passing(t("T1")), FakeSuite::passing(t("T2"))];
    let (run, _) = run_with(&settings, suites, |o| {
        o.level = Level::Nightly;
        o.group = Some("g".into());
    });
    let ids: Vec<SuiteId> = run.report.runs.iter().map(|r| r.suite).collect();
    assert_eq!(ids, vec![t("T1")]);
}

#[test]
fn br7_7_a_cross_platform_suite_writes_its_observations() {
    let mut settings = settings_with(&[t("T1")]);
    settings.suites[1].cross_platform = true;
    let (run, _) = run_with(&settings, vec![FakeSuite::passing(t("T1"))], only(t("T1")));
    let r = run.run(t("T1"));
    assert_eq!(r.outcome, Outcome::Pass);
    assert_eq!(
        r.observations_path.as_deref(),
        Some("pr/T1/observations-macos.json")
    );
    let file = run.out.path().join("pr/T1/observations-macos.json");
    let value = rail_json::parse(&std::fs::read(file).unwrap()).unwrap();
    assert_eq!(value.as_array().unwrap().len(), 2);
}
