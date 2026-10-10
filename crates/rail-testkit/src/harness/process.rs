//! Acceptance harness: the process runner for suites.
//!
//! - Every `rail` process gets only an allow-listed environment: `PATH`
//!   (the harness's own), `LANG=C.UTF-8`, `TZ=UTC`, and `HOME` and `TMPDIR`
//!   inside the run directory, plus literal variables a suite names in its
//!   code. Nothing is copied from the host (NFR5.7).
//! - Each suite run has a fresh run directory with `home/`, `tmp/` and
//!   `work/`, removed after the run unless it failed (NFR5.8).
//! - Each process starts in its own process group; at its deadline, or when
//!   the run is stopped, the whole group is killed, grandchildren included
//!   (BR5.2, NFR9.18). After a stop no new process starts.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::harness::settings::SuiteId;

/// One process a suite asks the harness to start.
#[derive(Clone, Debug)]
pub struct Proc {
    /// The program.
    pub program: PathBuf,
    /// Its arguments.
    pub args: Vec<OsString>,
    /// Extra variables: literal names and values only (NFR5.7).
    pub env: Vec<(&'static str, &'static str)>,
    /// Its working directory (inside the run directory).
    pub cwd: Option<PathBuf>,
    /// Bytes written to its standard input.
    pub stdin: Vec<u8>,
    /// Its deadline; the suite's recorded deadline when absent.
    pub deadline: Option<Duration>,
}

impl Proc {
    /// A process running `program`.
    pub fn new(program: impl Into<PathBuf>) -> Proc {
        Proc {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
            stdin: Vec::new(),
            deadline: None,
        }
    }

    /// Adds one argument.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Proc {
        self.args.push(arg.into());
        self
    }

    /// Adds arguments.
    pub fn args<I, S>(mut self, args: I) -> Proc
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Adds one literal environment variable.
    pub fn env(mut self, name: &'static str, value: &'static str) -> Proc {
        self.env.push((name, value));
        self
    }

    /// Sets the working directory.
    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Proc {
        self.cwd = Some(dir.into());
        self
    }

    /// Sets standard input.
    pub fn stdin(mut self, bytes: impl Into<Vec<u8>>) -> Proc {
        self.stdin = bytes.into();
        self
    }

    /// Sets a tighter deadline than the suite's.
    pub fn deadline(mut self, deadline: Duration) -> Proc {
        self.deadline = Some(deadline);
        self
    }
}

/// What a finished process produced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcResult {
    /// The exit code, when it exited normally.
    pub code: Option<i32>,
    /// The signal that ended it, if any.
    pub signal: Option<i32>,
    /// Captured standard output.
    pub stdout: Vec<u8>,
    /// Captured standard error.
    pub stderr: Vec<u8>,
    /// Whether it ran past its deadline and was killed with its group.
    pub timed_out: bool,
}

