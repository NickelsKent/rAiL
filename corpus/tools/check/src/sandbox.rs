//! The OS sandbox every corpus program runs in (NFR5.9): `sandbox-exec` on
//! macOS and `bwrap` on Linux. Nothing is ever started unconfined: a missing
//! tool, an old tool or a failed self-test is an error, and `Sandbox` values
//! are the only way to start a corpus program.

use crate::pins::{Tool, parse_version_output};
use crate::proc::run_command;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// The platforms with a sandbox wrapper.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    MacOs,
    Linux,
}

impl Platform {
    /// The platform this build runs on.
    pub fn current() -> Platform {
        if cfg!(target_os = "macos") {
            Platform::MacOs
        } else {
            Platform::Linux
        }
    }

    fn tool_name(self) -> &'static str {
        match self {
            Platform::MacOs => "sandbox-exec",
            Platform::Linux => "bwrap",
        }
    }
}

/// Why no sandbox is available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SandboxError {
    /// The wrapper program was not found.
    ToolMissing {
        tool: String,
        searched: Vec<PathBuf>,
    },
    /// The wrapper is older than `corpus/toolchains.toml` allows.
    TooOld { found: String, minimum: String },
    /// The self-test showed the sandbox does not confine.
    SelfTest(String),
    /// Anything else (files, processes).
    Io(String),
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxError::ToolMissing { tool, searched } => {
                let dirs: Vec<String> = searched.iter().map(|p| p.display().to_string()).collect();
                write!(
                    f,
                    "the sandbox tool `{tool}` was not found (looked in: {}); no corpus program was run",
                    dirs.join(", ")
                )
            }
            SandboxError::TooOld { found, minimum } => write!(
                f,
                "bubblewrap {found} is older than the minimum {minimum} in toolchains.toml; no corpus program was run"
            ),
            SandboxError::SelfTest(why) => write!(
                f,
                "the sandbox self-test failed: {why}; no corpus program was run"
            ),
            SandboxError::Io(why) => write!(f, "the sandbox could not be prepared: {why}"),
        }
    }
}

impl std::error::Error for SandboxError {}

/// One mount of the Linux sandbox's read-only system view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mount {
    RoBind(PathBuf),
    Symlink { target: PathBuf, path: PathBuf },
}

/// What to run, and what it may see.
#[derive(Clone, Debug)]
pub struct SandboxSpec {
    pub exe: PathBuf,
    /// Relative paths and flags only.
    pub args: Vec<OsString>,
    /// Read-write; the current directory, `HOME` and `TMPDIR`.
    pub run_dir: PathBuf,
    /// Read-only: the program's own build directory.
    pub build_dir: PathBuf,
    /// Further read-only directories (the process-pipeline helper).
    pub read_dirs: Vec<PathBuf>,
    /// Environment beyond the fixed one.
    pub env: Vec<(String, String)>,
}

/// A located sandbox wrapper.
#[derive(Clone, Debug)]
pub struct Sandbox {
    pub platform: Platform,
    pub tool: PathBuf,
    pub profile: PathBuf,
    pub mounts: Vec<Mount>,
}

/// Where the wrapper tools are looked for.
pub fn default_search_path() -> Vec<PathBuf> {
    match Platform::current() {
        Platform::MacOs => vec![PathBuf::from("/usr/bin")],
        Platform::Linux => ["/usr/bin", "/bin", "/usr/local/bin"]
            .iter()
            .map(PathBuf::from)
            .collect(),
    }
}

/// The Linux sandbox's read-only system mounts as found on this machine:
/// `/usr`, `/lib*`, `/bin`, `/sbin` (as links when the machine links them)
/// and the dynamic loader's `/etc/ld.so*` files. Nothing else of the file
/// system is visible.
pub fn system_mounts() -> Vec<Mount> {
    let mut mounts = Vec::new();
    for dir in [
        "/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32",
    ] {
        let path = PathBuf::from(dir);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                if let Ok(target) = fs::read_link(&path) {
                    mounts.push(Mount::Symlink { target, path });
                }
            }
            Ok(_) => mounts.push(Mount::RoBind(path)),
            Err(_) => {}
        }
    }
    let mut loader: Vec<PathBuf> = fs::read_dir("/etc")
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("ld.so"))
        .map(|e| e.path())
        .collect();
    loader.sort();
    mounts.extend(loader.into_iter().map(Mount::RoBind));
    mounts
}

