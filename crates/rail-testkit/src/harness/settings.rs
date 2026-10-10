//! The recorded suite settings, `tests/acceptance/suites.toml` (contract C9),
//! and their validation (BR1.1-BR1.7, BR3.6, BR6.1, NFR9.17).
//!
//! The file is read with `toml` + `serde`, with unknown keys refused
//! everywhere (BR1.1). [`Settings::parse`] checks everything that depends
//! only on the file; [`Settings::check_environment`] checks what depends on
//! the repository and the registered suites (input paths, generator names,
//! BR6.1).

use std::collections::BTreeSet;
use std::fmt;
use std::path::{Component, Path};

use serde::Deserialize;
use toml::Spanned;

use crate::suites::Registry;

/// The settings file's path, relative to the repository root.
pub const SETTINGS_PATH: &str = "tests/acceptance/suites.toml";

/// The only schema version this harness reads (BR1.1).
pub const SCHEMA_VERSION: u32 = 1;

/// The largest per-process deadline a suite may record (BR5.2).
pub const MAX_DEADLINE_SECONDS: u64 = 90_000;

/// A suite identifier: S0 or T1..T14, ordered S0, T1, ..., T14.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(try_from = "String")]
pub struct SuiteId(u8);

impl SuiteId {
    /// The walking-skeleton smoke suite.
    pub const S0: SuiteId = SuiteId(0);
    /// T13, performance targets (reference machines only, BR3.5).
    pub const T13: SuiteId = SuiteId(13);
    /// T14, minimal dependency operation.
    pub const T14: SuiteId = SuiteId(14);

    /// Every suite, in the fixed order (BR1.2).
    pub const ALL: [SuiteId; 15] = [
        SuiteId(0),
        SuiteId(1),
        SuiteId(2),
        SuiteId(3),
        SuiteId(4),
        SuiteId(5),
        SuiteId(6),
        SuiteId(7),
        SuiteId(8),
        SuiteId(9),
        SuiteId(10),
        SuiteId(11),
        SuiteId(12),
        SuiteId(13),
        SuiteId(14),
    ];

    /// Parses `S0` or `T1`..`T14`.
    pub fn parse(text: &str) -> Option<SuiteId> {
        SuiteId::ALL.into_iter().find(|id| id.to_string() == text)
    }

    /// The suite's position in the fixed order.
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl TryFrom<String> for SuiteId {
    type Error = String;
    fn try_from(text: String) -> Result<SuiteId, String> {
        SuiteId::parse(&text).ok_or_else(|| format!("unknown suite id {text:?} (S0, T1..T14)"))
    }
}

impl fmt::Display for SuiteId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0 == 0 {
            f.write_str("S0")
        } else {
            write!(f, "T{}", self.0)
        }
    }
}

