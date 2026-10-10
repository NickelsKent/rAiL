//! The command line of the corpus check (WF3).

use crate::budget::Budget;
use crate::build::build_version;
use crate::check::{Context, Job, Report, Status, VersionReport, run_jobs};
use crate::expected::Mode;
use crate::finding::Finding;
use crate::index::{IndexEntry, check_composition, check_layout, check_rail_ids, parse_index};
use crate::inputs::{LaunchSpec, check_fixed_inputs, check_generator_determinism};
use crate::manifest::check_manifests;
use crate::model::{Class, Composition, Lang};
use crate::pins::{Pins, detect_installed, parse_pins, verify_installed};
use crate::sandbox::{Platform, SandboxSpec, default_search_path, prepare_sandbox};
use crate::setup::running_as_root;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// What the command line asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub corpus_dir: PathBuf,
    pub class: Option<Class>,
    pub program: Option<String>,
    pub langs: Vec<Lang>,
    pub bless: bool,
    /// Require all four classes with ten programs each.
    pub complete: bool,
    /// The pull-request budget in seconds; 0 is unlimited (nightly).
    pub budget_seconds: u64,
    /// The per-program-run deadline in seconds.
    pub deadline_seconds: u64,
    pub sandbox_profile: Option<PathBuf>,
    pub work_dir: Option<PathBuf>,
}

/// The usage text.
pub const USAGE: &str = "\
usage: corpus-check [options]

Builds and runs the Rust, Go and OCaml versions of the corpus programs on
their fixed inputs, in an OS sandbox, and compares them with expected/.

options:
  --class <class>          only this class (pure-transform, parsing-text,
                           io-services, data-parallel)
  --program <id>           only this program
  --lang <list>            comma-separated subset of rust,go,ocaml (default: all)
  --bless                  write expected/ from the Rust versions instead of
                           comparing (also RAIL_BLESS=1)
  --complete               require all four classes with ten programs each
  --budget-seconds <n>     time budget for the whole check (default 1200;
                           0 is unlimited)
  --deadline-seconds <n>   deadline for one program run (default 60)
  --corpus-dir <dir>       the corpus directory (default: corpus)
  --sandbox-profile <file> the macOS sandbox profile
                           (default: <corpus-dir>/tools/sandbox/macos.sb)
  --work-dir <dir>         where builds and run directories go
  --help                   print this text

exit codes: 0 everything passed, 1 the corpus has problems, 2 the check
could not run (bad usage, toolchain or sandbox problem).
";

