//! Running versions on fixed inputs and judging them (WF3 steps 6 to 8).

use crate::budget::Budget;
use crate::expected::{Expected, Mode, Verdict, expected_from_outcome, judge};
use crate::index::{Comparison, IndexEntry, InputMode};
use crate::model::Lang;
use crate::proc::run_command;
use crate::sandbox::{Sandbox, SandboxSpec};
use crate::setup::{SetupOp, apply_setup, parse_setup, restore_permissions};
use crate::tree::tree_listing;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// What the runs share.
pub struct Context {
    pub sandbox: Sandbox,
    pub mode: Mode,
    /// The per-run deadline (60 seconds in production).
    pub deadline: Duration,
    pub budget: Budget,
    pub is_root: bool,
    /// The core count used as the second thread count (at least 2).
    pub cores: usize,
    /// Where run directories are made.
    pub scratch: PathBuf,
    /// The process-pipeline helper, when built.
    pub helper: Option<PathBuf>,
}

/// One built version of one program.
#[derive(Clone, Debug)]
pub struct Job {
    pub entry: IndexEntry,
    pub lang: Lang,
    pub program_dir: PathBuf,
    pub exe: PathBuf,
    pub build_dir: PathBuf,
}

/// The outcome for one program and language.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    Pass,
    Blessed,
    Fail(String),
    Skipped(String),
    NotRun(String),
}

#[derive(Clone, Debug)]
pub struct VersionReport {
    pub program: String,
    pub class: String,
    pub lang: Lang,
    pub status: Status,
    /// Fixed inputs that ran.
    pub inputs_run: usize,
    pub elapsed: Duration,
}

