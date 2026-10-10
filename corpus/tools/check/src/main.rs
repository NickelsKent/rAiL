use corpus_check::cli::{USAGE, parse_args, run};
use corpus_check::probe::run_probe;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The sandbox self-test starts this program as its probe.
    if args.first().map(String::as_str) == Some("--sandbox-probe") {
        return ExitCode::from(u8::try_from(run_probe(&args[1..])).unwrap_or(2));
    }
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let env_bless = std::env::var("RAIL_BLESS").is_ok_and(|v| v == "1");
    match parse_args(&args, env_bless) {
        Ok(options) => {
            let code = run(&options, &mut std::io::stdout(), &mut std::io::stderr());
            ExitCode::from(u8::try_from(code).unwrap_or(2))
        }
        Err(message) => {
            eprintln!("error: {message}");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
