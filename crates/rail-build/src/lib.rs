//! Dev-mode build driver: check, lower, generate code, link with `cc`.
//!
//! Building block: BuildDriver. Owning unit: U1 walking-skeleton (thin first
//! version; completed by U6 execution-core).
//!
//! A build always checks first (BR5.2) and writes only under the workspace's
//! `.rail/build/dev/` (NFR5.3). The object and the executable are written to
//! `.partial` files and renamed into place only on success, so a failed build
//! leaves nothing behind (NFR9.8). `cc` is started with an argument list in
//! the module's build directory with relative paths only, never through a
//! shell (NFR5.4), so no absolute path reaches the linker (BR5.5).

#![forbid(unsafe_code)]
#![deny(clippy::print_stdout, clippy::print_stderr)]

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::process::Command;

use rail_diag::CheckResult;

/// The runtime archive, compiled by the build script for this host.
const RUNTIME_ARCHIVE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/librail_runtime.a"));

/// The build directory, relative to the workspace root.
pub const BUILD_DIR: &str = ".rail/build/dev";

/// How many lines of `cc`'s error output a failure reports.
const CC_ERROR_LINES: usize = 20;

/// Build settings.
#[derive(Clone, Debug)]
pub struct BuildConfig {
    /// The C compiler driver used to link (found through `PATH`).
    pub cc: OsString,
}

impl Default for BuildConfig {
    fn default() -> BuildConfig {
        BuildConfig { cc: "cc".into() }
    }
}

/// A built executable (a thin `BuildArtifact`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Artifact {
    /// Module qname.
    pub module: String,
    /// Host target name (`aarch64-macos` or `x86_64-linux`).
    pub target: &'static str,
    /// Always `dev`.
    pub mode: &'static str,
    /// Workspace-relative path of the executable.
    pub path: String,
}

/// Why a build produced no artifact.
#[derive(Clone, Debug)]
pub enum BuildError {
    /// The check found blocking diagnostics (BR4.2).
    Blocked(CheckResult),
    /// The module has no `main` (BR2.8).
    NoEntry,
    /// Code generation, linking or file writing failed.
    Failed(String),
}

/// The runtime archive's file name, derived from its content.
pub fn runtime_archive_name() -> String {
    format!("rt-{:016x}.a", fnv1a64(RUNTIME_ARCHIVE))
}

/// FNV-1a, 64-bit: a small, stable content hash for naming the archive.
fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Workspace-relative executable path for `qname`.
pub fn artifact_path(qname: &str) -> String {
    format!("{BUILD_DIR}/{}", qname.replace('.', "/"))
}

/// Checks, lowers, compiles and links module `qname` from `source`, writing
/// under `root/.rail/build/dev/`.
pub fn build(
    root: &Path,
    qname: &str,
    source: &[u8],
    config: &BuildConfig,
) -> Result<Artifact, BuildError> {
    let checked = rail_check::check(qname, source);
    let Some(typed) = checked.typed else {
        return Err(BuildError::Blocked(checked.result));
    };
    if !typed.has_main() {
        return Err(BuildError::NoEntry);
    }
    let target = rail_codegen::host_target().map_err(|e| BuildError::Failed(e.message))?;
    let ir = rail_lower::lower(&typed);
    let object = rail_codegen::compile(&ir, qname).map_err(|e| BuildError::Failed(e.message))?;

    let segments: Vec<&str> = qname.split('.').collect();
    let (stem, parents) = segments
        .split_last()
        .ok_or_else(|| BuildError::Failed("empty module name".into()))?;
    let build_root = root.join(BUILD_DIR);
    let module_dir = parents
        .iter()
        .fold(build_root.clone(), |dir, s| dir.join(s));
    let runtime_rel = format!(
        "{}runtime/{}",
        "../".repeat(parents.len()),
        runtime_archive_name()
    );
    fs::create_dir_all(&module_dir).map_err(|e| failed("create the build directory", &e))?;
    write_runtime(&build_root)?;

    let files = Files::new(&module_dir, stem);
    let result = link(&files, &object, &runtime_rel, stem, config);
    files.cleanup();
    result?;
    Ok(Artifact {
        module: qname.to_string(),
        target: target.name,
        mode: "dev",
        path: artifact_path(qname),
    })
}

fn failed(what: &str, e: &std::io::Error) -> BuildError {
    BuildError::Failed(format!("could not {what}: {}", e.kind()))
}

/// Writes the runtime archive under its content-hash name, atomically.
fn write_runtime(build_root: &Path) -> Result<(), BuildError> {
    let dir = build_root.join("runtime");
    let path = dir.join(runtime_archive_name());
    if fs::metadata(&path).is_ok_and(|m| m.len() == RUNTIME_ARCHIVE.len() as u64) {
        return Ok(());
    }
    fs::create_dir_all(&dir).map_err(|e| failed("create the runtime directory", &e))?;
    let partial = dir.join(format!("{}.partial", runtime_archive_name()));
    fs::write(&partial, RUNTIME_ARCHIVE).map_err(|e| failed("write the runtime archive", &e))?;
    fs::rename(&partial, &path).map_err(|e| {
        let _ = fs::remove_file(&partial);
        failed("write the runtime archive", &e)
    })
}

/// The files one module's build touches.
struct Files {
    object_partial: PathBuf,
    object: PathBuf,
    exe_partial: PathBuf,
    exe: PathBuf,
}

impl Files {
    fn new(dir: &Path, stem: &str) -> Files {
        Files {
            object_partial: dir.join(format!("{stem}.o.partial")),
            object: dir.join(format!("{stem}.o")),
            exe_partial: dir.join(format!("{stem}.partial")),
            exe: dir.join(stem),
        }
    }

    /// Removes the temporary files. After a failure an executable from an
    /// earlier successful build is left untouched.
    fn cleanup(&self) {
        for path in [&self.object_partial, &self.object, &self.exe_partial] {
            let _ = fs::remove_file(path);
        }
    }
}

fn link(
    files: &Files,
    object: &[u8],
    runtime_rel: &str,
    stem: &str,
    config: &BuildConfig,
) -> Result<(), BuildError> {
    fs::write(&files.object_partial, object).map_err(|e| failed("write the object file", &e))?;
    fs::rename(&files.object_partial, &files.object)
        .map_err(|e| failed("write the object file", &e))?;
    let dir = files.exe.parent().unwrap_or(Path::new("."));
    let output = Command::new(&config.cc)
        .current_dir(dir)
        .arg("-o")
        .arg(format!("{stem}.partial"))
        .arg(format!("{stem}.o"))
        .arg(runtime_rel)
        // Leave out debug information, which would carry object paths.
        .arg("-Wl,-S")
        .output()
        .map_err(|e| {
            if e.kind() == ErrorKind::NotFound {
                BuildError::Failed(
                    "`cc` not found; install the Xcode Command Line Tools (macOS) or a C compiler (Linux)".into(),
                )
            } else {
                BuildError::Failed(format!("`cc` could not be started: {}", e.kind()))
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first: Vec<&str> = stderr.lines().take(CC_ERROR_LINES).collect();
        return Err(BuildError::Failed(format!(
            "`cc` failed:\n{}",
            first.join("\n")
        )));
    }
    fs::rename(&files.exe_partial, &files.exe)
        .map_err(|e| failed("move the executable into place", &e))
}