macro_rules! named_enum {
    ($(#[$doc:meta])* $name:ident { $($(#[$vdoc:meta])* $variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
        pub enum $name {
            $($(#[$vdoc])* #[serde(rename = $text)] $variant),+
        }

        impl $name {
            /// Every value, in declaration order.
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            /// The value's name as written in settings and results.
            pub fn as_str(self) -> &'static str {
                match self { $($name::$variant => $text),+ }
            }

            /// Parses the written name.
            pub fn parse(text: &str) -> Option<$name> {
                $name::ALL.iter().copied().find(|v| v.as_str() == text)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };
}

named_enum!(
    /// A harness level (WF6).
    Level {
        /// Every pull request.
        Pr => "pr",
        /// The nightly workflow.
        Nightly => "nightly",
        /// The release check on the owner's machines.
        Release => "release",
    }
);
named_enum!(
    /// A required platform (BR1.7).
    Platform {
        /// macOS.
        Macos => "macos",
        /// Linux.
        Linux => "linux",
    }
);
named_enum!(
    /// The build of `rail` a suite runs against (BR3.6).
    Build {
        /// `cargo build --locked -p rail`.
        Debug => "debug",
        /// `cargo build --locked --release -p rail`.
        Release => "release",
        /// The pinned nightly with the suite's sanitizer flags.
        Sanitizer => "sanitizer",
    }
);
named_enum!(
    /// How a suite observes `rail` (contracts E1, E2).
    Driver {
        /// The command line.
        Cli => "cli",
        /// The RAP protocol.
        Rap => "rap",
    }
);
named_enum!(
    /// Whether a suite's implementation is registered (C9).
    Status {
        /// Registered by U2, not yet implemented.
        Pending => "pending",
        /// Implemented by its owning unit.
        Implemented => "implemented",
    }
);

impl Platform {
    /// The platform this program was built for, if it is a required one.
    pub fn host() -> Option<Platform> {
        if cfg!(target_os = "macos") {
            Some(Platform::Macos)
        } else if cfg!(target_os = "linux") {
            Some(Platform::Linux)
        } else {
            None
        }
    }
}

/// One recorded volume entry. Ordered for the ratchet:
/// `not_run` < `count(n)` < `exhaustive` (BR2.1).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Volume {
    /// A number of units.
    Count {
        /// How many units (at least 1).
        count: u64,
    },
    /// The whole of a named, finite set.
    Exhaustive {
        /// The set, e.g. "every §2.7 rule".
        over: String,
    },
    /// The suite does not run at this level.
    NotRun {
        /// Why not.
        reason: String,
    },
}

impl Volume {
    /// The rank used to order kinds: `not_run` 0, `count` 1, `exhaustive` 2.
    pub fn rank(&self) -> u8 {
        match self {
            Volume::NotRun { .. } => 0,
            Volume::Count { .. } => 1,
            Volume::Exhaustive { .. } => 2,
        }
    }

    /// Whether `self` is lower than `other` (BR1.5, BR2.1).
    pub fn is_lower_than(&self, other: &Volume) -> bool {
        match (self, other) {
            (Volume::Count { count: a }, Volume::Count { count: b }) => a < b,
            _ => self.rank() < other.rank(),
        }
    }
}

impl fmt::Display for Volume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Volume::Count { count } => write!(f, "count {count}"),
            Volume::Exhaustive { over } => write!(f, "exhaustive over {over:?}"),
            Volume::NotRun { reason } => write!(f, "not_run ({reason})"),
        }
    }
}

/// The three recorded volumes of a suite.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VolumeSet {
    /// What one unit of volume is.
    pub unit_of_measure: String,
    /// Pull-request volume (implemented suites only).
    pub pr: Option<Vec<Volume>>,
    /// Nightly volume (implemented suites only).
    pub nightly: Option<Vec<Volume>>,
    /// Full (release) volume, from spec §12.
    pub full: Vec<Volume>,
}

impl VolumeSet {
    /// The entries recorded for `level`.
    pub fn for_level(&self, level: Level) -> Option<&[Volume]> {
        match level {
            Level::Pr => self.pr.as_deref(),
            Level::Nightly => self.nightly.as_deref(),
            Level::Release => Some(&self.full),
        }
    }
}

/// The only allowed value for nightly and release seeds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum Fresh {
    /// A fresh seed from the operating system, printed before use.
    #[serde(rename = "fresh")]
    Fresh,
}

/// How a suite's generated inputs are seeded (BR3.1, BR4).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedPolicy {
    /// Whether the suite generates inputs from a seed.
    pub uses_seed: bool,
    /// The recorded pull-request seed. TOML integers are signed 64-bit, so a
    /// recorded seed is at most `i64::MAX`.
    pub pr_seed: Option<u64>,
    /// Always `fresh`.
    pub nightly_seed: Fresh,
    /// Always `fresh`.
    pub release_seed: Fresh,
}

/// Where a suite's inputs come from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputSource {
    /// A generator registered by the suite.
    Generator {
        /// Its name.
        generator: String,
    },
    /// A corpus directory (C10 layout).
    Corpus {
        /// Repository-relative path.
        path: String,
    },
    /// Fixed input files.
    Fixed {
        /// Repository-relative path.
        path: String,
    },
}

