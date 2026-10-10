//! Shared helpers for the acceptance harness tests: fixture paths and fake
//! suites registered only in tests.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rail_testkit::harness::process::{Proc, RunContext, RunError};
use rail_testkit::harness::settings::{Driver, SuiteId, Volume};
use rail_testkit::suites::{Input, Observation, Suite};

/// A fixture under `tests/data/harness/`.
pub fn data(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/data/harness")
        .join(name)
}

/// The valid 15-suite settings fixture.
pub fn valid_text() -> String {
    std::fs::read_to_string(data("valid.toml")).expect("valid.toml")
}

/// The test-only stand-in for `rail`.
pub fn fake_rail() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_fake-rail"))
}

/// What a fake suite does.
#[derive(Clone)]
pub enum Behaviour {
    /// Every input passes.
    Pass,
    /// The input at this index fails the judge.
    FailAt(usize),
    /// `run` returns a harness error for the input at this index.
    ErrorAt(usize),
    /// Each input runs `fake-rail` with these arguments.
    Rail(Vec<&'static str>),
    /// Inputs differ between two calls with the same seed.
    Nondeterministic,
    /// Fails on its first call to `judge`, passes afterwards.
    FlakyOnce,
}

/// A suite implementation for tests, with a shared call log.
#[derive(Clone)]
pub struct FakeSuite {
    pub id: SuiteId,
    pub behaviour: Behaviour,
    pub log: Arc<Mutex<Vec<String>>>,
    pub generators: Vec<&'static str>,
    calls: Arc<AtomicUsize>,
}

impl FakeSuite {
    pub fn new(id: SuiteId, behaviour: Behaviour) -> FakeSuite {
        FakeSuite {
            id,
            behaviour,
            log: Arc::new(Mutex::new(Vec::new())),
            generators: Vec::new(),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn passing(id: SuiteId) -> FakeSuite {
        FakeSuite::new(id, Behaviour::Pass)
    }

    pub fn with_log(mut self, log: Arc<Mutex<Vec<String>>>) -> FakeSuite {
        self.log = log;
        self
    }

    pub fn entries(&self) -> Vec<String> {
        self.log.lock().unwrap().clone()
    }

    fn note(&self, entry: String) {
        self.log.lock().unwrap().push(entry);
    }
}

fn count(volume: &[Volume]) -> u64 {
    volume
        .iter()
        .map(|v| match v {
            Volume::Count { count } => *count,
            Volume::Exhaustive { .. } => 3,
            Volume::NotRun { .. } => 0,
        })
        .sum()
}

impl Suite for FakeSuite {
    fn id(&self) -> SuiteId {
        self.id
    }

    fn generators(&self) -> Vec<&'static str> {
        self.generators.clone()
    }

    fn generate_inputs(&self, seed: u64, volume: &[Volume]) -> Result<Vec<Input>, String> {
        let n = count(volume);
        self.note(format!("generate {} seed {seed} count {n}", self.id));
        let salt = if matches!(self.behaviour, Behaviour::Nondeterministic) {
            self.calls.fetch_add(1, Ordering::SeqCst) as u64
        } else {
            0
        };
        Ok((0..n)
            .map(|i| Input::new(format!("input {i}"), format!("{seed}-{i}-{salt}")))
            .collect())
    }

    fn run(&self, ctx: &RunContext, input: &Input) -> Result<Observation, RunError> {
        let index: usize = input
            .label
            .trim_start_matches("input ")
            .parse()
            .unwrap_or(0);
        match &self.behaviour {
            Behaviour::ErrorAt(i) if *i == index => {
                Err(RunError::Suite(format!("cannot observe {}", input.label)))
            }
            Behaviour::Rail(args) => {
                let proc = Proc::new(fake_rail())
                    .args(args.iter().copied())
                    .cwd(ctx.work_dir());
                let out = ctx.run(&proc)?;
                Ok(Observation::from_process(Driver::Cli, out))
            }
            _ => Ok(Observation::new(Driver::Cli)),
        }
    }

    fn judge(&self, inputs: &[Input], _observations: &[Observation]) -> Vec<Option<String>> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        self.note(format!("judge {} call {call}", self.id));
        (0..inputs.len())
            .map(|i| match self.behaviour {
                Behaviour::FailAt(at) if at == i => Some(format!("input {i} is wrong")),
                Behaviour::FlakyOnce if call == 0 && i == 0 => Some("flaky".to_string()),
                _ => None,
            })
            .collect()
    }
}
