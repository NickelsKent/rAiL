//! `rail-accept`: runs the acceptance suites (FR57, WF1-WF6).
//!
//! ```text
//! rail-accept --level <pr|nightly|release> [--suite <id>]... [--group <name>]
//!             [--seed <n>] [--rail <build>=<path>]... [--reference-machine <id>]
//!             [--settings <file>] [--root <dir>] [--out <dir>]
//! rail-accept --check-ratchet <base settings> --against <new settings>
//! rail-accept --compare <results file>... [--settings <file>]
//! rail-accept --list-groups [--settings <file>]
//! ```
//!
//! Exit codes: 0 pass, 1 fail, 2 usage, settings or build error (BR7.4).
//! This program is the only part of the harness that prints (BR7.6).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

use rail_testkit::harness::build::build_rail;
use rail_testkit::harness::compare::compare;
use rail_testkit::harness::ratchet::check_ratchet;
use rail_testkit::harness::report::{RunReport, SuiteRun, write_results};
use rail_testkit::harness::run::{Events, RunOptions, exit_code, planned_builds, run_suites};
use rail_testkit::harness::settings::{Build, Level, Platform, SETTINGS_PATH, Settings, SuiteId};
use rail_testkit::suites::Registry;

const USAGE: &str = "usage: rail-accept --level <pr|nightly|release> [--suite <id>]... [--group <name>] [--seed <n>] [--rail <build>=<path>]... [--reference-machine <id>] [--settings <file>] [--root <dir>] [--out <dir>]
       rail-accept --check-ratchet <base settings> --against <new settings>
       rail-accept --compare <results file>... [--settings <file>]
       rail-accept --list-groups [--settings <file>]";

/// A failure that ends the program with exit code 2.
struct Stop(String);

enum Mode {
    Run,
    Ratchet { base: PathBuf, new: PathBuf },
    Compare(Vec<PathBuf>),
    ListGroups,
}

#[derive(Default)]
struct Args {
    level: Option<Level>,
    suites: Vec<SuiteId>,
    group: Option<String>,
    seed: Option<u64>,
    rails: BTreeMap<Build, PathBuf>,
    reference_machine: Option<String>,
    settings: Option<PathBuf>,
    root: Option<PathBuf>,
    out: Option<PathBuf>,
    ratchet_base: Option<PathBuf>,
    ratchet_new: Option<PathBuf>,
    compare: Vec<PathBuf>,
    list_groups: bool,
}

fn usage(message: impl std::fmt::Display) -> Stop {
    Stop(format!("usage error: {message}\n{USAGE}"))
}

fn value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, Stop> {
    args.next()
        .ok_or_else(|| usage(format!("{flag} needs a value")))
}

fn parse_rail(text: &str) -> Result<(Build, PathBuf), Stop> {
    let (build, path) = text
        .split_once('=')
        .ok_or_else(|| usage(format!("--rail takes <build>=<path>, not {text:?}")))?;
    let build = Build::parse(build).ok_or_else(|| {
        usage(format!(
            "--rail: unknown build {build:?} (debug, release, sanitizer)"
        ))
    })?;
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(usage(format!("--rail: {} is not a file", path.display())));
    }
    Ok((build, path))
}

