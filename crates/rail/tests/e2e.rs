//! WF6 end-to-end acceptance tests for the walking skeleton (FR27).
//!
//! Each test copies the two fixtures into its own temporary workspace and
//! drives the real `rail` binary, through the command line and through the
//! protocol server. Expected outputs are the shared golden files in
//! `tests/golden/`, used unchanged on macOS and Linux (NFR6.1).

use std::path::{Path, PathBuf};
use std::process::Command;

use rail_testkit::{RapClient, TempDir, assert_golden, copy_dir, repo_root, run_process};

const RAIL: &str = env!("CARGO_BIN_EXE_rail");

fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn golden_text(name: &str) -> String {
    std::fs::read_to_string(golden(name)).expect("golden file")
}

/// A fresh workspace holding `skeleton/answer.rlc` and `skeleton/broken.rlc`.
fn workspace(label: &str) -> TempDir {
    let dir = TempDir::new(label);
    copy_dir(&repo_root().join("fixtures"), dir.path()).expect("copy fixtures");
    dir
}

fn rail(ws: &Path, args: &[&str]) -> rail_testkit::ProcessOutput {
    let mut cmd = Command::new(RAIL);
    cmd.args(args).current_dir(ws).env_remove("RAIL_LOG");
    run_process(&mut cmd, b"")
}

fn json_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

fn request(id: i64, method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{params}}}"#)
}