fn numbers(version: &str) -> Vec<u64> {
    version
        .split('.')
        .map(|part| {
            part.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
        .collect()
}

/// Whether `have` is at least `min` (dotted numbers).
pub fn version_at_least(have: &str, min: &str) -> bool {
    let (mut a, mut b) = (numbers(have), numbers(min));
    let len = a.len().max(b.len());
    a.resize(len, 0);
    b.resize(len, 0);
    a >= b
}

/// The environment every corpus program gets (and nothing else).
fn base_env(spec: &SandboxSpec) -> Vec<(String, String)> {
    let run = spec.run_dir.to_string_lossy().into_owned();
    let mut env = vec![
        ("HOME".to_string(), run.clone()),
        ("TMPDIR".to_string(), run),
        ("LANG".to_string(), "C.UTF-8".to_string()),
        ("TZ".to_string(), "UTC".to_string()),
        ("PATH".to_string(), "/usr/bin:/bin".to_string()),
    ];
    env.extend(spec.env.iter().cloned());
    env
}

/// The `bwrap` arguments for `spec` (without the `bwrap` itself).
pub fn bwrap_args(spec: &SandboxSpec, mounts: &[Mount]) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    let mut push = |items: &[&str]| args.extend(items.iter().map(OsString::from));
    push(&[
        "--die-with-parent",
        "--unshare-user-try",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--unshare-net",
        "--clearenv",
        "--proc",
        "/proc",
        "--dev",
        "/dev",
        // First, so binds that live under /tmp land inside the private one.
        "--tmpfs",
        "/tmp",
    ]);
    let mut paths: Vec<OsString> = Vec::new();
    for mount in mounts {
        match mount {
            Mount::RoBind(p) => {
                paths.extend(["--ro-bind".into(), p.into(), p.into()]);
            }
            Mount::Symlink { target, path } => {
                paths.extend(["--symlink".into(), target.into(), path.into()]);
            }
        }
    }
    for dir in std::iter::once(&spec.build_dir).chain(&spec.read_dirs) {
        paths.extend(["--ro-bind".into(), dir.into(), dir.into()]);
    }
    paths.extend([
        "--bind".into(),
        (&spec.run_dir).into(),
        (&spec.run_dir).into(),
        "--chdir".into(),
        (&spec.run_dir).into(),
    ]);
    for (key, value) in base_env(spec) {
        paths.extend(["--setenv".into(), key.into(), value.into()]);
    }
    paths.push("--".into());
    paths.push((&spec.exe).into());
    paths.extend(spec.args.iter().cloned());
    args.extend(paths);
    args
}

/// The `sandbox-exec` arguments for `spec` (without `sandbox-exec` itself).
/// The profile reads the paths from the `-D` parameters.
pub fn macos_args(profile: &Path, spec: &SandboxSpec) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["-f".into(), profile.into()];
    let mut define = |name: &str, path: &Path| {
        args.push("-D".into());
        let mut value = OsString::from(format!("{name}="));
        value.push(path);
        args.push(value);
    };
    define("RUN", &spec.run_dir);
    define("BUILD", &spec.build_dir);
    // The profile has two spare read-only slots; unused ones repeat BUILD.
    define("READ1", spec.read_dirs.first().unwrap_or(&spec.build_dir));
    define("READ2", spec.read_dirs.get(1).unwrap_or(&spec.build_dir));
    args.push((&spec.exe).into());
    args.extend(spec.args.iter().cloned());
    args
}

impl Sandbox {
    /// Finds the wrapper for `platform` in `search`; does not run it.
    pub fn locate(
        platform: Platform,
        search: &[PathBuf],
        profile: &Path,
    ) -> Result<Sandbox, SandboxError> {
        let name = platform.tool_name();
        let tool = search
            .iter()
            .map(|dir| dir.join(name))
            .find(|p| p.is_file())
            .ok_or_else(|| SandboxError::ToolMissing {
                tool: name.to_string(),
                searched: search.to_vec(),
            })?;
        let mounts = match platform {
            Platform::Linux => system_mounts(),
            Platform::MacOs => Vec::new(),
        };
        Ok(Sandbox {
            platform,
            tool,
            profile: profile.to_path_buf(),
            mounts,
        })
    }

    /// The command that starts `spec` inside the sandbox.
    pub fn command(&self, spec: &SandboxSpec) -> Command {
        let mut cmd = Command::new(&self.tool);
        cmd.env_clear().current_dir(&spec.run_dir);
        match self.platform {
            Platform::MacOs => {
                cmd.args(macos_args(&self.profile, spec));
                cmd.envs(base_env(spec));
            }
            Platform::Linux => {
                cmd.args(bwrap_args(spec, &self.mounts));
            }
        }
        cmd
    }