fn parse_args() -> Result<(Mode, Args), Stop> {
    let mut a = Args::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--level" => {
                let v = value(&mut args, "--level")?;
                a.level =
                    Some(Level::parse(&v).ok_or_else(|| {
                        usage(format!("unknown level {v:?} (pr, nightly, release)"))
                    })?);
            }
            "--suite" => {
                let v = value(&mut args, "--suite")?;
                a.suites.push(
                    SuiteId::parse(&v)
                        .ok_or_else(|| usage(format!("unknown suite {v:?} (S0, T1..T14)")))?,
                );
            }
            "--group" => a.group = Some(value(&mut args, "--group")?),
            "--seed" => {
                let v = value(&mut args, "--seed")?;
                a.seed = Some(
                    v.parse()
                        .map_err(|_| usage(format!("--seed takes a number, not {v:?}")))?,
                );
            }
            "--rail" => {
                let (build, path) = parse_rail(&value(&mut args, "--rail")?)?;
                a.rails.insert(build, path);
            }
            "--reference-machine" => {
                a.reference_machine = Some(value(&mut args, "--reference-machine")?)
            }
            "--settings" => a.settings = Some(value(&mut args, "--settings")?.into()),
            "--root" => a.root = Some(value(&mut args, "--root")?.into()),
            "--out" => a.out = Some(value(&mut args, "--out")?.into()),
            "--check-ratchet" => a.ratchet_base = Some(value(&mut args, "--check-ratchet")?.into()),
            "--against" => a.ratchet_new = Some(value(&mut args, "--against")?.into()),
            "--compare" => {
                a.compare.push(value(&mut args, "--compare")?.into());
            }
            "--list-groups" => a.list_groups = true,
            other if !a.compare.is_empty() && !other.starts_with("--") => {
                a.compare.push(other.into())
            }
            other => return Err(usage(format!("unknown argument {other:?}"))),
        }
    }
    let modes = [
        a.level.is_some(),
        a.ratchet_base.is_some(),
        !a.compare.is_empty(),
        a.list_groups,
    ];
    if modes.iter().filter(|m| **m).count() != 1 {
        return Err(usage(
            "give exactly one of --level, --check-ratchet, --compare or --list-groups",
        ));
    }
    let mode = if let Some(base) = a.ratchet_base.clone() {
        let new = a
            .ratchet_new
            .clone()
            .ok_or_else(|| usage("--check-ratchet needs --against <new settings>"))?;
        Mode::Ratchet { base, new }
    } else if !a.compare.is_empty() {
        Mode::Compare(a.compare.clone())
    } else if a.list_groups {
        Mode::ListGroups
    } else {
        Mode::Run
    };
    if matches!(mode, Mode::Run) {
        if a.seed.is_some() && a.suites.len() != 1 {
            return Err(usage("--seed needs exactly one --suite"));
        }
        if a.group.is_some() && a.level != Some(Level::Nightly) {
            return Err(usage("--group is for --level nightly only"));
        }
    }
    Ok((mode, a))
}

/// The workspace this program was built from.
fn default_root() -> PathBuf {
    let built = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    built.canonicalize().unwrap_or(built)
}

fn load_settings(a: &Args, root: &Path) -> Result<Settings, Stop> {
    let (path, label) = match &a.settings {
        Some(path) => (path.clone(), path.display().to_string()),
        None => (root.join(SETTINGS_PATH), SETTINGS_PATH.to_string()),
    };
    Settings::load(&path, &label).map_err(|e| Stop(format!("settings error: {e}")))
}

fn main() -> ExitCode {
    let result = parse_args().and_then(|(mode, args)| {
        let root = args.root.clone().unwrap_or_else(default_root);
        match mode {
            Mode::Run => run(&args, &root),
            Mode::Ratchet { base, new } => ratchet(&base, &new),
            Mode::Compare(files) => compare_files(&args, &root, &files),
            Mode::ListGroups => {
                let settings = load_settings(&args, &root)?;
                let groups = settings
                    .nightly_groups()
                    .iter()
                    .map(|g| rail_json::Value::str(g))
                    .collect();
                println!("{}", rail_json::to_string(&rail_json::Value::Array(groups)));
                Ok(0)
            }
        }
    });
    match result {
        Ok(code) => ExitCode::from(code),
        Err(Stop(message)) => {
            eprintln!("rail-accept: {message}");
            ExitCode::from(2)
        }
    }
}