impl VersionReport {
    /// The report line for this version.
    pub fn line(&self) -> String {
        let name = format!("{}/{} {}", self.class, self.program, self.lang);
        match &self.status {
            Status::Pass => format!("PASS {name} ({} inputs)", self.inputs_run),
            Status::Blessed => format!("BLESSED {name} ({} inputs)", self.inputs_run),
            Status::Fail(m) => format!("FAIL {name}: {m}"),
            Status::Skipped(m) => format!("SKIP {name}: {m}"),
            Status::NotRun(m) => format!("NOTRUN {name}: {m}"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub versions: Vec<VersionReport>,
    pub over_budget: bool,
}

impl Report {
    /// Whether the check failed: any failure, or the budget ran out.
    pub fn failed(&self) -> bool {
        self.over_budget
            || self
                .versions
                .iter()
                .any(|v| matches!(v.status, Status::Fail(_) | Status::NotRun(_)))
    }

    /// One line per program and language: `PASS`, `BLESSED`, `FAIL`, `SKIP`
    /// or `NOTRUN`, then `<class>/<program> <language>`.
    pub fn lines(&self) -> Vec<String> {
        self.versions.iter().map(VersionReport::line).collect()
    }

    /// The slowest `n` versions, slowest first.
    pub fn slowest(&self, n: usize) -> Vec<&VersionReport> {
        let mut all: Vec<&VersionReport> = self.versions.iter().collect();
        all.sort_by_key(|v| std::cmp::Reverse(v.elapsed));
        all.truncate(n);
        all
    }
}

/// Why one run could not be judged.
struct RunFailure {
    message: String,
    /// The time budget ran out during the run.
    budget: bool,
}

impl RunFailure {
    fn new(message: impl Into<String>) -> RunFailure {
        RunFailure {
            message: message.into(),
            budget: false,
        }
    }
}

struct VersionResult {
    status: Status,
    inputs_run: usize,
    budget_hit: bool,
}

/// Runs every job on every fixed input and judges the results. Once the time
/// budget is spent, the running program is stopped and every remaining job
/// is recorded as not run.
pub fn run_jobs(ctx: &Context, jobs: &[Job]) -> Report {
    let mut report = Report::default();
    for job in jobs {
        let mut version = VersionReport {
            program: job.entry.program_id.clone(),
            class: job.entry.class.name().to_string(),
            lang: job.lang,
            status: Status::Pass,
            inputs_run: 0,
            elapsed: Duration::ZERO,
        };
        if ctx.budget.exhausted() {
            report.over_budget = true;
            version.status = Status::NotRun("over time budget".to_string());
            report.versions.push(version);
            continue;
        }
        let started = Instant::now();
        let result = check_version(ctx, job);
        version.elapsed = started.elapsed();
        version.status = result.status;
        version.inputs_run = result.inputs_run;
        if result.budget_hit {
            report.over_budget = true;
        }
        report.versions.push(version);
    }
    report
}

fn check_version(ctx: &Context, job: &Job) -> VersionResult {
    let entry = &job.entry;
    let mut inputs_run = 0;
    let mut skipped = Vec::new();
    let mut blessed = false;
    let fail = |message: String, inputs_run: usize, budget_hit: bool| VersionResult {
        status: Status::Fail(message),
        inputs_run,
        budget_hit,
    };
    for name in &entry.fixed_inputs {
        let setup = match load_setup(&job.program_dir, name) {
            Ok(ops) => ops,
            Err(message) => return fail(format!("input `{name}`: {message}"), inputs_run, false),
        };
        if !setup.is_empty() && ctx.is_root {
            skipped.push(name.clone());
            continue;
        }
        match run_input(ctx, job, name, &setup) {
            Ok(Verdict::Pass) => inputs_run += 1,
            Ok(Verdict::Blessed) => {
                inputs_run += 1;
                blessed = true;
            }
            Ok(Verdict::Fail(message)) => {
                return fail(format!("input `{name}`: {message}"), inputs_run + 1, false);
            }
            Err(f) => {
                let message = if f.budget {
                    f.message
                } else {
                    format!("input `{name}`: {}", f.message)
                };
                return fail(message, inputs_run + 1, f.budget);
            }
        }
    }
    let status = if !skipped.is_empty() {
        Status::Skipped(format!(
            "running as root: the permission cases of {} were not run",
            skipped
                .iter()
                .map(|n| format!("`{n}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    } else if blessed {
        Status::Blessed
    } else {
        Status::Pass
    };
    VersionResult {
        status,
        inputs_run,
        budget_hit: false,
    }
}

fn load_setup(program_dir: &Path, input: &str) -> Result<Vec<SetupOp>, String> {
    let path = program_dir.join(format!("inputs/fixed/{input}.setup"));
    match fs::read_to_string(&path) {
        Ok(text) => parse_setup(&text).map_err(|e| format!("{input}.setup: {e}")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!("{input}.setup: {e}")),
    }
}

/// Runs one fixed input (at two thread counts for data-parallel programs)
/// and judges it.
fn run_input(
    ctx: &Context,
    job: &Job,
    input: &str,
    setup: &[SetupOp],
) -> Result<Verdict, RunFailure> {
    let entry = &job.entry;
    let thread_counts: Vec<Option<usize>> = if entry.threads_flag {
        vec![Some(1), Some(ctx.cores.max(2))]
    } else {
        vec![None]
    };
    let mut first: Option<Expected> = None;
    for threads in thread_counts {
        let got = run_once(ctx, job, input, setup, threads)?;
        match &first {
            None => first = Some(got),
            Some(prev) if *prev != got => {
                return Err(RunFailure::new(format!(
                    "output differs between 1 and {} threads (program {})",
                    threads.unwrap_or(1),
                    entry.program_id
                )));
            }
            Some(_) => {}
        }
    }
    let actual = first.expect("at least one thread count");
    Ok(judge(ctx.mode, job.lang, &job.program_dir, input, &actual))
}

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Removes a run directory, restoring permissions first.
struct RunArea(PathBuf);

impl Drop for RunArea {
    fn drop(&mut self) {
        restore_permissions(&self.0);
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn run_once(
    ctx: &Context,
    job: &Job,
    input: &str,
    setup: &[SetupOp],
    threads: Option<usize>,
) -> Result<Expected, RunFailure> {
    let entry = &job.entry;
    let tool = |what: &str, e: io::Error| RunFailure::new(format!("{what}: {e}"));
    let id = RUN_COUNTER.fetch_add(1, Ordering::SeqCst);
    let area = RunArea(ctx.scratch.join(format!("run-{}-{id}", std::process::id())));
    let (run_dir, canary) = (area.0.join("run"), area.0.join("canary"));
    for dir in [&run_dir, &canary] {
        fs::create_dir_all(dir).map_err(|e| tool("cannot create the run directory", e))?;
    }
    fs::write(canary.join("marker.txt"), b"canary\n")
        .map_err(|e| tool("cannot write the canary", e))?;
    copy_dir(&job.program_dir.join("inputs/fixed").join(input), &run_dir)
        .map_err(|e| tool("cannot copy the fixed input", e))?;
    let stdin = match (entry.input_mode, &entry.stdin_from) {
        (InputMode::Stdin, Some(name)) => {
            fs::read(run_dir.join(name)).map_err(|e| tool("cannot read the stdin file", e))?
        }
        _ => Vec::new(),
    };
    apply_setup(&run_dir, setup).map_err(|e| tool("cannot create the run-time cases", e))?;
    let canary_before = tree_listing(&canary).map_err(|e| tool("cannot list the canary", e))?;

    let helper = ctx.helper.as_deref();
    if entry.args.iter().any(|a| a.contains("{helper}")) && helper.is_none() {
        return Err(RunFailure::new(
            "this program needs the helper, which was not built",
        ));
    }
    let spec = SandboxSpec {
        exe: job.exe.clone(),
        args: expand_args(&entry.args, threads, helper),
        run_dir: run_dir.clone(),
        build_dir: job.build_dir.clone(),
        read_dirs: helper
            .and_then(Path::parent)
            .map(|p| vec![p.to_path_buf()])
            .unwrap_or_default(),
        env: Vec::new(),
    };
    let deadline = ctx
        .budget
        .remaining()
        .map_or(ctx.deadline, |left| left.min(ctx.deadline));
    let outcome = run_command(ctx.sandbox.command(&spec), &stdin, deadline)
        .map_err(|e| tool("cannot start the program", e))?;
    restore_permissions(&run_dir);

    let canary_after = tree_listing(&canary).map_err(|e| tool("cannot list the canary", e))?;
    if canary_after != canary_before {
        return Err(RunFailure::new(
            "the canary directory changed: the program wrote outside its run directory",
        ));
    }
    if outcome.timed_out {
        return Err(if ctx.budget.exhausted() {
            RunFailure {
                message: "over time budget".to_string(),
                budget: true,
            }
        } else {
            RunFailure::new(format!(
                "the program exceeded the {}-second deadline and was stopped",
                ctx.deadline.as_secs().max(1)
            ))
        });
    }
    let tree = match entry.comparison {
        Comparison::StdoutAndTree => {
            Some(tree_listing(&run_dir).map_err(|e| tool("cannot list the run directory", e))?)
        }
        Comparison::StdoutExact => None,
    };
    expected_from_outcome(&outcome, tree).map_err(RunFailure::new)
}

/// Replaces `{input:<name>}` with the name (a path inside the run
/// directory), `{threads}` with the thread count and `{helper}` with the
/// helper's path.
pub fn expand_args(
    template: &[String],
    threads: Option<usize>,
    helper: Option<&Path>,
) -> Vec<OsString> {
    template
        .iter()
        .map(|arg| {
            let mut out = String::new();
            let mut rest = arg.as_str();
            while let Some(start) = rest.find('{') {
                out.push_str(&rest[..start]);
                let Some(len) = rest[start..].find('}') else {
                    break;
                };
                let name = &rest[start + 1..start + len];
                match name {
                    "threads" => out.push_str(&threads.unwrap_or(1).to_string()),
                    "helper" => out.push_str(
                        &helper
                            .map(|h| h.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    ),
                    _ => out.push_str(name.strip_prefix("input:").unwrap_or(name)),
                }
                rest = &rest[start + len + 1..];
            }
            out.push_str(rest);
            OsString::from(out)
        })
        .collect()
}

/// Copies the contents of `from` into the existing directory `to`.
fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            fs::create_dir_all(&target)?;
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
