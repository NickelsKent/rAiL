//! The helper's behaviour is chosen entirely by its arguments (BR6.6).

use std::process::Command;

fn helper(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_corpus-helper"))
        .args(args)
        .output()
        .expect("helper starts")
}

#[test]
fn echo_prints_its_arguments_on_stdout() {
    let out = helper(&["echo", "hello", "world"]);
    assert_eq!(out.stdout, b"hello world\n");
    assert!(out.stderr.is_empty());
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn stderr_prints_on_stderr_only() {
    let out = helper(&["stderr", "oops"]);
    assert_eq!(out.stderr, b"oops\n");
    assert!(out.stdout.is_empty());
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn exit_uses_the_given_status() {
    let out = helper(&["exit", "3"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(out.stdout.is_empty() && out.stderr.is_empty());
}

#[test]
fn unknown_or_malformed_commands_exit_two_with_one_usage_line() {
    for args in [&["bogus"][..], &[][..], &["exit", "x"][..], &["exit"][..]] {
        let out = helper(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        let err = String::from_utf8(out.stderr).unwrap();
        assert_eq!(err.lines().count(), 1, "{args:?}: {err:?}");
        assert!(err.starts_with("usage: "), "{err:?}");
    }
}
