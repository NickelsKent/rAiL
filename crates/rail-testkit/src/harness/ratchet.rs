//! Acceptance harness: the volume ratchet (BR2.1-BR2.5, NFR9.15). It
//! compares a base settings file (the one on `main`) with a new one and
//! refuses anything that lowers or narrows what a suite checks. Changes that
//! cannot weaken a suite pass with a notice.

use std::fmt::Display;

use crate::harness::settings::{
    Level, Settings, SettingsError, Status, SuiteConfig, Volume, raw_schema_version,
};

/// The ratchet's verdict.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RatchetReport {
    /// Refusals: each names the suite, field, old and new value.
    pub failures: Vec<String>,
    /// Changes reported for review but not refused.
    pub notices: Vec<String>,
}

impl RatchetReport {
    /// Whether nothing was refused (exit code 0).
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }

    fn fail(
        &mut self,
        suite: &SuiteConfig,
        field: &str,
        old: impl Display,
        new: impl Display,
        what: &str,
    ) {
        self.failures.push(format!(
            "suite {}, {field}: {what}: {old} -> {new}",
            suite.id()
        ));
    }

    fn note(&mut self, suite: &SuiteConfig, field: &str, text: impl Display) {
        self.notices
            .push(format!("suite {}, {field}: {text}", suite.id()));
    }
}

/// Checks `new` settings text against `base` text (`None` when `main` has
/// no settings file yet). Both must be valid settings; an invalid file is a
/// [`SettingsError`] (exit code 2).
pub fn check_ratchet(
    base: Option<&str>,
    base_label: &str,
    new: &str,
    new_label: &str,
) -> Result<RatchetReport, SettingsError> {
    let Some(base) = base else {
        Settings::parse(new, new_label)?;
        return Ok(RatchetReport {
            failures: Vec::new(),
            notices: vec!["no base settings; the ratchet starts from this file".to_string()],
        });
    };
    let (old_version, new_version) = (raw_schema_version(base), raw_schema_version(new));
    if old_version != new_version {
        // No comparison exists between different schema versions (BR2.5).
        let show = |v: Option<i64>| v.map_or("none".to_string(), |v| v.to_string());
        return Ok(RatchetReport {
            failures: vec![format!(
                "schema_version: the files use different schema versions ({} in {base_label}, {} in {new_label}) and the harness has no comparison for that pair",
                show(old_version),
                show(new_version)
            )],
            notices: Vec::new(),
        });
    }
    let base = Settings::parse(base, base_label)?;
    let new = Settings::parse(new, new_label)?;
    Ok(compare_settings(&base, &new))
}

/// Compares two parsed settings files (BR2.1-BR2.4).
pub fn compare_settings(base: &Settings, new: &Settings) -> RatchetReport {
    let mut report = RatchetReport::default();
    if base.budgets != new.budgets {
        report.notices.push(format!(
            "budgets: pr_minutes {} -> {}, nightly_group_minutes {} -> {}",
            base.budgets.pr_minutes,
            new.budgets.pr_minutes,
            base.budgets.nightly_group_minutes,
            new.budgets.nightly_group_minutes
        ));
    }
    if base.reference_machines != new.reference_machines {
        report
            .notices
            .push("reference_machines: the list changed".to_string());
    }
    for old in &base.suites {
        match new.suites.iter().find(|s| s.id() == old.id()) {
            Some(current) => compare_suite(&mut report, old, current),
            None => report.failures.push(format!(
                "suite {}: removed; a suite is never removed (BR2.2)",
                old.id()
            )),
        }
    }
    report
}

