//! The one fixed child program that `process-pipeline` runs (BR6.6). Its
//! behaviour is chosen by its arguments:
//!
//! * `echo <word>...`  prints the words, separated by spaces, on stdout
//! * `stderr <text>`   prints the text on stderr
//! * `exit <status>`   exits with that status

use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "usage: corpus-helper (echo <word>... | stderr <text> | exit <status>)";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("echo") => {
            let _ = writeln!(std::io::stdout(), "{}", args[1..].join(" "));
            ExitCode::SUCCESS
        }
        Some("stderr") if args.len() == 2 => {
            let _ = writeln!(std::io::stderr(), "{}", args[1]);
            ExitCode::SUCCESS
        }
        Some("exit") if args.len() == 2 => match args[1].parse::<u8>() {
            Ok(status) => ExitCode::from(status),
            Err(_) => usage(),
        },
        _ => usage(),
    }
}

fn usage() -> ExitCode {
    let _ = writeln!(std::io::stderr(), "{USAGE}");
    ExitCode::from(2)
}
