//! The suite interface and the one static registry (contract C9, BR5.1).
//!
//! Each owning unit adds `suites/<id>.rs`, implements [`Suite`] for it, adds
//! it to [`Registry::production`], and flips its settings entry to
//! `implemented` in the same change (BR6.4).

pub mod s0;

use crate::harness::process::{ProcResult, RunContext, RunError};
use crate::harness::settings::{Driver, SuiteId, Volume};

/// One generated or fixed input, in generation order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Input {
    /// A short name for the input (for example a step or a file name).
    pub label: String,
    /// The input itself.
    pub data: Vec<u8>,
}

impl Input {
    /// An input from its label and bytes.
    pub fn new(label: impl Into<String>, data: impl Into<Vec<u8>>) -> Input {
        Input {
            label: label.into(),
            data: data.into(),
        }
    }
}

/// What a suite saw from `rail` for one input.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Observation {
    /// How `rail` was driven (`None` when no process was started).
    pub driver: Option<Driver>,
    /// The exit code of the (last) process, when it exited normally.
    pub exit_status: Option<i32>,
    /// The signal that ended it, if any.
    pub signal: Option<i32>,
    /// Captured standard output.
    pub stdout: Vec<u8>,
    /// Captured standard error.
    pub stderr: Vec<u8>,
    /// RAP response bodies, in order.
    pub messages: Vec<String>,
    /// Whether a process ran past its deadline.
    pub timed_out: bool,
    /// What the suite noted while observing (for example a mismatch with a
    /// golden file). The judge decides what they mean.
    pub findings: Vec<String>,
}

impl Observation {
    /// An empty observation made through `driver`.
    pub fn new(driver: Driver) -> Observation {
        Observation {
            driver: Some(driver),
            ..Observation::default()
        }
    }

    /// The observation of one finished process.
    pub fn from_process(driver: Driver, result: ProcResult) -> Observation {
        Observation {
            driver: Some(driver),
            exit_status: result.code,
            signal: result.signal,
            stdout: result.stdout,
            stderr: result.stderr,
            timed_out: result.timed_out,
            ..Observation::default()
        }
    }
}

/// A suite implementation (C9 `suite_interface`).
pub trait Suite: Send + Sync {
    /// The suite it implements.
    fn id(&self) -> SuiteId;

    /// The generator names it registers (BR1.6).
    fn generators(&self) -> Vec<&'static str> {
        Vec::new()
    }

    /// Ordered inputs for `seed` and the level's volume entries. A pure
    /// function of its arguments (BR4.3): no clocks, environment, hash-map
    /// ordering or files beyond the recorded inputs.
    fn generate_inputs(&self, seed: u64, volume: &[Volume]) -> Result<Vec<Input>, String>;

    /// Observes `rail` for one input, through the command line or RAP only
    /// (BR5.1), starting processes only through `ctx`.
    fn run(&self, ctx: &RunContext, input: &Input) -> Result<Observation, RunError>;

    /// Judges every input: `None` when it meets the pass criterion, or the
    /// reason it does not.
    fn judge(&self, inputs: &[Input], observations: &[Observation]) -> Vec<Option<String>>;

    /// The normalized form of one observation written for `--compare`
    /// (cross-platform suites, BR7.7): no paths or platform-specific fields.
    fn normalize(&self, input: &Input, observation: &Observation) -> rail_json::Value {
        rail_json::Value::object([
            ("input", rail_json::Value::str(&input.label)),
            (
                "exit_status",
                observation
                    .exit_status
                    .map_or(rail_json::Value::Null, |c| rail_json::Value::int(c.into())),
            ),
            (
                "stdout",
                rail_json::Value::str(&String::from_utf8_lossy(&observation.stdout)),
            ),
            (
                "stderr",
                rail_json::Value::str(&String::from_utf8_lossy(&observation.stderr)),
            ),
        ])
    }

    /// Whether `rail-accept` prints one PASS or FAIL line per input (S0).
    fn report_each_input(&self) -> bool {
        false
    }

    /// Whether a timed-out process is an expected observation rather than
    /// an `error` (BR5.2).
    fn expects_timeouts(&self) -> bool {
        false
    }
}

/// The registered suite implementations.
pub struct Registry {
    suites: Vec<Box<dyn Suite>>,
}

impl Registry {
    /// A registry of the given implementations (tests register fakes).
    pub fn new(suites: Vec<Box<dyn Suite>>) -> Registry {
        Registry { suites }
    }

    /// The production registry: every implemented suite.
    pub fn production() -> Registry {
        Registry::new(vec![Box::new(s0::SkeletonSmoke)])
    }

    /// The implementation of `id`, if one is registered.
    pub fn get(&self, id: SuiteId) -> Option<&dyn Suite> {
        self.suites
            .iter()
            .find(|s| s.id() == id)
            .map(|s| s.as_ref())
    }
}
