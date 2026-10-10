//! Building the Rust, Go and OCaml versions offline with the pinned
//! toolchains (WF3 step 5).

use crate::model::Lang;
use crate::proc::run_command;
use crate::toml::{self, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// How long one build may take.
const BUILD_DEADLINE: Duration = Duration::from_secs(15 * 60);

/// The most tool output kept in an error message.
const MESSAGE_LIMIT: usize = 4000;

/// A built version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Built {
    pub exe: PathBuf,
    /// The directory the sandbox exposes read-only: it holds only the
    /// program, a copy made after the build.
    pub build_dir: PathBuf,
}

/// Builds the version in `version_dir` (the `rust/`, `go/` or `ocaml/`
/// directory) into `out_dir`. The error is the tool's own message, trimmed.
pub fn build_version(lang: Lang, version_dir: &Path, out_dir: &Path) -> Result<Built, String> {
    fs::create_dir_all(out_dir).map_err(|e| format!("cannot create {}: {e}", out_dir.display()))?;
    let out_dir = out_dir
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", out_dir.display()))?;
    let version_dir = version_dir
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", version_dir.display()))?;
    let work = out_dir.join("work").join(lang.name());
    fs::create_dir_all(out_dir.join("work"))
        .map_err(|e| format!("cannot create the work directory: {e}"))?;
    let built_exe = match lang {
        Lang::Rust => build_rust(&version_dir, &work)?,
        Lang::Go => build_go(&version_dir, &work)?,
        Lang::Ocaml => build_ocaml(&version_dir, &work)?,
        Lang::Rail => {
            return Err("the rail version is not built by the corpus check (BR8.2)".to_string());
        }
    };
    // Only the finished program is exposed to the sandbox, not the build tree.
    let build_dir = out_dir.join("exe").join(lang.name());
    fs::create_dir_all(&build_dir).map_err(|e| format!("cannot create the exe directory: {e}"))?;
    let exe = build_dir.join("prog");
    fs::copy(&built_exe, &exe).map_err(|e| format!("cannot copy the built program: {e}"))?;
    Ok(Built { exe, build_dir })
}

fn run_tool(mut cmd: Command, name: &str) -> Result<(), String> {
    cmd.env("NO_COLOR", "1");
    let outcome =
        run_command(cmd, b"", BUILD_DEADLINE).map_err(|e| format!("cannot run {name}: {e}"))?;
    if outcome.timed_out {
        return Err(format!("{name} ran past the build deadline"));
    }
    if outcome.code == Some(0) {
        return Ok(());
    }
    let mut text = String::from_utf8_lossy(&outcome.stderr).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&outcome.stdout).into_owned();
    }
    let text = text.trim();
    let shown: String = text.chars().take(MESSAGE_LIMIT).collect();
    Err(format!("{name} failed: {shown}"))
}

fn build_rust(dir: &Path, work: &Path) -> Result<PathBuf, String> {
    let manifest = dir.join("Cargo.toml");
    let text = fs::read_to_string(&manifest).map_err(|e| format!("cannot read Cargo.toml: {e}"))?;
    let doc = toml::parse(&text).map_err(|e| format!("Cargo.toml: {e}"))?;
    let name = doc
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(Value::as_str)
        .ok_or("Cargo.toml has no [package] name")?
        .to_string();
    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--release",
        "--locked",
        "--offline",
        "--manifest-path",
    ])
    .arg(&manifest)
    .arg("--target-dir")
    .arg(work)
    .current_dir(dir)
    // An outer `cargo test` or `cargo run` must not steer this build.
    .env_remove("CARGO_TARGET_DIR")
    .env_remove("RUSTFLAGS");
    run_tool(cmd, "cargo")?;
    Ok(work.join("release").join(name))
}

fn build_go(dir: &Path, work: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let exe = work.join("prog");
    let mut cmd = Command::new("go");
    cmd.args(["build", "-trimpath", "-o"])
        .arg(&exe)
        .arg(".")
        .current_dir(dir)
        .env("GOPROXY", "off")
        .env("GOTOOLCHAIN", "local")
        .env("GOFLAGS", "-buildvcs=false")
        .env("CGO_ENABLED", "0");
    run_tool(cmd, "go")?;
    Ok(exe)
}

fn build_ocaml(dir: &Path, work: &Path) -> Result<PathBuf, String> {
    let jobs = std::thread::available_parallelism().map_or(2, |n| n.get().min(8));
    let mut cmd = Command::new("dune");
    cmd.args(["build", "--root"])
        .arg(dir)
        .arg("--build-dir")
        .arg(work)
        // An explicit job count: dune's own CPU detection crashes in some
        // restricted environments.
        .args(["-j", &jobs.to_string()])
        .args([
            "--display",
            "quiet",
            "--profile",
            "release",
            "./bin/main.exe",
        ])
        .current_dir(dir)
        .env("DUNE_CACHE", "disabled");
    run_tool(cmd, "dune")?;
    Ok(work.join("default/bin/main.exe"))
}
