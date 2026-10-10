//! Acceptance harness: the scheduler (WF1). It runs the selected suites in
//! order S0, T1..T14, one attempt each (NFR3.6), chooses volume and seed by
//! level (BR3.1, BR4), records one outcome per suite (BR7.1), keeps going
//! after a failure (BR7.5), and enforces the level's time budget with a
//! watchdog (NFR9.18). It never prints: events go to an [`Events`]
//! implementation, which `rail-accept` prints (BR7.6).

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::harness::process::{RunContext, RunDir, RunError, StopSignal};
use crate::harness::report::{
    FailureRecord, ObservationWriter, Outcome, Overall, RunReport, SeedOrigin, SuiteRun,
    observations_rel_path,
};
use crate::harness::settings::{
    Build, Level, Platform, Settings, Status, SuiteConfig, SuiteId, Volume,
};
use crate::suites::{Input, Observation, Registry, Suite};

/// The largest input written inline into a failure record (BR7.3).
pub const INLINE_INPUT_LIMIT: usize = 4096;

/// How often, at most, the budget watchdog looks at the clock.
const WATCHDOG_TICK: Duration = Duration::from_millis(250);

/// One `rail-accept --level` request, after argument parsing.
#[derive(Clone, Debug)]
pub struct RunOptions {
    /// The level.
    pub level: Level,
    /// `--suite` filter; `None` selects every suite.
    pub suites: Option<Vec<SuiteId>>,
    /// `--group`: only implemented suites in this nightly group.
    pub group: Option<String>,
    /// `--seed`, valid with exactly one suite (checked by the program).
    pub seed: Option<u64>,
    /// `--reference-machine`, as given.
    pub reference_machine: Option<String>,
    /// The host platform.
    pub platform: Platform,
    /// A budget override (tests); `None` uses the settings' budget.
    pub budget: Option<Duration>,
    /// The `rail` binary for each build the suites need.
    pub rails: BTreeMap<Build, PathBuf>,
    /// The repository root.
    pub root: PathBuf,
    /// Where results and observation files go.
    pub out_dir: PathBuf,
}

/// What the scheduler reports while it runs. Every method has an empty
/// default.
pub trait Events {
    /// A fresh seed was drawn; called before the suite's inputs are
    /// generated (BR4.1).
    fn seed_drawn(&mut self, _suite: SuiteId, _seed: u64) {}
    /// A suite is about to be considered.
    fn suite_started(&mut self, _suite: SuiteId) {}
    /// A suite has its outcome.
    fn suite_finished(&mut self, _run: &SuiteRun) {}
}

/// Events that go nowhere.
pub struct NoEvents;

impl Events for NoEvents {}

/// SplitMix64: the fixed, documented generator suites share for inputs
/// (BR4.3). Its output depends only on the seed, on every platform.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    /// A generator starting from `seed`.
    pub fn new(seed: u64) -> SplitMix64 {
        SplitMix64 { state: seed }
    }

    /// The next 64-bit value.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A fresh seed: 8 bytes from `/dev/urandom` (BR4.1).
pub fn fresh_seed() -> io::Result<u64> {
    let mut bytes = [0u8; 8];
    File::open("/dev/urandom")?.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

/// The level's time budget from the settings; the release check has none.
pub fn budget_for(settings: &Settings, level: Level) -> Option<Duration> {
    let minutes = match level {
        Level::Pr => settings.budgets.pr_minutes,
        Level::Nightly => settings.budgets.nightly_group_minutes,
        Level::Release => return None,
    };
    Some(Duration::from_secs(minutes.saturating_mul(60)))
}

/// The command that reproduces one suite's run (BR4.2).
pub fn rerun_command(
    level: Level,
    suite: SuiteId,
    seed: Option<u64>,
    machine: Option<&str>,
) -> String {
    let mut command = format!("rail-accept --level {level} --suite {suite}");
    if let Some(seed) = seed {
        command.push_str(&format!(" --seed {seed}"));
    }
    if let Some(machine) = machine {
        command.push_str(&format!(" --reference-machine {machine}"));
    }
    command
}

/// The suites a request selects, in order.
pub fn selected<'a>(settings: &'a Settings, opts: &RunOptions) -> Vec<&'a SuiteConfig> {
    settings
        .suites
        .iter()
        .filter(|s| opts.suites.as_ref().is_none_or(|ids| ids.contains(&s.id())))
        .filter(|s| {
            opts.group.as_ref().is_none_or(|g| {
                s.status == Status::Implemented && s.nightly_group.as_ref() == Some(g)
            })
        })
        .collect()
}

