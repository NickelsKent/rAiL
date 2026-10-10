//! The sandbox probe: a tiny stand-in that tries one operation and reports
//! whether the operating system allowed it. The sandbox self-test runs it
//! under the sandbox (`corpus-check --sandbox-probe <op> <arg>`).
//!
//! Exit codes: 0 the operation succeeded, 3 the operating system refused it,
//! 2 bad usage.

use std::fs;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Runs the probe for `args` (the words after `--sandbox-probe`) and returns
/// its exit code.
pub fn run_probe(args: &[String]) -> i32 {
    let [op, arg] = args else {
        return 2;
    };
    let allowed = match op.as_str() {
        "write" => fs::write(arg, b"probe\n").is_ok(),
        "read" => fs::read(arg).is_ok(),
        "read-dir" => fs::read_dir(arg).is_ok(),
        "connect" => match arg.parse::<SocketAddr>() {
            Ok(addr) => TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok(),
            Err(_) => return 2,
        },
        _ => return 2,
    };
    if allowed { 0 } else { 3 }
}
