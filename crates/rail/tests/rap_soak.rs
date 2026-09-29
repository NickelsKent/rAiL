//! A long protocol session does not grow (NFR9.5): resident memory after
//! 1,000 `check.run` requests stays within 10% or 1 MiB of the reading
//! after the first 100.

use std::path::Path;
use std::process::Command;

use rail_testkit::{RapClient, TempDir, copy_dir, repo_root};

const RAIL: &str = env!("CARGO_BIN_EXE_rail");

/// Resident set size of `pid` in KiB.
fn rss_kib(pid: u32) -> u64 {
    if cfg!(target_os = "linux") {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).expect("proc status");
        let line = status
            .lines()
            .find(|l| l.starts_with("VmRSS:"))
            .expect("VmRSS");
        line.split_whitespace()
            .nth(1)
            .and_then(|n| n.parse().ok())
            .expect("VmRSS value")
    } else {
        let out = Command::new("ps")
            .args(["-o", "rss=", "-p", &pid.to_string()])
            .output()
            .expect("ps");
        String::from_utf8_lossy(&out.stdout)
            .trim()
            .parse()
            .expect("rss")
    }
}

#[test]
fn a_thousand_requests_do_not_grow_the_server() {
    let ws = TempDir::new("rap-soak");
    copy_dir(&repo_root().join("fixtures"), ws.path()).unwrap();
    let mut client = RapClient::spawn(Path::new(RAIL), ws.path());
    let root = ws
        .path()
        .display()
        .to_string()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    client.send(&format!(
        r#"{{"jsonrpc":"2.0","id":0,"method":"initialize","params":{{"client_versions":["0.1"],"workspace_root":"{root}"}}}}"#
    ));
    client.recv();
    let expected_tail =
        r#""result":{"module":"skeleton.answer","diagnostics":[],"blocking":false}}"#;
    let mut check = |id: u32| {
        client.send(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"check.run","params":{{"module":"skeleton/answer.rlc"}}}}"#
        ));
        let response = client.recv();
        assert!(response.ends_with(expected_tail), "{response}");
    };
    for id in 1..=100 {
        check(id);
    }
    let pid = client.pid();
    let after_100 = rss_kib(pid);
    let mut check = |id: u32| {
        client.send(&format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"check.run","params":{{"module":"skeleton/answer.rlc"}}}}"#
        ));
        assert!(client.recv().ends_with(expected_tail));
    };
    for id in 101..=1000 {
        check(id);
    }
    let after_1000 = rss_kib(pid);
    let allowed = (after_100 / 10).max(1024);
    assert!(
        after_1000 <= after_100 + allowed,
        "resident memory grew from {after_100} KiB to {after_1000} KiB"
    );
    let out = client.close();
    assert_eq!(out.code, Some(0));
}