    /// Runs the probe (a copy of `probe_exe`) under the sandbox and requires
    /// it to be stopped from writing outside its run directory, reading the
    /// user's home directory and another path, and opening a socket, while a
    /// control write inside the run directory succeeds.
    pub fn self_test(&self, probe_exe: &Path, scratch: &Path) -> Result<(), SandboxError> {
        let io = |what: &str, e: std::io::Error| SandboxError::Io(format!("{what}: {e}"));
        let base = scratch.join(format!("selftest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&base);
        for dir in ["build", "run", "outside"] {
            fs::create_dir_all(base.join(dir)).map_err(|e| io("cannot create a directory", e))?;
        }
        let base = base
            .canonicalize()
            .map_err(|e| io("cannot resolve the scratch directory", e))?;
        let (build, run, outside) = (base.join("build"), base.join("run"), base.join("outside"));
        let probe = build.join("probe");
        fs::copy(probe_exe, &probe).map_err(|e| io("cannot copy the probe", e))?;
        let secret = outside.join("secret.txt");
        fs::write(&secret, b"secret\n").map_err(|e| io("cannot write the secret file", e))?;
        let escape = outside.join("escaped.txt");
        let home = std::env::var("HOME").ok().filter(|h| Path::new(h).is_dir());
        let listener =
            TcpListener::bind("127.0.0.1:0").map_err(|e| io("cannot open a local listener", e))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| io("cannot configure the listener", e))?;
        let addr = listener
            .local_addr()
            .map_err(|e| io("cannot read the listener address", e))?
            .to_string();

        let result = (|| {
            let probe_run = |op: &str, arg: &str| -> Result<(Option<i32>, String), SandboxError> {
                let spec = SandboxSpec {
                    exe: probe.clone(),
                    args: ["--sandbox-probe", op, arg]
                        .iter()
                        .map(OsString::from)
                        .collect(),
                    run_dir: run.clone(),
                    build_dir: build.clone(),
                    read_dirs: Vec::new(),
                    env: Vec::new(),
                };
                let out = run_command(self.command(&spec), b"", Duration::from_secs(30))
                    .map_err(|e| io("cannot start the sandbox", e))?;
                Ok((
                    out.code,
                    String::from_utf8_lossy(&out.stderr).trim().to_string(),
                ))
            };

            let (code, stderr) = probe_run("write", "control.txt")?;
            if code != Some(0) || !run.join("control.txt").is_file() {
                return Err(SandboxError::SelfTest(format!(
                    "a write inside the run directory failed (exit code {code:?}): the \
                     sandbox blocks too much to run programs. {stderr}"
                )));
            }
            let mut hostile: Vec<(&str, &str, String)> = vec![
                (
                    "write",
                    "a write outside the run directory",
                    escape.display().to_string(),
                ),
                (
                    "read",
                    "reading a file outside the run directory",
                    secret.display().to_string(),
                ),
                ("connect", "opening a network connection", addr.clone()),
            ];
            if let Some(home) = &home {
                hostile.push((
                    "read-dir",
                    "listing the user's home directory",
                    home.clone(),
                ));
            }
            for (op, what, arg) in hostile {
                let (code, _) = probe_run(op, &arg)?;
                if code != Some(3) {
                    return Err(SandboxError::SelfTest(format!(
                        "{what} was not stopped (probe exit code {code:?})"
                    )));
                }
            }
            if escape.exists() {
                return Err(SandboxError::SelfTest(
                    "the file written outside the run directory exists".to_string(),
                ));
            }
            if listener.accept().is_ok() {
                return Err(SandboxError::SelfTest(
                    "the probe reached a local listener".to_string(),
                ));
            }
            Ok(())
        })();
        let _ = fs::remove_dir_all(&base);
        result
    }
}

/// Locates the sandbox, checks its version (Linux) and runs the self-test.
/// An error means no corpus program may run.
pub fn prepare_sandbox(
    platform: Platform,
    search: &[PathBuf],
    profile: &Path,
    bubblewrap_min: &str,
    probe_exe: &Path,
    scratch: &Path,
) -> Result<Sandbox, SandboxError> {
    let sandbox = Sandbox::locate(platform, search, profile)?;
    if platform == Platform::Linux {
        let out = Command::new(&sandbox.tool)
            .arg("--version")
            .output()
            .map_err(|e| SandboxError::Io(format!("cannot run bwrap --version: {e}")))?;
        let text = String::from_utf8_lossy(&out.stdout);
        let found = parse_version_output(Tool::Bwrap, &text).ok_or_else(|| {
            SandboxError::Io(format!("unexpected bwrap --version output: {text}"))
        })?;
        if !version_at_least(&found, bubblewrap_min) {
            return Err(SandboxError::TooOld {
                found,
                minimum: bubblewrap_min.to_string(),
            });
        }
    }
    sandbox.self_test(probe_exe, scratch)?;
    Ok(sandbox)
}
