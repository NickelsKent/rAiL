//! The command line and `corpus/check.sh`: flags, exit codes 0/1/2 and one
//! output line per program and language (WF3).

mod common;

use common::{GEN_RANDOM, Synth, TempDir, have_all_toolchains, run_cli};
use corpus_check::cli::{Options, parse_args};
use corpus_check::model::{Class, Lang};
use std::path::{Path, PathBuf};
use std::process::Command;

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(ToString::to_string).collect()
}

fn lines_with<'a>(stdout: &'a str, word: &str) -> Vec<&'a str> {
    stdout.lines().filter(|l| l.contains(word)).collect()
}

// ---- argument parsing ----

#[test]
fn defaults_cover_the_whole_corpus_in_check_mode() {
    let o = parse_args(&args(&[]), false).unwrap();
    assert_eq!(o.class, None);
    assert_eq!(o.program, None);
    assert_eq!(o.langs, vec![Lang::Rust, Lang::Go, Lang::Ocaml]);
    assert!(!o.bless && !o.complete);
    assert_eq!(o.budget_seconds, 20 * 60);
    assert_eq!(o.deadline_seconds, 60);
    assert_eq!(o.corpus_dir, PathBuf::from("corpus"));
}

#[test]
fn every_flag_is_understood() {
    let o = parse_args(
        &args(&[
            "--class",
            "io-services",
            "--program",
            "dir-summary",
            "--lang",
            "rust,go",
            "--bless",
            "--complete",
            "--budget-seconds",
            "0",
            "--deadline-seconds",
            "5",
            "--corpus-dir",
            "/c",
            "--sandbox-profile",
            "/p.sb",
            "--work-dir",
            "/w",
        ]),
        false,
    )
    .unwrap();
    assert_eq!(
        o,
        Options {
            corpus_dir: PathBuf::from("/c"),
            class: Some(Class::IoServices),
            program: Some("dir-summary".to_string()),
            langs: vec![Lang::Rust, Lang::Go],
            bless: true,
            complete: true,
            budget_seconds: 0,
            deadline_seconds: 5,
            sandbox_profile: Some(PathBuf::from("/p.sb")),
            work_dir: Some(PathBuf::from("/w")),
        }
    );
}

#[test]
fn rail_bless_in_the_environment_is_the_same_as_the_flag() {
    assert!(parse_args(&args(&[]), true).unwrap().bless);
    assert!(parse_args(&args(&["--bless"]), false).unwrap().bless);
    assert!(!parse_args(&args(&[]), false).unwrap().bless);
}

#[test]
fn bad_arguments_are_refused_with_a_message() {
    for bad in [
        &["--frobnicate"][..],
        &["--class", "bogus"],
        &["--class"],
        &["--lang", "rail"],
        &["--lang", "cobol"],
        &["--budget-seconds", "soon"],
        &["--deadline-seconds", "0"],
        &["stray"],
    ] {
        let err = parse_args(&args(bad), false).expect_err(&format!("{bad:?}"));
        assert!(!err.is_empty());
    }
}

// ---- end to end with synthetic corpora ----

#[test]
fn a_passing_corpus_exits_0_with_one_line_per_program_and_language() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let (code, stdout, stderr) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert_eq!(
        lines_with(&stdout, "pure-transform/binary-trees rust"),
        vec!["PASS pure-transform/binary-trees rust (2 inputs)"]
    );
    assert_eq!(
        lines_with(&stdout, "generator"),
        vec!["PASS pure-transform/binary-trees generator"]
    );
    assert!(
        stdout.lines().last().unwrap().starts_with("corpus check:"),
        "{stdout}"
    );
    assert!(
        !stdout.contains(synth.root.path().to_str().unwrap()),
        "no absolute paths in the report"
    );
}

#[test]
fn all_three_languages_each_get_a_line() {
    if !have_all_toolchains() {
        return;
    }
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let (code, stdout, stderr) = run_cli(&synth, &[], &[]);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    for lang in ["rust", "go", "ocaml"] {
        let found = lines_with(&stdout, &format!("pure-transform/binary-trees {lang}"));
        assert_eq!(
            found,
            vec![format!(
                "PASS pure-transform/binary-trees {lang} (2 inputs)"
            )],
            "{stdout}"
        );
    }
}

