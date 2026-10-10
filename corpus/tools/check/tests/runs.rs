//! Business logic: running versions in the sandbox and judging them
//! (BR2.2, BR2.3, BR6.4, BR6.5, NFR5.9, NFR9.19).

mod common;

use common::{
    PROBE, TempDir, build_standin, context, job, live_sandbox, make_entry, make_program_dir,
};
use corpus_check::budget::Budget;
use corpus_check::check::{Report, Status, run_jobs};
use corpus_check::expected::{Expected, Mode, write_expected};
use corpus_check::index::InputMode;
use corpus_check::model::{Class, Lang};
use corpus_check::sandbox::{Platform, default_search_path, prepare_sandbox};
use std::path::Path;
use std::time::{Duration, Instant};

fn ok_expected(stdout: &str) -> Expected {
    Expected {
        stdout: stdout.as_bytes().to_vec(),
        stderr: None,
        tree: None,
    }
}

fn only(report: &Report) -> &corpus_check::check::VersionReport {
    assert_eq!(report.versions.len(), 1, "{:?}", report.versions);
    &report.versions[0]
}

fn fail_text(report: &Report) -> String {
    match &only(report).status {
        Status::Fail(m) => m.clone(),
        other => panic!("expected a failure, got {other:?}"),
    }
}

/// The program saw its operation refused (it exits 7 on a refusal) and the run failed.
fn assert_refused(report: &Report) {
    let text = fail_text(report);
    assert!(
        text.contains("code 7"),
        "the program was not refused by the sandbox: {text}"
    );
}

#[test]
fn a_matching_run_passes() {
    let tmp = TempDir::new("runs-pass");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let entry = make_entry("p1", Class::PureTransform, &["ok"], &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    assert_eq!(only(&report).status, Status::Pass, "{report:?}");
    assert_eq!(only(&report).inputs_run, 1);
    assert!(!report.failed());
}

#[test]
fn a_one_byte_mismatch_fails_with_the_first_differing_byte() {
    let tmp = TempDir::new("runs-mismatch");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let entry = make_entry("p1", Class::PureTransform, &["wrong"], &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Go, &dir, &exe, &build)]);
    let text = fail_text(&report);
    assert!(
        text.contains("basic")
            && text.contains("byte 4")
            && text.contains("0x6f")
            && text.contains("0x70"),
        "{text}"
    );
    assert!(report.failed());
}

#[test]
fn a_shorter_or_longer_output_reports_where_it_ends() {
    let tmp = TempDir::new("runs-length");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let entry = make_entry("p1", Class::PureTransform, &["short"], &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Ocaml, &dir, &exe, &build)]);
    let text = fail_text(&report);
    assert!(text.contains("byte 4") && text.contains("ends"), "{text}");
}

#[test]
fn a_thread_dependent_program_fails_naming_both_thread_counts() {
    let tmp = TempDir::new("runs-threads");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("total=1000\n")).unwrap();
    let args = ["echo-threads", "--threads", "{threads}"];
    let entry = make_entry("p1", Class::DataParallel, &args, &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    let text = fail_text(&report);
    assert!(
        text.contains("1 and 4 threads") && text.contains("p1"),
        "{text}"
    );

    // The same program with a thread-independent result passes at both counts.
    let args = ["stable-threads", "--threads", "{threads}"];
    let entry = make_entry("p1", Class::DataParallel, &args, &["basic"]);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    assert_eq!(only(&report).status, Status::Pass, "{report:?}");
}

#[test]
fn input_arguments_and_stdin_come_from_the_copied_input() {
    let tmp = TempDir::new("runs-inputs");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "hello\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);

    let files = make_entry(
        "p1",
        Class::PureTransform,
        &["cat-input", "{input:data.txt}"],
        &["basic"],
    );
    let report = run_jobs(&ctx, &[job(&files, Lang::Rust, &dir, &exe, &build)]);
    assert_eq!(only(&report).status, Status::Pass, "{report:?}");

    write_expected(&dir, "basic", &ok_expected("HELLO\n")).unwrap();
    let mut stdin = make_entry("p1", Class::PureTransform, &["stdin-upper"], &["basic"]);
    stdin.input_mode = InputMode::Stdin;
    stdin.stdin_from = Some("data.txt".to_string());
    let report = run_jobs(&ctx, &[job(&stdin, Lang::Rust, &dir, &exe, &build)]);
    assert_eq!(only(&report).status, Status::Pass, "{report:?}");
}

