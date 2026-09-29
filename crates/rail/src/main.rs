//! The `rail` command line: `parse`, `check`, `build`, `run` and `rap`.
//!
//! Building block: Cli (contract E2). Owning unit: U1 walking-skeleton (thin
//! first version; completed by U5 agent-loop).
//!
//! `rail <parse|check|build|run> [--json] <module>` runs one operation in the
//! current directory's workspace; `--json` prints exactly the protocol result
//! (BR6.1). `rail rap` serves the agent protocol on standard input and output.
//! Exit codes follow E2: 0 success, 1 blocking diagnostics, 2 usage error, 3
//! tool failure; `run` exits with the program's own code (BR6.2). This binary
//! is the only code that writes to the standard streams (NFR9.12).

#![forbid(unsafe_code)]

use std::io::Write;
use std::process::ExitCode;

use rail_json::Value;
use rail_tools::{
    ErrorCode, LogLevel, Operation, Outcome, Record, ToolError, Workspace, perform_logged,
};

const USAGE: &str = "usage: rail <parse|check|build|run> [--json] <module>\n       rail rap\n";

fn main() -> ExitCode {
    rail_tools::install_quiet_panic_hook();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = run(&args, log_level());
    ExitCode::from(u8::try_from(code).unwrap_or(3))
}

/// The configured `RAIL_LOG` level; off when unset (NFR9.13).
fn log_level() -> Option<LogLevel> {
    let value = std::env::var("RAIL_LOG").ok()?;
    if value.is_empty() {
        return None;
    }
    let level = LogLevel::parse(&value);
    if level.is_none() {
        eprintln!("rail warning RAIL_LOG must be error, info or debug; logging is off");
    }
    level
}

fn logger(level: Option<LogLevel>) -> impl FnMut(Record) {
    move |record: Record| {
        if level.is_some_and(|l| record.level <= l) {
            eprintln!("{}", record.line());
        }
    }
}

fn usage() -> i32 {
    eprint!("{USAGE}");
    2
}

fn run(args: &[String], level: Option<LogLevel>) -> i32 {
    let Some((command, rest)) = args.split_first() else {
        return usage();
    };
    if command == "rap" {
        if !rest.is_empty() {
            return usage();
        }
        let mut log = logger(level);
        let stdin = std::io::stdin().lock();
        let stdout = std::io::stdout().lock();
        return match rail_rap::serve(stdin, stdout, &mut log) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("error: protocol output failed: {}", e.kind());
                3
            }
        };
    }
    let Some(op) = Operation::from_command(command) else {
        return usage();
    };
    let json = rest.iter().any(|a| a == "--json");
    let positional: Vec<&String> = rest.iter().filter(|a| *a != "--json").collect();
    if positional.len() != 1 || positional[0].starts_with("--") {
        return usage();
    }
    let module = positional[0].as_str();
    let workspace = match Workspace::open(std::path::Path::new(".")) {
        Ok(ws) => ws,
        Err(e) => return report_error(&e, json),
    };
    match perform_logged(&workspace, op, module, &mut logger(level)) {
        Ok(outcome) if json => {
            print_line(&rail_json::to_string(&outcome.to_json()));
            exit_code(&outcome)
        }
        Ok(outcome) => human(&outcome),
        Err(e) => report_error(&e, json),
    }
}

fn print_line(text: &str) {
    let mut out = std::io::stdout().lock();
    let _ = writeln!(out, "{text}");
    let _ = out.flush();
}

/// The exit code of a successful operation (E2, E7).
fn exit_code(outcome: &Outcome) -> i32 {
    match outcome {
        Outcome::Parsed(p) => i32::from(!p.diagnostics.is_empty()),
        Outcome::Checked(c) => i32::from(c.blocking),
        Outcome::Built(_) => 0,
        Outcome::Ran(r) => r.exit_code,
    }
}

fn human(outcome: &Outcome) -> i32 {
    match outcome {
        Outcome::Parsed(p) => match &p.tree {
            Some(tree) => print_line(&format!(
                "{}: parsed, {} definitions",
                p.module,
                tree.fns.len()
            )),
            None => p.diagnostics.iter().for_each(|d| print_line(&d.render())),
        },
        Outcome::Checked(c) if c.diagnostics.is_empty() => {
            print_line(&format!("{}: no diagnostics", c.module));
        }
        Outcome::Checked(c) => c.diagnostics.iter().for_each(|d| print_line(&d.render())),
        Outcome::Built(a) => print_line(&format!("{}: built {}", a.module, a.path)),
        Outcome::Ran(r) => {
            let mut out = std::io::stdout().lock();
            let _ = out.write_all(r.stdout.as_bytes());
            let _ = out.flush();
            let mut err = std::io::stderr().lock();
            let _ = err.write_all(r.stderr.as_bytes());
            let _ = err.flush();
        }
    }
    exit_code(outcome)
}

fn report_error(e: &ToolError, json: bool) -> i32 {
    if json {
        print_line(&rail_json::to_string(&Value::object([(
            "error",
            e.to_json(),
        )])));
    } else {
        if e.code == ErrorCode::BuildBlocked
            && let Some(result) = &e.data
        {
            for d in result
                .get("diagnostics")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                print_line(&render_diagnostic(d));
            }
        }
        eprintln!("error: {}: {}", e.code.as_str(), e.message);
    }
    e.code.exit_code()
}

/// The human line of a diagnostic carried as JSON in a ToolError.
fn render_diagnostic(d: &Value) -> String {
    let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or("").to_string();
    let loc = d.get("loc");
    let span = loc.and_then(|l| l.get("span")).and_then(Value::as_array);
    let at = |i: usize| {
        span.and_then(|s| s.get(i))
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    format!(
        "error {} {} {}/{} [{},{}]: {}",
        text(d.get("rule")),
        text(loc.and_then(|l| l.get("module"))),
        text(loc.and_then(|l| l.get("def"))),
        text(loc.and_then(|l| l.get("path"))),
        at(0),
        at(1),
        text(d.get("message"))
    )
}
