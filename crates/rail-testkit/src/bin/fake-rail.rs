//! Test helper: a stand-in for `rail` used by the acceptance harness's own
//! tests (process runner, deadlines, environment allow-list, group kills).
//! Its behaviour is chosen by its first argument:
//!
//! - `print <text>`: prints `<text>` and a newline, exits 0.
//! - `fail <code>`: prints `failing` to standard error, exits with `<code>`.
//! - `sleep <seconds>`: sleeps, then exits 0.
//! - `grandchild <file>`: starts `fake-rail sleep 300` as its own child,
//!   writes that child's process ID to `<file>`, then sleeps.
//! - `env`: prints its environment as sorted `NAME=value` lines.
//! - `pwd`: prints its current directory.
//! - `touch <file>`: creates `<file>` (relative to the current directory).

#![forbid(unsafe_code)]

use std::process::{Command, ExitCode};
use std::time::Duration;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let arg = |i: usize| args.get(i).cloned().unwrap_or_default();
    match arg(0).as_str() {
        "print" => {
            println!("{}", arg(1));
            ExitCode::SUCCESS
        }
        "fail" => {
            eprintln!("failing");
            ExitCode::from(arg(1).parse::<u8>().unwrap_or(1))
        }
        "sleep" => {
            std::thread::sleep(Duration::from_secs(arg(1).parse().unwrap_or(1)));
            ExitCode::SUCCESS
        }
        "grandchild" => {
            let me = std::env::current_exe().expect("own path");
            let mut child = Command::new(me)
                .args(["sleep", "300"])
                .spawn()
                .expect("start grandchild");
            std::fs::write(arg(1), child.id().to_string()).expect("write pid file");
            // Waits as long as the grandchild sleeps (300 s).
            let _ = child.wait();
            ExitCode::SUCCESS
        }
        "env" => {
            let mut vars: Vec<(String, String)> = std::env::vars().collect();
            vars.sort();
            for (name, value) in vars {
                println!("{name}={value}");
            }
            ExitCode::SUCCESS
        }
        "pwd" => {
            let dir = std::env::current_dir().expect("current directory");
            println!("{}", dir.display());
            ExitCode::SUCCESS
        }
        "touch" => {
            std::fs::write(arg(1), b"").expect("create file");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("fake-rail: unknown behaviour {other:?}");
            ExitCode::from(2)
        }
    }
}
