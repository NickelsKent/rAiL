//! The `rail` command line: exit codes, `--json`, logging (BR6.1, BR6.2,
//! NFR3.5, NFR9.10, NFR9.12, NFR9.13).

use std::fs;
use std::path::Path;
use std::process::Command;

use rail_testkit::{ProcessOutput, TempDir, copy_dir, repo_root, run_process};

const RAIL: &str = env!("CARGO_BIN_EXE_rail");

fn workspace(label: &str) -> TempDir {
    let dir = TempDir::new(label);
    copy_dir(&repo_root().join("fixtures"), dir.path()).unwrap();
    dir
}

fn rail_env(ws: &Path, args: &[&str], log: Option<&str>) -> ProcessOutput {
    let mut cmd = Command::new(RAIL);
    cmd.args(args).current_dir(ws).env_remove("RAIL_LOG");
    if let Some(level) = log {
        cmd.env("RAIL_LOG", level);
    }
    run_process(&mut cmd, b"")
}

fn rail(ws: &Path, args: &[&str]) -> ProcessOutput {
    rail_env(ws, args, None)
}

fn write(ws: &TempDir, path: &str, text: &str) {
    fs::write(ws.path().join(path), text).unwrap();
}

#[test]
fn successful_commands_exit_0_with_human_output() {
    let ws = workspace("cli-ok");
    let parse = rail(ws.path(), &["parse", "skeleton/answer.rlc"]);
    assert_eq!(
        (parse.code, parse.stdout_str().as_str()),
        (Some(0), "skeleton.answer: parsed, 2 definitions\n")
    );
    let check = rail(ws.path(), &["check", "skeleton/answer.rlc"]);
    assert_eq!(
        (check.code, check.stdout_str().as_str()),
        (Some(0), "skeleton.answer: no diagnostics\n")
    );
    let build = rail(ws.path(), &["build", "skeleton/answer.rlc"]);
    assert_eq!(
        (build.code, build.stdout_str().as_str()),
        (
            Some(0),
            "skeleton.answer: built .rail/build/dev/skeleton/answer\n"
        )
    );
    for out in [&parse, &check, &build] {
        assert_eq!(out.stderr_str(), "", "logging is off by default");
    }
}