#[test]
fn bless_writes_expected_output_from_rust_and_the_next_check_passes() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust", "--bless"], &[]);
    assert_eq!(code, 0, "{stdout}");
    assert!(
        stdout.contains("BLESSED pure-transform/binary-trees rust"),
        "{stdout}"
    );
    let dir = synth.program("pure-transform", "binary-trees");
    assert_eq!(
        std::fs::read(dir.join("expected/one.out")).unwrap(),
        b"hello one\n"
    );
    assert_eq!(
        std::fs::read(dir.join("expected/two.out")).unwrap(),
        b"hello two\n"
    );

    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 0, "{stdout}");
    assert!(stdout.contains("PASS pure-transform/binary-trees rust"));

    // RAIL_BLESS=1 is the same as --bless.
    std::fs::write(dir.join("expected/one.out"), "stale\n").unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[("RAIL_BLESS", "1")]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(
        std::fs::read(dir.join("expected/one.out")).unwrap(),
        b"hello one\n"
    );
}

#[test]
fn a_mismatch_exits_1_naming_program_language_input_and_byte() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    std::fs::write(
        synth
            .program("pure-transform", "binary-trees")
            .join("expected/two.out"),
        "hello twp\n",
    )
    .unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 1, "{stdout}");
    let line = lines_with(&stdout, "FAIL")
        .into_iter()
        .next()
        .expect("a FAIL line");
    for part in [
        "pure-transform/binary-trees",
        "rust",
        "two",
        "byte 8",
        "0x70",
        "0x6f",
    ] {
        assert!(line.contains(part), "{part}: {line}");
    }
}

#[test]
fn a_toolchain_mismatch_exits_2_naming_both_versions() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let pins = std::fs::read_to_string(synth.corpus().join("toolchains.toml")).unwrap();
    let rust_line = pins
        .lines()
        .find(|l| l.starts_with("rust"))
        .unwrap()
        .to_string();
    std::fs::write(
        synth.corpus().join("toolchains.toml"),
        pins.replace(&rust_line, "rust = \"1.50.0\""),
    )
    .unwrap();
    let (code, stdout, stderr) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 2, "{stdout}{stderr}");
    assert!(
        stderr.contains("1.50.0") && stderr.contains("rust"),
        "{stderr}"
    );
    assert!(!stdout.contains("PASS"), "nothing ran");
}

#[test]
fn a_corpus_problem_exits_1_and_nothing_runs() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    std::fs::remove_file(
        synth
            .program("pure-transform", "binary-trees")
            .join("prompt.md"),
    )
    .unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.contains("FAIL") && stdout.contains("prompt.md") && stdout.contains("binary-trees"),
        "{stdout}"
    );
    assert!(!stdout.contains("PASS"), "nothing ran: {stdout}");

    // a disallowed dependency is a corpus problem too
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let manifest = synth
        .program("pure-transform", "binary-trees")
        .join("rust/Cargo.toml");
    let text = std::fs::read_to_string(&manifest).unwrap();
    std::fs::write(
        &manifest,
        format!("{text}\n[dependencies]\nserde = \"1\"\n"),
    )
    .unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("serde"), "{stdout}");
}

#[test]
fn a_nondeterministic_generator_fails_the_check() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let gen_main = synth
        .program("pure-transform", "binary-trees")
        .join("inputs/gen/src/main.rs");
    std::fs::write(gen_main, GEN_RANDOM).unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 1, "{stdout}");
    let line = lines_with(&stdout, "generator")
        .into_iter()
        .next()
        .expect("a generator line");
    assert!(
        line.starts_with("FAIL") && line.contains("binary-trees") && line.contains("data.txt"),
        "{line}"
    );
}

