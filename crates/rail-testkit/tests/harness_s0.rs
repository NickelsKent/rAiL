//! Acceptance harness tests: S0, the walking-skeleton smoke suite, driven
//! through `rail-accept` against the real `rail` (BR8.1, BR7.6, NFR9.18).
//! S0 runs through the scrubbed environment (NFR5.7), so these tests also
//! show that `rail build` links with only `PATH` and a temporary `HOME`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use rail_testkit::harness::build::build_rail;
use rail_testkit::harness::settings::Build;
use rail_testkit::{TempDir, copy_dir, repo_root};

fn accept(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rail-accept"))
        .args(args)
        .output()
        .expect("start rail-accept")
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn path(p: &Path) -> &str {
    p.to_str().expect("UTF-8 path")
}

const STEPS: [&str; 7] = [
    "1 check skeleton.answer: no diagnostics",
    "2 run skeleton.answer: prints 42, exit 0",
    "3 protocol session on skeleton.answer: run.run returns 42",
    "4 skeleton.broken refused with one TY001 (command line and protocol)",
    "5 server answers after a malformed message",
    "6 two builds of skeleton.answer are byte-identical",
    "7 outputs of steps 1-4 equal the shared golden files",
];

#[test]
fn br8_1_s0_passes_against_the_real_rail() {
    let out_dir = TempDir::new("s0-pass");
    let out = accept(&[
        "--level",
        "pr",
        "--suite",
        "S0",
        "--out",
        path(out_dir.path()),
    ]);
    let stdout = text(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{stdout}\n{}",
        text(&out.stderr)
    );
    for step in STEPS {
        assert!(
            stdout.contains(&format!("PASS {step}\n")),
            "{step:?} in {stdout}"
        );
    }
    assert!(stdout.contains("% of budget"), "{stdout}");
    let json = std::fs::read_to_string(out_dir.path().join("results-pr.json")).unwrap();
    assert!(
        json.contains(r#"{"suite":"S0","outcome":"pass","build":"debug""#),
        "{json}"
    );
}

/// A copy of what S0 reads: the settings file, the fixtures and the shared
/// golden files.
fn scratch_root() -> TempDir {
    let dir = TempDir::new("s0-root");
    let root = repo_root();
    for rel in ["fixtures", "crates/rail/tests/golden"] {
        copy_dir(&root.join(rel), &dir.path().join(rel)).unwrap();
    }
    let settings = dir.path().join("tests/acceptance");
    std::fs::create_dir_all(&settings).unwrap();
    std::fs::copy(
        root.join("tests/acceptance/suites.toml"),
        settings.join("suites.toml"),
    )
    .unwrap();
    dir
}

fn kept_dir(stdout: &str) -> Option<PathBuf> {
    stdout
        .lines()
        .find_map(|l| l.trim().strip_prefix("kept run directory: "))
        .map(PathBuf::from)
}

#[test]
fn br8_1_a_corrupted_golden_file_fails_s0_naming_the_step() {
    let rail = build_rail(&repo_root(), Build::Debug, None).expect("debug rail");
    let root = scratch_root();
    let golden = root
        .path()
        .join("crates/rail/tests/golden/check-answer.json");
    std::fs::write(&golden, "{\"corrupted\":true}\n").unwrap();
    let out_dir = TempDir::new("s0-fail");
    let out = accept(&[
        "--level",
        "pr",
        "--suite",
        "S0",
        "--root",
        path(root.path()),
        "--rail",
        &format!("debug={}", rail.display()),
        "--out",
        path(out_dir.path()),
    ]);
    let stdout = text(&out.stdout);
    if let Some(kept) = kept_dir(&stdout) {
        let _ = std::fs::remove_dir_all(kept);
    }
    assert_eq!(
        out.status.code(),
        Some(1),
        "{stdout}\n{}",
        text(&out.stderr)
    );
    assert!(stdout.contains(&format!("FAIL {}", STEPS[0])), "{stdout}");
    assert!(stdout.contains(&format!("PASS {}", STEPS[1])), "{stdout}");
    assert!(stdout.contains(&format!("FAIL {}", STEPS[6])), "{stdout}");
    assert!(
        stdout.contains("rerun: rail-accept --level pr --suite S0"),
        "{stdout}"
    );
    let json = std::fs::read_to_string(out_dir.path().join("results-pr.json")).unwrap();
    assert!(json.contains(r#""outcome":"fail""#), "{json}");
    assert!(json.contains("check skeleton.answer"), "{json}");
}