fn compare_suite(report: &mut RatchetReport, old: &SuiteConfig, new: &SuiteConfig) {
    let implemented = old.status == Status::Implemented;
    if implemented && new.status == Status::Pending {
        report.fail(
            old,
            "status",
            "implemented",
            "pending",
            "a suite never goes back to pending",
        );
    }
    if old.volumes.unit_of_measure != new.volumes.unit_of_measure {
        report.fail(
            old,
            "volumes.unit_of_measure",
            format!("{:?}", old.volumes.unit_of_measure),
            format!("{:?}", new.volumes.unit_of_measure),
            "the unit of measure never changes",
        );
    }
    for level in Level::ALL {
        compare_level(report, old, new, *level);
    }
    compare_lists(report, old, "inputs", &old.inputs, &new.inputs);
    compare_lists(report, old, "platforms", &old.platforms, &new.platforms);
    compare_lists(report, old, "drivers", &old.drivers, &new.drivers);
    if old.build != new.build {
        if implemented {
            report.fail(
                old,
                "build",
                old.build,
                new.build,
                "the build of an implemented suite never changes",
            );
        } else {
            report.note(
                old,
                "build",
                format!("{} -> {} (pending suite)", old.build, new.build),
            );
        }
    }
    if new.process_deadline_seconds < old.process_deadline_seconds {
        report.fail(
            old,
            "process_deadline_seconds",
            old.process_deadline_seconds,
            new.process_deadline_seconds,
            "lowered",
        );
    }
    if old.cross_platform && !new.cross_platform {
        report.fail(
            old,
            "cross_platform",
            true,
            false,
            "a cross-platform comparison is never dropped",
        );
    }
    if implemented && old.seed_policy.uses_seed && !new.seed_policy.uses_seed {
        report.fail(
            old,
            "seed_policy.uses_seed",
            true,
            false,
            "generated inputs are never dropped",
        );
    }
    let show = |seed: Option<u64>| seed.map_or("none".to_string(), |s| s.to_string());
    if old.seed_policy.pr_seed != new.seed_policy.pr_seed {
        report.note(
            old,
            "seed_policy.pr_seed",
            format!(
                "{} -> {}",
                show(old.seed_policy.pr_seed),
                show(new.seed_policy.pr_seed)
            ),
        );
    }
    if old.nightly_group != new.nightly_group {
        let group = |g: &Option<String>| g.clone().unwrap_or_else(|| "none".into());
        report.note(
            old,
            "nightly_group",
            format!(
                "{} -> {}",
                group(&old.nightly_group),
                group(&new.nightly_group)
            ),
        );
    }
    if old.sanitizer_rustflags != new.sanitizer_rustflags {
        report.note(old, "sanitizer_rustflags", "changed");
    }
}

fn compare_level(report: &mut RatchetReport, old: &SuiteConfig, new: &SuiteConfig, level: Level) {
    let field = match level {
        Level::Release => "volumes.full".to_string(),
        other => format!("volumes.{other}"),
    };
    let Some(before) = old.volumes.for_level(level) else {
        return;
    };
    let after = new.volumes.for_level(level).unwrap_or(&[]);
    for (i, b) in before.iter().enumerate() {
        let entry = format!("{field}[{i}]");
        match after.get(i) {
            None => report.fail(old, &entry, b, "(none)", "volume entry removed"),
            Some(a) => compare_entry(report, old, &entry, b, a),
        }
    }
    for (i, a) in after.iter().enumerate().skip(before.len()) {
        report.note(old, &format!("{field}[{i}]"), format!("added {a}"));
    }
}

fn compare_entry(
    report: &mut RatchetReport,
    suite: &SuiteConfig,
    entry: &str,
    old: &Volume,
    new: &Volume,
) {
    if new.is_lower_than(old) {
        report.fail(suite, entry, old, new, "lowered");
        return;
    }
    if let (Volume::Exhaustive { over: a }, Volume::Exhaustive { over: b }) = (old, new)
        && a != b
    {
        report.fail(
            suite,
            entry,
            old,
            new,
            "the exhaustive set (over) never changes",
        );
        return;
    }
    if old.is_lower_than(new) || (old.rank() == 0 && new != old) {
        report.note(suite, entry, format!("raised {old} -> {new}"));
    }
}

fn compare_lists<T: PartialEq + Display>(
    report: &mut RatchetReport,
    suite: &SuiteConfig,
    field: &str,
    old: &[T],
    new: &[T],
) {
    for item in old.iter().filter(|o| !new.contains(o)) {
        report.fail(suite, field, item, "(removed)", "removed or replaced");
    }
    for item in new.iter().filter(|n| !old.contains(n)) {
        report.note(suite, field, format!("added {item}"));
    }
}
