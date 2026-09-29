//! Dev-mode builds linked with `cc` and the skeleton runtime (BR2.8, BR4.2,
//! BR5.1–BR5.7, NFR3.2, NFR4.2, NFR5.4, NFR9.8, NFR9.10).

use std::fs;
use std::path::Path;
use std::process::Command;

use rail_build::{BuildConfig, BuildError, build};
use rail_testkit::{ProcessOutput, TempDir, copy_dir, repo_root, run_process};

fn fixtures_in(dir: &Path) {
    copy_dir(&repo_root().join("fixtures"), dir).expect("copy fixtures");
}

fn answer_source() -> Vec<u8> {
    fs::read(repo_root().join("fixtures/skeleton/answer.rlc")).unwrap()
}

/// Builds module `t.m` from `main_body` (a `main` returning Result unit unit).
fn build_main(dir: &Path, main_body: &str) -> Result<rail_build::Artifact, BuildError> {
    let src = format!(
        "(mod t.m)\n(pub main)\n(fn #aaaaaa main (_) (: (-> (Caps) (Result unit unit) log)) {main_body})\n"
    );
    build(dir, "t.m", src.as_bytes(), &BuildConfig::default())
}

fn run(dir: &Path, rel: &str) -> ProcessOutput {
    let mut cmd = Command::new(dir.join(rel));
    cmd.env_clear().current_dir(dir);
    run_process(&mut cmd, b"")
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}

/// Every file under `dir`, workspace-relative, sorted.
fn files(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path.strip_prefix(dir).unwrap().display().to_string());
            }
        }
    }
    out.sort();
    out
}

#[test]
fn building_answer_prints_42_and_exits_0() {
    let dir = TempDir::new("build-answer");
    fixtures_in(dir.path());
    let artifact = build(
        dir.path(),
        "skeleton.answer",
        &answer_source(),
        &BuildConfig::default(),
    )
    .expect("build");
    assert_eq!(artifact.module, "skeleton.answer");
    assert_eq!(artifact.mode, "dev");
    assert_eq!(artifact.path, ".rail/build/dev/skeleton/answer");
    assert_eq!(artifact.target, rail_codegen::host_target().unwrap().name);
    let out = run(dir.path(), &artifact.path);
    assert_eq!(
        (out.code, out.stdout_str(), out.stderr_str()),
        (Some(0), "42\n".into(), String::new())
    );
}

#[test]
fn only_the_executable_and_the_runtime_archive_remain() {
    let dir = TempDir::new("build-layout");
    build(
        dir.path(),
        "skeleton.answer",
        &answer_source(),
        &BuildConfig::default(),
    )
    .expect("build");
    let found = files(dir.path());
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(
        found[0],
        ".rail/build/dev/runtime/".to_string() + &rail_build::runtime_archive_name()
    );
    assert_eq!(found[1], ".rail/build/dev/skeleton/answer");
    assert!(rail_build::runtime_archive_name().starts_with("rt-"));
}

#[test]
fn an_err_main_exits_1_and_says_so() {
    let dir = TempDir::new("build-err");
    let artifact =
        build_main(dir.path(), "(let (_ (skel.print_i64 -12)) (Err ()))").expect("build");
    let out = run(dir.path(), &artifact.path);
    assert_eq!(out.code, Some(1));
    assert_eq!(out.stdout_str(), "-12\n");
    assert_eq!(out.stderr_str(), "error: main returned Err\n");
}

#[test]
fn overflow_and_division_by_zero_trap_with_exit_70() {
    let dir = TempDir::new("build-traps");
    let cases = [
        ("(+ 9223372036854775807 1)", "trap: integer overflow\n"),
        ("(- -9223372036854775808 1)", "trap: integer overflow\n"),
        ("(* 4611686018427387904 2)", "trap: integer overflow\n"),
        ("(/ -9223372036854775808 -1)", "trap: integer overflow\n"),
        ("(% -9223372036854775808 -1)", "trap: integer overflow\n"),
        ("(/ 1 0)", "trap: division by zero\n"),
        ("(% 1 0)", "trap: division by zero\n"),
    ];
    for (expr, message) in cases {
        let artifact = build_main(
            dir.path(),
            &format!("(let (_ (skel.print_i64 1)) (_ (skel.print_i64 {expr})) (Ok ()))"),
        )
        .unwrap_or_else(|e| panic!("{expr}: {e:?}"));
        let out = run(dir.path(), &artifact.path);
        assert_eq!(out.code, Some(70), "{expr}: {out:?}");
        assert_eq!(
            out.stdout_str(),
            "1\n",
            "output before the trap is written out"
        );
        assert_eq!(out.stderr_str(), message, "{expr}");
    }
    let fine = build_main(
        dir.path(),
        "(let (_ (skel.print_i64 (/ -7 2))) (_ (skel.print_i64 (% -7 2))) (Ok ()))",
    )
    .unwrap();
    assert_eq!(run(dir.path(), &fine.path).stdout_str(), "-3\n-1\n");
}