#[test]
fn a_write_outside_the_run_directory_is_stopped_and_reported() {
    let tmp = TempDir::new("runs-write");
    let outside = TempDir::new("runs-write-outside");
    let target = outside.path().join("escaped.txt");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("wrote\n")).unwrap();
    let entry = make_entry(
        "p1",
        Class::IoServices,
        &["write-abs", target.to_str().unwrap()],
        &["basic"],
    );
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Go, &dir, &exe, &build)]);
    assert_refused(&report);
    assert!(
        !target.exists(),
        "the sandbox let the program write outside its run directory"
    );
}

#[test]
fn reading_the_users_home_directory_is_stopped_and_reported() {
    let home = std::env::var("HOME").expect("HOME is set");
    let tmp = TempDir::new("runs-home");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("listed\n")).unwrap();
    let entry = make_entry("p1", Class::IoServices, &["read-dir", &home], &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    // The reference run would have listed the directory; the sandboxed one is refused.
    assert_refused(&report);
}

#[test]
fn opening_a_socket_is_stopped_and_reported() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    let tmp = TempDir::new("runs-net");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("connected\n")).unwrap();
    let entry = make_entry("p1", Class::IoServices, &["connect", &addr], &["basic"]);
    let ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Ocaml, &dir, &exe, &build)]);
    assert_refused(&report);
    let accepted = listener.accept();
    assert!(
        matches!(&accepted, Err(e) if e.kind() == std::io::ErrorKind::WouldBlock),
        "the program reached the listener: {accepted:?}"
    );
}

#[test]
fn a_program_past_the_deadline_fails_and_does_not_hold_the_check() {
    let tmp = TempDir::new("runs-deadline");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let entry = make_entry("p1", Class::PureTransform, &["hang"], &["basic"]);
    let mut ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    ctx.deadline = Duration::from_millis(400);
    let start = Instant::now();
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    assert!(fail_text(&report).contains("deadline"));
    assert!(start.elapsed() < Duration::from_secs(20));
}

fn permission_fixture(tmp: &TempDir) -> std::path::PathBuf {
    let dir = make_program_dir(tmp, "p1", &[("perm", &[("data.txt", "x\n")])]);
    tmp.write(
        "programs/p1/inputs/fixed/perm.setup",
        "# created at run time\nunreadable locked.txt\n",
    );
    dir
}

#[test]
fn permission_denied_cases_are_created_at_run_time() {
    let tmp = TempDir::new("runs-perm");
    let (exe, build) = build_standin(tmp.path());
    let dir = permission_fixture(&tmp);
    let entry = make_entry("p1", Class::IoServices, &["perm"], &["perm"]);
    let sandbox = live_sandbox();
    let version = job(&entry, Lang::Rust, &dir, &exe, &build);

    // Skip the live part when the test itself runs as root, which can read anything.
    if !corpus_check::setup::running_as_root() {
        let bless = context(sandbox.clone(), tmp.path(), Mode::Bless);
        let report = run_jobs(&bless, std::slice::from_ref(&version));
        assert_eq!(only(&report).status, Status::Blessed, "{report:?}");
        let err = std::fs::read(dir.join("expected/perm.err")).unwrap();
        assert_eq!(err, b"error: permission: locked.txt\n");
        let tree = std::fs::read_to_string(dir.join("expected/perm.tree")).unwrap();
        assert!(
            tree.contains("locked.txt") && tree.contains("data.txt"),
            "{tree}"
        );

        let check = context(sandbox.clone(), tmp.path(), Mode::Check);
        let report = run_jobs(&check, std::slice::from_ref(&version));
        assert_eq!(only(&report).status, Status::Pass, "{report:?}");
    }

    // As root the case cannot fail, so it is skipped with a notice, never passed.
    let mut root = context(sandbox, tmp.path(), Mode::Check);
    root.is_root = true;
    let report = run_jobs(&root, &[version]);
    let v = only(&report);
    assert!(
        matches!(&v.status, Status::Skipped(n) if n.contains("root")),
        "{v:?}"
    );
    assert_eq!(v.inputs_run, 0);
    assert!(!report.failed(), "a skip is not a failure");
}