/// Parses the arguments after the program name. `env_bless` is whether
/// `RAIL_BLESS=1` is set.
pub fn parse_args(args: &[String], env_bless: bool) -> Result<Options, String> {
    let mut o = Options {
        corpus_dir: PathBuf::from("corpus"),
        class: None,
        program: None,
        langs: Lang::RUNNABLE.to_vec(),
        bless: env_bless,
        complete: false,
        budget_seconds: 20 * 60,
        deadline_seconds: 60,
        sandbox_profile: None,
        work_dir: None,
    };
    let mut it = args.iter();
    while let Some(flag) = it.next() {
        let mut value = || -> Result<&String, String> {
            it.next().ok_or_else(|| format!("{flag} needs a value"))
        };
        match flag.as_str() {
            "--class" => {
                let v = value()?;
                o.class = Some(Class::parse(v).ok_or_else(|| format!("unknown class `{v}`"))?);
            }
            "--program" => o.program = Some(value()?.clone()),
            "--lang" => {
                let v = value()?;
                let mut langs = Vec::new();
                for name in v.split(',') {
                    match Lang::parse(name) {
                        Some(l) if Lang::RUNNABLE.contains(&l) => langs.push(l),
                        _ => return Err(format!("`{name}` is not one of rust, go, ocaml")),
                    }
                }
                langs.sort();
                langs.dedup();
                o.langs = langs;
            }
            "--bless" => o.bless = true,
            "--complete" => o.complete = true,
            "--budget-seconds" => {
                let v = value()?;
                o.budget_seconds = v
                    .parse()
                    .map_err(|_| format!("--budget-seconds needs a number, not `{v}`"))?;
            }
            "--deadline-seconds" => {
                let v = value()?;
                o.deadline_seconds =
                    v.parse::<u64>().ok().filter(|n| *n >= 1).ok_or_else(|| {
                        format!("--deadline-seconds needs a number of at least 1, not `{v}`")
                    })?;
            }
            "--corpus-dir" => o.corpus_dir = PathBuf::from(value()?),
            "--sandbox-profile" => o.sandbox_profile = Some(PathBuf::from(value()?)),
            "--work-dir" => o.work_dir = Some(PathBuf::from(value()?)),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    Ok(o)
}

/// Runs the check, writing the report to `out` and tool errors to `err`.
/// Returns the exit code: 0 all passed, 1 the corpus has problems, 2 the
/// check could not run.
pub fn run(options: &Options, out: &mut dyn Write, err: &mut dyn Write) -> i32 {
    match run_check(options, out) {
        Ok(code) => code,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            2
        }
    }
}

fn say(out: &mut dyn Write, line: &str) {
    let _ = writeln!(out, "{line}");
    let _ = out.flush();
}

/// Reads and parses the index and the pins; problems are findings.
fn load(corpus: &Path) -> Result<Result<(crate::index::Index, Pins), Vec<Finding>>, String> {
    let read = |name: &str| {
        fs::read_to_string(corpus.join(name)).map_err(|e| format!("cannot read {name}: {e}"))
    };
    let pins = parse_pins("toolchains.toml", &read("toolchains.toml")?);
    let index = parse_index("index.toml", &read("index.toml")?);
    Ok(match (index, pins) {
        (Ok(index), Ok(pins)) => Ok((index, pins)),
        (index, pins) => Err(index
            .err()
            .into_iter()
            .chain(pins.err())
            .flatten()
            .collect()),
    })
}

/// The structural checks of WF3 steps 2 to 4 (index, layout, rail IDs,
/// manifests, fixed inputs).
fn structural_findings(
    options: &Options,
    corpus: &Path,
    selected: &[&IndexEntry],
    index: &crate::index::Index,
    pins: &Pins,
) -> Vec<Finding> {
    let mut findings = Vec::new();
    if options.program.is_none() {
        let in_scope = crate::index::Index {
            schema_version: 1,
            entries: index
                .entries
                .iter()
                .filter(|e| options.class.is_none_or(|c| e.class == c))
                .cloned()
                .collect(),
        };
        // Classes with an APPROVAL.md are delivered and must be complete
        // (BR4.2); `--complete` requires all four.
        let full: Vec<Class> = Class::ALL
            .into_iter()
            .filter(|c| options.complete || corpus.join(c.name()).join("APPROVAL.md").is_file())
            .collect();
        findings.extend(check_composition(
            "index.toml",
            &in_scope,
            &Composition::approved(),
            &full,
        ));
    }
    let chosen = crate::index::Index {
        schema_version: 1,
        entries: selected.iter().map(|e| (*e).clone()).collect(),
    };
    findings.extend(check_layout(corpus, &chosen));
    for entry in selected {
        let dir = corpus.join(entry.class.name()).join(&entry.program_id);
        if let Ok(text) = fs::read_to_string(dir.join("rail/main.rlc")) {
            findings.extend(check_rail_ids(
                &format!("{}/{}/rail/main.rlc", entry.class, entry.program_id),
                &entry.program_id,
                &text,
            ));
        }
        findings.extend(check_manifests(&dir, entry, pins));
        findings.extend(check_fixed_inputs(&dir, entry));
    }
    findings
}

fn run_check(options: &Options, out: &mut dyn Write) -> Result<i32, String> {
    let started = Instant::now();
    let corpus = options
        .corpus_dir
        .canonicalize()
        .map_err(|e| format!("corpus directory {}: {e}", options.corpus_dir.display()))?;
    let root = corpus.parent().unwrap_or(&corpus).to_path_buf();

    let (index, pins) = match load(&corpus)? {
        Ok(loaded) => loaded,
        Err(findings) => {
            for f in &findings {
                say(out, &format!("FAIL {f}"));
            }
            say(out, "corpus check: the corpus files have problems");
            return Ok(1);
        }
    };

    let workspace_toolchain = fs::read_to_string(root.join("rust-toolchain.toml"))
        .map_err(|e| format!("cannot read rust-toolchain.toml: {e}"))?;
    let installed = detect_installed(&options.langs, &root)?;
    let mismatches = verify_installed(&pins, &installed, &workspace_toolchain);
    if !mismatches.is_empty() {
        return Err(mismatches
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\nerror: "));
    }

    let selected: Vec<&IndexEntry> = index
        .entries
        .iter()
        .filter(|e| options.class.is_none_or(|c| e.class == c))
        .filter(|e| options.program.as_ref().is_none_or(|p| *p == e.program_id))
        .collect();
    if let Some(p) = &options.program
        && selected.is_empty()
    {
        return Err(format!("no program named `{p}` in the index"));
    }

    let findings = structural_findings(options, &corpus, &selected, &index, &pins);
    if !findings.is_empty() {
        for f in &findings {
            say(out, &format!("FAIL {f}"));
        }
        say(
            out,
            &format!(
                "corpus check: {} problem(s) found; nothing was run",
                findings.len()
            ),
        );
        return Ok(1);
    }
    if selected.is_empty() {
        say(
            out,
            "corpus check: 0 passed, 0 failed, 0 skipped, 0 not run (no programs to check)",
        );
        return Ok(0);
    }

    let work = options
        .work_dir
        .clone()
        .unwrap_or_else(|| std::env::temp_dir().join("rail-corpus-check"));
    fs::create_dir_all(&work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let work = work
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", work.display()))?;
    let scratch = work.join(format!("scratch-{}", std::process::id()));
    fs::create_dir_all(&scratch)
        .map_err(|e| format!("cannot create {}: {e}", scratch.display()))?;
    let dirs = Dirs {
        corpus: &corpus,
        work: &work,
        scratch: &scratch,
    };
    let result = run_programs(options, &dirs, &pins, &selected, started, out);
    let _ = fs::remove_dir_all(&scratch);
    result
}

/// The directories one run uses.
struct Dirs<'a> {
    corpus: &'a Path,
    work: &'a Path,
    scratch: &'a Path,
}

fn run_programs(
    options: &Options,
    dirs: &Dirs,
    pins: &Pins,
    selected: &[&IndexEntry],
    started: Instant,
    out: &mut dyn Write,
) -> Result<i32, String> {
    let (corpus, work, scratch) = (dirs.corpus, dirs.work, dirs.scratch);
    let profile = options
        .sandbox_profile
        .clone()
        .unwrap_or_else(|| corpus.join("tools/sandbox/macos.sb"));
    let probe =
        std::env::current_exe().map_err(|e| format!("cannot find the checker itself: {e}"))?;
    let sandbox = prepare_sandbox(
        Platform::current(),
        &default_search_path(),
        &profile,
        &pins.bubblewrap_min,
        &probe,
        scratch,
    )
    .map_err(|e| e.to_string())?;

    let helper = if selected
        .iter()
        .any(|e| e.args.iter().any(|a| a.contains("{helper}")))
    {
        let built = build_version(
            Lang::Rust,
            &corpus.join("tools/helper"),
            &work.join("build/helper"),
        )
        .map_err(|e| format!("cannot build the helper: {e}"))?;
        Some(built.exe)
    } else {
        None
    };
    let ctx = Context {
        sandbox,
        mode: if options.bless {
            Mode::Bless
        } else {
            Mode::Check
        },
        deadline: Duration::from_secs(options.deadline_seconds),
        budget: Budget::new((options.budget_seconds > 0).then(|| {
            Duration::from_secs(options.budget_seconds).saturating_sub(started.elapsed())
        })),
        is_root: running_as_root(),
        cores: std::thread::available_parallelism()
            .map_or(2, |n| n.get())
            .max(2),
        scratch: scratch.to_path_buf(),
        helper,
    };

    let mut report = Report::default();
    let mut generator_failures = 0usize;
    let mut generator_passes = 0usize;
    for entry in selected {
        let dir = corpus.join(entry.class.name()).join(&entry.program_id);
        let name = format!("{}/{}", entry.class, entry.program_id);

        match check_generator(&ctx, entry, &dir, work, &name) {
            Ok(()) => {
                generator_passes += 1;
                say(out, &format!("PASS {name} generator"));
            }
            Err(message) => {
                generator_failures += 1;
                say(out, &format!("FAIL {name} generator: {message}"));
            }
        }

        let mut jobs = Vec::new();
        let mut build_time = Vec::new();
        let mut before: Vec<VersionReport> = Vec::new();
        for &lang in &options.langs {
            let mut version = VersionReport {
                program: entry.program_id.clone(),
                class: entry.class.name().to_string(),
                lang,
                status: Status::Pass,
                inputs_run: 0,
                elapsed: Duration::ZERO,
            };
            if ctx.budget.exhausted() {
                report.over_budget = true;
                version.status = Status::NotRun("over time budget".to_string());
                before.push(version);
                continue;
            }
            let t = Instant::now();
            let out_dir = work
                .join("build")
                .join(entry.class.name())
                .join(&entry.program_id)
                .join(lang.name());
            match build_version(lang, &dir.join(lang.name()), &out_dir) {
                Ok(built) => {
                    build_time.push((lang, t.elapsed()));
                    jobs.push(Job {
                        entry: (*entry).clone(),
                        lang,
                        program_dir: dir.clone(),
                        exe: built.exe,
                        build_dir: built.build_dir,
                    });
                }
                Err(message) => {
                    version.status = Status::Fail(format!("build failed: {message}"));
                    version.elapsed = t.elapsed();
                    before.push(version);
                }
            }
        }
        let mut part = run_jobs(&ctx, &jobs);
        for v in &mut part.versions {
            if let Some((_, t)) = build_time.iter().find(|(l, _)| *l == v.lang) {
                v.elapsed += *t;
            }
        }
        report.over_budget |= part.over_budget;
        let mut all: Vec<VersionReport> = before;
        all.extend(part.versions);
        all.sort_by_key(|v| v.lang);
        for v in all {
            say(out, &v.line());
            report.versions.push(v);
        }
    }

    summarize(&report, generator_passes, generator_failures, out);
    Ok(i32::from(report.failed() || generator_failures > 0))
}

/// BR7.1 for one program: build the generator and run it twice in the sandbox.
fn check_generator(
    ctx: &Context,
    entry: &IndexEntry,
    dir: &Path,
    work: &Path,
    name: &str,
) -> Result<(), String> {
    let out_dir = work
        .join("build")
        .join(entry.class.name())
        .join(&entry.program_id)
        .join("gen");
    let built = build_version(Lang::Rust, &dir.join("inputs/gen"), &out_dir)
        .map_err(|e| format!("build failed: {e}"))?;
    let runs = work.join("gen-runs").join(name.replace('/', "-"));
    fs::create_dir_all(&runs).map_err(|e| e.to_string())?;
    let launch = |spec: &LaunchSpec| {
        ctx.sandbox.command(&SandboxSpec {
            exe: spec.exe.clone(),
            args: spec.args.clone(),
            run_dir: spec.run_dir.clone(),
            build_dir: built.build_dir.clone(),
            read_dirs: Vec::new(),
            env: Vec::new(),
        })
    };
    let result = check_generator_determinism(
        &launch,
        &entry.program_id,
        &built.exe,
        1,
        entry.size_range.0,
        &runs,
    )
    .map_err(|f| f.message);
    let _ = fs::remove_dir_all(&runs);
    result
}

fn summarize(report: &Report, gen_pass: usize, gen_fail: usize, out: &mut dyn Write) {
    let count = |f: fn(&Status) -> bool| report.versions.iter().filter(|v| f(&v.status)).count();
    let passed = count(|s| matches!(s, Status::Pass | Status::Blessed)) + gen_pass;
    let failed = count(|s| matches!(s, Status::Fail(_))) + gen_fail;
    let skipped = count(|s| matches!(s, Status::Skipped(_)));
    let not_run = count(|s| matches!(s, Status::NotRun(_)));
    if report.over_budget {
        say(
            out,
            "over time budget: the check was stopped and the remaining versions were not run",
        );
        say(out, "slowest versions:");
        for v in report.slowest(10) {
            say(
                out,
                &format!(
                    "  {:>7.1}s {}/{} {}",
                    v.elapsed.as_secs_f64(),
                    v.class,
                    v.program,
                    v.lang
                ),
            );
        }
    }
    say(
        out,
        &format!(
            "corpus check: {passed} passed, {failed} failed, {skipped} skipped, {not_run} not run"
        ),
    );
}