impl InputSource {
    /// The repository-relative path, for corpus and fixed inputs.
    pub fn path(&self) -> Option<&str> {
        match self {
            InputSource::Generator { .. } => None,
            InputSource::Corpus { path } | InputSource::Fixed { path } => Some(path),
        }
    }
}

impl fmt::Display for InputSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputSource::Generator { generator } => write!(f, "generator {generator:?}"),
            InputSource::Corpus { path } => write!(f, "corpus {path:?}"),
            InputSource::Fixed { path } => write!(f, "fixed {path:?}"),
        }
    }
}

/// One suite's recorded settings (C9).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuiteConfig {
    /// The suite id, with its place in the file.
    pub id: Spanned<SuiteId>,
    /// The property name from spec §12.
    pub title: String,
    /// The unit that implements the suite.
    pub owning_unit: String,
    /// Pending or implemented.
    pub status: Status,
    /// How the suite observes `rail`.
    pub drivers: Vec<Driver>,
    /// Where the suite runs.
    pub platforms: Vec<Platform>,
    /// Which build of `rail` it runs against.
    pub build: Build,
    /// Deadline for each `rail` process (BR5.2).
    pub process_deadline_seconds: u64,
    /// Whether `--compare` checks its observations across platforms (BR7.7).
    pub cross_platform: bool,
    /// The nightly group (required once implemented, NFR9.17).
    pub nightly_group: Option<String>,
    /// `RUSTFLAGS` for the sanitizer build (required once an implemented
    /// suite records `build = "sanitizer"`).
    pub sanitizer_rustflags: Option<String>,
    /// The suite's row in spec §12.
    pub pass_criterion_ref: String,
    /// Its input sources.
    pub inputs: Vec<InputSource>,
    /// Its seed policy.
    pub seed_policy: SeedPolicy,
    /// Its volumes.
    pub volumes: VolumeSet,
    /// The 1-based line of its `id` key (filled in after parsing).
    #[serde(skip)]
    pub line: usize,
}

impl SuiteConfig {
    /// The suite id.
    pub fn id(&self) -> SuiteId {
        *self.id.get_ref()
    }
}

/// Level time budgets (NFR9.16-NFR9.18). The release check has none.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Budgets {
    /// Budget for a whole pull-request run, per platform.
    pub pr_minutes: u64,
    /// Budget for one nightly group, per platform.
    pub nightly_group_minutes: u64,
}

/// One of the owner's reference machines (BR3.5).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMachine {
    /// Short lowercase name, never a hostname.
    pub id: String,
    /// Its platform.
    pub platform: Platform,
    /// CPU and memory.
    pub description: String,
}

/// The whole settings file.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    /// Always 1.
    pub schema_version: u32,
    /// Reference machines whose T13 results count.
    pub reference_machines: Vec<ReferenceMachine>,
    /// Level time budgets.
    pub budgets: Budgets,
    /// Every suite, S0, T1..T14.
    pub suites: Vec<SuiteConfig>,
    /// The file name used in error messages.
    #[serde(skip)]
    pub file: String,
}

/// A problem with the settings file: exit code 2, no suite runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingsError {
    /// The file, as named to the user.
    pub file: String,
    /// The 1-based line, when known.
    pub line: Option<usize>,
    /// The key, e.g. `suites[T3].volumes.pr`.
    pub key: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: ", self.file)?,
            None => write!(f, "{}: ", self.file)?,
        }
        write!(f, "{}: {}", self.key, self.message)
    }
}

impl std::error::Error for SettingsError {}

/// The 1-based line holding byte `offset` of `text`.
fn line_of(text: &str, offset: usize) -> usize {
    text.as_bytes()[..offset.min(text.len())]
        .iter()
        .filter(|b| **b == b'\n')
        .count()
        + 1
}

/// The key named in a `toml` error message, if any (e.g. "unknown field
/// `colour`").
fn key_in(message: &str) -> String {
    message
        .split('`')
        .nth(1)
        .map_or_else(|| "(file)".to_string(), str::to_string)
}