#[test]
fn a_stray_file_in_the_run_directory_is_caught_by_the_tree_check() {
    let tmp = TempDir::new("runs-tree");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    let entry = make_entry("p1", Class::IoServices, &["ok"], &["basic"]);
    let stray = make_entry("p1", Class::IoServices, &["stray"], &["basic"]);
    let sandbox = live_sandbox();

    // Bless from the Rust version records the tree as well.
    let ctx = context(sandbox.clone(), tmp.path(), Mode::Bless);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    assert_eq!(only(&report).status, Status::Blessed, "{report:?}");
    assert!(dir.join("expected/basic.tree").is_file());
    assert_eq!(
        std::fs::read(dir.join("expected/basic.out")).unwrap(),
        b"hello\n"
    );

    // A later check of a version that leaves an extra file fails on the tree.
    let ctx = context(sandbox, tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&stray, Lang::Go, &dir, &exe, &build)]);
    let text = fail_text(&report);
    assert!(
        text.contains("tree") && text.contains("stray.txt"),
        "{text}"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn the_canary_catches_a_write_a_loose_sandbox_allowed() {
    use corpus_check::sandbox::Sandbox;
    let tmp = TempDir::new("runs-canary");
    let loose = tmp.write("loose.sb", "(version 1)\n(allow default)\n");
    let sandbox = Sandbox::locate(Platform::MacOs, &default_search_path(), &loose).unwrap();
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let entry = make_entry("p1", Class::IoServices, &["escape-rel"], &["basic"]);
    let ctx = context(sandbox, tmp.path(), Mode::Check);
    let report = run_jobs(&ctx, &[job(&entry, Lang::Rust, &dir, &exe, &build)]);
    let text = fail_text(&report);
    assert!(text.contains("canary"), "{text}");
}

#[test]
fn the_budget_stops_the_running_program_and_records_the_rest_as_not_run() {
    let tmp = TempDir::new("runs-budget");
    let (exe, build) = build_standin(tmp.path());
    let dir = make_program_dir(&tmp, "p1", &[("basic", &[("data.txt", "x\n")])]);
    write_expected(&dir, "basic", &ok_expected("hello\n")).unwrap();
    let hang = make_entry("p1", Class::PureTransform, &["hang"], &["basic"]);
    let fine = make_entry("p1", Class::PureTransform, &["ok"], &["basic"]);
    let mut ctx = context(live_sandbox(), tmp.path(), Mode::Check);
    ctx.budget = Budget::new(Some(Duration::from_secs(2)));
    let jobs = [
        job(&hang, Lang::Rust, &dir, &exe, &build),
        job(&fine, Lang::Go, &dir, &exe, &build),
        job(&fine, Lang::Ocaml, &dir, &exe, &build),
    ];
    let start = Instant::now();
    let report = run_jobs(&ctx, &jobs);
    assert!(start.elapsed() < Duration::from_secs(30));
    assert!(report.over_budget && report.failed());
    assert_eq!(report.versions.len(), 3);
    assert!(
        matches!(&report.versions[0].status, Status::Fail(m) if m.contains("over time budget"))
    );
    for v in &report.versions[1..] {
        assert!(
            matches!(&v.status, Status::NotRun(m) if m.contains("over time budget")),
            "{v:?}"
        );
    }
    let slowest = report.slowest(10);
    assert_eq!(slowest.len(), 3);
    assert_eq!(
        slowest[0].lang,
        Lang::Rust,
        "the version that ate the budget is slowest"
    );
}

#[test]
fn nothing_runs_when_the_sandbox_cannot_be_prepared() {
    // `run_jobs` needs a prepared sandbox; without one there is nothing to call.
    let scratch = TempDir::new("runs-refuse");
    let marker = scratch.path().join("ran.txt");
    let result = prepare_sandbox(
        Platform::current(),
        &[],
        Path::new("/nonexistent.sb"),
        "0.6.0",
        Path::new(PROBE),
        scratch.path(),
    );
    assert!(result.is_err());
    assert!(!marker.exists());
}
