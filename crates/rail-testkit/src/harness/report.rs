//! Acceptance harness: the results model (`SuiteRun`, `RunReport`) and its
//! deterministic files (BR7.1-BR7.3, BR7.7, NFR3.7).

use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use rail_json::{Number, Value};

use crate::harness::settings::{Build, Level, Platform, SuiteId, Volume};

/// How one selected suite ended (BR7.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Outcome {
    /// The pass criterion holds.
    Pass,
    /// The pass criterion does not hold.
    Fail,
    /// The harness or `rail` could not produce an observation.
    Error,
    /// Not implemented yet (BR6.2).
    Pending,
    /// Not run at this level (BR3.4, BR3.5) or the budget ran out.
    NotRun,
    /// The host platform is not listed (BR3.2).
    SkippedPlatform,
}

impl Outcome {
    /// The name written in results files.
    pub fn as_str(self) -> &'static str {
        match self {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Error => "error",
            Outcome::Pending => "pending",
            Outcome::NotRun => "not_run",
            Outcome::SkippedPlatform => "skipped_platform",
        }
    }

    /// Parses a written name.
    pub fn parse(text: &str) -> Option<Outcome> {
        [
            Outcome::Pass,
            Outcome::Fail,
            Outcome::Error,
            Outcome::Pending,
            Outcome::NotRun,
            Outcome::SkippedPlatform,
        ]
        .into_iter()
        .find(|o| o.as_str() == text)
    }

    /// Whether the suite actually ran (pass, fail or error).
    pub fn ran(self) -> bool {
        matches!(self, Outcome::Pass | Outcome::Fail | Outcome::Error)
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Where a run's seed came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeedOrigin {
    /// The recorded `pr_seed`.
    Recorded,
    /// Drawn from the operating system and printed (BR4.1).
    Fresh,
    /// Given with `--seed`.
    Override,
}

impl SeedOrigin {
    /// The name written in results files.
    pub fn as_str(self) -> &'static str {
        match self {
            SeedOrigin::Recorded => "recorded",
            SeedOrigin::Fresh => "fresh",
            SeedOrigin::Override => "override",
        }
    }
}

/// The first failing input of a failed or errored run (BR7.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FailureRecord {
    /// Position in generation order.
    pub input_index: usize,
    /// The input itself (at most 4 KiB of UTF-8), or its label and index.
    pub input: String,
    /// The judge's reason, or the harness error.
    pub reason: String,
}

/// One suite at one level on one platform.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuiteRun {
    /// The suite.
    pub suite: SuiteId,
    /// The build of `rail` it ran (or would run) against.
    pub build: Build,
    /// The recorded volume entries for the level (empty while pending).
    pub volume: Vec<Volume>,
    /// The seed, when the suite uses one and ran.
    pub seed: Option<u64>,
    /// Where the seed came from.
    pub seed_origin: Option<SeedOrigin>,
    /// The declared reference machine, for T13 runs that count.
    pub reference_machine: Option<String>,
    /// The observations file, relative to the results file's directory.
    pub observations_path: Option<String>,
    /// How it ended.
    pub outcome: Outcome,
    /// Why, for pending, not_run and error outcomes.
    pub reason: Option<String>,
    /// The first failing input (fail and error).
    pub first_failure: Option<FailureRecord>,
    /// The command that reproduces the run (fail and error, BR4.2).
    pub rerun_command: Option<String>,
    /// Terminal only: elapsed time (never written to the results file).
    pub elapsed: Duration,
    /// Terminal only: the kept run directory of a failed run (NFR5.8).
    pub kept_dir: Option<PathBuf>,
    /// Terminal only: one line per input for suites that report each input.
    pub input_lines: Vec<(String, Option<String>)>,
}

impl SuiteRun {
    /// A run that ended without running anything.
    pub fn without_running(
        suite: SuiteId,
        build: Build,
        outcome: Outcome,
        reason: String,
    ) -> SuiteRun {
        SuiteRun {
            suite,
            build,
            volume: Vec::new(),
            seed: None,
            seed_origin: None,
            reference_machine: None,
            observations_path: None,
            outcome,
            reason: Some(reason),
            first_failure: None,
            rerun_command: None,
            elapsed: Duration::ZERO,
            kept_dir: None,
            input_lines: Vec::new(),
        }
    }
}

/// The overall result of one invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overall {
    /// Exit code 0.
    Pass,
    /// Exit code 1.
    Fail,
}

/// The result of one `rail-accept --level` run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunReport {
    /// The level.
    pub level: Level,
    /// The host platform.
    pub platform: Platform,
    /// The declared reference machine, when it names a recorded one.
    pub reference_machine: Option<String>,
    /// One run per selected suite, ordered S0, T1..T14.
    pub runs: Vec<SuiteRun>,
    /// Pass or fail (BR7.4, BR6.3).
    pub overall: Overall,
    /// Terminal only: whether the time budget ran out (NFR9.18).
    pub over_budget: bool,
    /// Terminal only: the level's budget, when it has one.
    pub budget: Option<Duration>,
}