/// Why a suite could not produce an observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunError {
    /// The run was stopped (time budget); no new process may start.
    Stopped,
    /// A process could not be started.
    Spawn(String),
    /// The suite itself could not observe a result.
    Suite(String),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::Stopped => f.write_str("over time budget"),
            RunError::Spawn(m) => write!(f, "could not start a process: {m}"),
            RunError::Suite(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for RunError {}

/// A suite run's directory: `home/`, `tmp/` and `work/` under the system
/// temporary directory. Removed on drop unless [`RunDir::keep`] was called.
#[derive(Debug)]
pub struct RunDir {
    path: PathBuf,
    kept: bool,
}

impl RunDir {
    /// Creates a fresh, uniquely named run directory for `suite`.
    pub fn create(suite: SuiteId) -> io::Result<RunDir> {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let name = format!("rail-accept-{suite}-{}-{n}-{nanos}", std::process::id());
        let dir = RunDir {
            path: std::env::temp_dir().join(name),
            kept: false,
        };
        for sub in ["home", "tmp", "work"] {
            fs::create_dir_all(dir.path.join(sub))?;
        }
        Ok(dir)
    }

    /// The directory itself.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `HOME` of every process in the run.
    pub fn home(&self) -> PathBuf {
        self.path.join("home")
    }

    /// The `TMPDIR` of every process in the run.
    pub fn tmp(&self) -> PathBuf {
        self.path.join("tmp")
    }

    /// The working directory; suite inputs are copied here.
    pub fn work(&self) -> PathBuf {
        self.path.join("work")
    }

    /// Keeps the directory (after a failure) and returns its path.
    pub fn keep(mut self) -> PathBuf {
        self.kept = true;
        self.path.clone()
    }
}

impl Drop for RunDir {
    fn drop(&mut self) {
        if !self.kept {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

/// Kills every process in group `pgid` with `/bin/kill -KILL -- -<pgid>`,
/// started as an argument list (no shell, no `unsafe`).
pub fn kill_process_group(pgid: u32) -> io::Result<()> {
    let status = Command::new("/bin/kill")
        .args(["-KILL", "--", &format!("-{pgid}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "kill of process group {pgid} failed"
        )))
    }
}

#[derive(Debug, Default)]
struct StopState {
    stopped: AtomicBool,
    running: Mutex<BTreeSet<u32>>,
}

/// Shared between the scheduler, the budget watchdog and the run context:
/// once stopped, the running process groups are killed and no new process
/// starts (NFR9.18).
#[derive(Clone, Debug, Default)]
pub struct StopSignal {
    state: Arc<StopState>,
}

impl StopSignal {
    /// A signal that has not fired.
    pub fn new() -> StopSignal {
        StopSignal::default()
    }

    /// Stops the run: kills every running process group.
    pub fn stop(&self) {
        self.state.stopped.store(true, Ordering::SeqCst);
        let running = self
            .state
            .running
            .lock()
            .map(|set| set.clone())
            .unwrap_or_default();
        for pgid in running {
            let _ = kill_process_group(pgid);
        }
    }

    /// Whether the run was stopped.
    pub fn is_stopped(&self) -> bool {
        self.state.stopped.load(Ordering::SeqCst)
    }

    fn register(&self, pgid: u32) {
        if let Ok(mut set) = self.state.running.lock() {
            set.insert(pgid);
        }
        // A stop that fired between the check and the registration.
        if self.is_stopped() {
            let _ = kill_process_group(pgid);
        }
    }

    fn unregister(&self, pgid: u32) {
        if let Ok(mut set) = self.state.running.lock() {
            set.remove(&pgid);
        }
    }
}

/// The context one suite run starts processes through.
pub struct RunContext {
    home: PathBuf,
    tmp: PathBuf,
    work: PathBuf,
    root: PathBuf,
    rail: PathBuf,
    deadline: Duration,
    stop: StopSignal,
}

impl RunContext {
    /// A context for one run in `dir`: `root` is the repository (inputs
    /// are copied from it), `rail` the build the suite runs against, and
    /// `deadline` the suite's recorded per-process deadline.
    pub fn new(
        dir: &RunDir,
        root: &Path,
        rail: &Path,
        deadline: Duration,
        stop: StopSignal,
    ) -> RunContext {
        RunContext {
            home: dir.home(),
            tmp: dir.tmp(),
            work: dir.work(),
            root: root.to_path_buf(),
            rail: rail.to_path_buf(),
            deadline,
            stop,
        }
    }

    /// The run's working directory.
    pub fn work_dir(&self) -> &Path {
        &self.work
    }

    /// The repository root, for copying recorded inputs into `work/`.
    pub fn repo_root(&self) -> &Path {
        &self.root
    }

    /// The `rail` build this suite runs against.
    pub fn rail(&self) -> &Path {
        &self.rail
    }

    /// Runs one process with the scrubbed environment, in its own process
    /// group, until it exits, reaches its deadline, or the run is stopped.
    pub fn run(&self, proc: &Proc) -> Result<ProcResult, RunError> {
        if self.stop.is_stopped() {
            return Err(RunError::Stopped);
        }
        let mut cmd = Command::new(&proc.program);
        cmd.args(&proc.args)
            .env_clear()
            .env("LANG", "C.UTF-8")
            .env("TZ", "UTC")
            .env("HOME", &self.home)
            .env("TMPDIR", &self.tmp)
            .current_dir(proc.cwd.as_ref().unwrap_or(&self.work))
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(path) = std::env::var_os("PATH") {
            cmd.env("PATH", path);
        }
        for (name, value) in &proc.env {
            cmd.env(name, value);
        }
        let deadline = proc.deadline.unwrap_or(self.deadline);
        let result = supervise(&mut cmd, &proc.stdin, deadline, &self.stop)
            .map_err(|e| RunError::Spawn(format!("{}: {e}", proc.program.display())))?;
        if self.stop.is_stopped() {
            return Err(RunError::Stopped);
        }
        Ok(result)
    }
}

fn reader<R: Read + Send + 'static>(mut source: R) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = source.read_to_end(&mut buffer);
        buffer
    })
}

/// Starts `cmd` (already configured with its own process group), feeds it
/// `stdin`, and waits for it, killing its group at `deadline` or on a stop.
fn supervise(
    cmd: &mut Command,
    stdin: &[u8],
    deadline: Duration,
    stop: &StopSignal,
) -> io::Result<ProcResult> {
    let mut child = cmd.spawn()?;
    let pgid = child.id();
    stop.register(pgid);
    let input = stdin.to_vec();
    let writer = child.stdin.take().map(|mut pipe| {
        thread::spawn(move || {
            let _ = pipe.write_all(&input);
        })
    });
    let out = child.stdout.take().map(reader);
    let err = child.stderr.take().map(reader);
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(e) => {
                let _ = kill_process_group(pgid);
                stop.unregister(pgid);
                return Err(e);
            }
        }
        if start.elapsed() >= deadline || stop.is_stopped() {
            timed_out = !stop.is_stopped();
            if kill_process_group(pgid).is_err() {
                let _ = child.kill();
            }
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(5));
    };
    // Descendants left behind by a process that exited would hold its
    // output pipes open; the group is ended with it.
    let _ = kill_process_group(pgid);
    stop.unregister(pgid);
    if let Some(w) = writer {
        let _ = w.join();
    }
    let join =
        |h: Option<thread::JoinHandle<Vec<u8>>>| h.and_then(|h| h.join().ok()).unwrap_or_default();
    Ok(ProcResult {
        code: status.code(),
        signal: status.signal(),
        stdout: join(out),
        stderr: join(err),
        timed_out,
    })
}
