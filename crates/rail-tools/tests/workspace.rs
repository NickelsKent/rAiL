//! Workspace access: path confinement and bounded reads (NFR5.3, NFR4.4).

use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};

use rail_testkit::TempDir;
use rail_tools::{ErrorCode, MAX_SOURCE_BYTES, Workspace};

fn workspace_with(files: &[(&str, &[u8])]) -> (TempDir, Workspace) {
    let dir = TempDir::new("ws-access");
    for (path, bytes) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, bytes).unwrap();
    }
    let ws = Workspace::open(dir.path()).expect("open workspace");
    (dir, ws)
}

#[test]
fn a_module_path_resolves_inside_the_root_with_its_qname() {
    let (_dir, ws) = workspace_with(&[("skeleton/answer.rlc", b"(mod skeleton.answer)\n")]);
    let source = ws
        .read_module("skeleton/answer.rlc")
        .expect("readable module");
    assert_eq!(source.path.rel, "skeleton/answer.rlc");
    assert_eq!(source.path.qname, "skeleton.answer");
    assert!(source.path.abs.starts_with(ws.root()));
    assert_eq!(source.bytes, b"(mod skeleton.answer)\n");
    assert!(!source.oversized);
    assert!(source.limit_diagnostic().is_none());
}

#[test]
fn paths_that_could_leave_the_root_are_not_found() {
    let (dir, ws) = workspace_with(&[("a/b.rlc", b"")]);
    let outside = TempDir::new("ws-outside");
    fs::write(outside.path().join("secret.rlc"), b"(mod secret)\n").unwrap();
    symlink(
        outside.path().join("secret.rlc"),
        dir.path().join("a/link.rlc"),
    )
    .unwrap();
    symlink(outside.path(), dir.path().join("out")).unwrap();
    let absolute = outside.path().join("secret.rlc").display().to_string();
    for path in [
        "",
        "../a/b.rlc",
        "a/../a/b.rlc",
        "./a/b.rlc",
        absolute.as_str(),
        "a\\b.rlc",
        "a/b.rlc\0",
        "a//b.rlc",
        "a/link.rlc",
        "out/secret.rlc",
        "a/missing.rlc",
        "a/b.txt",
        "A/b.rlc",
    ] {
        let err = ws.read_module(path).expect_err(path);
        assert_eq!(
            err.code,
            ErrorCode::ModuleNotFound,
            "{path:?}: {}",
            err.message
        );
    }
}

#[test]
fn an_unreadable_file_is_module_unreadable() {
    let (dir, ws) = workspace_with(&[("a/locked.rlc", b"(mod a.locked)\n")]);
    let file = dir.path().join("a/locked.rlc");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o000)).unwrap();
    let result = ws.read_module("a/locked.rlc");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).unwrap();
    let err = result.expect_err("unreadable");
    assert_eq!(err.code, ErrorCode::ModuleUnreadable);
    assert_eq!(err.code.as_str(), "module.unreadable");
}

#[test]
fn a_directory_named_like_a_module_is_unreadable() {
    let (dir, ws) = workspace_with(&[]);
    fs::create_dir_all(dir.path().join("a/dir.rlc")).unwrap();
    let err = ws.read_module("a/dir.rlc").expect_err("directory");
    assert_eq!(err.code, ErrorCode::ModuleUnreadable);
}

#[test]
fn an_oversized_file_is_skl001_at_byte_16_mib() {
    assert_eq!(MAX_SOURCE_BYTES, 16 * 1024 * 1024);
    let big = vec![b' '; MAX_SOURCE_BYTES + 100];
    let (_dir, ws) = workspace_with(&[("big.rlc", &big)]);
    let source = ws.read_module("big.rlc").expect("readable");
    assert!(source.oversized);
    assert_eq!(source.bytes.len(), MAX_SOURCE_BYTES + 1, "bounded read");
    let diag = source.limit_diagnostic().expect("SKL001");
    assert_eq!(diag.rule.as_str(), "SKL001");
    assert_eq!(
        (diag.loc.span.start, diag.loc.span.end),
        (MAX_SOURCE_BYTES, MAX_SOURCE_BYTES + 1)
    );
    assert_eq!(diag.loc.module, "big");
    assert_eq!(diag.loc.def, "#000000");
}

#[test]
fn the_workspace_root_must_be_a_directory() {
    let dir = TempDir::new("ws-root");
    let file = dir.path().join("file");
    fs::write(&file, b"").unwrap();
    assert!(Workspace::open(&file).is_err());
    assert!(Workspace::open(&dir.path().join("missing")).is_err());
}

#[test]
fn tool_errors_serialise_with_code_and_message() {
    let (_dir, ws) = workspace_with(&[]);
    let err = ws.read_module("nope.rlc").expect_err("missing");
    assert_eq!(
        rail_json::to_string(&err.to_json()),
        r#"{"code":"module.not_found","message":"module nope.rlc not found in the workspace"}"#
    );
}
