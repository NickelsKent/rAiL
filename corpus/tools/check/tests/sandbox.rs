//! The OS sandbox and its probe (NFR5.9). Structure tests run on every
//! platform; live tests run against the sandbox of the platform they are on.

mod common;

use common::{PROBE, TempDir, live_sandbox, macos_profile};
use corpus_check::probe::run_probe;
use corpus_check::sandbox::{
    Mount, Platform, Sandbox, SandboxError, SandboxSpec, bwrap_args, default_search_path,
    macos_args, prepare_sandbox, version_at_least,
};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

fn spec() -> SandboxSpec {
    SandboxSpec {
        exe: PathBuf::from("/build/prog"),
        args: vec![OsString::from("input.txt"), OsString::from("--threads")],
        run_dir: PathBuf::from("/scratch/run"),
        build_dir: PathBuf::from("/build"),
        read_dirs: vec![PathBuf::from("/helper")],
        env: vec![("EXTRA".to_string(), "1".to_string())],
    }
}

fn strings(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn position(args: &[String], needle: &[&str]) -> Option<usize> {
    args.windows(needle.len())
        .position(|w| w.iter().zip(needle).all(|(a, b)| a == b))
}

#[test]
fn versions_compare_as_dotted_numbers() {
    assert!(version_at_least("0.9.0", "0.6.0"));
    assert!(version_at_least("0.6.0", "0.6.0"));
    assert!(version_at_least("0.10.0", "0.9.9"));
    assert!(version_at_least("1.0", "0.6.0"));
    assert!(!version_at_least("0.5.9", "0.6.0"));
    assert!(!version_at_least("0.4", "0.6.0"));
}

#[test]
fn the_linux_command_exposes_only_what_a_program_needs() {
    let mounts = vec![
        Mount::RoBind(PathBuf::from("/usr")),
        Mount::Symlink {
            target: PathBuf::from("usr/lib"),
            path: PathBuf::from("/lib"),
        },
        Mount::RoBind(PathBuf::from("/etc/ld.so.cache")),
    ];
    let args = strings(&bwrap_args(&spec(), &mounts));
    for flag in [
        "--unshare-net",
        "--die-with-parent",
        "--clearenv",
        "--unshare-pid",
    ] {
        assert!(args.iter().any(|a| a == flag), "missing {flag}: {args:?}");
    }
    assert!(
        position(&args, &["--ro-bind", "/usr", "/usr"]).is_some(),
        "{args:?}"
    );
    assert!(
        position(&args, &["--symlink", "usr/lib", "/lib"]).is_some(),
        "{args:?}"
    );
    assert!(
        position(
            &args,
            &["--ro-bind", "/etc/ld.so.cache", "/etc/ld.so.cache"]
        )
        .is_some()
    );
    assert!(position(&args, &["--ro-bind", "/build", "/build"]).is_some());
    assert!(position(&args, &["--ro-bind", "/helper", "/helper"]).is_some());
    assert!(position(&args, &["--bind", "/scratch/run", "/scratch/run"]).is_some());
    assert!(position(&args, &["--chdir", "/scratch/run"]).is_some());
    assert!(position(&args, &["--setenv", "HOME", "/scratch/run"]).is_some());
    assert!(position(&args, &["--setenv", "TMPDIR", "/scratch/run"]).is_some());
    assert!(position(&args, &["--setenv", "EXTRA", "1"]).is_some());
    // the private /tmp comes before any bind that may live under it
    let tmpfs = position(&args, &["--tmpfs", "/tmp"]).expect("private /tmp");
    assert!(tmpfs < position(&args, &["--bind", "/scratch/run", "/scratch/run"]).unwrap());
    // never the whole file system, the home directory or the run directory's parent
    assert!(position(&args, &["--bind", "/", "/"]).is_none());
    assert!(position(&args, &["--ro-bind", "/", "/"]).is_none());
    assert!(
        !args
            .iter()
            .any(|a| a == "/scratch" || a.starts_with("/home") || a.starts_with("/Users"))
    );
    // the program and its relative arguments come last, after `--`
    let tail = &args[args.len() - 4..];
    assert_eq!(tail, ["--", "/build/prog", "input.txt", "--threads"]);
}

#[test]
fn the_macos_command_passes_the_profile_and_its_parameters() {
    let args = strings(&macos_args(Path::new("/profile.sb"), &spec()));
    assert_eq!(&args[..2], ["-f", "/profile.sb"]);
    assert!(
        position(&args, &["-D", "RUN=/scratch/run"]).is_some(),
        "{args:?}"
    );
    assert!(
        position(&args, &["-D", "BUILD=/build"]).is_some(),
        "{args:?}"
    );
    assert!(
        position(&args, &["-D", "READ1=/helper"]).is_some(),
        "{args:?}"
    );
    let tail = &args[args.len() - 3..];
    assert_eq!(tail, ["/build/prog", "input.txt", "--threads"]);
}

#[test]
fn a_missing_sandbox_tool_is_an_error_that_stops_everything() {
    let scratch = TempDir::new("no-tool");
    let err = prepare_sandbox(
        Platform::current(),
        &[],
        &macos_profile(),
        "0.6.0",
        Path::new(PROBE),
        scratch.path(),
    )
    .expect_err("no tool in an empty search path");
    assert!(matches!(err, SandboxError::ToolMissing { .. }), "{err:?}");
    let text = err.to_string();
    assert!(
        text.contains("sandbox-exec") || text.contains("bwrap"),
        "{text}"
    );
    assert!(Sandbox::locate(Platform::current(), &[], &macos_profile()).is_err());
    assert!(!default_search_path().is_empty());
}

#[test]
fn the_self_test_passes_on_this_platform() {
    let scratch = TempDir::new("self-test");
    prepare_sandbox(
        Platform::current(),
        &default_search_path(),
        &macos_profile(),
        "0.6.0",
        Path::new(PROBE),
        scratch.path(),
    )
    .expect("the sandbox confines the probe");
}

#[cfg(target_os = "macos")]
#[test]
fn the_self_test_fails_for_a_sandbox_that_does_not_confine() {
    let scratch = TempDir::new("self-test-loose");
    let loose = scratch.write("loose.sb", "(version 1)\n(allow default)\n");
    let sandbox = Sandbox::locate(Platform::MacOs, &default_search_path(), &loose).unwrap();
    let err = sandbox
        .self_test(Path::new(PROBE), scratch.path())
        .expect_err("an allow-everything profile must be detected");
    assert!(matches!(err, SandboxError::SelfTest(_)), "{err:?}");
}

#[test]
fn the_sandbox_command_for_this_platform_starts_the_wrapper() {
    let sandbox = live_sandbox();
    let cmd = sandbox.command(&spec());
    let program = cmd.get_program().to_string_lossy().into_owned();
    assert!(
        program.ends_with("sandbox-exec") || program.ends_with("bwrap"),
        "{program}"
    );
}

// ---- the probe (run directly, outside any sandbox) ----

fn probe(args: &[&str]) -> i32 {
    run_probe(&args.iter().map(ToString::to_string).collect::<Vec<_>>())
}

#[test]
fn the_probe_reports_whether_the_operation_was_allowed() {
    let dir = TempDir::new("probe");
    let file = dir.path().join("f.txt");
    assert_eq!(probe(&["write", file.to_str().unwrap()]), 0);
    assert!(file.is_file());
    assert_eq!(
        probe(&["write", dir.path().join("no/such/dir/f").to_str().unwrap()]),
        3
    );
    assert_eq!(probe(&["read-dir", dir.path().to_str().unwrap()]), 0);
    assert_eq!(
        probe(&["read-dir", dir.path().join("absent").to_str().unwrap()]),
        3
    );
    assert_eq!(probe(&["read", file.to_str().unwrap()]), 0);

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap().to_string();
    assert_eq!(probe(&["connect", &addr]), 0);
    drop(listener);
    assert_eq!(probe(&["connect", &addr]), 3, "nothing listens any more");

    assert_eq!(probe(&["frobnicate", "x"]), 2);
    assert_eq!(probe(&["write"]), 2);
}