impl Settings {
    /// Parses and validates settings text. `file` names the file in errors.
    pub fn parse(text: &str, file: &str) -> Result<Settings, SettingsError> {
        check_schema_version(text, file)?;
        let mut settings: Settings = toml::from_str(text).map_err(|e| {
            let message = e.message().trim().to_string();
            SettingsError {
                file: file.to_string(),
                line: e.span().map(|span| line_of(text, span.start)),
                key: key_in(&message),
                message,
            }
        })?;
        settings.file = file.to_string();
        for suite in &mut settings.suites {
            suite.line = line_of(text, suite.id.span().start);
        }
        settings.validate()?;
        Ok(settings)
    }

    /// The settings of `id`. Validated settings hold every suite in order.
    pub fn suite(&self, id: SuiteId) -> &SuiteConfig {
        &self.suites[id.index()]
    }

    /// The nightly group names, sorted and without duplicates.
    pub fn nightly_groups(&self) -> Vec<String> {
        let groups: BTreeSet<&String> = self
            .suites
            .iter()
            .filter_map(|s| s.nightly_group.as_ref())
            .collect();
        groups.into_iter().cloned().collect()
    }

    fn error(&self, suite: Option<&SuiteConfig>, key: &str, message: String) -> SettingsError {
        SettingsError {
            file: self.file.clone(),
            line: suite.map(|s| s.line),
            key: match suite {
                Some(s) => format!("suites[{}].{key}", s.id()),
                None => key.to_string(),
            },
            message,
        }
    }

    fn validate(&self) -> Result<(), SettingsError> {
        if self.budgets.pr_minutes == 0 || self.budgets.nightly_group_minutes == 0 {
            return Err(self.error(None, "budgets", "budgets must be at least 1 minute".into()));
        }
        for machine in &self.reference_machines {
            if !is_name(&machine.id) {
                return Err(self.error(
                    None,
                    "reference_machines",
                    format!(
                        "reference machine id {:?} must be a short lowercase name",
                        machine.id
                    ),
                ));
            }
        }
        self.validate_order()?;
        self.suites.iter().try_for_each(|s| self.validate_suite(s))
    }

    /// BR1.2: exactly S0, T1..T14, each once, in order.
    fn validate_order(&self) -> Result<(), SettingsError> {
        for (position, expected) in SuiteId::ALL.iter().enumerate() {
            let Some(s) = self.suites.get(position) else {
                return Err(self.error(None, "suites", format!("suite {expected} is missing")));
            };
            if s.id() == *expected {
                continue;
            }
            let count = |id: SuiteId| self.suites.iter().filter(|o| o.id() == id).count();
            let message = if count(s.id()) > 1 {
                format!("suite {} is recorded more than once", s.id())
            } else if count(*expected) > 0 {
                format!("suite {} is out of order; expected {expected} here", s.id())
            } else {
                format!(
                    "suite {expected} is missing (found {} in its place)",
                    s.id()
                )
            };
            return Err(self.error(Some(s), "suites", message));
        }
        match self.suites.get(SuiteId::ALL.len()) {
            Some(extra) => Err(self.error(
                Some(extra),
                "suites",
                format!("suite {} is recorded more than once", extra.id()),
            )),
            None => Ok(()),
        }
    }

    fn validate_suite(&self, s: &SuiteConfig) -> Result<(), SettingsError> {
        let id = s.id();
        let fail = |key: &str, message: String| Err(self.error(Some(s), key, message));
        if s.drivers.is_empty() {
            return fail("drivers", format!("{id} lists no driver"));
        }
        if s.platforms.is_empty() {
            return fail(
                "platforms",
                format!("{id} lists no platform (macos, linux)"),
            );
        }
        if s.process_deadline_seconds == 0 || s.process_deadline_seconds > MAX_DEADLINE_SECONDS {
            return fail(
                "process_deadline_seconds",
                format!("{id}: must be between 1 and {MAX_DEADLINE_SECONDS}"),
            );
        }
        if (id == SuiteId::T13 || id == SuiteId::T14) && s.build != Build::Release {
            return fail(
                "build",
                format!(
                    "{id} must record build = \"release\" (BR3.6), not {:?}",
                    s.build.as_str()
                ),
            );
        }
        if s.nightly_group.as_deref().is_some_and(|g| !is_name(g)) {
            return fail(
                "nightly_group",
                format!("{id}: the group must be a lowercase name"),
            );
        }
        self.validate_volumes(s)?;
        self.validate_input_paths(s)?;
        match s.status {
            Status::Pending => self.validate_pending(s),
            Status::Implemented => self.validate_implemented(s),
        }
    }

