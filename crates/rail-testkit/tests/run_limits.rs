//! The run operation's limits, driven through the same child-process code
//! path `rail` uses for built programs (NFR5.5, NFR9.9).

use std::path::Path;
use std::time::{Duration, Instant};

use rail_tools::{ErrorCode, RunLimits, run_executable};

fn limits(time_ms: u64, output_bytes: usize) -> RunLimits {
    RunLimits {
        time: Duration::from_millis(time_ms),
        output_bytes,
    }
}

#[test]
fn a_program_past_the_time_limit_is_killed() {
    let start = Instant::now();
    let err = run_executable(
        Path::new(env!("CARGO_BIN_EXE_never-ends")),
        &limits(300, 1024),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::RunFailed);
    assert_eq!(err.message, "time limit exceeded");
    assert!(start.elapsed() < Duration::from_secs(5));
}

#[test]
fn a_program_past_the_output_cap_is_killed() {
    let err = run_executable(
        Path::new(env!("CARGO_BIN_EXE_floods-output")),
        &limits(20_000, 1 << 20),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::RunFailed);
    assert_eq!(err.message, "output limit exceeded");
}

#[test]
fn a_program_ended_by_a_signal_is_run_failed() {
    let err = run_executable(
        Path::new(env!("CARGO_BIN_EXE_aborts")),
        &RunLimits::default(),
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::RunFailed);
    assert_eq!(err.message, "ended by signal SIGABRT");
}

#[test]
fn a_program_that_cannot_start_is_run_failed() {
    let err = run_executable(Path::new("/nonexistent/program"), &RunLimits::default()).unwrap_err();
    assert_eq!(err.code, ErrorCode::RunFailed);
    assert!(
        err.message.starts_with("could not start"),
        "{}",
        err.message
    );
}
