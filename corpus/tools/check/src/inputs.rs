//! Fixed and generated inputs (BR7.1, BR7.3).

use crate::finding::Finding;
use crate::index::{IndexEntry, MAX_FIXED_INPUTS};
use crate::proc::{ProcOutcome, run_command};
use crate::tree::tree_listing;
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The largest committed fixed input file (BR7.3).
pub const MAX_FIXED_BYTES: u64 = 64 * 1024;

/// How long one generator run may take.
const GENERATOR_DEADLINE: Duration = Duration::from_secs(60);

/// What to start: the caller decides how (sandboxed in production).
#[derive(Clone, Debug)]
pub struct LaunchSpec {
    pub exe: PathBuf,
    pub args: Vec<OsString>,
    /// The run directory: the current directory of the process.
    pub run_dir: PathBuf,
}

/// BR7.3: fixed inputs exist as listed, are at most eight, and every file in
/// them is at most 64 KiB.
pub fn check_fixed_inputs(program_dir: &Path, entry: &IndexEntry) -> Vec<Finding> {
    let mut findings = Vec::new();
    let fixed = program_dir.join("inputs/fixed");
    let all: BTreeSet<String> = fs::read_dir(&fixed)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    // `<input>.setup` files describe run-time cases of a listed input
    // (BR6.5); they are not inputs themselves.
    let is_setup = |name: &String| {
        name.strip_suffix(".setup")
            .is_some_and(|stem| entry.fixed_inputs.iter().any(|f| f == stem))
    };
    let found: BTreeSet<String> = all.iter().filter(|n| !is_setup(n)).cloned().collect();
    let problem =
        |file: &str, message: String| Finding::new(file, message).program(&entry.program_id);
    if found.len() > MAX_FIXED_INPUTS {
        findings.push(problem(
            "inputs/fixed",
            format!(
                "has more than {MAX_FIXED_INPUTS} fixed inputs ({})",
                found.len()
            ),
        ));
    }
    for name in &entry.fixed_inputs {
        if !fixed.join(name).is_dir() {
            findings.push(problem(
                &format!("inputs/fixed/{name}"),
                "fixed input listed in the index is missing (it must be a directory)".to_string(),
            ));
        }
    }
    for name in &found {
        if !entry.fixed_inputs.contains(name) {
            findings.push(problem(
                &format!("inputs/fixed/{name}"),
                "fixed input is not listed in the index".to_string(),
            ));
        }
    }
    for name in &all {
        check_sizes(
            &fixed.join(name),
            &format!("inputs/fixed/{name}"),
            &problem,
            &mut findings,
        );
    }
    findings
}

fn check_sizes(
    path: &Path,
    shown: &str,
    problem: &dyn Fn(&str, String) -> Finding,
    findings: &mut Vec<Finding>,
) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if meta.is_dir() {
        let mut names: Vec<String> = fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        for name in names {
            check_sizes(
                &path.join(&name),
                &format!("{shown}/{name}"),
                problem,
                findings,
            );
        }
    } else if meta.len() > MAX_FIXED_BYTES {
        findings.push(problem(
            shown,
            format!(
                "fixed input is {} bytes; the limit is {MAX_FIXED_BYTES} (generate large inputs instead)",
                meta.len()
            ),
        ));
    }
}

/// BR7.1: runs the generator twice with `--seed <seed> --size <size> --out
/// out` in two fresh run directories under `work`, and requires the same
/// files with the same bytes.
pub fn check_generator_determinism(
    launch: &dyn Fn(&LaunchSpec) -> Command,
    program: &str,
    gen_exe: &Path,
    seed: u64,
    size: u64,
    work: &Path,
) -> Result<(), Finding> {
    let problem = |message: String| Finding::new("inputs/gen", message).program(program);
    let mut listings = Vec::new();
    for n in 1..=2 {
        let run_dir = work.join(format!("gen-{n}"));
        fs::create_dir_all(&run_dir)
            .map_err(|e| problem(format!("cannot create the run directory: {e}")))?;
        let spec = LaunchSpec {
            exe: gen_exe.to_path_buf(),
            args: [
                "--seed",
                &seed.to_string(),
                "--size",
                &size.to_string(),
                "--out",
                "out",
            ]
            .iter()
            .map(OsString::from)
            .collect(),
            run_dir: run_dir.clone(),
        };
        let outcome = run_command(launch(&spec), b"", GENERATOR_DEADLINE)
            .map_err(|e| problem(format!("cannot start the generator: {e}")))?;
        if !generator_succeeded(&outcome) {
            return Err(problem(format!(
                "generator failed with exit code {} for --seed {seed} --size {size}",
                outcome
                    .code
                    .map_or_else(|| "none".to_string(), |c| c.to_string())
            )));
        }
        let listing = tree_listing(&run_dir.join("out"))
            .map_err(|e| problem(format!("generator wrote no output directory: {e}")))?;
        listings.push(listing);
    }
    if listings[0] != listings[1] {
        let first: BTreeSet<&str> = listings[0].lines().collect();
        let second: BTreeSet<&str> = listings[1].lines().collect();
        let differing = first
            .symmetric_difference(&second)
            .next()
            .copied()
            .unwrap_or("");
        let name = differing.rsplit(' ').next().unwrap_or("");
        return Err(problem(format!(
            "generator is not deterministic: two runs with --seed {seed} --size {size} \
             differ at {name}"
        )));
    }
    Ok(())
}

/// Whether a finished generator run counts as success.
pub fn generator_succeeded(outcome: &ProcOutcome) -> bool {
    outcome.code == Some(0) && !outcome.timed_out
}