    fn validate_volumes(&self, s: &SuiteConfig) -> Result<(), SettingsError> {
        let id = s.id();
        if s.volumes.full.is_empty() {
            return Err(self.error(
                Some(s),
                "volumes.full",
                format!("{id} has no full volume (BR1.3)"),
            ));
        }
        for level in Level::ALL {
            for v in s.volumes.for_level(*level).unwrap_or(&[]) {
                if matches!(v, Volume::Count { count: 0 }) {
                    return Err(self.error(
                        Some(s),
                        &level_key(*level),
                        format!("{id}: a count must be at least 1"),
                    ));
                }
                if *level == Level::Release && matches!(v, Volume::NotRun { .. }) {
                    return Err(self.error(
                        Some(s),
                        "volumes.full",
                        format!("{id}: the full volume can never be not_run (BR1.3)"),
                    ));
                }
            }
        }
        Ok(())
    }

    fn validate_input_paths(&self, s: &SuiteConfig) -> Result<(), SettingsError> {
        for path in s.inputs.iter().filter_map(InputSource::path) {
            let p = Path::new(path);
            let problem = if p.is_absolute() {
                "is absolute"
            } else if path.is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_)))
            {
                "must stay inside the repository (no '..')"
            } else {
                continue;
            };
            return Err(self.error(
                Some(s),
                "inputs",
                format!("{}: path {path:?} {problem} (BR1.6)", s.id()),
            ));
        }
        Ok(())
    }

    fn validate_pending(&self, s: &SuiteConfig) -> Result<(), SettingsError> {
        for (present, key) in [
            (s.volumes.pr.is_some(), "volumes.pr"),
            (s.volumes.nightly.is_some(), "volumes.nightly"),
            (s.seed_policy.pr_seed.is_some(), "seed_policy.pr_seed"),
        ] {
            if present {
                return Err(self.error(
                    Some(s),
                    key,
                    format!(
                        "{} is pending, so it records only its full volume (BR1.4)",
                        s.id()
                    ),
                ));
            }
        }
        Ok(())
    }

    fn validate_implemented(&self, s: &SuiteConfig) -> Result<(), SettingsError> {
        let id = s.id();
        let fail = |key: &str, message: String| Err(self.error(Some(s), key, message));
        let (Some(pr), Some(nightly)) = (&s.volumes.pr, &s.volumes.nightly) else {
            let key = if s.volumes.pr.is_none() {
                "volumes.pr"
            } else {
                "volumes.nightly"
            };
            return fail(
                key,
                format!("{id} is implemented and must record pr and nightly volumes (BR1.4)"),
            );
        };
        if s.seed_policy.uses_seed && s.seed_policy.pr_seed.is_none() {
            return fail(
                "seed_policy.pr_seed",
                format!("{id} uses a seed and must record pr_seed (BR1.4)"),
            );
        }
        if s.inputs.is_empty() {
            return fail(
                "inputs",
                format!("{id} is implemented and must name its inputs (BR1.4)"),
            );
        }
        if s.nightly_group.is_none() {
            return fail(
                "nightly_group",
                format!("{id} is implemented and must name its nightly group (NFR9.17)"),
            );
        }
        if s.build == Build::Sanitizer && s.sanitizer_rustflags.as_deref().is_none_or(str::is_empty)
        {
            return fail(
                "sanitizer_rustflags",
                format!(
                    "{id} records build = \"sanitizer\" and must record sanitizer_rustflags (BR3.6)"
                ),
            );
        }
        let pairs = [
            (pr, nightly, "pr > nightly"),
            (nightly, &s.volumes.full, "nightly > full"),
        ];
        for (lower, higher, name) in pairs {
            if lower.len() != higher.len() {
                return fail(
                    "volumes",
                    format!("{id}: every level must record the same entries ({name}) (BR1.5)"),
                );
            }
            if let Some(i) = (0..lower.len()).find(|i| higher[*i].is_lower_than(&lower[*i])) {
                return fail("volumes", format!("{id}: {name} in entry {i} (BR1.5)"));
            }
        }
        Ok(())
    }

    /// Checks what depends on the repository at `root` and on the
    /// registered suites: input paths exist, generators are registered by
    /// their suite (BR1.6), and implementations agree with statuses (BR6.1).
    pub fn check_environment(&self, root: &Path, registry: &Registry) -> Result<(), SettingsError> {
        for s in &self.suites {
            let registered = registry.get(s.id());
            let mismatch = match (s.status, registered.is_some()) {
                (Status::Implemented, false) => {
                    "is implemented but no implementation is registered"
                }
                (Status::Pending, true) => {
                    "has a registered implementation but the settings say pending"
                }
                _ => "",
            };
            if !mismatch.is_empty() {
                return Err(self.error(
                    Some(s),
                    "status",
                    format!("{} {mismatch} (BR6.1)", s.id()),
                ));
            }
            for input in &s.inputs {
                let problem = match input {
                    InputSource::Generator { generator } => {
                        let known = registered
                            .is_some_and(|suite| suite.generators().contains(&generator.as_str()));
                        (!known).then(|| format!("{input} is not registered by the suite"))
                    }
                    InputSource::Corpus { path } | InputSource::Fixed { path } => {
                        (!root.join(path).exists()).then(|| format!("path {path:?} does not exist"))
                    }
                };
                if let Some(problem) = problem {
                    return Err(self.error(
                        Some(s),
                        "inputs",
                        format!("{}: {problem} (BR1.6)", s.id()),
                    ));
                }
            }
        }
        Ok(())
    }
}

