//! Acceptance harness: `rail-accept --compare` (BR7.7, BR3.5, NFR1.4).
//!
//! Cross-platform suites (T1, T3, T7) pass only when every platform they
//! list produced byte-identical normalized observations. T13 passes only
//! with a passing result from every recorded reference machine. A
//! cross-platform suite that did not run on any compared platform is
//! skipped with a notice; one that ran on some platforms but not others
//! fails.

use std::fs;
use std::path::{Path, PathBuf};

use rail_json::Value;

use crate::harness::report::Outcome;
use crate::harness::settings::{Level, Platform, Settings, SuiteConfig, SuiteId};

/// The comparer's verdict.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CompareReport {
    /// Differences and missing files (exit code 1).
    pub failures: Vec<String>,
    /// Suites skipped and other information.
    pub notices: Vec<String>,
}

impl CompareReport {
    /// Whether nothing failed (exit code 0).
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// One suite's entry in a results file.
#[derive(Clone, Debug)]
struct RunEntry {
    suite: SuiteId,
    outcome: Outcome,
    observations: Option<PathBuf>,
}

/// The parts of a results file the comparer reads.
#[derive(Clone, Debug)]
struct ResultsFile {
    path: PathBuf,
    level: Level,
    platform: Platform,
    reference_machine: Option<String>,
    runs: Vec<RunEntry>,
}

fn read_results(path: &Path) -> Result<ResultsFile, String> {
    let fail = |what: String| format!("cannot read results file {}: {what}", path.display());
    let bytes = fs::read(path).map_err(|e| fail(e.to_string()))?;
    let root = rail_json::parse(&bytes).map_err(|e| fail(e.to_string()))?;
    let text = |v: &Value, key: &str| -> Result<String, String> {
        v.get(key)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| fail(format!("missing {key:?}")))
    };
    let level = Level::parse(&text(&root, "level")?).ok_or_else(|| fail("unknown level".into()))?;
    let platform = Platform::parse(&text(&root, "platform")?)
        .ok_or_else(|| fail("unknown platform".into()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut runs = Vec::new();
    for run in root
        .get("runs")
        .and_then(Value::as_array)
        .ok_or_else(|| fail("missing \"runs\"".into()))?
    {
        runs.push(RunEntry {
            suite: SuiteId::parse(&text(run, "suite")?)
                .ok_or_else(|| fail("unknown suite".into()))?,
            outcome: Outcome::parse(&text(run, "outcome")?)
                .ok_or_else(|| fail("unknown outcome".into()))?,
            observations: run
                .get("observations_path")
                .and_then(Value::as_str)
                .map(|rel| dir.join(rel)),
        });
    }
    Ok(ResultsFile {
        path: path.to_path_buf(),
        level,
        platform,
        reference_machine: root
            .get("reference_machine")
            .and_then(Value::as_str)
            .map(str::to_string),
        runs,
    })
}

/// Compares the given results files of one level.
pub fn compare(settings: &Settings, files: &[PathBuf]) -> CompareReport {
    let mut report = CompareReport::default();
    let mut results = Vec::new();
    for path in files {
        match read_results(path) {
            Ok(r) => results.push(r),
            Err(e) => report.failures.push(e),
        }
    }
    if !report.passed() {
        return report;
    }
    if let Some(first) = results.first()
        && let Some(other) = results.iter().find(|r| r.level != first.level)
    {
        report.failures.push(format!(
            "results files are for different levels: {} is {}, {} is {}",
            first.path.display(),
            first.level,
            other.path.display(),
            other.level
        ));
        return report;
    }
    for suite in settings.suites.iter().filter(|s| s.cross_platform) {
        compare_suite(&mut report, suite, &results);
    }
    compare_reference_machines(&mut report, settings, &results);
    report
}

fn compare_suite(report: &mut CompareReport, suite: &SuiteConfig, results: &[ResultsFile]) {
    let id = suite.id();
    let entries: Vec<(&ResultsFile, &RunEntry)> = results
        .iter()
        .flat_map(|r| r.runs.iter().filter(|e| e.suite == id).map(move |e| (r, e)))
        .collect();
    if !entries.iter().any(|(_, e)| e.outcome.ran()) {
        report.notices.push(format!(
            "{id}: pending or not run on every compared platform; skipped"
        ));
        return;
    }
    let mut files = Vec::new();
    for platform in &suite.platforms {
        let ran = entries
            .iter()
            .find(|(r, e)| r.platform == *platform && e.outcome.ran());
        match ran {
            Some((_, e)) => match &e.observations {
                Some(path) => files.push((*platform, path.clone())),
                None => report.failures.push(format!(
                    "{id}: the {platform} results name no observations file"
                )),
            },
            None => {
                let seen = entries.iter().find(|(r, _)| r.platform == *platform);
                report.failures.push(match seen {
                    Some((_, e)) => format!(
                        "{id}: ran on another platform but is {} on {platform}",
                        e.outcome
                    ),
                    None => format!("{id}: no {platform} results"),
                });
            }
        }
    }
    if let Some(((first_platform, first), rest)) = files.split_first() {
        for (platform, path) in rest {
            if let Some(difference) = first_difference(first, path) {
                report.failures.push(format!(
                    "{id}: {first_platform} and {platform} observations differ: {difference}"
                ));
            }
        }
    }
}

/// Where two observation files first differ, or `None` when they are
/// byte-identical.
fn first_difference(a: &Path, b: &Path) -> Option<String> {
    let read = |p: &Path| fs::read(p).map_err(|e| format!("cannot read {}: {e}", p.display()));
    let (left, right) = match (read(a), read(b)) {
        (Ok(l), Ok(r)) => (l, r),
        (Err(e), _) | (_, Err(e)) => return Some(e),
    };
    if left == right {
        return None;
    }
    let entries = |bytes: &[u8]| {
        rail_json::parse(bytes)
            .ok()
            .and_then(|v| v.as_array().cloned())
            .unwrap_or_default()
    };
    let (l, r) = (entries(&left), entries(&right));
    let index = (0..l.len().max(r.len()))
        .find(|i| l.get(*i) != r.get(*i))
        .unwrap_or(0);
    let label = l
        .get(index)
        .or_else(|| r.get(index))
        .and_then(|e| e.get("input"))
        .and_then(Value::as_str)
        .unwrap_or("?");
    Some(format!("first difference at input {index} ({label})"))
}

/// T13 needs a passing result from every recorded reference machine
/// (BR3.5, NFR1.4).
fn compare_reference_machines(
    report: &mut CompareReport,
    settings: &Settings,
    results: &[ResultsFile],
) {
    let t13: Vec<(&ResultsFile, &RunEntry)> = results
        .iter()
        .flat_map(|r| {
            r.runs
                .iter()
                .filter(|e| e.suite == SuiteId::T13)
                .map(move |e| (r, e))
        })
        .collect();
    if !t13.iter().any(|(_, e)| e.outcome.ran()) {
        if !t13.is_empty() {
            report
                .notices
                .push("T13: not run on any reference machine; skipped".to_string());
        }
        return;
    }
    for machine in &settings.reference_machines {
        let passed = t13.iter().any(|(r, e)| {
            r.reference_machine.as_deref() == Some(machine.id.as_str())
                && e.outcome == Outcome::Pass
        });
        if !passed {
            report.failures.push(format!(
                "T13: no passing result from reference machine {}",
                machine.id
            ));
        }
    }
}
