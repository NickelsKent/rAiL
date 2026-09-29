//! The four operations over workspace access (BR6.1, BR4.2, BR2.8, BR5.2,
//! BR6.6, NFR5.1, NFR9.11).

use std::fs;
use std::path::Path;

use rail_json::Value;
use rail_testkit::{TempDir, copy_dir, repo_root};
use rail_tools::{ErrorCode, Operation, RunLimits, Workspace, invoke, run_executable};

fn workspace(label: &str) -> (TempDir, Workspace) {
    let dir = TempDir::new(label);
    copy_dir(&repo_root().join("fixtures"), dir.path()).unwrap();
    let ws = Workspace::open(dir.path()).unwrap();
    (dir, ws)
}

fn json(value: &Value) -> String {
    rail_json::to_string(value)
}

#[test]
fn operations_map_to_commands_and_methods() {
    for (op, command, method) in [
        (Operation::Parse, "parse", "tree.get"),
        (Operation::Check, "check", "check.run"),
        (Operation::Build, "build", "build.run"),
        (Operation::Run, "run", "run.run"),
    ] {
        assert_eq!(op.command(), command);
        assert_eq!(op.method(), method);
        assert_eq!(Operation::from_command(command), Some(op));
        assert_eq!(Operation::from_method(method), Some(op));
    }
    assert_eq!(Operation::from_method("fmt.apply"), None);
    assert_eq!(Operation::from_command("fmt"), None);
}

