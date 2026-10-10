//! Building the Rust, Go and OCaml versions offline with the pinned
//! toolchains (WF3 step 5).

mod common;

use common::TempDir;
use corpus_check::build::build_version;
use corpus_check::model::Lang;
use std::process::Command;

/// The tests need Go and OCaml on the path. Without them the test says so and
/// stops, unless `CORPUS_CHECK_REQUIRE_TOOLCHAINS` is set (CI sets it), in
/// which case a missing tool is a failure.
fn have(tool: &str, version_arg: &str) -> bool {
    let found = Command::new(tool)
        .arg(version_arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !found {
        assert!(
            std::env::var_os("CORPUS_CHECK_REQUIRE_TOOLCHAINS").is_none(),
            "{tool} is required but not on PATH"
        );
        eprintln!(
            "SKIPPED: {tool} is not on PATH (set CORPUS_CHECK_REQUIRE_TOOLCHAINS=1 to require it)"
        );
    }
    found
}

fn run(exe: &std::path::Path) -> String {
    let out = Command::new(exe).output().expect("built program runs");
    assert!(out.status.success());
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn builds_a_rust_package_offline_from_its_lockfile() {
    let src = TempDir::new("build-rust-src");
    src.write(
        "Cargo.toml",
        "[package]\nname = \"hello\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    src.write(
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"hello\"\nversion = \"0.1.0\"\n",
    );
    src.write("src/main.rs", "fn main() { println!(\"hello rust\"); }\n");
    let out = TempDir::new("build-rust-out");
    let built = build_version(Lang::Rust, src.path(), out.path()).expect("builds");
    assert_eq!(run(&built.exe), "hello rust\n");
    assert!(built.exe.starts_with(&built.build_dir));
    assert!(built.build_dir.starts_with(out.path()));
}

#[test]
fn a_rust_build_failure_carries_the_compiler_message() {
    let src = TempDir::new("build-rust-bad");
    src.write(
        "Cargo.toml",
        "[package]\nname = \"bad\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    src.write(
        "Cargo.lock",
        "version = 4\n\n[[package]]\nname = \"bad\"\nversion = \"0.1.0\"\n",
    );
    src.write("src/main.rs", "fn main() { let x: u32 = \"no\"; }\n");
    let out = TempDir::new("build-rust-bad-out");
    let err = build_version(Lang::Rust, src.path(), out.path()).expect_err("does not compile");
    assert!(err.contains("mismatched types"), "{err}");
}

#[test]
fn a_rust_package_without_a_lockfile_is_not_built() {
    let src = TempDir::new("build-rust-nolock");
    src.write(
        "Cargo.toml",
        "[package]\nname = \"nolock\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    src.write("src/main.rs", "fn main() {}\n");
    let out = TempDir::new("build-rust-nolock-out");
    assert!(build_version(Lang::Rust, src.path(), out.path()).is_err());
}

#[test]
fn builds_a_go_module() {
    if !have("go", "version") {
        return;
    }
    let src = TempDir::new("build-go-src");
    src.write("go.mod", "module hello\n\ngo 1.27\n");
    src.write(
        "main.go",
        "package main\n\nimport \"fmt\"\n\nfunc main() { fmt.Println(\"hello go\") }\n",
    );
    let out = TempDir::new("build-go-out");
    let built = build_version(Lang::Go, src.path(), out.path()).expect("builds");
    assert_eq!(run(&built.exe), "hello go\n");
}

#[test]
fn builds_an_ocaml_dune_project() {
    if !have("dune", "--version") {
        return;
    }
    let src = TempDir::new("build-ocaml-src");
    src.write("dune-project", "(lang dune 3.0)\n");
    src.write("bin/dune", "(executable (name main))\n");
    src.write("bin/main.ml", "let () = print_endline \"hello ocaml\"\n");
    let out = TempDir::new("build-ocaml-out");
    let built = build_version(Lang::Ocaml, src.path(), out.path()).expect("builds");
    assert_eq!(run(&built.exe), "hello ocaml\n");
}

#[test]
fn the_rail_version_is_not_built() {
    let src = TempDir::new("build-rail");
    let out = TempDir::new("build-rail-out");
    assert!(build_version(Lang::Rail, src.path(), out.path()).is_err());
}
