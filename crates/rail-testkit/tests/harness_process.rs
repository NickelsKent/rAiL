//! Acceptance harness tests: the process runner for suites. Scrubbed
//! environment (NFR5.7), per-run directories (NFR5.8), deadlines and
//! process-group kills (BR5.2), and the stop flag (NFR9.18).

mod common;

use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use rail_testkit::harness::process::{Proc, RunContext, RunDir, RunError, StopSignal};
use rail_testkit::harness::settings::SuiteId;
use rail_testkit::{repo_root, run_process_with_deadline};

use common::fake_rail;

fn context(dir: &RunDir, deadline: Duration, stop: &StopSignal) -> RunContext {
    RunContext::new(dir, &repo_root(), &fake_rail(), deadline, stop.clone())
}

fn alive(pid: &str) -> bool {
    Command::new("/bin/kill")
        .args(["-0", pid])
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Waits up to `limit` for `pid` to disappear.
fn gone_within(pid: &str, limit: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < limit {
        if !alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Runs `fake-rail env` through the runner and prints what it saw. Run on
/// its own (ignored) by `nfr5_7_...`, which starts this test binary with a
/// secret in its environment.
#[test]
#[ignore = "helper started by nfr5_7_rail_sees_only_the_allow_listed_environment"]
fn helper_print_runner_environment() {
    let dir = RunDir::create(SuiteId::S0).unwrap();
    let stop = StopSignal::new();
    let ctx = context(&dir, Duration::from_secs(30), &stop);
    let secret_on_host = std::env::var("RAIL_TEST_SECRET").is_ok();
    let out = ctx
        .run(
            &Proc::new(fake_rail())
                .arg("env")
                .env("RAIL_THREADS", "1")
                .cwd(dir.work()),
        )
        .unwrap();
    println!("HOST_SECRET={secret_on_host}");
    println!("RUN_DIR={}", dir.path().display());
    print!("{}", String::from_utf8_lossy(&out.stdout));
}

#[test]
fn nfr5_7_rail_sees_only_the_allow_listed_environment() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "helper_print_runner_environment",
            "--exact",
            "--include-ignored",
            "--nocapture",
            "--test-threads",
            "1",
        ])
        .env("RAIL_TEST_SECRET", "do-not-pass-me-on")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(text.contains("HOST_SECRET=true"), "{text}");
    assert!(!text.contains("do-not-pass-me-on"), "{text}");
    let run_dir = text
        .lines()
        .find_map(|l| l.strip_prefix("RUN_DIR="))
        .expect("run dir line")
        .to_string();
    let names: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(name, _)| name)
        .filter(|n| {
            *n != "HOST_SECRET"
                && *n != "RUN_DIR"
                && n.chars().all(|c| c.is_ascii_uppercase() || c == '_')
        })
        .collect();
    assert_eq!(
        names,
        ["HOME", "LANG", "PATH", "RAIL_THREADS", "TMPDIR", "TZ"],
        "{text}"
    );
    assert!(text.contains("LANG=C.UTF-8\n"), "{text}");
    assert!(text.contains("TZ=UTC\n"), "{text}");
    assert!(text.contains("RAIL_THREADS=1\n"), "{text}");
    assert!(text.contains(&format!("HOME={run_dir}/home\n")), "{text}");
    assert!(text.contains(&format!("TMPDIR={run_dir}/tmp\n")), "{text}");
}

#[test]
fn nfr5_8_each_run_gets_fresh_directories_and_runs_in_work() {
    let a = RunDir::create(SuiteId::S0).unwrap();
    let b = RunDir::create(SuiteId::S0).unwrap();
    assert_ne!(a.path(), b.path());
    for dir in [&a, &b] {
        for sub in [dir.home(), dir.tmp(), dir.work()] {
            assert!(sub.is_dir(), "{}", sub.display());
            assert!(sub.starts_with(dir.path()));
        }
    }
    let stop = StopSignal::new();
    let ctx = context(&a, Duration::from_secs(30), &stop);
    let out = ctx
        .run(&Proc::new(fake_rail()).arg("pwd").cwd(a.work()))
        .unwrap();
    let pwd = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(
        Path::new(&pwd).canonicalize().unwrap(),
        a.work().canonicalize().unwrap()
    );
    assert!(!a.path().starts_with(repo_root()));
}