#[test]
fn parse_returns_the_tree() {
    let (_dir, ws) = workspace("ops-parse");
    let result = invoke(&ws, Operation::Parse, "skeleton/answer.rlc").unwrap();
    assert_eq!(
        result.get("module").and_then(Value::as_str),
        Some("skeleton.answer")
    );
    assert_eq!(json(result.get("diagnostics").unwrap()), "[]");
    let tree = result.get("tree").unwrap();
    assert_eq!(json(tree.get("exports").unwrap()), r#"["answer","main"]"#);
    let defs = tree.get("definitions").and_then(Value::as_array).unwrap();
    assert_eq!(defs.len(), 2);
    let answer = &defs[0];
    assert_eq!(json(answer.get("defid").unwrap()), r##""#sk0a1b""##);
    assert_eq!(
        json(answer.get("signature").unwrap()),
        r#""(-> (i64) i64)""#
    );
    assert_eq!(
        json(answer.get("body").unwrap()),
        r#"{"form":"application","path":"4","span":[82,89],"children":[{"form":"reference","path":"4.0","span":[83,84],"name":"*"},{"form":"reference","path":"4.1","span":[85,86],"name":"x"},{"form":"int_literal","path":"4.2","span":[87,88],"value":7}]}"#
    );
}

#[test]
fn parse_reports_skl001_instead_of_a_tree() {
    let (dir, ws) = workspace("ops-parse-bad");
    fs::write(dir.path().join("bad.rlc"), b"(mod bad)\n(use a.b)\n").unwrap();
    let result = invoke(&ws, Operation::Parse, "bad.rlc").unwrap();
    assert!(result.get("tree").is_some_and(Value::is_null));
    let diags = result.get("diagnostics").and_then(Value::as_array).unwrap();
    assert_eq!(diags[0].get("rule").and_then(Value::as_str), Some("SKL001"));
}

#[test]
fn check_returns_the_check_result() {
    let (_dir, ws) = workspace("ops-check");
    let clean = invoke(&ws, Operation::Check, "skeleton/answer.rlc").unwrap();
    assert_eq!(
        json(&clean),
        r#"{"module":"skeleton.answer","diagnostics":[],"blocking":false}"#
    );
    let broken = invoke(&ws, Operation::Check, "skeleton/broken.rlc").unwrap();
    assert_eq!(broken.get("blocking").and_then(Value::as_bool), Some(true));
}

#[test]
fn build_returns_the_artifact_or_refuses() {
    let (dir, ws) = workspace("ops-build");
    let artifact = invoke(&ws, Operation::Build, "skeleton/answer.rlc").unwrap();
    let target = rail_codegen::host_target().unwrap().name;
    assert_eq!(
        json(&artifact),
        format!(
            r#"{{"module":"skeleton.answer","target":"{target}","mode":"dev","path":".rail/build/dev/skeleton/answer"}}"#
        )
    );
    let blocked = invoke(&ws, Operation::Build, "skeleton/broken.rlc").unwrap_err();
    assert_eq!(blocked.code, ErrorCode::BuildBlocked);
    assert_eq!(blocked.message, "skeleton.broken has blocking diagnostics");
    let data = blocked.data.expect("CheckResult in data");
    assert_eq!(data.get("blocking").and_then(Value::as_bool), Some(true));

    fs::write(
        dir.path().join("lib.rlc"),
        b"(mod lib)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) x)\n",
    )
    .unwrap();
    let no_entry = invoke(&ws, Operation::Build, "lib.rlc").unwrap_err();
    assert_eq!(no_entry.code, ErrorCode::BuildNoEntry);
    assert_eq!(no_entry.code.exit_code(), 1);
}

#[test]
fn run_returns_the_program_result() {
    let (dir, ws) = workspace("ops-run");
    let result = invoke(&ws, Operation::Run, "skeleton/answer.rlc").unwrap();
    assert_eq!(
        json(&result),
        r#"{"exit_code":0,"stdout":"42\n","stderr":""}"#
    );
    fs::write(
        dir.path().join("fails.rlc"),
        b"(mod fails)\n(pub main)\n(fn #aaaaaa main (_) (: (-> (Caps) (Result unit unit))) (Err ()))\n",
    )
    .unwrap();
    let err = invoke(&ws, Operation::Run, "fails.rlc").unwrap();
    assert_eq!(
        json(&err),
        r#"{"exit_code":1,"stdout":"","stderr":"error: main returned Err\n"}"#
    );
    let blocked = invoke(&ws, Operation::Run, "skeleton/broken.rlc").unwrap_err();
    assert_eq!(blocked.code, ErrorCode::BuildBlocked);
}

#[test]
fn module_errors_and_the_source_limit() {
    let (dir, ws) = workspace("ops-errors");
    for op in [
        Operation::Parse,
        Operation::Check,
        Operation::Build,
        Operation::Run,
    ] {
        let err = invoke(&ws, op, "skeleton/missing.rlc").unwrap_err();
        assert_eq!(err.code, ErrorCode::ModuleNotFound, "{op:?}");
        assert_eq!(err.code.exit_code(), 3);
    }
    fs::write(
        dir.path().join("big.rlc"),
        vec![b'x'; rail_tools::MAX_SOURCE_BYTES + 1],
    )
    .unwrap();
    let parsed = invoke(&ws, Operation::Parse, "big.rlc").unwrap();
    assert!(json(&parsed).contains(r#""rule":"SKL001""#));
    let checked = invoke(&ws, Operation::Check, "big.rlc").unwrap();
    assert_eq!(checked.get("blocking").and_then(Value::as_bool), Some(true));
    assert_eq!(
        invoke(&ws, Operation::Build, "big.rlc").unwrap_err().code,
        ErrorCode::BuildBlocked
    );
}

#[test]
fn programs_run_with_a_cleared_environment_and_no_input() {
    let env = run_executable(Path::new("/usr/bin/env"), &RunLimits::default()).unwrap();
    assert_eq!((env.exit_code, env.stdout.as_str()), (0, ""));
    let cat = run_executable(Path::new("/bin/cat"), &RunLimits::default()).unwrap();
    assert_eq!(
        (cat.exit_code, cat.stdout.as_str()),
        (0, ""),
        "stdin is empty"
    );
    assert_eq!(
        RunLimits::default().time,
        std::time::Duration::from_secs(10)
    );
    assert_eq!(RunLimits::default().output_bytes, 16 * 1024 * 1024);
}
