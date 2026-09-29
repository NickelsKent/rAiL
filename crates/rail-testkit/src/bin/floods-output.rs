//! Test helper: writes to standard output without end (tests the output cap).

#![forbid(unsafe_code)]

use std::io::Write;

fn main() {
    let chunk = [b'x'; 65536];
    let mut out = std::io::stdout().lock();
    while out.write_all(&chunk).is_ok() {}
}
