//! Tests of the test tooling itself: golden files and the process runner.

use std::process::Command;
use std::time::Duration;

use rail_testkit::{TempDir, check_golden, frame, run_process, run_process_with_deadline};

#[test]
fn golden_compare_passes_on_equal_and_fails_on_different_output() {
    let dir = TempDir::new("golden-compare");
    let path = dir.path().join("out.txt");
    std::fs::write(&path, "expected\n").unwrap();
    assert!(check_golden(&path, "expected\n", false).is_ok());
    let err = check_golden(&path, "actual\n", false).unwrap_err();
    assert!(err.contains("RAIL_BLESS=1"), "{err}");
    // Without the bless switch the file is never rewritten.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "expected\n");
}

#[test]
fn golden_bless_switch_rewrites_and_missing_file_fails() {
    let dir = TempDir::new("golden-bless");
    let path = dir.path().join("nested/new.txt");
    assert!(check_golden(&path, "one\n", false).is_err());
    check_golden(&path, "one\n", true).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "one\n");
    assert!(check_golden(&path, "one\n", false).is_ok());
}

#[test]
fn process_runner_kills_a_process_past_its_deadline() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_never-ends"));
    let err = run_process_with_deadline(&mut cmd, b"", Duration::from_millis(300)).unwrap_err();
    assert!(err.command.ends_with("never-ends"), "{}", err.command);
    assert!(err.partial.signal.is_some());
}

#[test]
fn process_runner_captures_exit_code_and_streams() {
    let mut cmd = Command::new("/bin/sh");
    cmd.args(["-c", "cat; echo err >&2; exit 3"]);
    let out = run_process(&mut cmd, b"in\n");
    assert_eq!(out.code, Some(3));
    assert_eq!(out.stdout_str(), "in\n");
    assert_eq!(out.stderr_str(), "err\n");
}

#[test]
fn process_runner_reports_a_signal() {
    let out = run_process(&mut Command::new(env!("CARGO_BIN_EXE_aborts")), b"");
    assert_eq!(out.code, None);
    assert_eq!(out.signal, Some(6));
}

#[test]
fn frame_prefixes_the_content_length() {
    assert_eq!(frame(b"{}"), b"Content-Length: 2\r\n\r\n{}".to_vec());
}