fn initialize(client: &mut RapClient, ws: &Path) {
    let root = json_string(&ws.display().to_string());
    client.send(&request(
        1,
        "initialize",
        &format!(r#"{{"client_versions":["0.1"],"workspace_root":{root}}}"#),
    ));
    assert_eq!(
        client.recv(),
        r#"{"jsonrpc":"2.0","id":1,"result":{"rap_version":"0.1","capabilities":["tree.get","check.run","build.run","run.run"]}}"#
    );
}

/// The `result` of a successful response, as compact JSON text plus a newline
/// (the same bytes `rail <op> --json` prints). Responses are written with the
/// fixed field order `jsonrpc`, `id`, `result`.
fn result_text(response: &str, id: i64) -> String {
    let prefix = format!(r#"{{"jsonrpc":"2.0","id":{id},"result":"#);
    let inner = response
        .strip_prefix(&prefix)
        .and_then(|rest| rest.strip_suffix('}'))
        .unwrap_or_else(|| panic!("expected a result for id {id}: {response}"));
    format!("{inner}\n")
}

#[test]
fn step1_check_answer_reports_no_diagnostics() {
    let ws = workspace("e2e-check-answer");
    let out = rail(ws.path(), &["check", "--json", "skeleton/answer.rlc"]);
    assert_eq!(out.code, Some(0), "{out:?}");
    assert_golden(&golden("check-answer.json"), &out.stdout_str());
    assert_eq!(out.stderr_str(), "");
}

#[test]
fn step2_run_answer_prints_42_and_exits_0() {
    let ws = workspace("e2e-run-answer");
    let out = rail(ws.path(), &["run", "skeleton/answer.rlc"]);
    assert_eq!(out.stdout_str(), "42\n", "{out:?}");
    assert_eq!(out.stderr_str(), "");
    assert_eq!(out.code, Some(0));
    let json = rail(ws.path(), &["run", "--json", "skeleton/answer.rlc"]);
    assert_eq!(json.code, Some(0), "{json:?}");
    assert_golden(&golden("run-answer.json"), &json.stdout_str());
}

#[test]
fn step3_protocol_session_on_answer() {
    let ws = workspace("e2e-rap-answer");
    let mut client = RapClient::spawn(Path::new(RAIL), ws.path());
    initialize(&mut client, ws.path());
    let module = r#"{"module":"skeleton/answer.rlc"}"#;

    client.send(&request(2, "tree.get", module));
    let tree = result_text(&client.recv(), 2);
    assert_golden(&golden("tree-answer.json"), &tree);

    client.send(&request(3, "check.run", module));
    assert_golden(
        &golden("check-answer.json"),
        &result_text(&client.recv(), 3),
    );

    client.send(&request(4, "build.run", module));
    let expected_target = if cfg!(target_os = "macos") {
        "aarch64-macos"
    } else {
        "x86_64-linux"
    };
    assert_eq!(
        result_text(&client.recv(), 4),
        format!(
            "{{\"module\":\"skeleton.answer\",\"target\":\"{expected_target}\",\"mode\":\"dev\",\"path\":\".rail/build/dev/skeleton/answer\"}}\n"
        )
    );
    assert!(ws.path().join(".rail/build/dev/skeleton/answer").is_file());

    client.send(&request(5, "run.run", module));
    assert_golden(&golden("run-answer.json"), &result_text(&client.recv(), 5));

    let out = client.close();
    assert_eq!(out.code, Some(0), "{out:?}");
}

#[test]
fn step4_broken_is_refused_on_the_command_line() {
    let ws = workspace("e2e-broken-cli");
    let check = rail(ws.path(), &["check", "--json", "skeleton/broken.rlc"]);
    assert_eq!(check.code, Some(1), "{check:?}");
    assert_golden(&golden("check-broken.json"), &check.stdout_str());

    let human = rail(ws.path(), &["check", "skeleton/broken.rlc"]);
    assert_eq!(human.code, Some(1), "{human:?}");
    assert_golden(&golden("check-broken.txt"), &human.stdout_str());

    let run = rail(ws.path(), &["run", "--json", "skeleton/broken.rlc"]);
    assert_eq!(run.code, Some(1), "{run:?}");
    assert_golden(&golden("run-broken.json"), &run.stdout_str());

    let build = rail(ws.path(), &["build", "skeleton/broken.rlc"]);
    assert_eq!(build.code, Some(1), "{build:?}");
    assert!(!ws.path().join(".rail/build/dev/skeleton/broken").exists());
}

#[test]
fn step4_broken_is_refused_over_the_protocol() {
    let ws = workspace("e2e-broken-rap");
    let mut client = RapClient::spawn(Path::new(RAIL), ws.path());
    initialize(&mut client, ws.path());
    let module = r#"{"module":"skeleton/broken.rlc"}"#;

    client.send(&request(2, "check.run", module));
    assert_golden(
        &golden("check-broken.json"),
        &result_text(&client.recv(), 2),
    );

    client.send(&request(3, "run.run", module));
    // The ToolError is exactly what `rail run --json` prints under "error".
    let cli = golden_text("run-broken.json");
    let tool_error = cli
        .strip_prefix(r#"{"error":"#)
        .and_then(|rest| rest.strip_suffix("}\n"))
        .expect("golden shape");
    assert_eq!(
        client.recv(),
        format!(
            r#"{{"jsonrpc":"2.0","id":3,"error":{{"code":-32000,"message":"skeleton.broken has blocking diagnostics","data":{tool_error}}}}}"#
        )
    );
    assert!(!ws.path().join(".rail/build/dev/skeleton/broken").exists());
    client.close();
}

#[test]
fn step5_server_still_answers_after_a_malformed_message() {
    let ws = workspace("e2e-malformed");
    let mut client = RapClient::spawn(Path::new(RAIL), ws.path());
    initialize(&mut client, ws.path());

    client.send("{\"jsonrpc\":\"2.0\",\"id\":2,");
    let response = client.recv();
    assert!(
        response.starts_with(r#"{"jsonrpc":"2.0","id":null,"error":{"code":-32700,"#),
        "{response}"
    );

    client.send(&request(
        3,
        "check.run",
        r#"{"module":"skeleton/answer.rlc"}"#,
    ));
    assert_golden(
        &golden("check-answer.json"),
        &result_text(&client.recv(), 3),
    );
    client.close();
}

#[test]
fn step6_two_builds_are_byte_identical() {
    let first = workspace("e2e-build-one");
    let second = workspace("e2e-build-two");
    for ws in [&first, &second] {
        let out = rail(ws.path(), &["build", "skeleton/answer.rlc"]);
        assert_eq!(out.code, Some(0), "{out:?}");
    }
    let a = std::fs::read(first.path().join(".rail/build/dev/skeleton/answer")).expect("first");
    let b = std::fs::read(second.path().join(".rail/build/dev/skeleton/answer")).expect("second");
    assert!(a == b, "the two builds differ");
}

#[test]
fn step7_cli_json_equals_protocol_results() {
    let ws = workspace("e2e-parity");
    let mut client = RapClient::spawn(Path::new(RAIL), ws.path());
    initialize(&mut client, ws.path());
    let cases = [
        ("parse", "tree.get", "skeleton/answer.rlc"),
        ("check", "check.run", "skeleton/answer.rlc"),
        ("check", "check.run", "skeleton/broken.rlc"),
        ("run", "run.run", "skeleton/answer.rlc"),
    ];
    for (id, (command, method, module)) in (10..).zip(cases) {
        let cli = rail(ws.path(), &[command, "--json", module]);
        client.send(&request(id, method, &format!(r#"{{"module":"{module}"}}"#)));
        assert_eq!(
            cli.stdout_str(),
            result_text(&client.recv(), id),
            "{command} {module}"
        );
    }
    client.close();
}
