//! Repository / data access: checksums, tree listings, fixed input limits,
//! generator determinism, expected outputs and bless mode
//! (BR2.5, BR6.2, BR7.1 to BR7.3).

mod common;

use common::{TempDir, build_rust, entry_toml, generator_source};
use corpus_check::expected::{
    Expected, Mode, Verdict, expected_from_outcome, judge, load_expected, write_expected,
};
use corpus_check::index::{IndexEntry, parse_index};
use corpus_check::inputs::{LaunchSpec, check_fixed_inputs, check_generator_determinism};
use corpus_check::model::Lang;
use corpus_check::proc::ProcOutcome;
use corpus_check::sha256::sha256_hex;
use corpus_check::tree::tree_listing;
use std::fs;
use std::process::Command;
use std::time::{Duration, SystemTime};

// ---- SHA-256 against published and independently computed vectors ----

#[test]
fn sha256_matches_the_published_test_vectors() {
    assert_eq!(
        sha256_hex(b""),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(
        sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
    assert_eq!(
        sha256_hex(&vec![b'a'; 1_000_000]),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn sha256_handles_every_padding_boundary() {
    // Lengths around the 55/56 and 64 byte block boundaries; expected values
    // computed with an independent implementation (Python hashlib).
    for (n, want) in [
        (
            55,
            "9f4390f8d30c2dd92ec9f095b65e2b9ae9b0a925a5258e241c9f1e910f734318",
        ),
        (
            56,
            "b35439a4ac6f0948b6d6f9e3c6af0f5f590ce20f1bde7090ef7970686ec6738a",
        ),
        (
            63,
            "7d3e74a05d7db15bce4ad9ec0658ea98e3f06eeecf16b4c6fff2da457ddc2f34",
        ),
        (
            64,
            "ffe054fe7ae0cb6dc65c3af9b61d5209f439851db43d0ba5997337df154668eb",
        ),
        (
            65,
            "635361c48bb9eab14198e76ea8ab7f1a41685d6ad62aa9146d301d4f17eb0ae0",
        ),
        (
            119,
            "31eba51c313a5c08226adf18d4a359cfdfd8d2e816b13f4af952f7ea6584dcfb",
        ),
        (
            120,
            "2f3d335432c70b580af0e8e1b3674a7c020d683aa5f73aaaedfdc55af904c21c",
        ),
    ] {
        assert_eq!(sha256_hex(&vec![b'a'; n]), want, "length {n}");
    }
}

// ---- tree listings (BR2.5, BR6.4) ----

#[test]
fn tree_listing_is_sorted_by_path_bytes_with_content_checksums() {
    let dir = TempDir::new("tree");
    dir.write("b2/c.txt", "x");
    dir.write("a.txt", "abc");
    dir.mkdir("b");
    dir.write("B.txt", "");
    let listing = tree_listing(dir.path()).unwrap();
    let want = "\
f e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 B.txt
f ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad a.txt
d b
d b2
f 2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881 b2/c.txt
";
    assert_eq!(listing, want);
}

#[test]
fn tree_listing_ignores_timestamps_but_not_content() {
    let dir = TempDir::new("tree-time");
    let file = dir.write("a.txt", "abc");
    let before = tree_listing(dir.path()).unwrap();
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
    fs::File::options()
        .write(true)
        .open(&file)
        .unwrap()
        .set_modified(old)
        .unwrap();
    assert_eq!(tree_listing(dir.path()).unwrap(), before);
    fs::write(&file, "abd").unwrap();
    assert_ne!(tree_listing(dir.path()).unwrap(), before);
}

// ---- fixed input limits (BR7.3) ----

fn entry(program: &str) -> IndexEntry {
    let text = format!(
        "schema_version = 1\n{}",
        entry_toml(program, "pure-transform")
    );
    parse_index("index.toml", &text).unwrap().entries.remove(0)
}

fn program_with_fixed(sizes: &[(&str, usize)]) -> TempDir {
    let dir = TempDir::new("fixed");
    dir.mkdir("inputs/fixed");
    for (path, size) in sizes {
        dir.write(&format!("inputs/fixed/{path}"), vec![b'x'; *size]);
    }
    dir
}

#[test]
fn fixed_inputs_at_the_limit_pass() {
    let dir = program_with_fixed(&[("basic/data.txt", 64 * 1024), ("bad/data.txt", 3)]);
    assert!(check_fixed_inputs(dir.path(), &entry("p1")).is_empty());
}

#[test]
fn a_fixed_input_file_over_64_kib_is_refused_by_name() {
    let dir = program_with_fixed(&[("basic/data.txt", 64 * 1024 + 1), ("bad/data.txt", 3)]);
    let findings = check_fixed_inputs(dir.path(), &entry("p1"));
    assert_eq!(findings.len(), 1, "{findings:?}");
    let text = findings[0].to_string();
    assert!(
        text.contains("basic/data.txt") && text.contains("p1"),
        "{text}"
    );
}

#[test]
fn more_than_eight_fixed_inputs_are_refused() {
    let mut sizes = vec![("basic/data.txt", 1), ("bad/data.txt", 1)];
    let extra = ["c", "d", "e", "f", "g", "h", "i"].map(|n| format!("{n}/data.txt"));
    for e in &extra {
        sizes.push((e.as_str(), 1));
    }
    let dir = program_with_fixed(&sizes);
    let findings = check_fixed_inputs(dir.path(), &entry("p1"));
    assert!(
        findings.iter().any(|f| f.message.contains("more than 8")),
        "{findings:?}"
    );
}

#[test]
fn listed_fixed_inputs_must_exist_and_unlisted_ones_must_not() {
    let dir = program_with_fixed(&[("basic/data.txt", 1), ("stray/data.txt", 1)]);
    let findings = check_fixed_inputs(dir.path(), &entry("p1"));
    let text: Vec<String> = findings.iter().map(ToString::to_string).collect();
    assert!(
        text.iter()
            .any(|t| t.contains("bad") && t.contains("missing")),
        "{text:?}"
    );
    assert!(
        text.iter()
            .any(|t| t.contains("stray") && t.contains("not listed")),
        "{text:?}"
    );
}

// ---- generators (BR7.1) ----

fn plain_launch(spec: &LaunchSpec) -> Command {
    let mut cmd = Command::new(&spec.exe);
    cmd.args(&spec.args).current_dir(&spec.run_dir);
    cmd
}

fn check_generator(mode: &str) -> Result<(), corpus_check::finding::Finding> {
    let dir = TempDir::new("gen");
    let exe = build_rust(dir.path(), "gen", &generator_source(mode));
    let work = dir.mkdir("work");
    check_generator_determinism(&plain_launch, "p1", &exe, 7, 50, &work)
}

#[test]
fn a_deterministic_generator_passes() {
    check_generator("ok").expect("same seed and size, same files");
}

#[test]
fn a_generator_whose_bytes_change_between_runs_fails_by_name() {
    let f = check_generator("nondet").expect_err("nondeterministic bytes");
    let text = f.to_string();
    assert!(text.contains("p1") && text.contains("data.txt"), "{text}");
}

#[test]
fn a_generator_whose_file_set_changes_between_runs_fails() {
    let f = check_generator("extra").expect_err("nondeterministic file set");
    assert!(f.to_string().contains("p1"), "{f}");
}

#[test]
fn a_generator_that_fails_is_reported_with_its_exit_code() {
    let f = check_generator("fail").expect_err("generator exits 3");
    let text = f.to_string();
    assert!(text.contains("p1") && text.contains('3'), "{text}");
}

// ---- expected outputs and bless (BR6.2, BR7.2) ----

fn outcome(code: i32, stdout: &str, stderr: &str) -> ProcOutcome {
    ProcOutcome {
        code: Some(code),
        stdout: stdout.as_bytes().to_vec(),
        stderr: stderr.as_bytes().to_vec(),
        ..ProcOutcome::default()
    }
}

#[test]
fn a_run_becomes_expected_output_only_in_the_two_allowed_shapes() {
    let ok = expected_from_outcome(&outcome(0, "hi\n", ""), None).unwrap();
    assert_eq!(ok.stdout, b"hi\n");
    assert_eq!(ok.stderr, None);

    let err = expected_from_outcome(&outcome(1, "", "error: parse: bad token\n"), None).unwrap();
    assert_eq!(
        err.stderr.as_deref(),
        Some(&b"error: parse: bad token\n"[..])
    );

    for (bad, why) in [
        (outcome(2, "", ""), "exit code 2"),
        (outcome(0, "", "noise\n"), "stderr on success"),
        (outcome(1, "", ""), "exit 1 without an error line"),
        (
            outcome(1, "", "error: a: b\nerror: c: d\n"),
            "two error lines",
        ),
        (outcome(1, "", "oops\n"), "not the typed shape"),
        (
            outcome(1, "", "error: parse: bad token"),
            "no final newline",
        ),
        (
            outcome(1, "", "error: parse: in /home/me/x\n"),
            "absolute path",
        ),
        (
            ProcOutcome {
                timed_out: true,
                ..outcome(0, "", "")
            },
            "timed out",
        ),
        (
            ProcOutcome {
                code: None,
                signal: Some(11),
                ..ProcOutcome::default()
            },
            "killed by a signal",
        ),
    ] {
        assert!(expected_from_outcome(&bad, None).is_err(), "{why}");
    }
}

#[test]
fn expected_output_round_trips_through_the_expected_directory() {
    let dir = TempDir::new("expected");
    let e = Expected {
        stdout: b"line\n".to_vec(),
        stderr: None,
        tree: Some("d a\n".to_string()),
    };
    write_expected(dir.path(), "basic", &e).unwrap();
    assert!(dir.path().join("expected/basic.out").is_file());
    assert!(!dir.path().join("expected/basic.err").exists());
    assert_eq!(load_expected(dir.path(), "basic", true).unwrap(), e);

    let err = Expected {
        stdout: Vec::new(),
        stderr: Some(b"error: x: y\n".to_vec()),
        tree: None,
    };
    write_expected(dir.path(), "bad", &err).unwrap();
    assert_eq!(load_expected(dir.path(), "bad", false).unwrap(), err);

    // A later success removes the stale error file.
    write_expected(dir.path(), "bad", &e).unwrap();
    assert!(!dir.path().join("expected/bad.err").exists());

    assert!(load_expected(dir.path(), "absent", false).is_err());
}

fn actual(stdout: &str) -> Expected {
    Expected {
        stdout: stdout.as_bytes().to_vec(),
        stderr: None,
        tree: None,
    }
}

#[test]
fn check_mode_never_writes_expected_output() {
    let dir = TempDir::new("check-mode");
    dir.mkdir("expected");
    let verdict = judge(
        Mode::Check,
        Lang::Rust,
        dir.path(),
        "basic",
        &actual("hi\n"),
    );
    assert!(matches!(verdict, Verdict::Fail(_)), "{verdict:?}");
    assert!(
        fs::read_dir(dir.path().join("expected"))
            .unwrap()
            .next()
            .is_none(),
        "check mode wrote a file"
    );
}

#[test]
fn bless_mode_writes_from_the_rust_version_only() {
    let dir = TempDir::new("bless-mode");
    dir.mkdir("expected");
    // Go goes first here on purpose: it must not write anything.
    let go = judge(Mode::Bless, Lang::Go, dir.path(), "basic", &actual("go\n"));
    assert!(matches!(go, Verdict::Fail(_)), "{go:?}");
    assert!(!dir.path().join("expected/basic.out").exists());

    let rust = judge(
        Mode::Bless,
        Lang::Rust,
        dir.path(),
        "basic",
        &actual("hi\n"),
    );
    assert_eq!(rust, Verdict::Blessed);
    assert_eq!(
        fs::read(dir.path().join("expected/basic.out")).unwrap(),
        b"hi\n"
    );

    // The other languages are then compared with what Rust wrote.
    assert_eq!(
        judge(
            Mode::Bless,
            Lang::Ocaml,
            dir.path(),
            "basic",
            &actual("hi\n")
        ),
        Verdict::Pass
    );
    assert!(matches!(
        judge(Mode::Bless, Lang::Go, dir.path(), "basic", &actual("go\n")),
        Verdict::Fail(_)
    ));
    assert_eq!(
        fs::read(dir.path().join("expected/basic.out")).unwrap(),
        b"hi\n"
    );
}

#[test]
fn check_mode_passes_a_matching_run() {
    let dir = TempDir::new("check-pass");
    write_expected(dir.path(), "basic", &actual("hi\n")).unwrap();
    assert_eq!(
        judge(Mode::Check, Lang::Go, dir.path(), "basic", &actual("hi\n")),
        Verdict::Pass
    );
}

// ---- Step 9 additions: setup files, judge detail ----

use corpus_check::setup::{SetupOp, apply_setup, parse_setup, restore_permissions};

#[test]
fn setup_files_describe_run_time_cases() {
    let ops = parse_setup("# comment\nunreadable locked.txt\n\nunwritable-dir out\n").unwrap();
    assert_eq!(
        ops,
        vec![
            SetupOp::Unreadable("locked.txt".to_string()),
            SetupOp::ReadOnlyDir("out".to_string()),
        ]
    );
    for bad in [
        "frobnicate x\n",
        "unreadable\n",
        "unreadable /etc/passwd\n",
        "unreadable ../x\n",
    ] {
        assert!(parse_setup(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn setup_creates_unreadable_files_and_read_only_directories_then_restores_them() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new("setup");
    dir.write("have.txt", "kept");
    apply_setup(
        dir.path(),
        &[
            SetupOp::Unreadable("have.txt".to_string()),
            SetupOp::Unreadable("sub/new.txt".to_string()),
            SetupOp::ReadOnlyDir("ro".to_string()),
        ],
    )
    .unwrap();
    for f in ["have.txt", "sub/new.txt"] {
        let mode = fs::metadata(dir.path().join(f))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0, "{f}");
    }
    let mode = fs::metadata(dir.path().join("ro"))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode & 0o222, 0, "read-only directory");
    restore_permissions(dir.path());
    assert!(tree_listing(dir.path()).unwrap().contains("have.txt"));
    fs::remove_dir_all(dir.path().join("sub")).unwrap();
}

#[test]
fn a_setup_file_belongs_to_a_listed_fixed_input() {
    let dir = program_with_fixed(&[
        ("basic/data.txt", 1),
        ("bad/data.txt", 1),
        ("bad.setup", 20),
    ]);
    assert!(check_fixed_inputs(dir.path(), &entry("p1")).is_empty());
    let dir = program_with_fixed(&[
        ("basic/data.txt", 1),
        ("bad/data.txt", 1),
        ("stray.setup", 20),
    ]);
    let findings = check_fixed_inputs(dir.path(), &entry("p1"));
    assert!(
        findings
            .iter()
            .any(|f| f.to_string().contains("stray.setup")),
        "{findings:?}"
    );
}

#[test]
fn a_mismatch_names_the_first_differing_byte_and_both_values() {
    let dir = TempDir::new("judge-detail");
    write_expected(dir.path(), "basic", &actual("hello\n")).unwrap();
    let Verdict::Fail(text) = judge(
        Mode::Check,
        Lang::Go,
        dir.path(),
        "basic",
        &actual("hellp\n"),
    ) else {
        panic!("a differing byte must fail");
    };
    assert!(
        text.contains("stdout")
            && text.contains("byte 4")
            && text.contains("0x6f")
            && text.contains("0x70"),
        "{text}"
    );
    let Verdict::Fail(text) = judge(
        Mode::Check,
        Lang::Go,
        dir.path(),
        "basic",
        &actual("hello\nmore"),
    ) else {
        panic!("extra output must fail");
    };
    assert!(
        text.contains("byte 6") && text.contains("expected output ends"),
        "{text}"
    );
    let Verdict::Fail(text) = judge(Mode::Check, Lang::Go, dir.path(), "basic", &actual("hell"))
    else {
        panic!("short output must fail");
    };
    assert!(
        text.contains("byte 4") && text.contains("output ends"),
        "{text}"
    );
}