#[test]
fn class_and_program_flags_scope_the_run() {
    let synth = Synth::new(&[
        ("binary-trees", "pure-transform"),
        ("json-validate", "parsing-text"),
    ]);
    synth.write_expected("pure-transform", "binary-trees");
    synth.write_expected("parsing-text", "json-validate");
    let (code, stdout, _) = run_cli(
        &synth,
        &["--lang", "rust", "--program", "binary-trees"],
        &[],
    );
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(lines_with(&stdout, " rust ").len(), 1, "{stdout}");
    assert!(stdout.contains("pure-transform/binary-trees rust"));

    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust", "--class", "parsing-text"], &[]);
    assert_eq!(code, 0, "{stdout}");
    assert_eq!(lines_with(&stdout, " rust ").len(), 1, "{stdout}");
    assert!(stdout.contains("parsing-text/json-validate rust"));

    let (code, _, stderr) = run_cli(&synth, &["--lang", "rust", "--program", "nope"], &[]);
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("nope"));
}

#[test]
fn an_approved_class_must_be_complete_and_complete_requires_all_four() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    // A class still being written may have fewer than ten programs.
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 0, "{stdout}");
    // Once the owner has approved the class (APPROVAL.md), it must be whole.
    std::fs::write(
        synth.corpus().join("pure-transform/APPROVAL.md"),
        "approved\n",
    )
    .unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust"], &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("has 1 programs, expected 10"), "{stdout}");
    // --complete requires every class.
    std::fs::remove_file(synth.corpus().join("pure-transform/APPROVAL.md")).unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust", "--complete"], &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(
        stdout.contains("class parsing-text has 0 programs"),
        "{stdout}"
    );
}

#[test]
fn usage_errors_exit_2() {
    let synth = Synth::new(&[]);
    let (code, _, stderr) = run_cli(&synth, &["--frobnicate"], &[]);
    assert_eq!(code, 2);
    assert!(
        stderr.contains("error:") && stderr.contains("--frobnicate"),
        "{stderr}"
    );
}

#[test]
fn the_time_budget_stops_the_check_and_fails_it() {
    let synth = Synth::new(&[("binary-trees", "pure-transform")]);
    synth.write_expected("pure-transform", "binary-trees");
    let dir = synth.program("pure-transform", "binary-trees");
    // The second input makes the Rust version sleep for 30 seconds.
    std::fs::write(dir.join("inputs/fixed/two/data.txt"), "slow\n").unwrap();
    let (code, stdout, _) = run_cli(&synth, &["--lang", "rust", "--budget-seconds", "4"], &[]);
    assert_eq!(code, 1, "{stdout}");
    assert!(stdout.contains("over time budget"), "{stdout}");
    assert!(stdout.contains("slowest"), "{stdout}");
}

// ---- corpus/check.sh ----

fn check_sh() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../check.sh")
}

/// check.sh builds the checker itself. It must not build into the target
/// directory of the running `cargo test`: that would replace the binary the
/// other tests are executing (the root cause of a flaky exit 2). It builds
/// into one private directory, and the script tests run one at a time.
fn script(args: &[&str]) -> (i32, String, String) {
    static SERIAL: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let target =
        std::env::temp_dir().join(format!("corpus-check-sh-target-{}", std::process::id()));
    let out = Command::new("bash")
        .env("CORPUS_CHECK_TARGET_DIR", target)
        .arg(check_sh())
        .args(args)
        .output()
        .expect("bash starts");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn check_sh_builds_the_checker_and_passes_its_arguments_and_exit_code_through() {
    let (code, stdout, _) = script(&["--help"]);
    assert_eq!(code, 0);
    assert!(
        stdout.contains("usage:") && stdout.contains("--class"),
        "{stdout}"
    );

    let (code, _, stderr) = script(&["--frobnicate"]);
    assert_eq!(code, 2);
    assert!(stderr.contains("--frobnicate"), "{stderr}");
}

#[test]
fn check_sh_passes_on_the_corpus_as_it_stands() {
    // No programs are delivered yet in a fresh checkout: an empty index passes.
    let (code, stdout, stderr) = script(&["--lang", "rust"]);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("corpus check:"), "{stdout}");
}

#[test]
fn check_sh_runs_the_checker_against_another_corpus_directory() {
    let synth = Synth::new(&[]);
    let dir = TempDir::new("sh-scratch");
    let (code, stdout, stderr) = script(&[
        "--lang",
        "rust",
        "--corpus-dir",
        synth.corpus().to_str().unwrap(),
        "--work-dir",
        dir.path().to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "stdout:\n{stdout}\nstderr:\n{stderr}");
}