#[test]
fn nfr5_8_a_run_directory_is_removed_after_a_pass_and_kept_after_a_failure() {
    let passed = RunDir::create(SuiteId::S0).unwrap();
    let path = passed.path().to_path_buf();
    drop(passed);
    assert!(!path.exists());

    let failed = RunDir::create(SuiteId::S0).unwrap();
    std::fs::write(failed.work().join("evidence"), b"x").unwrap();
    let kept = failed.keep();
    assert!(kept.join("work/evidence").is_file());
    std::fs::remove_dir_all(kept).unwrap();
}

#[test]
fn br5_2_a_process_past_its_deadline_is_killed_with_its_grandchild() {
    let dir = RunDir::create(SuiteId::S0).unwrap();
    let stop = StopSignal::new();
    let ctx = context(&dir, Duration::from_millis(800), &stop);
    let start = Instant::now();
    let out = ctx
        .run(
            &Proc::new(fake_rail())
                .args(["grandchild", "pid"])
                .cwd(dir.work()),
        )
        .unwrap();
    assert!(out.timed_out, "{out:?}");
    assert!(start.elapsed() < Duration::from_secs(10));
    let pid = std::fs::read_to_string(dir.work().join("pid")).unwrap();
    assert!(
        gone_within(pid.trim(), Duration::from_secs(5)),
        "grandchild {pid} survived"
    );
}

#[test]
fn a_process_within_its_deadline_reports_its_exit() {
    let dir = RunDir::create(SuiteId::S0).unwrap();
    let stop = StopSignal::new();
    let ctx = context(&dir, Duration::from_secs(30), &stop);
    let out = ctx
        .run(&Proc::new(fake_rail()).args(["fail", "3"]).cwd(dir.work()))
        .unwrap();
    assert_eq!(out.code, Some(3));
    assert!(!out.timed_out);
    assert_eq!(String::from_utf8_lossy(&out.stderr), "failing\n");
    // A program that cannot start is a typed error, not a panic.
    let err = ctx
        .run(&Proc::new("/nonexistent/rail").cwd(dir.work()))
        .unwrap_err();
    assert!(matches!(err, RunError::Spawn(_)), "{err:?}");
}

#[test]
fn nfr9_18_a_stop_kills_the_running_process_and_refuses_new_ones() {
    let dir = RunDir::create(SuiteId::S0).unwrap();
    let stop = StopSignal::new();
    let ctx = context(&dir, Duration::from_secs(60), &stop);
    let stopper = stop.clone();
    let timer = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        stopper.stop();
    });
    let start = Instant::now();
    let err = ctx
        .run(
            &Proc::new(fake_rail())
                .args(["grandchild", "pid"])
                .cwd(dir.work()),
        )
        .unwrap_err();
    timer.join().unwrap();
    assert_eq!(err, RunError::Stopped);
    assert!(start.elapsed() < Duration::from_secs(10));
    let pid = std::fs::read_to_string(dir.work().join("pid")).unwrap();
    assert!(gone_within(pid.trim(), Duration::from_secs(5)));
    let refused = ctx
        .run(&Proc::new(fake_rail()).args(["print", "x"]))
        .unwrap_err();
    assert_eq!(refused, RunError::Stopped);
}

#[test]
fn the_testkit_deadline_runner_also_kills_the_whole_group() {
    let dir = rail_testkit::TempDir::new("group-kill");
    let mut cmd = Command::new(fake_rail());
    cmd.args(["grandchild", "pid"]).current_dir(dir.path());
    let err = run_process_with_deadline(&mut cmd, b"", Duration::from_millis(800)).unwrap_err();
    assert!(err.partial.signal.is_some());
    let pid = std::fs::read_to_string(dir.path().join("pid")).unwrap();
    assert!(
        gone_within(pid.trim(), Duration::from_secs(5)),
        "grandchild {pid} survived"
    );
}