#[test]
fn blocking_diagnostics_exit_1() {
    let ws = workspace("cli-blocked");
    write(&ws, "bad.rlc", "(mod bad)\n(use a.b)\n");
    let parse = rail(ws.path(), &["parse", "bad.rlc"]);
    assert_eq!(parse.code, Some(1));
    assert!(
        parse
            .stdout_str()
            .starts_with("error SKL001 bad #000000/ [10,11]: item `use`"),
        "{parse:?}"
    );
    let parse_json = rail(ws.path(), &["parse", "--json", "bad.rlc"]);
    assert_eq!(parse_json.code, Some(1));
    assert!(
        parse_json
            .stdout_str()
            .starts_with(r#"{"module":"bad","tree":null,"diagnostics":[{"rule":"SKL001""#)
    );
    let build = rail(ws.path(), &["build", "skeleton/broken.rlc"]);
    assert_eq!(build.code, Some(1));
    assert_eq!(
        build.stderr_str(),
        "error: build.blocked: skeleton.broken has blocking diagnostics\n"
    );
    assert_eq!(
        build.stdout_str(),
        "error TY001 skeleton.broken #sk0a1b/4 [82,90]: expected i64, found bool\n"
    );
    write(
        &ws,
        "lib.rlc",
        "(mod lib)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) x)\n",
    );
    let no_entry = rail(ws.path(), &["run", "lib.rlc"]);
    assert_eq!(no_entry.code, Some(1));
    assert_eq!(
        no_entry.stderr_str(),
        "error: build.no_entry: lib has no exported main function\n"
    );
}

#[test]
fn run_exits_with_the_programs_own_code() {
    let ws = workspace("cli-run");
    write(
        &ws,
        "fails.rlc",
        "(mod fails)\n(pub main)\n(fn #aaaaaa main (_) (: (-> (Caps) (Result unit unit) log)) (let (_ (skel.print_i64 7)) (Err ())))\n",
    );
    let run = rail(ws.path(), &["run", "fails.rlc"]);
    assert_eq!(
        (
            run.code,
            run.stdout_str().as_str(),
            run.stderr_str().as_str()
        ),
        (Some(1), "7\n", "error: main returned Err\n")
    );
    let json = rail(ws.path(), &["run", "--json", "fails.rlc"]);
    assert_eq!(json.code, Some(1));
    assert_eq!(
        json.stdout_str(),
        "{\"exit_code\":1,\"stdout\":\"7\\n\",\"stderr\":\"error: main returned Err\\n\"}\n"
    );
    write(
        &ws,
        "traps.rlc",
        "(mod traps)\n(pub main)\n(fn #aaaaaa main (_) (: (-> (Caps) (Result unit unit) log)) (let (_ (skel.print_i64 (/ 1 0))) (Ok ())))\n",
    );
    let trap = rail(ws.path(), &["run", "traps.rlc"]);
    assert_eq!(
        (trap.code, trap.stderr_str().as_str()),
        (Some(70), "trap: division by zero\n")
    );
}

#[test]
fn tool_failures_exit_3() {
    let ws = workspace("cli-tool-failure");
    let missing = rail(ws.path(), &["check", "skeleton/missing.rlc"]);
    assert_eq!(missing.code, Some(3));
    assert_eq!(
        missing.stderr_str(),
        "error: module.not_found: module skeleton/missing.rlc not found in the workspace\n"
    );
    let json = rail(ws.path(), &["check", "--json", "../escape.rlc"]);
    assert_eq!(json.code, Some(3));
    assert_eq!(
        json.stdout_str(),
        "{\"error\":{\"code\":\"module.not_found\",\"message\":\"module ../escape.rlc not found in the workspace\"}}\n"
    );
}

#[test]
fn a_missing_cc_is_build_failed_with_exit_3() {
    let ws = workspace("cli-no-cc");
    let empty = TempDir::new("cli-empty-path");
    let mut cmd = Command::new(RAIL);
    cmd.args(["build", "skeleton/answer.rlc"])
        .current_dir(ws.path())
        .env_remove("RAIL_LOG")
        .env("PATH", empty.path());
    let out = run_process(&mut cmd, b"");
    assert_eq!(out.code, Some(3), "{out:?}");
    assert!(
        out.stderr_str()
            .starts_with("error: build.failed: `cc` not found"),
        "{out:?}"
    );
    assert!(!ws.path().join(".rail/build/dev/skeleton/answer").exists());
}

#[test]
fn usage_errors_exit_2() {
    let ws = workspace("cli-usage");
    for args in [
        &[][..],
        &["frobnicate", "x.rlc"][..],
        &["check"][..],
        &["check", "a.rlc", "b.rlc"][..],
        &["check", "--yaml", "skeleton/answer.rlc"][..],
        &["rap", "extra"][..],
    ] {
        let out = rail(ws.path(), args);
        assert_eq!(out.code, Some(2), "{args:?}");
        assert!(
            out.stderr_str().starts_with("usage: rail"),
            "{args:?}: {out:?}"
        );
        assert_eq!(out.stdout_str(), "");
    }
}

#[test]
fn json_output_equals_the_protocol_result_shape() {
    let ws = workspace("cli-json");
    let build = rail(ws.path(), &["build", "--json", "skeleton/answer.rlc"]);
    let target = if cfg!(target_os = "macos") {
        "aarch64-macos"
    } else {
        "x86_64-linux"
    };
    assert_eq!(
        build.stdout_str(),
        format!(
            "{{\"module\":\"skeleton.answer\",\"target\":\"{target}\",\"mode\":\"dev\",\"path\":\".rail/build/dev/skeleton/answer\"}}\n"
        )
    );
    let flag_first = rail(ws.path(), &["check", "skeleton/answer.rlc", "--json"]);
    assert_eq!(
        flag_first.stdout_str(),
        "{\"module\":\"skeleton.answer\",\"diagnostics\":[],\"blocking\":false}\n"
    );
}

#[test]
fn rail_log_writes_deterministic_records_to_stderr() {
    let ws = workspace("cli-log");
    let first = rail_env(ws.path(), &["check", "skeleton/answer.rlc"], Some("debug"));
    assert_eq!(first.stdout_str(), "skeleton.answer: no diagnostics\n");
    assert_eq!(
        first.stderr_str(),
        "rail debug check.run skeleton/answer.rlc start\nrail info check.run skeleton/answer.rlc ok\n"
    );
    let other = workspace("cli-log-elsewhere");
    let second = rail_env(
        other.path(),
        &["check", "skeleton/answer.rlc"],
        Some("debug"),
    );
    assert_eq!(
        first.stderr_str(),
        second.stderr_str(),
        "no paths, times or process IDs"
    );
    let error_only = rail_env(ws.path(), &["check", "skeleton/answer.rlc"], Some("error"));
    assert_eq!(error_only.stderr_str(), "");
    let failure = rail_env(ws.path(), &["build", "skeleton/broken.rlc"], Some("error"));
    assert_eq!(
        failure.stderr_str(),
        "rail error build.run skeleton/broken.rlc code=build.blocked skeleton.broken has blocking diagnostics\nerror: build.blocked: skeleton.broken has blocking diagnostics\n"
    );
    let bogus = rail_env(ws.path(), &["check", "skeleton/answer.rlc"], Some("loud"));
    assert_eq!(
        bogus.stderr_str(),
        "rail warning RAIL_LOG must be error, info or debug; logging is off\n"
    );
}