impl RunReport {
    /// Whether no selected suite actually ran (BR7.4 notice "no suite ran").
    pub fn no_suite_ran(&self) -> bool {
        !self.runs.iter().any(|r| r.outcome.ran())
    }
}

/// The results file name for `level` (and a nightly group).
pub fn results_file_name(level: Level, group: Option<&str>) -> String {
    match group {
        Some(group) => format!("results-{level}-{group}.json"),
        None => format!("results-{level}.json"),
    }
}

/// Where a suite's observations file goes, relative to the directory that
/// holds the results file (BR7.7).
pub fn observations_rel_path(level: Level, suite: SuiteId, platform: Platform) -> String {
    format!("{level}/{suite}/observations-{platform}.json")
}

fn number(n: u64) -> Value {
    Value::Number(Number::from_lexeme(&n.to_string()).unwrap_or_else(|| Number::from_i64(0)))
}

/// A volume entry as JSON.
pub fn volume_json(volume: &Volume) -> Value {
    match volume {
        Volume::Count { count } => {
            Value::object([("kind", Value::str("count")), ("count", number(*count))])
        }
        Volume::Exhaustive { over } => Value::object([
            ("kind", Value::str("exhaustive")),
            ("over", Value::str(over)),
        ]),
        Volume::NotRun { reason } => Value::object([
            ("kind", Value::str("not_run")),
            ("reason", Value::str(reason)),
        ]),
    }
}

fn run_json(run: &SuiteRun) -> Value {
    let mut fields = vec![
        ("suite", Value::str(&run.suite.to_string())),
        ("outcome", Value::str(run.outcome.as_str())),
        ("build", Value::str(run.build.as_str())),
        (
            "volume",
            Value::Array(run.volume.iter().map(volume_json).collect()),
        ),
    ];
    if let Some(seed) = run.seed {
        fields.push(("seed", number(seed)));
    }
    if let Some(origin) = run.seed_origin {
        fields.push(("seed_origin", Value::str(origin.as_str())));
    }
    if let Some(machine) = &run.reference_machine {
        fields.push(("reference_machine", Value::str(machine)));
    }
    if let Some(path) = &run.observations_path {
        fields.push(("observations_path", Value::str(path)));
    }
    if let Some(reason) = &run.reason {
        fields.push(("reason", Value::str(reason)));
    }
    if let Some(failure) = &run.first_failure {
        let index = i64::try_from(failure.input_index).unwrap_or(i64::MAX);
        fields.push((
            "first_failure",
            Value::object([
                ("input_index", Value::int(index)),
                ("input", Value::str(&failure.input)),
                ("reason", Value::str(&failure.reason)),
            ]),
        ));
    }
    if let Some(command) = &run.rerun_command {
        fields.push(("rerun_command", Value::str(command)));
    }
    Value::object(fields)
}

/// The deterministic results JSON (BR7.2, NFR3.7): fields in a fixed order,
/// runs ordered S0, T1..T14, no whitespace, a final newline, and none of
/// the terminal-only data (elapsed times, run directories, budgets).
pub fn results_json(report: &RunReport) -> String {
    let mut runs: Vec<&SuiteRun> = report.runs.iter().collect();
    runs.sort_by_key(|r| r.suite);
    let mut fields = vec![
        ("schema_version", Value::int(1)),
        ("level", Value::str(report.level.as_str())),
        ("platform", Value::str(report.platform.as_str())),
    ];
    if let Some(machine) = &report.reference_machine {
        fields.push(("reference_machine", Value::str(machine)));
    }
    let overall = match report.overall {
        Overall::Pass => "pass",
        Overall::Fail => "fail",
    };
    fields.push(("overall", Value::str(overall)));
    fields.push((
        "runs",
        Value::Array(runs.into_iter().map(run_json).collect()),
    ));
    let mut text = rail_json::to_string(&Value::object(fields));
    text.push('\n');
    text
}

/// Writes the results file into `dir` (created if needed) and returns its
/// path.
pub fn write_results(report: &RunReport, dir: &Path, group: Option<&str>) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let path = dir.join(results_file_name(report.level, group));
    fs::write(&path, results_json(report))?;
    Ok(path)
}

/// Writes a JSON array of normalized observations one entry at a time, so
/// large full-volume files never sit in memory whole (BR7.7).
pub struct ObservationWriter {
    out: BufWriter<File>,
    count: usize,
}

impl ObservationWriter {
    /// Creates `dir/rel` (and its parent directories).
    pub fn create(dir: &Path, rel: &str) -> io::Result<ObservationWriter> {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(ObservationWriter {
            out: BufWriter::new(File::create(path)?),
            count: 0,
        })
    }

    /// Appends one entry.
    pub fn push(&mut self, entry: &Value) -> io::Result<()> {
        let separator: &[u8] = if self.count == 0 { b"[" } else { b",\n" };
        self.out.write_all(separator)?;
        self.out.write_all(rail_json::to_string(entry).as_bytes())?;
        self.count += 1;
        Ok(())
    }

    /// Closes the array and flushes the file.
    pub fn finish(mut self) -> io::Result<()> {
        let end: &[u8] = if self.count == 0 { b"[]\n" } else { b"\n]\n" };
        self.out.write_all(end)?;
        self.out.flush()
    }
}