fn ratchet(base: &Path, new: &Path) -> Result<u8, Stop> {
    let base_text = match std::fs::read_to_string(base) {
        Ok(text) => Some(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(Stop(format!("cannot read {}: {e}", base.display()))),
    };
    let new_text = std::fs::read_to_string(new)
        .map_err(|e| Stop(format!("cannot read {}: {e}", new.display())))?;
    let report = check_ratchet(
        base_text.as_deref(),
        &base.display().to_string(),
        &new_text,
        &new.display().to_string(),
    )
    .map_err(|e| Stop(format!("settings error: {e}")))?;
    for notice in &report.notices {
        println!("notice: {notice}");
    }
    for failure in &report.failures {
        println!("FAIL {failure}");
    }
    println!("ratchet: {}", if report.passed() { "pass" } else { "fail" });
    Ok(u8::from(!report.passed()))
}

fn compare_files(a: &Args, root: &Path, files: &[PathBuf]) -> Result<u8, Stop> {
    let settings = load_settings(a, root)?;
    let report = compare(&settings, files);
    for notice in &report.notices {
        println!("notice: {notice}");
    }
    for failure in &report.failures {
        println!("FAIL {failure}");
    }
    println!("compare: {}", if report.passed() { "pass" } else { "fail" });
    Ok(u8::from(!report.passed()))
}

/// Prints seeds and each suite's per-input lines as the run goes.
struct Printer;

impl Events for Printer {
    fn seed_drawn(&mut self, suite: SuiteId, seed: u64) {
        println!("{suite} seed {seed}");
    }

    fn suite_finished(&mut self, run: &SuiteRun) {
        for (label, verdict) in &run.input_lines {
            match verdict {
                None => println!("PASS {label}"),
                Some(reason) => {
                    println!("FAIL {label}");
                    println!("  {reason}");
                }
            }
        }
    }
}

fn run(a: &Args, root: &Path) -> Result<u8, Stop> {
    let platform =
        Platform::host().ok_or_else(|| Stop("this platform is not macos or linux".into()))?;
    let settings = load_settings(a, root)?;
    let registry = Registry::production();
    settings
        .check_environment(root, &registry)
        .map_err(|e| Stop(format!("settings error: {e}")))?;
    if let Some(group) = &a.group
        && !settings.nightly_groups().contains(group)
    {
        return Err(usage(format!("no nightly group {group:?} in the settings")));
    }
    if let Some(machine) = &a.reference_machine
        && !settings.reference_machines.iter().any(|m| &m.id == machine)
    {
        println!(
            "notice: {machine} is not a recorded reference machine; T13 results will not count"
        );
    }
    let mut opts = RunOptions {
        level: a.level.unwrap_or(Level::Pr),
        suites: (!a.suites.is_empty()).then(|| a.suites.clone()),
        group: a.group.clone(),
        seed: a.seed,
        reference_machine: a.reference_machine.clone(),
        platform,
        budget: None,
        rails: a.rails.clone(),
        root: root.to_path_buf(),
        out_dir: a
            .out
            .clone()
            .unwrap_or_else(|| root.join("target/acceptance")),
    };
    // Builds happen before the budget clock starts (NFR Design Q2).
    for (build, flags) in planned_builds(&settings, &opts) {
        if opts.rails.contains_key(&build) {
            continue;
        }
        println!("building rail ({build})");
        let path = build_rail(root, build, flags.as_deref()).map_err(|e| Stop(e.to_string()))?;
        opts.rails.insert(build, path);
    }
    let report = run_suites(&settings, &registry, &opts, &mut Printer);
    let results = write_results(&report, &opts.out_dir, a.group.as_deref()).map_err(|e| {
        Stop(format!(
            "cannot write results to {}: {e}",
            opts.out_dir.display()
        ))
    })?;
    print_summary(&report);
    println!("results: {}", results.display());
    println!(
        "overall: {}",
        if exit_code(&report) == 0 {
            "pass"
        } else {
            "fail"
        }
    );
    Ok(if exit_code(&report) == 0 { 0 } else { 1 })
}

fn seconds(d: Duration) -> String {
    format!("{:.2}s", d.as_secs_f64())
}

fn print_summary(report: &RunReport) {
    let budget = report.budget.map_or("no time budget".to_string(), |b| {
        format!("budget {} min", b.as_secs() / 60)
    });
    println!(
        "summary: level {} on {} ({budget})",
        report.level, report.platform
    );
    for run in &report.runs {
        let mut line = format!(
            "  {:<4} {:<16}",
            run.suite.to_string(),
            run.outcome.as_str()
        );
        if run.outcome.ran() {
            line.push_str(&seconds(run.elapsed));
            if let Some(b) = report.budget.filter(|b| !b.is_zero()) {
                let share = 100.0 * run.elapsed.as_secs_f64() / b.as_secs_f64();
                line.push_str(&format!("  {share:.1}% of budget"));
            }
        }
        if let Some(reason) = &run.reason {
            line.push_str(&format!("  {reason}"));
        }
        println!("{}", line.trim_end());
        if let Some(failure) = &run.first_failure {
            println!(
                "       first failing input {}: {}",
                failure.input_index, failure.input
            );
            println!("       reason: {}", failure.reason);
        }
        if let Some(command) = &run.rerun_command {
            println!("       rerun: {command}");
        }
        if let Some(dir) = &run.kept_dir {
            println!("       kept run directory: {}", dir.display());
        }
    }
    if report.no_suite_ran() {
        println!("no suite ran");
    }
    if report.over_budget {
        let mut slowest: Vec<&SuiteRun> = report.runs.iter().filter(|r| r.outcome.ran()).collect();
        slowest.sort_by_key(|r| std::cmp::Reverse(r.elapsed));
        let names: Vec<String> = slowest
            .iter()
            .take(3)
            .map(|r| format!("{} ({})", r.suite, seconds(r.elapsed)))
            .collect();
        println!("over time budget; slowest suites: {}", names.join(", "));
    }
}