#[test]
fn a_blocked_module_is_refused_with_its_check_result() {
    let dir = TempDir::new("build-blocked");
    let src = fs::read(repo_root().join("fixtures/skeleton/broken.rlc")).unwrap();
    match build(dir.path(), "skeleton.broken", &src, &BuildConfig::default()) {
        Err(BuildError::Blocked(result)) => {
            assert!(result.blocking);
            assert_eq!(result.diagnostics[0].rule.as_str(), "TY001");
        }
        other => panic!("expected Blocked, got {other:?}"),
    }
    assert!(files(dir.path()).is_empty(), "nothing is written");
}

#[test]
fn a_module_without_main_has_no_entry() {
    let dir = TempDir::new("build-no-entry");
    let src = b"(mod t.m)\n(pub f)\n(fn #aaaaaa f (x) (: (-> (i64) i64)) x)\n";
    assert!(matches!(
        build(dir.path(), "t.m", src, &BuildConfig::default()),
        Err(BuildError::NoEntry)
    ));
    assert!(files(dir.path()).is_empty());
}

#[test]
fn a_missing_cc_is_build_failed() {
    let dir = TempDir::new("build-no-cc");
    let config = BuildConfig {
        cc: dir.path().join("no-such-dir/cc").into_os_string(),
    };
    match build(dir.path(), "skeleton.answer", &answer_source(), &config) {
        Err(BuildError::Failed(message)) => {
            assert!(message.contains("`cc` not found"), "{message}")
        }
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(
        !files(dir.path()).iter().any(|f| f.contains("skeleton/")),
        "{:?}",
        files(dir.path())
    );
}

#[test]
fn a_failed_link_leaves_no_artifact_or_temporary_file() {
    let dir = TempDir::new("build-link-fails");
    let fake = dir.path().join("fake-cc");
    fs::write(
        &fake,
        "#!/bin/sh\necho 'ld: simulated failure' >&2\nexit 1\n",
    )
    .unwrap();
    fs::set_permissions(&fake, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
    let config = BuildConfig {
        cc: fake.into_os_string(),
    };
    match build(dir.path(), "skeleton.answer", &answer_source(), &config) {
        Err(BuildError::Failed(message)) => {
            assert!(message.contains("ld: simulated failure"), "{message}")
        }
        other => panic!("expected Failed, got {other:?}"),
    }
    let left: Vec<String> = files(dir.path())
        .into_iter()
        .filter(|f| f.contains("skeleton/"))
        .collect();
    assert!(left.is_empty(), "{left:?}");
}

#[test]
fn builds_in_two_directories_are_identical_and_hold_no_absolute_path() {
    let one = TempDir::new("build-det-one");
    let two = TempDir::new("build-det-two-longer-name");
    let mut binaries = Vec::new();
    for dir in [&one, &two] {
        let artifact = build(
            dir.path(),
            "skeleton.answer",
            &answer_source(),
            &BuildConfig::default(),
        )
        .unwrap();
        let bytes = fs::read(dir.path().join(&artifact.path)).unwrap();
        for path in [dir.path().to_path_buf(), dir.path().canonicalize().unwrap()] {
            let text = path.display().to_string();
            assert!(!contains(&bytes, text.as_bytes()), "binary contains {text}");
        }
        binaries.push(bytes);
    }
    assert!(binaries[0] == binaries[1], "builds differ");
}

#[test]
fn directory_names_with_spaces_and_shell_metacharacters_are_safe() {
    let outer = TempDir::new("build-shell");
    let dir = outer.path().join("a b;$(touch pwned)`x`'\"&|");
    fs::create_dir_all(&dir).unwrap();
    let artifact = build(
        &dir,
        "skeleton.answer",
        &answer_source(),
        &BuildConfig::default(),
    )
    .expect("build");
    assert_eq!(run(&dir, &artifact.path).stdout_str(), "42\n");
    for place in [
        outer.path().to_path_buf(),
        dir.clone(),
        dir.join(".rail/build/dev/skeleton"),
    ] {
        assert!(!place.join("pwned").exists(), "{}", place.display());
    }
}