/// The recorded reference machine named by the request, if it names one.
fn declared_machine(settings: &Settings, opts: &RunOptions) -> Option<String> {
    opts.reference_machine
        .as_ref()
        .filter(|id| settings.reference_machines.iter().any(|m| &m.id == *id))
        .cloned()
}

/// Why a selected suite will not run, if it will not (BR3.2, BR3.4, BR3.5,
/// BR6.2).
fn skip_reason(
    settings: &Settings,
    opts: &RunOptions,
    s: &SuiteConfig,
) -> Option<(Outcome, String)> {
    if !s.platforms.contains(&opts.platform) {
        return Some((
            Outcome::SkippedPlatform,
            format!("not listed for {}", opts.platform),
        ));
    }
    if s.status == Status::Pending {
        return Some((Outcome::Pending, format!("pending ({})", s.owning_unit)));
    }
    let volume = s.volumes.for_level(opts.level).unwrap_or(&[]);
    if volume.iter().all(|v| matches!(v, Volume::NotRun { .. })) {
        let reason = volume
            .iter()
            .find_map(|v| match v {
                Volume::NotRun { reason } => Some(reason.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "no volume at this level".to_string());
        return Some((Outcome::NotRun, reason));
    }
    if s.id() == SuiteId::T13 && declared_machine(settings, opts).is_none() {
        return Some((Outcome::NotRun, "not a reference machine".to_string()));
    }
    None
}

/// The builds of `rail` the suites that will actually run need (BR3.6).
pub fn planned_builds(settings: &Settings, opts: &RunOptions) -> BTreeMap<Build, Option<String>> {
    selected(settings, opts)
        .into_iter()
        .filter(|s| skip_reason(settings, opts, s).is_none())
        .map(|s| (s.build, s.sanitizer_rustflags.clone()))
        .collect()
}

/// Whether the report's runs pass overall (BR7.4, BR6.3, NFR9.18).
fn overall(report_level: Level, runs: &[SuiteRun], over_budget: bool) -> Overall {
    let failed = runs
        .iter()
        .any(|r| matches!(r.outcome, Outcome::Fail | Outcome::Error));
    let incomplete = report_level == Level::Release
        && runs
            .iter()
            .any(|r| matches!(r.outcome, Outcome::Pending | Outcome::NotRun));
    if failed || incomplete || over_budget {
        Overall::Fail
    } else {
        Overall::Pass
    }
}

/// The exit code for a finished run: 0 pass, 1 fail (BR7.4). Settings,
/// usage and build errors (exit code 2) never reach a report.
pub fn exit_code(report: &RunReport) -> i32 {
    match report.overall {
        Overall::Pass => 0,
        Overall::Fail => 1,
    }
}

/// Starts the budget watchdog: once `budget` has passed, it stops the run.
fn watchdog(
    budget: Option<Duration>,
    stop: &StopSignal,
) -> (Arc<AtomicBool>, Option<thread::JoinHandle<()>>) {
    let done = Arc::new(AtomicBool::new(false));
    let Some(budget) = budget else {
        return (done, None);
    };
    let (flag, stop) = (done.clone(), stop.clone());
    let start = Instant::now();
    let handle = thread::spawn(move || {
        while !flag.load(Ordering::SeqCst) {
            if start.elapsed() >= budget {
                stop.stop();
                return;
            }
            thread::sleep(WATCHDOG_TICK);
        }
    });
    (done, Some(handle))
}

/// Runs the selected suites and returns the report. Nothing is printed;
/// the caller writes the results file and the terminal summary.
pub fn run_suites(
    settings: &Settings,
    registry: &Registry,
    opts: &RunOptions,
    events: &mut dyn Events,
) -> RunReport {
    let stop = StopSignal::new();
    let budget = opts.budget.or_else(|| budget_for(settings, opts.level));
    let (done, guard) = watchdog(budget, &stop);
    let machine = declared_machine(settings, opts);
    let mut runs = Vec::new();
    for s in selected(settings, opts) {
        events.suite_started(s.id());
        let run = if stop.is_stopped() {
            let mut run = SuiteRun::without_running(
                s.id(),
                s.build,
                Outcome::NotRun,
                "budget exhausted".into(),
            );
            run.volume = level_volume(s, opts.level);
            run
        } else {
            run_one(
                settings,
                registry,
                opts,
                s,
                &stop,
                machine.as_deref(),
                events,
            )
        };
        events.suite_finished(&run);
        runs.push(run);
    }
    done.store(true, Ordering::SeqCst);
    if let Some(guard) = guard {
        let _ = guard.join();
    }
    let over_budget = stop.is_stopped();
    RunReport {
        level: opts.level,
        platform: opts.platform,
        reference_machine: machine,
        overall: overall(opts.level, &runs, over_budget),
        runs,
        over_budget,
        budget,
    }
}

fn level_volume(s: &SuiteConfig, level: Level) -> Vec<Volume> {
    s.volumes
        .for_level(level)
        .map(<[Volume]>::to_vec)
        .unwrap_or_default()
}

/// Decides the seed for a suite run, reporting fresh seeds before use.
fn choose_seed(
    s: &SuiteConfig,
    opts: &RunOptions,
    events: &mut dyn Events,
) -> Result<Option<(u64, SeedOrigin)>, String> {
    if !s.seed_policy.uses_seed {
        return Ok(None);
    }
    if let Some(seed) = opts.seed {
        return Ok(Some((seed, SeedOrigin::Override)));
    }
    if opts.level == Level::Pr {
        return s
            .seed_policy
            .pr_seed
            .map(|seed| Some((seed, SeedOrigin::Recorded)))
            .ok_or_else(|| format!("{} records no pr_seed", s.id()));
    }
    let seed = fresh_seed().map_err(|e| format!("cannot read a fresh seed: {e}"))?;
    events.seed_drawn(s.id(), seed);
    Ok(Some((seed, SeedOrigin::Fresh)))
}

/// The failure record's form of an input (BR7.3).
fn describe_input(index: usize, input: &Input) -> String {
    match std::str::from_utf8(&input.data) {
        Ok(text) if input.data.len() <= INLINE_INPUT_LIMIT => text.to_string(),
        _ => format!(
            "{} (input {index}, {} bytes)",
            input.label,
            input.data.len()
        ),
    }
}

/// What happened while observing the inputs.
enum Observed {
    All(Vec<Observation>),
    Error { index: usize, reason: String },
}

fn observe(suite: &dyn Suite, ctx: &RunContext, inputs: &[Input]) -> Observed {
    let mut observations = Vec::with_capacity(inputs.len());
    for (index, input) in inputs.iter().enumerate() {
        match suite.run(ctx, input) {
            Ok(obs) if obs.timed_out && !suite.expects_timeouts() => {
                return Observed::Error {
                    index,
                    reason: "timed out".to_string(),
                };
            }
            Ok(obs) => observations.push(obs),
            Err(e) => {
                return Observed::Error {
                    index,
                    reason: e.to_string(),
                };
            }
        }
    }
    Observed::All(observations)
}

fn write_observations(
    suite: &dyn Suite,
    opts: &RunOptions,
    inputs: &[Input],
    observations: &[Observation],
) -> io::Result<String> {
    let rel = observations_rel_path(opts.level, suite.id(), opts.platform);
    let mut writer = ObservationWriter::create(&opts.out_dir, &rel)?;
    for (input, obs) in inputs.iter().zip(observations) {
        writer.push(&suite.normalize(input, obs))?;
    }
    writer.finish()?;
    Ok(rel)
}

/// Generates the inputs twice and requires identical results (BR4.3).
fn generate_twice(suite: &dyn Suite, seed: u64, volume: &[Volume]) -> Result<Vec<Input>, String> {
    match (
        suite.generate_inputs(seed, volume),
        suite.generate_inputs(seed, volume),
    ) {
        (Ok(a), Ok(b)) if a == b => Ok(a),
        (Ok(_), Ok(_)) => Err("the input generator is not deterministic (BR4.3)".into()),
        (Err(e), _) | (_, Err(e)) => Err(format!("cannot generate inputs: {e}")),
    }
}

/// Records a fail or error outcome with its first failing input (BR7.3).
fn record_failure(
    run: &mut SuiteRun,
    inputs: &[Input],
    (outcome, index, reason): (Outcome, usize, String),
) {
    run.outcome = outcome;
    if outcome == Outcome::Error {
        run.reason = Some(reason.clone());
    }
    if let Some(input) = inputs.get(index) {
        run.first_failure = Some(FailureRecord {
            input_index: index,
            input: describe_input(index, input),
            reason,
        });
    }
}

fn run_one(
    settings: &Settings,
    registry: &Registry,
    opts: &RunOptions,
    s: &SuiteConfig,
    stop: &StopSignal,
    machine: Option<&str>,
    events: &mut dyn Events,
) -> SuiteRun {
    let start = Instant::now();
    let mut run = SuiteRun::without_running(s.id(), s.build, Outcome::Error, String::new());
    run.volume = level_volume(s, opts.level);
    if let Some((outcome, reason)) = skip_reason(settings, opts, s) {
        run.outcome = outcome;
        run.reason = Some(reason);
        return run;
    }
    if s.id() == SuiteId::T13 {
        run.reference_machine = machine.map(str::to_string);
    }
    let error = |mut run: SuiteRun, reason: String| {
        run.outcome = Outcome::Error;
        run.reason = Some(reason);
        run.rerun_command = Some(rerun_command(opts.level, s.id(), run.seed, machine));
        run.elapsed = start.elapsed();
        run
    };
    let Some(suite) = registry.get(s.id()) else {
        return error(
            run,
            format!("no implementation of {} is registered", s.id()),
        );
    };
    let Some(rail) = opts.rails.get(&s.build) else {
        return error(run, format!("no {} build of rail is available", s.build));
    };
    let seed = match choose_seed(s, opts, events) {
        Ok(seed) => seed,
        Err(reason) => return error(run, reason),
    };
    run.seed = seed.map(|(n, _)| n);
    run.seed_origin = seed.map(|(_, origin)| origin);
    let seed = run.seed.unwrap_or(0);
    let inputs = match generate_twice(suite, seed, &run.volume) {
        Ok(inputs) => inputs,
        Err(reason) => return error(run, reason),
    };
    let dir = match RunDir::create(s.id()) {
        Ok(dir) => dir,
        Err(e) => return error(run, format!("cannot create a run directory: {e}")),
    };
    let deadline = Duration::from_secs(s.process_deadline_seconds);
    let ctx = RunContext::new(&dir, &opts.root, rail, deadline, stop.clone());
    let failure = match observe(suite, &ctx, &inputs) {
        Observed::Error { index, reason } => {
            let reason = if stop.is_stopped() {
                RunError::Stopped.to_string()
            } else {
                reason
            };
            Some((Outcome::Error, index, reason))
        }
        Observed::All(observations) => judge(suite, opts, s, &inputs, &observations, &mut run),
    };
    match failure {
        Some(failure) => {
            record_failure(&mut run, &inputs, failure);
            run.rerun_command = Some(rerun_command(opts.level, s.id(), run.seed, machine));
            run.kept_dir = Some(dir.keep());
        }
        None => {
            run.outcome = Outcome::Pass;
            run.reason = None;
        }
    }
    run.elapsed = start.elapsed();
    run
}

/// Judges the observations; returns the first failure, if any, and fills
/// the per-input lines and the observations file.
fn judge(
    suite: &dyn Suite,
    opts: &RunOptions,
    s: &SuiteConfig,
    inputs: &[Input],
    observations: &[Observation],
    run: &mut SuiteRun,
) -> Option<(Outcome, usize, String)> {
    let mut verdicts = suite.judge(inputs, observations);
    verdicts.resize(inputs.len(), Some("the judge gave no verdict".to_string()));
    if suite.report_each_input() {
        run.input_lines = inputs
            .iter()
            .zip(&verdicts)
            .map(|(input, verdict)| (input.label.clone(), verdict.clone()))
            .collect();
    }
    if s.cross_platform {
        match write_observations(suite, opts, inputs, observations) {
            Ok(rel) => run.observations_path = Some(rel),
            Err(e) => {
                return Some((Outcome::Error, 0, format!("cannot write observations: {e}")));
            }
        }
    }
    verdicts
        .iter()
        .enumerate()
        .find_map(|(i, v)| v.as_ref().map(|reason| (Outcome::Fail, i, reason.clone())))
}
