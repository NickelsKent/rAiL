//! Starting processes in their own process group, with a deadline and an
//! output limit (NFR9.19, the run contract).

mod common;

use common::{TempDir, build_standin};
use corpus_check::proc::run_command;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn captures_output_exit_code_and_feeds_stdin() {
    let dir = TempDir::new("proc-ok");
    let (exe, _) = build_standin(dir.path());
    let mut cmd = Command::new(&exe);
    cmd.arg("stdin-upper");
    let out = run_command(cmd, b"abc\n", Duration::from_secs(30)).unwrap();
    assert_eq!(out.stdout, b"ABC\n");
    assert_eq!(out.code, Some(0));
    assert!(!out.timed_out && !out.output_truncated);

    let mut cmd = Command::new(&exe);
    cmd.arg("bogus");
    let out = run_command(cmd, b"", Duration::from_secs(30)).unwrap();
    assert_eq!(out.code, Some(9));
}

#[test]
fn a_program_past_its_deadline_is_killed() {
    let dir = TempDir::new("proc-deadline");
    let (exe, _) = build_standin(dir.path());
    let mut cmd = Command::new(&exe);
    cmd.arg("hang");
    let start = Instant::now();
    let out = run_command(cmd, b"", Duration::from_millis(300)).unwrap();
    assert!(out.timed_out, "{out:?}");
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "took {:?}",
        start.elapsed()
    );
}

fn alive(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn the_whole_process_group_is_killed_at_the_deadline() {
    let dir = TempDir::new("proc-group");
    let (exe, _) = build_standin(dir.path());
    let mut cmd = Command::new(&exe);
    cmd.arg("fork-hang");
    let out = run_command(cmd, b"", Duration::from_millis(1500)).unwrap();
    assert!(out.timed_out, "{out:?}");
    let text = String::from_utf8(out.stdout).unwrap();
    let pid: u32 = text
        .trim()
        .strip_prefix("child=")
        .unwrap_or_else(|| panic!("no child pid in {text:?}"))
        .parse()
        .unwrap();
    let gone = (0..40).any(|_| {
        if alive(pid) {
            std::thread::sleep(Duration::from_millis(50));
            false
        } else {
            true
        }
    });
    assert!(gone, "the grandchild {pid} outlived its process group");
}

#[test]
fn runaway_output_is_cut_off_and_the_program_stopped() {
    let dir = TempDir::new("proc-flood");
    let (exe, _) = build_standin(dir.path());
    let mut cmd = Command::new(&exe);
    cmd.arg("flood");
    let start = Instant::now();
    let out = run_command(cmd, b"", Duration::from_secs(60)).unwrap();
    assert!(
        out.output_truncated,
        "stdout was {} bytes",
        out.stdout.len()
    );
    assert!(start.elapsed() < Duration::from_secs(30));
}