fn level_key(level: Level) -> String {
    match level {
        Level::Release => "volumes.full".to_string(),
        other => format!("volumes.{other}"),
    }
}

fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
}

/// Reads `schema_version` before the full parse, so a newer file is
/// reported as a schema problem rather than as unknown keys (BR1.1).
pub fn check_schema_version(text: &str, file: &str) -> Result<(), SettingsError> {
    let Ok(table) = toml::from_str::<toml::Table>(text) else {
        // Not TOML at all: the full parse reports it with its line.
        return Ok(());
    };
    let found = table
        .get("schema_version")
        .and_then(toml::Value::as_integer);
    if found == Some(i64::from(SCHEMA_VERSION)) {
        return Ok(());
    }
    Err(SettingsError {
        file: file.to_string(),
        line: None,
        key: "schema_version".to_string(),
        message: match found {
            Some(v) => format!(
                "schema version {v} is not supported; this harness reads version {SCHEMA_VERSION}"
            ),
            None => "schema_version is missing or not an integer".to_string(),
        },
    })
}

/// The `schema_version` of settings text, if it is TOML with an integer one.
pub fn raw_schema_version(text: &str) -> Option<i64> {
    let table: toml::Table = toml::from_str(text).ok()?;
    table.get("schema_version")?.as_integer()
}

impl Settings {
    /// Reads and validates the settings file at `path`; `file` names it in
    /// errors (BR1.1).
    pub fn load(path: &Path, file: &str) -> Result<Settings, SettingsError> {
        let text = std::fs::read_to_string(path).map_err(|e| SettingsError {
            file: file.to_string(),
            line: None,
            key: "(file)".to_string(),
            message: format!("cannot read the settings file: {e}"),
        })?;
        Settings::parse(&text, file)
    }
}
